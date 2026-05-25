//! blues-cli
//!
//! CLI client. See ARCHITECTURE.md §2.11 + docs/ui/cli.md.
//!
//! v0.1 skeleton: command tree compiled but each subcommand is a stub.
//! Real wire-up to `blues-protocol` lands in task #7+.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "blues",
    about = "Blues — AI collaboration in the time domain.",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output format: plain | rich | json
    #[arg(long, global = true, default_value = "plain")]
    pub output: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Daemon lifecycle: start / stop / status
    Daemon,
    /// Project management: init / list / activate
    Project,
    /// Memory ops: query / save / inbox / consolidate
    Memory,
    /// Plan ops: create / start / pause / resume / cancel / state
    Plan,
    /// Model ops: list / route / health / usage
    Model,
    /// MCP ops: serve
    Mcp,
    /// Print build version + protocol version
    Version,
}
