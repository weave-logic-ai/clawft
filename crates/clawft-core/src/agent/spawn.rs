//! Subagent spawn seam (M4 — agent-initiated work).
//!
//! The conversational agent must be able to kick off work from inside a
//! turn — spawn a subagent, fan out a swarm, hand a task off — and have the
//! result flow back into the conversation as first-class ECC state. This
//! module defines the **injection seam** that makes that possible without a
//! layering violation.
//!
//! ## Layering (design D2)
//!
//! `clawft-tools` (the LLM tool world) sits *below* `clawft-weave` /
//! `clawft-service-agent` (the daemon), so a tool cannot name `AgentService`.
//! The [`SubagentSpawner`] trait inverts that dependency: the tool holds an
//! `Arc<dyn SubagentSpawner>`, and the concrete `DaemonSubagentSpawner` (which
//! *can* reach `AgentService::dispatch`) is injected at tool-registration time,
//! exactly like the existing `ConversationSink` / `chat_gate` handles. The
//! spawner holds only a `Weak` back-reference to the service, breaking the Arc
//! cycle (`AgentService` → tool registry → tool → spawner → `AgentService`).
//!
//! ## Backend tiering (design D2, K2 D2/D3)
//!
//! [`SpawnSpec::backend`] mirrors the kernel's `SpawnBackend`. M4 implements
//! only [`SpawnBackend::Conversation`] — a subagent is a daemon-hosted child
//! conversation on the same M2 commit pipeline (design D1). Every other variant
//! returns [`SpawnError::BackendNotAvailable`], exactly like
//! `AgentSupervisor::spawn`, so the tier boundary is crystallized now and wired
//! incrementally later.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Execution backend for a spawned subagent.
///
/// Mirrors the kernel's `SpawnBackend` (`clawft-kernel::supervisor`) but adds
/// [`Conversation`](SpawnBackend::Conversation) as the default M4 tier: a child
/// conversation on the daemon's multiplexed `AgentLoop`, inheriting the entire
/// M2 pipeline (durable JSONL sink, chain anchoring, `SessionTier`, gate, forest
/// commit, idle reaping). All heavier tiers are seam-only in M4 and return
/// [`SpawnError::BackendNotAvailable`].
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnBackend {
    /// Daemon-hosted child conversation (M4 default — the only implemented tier).
    #[default]
    Conversation,
    /// Tokio task with a kernel-supervised agent loop (escalation, deferred).
    Native,
    /// WASM sandbox via Wasmtime (K3).
    Wasm {
        /// Path to the compiled WASM module.
        module: String,
    },
    /// Docker/Podman container (K4).
    Container {
        /// Container image reference.
        image: String,
    },
    /// Trusted Execution Environment — SGX, TrustZone, SEV (K6+).
    Tee {
        /// Enclave type: "sgx", "trustzone", "sev".
        enclave_type: String,
    },
    /// Delegate to a remote node in the cluster (K6, mesh).
    Remote {
        /// Cluster node identifier.
        node_id: String,
    },
}

impl SpawnBackend {
    /// Short kind label for witnessing / error messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::Native => "native",
            Self::Wasm { .. } => "wasm",
            Self::Container { .. } => "container",
            Self::Tee { .. } => "tee",
            Self::Remote { .. } => "remote",
        }
    }
}

/// Optional per-child budget overrides (design D5). Any `None` field falls back
/// to the daemon's `CostBudgetConfig` default for that dimension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpawnBudget {
    /// Max cumulative input+output tokens for the child conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    /// Max cumulative USD spend for the child conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd: Option<f64>,
    /// Max cumulative LLM iterations (round-trips) for the child conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterations: Option<u32>,
}

/// A request to spawn a subagent (design D2/D4/D5/D7).
///
/// Built by the `agent_spawn` tool from the LLM's arguments plus the parent
/// conversation context the tool carries. Pure POD — no daemon dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSpec {
    /// The task the child agent should accomplish (required). The child's first
    /// user turn.
    pub goal: String,

    /// Optional persona/identity for the child, validated against the identity
    /// loader's known personas. `None` shares the parent's persona.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,

    /// Optional skills to grant the child, validated against known skills.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,

    /// When `true`, [`SubagentSpawner::spawn`] dispatches the child inline and
    /// the returned handle already reflects the completed outcome (the MVP
    /// path, design D3). When `false`, the child runs detached and the parent
    /// retrieves the result later via [`SubagentSpawner::result`].
    #[serde(rename = "await", default)]
    pub await_completion: bool,

    /// Per-child budget overrides (design D5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<SpawnBudget>,

    /// Swarm grouping id (design D7). Caller-supplied or minted by the first
    /// spawn; `task_status(group_id)` aggregates all siblings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    /// Opt-in proactive-completion flag (design D3.2, deferred/flag-gated). When
    /// set, completion *may* inject a synthetic parent turn if the parent is idle.
    #[serde(default)]
    pub notify_on_complete: bool,

    /// Requested execution backend. Defaults to [`SpawnBackend::Conversation`].
    #[serde(default)]
    pub backend: SpawnBackend,

    /// Spawn depth of this child = parent depth + 1. Refused past the WEFT-180
    /// recursion cap (design D5). The tool fills this from the parent's depth.
    #[serde(default)]
    pub depth: u32,

    /// The parent conversation id (`conv_id = P`). Used for the concurrency
    /// cap, forest edges, and the child conv id (`sub:<P>:<ulid>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_conv_id: Option<String>,

    /// Universal id of the parent turn that made the spawn call. The
    /// `TriggeredBy` (spawn) and `EvidenceFor` (completion) forest edges link
    /// the child to this turn (design D3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_turn_uid: Option<String>,

    /// The parent agent's principal id, for the spawn-time gate precheck
    /// (design D6). The child then runs under its own principal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<String>,
}

