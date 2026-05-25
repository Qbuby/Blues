//! Memory engine implementation.
//!
//! Wires `SqliteStore` + `dyn Embedder` behind the `MemoryEngine` trait.
//! v0.1 funnels every `save` through the inbox; `approve_inbox` is the only
//! path that actually lands a row in `facts` / `episodes`.
//!
//! 8.3+ flesh out query / compile_context / consolidate.

use std::sync::Arc;

use async_trait::async_trait;
use blues_core::{
    BluesError, FactId, InboxItemId, MemoryType, ProjectId, Provenance, Result,
};

use crate::{
    embed::Embedder,
    store::{attach_step, default_provenance, ensure_dim, InboxRow, SqliteStore},
    types::{
        CompiledContext, ConsolidateMode, ContextRequest, Edit, InboxFilter, InboxItem,
        InboxItems, MemoryQuery, MemoryRef, MemoryResults, MemoryWrite,
    },
    MemoryEngine,
};

/// v0.1 default inbox confidence when the writer didn't supply one. Picked
/// to land just above the "noise" floor so candidates are visible but never
/// auto-approved.
const DEFAULT_INBOX_CONFIDENCE: f32 = 0.5;

#[derive(Clone)]
pub struct Engine {
    store: SqliteStore,
    embedder: Arc<dyn Embedder>,
}

impl Engine {
    pub fn new(store: SqliteStore, embedder: Arc<dyn Embedder>) -> Self {
        Self { store, embedder }
    }
}

#[async_trait]
impl MemoryEngine for Engine {
    /// v0.1: every save is a candidate. Persistent storage happens later via
    /// `approve_inbox`. We attach an ingestion step to the provenance chain
    /// so the audit trail shows `memory:ingest` even before approval.
    async fn save(&self, project: ProjectId, write: MemoryWrite) -> Result<MemoryRef> {
        if write.content.trim().is_empty() {
            return Err(BluesError::InvalidArgument("memory content empty".into()));
        }
        let mut prov = write
            .provenance
            .unwrap_or_else(|| default_provenance(write.source.as_deref()));
        attach_step(&mut prov, "ingest", "memory");

        let row = InboxRow {
            id: InboxItemId::new(),
            kind: write.kind,
            content: write.content,
            confidence: write
                .confidence
                .unwrap_or(DEFAULT_INBOX_CONFIDENCE)
                .clamp(0.0, 1.0),
            provenance: prov,
            plan_id: write.plan_id,
            node_id: write.node_id,
        };
        let id = row.id;
        self.store.inbox_insert(project, &row)?;
        Ok(MemoryRef::Inbox(id))
    }

    async fn query(&self, project: ProjectId, q: MemoryQuery) -> Result<MemoryResults> {
        crate::recall::recall(&self.store, self.embedder.as_ref(), project, q).await
    }

    async fn compile_context(&self, req: ContextRequest) -> Result<CompiledContext> {
        crate::compiler::compile(&self.store, self.embedder.as_ref(), req).await
    }

    async fn list_inbox(
        &self,
        project: ProjectId,
        filter: InboxFilter,
    ) -> Result<InboxItems> {
        let rows = self.store.inbox_list(
            project,
            filter.kind,
            filter.min_confidence,
            filter.limit.unwrap_or(50),
        )?;
        let items = rows
            .into_iter()
            .map(|r| InboxItem {
                id: r.id,
                kind: r.kind,
                content: r.content,
                confidence: r.confidence,
                provenance: r.provenance,
                plan_id: r.plan_id,
                node_id: r.node_id,
            })
            .collect();
        Ok(InboxItems { items })
    }

