//! Public DTOs for the memory engine.
//!
//! These mirror the `MemoryEngine` trait surface so callers (daemon, CLI,
//! MCP) can speak in stable types regardless of storage backend.
//!
//! See ARCHITECTURE.md §2.4 + §4.

use blues_core::{
    EpisodeId, FactId, InboxItemId, MemoryType, NodeId, PlanId, ProjectId, Provenance,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWrite {
    pub content: String,
    pub kind: MemoryType,
    pub source: Option<String>,
    pub confidence: Option<f32>,
    pub plan_id: Option<PlanId>,
    pub node_id: Option<NodeId>,
    /// Initial provenance chain attached by the caller. Engine appends its
    /// own ingestion step on top.
    #[serde(default)]
    pub provenance: Option<Provenance>,
}

/// What `save` returns. v0.1 funnels everything through the inbox unless the
/// caller marks the write as already-confirmed (reserved for v0.2 `--no-inbox`),
/// so most paths produce `Inbox(_)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryRef {
    Inbox(InboxItemId),
    Fact(FactId),
    Episode(EpisodeId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query: String,
    pub top_k: usize,
    pub scope: Scope,
    pub kind: Option<MemoryType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Project,
    Linked,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResults {
    pub items: Vec<MemoryHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHit {
    pub score: f32,
    pub kind: MemoryType,
    pub content: String,
    pub provenance: Provenance,
    /// Stable id of the underlying confirmed record. Either fact or episode.
    pub id: HitId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HitId {
    Fact(FactId),
    Episode(EpisodeId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub project: ProjectId,
    pub task: String,
    pub token_budget: usize,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContext {
    pub blocks: Vec<ContextBlock>,
    pub total_tokens: usize,
    pub omitted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    pub kind: MemoryType,
    pub text: String,
    pub provenance: Provenance,
    pub tokens: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InboxFilter {
    pub kind: Option<MemoryType>,
    pub min_confidence: Option<f32>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItems {
    pub items: Vec<InboxItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    pub id: InboxItemId,
    pub kind: MemoryType,
    pub content: String,
    pub confidence: f32,
    pub provenance: Provenance,
    pub plan_id: Option<PlanId>,
    pub node_id: Option<NodeId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Edit {
    pub content: Option<String>,
    pub kind: Option<MemoryType>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidateMode {
    Decay,
    Merge,
    Procedure,
    All,
}
