//! 巩固：v0.1 只跑 `decay`；`merge` / `procedure` 提炼留给 v0.2。
//!
//! `mode = All` 跑所有已实现的 pass —— 当前只有 decay。`Merge` / `Procedure`
//! 留 no-op 占位，让上游 CLI / daemon 可以提前接线，不需要再次破坏 API。
//!
//! See ARCHITECTURE.md §2.4 + §4 ("巩固")。

pub mod decay;

use blues_core::{ProjectId, Result};

use crate::store::SqliteStore;
use crate::types::ConsolidateMode;

pub async fn run(
    store: &SqliteStore,
    project: ProjectId,
    mode: ConsolidateMode,
) -> Result<()> {
    match mode {
        ConsolidateMode::Decay | ConsolidateMode::All => {
            decay::run(store, project)?;
        }
        ConsolidateMode::Merge | ConsolidateMode::Procedure => {
            // Reserved for v0.2 — 见 ARCHITECTURE.md §2.4。
        }
    }
    Ok(())
}
