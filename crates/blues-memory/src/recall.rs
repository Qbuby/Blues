//! Multi-route recall: vector cosine + FTS BM25 fused via Reciprocal Rank
//! Fusion. RRF is the v0.1 choice because it sidesteps score-scale calibration
//! between BM25 (negative log-prob; lower is better) and cosine (0..1; higher
//! is better). A learned re-ranker is on the v0.2 menu.
//!
//! See ARCHITECTURE.md §4.

use blues_core::{EpisodeId, FactId, MemoryType, ProjectId, Result};
use std::collections::HashMap;
use std::str::FromStr;

use crate::{
    embed::{cosine, Embedder},
    store::{ConfirmedRow, SqliteStore},
    types::{HitId, MemoryHit, MemoryQuery, MemoryResults, Scope},
};

/// RRF k-constant. 60 is the value from the original Cormack et al. paper;
/// it deliberately damps the contribution of any single ranker so adding
/// a third route later doesn't blow up the early ranks.
const RRF_K: f32 = 60.0;

/// Cap on the per-route candidate pool. We always pull more than `top_k`
/// from each route so RRF has room to disagree.
const PER_ROUTE_CAP: usize = 64;

pub async fn recall(
    store: &SqliteStore,
    embedder: &dyn Embedder,
    project: ProjectId,
    q: MemoryQuery,
) -> Result<MemoryResults> {
    // v0.1: only `Scope::Project` is honoured. Linked / Global cross-project
    // recall ships in v0.2 along with the project-link table.
    match q.scope {
        Scope::Project => {}
        Scope::Linked | Scope::Global => {
            // No-op rather than error so callers can speculatively widen scope
            // without feature-flagging — they'll just get the project subset.
        }
    }

    let limit = q.top_k.max(1);
    let pool = limit.max(PER_ROUTE_CAP);

    // ── route 1: FTS BM25 ────────────────────────────────────────────────
    let fts = store.fts_search(project, &q.query, pool)?;
    // BM25 returns `(id, kind, score)` already ordered best-first.
    let fts_ranks: HashMap<String, usize> = fts
        .iter()
        .filter(|(_, k, _)| q.kind.is_none_or(|want| *k == want))
        .enumerate()
        .map(|(i, (id, _, _))| (id.clone(), i))
        .collect();

    // ── route 2: vector cosine ──────────────────────────────────────────
    let qvec = embedder.embed(&q.query).await?;
    let mut all = store.confirmed_all(project, q.kind)?;
    let mut vec_scored: Vec<(String, MemoryType, f32)> = all
        .drain(..)
        .filter_map(|row| {
            row.embedding
                .as_ref()
                .map(|e| (row.id.clone(), row.kind, cosine(&qvec, e)))
        })
        .collect();
    vec_scored.sort_by(|a, b| b.2.total_cmp(&a.2));
    vec_scored.truncate(pool);
    let vec_ranks: HashMap<String, usize> = vec_scored
        .iter()
        .enumerate()
        .map(|(i, (id, _, _))| (id.clone(), i))
        .collect();

    // ── fuse ────────────────────────────────────────────────────────────
    let mut fused: HashMap<String, (MemoryType, f32)> = HashMap::new();
    for (id, kind, _) in &fts {
        if q.kind.is_none_or(|want| *kind == want) {
            let r = fts_ranks[id] as f32;
            fused.entry(id.clone()).or_insert((*kind, 0.0)).1 += 1.0 / (RRF_K + r);
        }
    }
    for (id, kind, _) in &vec_scored {
        let r = vec_ranks[id] as f32;
        fused.entry(id.clone()).or_insert((*kind, 0.0)).1 += 1.0 / (RRF_K + r);
    }

    let mut ranked: Vec<(String, MemoryType, f32)> =
        fused.into_iter().map(|(id, (k, s))| (id, k, s)).collect();
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2));
    ranked.truncate(limit);

    // ── hydrate ──────────────────────────────────────────────────────────
    let mut items = Vec::with_capacity(ranked.len());
    for (id, kind, score) in ranked {
        let hit = hydrate(store, &id, kind, score)?;
        items.push(hit);
    }

    Ok(MemoryResults { items })
}

fn hydrate(
    store: &SqliteStore,
    id: &str,
    kind: MemoryType,
    score: f32,
) -> Result<MemoryHit> {
    let (row, hit_id) = match kind {
        MemoryType::Semantic | MemoryType::Procedural => {
            let fact = FactId::from_str(id).map_err(|e| {
                blues_core::BluesError::Internal(format!("fact id decode: {e}"))
            })?;
            (store.fact_get(fact)?, HitId::Fact(fact))
        }
        MemoryType::Episodic => {
            let ep = EpisodeId::from_str(id).map_err(|e| {
                blues_core::BluesError::Internal(format!("episode id decode: {e}"))
            })?;
            (store.episode_get(ep)?, HitId::Episode(ep))
        }
    };
    let ConfirmedRow {
        kind, content, provenance, ..
    } = row;
    Ok(MemoryHit {
        score,
        kind,
        content,
        provenance,
        id: hit_id,
    })
}
