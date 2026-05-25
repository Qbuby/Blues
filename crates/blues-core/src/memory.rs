//! Memory layer types. See ARCHITECTURE.md §4, protocol-and-project.md §4.2.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_type_wire_format_is_snake_case() {
        assert_eq!(serde_json::to_string(&MemoryType::Episodic).unwrap(), "\"episodic\"");
        assert_eq!(serde_json::to_string(&MemoryType::Semantic).unwrap(), "\"semantic\"");
        assert_eq!(serde_json::to_string(&MemoryType::Procedural).unwrap(), "\"procedural\"");
    }

    #[test]
    fn memory_type_roundtrip() {
        for m in [MemoryType::Episodic, MemoryType::Semantic, MemoryType::Procedural] {
            let j = serde_json::to_string(&m).unwrap();
            let p: MemoryType = serde_json::from_str(&j).unwrap();
            assert_eq!(m, p);
        }
    }
}
