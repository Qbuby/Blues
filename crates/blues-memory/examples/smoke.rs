//! Smoke demo for `blues-memory` v0.1.
//!
//! Drives the full `MemoryEngine` lifecycle against a tempfile sqlite store
//! and prints each step. Intended as a manual acceptance run for v0.1:
//!
//! ```text
//! cargo run --release -p blues-memory --example smoke
//! ```

use std::sync::Arc;

use blues_core::{MemoryType, ProjectId};
use blues_memory::{
    embed::HashEmbedder,
    store::SqliteStore,
    ConsolidateMode, ContextRequest, Edit, Engine, HitId, InboxFilter, MemoryEngine,
    MemoryQuery, MemoryRef, MemoryWrite, Scope,
};

fn write(content: &str, kind: MemoryType, conf: f32) -> MemoryWrite {
    MemoryWrite {
        content: content.into(),
        kind,
        source: Some("example:smoke".into()),
        confidence: Some(conf),
        plan_id: None,
        node_id: None,
        provenance: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("== blues-memory v0.1 smoke ==");

    let db_path = std::env::temp_dir().join(format!("blues-memory-smoke-{}.sqlite", std::process::id()));
    println!("sqlite : {}", db_path.display());

    let store = SqliteStore::open(&db_path)?;
    let engine = Engine::new(store, Arc::new(HashEmbedder));
    let project = ProjectId::new();
    println!("project: {project}");

    // ── 1. capture three candidates into the inbox
    println!("\n[1] save → inbox");
    let candidates = [
        ("logout flow lives in auth.ts", MemoryType::Semantic, 0.8),
        ("always run cargo fmt before commit", MemoryType::Procedural, 0.9),
        ("speculative claim about billing edge case", MemoryType::Semantic, 0.4),
    ];
    let mut item_ids = Vec::new();
    for (text, kind, conf) in candidates {
        let MemoryRef::Inbox(id) = engine.save(project, write(text, kind, conf)).await? else {
            anyhow::bail!("expected inbox ref");
        };
        println!("  · inbox {id}  conf={conf:.2}  kind={kind:?}");
        item_ids.push(id);
    }

    // ── 2. list inbox with a confidence filter (UI triage)
    println!("\n[2] list_inbox  min_confidence=0.5");
    let listing = engine
        .list_inbox(project, InboxFilter { kind: None, min_confidence: Some(0.5), limit: None })
        .await?;
    for it in &listing.items {
        println!("  · {}  conf={:.2}  '{}'", it.id, it.confidence, it.content);
    }

    // ── 3. approve two, edit one, reject one
    println!("\n[3] triage  approve(0)  approve+edit(1)  reject(2)");
    let fact_a = engine.approve_inbox(item_ids[0], None).await?;
    println!("  · approved fact {fact_a}");
    let fact_b = engine
        .approve_inbox(
            item_ids[1],
            Some(Edit { content: Some("ALWAYS run cargo fmt before commit".into()), kind: None }),
        )
        .await?;
    println!("  · approved+edited fact {fact_b}");
    engine.reject_inbox(item_ids[2]).await?;
    println!("  · rejected {}", item_ids[2]);

    // ── 4. recall
    println!("\n[4] query 'logout'  top_k=5");
    let r = engine
        .query(
            project,
            MemoryQuery {
                query: "logout".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await?;
    for hit in &r.items {
        let id = match hit.id {
            HitId::Fact(id) => id.to_string(),
            HitId::Episode(id) => id.to_string(),
        };
        println!("  · score={:.4}  kind={:?}  id={id}  '{}'", hit.score, hit.kind, hit.content);
    }

    // ── 5. compile_context with budget
    println!("\n[5] compile_context  task='logout'  budget=512");
    let cc = engine
        .compile_context(ContextRequest {
            project,
            task: "logout".into(),
            token_budget: 512,
            model_id: None,
        })
        .await?;
    println!("  blocks={}  total_tokens={}  omitted={}", cc.blocks.len(), cc.total_tokens, cc.omitted);
    for b in &cc.blocks {
        println!("  · [{:?}] tokens={} '{}'", b.kind, b.tokens, b.text);
    }

    // ── 6. blame
    println!("\n[6] blame  fact={fact_a}");
    let prov = engine.blame(fact_a).await?;
    for step in &prov.chain {
        println!("  · {}  action={}  by={}", step.at, step.action, step.by);
    }

    // ── 7. consolidate (decay)
    println!("\n[7] consolidate  All  → expect confidence ×0.95");
    engine.consolidate(project, ConsolidateMode::All).await?;
    let after = engine
        .query(
            project,
            MemoryQuery {
                query: "cargo logout".into(),
                top_k: 5,
                scope: Scope::Project,
                kind: None,
            },
        )
        .await?;
    println!("  recall after decay: {} hit(s)", after.items.len());

    // cleanup
    std::fs::remove_file(&db_path).ok();
    println!("\nOK — v0.1 acceptance flow green.");
    Ok(())
}
