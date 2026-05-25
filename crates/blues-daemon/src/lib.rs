//! blues-daemon
//!
//! Orchestrator + gRPC server + event bus + state persistence.
//! See ARCHITECTURE.md §2.10 + §8.
//!
//! Engines are wired via `Arc<dyn Trait>` — daemon is the only place that
//! holds concrete instances of memory / model / agent / plan / sandbox / skill.
//!
//! Skeleton stage: only the public marker types live here. Each submodule
//! (server / state / eventbus / lifecycle / orchestrator) is filled in later
//! tasks (#7 protocol, #11+ orchestrator).

pub mod prelude {
    pub use blues_core::{
        BluesError, Capability, EpisodeId, Event, EventEmitter, EventKind, FactId, InboxItemId,
        MemoryType, NodeId, NodeStatus, PlanId, PlanStatus, ProjectId, ProjectSlug, Provenance,
        Result,
    };
}

/// Daemon top-level configuration container. Filled out alongside
/// `blues-config` in task #5+.
#[derive(Debug, Default, Clone)]
pub struct DaemonConfig;

/// Marker for the running daemon. Concrete server / shutdown plumbing lands
/// when `blues-protocol` codegen turns on (task #7).
#[derive(Debug, Default)]
pub struct Daemon;

impl Daemon {
    pub fn new(_config: DaemonConfig) -> Self {
        Self
    }
}
