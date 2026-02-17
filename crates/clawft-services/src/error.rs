//! Service error types.

use thiserror::Error;

/// Errors produced by services in this crate.
#[derive(Error, Debug)]
pub enum ServiceError {
    /// A cron expression could not be parsed.
    #[error("invalid cron expression: {0}")]
    InvalidCronExpression(String),

    /// The requested job was not found.
    #[error("job not found: {0}")]
    JobNotFound(String),

    /// A job with the given name already exists.
    #[error("duplicate job name: {0}")]
    DuplicateJobName(String),

    /// MCP transport-layer failure.
    #[error("mcp transport error: {0}")]
    McpTransport(String),

    /// MCP protocol-layer failure (JSON-RPC error).
    #[error("mcp protocol error: {0}")]
    McpProtocol(String),

    /// Underlying I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization / deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// An internal channel was closed unexpectedly.
    #[error("channel closed")]
    ChannelClosed,
}

/// Convenience alias for results in this crate.
pub type Result<T> = std::result::Result<T, ServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ServiceError::InvalidCronExpression("bad".into());
        assert_eq!(err.to_string(), "invalid cron expression: bad");

        let err = ServiceError::JobNotFound("abc".into());
        assert_eq!(err.to_string(), "job not found: abc");

        let err = ServiceError::DuplicateJobName("daily".into());
        assert_eq!(err.to_string(), "duplicate job name: daily");

        let err = ServiceError::McpTransport("connection refused".into());
        assert_eq!(err.to_string(), "mcp transport error: connection refused");

        let err = ServiceError::McpProtocol("method not found".into());
        assert_eq!(err.to_string(), "mcp protocol error: method not found");

        let err = ServiceError::ChannelClosed;
        assert_eq!(err.to_string(), "channel closed");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: ServiceError = io_err.into();
        assert!(matches!(err, ServiceError::Io(_)));
    }

    #[test]
    fn error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("{{bad}}").unwrap_err();
        let err: ServiceError = json_err.into();
        assert!(matches!(err, ServiceError::Json(_)));
    }
}