    async fn approve_inbox(
        &self,
        item: InboxItemId,
        edit: Option<Edit>,
    ) -> Result<FactId> {
        let full = self
            .store
            .inbox_get(item)?
            .ok_or_else(|| BluesError::NotFound(format!("inbox item {item}")))?;
        let project = full.project;
        let row = full.row;

        let edit = edit.unwrap_or_default();
        let content = edit.content.unwrap_or(row.content);
        let kind = edit.kind.unwrap_or(row.kind);

        let mut prov = row.provenance;
        attach_step(&mut prov, "approve", "user");

        let emb = self.embedder.embed(&content).await?;
        ensure_dim(&emb)?;

        match kind {
            MemoryType::Semantic | MemoryType::Procedural => {
                let id = FactId::new();
                self.store
                    .fact_insert(project, id, kind, &content, &emb, &prov, row.confidence)?;
                self.store.inbox_delete(item)?;
                Ok(id)
            }
            MemoryType::Episodic => {
                // v0.1 returns FactId from approve; episodic candidates are
                // unusual on the human-approval path (they're typically auto-
                // captured), but if a caller does it, store as episode and
                // surface a NotFound-like error rather than fabricating an id.
                Err(BluesError::InvalidArgument(
                    "episodic approval requires episode promotion (v0.2)".into(),
                ))
            }
        }
    }

    async fn reject_inbox(&self, item: InboxItemId) -> Result<()> {
        self.store.inbox_delete(item)
    }

    async fn blame(&self, fact: FactId) -> Result<Provenance> {
        self.store.fact_provenance(fact)
    }

