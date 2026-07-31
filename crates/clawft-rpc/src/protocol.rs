//! Wire protocol for daemon <-> client communication.
//!
//! Uses line-delimited JSON over a local transport:
//! - **Unix**: Unix domain socket (`kernel.sock` under the runtime dir)
//! - **Windows**: named pipe derived from the same logical path (WEFT-11)
//!
//! Each message is a single JSON object terminated by `\n`.
//!
//! This protocol is intentionally simple and transport-agnostic —
//! the same types could be serialized over WebSocket, TCP, or
//! `postMessage` for browser contexts.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Default socket path (relative to config dir).
///
/// On Unix this is a real filesystem path component. On Windows it is
/// the *logical* name used to derive the named-pipe path via
/// [`pipe_name_for_path`]; PID/log files still live on the filesystem.
pub const SOCKET_NAME: &str = "kernel.sock";

/// Windows named-pipe name prefix (WEFT-11).
///
/// Full pipe paths look like `\\.\pipe\clawft-kernel-<hash>`. The hash
/// is derived from the logical `socket_path()` so project-local
/// runtimes stay isolated the same way UDS paths do on Unix.
pub const PIPE_NAME_PREFIX: &str = r"\\.\pipe\clawft-kernel";

/// PID file name.
pub const PID_FILE_NAME: &str = "kernel.pid";

/// Log file name.
pub const LOG_FILE_NAME: &str = "kernel.log";

/// Resolve the WeftOS runtime directory.
///
/// Resolution order:
/// 1. `WEFTOS_RUNTIME_DIR` environment variable (explicit override)
/// 2. `.weftos/runtime/` in the nearest ancestor with a `.weftos/` directory
///    (project-local kernel — allows multiple kernels per machine)
/// 3. `~/.clawft/` global fallback (single shared kernel)
///
/// This enables per-project kernels: each project with a `.weftos/` directory
/// gets its own socket, PID file, and log file.
pub fn runtime_dir() -> PathBuf {
    // 1. Explicit override
    if let Ok(dir) = std::env::var("WEFTOS_RUNTIME_DIR") {
        return PathBuf::from(dir);
    }

    // 2. Walk up from CWD looking for .weftos/
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.as_path();
        loop {
            let candidate = dir.join(".weftos");
            if candidate.is_dir() {
                return candidate.join("runtime");
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => break,
            }
        }
    }

    // 3. Global fallback
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".clawft")
}

/// Resolve the full *logical* socket path.
///
/// On Unix this is the UDS filesystem path. On Windows this remains a
/// path under [`runtime_dir`] (for PID/log co-location and hermetic
/// tests); the client maps it to a named-pipe name via
/// [`pipe_name_for_path`] before dialing.
pub fn socket_path() -> PathBuf {
    runtime_dir().join(SOCKET_NAME)
}

/// Map a logical socket path to a Windows named-pipe path (WEFT-11).
///
/// Named pipes must live under `\\.\pipe\`. Paths that already use that
/// prefix (or the forward-slash form `//./pipe/`) are returned as-is.
/// Otherwise a stable hash of the logical path is appended to
/// [`PIPE_NAME_PREFIX`] so project-local runtimes stay isolated.
///
/// Safe to call on every platform (used by unit tests and by docs).
pub fn pipe_name_for_path(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let s = path.to_string_lossy();
    if s.starts_with(r"\\.\pipe\") || s.starts_with("//./pipe/") {
        return s.into_owned();
    }
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!(r"{PIPE_NAME_PREFIX}-{:016x}", hasher.finish())
}

/// Named-pipe path for the default daemon endpoint (Windows).
///
/// Equivalent to `pipe_name_for_path(socket_path())`.
pub fn default_pipe_name() -> String {
    pipe_name_for_path(socket_path())
}

/// Resolve the PID file path.
pub fn pid_path() -> PathBuf {
    runtime_dir().join(PID_FILE_NAME)
}

/// Resolve the log file path.
pub fn log_path() -> PathBuf {
    runtime_dir().join(LOG_FILE_NAME)
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

    /// Optional bearer token for per-method capability gating
    /// (WEFT-479). When absent or empty, the daemon treats the
    /// caller as anonymous and only `Read` / `Chat` verbs succeed.
    /// When present, the daemon validates against
    /// `AuthService::validate_auth_token` and grants the token's
    /// scopes; an invalid token denies every gated verb.
    ///
    /// Wire format: any string (typically the `token_id` returned
    /// by `AuthService::authenticate`). The field is added with a
    /// serde default so existing clients remain wire-compatible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
}

impl Request {
    /// Create a request with no parameters.
    pub fn new(method: &str) -> Self {
        Self {
            method: method.to_owned(),
            params: serde_json::Value::Null,
            id: None,
            auth: None,
        }
    }

    /// Create a request with parameters.
    pub fn with_params(method: &str, params: serde_json::Value) -> Self {
        Self {
            method: method.to_owned(),
            params,
            id: None,
            auth: None,
        }
    }

    /// Attach a bearer token to this request (WEFT-479).
    ///
    /// The daemon will use the token to look up the caller's
    /// effective capabilities via the kernel `AuthService`.
    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(token.into());
        self
    }
}

// ── Responses ──────────────────────────────────────────────

/// A response from daemon to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Whether the request succeeded.
    pub ok: bool,

    /// Result data (if ok).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error message (if not ok).
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

    /// Unwrap the result or bail with the error message.
    pub fn into_result(self) -> anyhow::Result<serde_json::Value> {
        if self.ok {
            Ok(self.result.unwrap_or_default())
        } else {
            anyhow::bail!("{}", self.error.unwrap_or_else(|| "unknown error".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_new() {
        let req = Request::new("kernel.status");
        assert_eq!(req.method, "kernel.status");
        assert!(req.params.is_null());
    }

    #[test]
    fn request_with_params() {
        let req = Request::with_params("agent.spawn", serde_json::json!({"agent_id": "test"}));
        assert_eq!(req.method, "agent.spawn");
        assert_eq!(req.params["agent_id"], "test");
    }

    #[test]
    fn response_success() {
        let resp = Response::success(serde_json::json!({"status": "ok"}));
        assert!(resp.ok);
        assert!(resp.into_result().is_ok());
    }

    #[test]
    fn response_error() {
        let resp = Response::error("something broke");
        assert!(!resp.ok);
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("something broke"));
    }

    #[test]
    fn pipe_name_for_path_passes_through_pipe_prefix() {
        let raw = r"\\.\pipe\clawft-kernel-test";
        assert_eq!(pipe_name_for_path(raw), raw);
        let fwd = "//./pipe/clawft-kernel-test";
        assert_eq!(pipe_name_for_path(fwd), fwd);
    }

    #[test]
    fn pipe_name_for_path_is_stable_and_under_pipe_namespace() {
        let a = pipe_name_for_path("/tmp/project-a/.weftos/runtime/kernel.sock");
        let b = pipe_name_for_path("/tmp/project-a/.weftos/runtime/kernel.sock");
        let c = pipe_name_for_path("/tmp/project-b/.weftos/runtime/kernel.sock");
        assert_eq!(a, b, "same logical path must yield the same pipe name");
        assert_ne!(a, c, "different runtime paths must not collide");
        assert!(
            a.starts_with(r"\\.\pipe\clawft-kernel-"),
            "pipe name must live under \\\\.\\pipe\\: {a}"
        );
    }

    #[test]
    fn default_pipe_name_matches_socket_path() {
        assert_eq!(default_pipe_name(), pipe_name_for_path(socket_path()));
    }
}
