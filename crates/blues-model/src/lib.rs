//! blues-model
//!
//! Model router + provider abstractions. See ARCHITECTURE.md §2.5 + §6.
//!
//! v0.1 providers: kiro (OpenAI-compat) / anthropic / ollama.
//! v0.1 presets: smart (default) / economy / performance.

use async_trait::async_trait;
use blues_core::Result;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait ModelEngine: Send + Sync {
    async fn list(&self) -> Result<ModelList>;
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
    async fn usage(&self, query: UsageQuery) -> Result<UsageReport>;
    async fn health(&self, provider: &str) -> Result<bool>;
}

/// One concrete provider (e.g. an OpenAI-compatible endpoint).
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ProviderKind;
    async fn list(&self) -> Result<Vec<ModelInfo>>;
    async fn chat(&self, req: ChatRequest) -> Result<ChatStream>;
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
    async fn health(&self) -> Result<bool>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAiCompat,
    Anthropic,
    Stub,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterPreset {
    Smart,
    Economy,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub context_window: usize,
    pub supports_tools: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelList {
    pub items: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSpec>,
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

/// Stream of chat events. v0.1 task #7+ will pin a concrete async stream type;
/// the skeleton uses an opaque marker so cross-crate consumers can wire DI.
#[derive(Debug, Default)]
pub struct ChatStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub texts: Vec<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageQuery {
    pub since: Option<String>,
    pub by: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageReport {
    pub rows: Vec<UsageRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRow {
    pub bucket: String,
    pub provider: String,
    pub model: String,
    pub in_tokens: u64,
    pub out_tokens: u64,
    pub cost_usd: f64,
}
