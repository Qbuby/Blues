//! blues-plan
//!
//! Plan Graph DAG engine. See ARCHITECTURE.md §2.7 + §3.
//!
//! v0.1: create / start / pause / resume / cancel / edit / inject / state.
//! v0.2: fork / replay / rewind / merge / diff.

use async_trait::async_trait;
use blues_core::{NodeId, NodeStatus, PlanId, PlanStatus, ProjectId, Result};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait PlanEngine: Send + Sync {
    async fn create(&self, req: CreatePlanReq) -> Result<Plan>;
    async fn start(&self, plan: PlanId) -> Result<PlanHandle>;
    async fn pause(&self, plan: PlanId) -> Result<()>;
    async fn resume(&self, plan: PlanId) -> Result<()>;
    async fn cancel(&self, plan: PlanId) -> Result<()>;
    async fn edit_node(&self, req: EditNodeReq) -> Result<Node>;
    async fn inject(&self, req: InjectReq) -> Result<()>;
    async fn state(&self, plan: PlanId) -> Result<PlanState>;

    // v0.2 surface — implementations may return BluesError::Unavailable.
    async fn fork(&self, plan: PlanId, node: NodeId) -> Result<PlanId>;
    async fn replay(&self, plan: PlanId, node: NodeId) -> Result<PlanHandle>;
    async fn rewind(&self, plan: PlanId, node: NodeId) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlanReq {
    pub project: ProjectId,
    pub intent: String,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    pub project: ProjectId,
    pub intent: String,
    pub status: PlanStatus,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub title: String,
    pub status: NodeStatus,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    LlmTask,
    Tool,
    Subgraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditNodeReq {
    pub plan: PlanId,
    pub node: NodeId,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectReq {
    pub plan: PlanId,
    pub node: NodeId,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanState {
    pub plan: Plan,
    pub running_node: Option<NodeId>,
}

/// Handle returned from `start`. Concrete async stream pinned later.
#[derive(Debug, Default)]
pub struct PlanHandle;
