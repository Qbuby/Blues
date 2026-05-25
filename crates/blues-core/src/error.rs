//! Error types. See ARCHITECTURE.md §2.1, protocol-and-project.md §3.4.

use thiserror::Error;

/// Canonical Blues error. Maps 1:1 to gRPC `tonic::Status` codes
/// (see `protocol-and-project.md` §3.4) and to CLI exit codes
/// (see `docs/ui/cli.md` §6).
#[derive(Debug, Error)]
pub enum BluesError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("protocol mismatch: {0}")]
    ProtocolMismatch(String),

    #[error("unavailable: {0}")]
    Unavailable(String),

    #[error("internal: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BluesError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_carries_message() {
        let e = BluesError::NotFound("plan/01J".into());
        assert_eq!(e.to_string(), "not found: plan/01J");
        let e = BluesError::PermissionDenied("net.fetch".into());
        assert_eq!(e.to_string(), "permission denied: net.fetch");
        let e = BluesError::ProtocolMismatch("v2 != v1".into());
        assert_eq!(e.to_string(), "protocol mismatch: v2 != v1");
    }

    #[test]
    fn result_alias_compiles() {
        fn ok() -> Result<u32> { Ok(7) }
        fn err() -> Result<u32> { Err(BluesError::Internal("boom".into())) }
        assert_eq!(ok().unwrap(), 7);
        assert!(err().is_err());
    }
}
