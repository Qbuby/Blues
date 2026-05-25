//! blues-skill
//!
//! Skill loading + scheduling. See ARCHITECTURE.md §2.9.
//!
//! v0.1: Claude Code-compatible skill format, load + enable/disable + invoke
//! through `blues-sandbox` (capability-aware).
//! v0.3+: online marketplace + signature verification.

use async_trait::async_trait;
use blues_core::{Capability, ProjectId, Result};
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait SkillEngine: Send + Sync {
    async fn list(&self, project: ProjectId) -> Result<Vec<Skill>>;
    async fn install(&self, source: SkillSource) -> Result<Skill>;
    async fn enable(&self, project: ProjectId, name: &str) -> Result<()>;
    async fn disable(&self, project: ProjectId, name: &str) -> Result<()>;
    async fn invoke(&self, req: SkillInvocation) -> Result<SkillResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillSource {
    Local { path: String },
    Git { url: String, rev: Option<String> },
    Registry { name: String, version: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub project: ProjectId,
    pub skill: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub output: serde_json::Value,
}