    async fn consolidate(
        &self,
        project: ProjectId,
        mode: ConsolidateMode,
    ) -> Result<()> {
        crate::consolidate::run(&self.store, project, mode).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::HashEmbedder;

    fn engine() -> Engine {
        Engine::new(
            SqliteStore::open_in_memory().unwrap(),
            Arc::new(HashEmbedder),
        )
    }

    #[tokio::test]
    async fn save_funnels_into_inbox() {
        let e = engine();
        let p = ProjectId::new();
        let r = e
            .save(
                p,
                MemoryWrite {
                    content: "auth.ts owns logout flow".into(),
                    kind: MemoryType::Semantic,
                    source: Some("agent:smart".into()),
                    confidence: Some(0.7),
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap();
        assert!(matches!(r, MemoryRef::Inbox(_)));

        let listed = e
            .list_inbox(p, InboxFilter::default())
            .await
            .unwrap();
        assert_eq!(listed.items.len(), 1);
        let item = &listed.items[0];
        assert_eq!(item.kind, MemoryType::Semantic);
        assert!((item.confidence - 0.7).abs() < 1e-5);
        // ingestion step recorded
        assert!(item
            .provenance
            .chain
            .iter()
            .any(|s| s.action == "ingest" && s.by == "memory"));
    }

    #[tokio::test]
    async fn save_rejects_empty_content() {
        let e = engine();
        let p = ProjectId::new();
        let err = e
            .save(
                p,
                MemoryWrite {
                    content: "   ".into(),
                    kind: MemoryType::Semantic,
                    source: None,
                    confidence: None,
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, BluesError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn approve_promotes_inbox_into_fact() {
        let e = engine();
        let p = ProjectId::new();
        let r = e
            .save(
                p,
                MemoryWrite {
                    content: "logout flow lives in auth.ts".into(),
                    kind: MemoryType::Semantic,
                    source: None,
                    confidence: Some(0.9),
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap();
        let MemoryRef::Inbox(item_id) = r else { panic!("expected inbox ref") };

        let fact_id = e.approve_inbox(item_id, None).await.unwrap();
        // Inbox drained
        assert!(e
            .list_inbox(p, InboxFilter::default())
            .await
            .unwrap()
            .items
            .is_empty());

        // Provenance carries both ingest + approve steps
        let prov = e.blame(fact_id).await.unwrap();
        let actions: Vec<&str> = prov.chain.iter().map(|s| s.action.as_str()).collect();
        assert!(actions.contains(&"ingest"));
        assert!(actions.contains(&"approve"));
    }

    #[tokio::test]
    async fn approve_with_edit_overrides_content_and_kind() {
        let e = engine();
        let p = ProjectId::new();
        let MemoryRef::Inbox(item) = e
            .save(
                p,
                MemoryWrite {
                    content: "draft".into(),
                    kind: MemoryType::Semantic,
                    source: None,
                    confidence: None,
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        let fact = e
            .approve_inbox(
                item,
                Some(Edit {
                    content: Some("polished".into()),
                    kind: Some(MemoryType::Procedural),
                }),
            )
            .await
            .unwrap();
        let prov = e.blame(fact).await.unwrap();
        // Stored under procedural kind; verify by direct store read
        let row = e.store.fact_get(fact).unwrap();
        assert_eq!(row.kind, MemoryType::Procedural);
        assert_eq!(row.content, "polished");
        assert!(prov.chain.iter().any(|s| s.action == "approve"));
    }

    #[tokio::test]
    async fn reject_drops_inbox_item() {
        let e = engine();
        let p = ProjectId::new();
        let MemoryRef::Inbox(item) = e
            .save(
                p,
                MemoryWrite {
                    content: "x".into(),
                    kind: MemoryType::Semantic,
                    source: None,
                    confidence: None,
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        e.reject_inbox(item).await.unwrap();
        let err = e.reject_inbox(item).await.unwrap_err();
        assert!(matches!(err, BluesError::NotFound(_)));
    }

    #[tokio::test]
    async fn approve_unknown_inbox_is_not_found() {
        let e = engine();
        let err = e.approve_inbox(InboxItemId::new(), None).await.unwrap_err();
        assert!(matches!(err, BluesError::NotFound(_)));
    }

    async fn save_and_approve(
        e: &Engine,
        p: ProjectId,
        content: &str,
        kind: MemoryType,
    ) -> FactId {
        let MemoryRef::Inbox(item) = e
            .save(
                p,
                MemoryWrite {
                    content: content.into(),
                    kind,
                    source: None,
                    confidence: Some(0.9),
                    plan_id: None,
                    node_id: None,
                    provenance: None,
                },
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        e.approve_inbox(item, None).await.unwrap()
    }

    #[tokio::test]
    async fn query_finds_fts_match() {
        let e = engine();
        let p = ProjectId::new();
        let target = save_and_approve(&e, p, "logout flow lives in auth.ts", MemoryType::Semantic).await;
        let _other = save_and_approve(&e, p, "billing logic lives in pay.ts", MemoryType::Semantic).await;

        let r = e
            .query(
                p,
                MemoryQuery {
                    query: "logout".into(),
                    top_k: 5,
                    scope: crate::types::Scope::Project,
                    kind: None,
                },
            )
            .await
            .unwrap();
        assert!(!r.items.is_empty());
        match r.items[0].id {
            crate::types::HitId::Fact(id) => assert_eq!(id, target),
            _ => panic!("expected fact hit"),
        }
    }

    #[tokio::test]
    async fn query_kind_filter_excludes_other_kinds() {
        let e = engine();
        let p = ProjectId::new();
        let _proc = save_and_approve(
            &e,
            p,
            "always run cargo fmt before commit",
            MemoryType::Procedural,
        )
        .await;
        let sem = save_and_approve(&e, p, "cargo workspace lives at /blues", MemoryType::Semantic).await;

        let r = e
            .query(
                p,
                MemoryQuery {
                    query: "cargo".into(),
                    top_k: 5,
                    scope: crate::types::Scope::Project,
                    kind: Some(MemoryType::Semantic),
                },
            )
            .await
            .unwrap();
        assert!(!r.items.is_empty());
        for hit in &r.items {
            assert_eq!(hit.kind, MemoryType::Semantic);
        }
        match r.items[0].id {
            crate::types::HitId::Fact(id) => assert_eq!(id, sem),
            _ => panic!(),
        }
    }

    #[tokio::test]
    async fn query_other_project_returns_empty() {
        let e = engine();
        let p1 = ProjectId::new();
        let p2 = ProjectId::new();
        let _ = save_and_approve(&e, p1, "logout flow lives in auth.ts", MemoryType::Semantic).await;
        let r = e
            .query(
                p2,
                MemoryQuery {
                    query: "logout".into(),
                    top_k: 5,
                    scope: crate::types::Scope::Project,
                    kind: None,
                },
            )
            .await
            .unwrap();
        assert!(r.items.is_empty());
    }

    #[tokio::test]
    async fn compile_context_zero_budget_is_empty() {
        let e = engine();
        let p = ProjectId::new();
        let _ = save_and_approve(&e, p, "anything", MemoryType::Semantic).await;
        let cc = e
            .compile_context(crate::types::ContextRequest {
                project: p,
                task: "anything".into(),
                token_budget: 0,
                model_id: None,
            })
            .await
            .unwrap();
        assert!(cc.blocks.is_empty());
        assert_eq!(cc.total_tokens, 0);
    }

    #[tokio::test]
    async fn compile_context_admits_within_budget() {
        let e = engine();
        let p = ProjectId::new();
        let _ = save_and_approve(&e, p, "logout flow lives in auth.ts", MemoryType::Semantic).await;
        let _ = save_and_approve(&e, p, "logout cleanup hits revoke endpoint", MemoryType::Semantic).await;
        let cc = e
            .compile_context(crate::types::ContextRequest {
                project: p,
                task: "logout".into(),
                token_budget: 4096,
                model_id: None,
            })
            .await
            .unwrap();
        assert_eq!(cc.blocks.len(), 2);
        assert!(cc.total_tokens > 0);
        assert!(cc.total_tokens <= 4096);
        // every block keeps its provenance trail (`ingest` + `approve`)
        for b in &cc.blocks {
            assert!(b.provenance.chain.iter().any(|s| s.action == "ingest"));
        }
    }

    #[tokio::test]
    async fn compile_context_respects_tight_budget() {
        let e = engine();
        let p = ProjectId::new();
        // each block is ~10 chars => ~3 tokens. Budget 3 fits exactly one.
        let _ = save_and_approve(&e, p, "logout one", MemoryType::Semantic).await;
        let _ = save_and_approve(&e, p, "logout two", MemoryType::Semantic).await;
        let _ = save_and_approve(&e, p, "logout three", MemoryType::Semantic).await;
        let cc = e
            .compile_context(crate::types::ContextRequest {
                project: p,
                task: "logout".into(),
                token_budget: 3,
                model_id: None,
            })
            .await
            .unwrap();
        assert!(cc.total_tokens <= 3);
        // at least one had to be dropped
        assert!(cc.omitted >= 1);
    }

    #[tokio::test]
    async fn compile_context_dedups_identical_content() {
        let e = engine();
        let p = ProjectId::new();
        let _ = save_and_approve(&e, p, "twin block", MemoryType::Semantic).await;
        let _ = save_and_approve(&e, p, "twin block", MemoryType::Semantic).await;
        let cc = e
            .compile_context(crate::types::ContextRequest {
                project: p,
                task: "twin".into(),
                token_budget: 4096,
                model_id: None,
            })
            .await
            .unwrap();
        assert_eq!(cc.blocks.len(), 1);
        assert_eq!(cc.omitted, 1);
    }

    #[tokio::test]
    async fn consolidate_decay_lowers_fact_confidence() {
        let e = engine();
        let p = ProjectId::new();
        let fact = save_and_approve(&e, p, "decay me", MemoryType::Semantic).await;
        let before = e.store.fact_get(fact).unwrap().confidence;
        e.consolidate(p, ConsolidateMode::Decay).await.unwrap();
        let after = e.store.fact_get(fact).unwrap().confidence;
        assert!(after < before, "expected decay; before={before} after={after}");
    }

    #[tokio::test]
    async fn consolidate_merge_and_procedure_are_noops_in_v01() {
        let e = engine();
        let p = ProjectId::new();
        let fact = save_and_approve(&e, p, "untouched", MemoryType::Semantic).await;
        let before = e.store.fact_get(fact).unwrap().confidence;
        e.consolidate(p, ConsolidateMode::Merge).await.unwrap();
        e.consolidate(p, ConsolidateMode::Procedure).await.unwrap();
        let after = e.store.fact_get(fact).unwrap().confidence;
        assert_eq!(before, after);
    }
}
