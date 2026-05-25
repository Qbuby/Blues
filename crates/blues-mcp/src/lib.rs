//! blues-mcp
//!
//! MCP server. Exposes memory tools to third-party MCP hosts.
//! See ARCHITECTURE.md §2.12, protocol-and-project.md §4.
//!
//! v0.1 tools: query / compile_context / save (→ inbox) / inbox_list.
//! Implementation forwards to daemon via `blues-protocol`. No business
//! logic duplication.

/// Tool names exposed by `blues mcp serve`.
pub mod tools {
    pub const QUERY: &str = "blues_memory_query";
    pub const COMPILE_CONTEXT: &str = "blues_compile_context";
    pub const SAVE: &str = "blues_memory_save";
    pub const INBOX_LIST: &str = "blues_memory_inbox_list";
}

/// Marker for the running MCP server. Concrete stdio / HTTP transport
/// lands in task #7+.
#[derive(Debug, Default)]
pub struct McpServer;
