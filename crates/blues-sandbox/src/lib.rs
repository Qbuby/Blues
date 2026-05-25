//! blues-sandbox
//!
//! Sandbox backends + capability-based policy enforcement.
//! See ARCHITECTURE.md §2.8 + §5.
//!
//! v0.1 backends: host, worktree.
//! v0.2 backends: cubesandbox (KVM MicroVM + eBPF).

use async_trait::async_trait;
use blues_core::{Capability, Result};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait SandboxBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    async fn spawn(&self, spec: SandboxSpec) -> Result<SandboxHandle>;
    async fn exec(&self, h: &SandboxHandle, cmd: ExecCmd) -> Result<ExecResult>;
    async fn fs_op(&self, h: &SandboxHandle, op: FsOp) -> Result<FsOpResult>;
    async fn destroy(&self, h: SandboxHandle) -> Result<()>;

    // v0.2:
    async fn snapshot(&self, h: &SandboxHandle) -> Result<SnapshotId>;
    async fn restore(&self, snap: SnapshotId) -> Result<SandboxHandle>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    Host,
    Worktree,
    CubeSandbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSpec {
    pub workdir: String,
    pub allow: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxHandle {
    pub id: String,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecCmd {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FsOp {
    Read { path: String },
    Write { path: String, content: String },
    List { path: String },
    Stat { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsOpResult {
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

/// Loaded from `<project>/.blues/policy.toml`. Maps capabilities to
/// the backend that should execute them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub default: Option<BackendKind>,
    pub per_capability: std::collections::BTreeMap<String, BackendKind>,
}
