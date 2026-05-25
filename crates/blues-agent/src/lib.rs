//! blues-agent
//!
//! Single-agent ReAct executor. One agent run per plan LlmTask node.
//! See ARCHITECTURE.md §2.6 + §3 (双层编排).
//!
//! Rule: agents never directly hold engine instances; they receive
//! `Arc<dyn Trait>` via DI through this crate's request types.

use async_trait::async_trait;
use blues_core::{NodeId, PlanId, ProjectId, Result};
use blues_memory::CompiledContext;
use blues_model::ToolSpec;

#[async_trait]
pub trait AgentEngine: Send + Sync {
    async fn run(&self, req: AgentRequest) -> Result<AgentStream>;
}

#[derive(Debug)]
pub struct AgentRequest {
    pub plan_id: PlanId,
    pub node_id: NodeId,
    pub project: ProjectId,
    pub prompt: String,
    pub context: CompiledContext,
    pub tools: Vec<ToolSpec>,
    pub model_hint: Option<String>,
    pub budget: Budget,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Budget {
    pub max_tokens: Option<usize>,
    pub max_steps: Option<usize>,
    pub max_wall_secs: Option<u64>,
}

/// Stream marker; concrete async stream type pinned in task #6/#7.
#[derive(Debug, Default)]
pub struct AgentStream;
