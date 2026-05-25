//! Event types broadcast through daemon's event bus.
//! See protocol-and-project.md §3.3 + UI overview §3.3 + vscode.md §9.

use serde::{Deserialize, Serialize};

use crate::ids::{InboxItemId, NodeId, PlanId, ProjectId};
use crate::status::{NodeStatus, PlanStatus};

/// Discriminator used by `EventFilter` to subscribe to a subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    PlanStateChanged,
    NodeStateChanged,
    NodeOutputDelta,
    MemoryInboxAdded,
    PermissionAsk,
    PermissionResolved,
    TokenUsage,
    ProjectActivated,
    ProjectArchived,
    ActiveContextChanged,
    PlanForked,
    PlanReplayed,
    DaemonShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    PlanStateChanged {
        plan_id: PlanId,
        old: PlanStatus,
        new: PlanStatus,
    },
    NodeStateChanged {
        plan_id: PlanId,
        node_id: NodeId,
        status: NodeStatus,
        error: Option<String>,
    },
    NodeOutputDelta {
        plan_id: PlanId,
        node_id: NodeId,
        delta: String,
    },
    MemoryInboxAdded {
        project_id: ProjectId,
        item_id: InboxItemId,
        plan_id: Option<PlanId>,
        node_id: Option<NodeId>,
        summary: String,
        confidence: f32,
    },
    PermissionAsk {
        plan_id: PlanId,
        node_id: NodeId,
        capability: String,
        request_id: String,
    },
    PermissionResolved {
        plan_id: PlanId,
        node_id: NodeId,
        request_id: String,
        decision: String,
        by_client_kind: String,
    },
    TokenUsage {
        plan_id: PlanId,
        node_id: NodeId,
        provider: String,
        in_tokens: u64,
        out_tokens: u64,
        cost_usd: f64,
    },
    ProjectActivated {
        project_id: ProjectId,
    },
    ProjectArchived {
        project_id: ProjectId,
    },
    ActiveContextChanged {
        project_id: Option<ProjectId>,
        plan_id: Option<PlanId>,
        by_client_kind: String,
    },
    PlanForked {
        plan_id: PlanId,
        parent_node_id: NodeId,
        new_plan_id: PlanId,
    },
    PlanReplayed {
        plan_id: PlanId,
        from_node_id: NodeId,
    },
    DaemonShuttingDown {
        eta_ms: u64,
    },
}

#[async_trait::async_trait]
pub trait EventEmitter: Send + Sync {
    async fn emit(&self, event: Event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_serializes_snake_case() {
        let cases = [
            (EventKind::PlanStateChanged, "\"plan_state_changed\""),
            (EventKind::NodeOutputDelta, "\"node_output_delta\""),
            (EventKind::MemoryInboxAdded, "\"memory_inbox_added\""),
            (EventKind::PermissionAsk, "\"permission_ask\""),
            (EventKind::PermissionResolved, "\"permission_resolved\""),
            (EventKind::ActiveContextChanged, "\"active_context_changed\""),
            (EventKind::PlanForked, "\"plan_forked\""),
            (EventKind::PlanReplayed, "\"plan_replayed\""),
            (EventKind::DaemonShuttingDown, "\"daemon_shutting_down\""),
        ];
        for (k, want) in cases {
            assert_eq!(serde_json::to_string(&k).unwrap(), want);
        }
    }

    #[test]
    fn event_uses_internal_kind_tag() {
        let ev = Event::DaemonShuttingDown { eta_ms: 250 };
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains("\"kind\":\"daemon_shutting_down\""), "got {j}");
        assert!(j.contains("\"eta_ms\":250"));
    }

    #[test]
    fn event_plan_state_changed_roundtrip() {
        let ev = Event::PlanStateChanged {
            plan_id: PlanId::new(),
            old: PlanStatus::Pending,
            new: PlanStatus::Running,
        };
        let j = serde_json::to_string(&ev).unwrap();
        let p: Event = serde_json::from_str(&j).unwrap();
        match (ev, p) {
            (
                Event::PlanStateChanged { plan_id: a, old: o1, new: n1 },
                Event::PlanStateChanged { plan_id: b, old: o2, new: n2 },
            ) => {
                assert_eq!(a, b);
                assert_eq!(o1, o2);
                assert_eq!(n1, n2);
            }
            _ => panic!("variant changed during roundtrip"),
        }
    }

    #[test]
    fn event_memory_inbox_added_carries_optional_plan_node() {
        let ev = Event::MemoryInboxAdded {
            project_id: ProjectId::new(),
            item_id: InboxItemId::new(),
            plan_id: Some(PlanId::new()),
            node_id: Some(NodeId::new()),
            summary: "found".into(),
            confidence: 0.82,
        };
        let j = serde_json::to_string(&ev).unwrap();
        assert!(j.contains("\"plan_id\""));
        assert!(j.contains("\"node_id\""));
        let _: Event = serde_json::from_str(&j).unwrap();
    }
}
