//! `blues` CLI binary.

use blues_cli::{Cli, Command};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            println!(
                "blues {} (protocol v{})",
                env!("CARGO_PKG_VERSION"),
                blues_protocol::PROTOCOL_VERSION
            );
        }
        other => {
            eprintln!("subcommand `{other:?}` is a skeleton — wired in task #7+");
        }
    }
    Ok(())
}