impl SpawnSpec {
    /// Construct a minimal spec for `goal`, with all optional fields defaulted.
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            persona: None,
            skills: Vec::new(),
            await_completion: false,
            budget: None,
            group_id: None,
            notify_on_complete: false,
            backend: SpawnBackend::default(),
            depth: 0,
            parent_conv_id: None,
            parent_turn_uid: None,
            parent_agent_id: None,
        }
    }
}

/// Lifecycle state of a spawned task (design D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// The child is dispatched and still working.
    Running,
    /// The child finished and produced a result.
    Completed,
    /// The child errored (LLM down, timeout, dispatch failure).
    Failed,
    /// The child hit its budget/iteration cap; a partial result may exist.
    Exhausted,
    /// The child was cancelled (explicit or parent-end cascade).
    Cancelled,
    /// The spawn was refused by governance at the tool boundary.
    Denied,
}

impl TaskStatus {
    /// Whether the task has reached a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Running)
    }

    /// Lowercase wire label (matches the serde representation).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Exhausted => "exhausted",
            Self::Cancelled => "cancelled",
            Self::Denied => "denied",
        }
    }
}

/// Handle returned by [`SubagentSpawner::spawn`].
///
/// For `await: true` the `status` already reflects the terminal outcome and
/// `outcome` carries the result; for `await: false` the `status` is
/// [`TaskStatus::Running`] and the result is fetched later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnHandle {
    /// Stable task identifier (also the `SpawnRegistry` key).
    pub task_id: String,
    /// The child conversation id (`sub:<parent>:<ulid>`).
    pub child_conv_id: String,
    /// Current status of the task.
    pub status: TaskStatus,
    /// Present when the task already reached a terminal state at spawn time
    /// (i.e. `await: true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<TaskOutcome>,
}

/// The terminal (or latest) result of a task, returned by
/// [`SubagentSpawner::result`] and [`SubagentSpawner::status`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutcome {
    /// The task this outcome belongs to.
    pub task_id: String,
    /// The child conversation id.
    pub child_conv_id: String,
    /// Status at the time of the query.
    pub status: TaskStatus,
    /// The child's final assistant text, if any (may be partial on
    /// [`TaskStatus::Exhausted`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// The child loop's finish reason (`"stop"`, `"max_iterations"`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Number of LLM iterations the child consumed.
    #[serde(default)]
    pub iterations: u32,
    /// Optional swarm group id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Error detail when `status` is [`TaskStatus::Failed`] /
    /// [`TaskStatus::Denied`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Errors from the spawn seam. Callers distinguish a governance *denial* from a
/// runtime *error* (design D6 / `execute_tool_with_guards`).
#[derive(Debug, Error)]
pub enum SpawnError {
    /// The requested backend is not implemented in this build/phase.
    #[error("spawn backend '{backend}' not available: {reason}")]
    BackendNotAvailable {
        /// Backend kind label (see [`SpawnBackend::kind`]).
        backend: String,
        /// Why it is unavailable.
        reason: String,
    },

    /// The spawn would exceed the recursion depth cap (WEFT-180, design D5).
    #[error("spawn depth {depth} exceeds max {max}")]
    DepthExceeded {
        /// The depth the child would have had.
        depth: u32,
        /// The configured cap.
        max: u32,
    },

    /// The parent conversation already has the maximum live children (design D5).
    #[error("max concurrent subagents ({max}) reached for conversation")]
    ConcurrencyExceeded {
        /// The configured per-conv cap.
        max: u32,
    },

    /// Governance denied the spawn at the tool boundary (design D6). Distinct
    /// from a runtime error so the tool can report `{denied, reason}`.
    #[error("spawn denied by governance: {reason}")]
    Denied {
        /// Human-readable denial reason from the gate.
        reason: String,
    },

    /// No task with the given id is tracked by the registry.
    #[error("task not found: {task_id}")]
    NotFound {
        /// The queried task id.
        task_id: String,
    },

    /// Dispatch into the child conversation failed (LLM down, service gone).
    #[error("subagent dispatch failed: {0}")]
    DispatchFailed(String),

    /// Feature/config disables subagent spawning entirely.
    #[error("subagent spawning is disabled")]
    Disabled,
}

/// The injected seam the `agent_spawn` / `task_status` / `task_result` /
/// `agent_message` tools call into (design D2).
///
/// Concrete impl: `DaemonSubagentSpawner` in `clawft-service-agent`, which
/// dispatches a child conversation through `AgentService::dispatch` (design D1)
/// and draws the `TriggeredBy` / `EvidenceFor` forest edges (design D3).
#[async_trait]
pub trait SubagentSpawner: Send + Sync {
    /// Spawn a subagent per `spec`. For `await: true` the returned handle is
    /// already terminal; for `await: false` it is [`TaskStatus::Running`].
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnHandle, SpawnError>;

    /// Current status/outcome of a single task, or `None` if unknown.
    async fn status(&self, task_id: &str) -> Option<TaskOutcome>;

    /// Status of every task in a swarm group (design D7).
    async fn group_status(&self, group_id: &str) -> Vec<TaskOutcome>;

    /// Fetch a task's outcome, optionally blocking up to `wait` for a running
    /// task to finish. On timeout the current (running) outcome is returned.
    async fn result(&self, task_id: &str, wait: Option<Duration>) -> Result<TaskOutcome, SpawnError>;

    /// Inject a message turn into a child conversation (A2A, design D4).
    async fn message(&self, task_id: &str, text: &str) -> Result<(), SpawnError>;

    /// Cancel a running task (explicit or parent-end cascade, design D5).
    async fn cancel(&self, task_id: &str) -> Result<(), SpawnError>;
}
