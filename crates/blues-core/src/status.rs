//! Plan / Node lifecycle states. See ARCHITECTURE.md §2.7, docs/ui/desktop.md §4.1.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Pending,
    Running,
    Paused,
    Done,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Paused,
    Done,
    Error,
    Forked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_status_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&PlanStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&PlanStatus::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&PlanStatus::Cancelled).unwrap(), "\"cancelled\"");
    }

    #[test]
    fn node_status_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&NodeStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&NodeStatus::Forked).unwrap(), "\"forked\"");
    }

    #[test]
    fn plan_status_roundtrip_all_variants() {
        for s in [
            PlanStatus::Pending,
            PlanStatus::Running,
            PlanStatus::Paused,
            PlanStatus::Done,
            PlanStatus::Error,
            PlanStatus::Cancelled,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let p: PlanStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(s, p);
        }
    }

    #[test]
    fn node_status_roundtrip_all_variants() {
        for s in [
            NodeStatus::Pending,
            NodeStatus::Running,
            NodeStatus::Paused,
            NodeStatus::Done,
            NodeStatus::Error,
            NodeStatus::Forked,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let p: NodeStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(s, p);
        }
    }
}
