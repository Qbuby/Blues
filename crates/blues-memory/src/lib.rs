//! blues-memory
//!
//! Memory engine: three cognitive layers (episodic / semantic / procedural)
//! with inbox-first write funnel and budget-aware context compiler.
//!
//! See ARCHITECTURE.md §2.4 + §4.

pub mod compiler;
pub mod consolidate;
pub mod embed;
pub mod engine;
pub mod recall;
pub mod store;
pub mod types;

use async_trait::async_trait;
use blues_core::{FactId, InboxItemId, ProjectId, Provenance, Result};

pub use engine::Engine;
pub use types::{
    CompiledContext, ConsolidateMode, ContextBlock, ContextRequest, Edit, HitId, InboxFilter,
    InboxItem, InboxItems, MemoryHit, MemoryQuery, MemoryRef, MemoryResults, MemoryWrite, Scope,
};

#[async_trait]
pub trait MemoryEngine: Send + Sync {
    async fn save(&self, project: ProjectId, write: MemoryWrite) -> Result<MemoryRef>;
    async fn query(&self, project: ProjectId, q: MemoryQuery) -> Result<MemoryResults>;
    async fn compile_context(&self, req: ContextRequest) -> Result<CompiledContext>;
    async fn list_inbox(&self, project: ProjectId, filter: InboxFilter) -> Result<InboxItems>;
    async fn approve_inbox(&self, item: InboxItemId, edit: Option<Edit>) -> Result<FactId>;
    async fn reject_inbox(&self, item: InboxItemId) -> Result<()>;
    async fn blame(&self, fact: FactId) -> Result<Provenance>;
    async fn consolidate(&self, project: ProjectId, mode: ConsolidateMode) -> Result<()>;
}
