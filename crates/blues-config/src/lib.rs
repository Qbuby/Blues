//! blues-config
//!
//! Configuration loading chain + secret store.
//! See ARCHITECTURE.md §2.3, protocol-and-project.md §5.

use blues_core::Result;

/// Layered config loaded with precedence:
/// CLI flag > BLUES_* env > <project>/.blues/*.toml > ~/.blues/config.toml > defaults
///
/// Each layer overlays only declared fields; never replace whole objects.
#[derive(Debug, Default, Clone)]
pub struct Config {
    // TODO(task #5+): populate with concrete fields per protocol-and-project.md §5.2
}

impl Config {
    /// Load config from the standard chain. Stub for v0.1 skeleton.
    pub fn load() -> Result<Self> {
        Ok(Self::default())
    }
}

/// Secret store contract. See ARCHITECTURE.md §2.3.
#[async_trait::async_trait]
pub trait SecretStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
}
