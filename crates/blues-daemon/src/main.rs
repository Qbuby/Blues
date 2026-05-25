//! `blues-daemon` binary entry point.
//!
//! v0.1 skeleton: parse args, init tracing, then exit. The actual gRPC
//! server lights up in task #7 (protocol) + task #11 (orchestrator).

use blues_daemon::{Daemon, DaemonConfig};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("blues-daemon skeleton — server bootstraps in task #7");
    let _daemon = Daemon::new(DaemonConfig::default());
    Ok(())
}
