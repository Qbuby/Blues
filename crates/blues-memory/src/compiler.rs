//! Budget-aware context compiler.
//!
//! v0.1 strategy: pull a deep recall pool (`top_k = 4×budget/avg_block`),
//! greedily admit blocks ordered by `score`, drop duplicates by content,
//! stop when adding the next block would exceed the token budget.
//!
//! Token estimation is `chars / TOKENS_PER_CHAR_DENOM` (≈ 4 chars/token —
//! good enough for English; a real tokenizer is on the v0.2 menu).
//!
//! See ARCHITECTURE.md §4.4.

use blues_core::{ProjectId, Result};
use std::collections::HashSet;

use crate::{
    embed::Embedder,
    recall::recall,
    store::SqliteStore,
    types::{
        CompiledContext, ContextBlock, ContextRequest, MemoryQuery, MemoryResults, Scope,
    },
};

/// Avg English token ≈ 4 chars. Deliberately conservative: better to overshoot
/// the per-block estimate and admit fewer blocks than to undershoot and blow
/// the budget when a real tokenizer disagrees.
const TOKENS_PER_CHAR_DENOM: usize = 4;

/// How many candidates to pull per ~budget-token-of-content. Wider than what
/// fits so the compiler has room to dedup and reject low-relevance blocks
/// without going back to the store.
const POOL_MULTIPLIER: usize = 4;

/// Floor on the recall pool — even tiny budgets pull at least this many
/// candidates so a stray short block doesn't starve the result set.
const MIN_POOL: usize = 16;

pub async fn compile(
    store: &SqliteStore,
    embedder: &dyn Embedder,
    req: ContextRequest,
) -> Result<CompiledContext> {
    if req.token_budget == 0 {
        return Ok(CompiledContext { blocks: vec![], total_tokens: 0, omitted: 0 });
    }

    let pool_size = (req.token_budget * POOL_MULTIPLIER / 32).max(MIN_POOL);

    let MemoryResults { items } = recall(
        store,
        embedder,
        req.project,
        MemoryQuery {
            query: req.task.clone(),
            top_k: pool_size,
            scope: Scope::Project,
            kind: None,
        },
    )
    .await?;

    let mut blocks: Vec<ContextBlock> = Vec::new();
    let mut total: usize = 0;
    let mut seen: HashSet<String> = HashSet::new();
    let mut omitted: usize = 0;

    for hit in items {
        if !seen.insert(hit.content.clone()) {
            omitted += 1;
            continue;
        }
        let tokens = estimate_tokens(&hit.content);
        if total + tokens > req.token_budget {
            omitted += 1;
            continue;
        }
        total += tokens;
        blocks.push(ContextBlock {
            kind: hit.kind,
            text: hit.content,
            provenance: hit.provenance,
            tokens,
        });
    }

    Ok(CompiledContext { blocks, total_tokens: total, omitted })
}

pub fn estimate_tokens(text: &str) -> usize {
    // ceil(len / 4); +1 token of slack so we never report 0 for a non-empty
    // block. Empty strings are reported as 0.
    if text.is_empty() {
        0
    } else {
        text.chars().count().div_ceil(TOKENS_PER_CHAR_DENOM)
    }
}

/// Helper used by tests + future budget UI: how big would `n` blocks be at
/// the v0.1 estimate? Lets callers reason about budget without round-tripping.
#[allow(dead_code)]
pub fn estimate_total<I: IntoIterator<Item = S>, S: AsRef<str>>(blocks: I) -> usize {
    blocks.into_iter().map(|s| estimate_tokens(s.as_ref())).sum()
}

/// `_project` is ignored at this layer — the engine clones it into
/// `ContextRequest`. Kept here so callers reading the module skim see that
/// project scoping happens upstream in `recall`.
#[allow(dead_code)]
fn _scope_marker(_project: ProjectId) {}
