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

/// PID file name.
pub const PID_FILE_NAME: &str = "kernel.pid";

/// Log file name.
pub const LOG_FILE_NAME: &str = "kernel.log";

/// Resolve the WeftOS runtime directory (`~/.clawft/`).
pub fn runtime_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".clawft")
}

/// Resolve the full socket path (`~/.clawft/kernel.sock`).
pub fn socket_path() -> std::path::PathBuf {
    runtime_dir().join(SOCKET_NAME)
}

/// Resolve the PID file path (`~/.clawft/kernel.pid`).
pub fn pid_path() -> std::path::PathBuf {
    runtime_dir().join(PID_FILE_NAME)
}

/// Resolve the log file path (`~/.clawft/kernel.log`).
pub fn log_path() -> std::path::PathBuf {
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

// ── Cluster result types ──────────────────────────────────

/// Result of `cluster.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatusResult {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_shards: usize,
    pub active_shards: usize,
    pub consensus_enabled: bool,
}

/// A single node entry for `cluster.nodes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNodeInfo {
    pub node_id: String,
    pub name: String,
    pub platform: String,
    pub state: String,
    pub address: Option<String>,
    pub last_seen: String,
}

/// A single shard entry for `cluster.shards`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterShardInfo {
    pub shard_id: u32,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub vector_count: usize,
    pub status: String,
}

/// Parameters for `cluster.join`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterJoinParams {
    /// Address of the node to join (for native nodes).
    #[serde(default)]
    pub address: Option<String>,
    /// Platform type: "native", "browser", "edge", "wasi".
    #[serde(default = "default_platform")]
    pub platform: String,
    /// Node display name.
    #[serde(default)]
    pub name: Option<String>,
}

fn default_platform() -> String {
    "native".into()
}

/// Parameters for `cluster.leave`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterLeaveParams {
    /// Node ID to remove from the cluster.
    pub node_id: String,
}

// ── Chain result types ────────────────────────────────────

/// Result of `chain.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatusResult {
    pub chain_id: u32,
    pub sequence: u64,
    pub event_count: usize,
    pub checkpoint_count: usize,
    pub events_since_checkpoint: u64,
    pub last_hash: String,
}

/// A single chain event for `chain.local`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEventInfo {
    pub sequence: u64,
    pub chain_id: u32,
    pub timestamp: String,
    pub source: String,
    pub kind: String,
    pub hash: String,
    /// Condensed payload summary (e.g. "pid=2 agent=worker-1").
    #[serde(default)]
    pub detail: String,
}

/// Parameters for `chain.local`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainLocalParams {
    /// Number of recent events to return (0 = all).
    #[serde(default)]
    pub count: usize,
}

/// Result of `chain.verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerifyResult {
    pub valid: bool,
    pub event_count: usize,
    pub errors: Vec<String>,
    /// Ed25519 signature verification: None=unsigned, Some(true)=valid, Some(false)=invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_verified: Option<bool>,
}

// ── Resource tree result types ────────────────────────────

/// Result of `resource.stats`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatsResult {
    pub total_nodes: usize,
    pub root_hash: String,
    pub namespaces: usize,
    pub services: usize,
    pub agents: usize,
    pub devices: usize,
}

/// A single resource node for `resource.tree` / `resource.inspect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNodeInfo {
    pub id: String,
    pub kind: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub metadata: serde_json::Value,
    pub merkle_hash: String,
    /// Optional 6-dimension scoring vector (present when scoring exists).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoring: Option<ResourceScoreResult>,
}

/// Parameters for `resource.inspect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInspectParams {
    /// Resource path to inspect.
    pub path: String,
}

// ── Agent result types ───────────────────────────────────

/// Parameters for `agent.spawn`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnParams {
    /// Unique identifier for the agent.
    pub agent_id: String,
    /// Optional parent PID.
    #[serde(default)]
    pub parent_pid: Option<u64>,
}

/// Result of `agent.spawn`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpawnResult {
    pub pid: u64,
    pub agent_id: String,
}

/// Parameters for `agent.stop`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStopParams {
    /// PID of the agent to stop.
    pub pid: u64,
    /// Whether to stop gracefully (default: true).
    #[serde(default = "default_true")]
    pub graceful: bool,
}

fn default_true() -> bool {
    true
}

/// Parameters for `agent.restart`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRestartParams {
    /// PID of the agent to restart.
    pub pid: u64,
}

/// Full inspection result for `agent.inspect`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInspectResult {
    pub pid: u64,
    pub agent_id: String,
    pub state: String,
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub messages_sent: u64,
    pub tool_calls: u64,
    pub topics: Vec<String>,
    pub parent_pid: Option<u64>,
    pub can_spawn: bool,
    pub can_ipc: bool,
    pub can_exec_tools: bool,
    pub can_network: bool,
}

/// Parameters for `agent.send`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSendParams {
    /// Target PID.
    pub pid: u64,
    /// Text message to send.
    pub message: String,
}

// ── Chain export types ───────────────────────────────────

/// Parameters for `chain.export`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainExportParams {
    /// Export format: "json" or "rvf".
    #[serde(default = "default_export_format")]
    pub format: String,
    /// Output file path (daemon-side, used for "rvf" format).
    #[serde(default)]
    pub output: Option<String>,
}

fn default_export_format() -> String {
    "json".into()
}

// ── Cron result types ────────────────────────────────────

/// Parameters for `cron.add`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronAddParams {
    /// Human-readable name for the job.
    pub name: String,
    /// Fire every N seconds.
    pub interval_secs: u64,
    /// Command payload to send.
    pub command: String,
    /// Target agent PID (optional).
    #[serde(default)]
    pub target_pid: Option<u64>,
}

/// Parameters for `cron.remove`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRemoveParams {
    /// Job ID to remove.
    pub id: String,
}

/// A single cron job entry for `cron.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobInfo {
    pub id: String,
    pub name: String,
    pub interval_secs: u64,
    pub command: String,
    pub target_pid: Option<u64>,
    pub enabled: bool,
    pub fire_count: u64,
    pub last_fired: Option<String>,
    pub created_at: String,
}

// ── IPC result types ─────────────────────────────────────

/// A topic entry for `ipc.topics`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcTopicInfo {
    pub topic: String,
    pub subscriber_count: usize,
    pub subscribers: Vec<u64>,
}

/// Parameters for `ipc.subscribe`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSubscribeParams {
    /// PID to subscribe.
    pub pid: u64,
    /// Topic name.
    pub topic: String,
}

/// Parameters for `ipc.publish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcPublishParams {
    /// Topic name.
    pub topic: String,
    /// Message payload (text or JSON string).
    pub message: String,
}

// ── Resource scoring types ───────────────────────────────

/// Parameters for `resource.score`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceScoreParams {
    /// Resource path to score.
    pub path: String,
}

/// Scoring result for `resource.score`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceScoreResult {
    pub path: String,
    pub trust: f32,
    pub performance: f32,
    pub difficulty: f32,
    pub reward: f32,
    pub reliability: f32,
    pub velocity: f32,
    pub composite: f32,
}

/// Parameters for `resource.rank`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRankParams {
    /// Number of top-ranked nodes to return.
    #[serde(default = "default_rank_count")]
    pub count: usize,
}

fn default_rank_count() -> usize {
    10
}

/// A ranked resource entry for `resource.rank`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRankEntry {
    pub path: String,
    pub score: f32,
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
