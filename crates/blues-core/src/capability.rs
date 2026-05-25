//! Capability declarations for tool calls. See ARCHITECTURE.md §5.

use serde::{Deserialize, Serialize};

/// A capability is a side-effect category a tool may request.
/// Examples: `fs.read`, `fs.write`, `net.fetch`, `shell.exec`.
///
/// Sandbox backend (host / worktree / cubesandbox) is decoupled from
/// capability — the binding lives in `<project>/.blues/policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capability(pub String);

impl Capability {
    pub const FS_READ: &'static str = "fs.read";
    pub const FS_WRITE: &'static str = "fs.write";
    pub const NET_FETCH: &'static str = "net.fetch";
    pub const SHELL_EXEC: &'static str = "shell.exec";

    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_constants_match_doc() {
        assert_eq!(Capability::FS_READ, "fs.read");
        assert_eq!(Capability::FS_WRITE, "fs.write");
        assert_eq!(Capability::NET_FETCH, "net.fetch");
        assert_eq!(Capability::SHELL_EXEC, "shell.exec");
    }

    #[test]
    fn capability_roundtrip_transparent() {
        let cap = Capability::new(Capability::FS_WRITE);
        let j = serde_json::to_string(&cap).unwrap();
        assert_eq!(j, "\"fs.write\"");
        let p: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(cap, p);
    }

    #[test]
    fn capability_display() {
        assert_eq!(Capability::new("net.fetch").to_string(), "net.fetch");
    }
}
