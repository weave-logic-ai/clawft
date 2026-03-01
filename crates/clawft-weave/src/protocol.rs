//! Wire protocol for daemon <-> client communication.
//!
//! Uses line-delimited JSON over Unix domain socket.
//! Each message is a single JSON object terminated by `\n`.
//!
//! This protocol is intentionally simple and transport-agnostic —
//! the same types could be serialized over WebSocket, TCP, or
//! `postMessage` for browser contexts.

use serde::{Deserialize, Serialize};

/// Default socket path (relative to config dir).
pub const SOCKET_NAME: &str = "kernel.sock";

/// Resolve the full socket path.
///
/// Uses `~/.clawft/kernel.sock` by default.
pub fn socket_path() -> std::path::PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".clawft");
    base.join(SOCKET_NAME)
}

// ── Requests ───────────────────────────────────────────────

/// A request from client to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Method name (e.g. "kernel.status", "agent.spawn").
    pub method: String,

    /// Method parameters (may be null/empty).
    #[serde(default)]
    pub params: serde_json::Value,

    /// Optional request ID for correlation.
    #[serde(default)]
    pub id: Option<String>,
}

impl Request {
    /// Create a request with no parameters.
    pub fn new(method: &str) -> Self {
        Self {
            method: method.to_owned(),
            params: serde_json::Value::Null,
            id: None,
        }
    }

    /// Create a request with parameters.
    pub fn with_params(method: &str, params: serde_json::Value) -> Self {
        Self {
            method: method.to_owned(),
            params,
            id: None,
        }
    }
}

// ── Responses ──────────────────────────────────────────────

/// A response from daemon to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// True if the request succeeded.
    pub ok: bool,

    /// Result payload (present on success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error message (present on failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Echoed request ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Response {
    /// Create a success response.
    pub fn success(result: serde_json::Value) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
            id: None,
        }
    }

    /// Create an error response.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error: Some(msg.into()),
            id: None,
        }
    }

    /// Attach a request ID.
    pub fn with_id(mut self, id: Option<String>) -> Self {
        self.id = id;
        self
    }
}

// ── Typed result structs ───────────────────────────────────

/// Result of `kernel.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelStatusResult {
    pub state: String,
    pub uptime_secs: f64,
    pub process_count: usize,
    pub service_count: usize,
    pub max_processes: u32,
    pub health_check_interval_secs: u64,
}

/// A single process entry for `kernel.ps`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u64,
    pub agent_id: String,
    pub state: String,
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub parent_pid: Option<u64>,
}

/// A single service entry for `kernel.services`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
    pub health: String,
}

/// A single log entry for `kernel.logs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub phase: String,
    pub level: String,
    pub message: String,
}

/// Parameters for `kernel.logs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsParams {
    /// Number of recent entries to return (0 = all).
    #[serde(default)]
    pub count: usize,
    /// Minimum level filter: "debug", "info", "warn", "error".
    #[serde(default)]
    pub level: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = Request::new("kernel.status");
        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "kernel.status");
    }

    #[test]
    fn response_success_roundtrip() {
        let resp = Response::success(serde_json::json!({"state": "running"}));
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert!(parsed.ok);
        assert_eq!(parsed.result.unwrap()["state"], "running");
    }

    #[test]
    fn response_error_roundtrip() {
        let resp = Response::error("not found");
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: Response = serde_json::from_str(&json).unwrap();
        assert!(!parsed.ok);
        assert_eq!(parsed.error.unwrap(), "not found");
    }

    #[test]
    fn request_with_params() {
        let req = Request::with_params(
            "agent.spawn",
            serde_json::json!({"name": "worker-1", "role": "processor"}),
        );
        assert_eq!(req.params["name"], "worker-1");
    }

    #[test]
    fn socket_path_not_empty() {
        let path = socket_path();
        assert!(path.to_string_lossy().contains("kernel.sock"));
    }
}
