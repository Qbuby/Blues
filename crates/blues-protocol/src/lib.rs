//! blues-protocol
//!
//! gRPC + MCP protocol layer. See ARCHITECTURE.md §2.2,
//! protocol-and-project.md §3 (gRPC) and §4 (MCP).
//!
//! - `gen` re-exports the tonic-generated code from `proto/blues.proto`.
//! - `mcp` will hold the MCP tool schema (memory only in v0.1).
//! - `BluesError` ↔ `tonic::Status` mapping per protocol-and-project.md §3.4.

use blues_core::BluesError;

/// Wire-protocol version. Bumped on breaking changes.
/// Pure additions (new RPCs / fields) keep this at 1.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("protocol mismatch: client={client}, server={server}")]
    UpgradeRequired { client: u32, server: u32 },
}

/// tonic-generated stubs for `proto/blues.proto`.
///
/// Re-exported under `blues_protocol::gen::*`; downstream crates should
/// not call `tonic::include_proto!` themselves.
pub mod gen {
    tonic::include_proto!("blues.v1");
}

pub mod mcp {
    //! MCP tool schema. See protocol-and-project.md §4.2.
    //! v0.1 exposes memory tools only; populated alongside blues-memory (#8).
}

/// Map a `BluesError` onto a `tonic::Status` per protocol-and-project.md §3.4.
pub fn status_from_error(e: &BluesError) -> tonic::Status {
    use tonic::Code;
    let (code, msg) = match e {
        BluesError::NotFound(m)         => (Code::NotFound,           m.as_str()),
        BluesError::PermissionDenied(m) => (Code::PermissionDenied,   m.as_str()),
        BluesError::Conflict(m)         => (Code::AlreadyExists,      m.as_str()),
        BluesError::InvalidArgument(m)  => (Code::InvalidArgument,    m.as_str()),
        BluesError::ProtocolMismatch(m) => (Code::FailedPrecondition, m.as_str()),
        BluesError::Unavailable(m)      => (Code::Unavailable,        m.as_str()),
        BluesError::Internal(m)         => (Code::Internal,           m.as_str()),
    };
    tonic::Status::new(code, msg)
}

/// Inverse mapping for clients receiving a `tonic::Status`.
pub fn error_from_status(s: tonic::Status) -> BluesError {
    use tonic::Code;
    let msg = s.message().to_string();
    match s.code() {
        Code::NotFound           => BluesError::NotFound(msg),
        Code::PermissionDenied   => BluesError::PermissionDenied(msg),
        Code::AlreadyExists      => BluesError::Conflict(msg),
        Code::InvalidArgument    => BluesError::InvalidArgument(msg),
        Code::FailedPrecondition => BluesError::ProtocolMismatch(msg),
        Code::Unavailable        => BluesError::Unavailable(msg),
        _                        => BluesError::Internal(msg),
    }
}

impl From<ProtocolError> for tonic::Status {
    fn from(e: ProtocolError) -> Self {
        match e {
            ProtocolError::UpgradeRequired { .. } => {
                tonic::Status::new(tonic::Code::FailedPrecondition, e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_one() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    #[test]
    fn gen_module_is_reachable() {
        let _ = gen::Empty {};
    }

    #[test]
    fn plan_status_enum_present() {
        let v = gen::PlanStatus::Pending as i32;
        assert_eq!(v, 1);
    }

    #[test]
    fn error_maps_to_tonic_codes() {
        use tonic::Code;
        assert_eq!(
            status_from_error(&BluesError::NotFound("x".into())).code(),
            Code::NotFound
        );
        assert_eq!(
            status_from_error(&BluesError::PermissionDenied("x".into())).code(),
            Code::PermissionDenied
        );
        assert_eq!(
            status_from_error(&BluesError::Conflict("x".into())).code(),
            Code::AlreadyExists
        );
        assert_eq!(
            status_from_error(&BluesError::InvalidArgument("x".into())).code(),
            Code::InvalidArgument
        );
        assert_eq!(
            status_from_error(&BluesError::ProtocolMismatch("x".into())).code(),
            Code::FailedPrecondition
        );
        assert_eq!(
            status_from_error(&BluesError::Unavailable("x".into())).code(),
            Code::Unavailable
        );
        assert_eq!(
            status_from_error(&BluesError::Internal("x".into())).code(),
            Code::Internal
        );
    }

    #[test]
    fn error_status_round_trip_preserves_kind() {
        let pairs = [
            BluesError::NotFound("a".into()),
            BluesError::PermissionDenied("b".into()),
            BluesError::Conflict("c".into()),
            BluesError::InvalidArgument("d".into()),
            BluesError::ProtocolMismatch("e".into()),
            BluesError::Unavailable("f".into()),
            BluesError::Internal("g".into()),
        ];
        for e in pairs {
            let original_msg = e.to_string();
            let payload = original_msg.split_once(": ").unwrap().1.to_string();
            let status = status_from_error(&e);
            let back = error_from_status(status);
            match &back {
                BluesError::NotFound(m)         => assert_eq!(m, &payload),
                BluesError::PermissionDenied(m) => assert_eq!(m, &payload),
                BluesError::Conflict(m)         => assert_eq!(m, &payload),
                BluesError::InvalidArgument(m)  => assert_eq!(m, &payload),
                BluesError::ProtocolMismatch(m) => assert_eq!(m, &payload),
                BluesError::Unavailable(m)      => assert_eq!(m, &payload),
                BluesError::Internal(m)         => assert_eq!(m, &payload),
            }
        }
    }
}
