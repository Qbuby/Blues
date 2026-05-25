//! blues-core
//!
//! Shared types, traits, and errors used by every blues-* crate.
//! See ARCHITECTURE.md §2.1.
//!
//! Rule: this crate MUST NOT depend on any other blues-* crate.

pub mod error;
pub mod ids;
pub mod status;
pub mod memory;
pub mod capability;
pub mod provenance;
pub mod event;

pub use error::{BluesError, Result};
pub use ids::{EpisodeId, FactId, InboxItemId, NodeId, PlanId, ProjectId, ProjectSlug};
pub use status::{NodeStatus, PlanStatus};
pub use memory::MemoryType;
pub use capability::Capability;
pub use provenance::{Provenance, Source, Step};
pub use event::{Event, EventEmitter, EventKind};
