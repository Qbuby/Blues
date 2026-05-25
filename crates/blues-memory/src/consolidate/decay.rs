//! 衰减 pass：每次手动调用都把所有 confirmed 行的 `confidence` 乘以 `FACTOR`。
//!
//! 0.95 ≈ 14 次 pass 后置信度对半，足够让"很久没人验证"的记忆自然沉底。
//! 真正的 nightly 调度 + 失效阈值留 v0.2（见 ARCHITECTURE.md §4 "巩固"）。

use blues_core::{ProjectId, Result};

use crate::store::SqliteStore;

/// Per-pass decay factor. 见模块 doc。
pub const FACTOR: f32 = 0.95;

pub fn run(store: &SqliteStore, project: ProjectId) -> Result<()> {
    store.decay(project, FACTOR)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::{Embedder, HashEmbedder};
    use crate::store::default_provenance;
    use blues_core::{FactId, MemoryType};

    #[tokio::test]
    async fn run_scales_confidence_by_factor() {
        let s = SqliteStore::open_in_memory().unwrap();
        let p = ProjectId::new();
        let id = FactId::new();
        let emb = HashEmbedder.embed("x").await.unwrap();
        s.fact_insert(p, id, MemoryType::Semantic, "x", &emb,
                      &default_provenance(None), 1.0).unwrap();
        run(&s, p).unwrap();
        let got = s.fact_get(id).unwrap();
        assert!((got.confidence - FACTOR).abs() < 1e-5);
    }
}
