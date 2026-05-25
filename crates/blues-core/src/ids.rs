//! Strongly-typed identifiers. ULID under the hood for sortability.
//! See ARCHITECTURE.md §2.1.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

macro_rules! ulid_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Ulid);

        impl $name {
            pub fn new() -> Self {
                Self(Ulid::new())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ulid::DecodeError;
            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ulid::from_string(s).map(Self)
            }
        }
    };
}

ulid_id!(ProjectId);
ulid_id!(PlanId);
ulid_id!(NodeId);
ulid_id!(FactId);
ulid_id!(EpisodeId);
ulid_id!(InboxItemId);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectSlug(pub String);

impl ProjectSlug {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_id_roundtrip_via_string() {
        let id = ProjectId::new();
        let s = id.to_string();
        let parsed: ProjectId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn ulid_id_roundtrip_via_serde() {
        let id = PlanId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: PlanId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn ulid_id_is_sortable_by_creation_time() {
        let a = NodeId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = NodeId::new();
        assert!(a < b, "ULIDs must be monotonically sortable");
    }

    #[test]
    fn ulid_id_serializes_as_transparent_string() {
        let id = FactId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"') && json.ends_with('"'));
        assert_eq!(json.trim_matches('"'), id.to_string());
    }

    #[test]
    fn project_slug_roundtrip() {
        let slug = ProjectSlug::new("my-project");
        let json = serde_json::to_string(&slug).unwrap();
        assert_eq!(json, "\"my-project\"");
        let parsed: ProjectSlug = serde_json::from_str(&json).unwrap();
        assert_eq!(slug, parsed);
    }

    #[test]
    fn distinct_id_types_do_not_mix() {
        let plan = PlanId::new();
        let json = serde_json::to_string(&plan).unwrap();
        let parsed_as_node: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.0, parsed_as_node.0);
    }
}
