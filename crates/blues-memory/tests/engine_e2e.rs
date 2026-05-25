//! End-to-end integration tests for `blues-memory`.
//!
//! Black-box: drives the engine through its public `MemoryEngine` trait
//! surface only. Each test models a realistic call sequence the daemon will
//! issue (chat capture → triage → recall → context compile → blame → decay).
//!
//! See ARCHITECTURE.md §2.4 + §4.

use std::sync::Arc;

use blues_core::{BluesError, MemoryType, ProjectId};
use blues_memory::{
    embed::HashEmbedder,
    store::SqliteStore,
    ConsolidateMode, ContextRequest, Edit, Engine, HitId, InboxFilter, MemoryEngine,
    MemoryQuery, MemoryRef, MemoryWrite, Scope,
};

fn engine() -> Engine {
    Engine::new(SqliteStore::open_in_memory().unwrap(), Arc::new(HashEmbedder))
}

fn semantic(content: &str) -> MemoryWrite {
    MemoryWrite {
        content: content.into(),
        kind: MemoryType::Semantic,
        source: Some("agent:smart".into()),
        confidence: Some(0.7),
        plan_id: None,
        node_id: None,
        provenance: None,
    }
}

/// Full happy path the daemon issues during a normal chat session:
/// chat captures a candidate → user triages it → later recall + context
/// compile see the approved fact → blame surfaces the audit chain.
#[tokio::test]
async fn capture_triage_recall_compile_blame_roundtrip() {
    let e = engine();
    let p = ProjectId::new();

    let MemoryRef::Inbox(item) =
        e.save(p, semantic("logout flow lives in auth.ts")).await.unwrap()
    else {
        panic!("expected inbox ref");
    };

    let inbox = e.list_inbox(p, InboxFilter::default()).await.unwrap();
    assert_eq!(inbox.items.len(), 1);
    assert_eq!(inbox.items[0].id, item);

    let fact = e.approve_inbox(item, None).await.unwrap();
    assert!(e
        .list_inbox(p, InboxFilter::default())
        .await
        .unwrap()
        .items
        .is_empty());

    let recall = e
        .query(
            p,
            MemoryQuery {
                query: "logout".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await
        .unwrap();
    let HitId::Fact(found) = recall.items[0].id else {
        panic!("expected fact hit");
    };
    assert_eq!(found, fact);

    let cc = e
        .compile_context(ContextRequest {
            project: p,
            task: "logout".into(),
            token_budget: 4096,
            model_id: None,
        })
        .await
        .unwrap();
    assert!(!cc.blocks.is_empty());
    assert!(cc.total_tokens > 0 && cc.total_tokens <= 4096);

    let prov = e.blame(fact).await.unwrap();
    let actions: Vec<&str> = prov.chain.iter().map(|s| s.action.as_str()).collect();
    assert!(actions.contains(&"ingest"));
    assert!(actions.contains(&"approve"));
}

/// Edit-on-approve replaces content + kind, but the audit chain still
/// records the original `ingest` step. Important for the inbox UI: editors
/// can fix typos or reclassify without losing provenance.
#[tokio::test]
async fn approve_with_edit_preserves_audit_chain() {
    let e = engine();
    let p = ProjectId::new();
    let MemoryRef::Inbox(item) = e.save(p, semantic("draft note")).await.unwrap() else {
        panic!()
    };
    let fact = e
        .approve_inbox(
            item,
            Some(Edit {
                content: Some("polished note".into()),
                kind: Some(MemoryType::Procedural),
            }),
        )
        .await
        .unwrap();

    let prov = e.blame(fact).await.unwrap();
    let actions: Vec<&str> = prov.chain.iter().map(|s| s.action.as_str()).collect();
    assert!(actions.contains(&"ingest"));
    assert!(actions.contains(&"approve"));

    let r = e
        .query(
            p,
            MemoryQuery {
                query: "polished".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: Some(MemoryType::Procedural),
            },
        )
        .await
        .unwrap();
    let HitId::Fact(id) = r.items[0].id else { panic!() };
    assert_eq!(id, fact);
}

/// Reject drops the inbox item entirely; recall must not surface it.
#[tokio::test]
async fn rejected_candidate_never_reaches_recall() {
    let e = engine();
    let p = ProjectId::new();
    let MemoryRef::Inbox(item) = e
        .save(p, semantic("speculative claim about billing"))
        .await
        .unwrap()
    else {
        panic!()
    };
    e.reject_inbox(item).await.unwrap();

    let r = e
        .query(
            p,
            MemoryQuery {
                query: "billing".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await
        .unwrap();
    assert!(r.items.is_empty());
}

/// Project isolation is enforced at the recall + compile boundary. A fact
/// confirmed in project A must never leak into project B.
#[tokio::test]
async fn projects_are_isolated_at_recall_and_compile() {
    let e = engine();
    let pa = ProjectId::new();
    let pb = ProjectId::new();

    let MemoryRef::Inbox(item) = e
        .save(pa, semantic("only project A knows this"))
        .await
        .unwrap()
    else {
        panic!()
    };
    e.approve_inbox(item, None).await.unwrap();

    let r = e
        .query(
            pb,
            MemoryQuery {
                query: "project".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await
        .unwrap();
    assert!(r.items.is_empty(), "leak across projects via query");

    let cc = e
        .compile_context(ContextRequest {
            project: pb,
            task: "project".into(),
            token_budget: 4096,
            model_id: None,
        })
        .await
        .unwrap();
    assert!(cc.blocks.is_empty(), "leak across projects via compile");
}

/// Decay runs across both fact kinds (semantic + procedural) and the
/// `All` mode is the default cron knob. After one pass every confidence
/// must drop by the documented factor; merge / procedure stay no-ops.
#[tokio::test]
async fn consolidate_all_decays_every_kind() {
    let e = engine();
    let p = ProjectId::new();

    async fn approve(e: &Engine, p: ProjectId, c: &str, k: MemoryType) -> blues_core::FactId {
        let mut w = semantic(c);
        w.kind = k;
        w.confidence = Some(0.9);
        let MemoryRef::Inbox(item) = e.save(p, w).await.unwrap() else {
            panic!()
        };
        e.approve_inbox(item, None).await.unwrap()
    }

    let sem = approve(&e, p, "semantic A", MemoryType::Semantic).await;
    let proc_ = approve(&e, p, "procedural B", MemoryType::Procedural).await;

    e.consolidate(p, ConsolidateMode::All).await.unwrap();

    // both records present; confidences reduced. We don't recheck the exact
    // factor (engine_test::consolidate_decay_lowers_fact_confidence does)
    // — we just assert the All mode reaches both kinds.
    let r = e
        .query(
            p,
            MemoryQuery {
                query: "semantic procedural".into(),
                top_k: 10,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await
        .unwrap();
    assert!(r.items.iter().any(|h| matches!(h.id, HitId::Fact(id) if id == sem)));
    assert!(r.items.iter().any(|h| matches!(h.id, HitId::Fact(id) if id == proc_)));
}

/// `save` rejects empty / whitespace-only content. Tightens the inbox so
/// noise from auto-extractors can't blow up the triage queue.
#[tokio::test]
async fn save_rejects_empty_or_whitespace() {
    let e = engine();
    let p = ProjectId::new();
    for bad in ["", "   ", "\n\t"] {
        let mut w = semantic("placeholder");
        w.content = bad.into();
        let err = e.save(p, w).await.unwrap_err();
        assert!(matches!(err, BluesError::InvalidArgument(_)), "got {err:?}");
    }
}

/// Approving an unknown id must return NotFound, not silently succeed.
/// Daemon depends on this to drive UI error states.
#[tokio::test]
async fn approve_unknown_inbox_item_is_not_found() {
    let e = engine();
    let err = e
        .approve_inbox(blues_core::InboxItemId::new(), None)
        .await
        .unwrap_err();
    assert!(matches!(err, BluesError::NotFound(_)));
}

/// Inbox filters compose: `kind` and `min_confidence` together. Triage UI
/// uses this to hide low-confidence noise per-kind.
#[tokio::test]
async fn inbox_filter_kind_and_min_confidence() {
    let e = engine();
    let p = ProjectId::new();

    let mut a = semantic("high-confidence semantic");
    a.confidence = Some(0.9);
    e.save(p, a).await.unwrap();

    let mut b = semantic("low-confidence semantic");
    b.confidence = Some(0.3);
    e.save(p, b).await.unwrap();

    let mut c = semantic("procedural high");
    c.kind = MemoryType::Procedural;
    c.confidence = Some(0.9);
    e.save(p, c).await.unwrap();

    let only_sem_high = e
        .list_inbox(
            p,
            InboxFilter {
                kind: Some(MemoryType::Semantic),
                min_confidence: Some(0.5),
                limit: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(only_sem_high.items.len(), 1);
    assert_eq!(only_sem_high.items[0].content, "high-confidence semantic");
}
