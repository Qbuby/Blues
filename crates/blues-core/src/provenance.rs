//! Provenance: the source-trace chain attached to facts and outputs.
//! See ARCHITECTURE.md §4, BLUES_VALUES.md §3 ("追溯 / trace").

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub sources: Vec<Source>,
    pub chain: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    Chat { plan: String, node: String },
    File { path: String, line: Option<u32> },
    User { input: String },
    Mcp { client: String },
    External { uri: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub action: String,
    pub by: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn source_tagged_snake_case() {
        let s = Source::Chat { plan: "P1".into(), node: "N1".into() };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("\"kind\":\"chat\""));
        let p: Source = serde_json::from_str(&j).unwrap();
        assert_eq!(s, p);
    }

    #[test]
    fn source_all_variants_roundtrip() {
        let cases = vec![
            Source::Chat { plan: "p".into(), node: "n".into() },
            Source::File { path: "/tmp/x".into(), line: Some(42) },
            Source::File { path: "/tmp/x".into(), line: None },
            Source::User { input: "hi".into() },
            Source::Mcp { client: "claude-code".into() },
            Source::External { uri: "https://x.test".into() },
        ];
        for s in cases {
            let j = serde_json::to_string(&s).unwrap();
            let p: Source = serde_json::from_str(&j).unwrap();
            assert_eq!(s, p);
        }
    }

    #[test]
    fn step_at_serializes_as_rfc3339() {
        let step = Step {
            at: datetime!(2026-05-25 14:32:00 UTC),
            action: "extract".into(),
            by: "agent:smart".into(),
        };
        let j = serde_json::to_string(&step).unwrap();
        assert!(j.contains("2026-05-25T14:32:00Z"), "got {j}");
        let p: Step = serde_json::from_str(&j).unwrap();
        assert_eq!(step, p);
    }

    #[test]
    fn provenance_roundtrip() {
        let prov = Provenance {
            sources: vec![Source::User { input: "do x".into() }],
            chain: vec![Step {
                at: datetime!(2026-05-25 14:32:00 UTC),
                action: "ingest".into(),
                by: "daemon".into(),
            }],
        };
        let j = serde_json::to_string(&prov).unwrap();
        let p: Provenance = serde_json::from_str(&j).unwrap();
        assert_eq!(prov, p);
    }
}
