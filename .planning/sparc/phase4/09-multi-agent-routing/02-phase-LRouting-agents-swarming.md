# SPARC Task: Agent Routing Table, Per-Agent Workspace Isolation & Multi-Agent Swarming

| Field | Value |
|-------|-------|
| **Element** | 09 -- Multi-Agent Routing & Claude Flow Integration |
| **Phase** | L-Routing: L1 (Agent Routing Table) + L2 (Per-Agent Workspace Isolation) + L3 (Multi-Agent Swarming) |
| **Timeline** | L1-L2: Weeks 5-7, L3: Weeks 7-8 |
| **Priority** | High (L1/L2 enable per-user agent routing; L3 enables inter-agent collaboration) |
| **Crates** | `clawft-core` (routing.rs, agent_bus.rs), `clawft-types` (routing.rs, agent_bus.rs) |
| **Dependencies** | 08/H1 (`WorkspaceManager::ensure_agent_workspace`), 03/B5 (shared tool registry), 05/D6 (sender_id threaded through pipeline), 05/D8 (bounded bus channels) |
| **Blocks** | L4 (planning strategies rely on routing), M5 (MCP bridge uses per-agent config) |
| **Status** | Planning |

---

## 1. Overview

This phase delivers the three core components of multi-agent routing:

1. **L1 -- Agent Routing Table**: A configurable routing layer that maps inbound messages to specific agents based on channel, user_id, phone number, or chat_id. When no route matches, explicit fallback behavior (catch-all or rejection) prevents silent message loss.

2. **L2 -- Per-Agent Workspace Isolation**: Each routed agent operates in its own isolated workspace with dedicated session store, personality files (SOUL.md, AGENTS.md, USER.md), skill overrides, and MCP server config. This builds on `WorkspaceManager::ensure_agent_workspace()` from Element 08/H1.

3. **L3 -- Multi-Agent Swarming**: An `AgentBus` enables inter-agent communication via typed `InterAgentMessage` structs. A `SwarmCoordinator` dispatches subtasks to worker agents and collects results, enabling collaborative multi-agent workflows.

The routing layer sits between the existing `MessageBus` (which handles channel-to-bus message transport) and the agent loop. The `AgentRouter` intercepts each `InboundMessage` from the `MessageBus`, determines which agent should handle it, ensures the agent's workspace exists, and dispatches the message to that agent's processing loop.

---

## 2. Current Code

### 2.1 MessageBus (crates/clawft-core/src/bus.rs)

The existing `MessageBus` provides unbounded MPSC channels for channel-to-agent message flow:

**MessageBus struct** (bus.rs:38-43):
```rust
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Mutex<mpsc::UnboundedReceiver<InboundMessage>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Mutex<mpsc::UnboundedReceiver<OutboundMessage>>,
}
```

**Key methods:**
- `publish_inbound()` (bus.rs:64-73): Sends `InboundMessage` to the bus from channel adapters
- `consume_inbound()` (bus.rs:79-82): Receives the next inbound message (used by agent loop)
- `dispatch_outbound()` (bus.rs:87-96): Sends `OutboundMessage` from the agent pipeline
- `inbound_sender()` (bus.rs:111-113): Returns a cloneable sender handle for multi-producer use

The `MessageBus` has a comprehensive test suite (bus.rs:130-332, 13 tests) covering multi-producer, concurrent publish/consume, and channel closure.

### 2.2 InboundMessage (crates/clawft-types/src/event.rs)

**InboundMessage struct** (event.rs:17-41):
```rust
pub struct InboundMessage {
    pub channel: String,      // "telegram", "slack", "discord", "whatsapp"
    pub sender_id: String,    // User identifier within the channel
    pub chat_id: String,      // Chat/conversation identifier
    pub content: String,      // Message text
    pub timestamp: DateTime<Utc>,
    pub media: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

**session_key()** (event.rs:45-47): Returns `"{channel}:{chat_id}"` -- used for session lookup. The routing layer will use `channel`, `sender_id`, and `chat_id` for agent resolution.

### 2.3 OutboundMessage (crates/clawft-types/src/event.rs)

**OutboundMessage struct** (event.rs:54-76):
```rust
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub media: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### 2.4 DelegationTarget (crates/clawft-types/src/delegation.rs)

**DelegationTarget enum** (delegation.rs:88-99):
```rust
pub enum DelegationTarget {
    Local,   // Execute locally (built-in tool pipeline)
    Claude,  // Delegate to Claude AI
    Flow,    // Delegate to Claude Flow orchestration
    Auto,    // Automatically decide based on complexity heuristics
}
```

L1 routing is orthogonal to delegation -- routing determines *which agent* handles a message, while delegation determines *how* that agent processes it.

### 2.5 WorkspaceManager (crates/clawft-core/src/workspace.rs)

**WorkspaceManager struct** (workspace.rs:76-82):
```rust
pub struct WorkspaceManager {
    registry_path: PathBuf,
    registry: WorkspaceRegistry,
}
```

**Key methods for L2:**
- `ensure_agent_workspace(agent_id)` -- From Element 08/H1. Creates `~/.clawft/agents/<agent_id>/` with subdirs (`sessions/`, `memory/`, `skills/`, `tool_state/`) and default files (`SOUL.md`, `AGENTS.md`, `USER.md`, `config.toml`). Idempotent.
- `create_agent_workspace(agent_id, template)` -- Explicit creation with optional template; fails if exists.
- `delete_agent_workspace(agent_id)` -- Removes workspace with path-traversal validation.
- `list_agent_workspaces()` -- Enumerates all agent IDs.

### 2.6 AgentLoop (crates/clawft-core/src/agent/loop_core.rs)

The `AgentLoop` (loop_core.rs:1-50) implements the consume-process-respond cycle:
```
InboundMessage -> Session lookup -> ContextBuilder -> Pipeline -> Tool loop -> OutboundMessage
```

Currently, a single `AgentLoop` processes all inbound messages. L1/L2 changes this to N agent loops (one per routed agent), each with its own workspace-scoped session manager, context builder, and tool registry.

---

## 3. Specification

### 3.1 L1: Agent Routing Table

#### 3.1.1 Type Definitions

**File**: `crates/clawft-types/src/routing.rs` (new section, extend existing file)

```rust
/// Criteria for matching an inbound message to an agent route.
///
/// All non-None fields must match for the route to activate.
/// An empty `MatchCriteria` (all None) matches any message on the given channel.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchCriteria {
    /// Match messages from this specific user ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Match messages from this phone number (WhatsApp, SMS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Match messages in this specific chat/conversation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
}

/// A single route mapping a channel + criteria to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    /// Channel name to match (e.g. "telegram", "whatsapp", "slack").
    pub channel: String,

    /// Match criteria within the channel. All specified fields must match.
    #[serde(default, rename = "match")]
    pub match_criteria: MatchCriteria,

    /// Agent ID to route matching messages to.
    pub agent: String,
}

/// Result of routing an inbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingResult {
    /// A specific route matched; use this agent.
    Agent(String),
    /// No specific route matched; using the catch-all agent.
    CatchAll(String),
    /// No route matched and no catch-all is configured.
    NoMatch,
}
```

#### 3.1.2 AgentRouter

**File**: `crates/clawft-core/src/routing.rs` (new file)

```rust
/// Routes inbound messages to agents based on configured rules.
///
/// Routes are evaluated in order; first match wins.
/// If no route matches, the catch-all agent is used (if configured).
pub struct AgentRouter {
    /// Ordered list of routing rules. First match wins.
    routes: Vec<AgentRoute>,
    /// Fallback agent for unmatched messages.
    catch_all: Option<String>,
}
```

#### 3.1.3 Config Format

Routes are defined in `clawft.toml` under `[[agent_routes]]`:

```toml
# Catch-all agent for unmatched messages
[routing]
catch_all = "default-agent"

# Route telegram user 12345 to work-agent
[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "work-agent"

# Route all WhatsApp messages from +1... to personal-agent
[[agent_routes]]
channel = "whatsapp"
match = { phone = "+1234567890" }
agent = "personal-agent"

# Route all messages in a specific Slack channel to project-agent
[[agent_routes]]
channel = "slack"
match = { chat_id = "C0123456789" }
agent = "project-agent"

# Route all Discord messages to discord-agent (no criteria = match all)
[[agent_routes]]
channel = "discord"
agent = "discord-agent"
```

#### 3.1.4 Routing Fallback Behavior (Orchestrator Section 7)

| Scenario | Behavior |
|----------|----------|
| **No matching route + no catch-all** | Return `RoutingResult::NoMatch`. Log at `warn` level: "no agent configured for {channel}:{sender_id}". Reject the message -- do not silently drop. |
| **Matched agent workspace does not exist** | Auto-create via `WorkspaceManager::ensure_agent_workspace(agent_id)`. Uses `~/.clawft/agents/default/` as template if it exists; otherwise bare minimum with empty `SOUL.md`. |
| **Anonymous messages** (empty `sender_id`) | Route to catch-all agent or dedicated "anonymous" agent. Anonymous agents operate with reduced permissions (no write tools, no delegation). |

### 3.2 L2: Per-Agent Workspace Isolation

Each routed agent operates in an isolated workspace created by `WorkspaceManager::ensure_agent_workspace()` (Element 08/H1). L2 extends this with runtime initialization:

#### 3.2.1 Agent Workspace Layout

```
~/.clawft/agents/<agentId>/
  SOUL.md                  # Agent personality / system prompt
  AGENTS.md                # Agent capabilities description
  USER.md                  # User context for this agent
  config.toml              # Per-agent config overrides
  sessions/                # Session store (one file per session key)
  memory/                  # Agent-scoped memory (HNSW, key-value)
  skills/                  # Per-agent skill overrides
  tool_state/              # Plugin state (Element 04 Contract 3.1)
```

#### 3.2.2 Per-Agent Runtime Initialization

**File**: `crates/clawft-core/src/routing.rs`

When a message is routed to an agent, the `AgentRouter` must ensure the agent runtime is initialized:

```rust
/// A fully initialized agent runtime, ready to process messages.
pub struct AgentRuntime {
    /// Agent identifier.
    pub agent_id: String,
    /// Absolute path to agent workspace root.
    pub workspace_path: PathBuf,
    /// Agent-scoped session manager.
    pub session_manager: SessionManager,
    /// Agent-scoped context builder (reads agent's SOUL.md, etc.).
    pub context_builder: ContextBuilder,
    /// Agent-scoped tool registry (inherits global + agent overrides).
    pub tool_registry: ToolRegistry,
    /// Agent-scoped config (global merged with agent's config.toml).
    pub config: AgentsConfig,
}
```

#### 3.2.3 Per-Agent MCP Config Override (Contract 3.2)

Each agent can override MCP server configuration in its `config.toml`:

```toml
# Agent-specific MCP servers (merged with global config)
[mcp_servers.custom-tool]
command = "npx"
args = ["-y", "custom-mcp-server"]

# Disable a global MCP server for this agent
[mcp_servers.github]
enabled = false
```

The merge order is: global config < agent config. Disabled servers are excluded from the agent's tool registry.

#### 3.2.4 Isolation Guarantees

| Property | Guarantee |
|----------|-----------|
| **Session isolation** | Each agent's `SessionManager` reads/writes only to `~/.clawft/agents/<agentId>/sessions/`. No cross-agent session access. |
| **Memory isolation** | Each agent's memory store is scoped to `~/.clawft/agents/<agentId>/memory/`. Cross-agent sharing only via shared memory protocol (H1). |
| **Tool isolation** | Each agent gets its own `ToolRegistry` instance. Shared `ToolRegistry` builder from B5 creates per-agent instances with agent-specific overrides. |
| **Config isolation** | Per-agent `config.toml` overrides global settings. Agent cannot read other agents' configs. |
| **File permissions** | Agent workspace directories are `0700` (owner-only on Unix). |

### 3.3 L3: Inter-Agent Communication

#### 3.3.1 InterAgentMessage Type

**File**: `crates/clawft-types/src/agent_bus.rs` (new file)

```rust
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A message sent between agents via the AgentBus.
///
/// Messages are delivered to the recipient's inbox. The sender cannot
/// read the recipient's inbox or intercept delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterAgentMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// Agent ID of the sender.
    pub from_agent: String,
    /// Agent ID of the recipient.
    pub to_agent: String,
    /// Task description or intent (human-readable).
    pub task: String,
    /// Arbitrary JSON payload containing task data.
    pub payload: Value,
    /// If this is a reply, the ID of the original message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Uuid>,
    /// Time-to-live: message expires if undelivered within this duration.
    /// Default: 5 minutes.
    #[serde(default = "default_ttl")]
    pub ttl: Duration,
}

fn default_ttl() -> Duration {
    Duration::from_secs(300)
}

/// Payload types for inter-agent messages.
///
/// Supports text, structured JSON, and binary data for future
/// canvas rendering and voice capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Plain text content.
    Text(String),
    /// Structured JSON content.
    Structured(Value),
    /// Binary content with MIME type (e.g., image/png, audio/wav).
    Binary {
        mime_type: String,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
}

/// Errors that can occur on the AgentBus.
#[derive(Debug, thiserror::Error)]
pub enum AgentBusError {
    /// The target agent is not registered on the bus.
    #[error("agent not registered: {agent_id}")]
    AgentNotRegistered { agent_id: String },

    /// The target agent's inbox is full (bounded channel capacity exceeded).
    #[error("inbox full for agent: {agent_id}")]
    InboxFull { agent_id: String },

    /// The message TTL has expired before delivery.
    #[error("message expired: id={id}, ttl={ttl:?}")]
    MessageExpired { id: Uuid, ttl: Duration },

    /// The bus has been shut down.
    #[error("agent bus is shut down")]
    BusShutdown,
}
```

#### 3.3.2 AgentBus

**File**: `crates/clawft-core/src/agent_bus.rs` (new file)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use clawft_types::agent_bus::{AgentBusError, InterAgentMessage};

/// Maximum inbox capacity per agent (D8: bounded bus channels).
const DEFAULT_INBOX_CAPACITY: usize = 256;

/// Inter-agent communication bus with per-agent inboxes.
///
/// Each registered agent gets a bounded MPSC channel as its inbox.
/// Messages are delivered by the sender calling `send()`, which
/// looks up the recipient's inbox and forwards the message.
///
/// Security: Agents can only read from their own inbox receiver.
/// The bus enforces agent-scoped delivery -- there is no broadcast
/// or eavesdropping capability.
pub struct AgentBus {
    /// Map of agent_id -> inbox sender handle.
    inboxes: Arc<RwLock<HashMap<String, mpsc::Sender<InterAgentMessage>>>>,
    /// Inbox capacity for new registrations.
    inbox_capacity: usize,
}

impl AgentBus {
    /// Create a new AgentBus with default inbox capacity.
    pub fn new() -> Self {
        Self {
            inboxes: Arc::new(RwLock::new(HashMap::new())),
            inbox_capacity: DEFAULT_INBOX_CAPACITY,
        }
    }

    /// Create a new AgentBus with custom inbox capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inboxes: Arc::new(RwLock::new(HashMap::new())),
            inbox_capacity: capacity,
        }
    }

    /// Register an agent on the bus and return its inbox receiver.
    ///
    /// The receiver is the only way to read messages for this agent.
    /// Calling `register_agent` for an already-registered agent replaces
    /// the old inbox (pending messages in the old channel are dropped).
    pub async fn register_agent(
        &self,
        agent_id: &str,
    ) -> mpsc::Receiver<InterAgentMessage> {
        let (tx, rx) = mpsc::channel(self.inbox_capacity);
        let mut inboxes = self.inboxes.write().await;

        if inboxes.contains_key(agent_id) {
            warn!(
                agent_id = %agent_id,
                "re-registering agent on bus; old inbox dropped"
            );
        }

        inboxes.insert(agent_id.to_string(), tx);
        debug!(agent_id = %agent_id, "agent registered on bus");
        rx
    }

    /// Unregister an agent from the bus.
    ///
    /// Drops the agent's inbox sender, causing the receiver to return
    /// `None` on the next `recv()` after buffered messages are consumed.
    pub async fn unregister_agent(&self, agent_id: &str) {
        let mut inboxes = self.inboxes.write().await;
        if inboxes.remove(agent_id).is_some() {
            debug!(agent_id = %agent_id, "agent unregistered from bus");
        } else {
            warn!(agent_id = %agent_id, "attempted to unregister unknown agent");
        }
    }

    /// Send a message to the target agent's inbox.
    ///
    /// Fails if the target agent is not registered or its inbox is full.
    /// TTL is checked at send time; expired messages are rejected.
    pub async fn send(&self, msg: InterAgentMessage) -> Result<(), AgentBusError> {
        // TTL check: reject messages that have already expired
        // (creation time is implicit -- caller is responsible for
        //  constructing messages with valid TTL relative to now)
        if msg.ttl.is_zero() {
            return Err(AgentBusError::MessageExpired {
                id: msg.id,
                ttl: msg.ttl,
            });
        }

        let inboxes = self.inboxes.read().await;
        let tx = inboxes
            .get(&msg.to_agent)
            .ok_or_else(|| AgentBusError::AgentNotRegistered {
                agent_id: msg.to_agent.clone(),
            })?;

        tx.try_send(msg.clone()).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => AgentBusError::InboxFull {
                agent_id: msg.to_agent.clone(),
            },
            mpsc::error::TrySendError::Closed(_) => AgentBusError::AgentNotRegistered {
                agent_id: msg.to_agent.clone(),
            },
        })
    }

    /// Check if an agent is registered on the bus.
    pub async fn is_registered(&self, agent_id: &str) -> bool {
        let inboxes = self.inboxes.read().await;
        inboxes.contains_key(agent_id)
    }

    /// List all registered agent IDs.
    pub async fn registered_agents(&self) -> Vec<String> {
        let inboxes = self.inboxes.read().await;
        inboxes.keys().cloned().collect()
    }
}

impl Default for AgentBus {
    fn default() -> Self {
        Self::new()
    }
}
```

#### 3.3.3 SwarmCoordinator

**File**: `crates/clawft-core/src/agent_bus.rs` (same file, below `AgentBus`)

```rust
use tokio::time::timeout;

/// Coordinates multi-agent subtask dispatch and result collection.
///
/// The coordinator registers itself and worker agents on the `AgentBus`,
/// dispatches subtasks as `InterAgentMessage` messages, and collects
/// replies within a configurable timeout.
pub struct SwarmCoordinator {
    /// Shared reference to the agent bus.
    agent_bus: Arc<AgentBus>,
    /// The coordinator's own agent ID (for receiving replies).
    coordinator_id: String,
    /// Inbox receiver for the coordinator.
    inbox: mpsc::Receiver<InterAgentMessage>,
    /// Known worker agent IDs.
    worker_agents: Vec<String>,
}

impl SwarmCoordinator {
    /// Create a new coordinator and register it on the bus.
    pub async fn new(
        agent_bus: Arc<AgentBus>,
        coordinator_id: String,
        worker_agents: Vec<String>,
    ) -> Self {
        let inbox = agent_bus.register_agent(&coordinator_id).await;
        Self {
            agent_bus,
            coordinator_id,
            inbox,
            worker_agents,
        }
    }

    /// Dispatch a subtask to a specific worker agent.
    ///
    /// Returns the message ID for correlating the reply.
    pub async fn dispatch_subtask(
        &self,
        task: &str,
        worker: &str,
        payload: Value,
    ) -> Result<Uuid, AgentBusError> {
        let id = Uuid::new_v4();
        let msg = InterAgentMessage {
            id,
            from_agent: self.coordinator_id.clone(),
            to_agent: worker.to_string(),
            task: task.to_string(),
            payload,
            reply_to: None,
            ttl: Duration::from_secs(300),
        };
        self.agent_bus.send(msg).await?;
        debug!(
            coordinator = %self.coordinator_id,
            worker = %worker,
            task = %task,
            msg_id = %id,
            "dispatched subtask"
        );
        Ok(id)
    }

    /// Collect results from worker agents within a timeout.
    ///
    /// Returns a vec of (worker_agent_id, result_payload) pairs.
    /// Messages that arrive after the timeout are left in the inbox
    /// for subsequent `collect_results` calls.
    pub async fn collect_results(
        &mut self,
        deadline: Duration,
    ) -> Vec<(String, Value)> {
        let mut results = Vec::new();
        let start = tokio::time::Instant::now();

        loop {
            let remaining = deadline.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            match timeout(remaining, self.inbox.recv()).await {
                Ok(Some(msg)) => {
                    debug!(
                        coordinator = %self.coordinator_id,
                        from = %msg.from_agent,
                        msg_id = %msg.id,
                        "received result"
                    );
                    results.push((msg.from_agent.clone(), msg.payload.clone()));
                }
                Ok(None) => {
                    // Inbox closed (bus shut down or coordinator unregistered)
                    warn!(
                        coordinator = %self.coordinator_id,
                        "inbox closed during result collection"
                    );
                    break;
                }
                Err(_) => {
                    // Timeout elapsed
                    debug!(
                        coordinator = %self.coordinator_id,
                        collected = results.len(),
                        timeout_ms = deadline.as_millis(),
                        "result collection timed out"
                    );
                    break;
                }
            }
        }

        results
    }

    /// Dispatch a task to all workers and collect results.
    ///
    /// Convenience method that sends the same task to every registered
    /// worker and waits for results with the given timeout.
    pub async fn fan_out(
        &mut self,
        task: &str,
        payload: Value,
        deadline: Duration,
    ) -> Vec<(String, Value)> {
        for worker in &self.worker_agents.clone() {
            if let Err(e) = self
                .dispatch_subtask(task, worker, payload.clone())
                .await
            {
                warn!(
                    coordinator = %self.coordinator_id,
                    worker = %worker,
                    error = %e,
                    "failed to dispatch to worker"
                );
            }
        }
        self.collect_results(deadline).await
    }
}
```

---

## 4. Pseudocode

### 4.1 L1: Message Routing Flow

```
FUNCTION route_message(msg: InboundMessage, router: AgentRouter) -> RoutingResult:
    FOR EACH route IN router.routes:
        IF route.channel != msg.channel:
            CONTINUE

        match_ok = TRUE

        IF route.match_criteria.user_id IS SOME(uid):
            IF msg.sender_id != uid:
                match_ok = FALSE

        IF route.match_criteria.phone IS SOME(phone):
            metadata_phone = msg.metadata.get("phone")
            IF metadata_phone != phone:
                match_ok = FALSE

        IF route.match_criteria.chat_id IS SOME(cid):
            IF msg.chat_id != cid:
                match_ok = FALSE

        IF match_ok:
            RETURN RoutingResult::Agent(route.agent)

    // No specific route matched
    IF router.catch_all IS SOME(agent):
        RETURN RoutingResult::CatchAll(agent)
    ELSE:
        LOG warn "no agent configured for {msg.channel}:{msg.sender_id}"
        RETURN RoutingResult::NoMatch
```

### 4.2 L1: AgentRouter Implementation

```
FUNCTION AgentRouter::new(routes, catch_all) -> AgentRouter:
    RETURN AgentRouter { routes, catch_all }

FUNCTION AgentRouter::from_config(config: Config) -> AgentRouter:
    routes = config.agent_routes  // Vec<AgentRoute> from clawft.toml
    catch_all = config.routing.catch_all
    RETURN AgentRouter::new(routes, catch_all)

FUNCTION AgentRouter::add_route(self, route: AgentRoute):
    self.routes.push(route)

FUNCTION AgentRouter::remove_routes_for_agent(self, agent_id: &str):
    self.routes.retain(|r| r.agent != agent_id)
```

### 4.3 L2: Agent Dispatch with Workspace Initialization

```
FUNCTION dispatch_to_agent(
    msg: InboundMessage,
    router: AgentRouter,
    workspace_mgr: WorkspaceManager,
    runtimes: HashMap<String, AgentRuntime>,
) -> Result<()>:

    result = router.route(&msg)

    MATCH result:
        RoutingResult::NoMatch:
            RETURN Err("no agent configured for {msg.channel}:{msg.sender_id}")

        RoutingResult::Agent(agent_id) | RoutingResult::CatchAll(agent_id):
            // Check for anonymous sender with reduced permissions
            is_anonymous = msg.sender_id.is_empty()

            // Ensure workspace exists (idempotent, from H1)
            workspace_path = workspace_mgr.ensure_agent_workspace(&agent_id)?

            // Initialize runtime if not already running
            IF agent_id NOT IN runtimes:
                runtime = initialize_agent_runtime(agent_id, workspace_path, is_anonymous)?
                runtimes.insert(agent_id, runtime)

            // Dispatch message to agent's processing loop
            runtime = runtimes.get(&agent_id)
            runtime.process_message(msg).await
```

### 4.4 L2: Agent Runtime Initialization

```
FUNCTION initialize_agent_runtime(
    agent_id: &str,
    workspace_path: PathBuf,
    is_anonymous: bool,
) -> Result<AgentRuntime>:

    // Load per-agent config (global merged with agent's config.toml)
    agent_config_path = workspace_path.join("config.toml")
    config = load_merged_agent_config(global_config, agent_config_path)?

    // Create agent-scoped session manager
    sessions_dir = workspace_path.join("sessions")
    session_manager = SessionManager::new(sessions_dir)

    // Create agent-scoped context builder
    // (reads SOUL.md, AGENTS.md, USER.md from agent workspace)
    context_builder = ContextBuilder::new(config, workspace_path)

    // Create agent-scoped tool registry
    // B5: shared tool registry builder creates per-agent instances
    tool_registry = ToolRegistryBuilder::new()
        .with_global_tools(global_tools)
        .with_agent_overrides(workspace_path.join("skills"))
        .with_mcp_overrides(config.mcp_servers)
        .build()?

    // Reduce permissions for anonymous agents
    IF is_anonymous:
        tool_registry.disable_write_tools()
        config.delegation.claude_enabled = false
        config.delegation.claude_flow_enabled = false

    RETURN AgentRuntime {
        agent_id,
        workspace_path,
        session_manager,
        context_builder,
        tool_registry,
        config,
    }
```

### 4.5 L3: Swarm Coordination Flow

```
FUNCTION run_swarm_task(
    coordinator: SwarmCoordinator,
    task: &str,
    subtasks: Vec<(String, Value)>,  // (worker_id, payload)
    timeout: Duration,
) -> Result<Vec<(String, Value)>>:

    // Dispatch subtasks to specific workers
    msg_ids = Vec::new()
    FOR (worker, payload) IN subtasks:
        id = coordinator.dispatch_subtask(task, worker, payload).await?
        msg_ids.push((worker, id))

    // Collect results with timeout
    results = coordinator.collect_results(timeout).await

    // Log any missing results
    received_workers = results.iter().map(|(w, _)| w).collect::<HashSet>()
    FOR (worker, id) IN msg_ids:
        IF worker NOT IN received_workers:
            LOG warn "no result from worker {worker} for msg {id}"

    RETURN results
```

### 4.6 L3: Worker Agent Message Processing

```
FUNCTION worker_message_loop(
    agent_id: &str,
    inbox: Receiver<InterAgentMessage>,
    agent_bus: Arc<AgentBus>,
    runtime: AgentRuntime,
):
    LOOP:
        msg = inbox.recv().await
        IF msg IS None:
            LOG info "worker {agent_id} inbox closed, shutting down"
            BREAK

        LOG debug "worker {agent_id} received task: {msg.task}"

        // Process the task using the agent's runtime
        result = runtime.process_task(msg.task, msg.payload).await

        // Send reply back to the sender
        reply = InterAgentMessage {
            id: Uuid::new_v4(),
            from_agent: agent_id,
            to_agent: msg.from_agent,
            task: format!("reply:{}", msg.task),
            payload: result,
            reply_to: Some(msg.id),
            ttl: Duration::from_secs(60),
        }

        IF let Err(e) = agent_bus.send(reply).await:
            LOG warn "worker {agent_id} failed to send reply: {e}"
```

---

## 5. Architecture

### 5.1 File Map

| File | Crate | New/Modified | Owner | Description |
|------|-------|-------------|-------|-------------|
| `crates/clawft-types/src/routing.rs` | clawft-types | Modified | L1 | Add `MatchCriteria`, `AgentRoute`, `RoutingResult` types |
| `crates/clawft-types/src/agent_bus.rs` | clawft-types | New | L3 | `InterAgentMessage`, `MessagePayload`, `AgentBusError` types |
| `crates/clawft-types/src/lib.rs` | clawft-types | Modified | L3 | Add `pub mod agent_bus;` |
| `crates/clawft-core/src/routing.rs` | clawft-core | New | L1/L2 | `AgentRouter`, `AgentRuntime`, dispatch logic |
| `crates/clawft-core/src/agent_bus.rs` | clawft-core | New | L3 | `AgentBus`, `SwarmCoordinator` |
| `crates/clawft-core/src/lib.rs` | clawft-core | Modified | L1/L3 | Add `pub mod routing;` and `pub mod agent_bus;` |
| `crates/clawft-core/src/agent/loop_core.rs` | clawft-core | Modified | L2 | Update to accept per-agent workspace context |

### 5.2 Component Interaction Diagram

```
Channel Adapters (Telegram, Slack, Discord, WhatsApp)
       |
       | InboundMessage
       v
  +-----------+
  | MessageBus |  (existing: bus.rs:38-43)
  +-----------+
       |
       | consume_inbound()
       v
  +-------------+
  | AgentRouter  |  (NEW: routing.rs)
  +-------------+
       |
       | route() -> RoutingResult
       |
       +--- Agent("work-agent") -----> +------------------+
       |                                | AgentRuntime     |
       +--- Agent("personal-agent") -> | (per-agent:       |
       |                                |  session_mgr,    |
       +--- CatchAll("default") -----> |  context_builder, |
       |                                |  tool_registry)  |
       +--- NoMatch --> warn + reject   +------------------+
                                              |
                                              | process_message()
                                              v
                                        +-----------+
                                        | AgentLoop  |  (existing: loop_core.rs)
                                        +-----------+
                                              |
                                              | OutboundMessage
                                              v
                                        +-----------+
                                        | MessageBus |
                                        +-----------+
                                              |
                                              v
                                     Channel Adapters (reply)
```

### 5.3 AgentBus Data Flow (L3)

```
+-------------------+        InterAgentMessage       +-------------------+
| SwarmCoordinator  | -----------------------------> | Worker Agent A    |
| (coordinator-1)   |    dispatch_subtask()          | (worker-a)        |
|                   |                                |                   |
|   inbox (rx) <----+--- reply (from_agent=worker-a) |   inbox (rx) <----+
+-------------------+                                +-------------------+
        |                                                    |
        | dispatch_subtask()      InterAgentMessage          |
        +------------------------------------->  +-------------------+
                                                 | Worker Agent B    |
                                                 | (worker-b)        |
                                                 |   inbox (rx) <----+
                                                 +-------------------+

                     +----------+
                     | AgentBus |  (shared, Arc-wrapped)
                     +----------+
                     | inboxes: HashMap<String, Sender<InterAgentMessage>> |
                     |                                                     |
                     | register_agent(id) -> Receiver                      |
                     | unregister_agent(id)                                 |
                     | send(msg) -> routes to target inbox                 |
                     +-----------------------------------------------------+
```

### 5.4 Cross-Element Data Flow

```
InboundMessage (event.rs:17-41)
  |
  | .channel, .sender_id, .chat_id  (D6: sender_id threaded through pipeline)
  v
AgentRouter.route()
  |
  | RoutingResult::Agent(agent_id)
  v
WorkspaceManager.ensure_agent_workspace(agent_id)  (H1: Element 08)
  |
  | PathBuf (workspace path)
  v
AgentRuntime (L2)
  |
  | Uses B5 shared ToolRegistry builder for per-agent tool instances
  | Uses D6 sender_id for auth context
  | Uses D8 bounded channels for AgentBus inboxes
  v
AgentLoop.process_message()
```

---

## 6. Refinement

### 6.1 Edge Cases

#### 6.1.1 Route Ordering and Overlap

Routes are evaluated in order; first match wins. When multiple routes could match the same message, the order in `clawft.toml` determines priority.

**Example conflict:**
```toml
# Route 1: All telegram messages to general-agent
[[agent_routes]]
channel = "telegram"
agent = "general-agent"

# Route 2: Specific telegram user to vip-agent (never reached!)
[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "vip-agent"
```

**Mitigation**: On config load, log a `warn` if a later route is strictly more specific than an earlier route on the same channel (shadow detection). Do not reorder -- respect user intent.

#### 6.1.2 Agent Hot-Swap

When routing config changes at runtime (via `clawft.toml` hot-reload), in-flight messages must complete processing with the old routing. New messages use the new routing table. Agent runtimes for orphaned routes (routes that no longer exist) are left running until idle, then gracefully shut down.

#### 6.1.3 Multi-Channel Same-Agent

A single agent can receive messages from multiple channels. For example:
```toml
[[agent_routes]]
channel = "telegram"
match = { user_id = "12345" }
agent = "personal-agent"

[[agent_routes]]
channel = "whatsapp"
match = { phone = "+1234567890" }
agent = "personal-agent"
```

Both routes dispatch to the same `AgentRuntime`. The session key (`{channel}:{chat_id}`) ensures sessions remain channel-scoped even within the same agent.

#### 6.1.4 Concurrent Message Delivery to Same Agent

Multiple inbound messages for the same agent may arrive concurrently. The `AgentRuntime` must serialize message processing (one at a time per agent) to prevent session corruption. Use a `tokio::sync::Mutex` around the agent loop's message processing, or a dedicated `mpsc` channel per agent.

#### 6.1.5 AgentBus Inbox Overflow

When a worker agent is slow or stuck, its inbox may fill up (bounded to `DEFAULT_INBOX_CAPACITY = 256`). The `send()` method returns `AgentBusError::InboxFull`, and the coordinator logs a warning. The coordinator does not retry -- the caller decides retry policy.

### 6.2 Security

#### 6.2.1 Agent Isolation

- **Bus message scoping**: Agents can only read from their own inbox receiver. The `AgentBus.send()` method requires the target agent_id; there is no broadcast or promiscuous mode.
- **Workspace file permissions**: Agent workspaces are `0700` (owner-only on Unix), set by `ensure_agent_workspace()` in H1.
- **No cross-agent workspace access**: Agents cannot access other agents' workspaces without explicit sharing via the shared memory protocol (H1).
- **Config isolation**: Per-agent `config.toml` is read only by that agent's runtime. No agent can read or modify another agent's config.

#### 6.2.2 Anonymous Agent Restrictions

Anonymous messages (empty `sender_id`) are routed to the catch-all or a dedicated "anonymous" agent. Anonymous agents operate with reduced permissions:

| Capability | Allowed | Reason |
|-----------|---------|--------|
| Read tools (web_search, read_file) | Yes | Safe, no side effects |
| Write tools (write_file, shell_exec) | No | Prevent unauthorized modifications |
| Claude delegation | No | Prevent cost abuse |
| Flow delegation | No | Prevent orchestration abuse |
| MCP tool calls | Read-only | Prevent external side effects |

#### 6.2.3 AgentBus Security

- Messages are tagged with `from_agent` and `to_agent`. The bus enforces delivery only to the `to_agent`'s inbox.
- There is no mechanism for an agent to forge its `from_agent` field -- the bus trusts the agent runtime (which is internal). External messages cannot inject into the `AgentBus`; it is not exposed to channel adapters.
- Bounded inbox size (`DEFAULT_INBOX_CAPACITY = 256`) prevents memory exhaustion from a malicious or runaway agent flooding another agent's inbox (D8 contract).

### 6.3 Concurrency

#### 6.3.1 AgentRouter Thread Safety

`AgentRouter` is read-heavy, write-rare (config reload only). Use `Arc<RwLock<AgentRouter>>`:
- Read lock: every incoming message (hot path)
- Write lock: config reload (rare, debounced 500ms)

#### 6.3.2 AgentRuntime Registry

The map of `agent_id -> AgentRuntime` is protected by `RwLock<HashMap<String, AgentRuntime>>`:
- Read lock: message dispatch to existing agent
- Write lock: first message to a new agent (workspace creation + runtime init)

#### 6.3.3 AgentBus Concurrency

The `AgentBus.inboxes` map uses `RwLock<HashMap<String, Sender>>`:
- Read lock: `send()` (hot path -- message delivery)
- Write lock: `register_agent()` / `unregister_agent()` (agent lifecycle, rare)

`mpsc::channel` (bounded) provides backpressure. `try_send()` is non-blocking and returns immediately if the inbox is full.

### 6.4 Performance Considerations

| Operation | Expected Latency | Notes |
|-----------|-----------------|-------|
| `route()` | < 1us | Linear scan of routes; typical deployments have < 50 routes |
| `ensure_agent_workspace()` | < 5ms (miss), < 1us (hit) | Filesystem ops on first call; path check on subsequent |
| `AgentBus.send()` | < 1us | RwLock read + channel try_send |
| `collect_results()` | Configurable timeout | Bounded by slowest worker + timeout |

For deployments with > 100 routes, consider indexing routes by channel name (HashMap<channel, Vec<AgentRoute>>). This optimization is out of scope for initial implementation but documented for future work.

---

## 7. Completion

### 7.1 Acceptance Criteria Checklist

#### L1: Agent Routing Table

- [ ] `MatchCriteria` struct defined with `user_id`, `phone`, `chat_id` fields (all optional)
- [ ] `AgentRoute` struct defined with `channel`, `match_criteria`, `agent` fields
- [ ] `RoutingResult` enum defined with `Agent(String)`, `CatchAll(String)`, `NoMatch` variants
- [ ] `AgentRouter` struct implemented with `route()`, `from_config()`, `add_route()`, `remove_routes_for_agent()`
- [ ] Route evaluation is ordered (first match wins)
- [ ] `MatchCriteria` fields are AND-ed (all specified fields must match)
- [ ] Catch-all fallback works when no specific route matches
- [ ] `RoutingResult::NoMatch` returned when no route and no catch-all
- [ ] `warn`-level log emitted on `NoMatch`
- [ ] Config deserialization from TOML `[[agent_routes]]` works
- [ ] Route shadow detection logs `warn` on config load

#### L2: Per-Agent Workspace Isolation

- [ ] `AgentRuntime` struct defined with `agent_id`, `workspace_path`, `session_manager`, `context_builder`, `tool_registry`, `config`
- [ ] `ensure_agent_workspace()` called on first message to an agent (from H1)
- [ ] Per-agent `SessionManager` reads/writes only from agent's `sessions/` directory
- [ ] Per-agent `ContextBuilder` reads `SOUL.md`, `AGENTS.md`, `USER.md` from agent workspace
- [ ] Per-agent `ToolRegistry` created via B5 shared builder with agent overrides
- [ ] Per-agent MCP config override from agent's `config.toml`
- [ ] Anonymous agents have reduced permissions (no write tools, no delegation)
- [ ] Agent runtimes are cached (not re-created on every message)
- [ ] Concurrent messages to the same agent are serialized

#### L3: Multi-Agent Swarming

- [ ] `InterAgentMessage` struct defined with `id`, `from_agent`, `to_agent`, `task`, `payload`, `reply_to`, `ttl`
- [ ] `MessagePayload` enum defined with `Text`, `Structured`, `Binary` variants
- [ ] `AgentBusError` enum defined with `AgentNotRegistered`, `InboxFull`, `MessageExpired`, `BusShutdown`
- [ ] `AgentBus` struct implemented with `register_agent()`, `unregister_agent()`, `send()`, `is_registered()`, `registered_agents()`
- [ ] `AgentBus` uses bounded channels (`DEFAULT_INBOX_CAPACITY = 256`)
- [ ] `AgentBus.send()` returns `InboxFull` when channel is at capacity
- [ ] `AgentBus.send()` returns `AgentNotRegistered` for unknown agents
- [ ] `SwarmCoordinator` implemented with `dispatch_subtask()`, `collect_results()`, `fan_out()`
- [ ] `collect_results()` respects timeout and returns partial results
- [ ] Agent-scoped delivery enforced (agents can only read their own inbox)

### 7.2 Test Plan

#### L1: Agent Routing Table Tests

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `route_exact_user_match` | unit | Message with matching channel + user_id routes to correct agent |
| 2 | `route_exact_phone_match` | unit | Message with matching channel + phone routes to correct agent |
| 3 | `route_exact_chat_id_match` | unit | Message with matching channel + chat_id routes to correct agent |
| 4 | `route_channel_only_match` | unit | Route with no criteria matches any message on that channel |
| 5 | `route_multi_criteria_and` | unit | Route with user_id + chat_id requires both to match |
| 6 | `route_first_match_wins` | unit | Two matching routes; first one in order is selected |
| 7 | `route_catch_all_fallback` | unit | Unmatched message with catch_all returns `CatchAll(agent)` |
| 8 | `route_no_match_no_catchall` | unit | Unmatched message without catch_all returns `NoMatch` |
| 9 | `route_anonymous_to_catchall` | unit | Message with empty sender_id routes to catch-all |
| 10 | `route_empty_routes_no_catchall` | unit | Empty router with no catch-all returns `NoMatch` for any message |
| 11 | `route_from_config` | unit | `AgentRouter::from_config()` correctly parses routes from config struct |
| 12 | `route_add_remove` | unit | `add_route()` and `remove_routes_for_agent()` modify routing table |
| 13 | `route_serde_roundtrip` | unit | `AgentRoute` and `MatchCriteria` serialize/deserialize correctly |

#### L2: Per-Agent Workspace Isolation Tests

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 14 | `dispatch_creates_workspace` | integration | First message to an agent triggers `ensure_agent_workspace()` |
| 15 | `dispatch_reuses_runtime` | unit | Second message to same agent reuses existing `AgentRuntime` |
| 16 | `dispatch_no_match_returns_error` | unit | `NoMatch` routing result returns an error, not silent drop |
| 17 | `agent_session_isolation` | integration | Two agents processing messages have separate session files |
| 18 | `agent_context_isolation` | integration | Each agent reads its own SOUL.md, not the other agent's |
| 19 | `anonymous_reduced_permissions` | unit | Anonymous agent runtime has write tools disabled |
| 20 | `per_agent_mcp_override` | unit | Agent config.toml MCP overrides applied to tool registry |
| 21 | `concurrent_dispatch_same_agent` | concurrency | Two concurrent messages to same agent are serialized |

#### L3: Multi-Agent Swarming Tests

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 22 | `agent_bus_register_and_receive` | unit | Registered agent receives sent messages |
| 23 | `agent_bus_unregister_drops` | unit | After unregister, inbox recv returns None |
| 24 | `agent_bus_send_to_unknown` | unit | Send to unregistered agent returns `AgentNotRegistered` |
| 25 | `agent_bus_inbox_full` | unit | Send to full inbox returns `InboxFull` |
| 26 | `agent_bus_scoped_delivery` | unit | Agent A cannot read Agent B's messages |
| 27 | `agent_bus_concurrent_send` | concurrency | Multiple senders to same agent; all messages delivered |
| 28 | `agent_bus_reregister_replaces` | unit | Re-registering agent drops old inbox, creates new one |
| 29 | `agent_bus_is_send_sync` | compile | `AgentBus` is `Send + Sync` |
| 30 | `swarm_dispatch_and_collect` | integration | Coordinator dispatches to 3 workers, collects all replies |
| 31 | `swarm_collect_timeout` | unit | `collect_results` returns partial results on timeout |
| 32 | `swarm_fan_out` | integration | `fan_out()` dispatches to all workers and collects |
| 33 | `swarm_worker_reply_correlation` | unit | Reply `reply_to` field matches original message `id` |
| 34 | `agent_bus_expired_ttl_rejected` | unit | Message with zero TTL returns `MessageExpired` |
| 35 | `agent_bus_registered_agents_list` | unit | `registered_agents()` returns correct list |

### 7.3 Exit Criteria

All of the following must be true before this phase is complete:

1. **All 35 tests pass** (`cargo test -p clawft-types -p clawft-core`)
2. **No regressions** in existing `MessageBus` tests (bus.rs:130-332)
3. **No regressions** in existing `AgentLoop` tests (loop_core.rs)
4. **Config integration**: `[[agent_routes]]` and `[routing]` sections parse correctly from `clawft.toml`
5. **Documentation**: Routing config documented in agent workspace `config.toml` comments
6. **Security**: Bus message isolation verified (test #26), anonymous permissions verified (test #19)
7. **Bounded channels**: `DEFAULT_INBOX_CAPACITY = 256` enforced (test #25, D8 contract)
8. **Cross-element contract 3.3**: L2 successfully calls `WorkspaceManager::ensure_agent_workspace()` from H1

---

## 8. Risk Notes

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| **H1 not ready when L2 starts** -- `ensure_agent_workspace()` is a hard dependency | Medium | High | **6** | H1 is prioritized for Weeks 4-6. If delayed, L2 can stub the workspace creation with a simple `create_dir_all`. |
| **B5 shared tool registry not ready** -- Per-agent tool registry depends on B5 builder pattern | Medium | Medium | **4** | L2 can clone the global `ToolRegistry` as a temporary measure. Agent-specific overrides deferred until B5 ships. |
| **Route ordering confusion** -- Users may not understand first-match-wins semantics | Medium | Low | **2** | Shadow detection warns on config load. Document ordering behavior in config comments and docs. |
| **Cross-agent bus message eavesdropping** -- Bug in `AgentBus` allows inbox cross-read | Low | High | **4** | Per-agent inbox receivers are never shared. Test #26 verifies isolation. |
| **AgentBus inbox memory exhaustion** -- Bounded channel prevents OOM but drops messages | Low | Medium | **3** | `DEFAULT_INBOX_CAPACITY = 256` is generous for subtask coordination. `InboxFull` error logged at `warn`. Capacity configurable. |
| **Concurrent runtime initialization race** -- Two messages arrive simultaneously for a new agent | Medium | Medium | **4** | `RwLock` on the runtime map. First writer creates the runtime; second writer finds it already initialized. `ensure_agent_workspace()` is idempotent. |
| **Config hot-reload routing change during in-flight messages** | Low | Low | **2** | In-flight messages complete with old routing. New messages use new routing table. Agent runtimes for orphaned routes gracefully shut down on idle. |
| **SwarmCoordinator timeout too short** -- Workers don't reply in time | Medium | Low | **2** | Timeout is caller-configurable. Default 5 minutes. `collect_results()` returns partial results, not an error. |

---

## 9. Cross-Element Dependencies

| Direction | Element | Item | Contract |
|-----------|---------|------|----------|
| **Depends on** | 08 (Memory & Workspace) | H1: `ensure_agent_workspace()` | L2 calls this on routing dispatch. Must be implemented before L2. |
| **Depends on** | 03 (Critical Fixes) | B5: Shared tool registry builder | L2 needs per-agent `ToolRegistry` instances from the shared builder. |
| **Depends on** | 05 (Pipeline) | D6: `sender_id` threaded through pipeline | L1 routes on `sender_id`. Must be available in `InboundMessage`. Already present (event.rs:22). |
| **Depends on** | 05 (Pipeline) | D8: Bounded bus channels | L3 `AgentBus` uses bounded `mpsc::channel`. Contract requires bounded capacity. |
| **This blocks** | 09 (Multi-Agent Routing) | L4: Planning strategies | L4 ReAct/Plan-and-Execute operates within the routing framework. |
| **This blocks** | 09 (Multi-Agent Routing) | M5: MCP bridge | Per-agent MCP config overrides (L2) are consumed by the bidirectional bridge. |
| **Contract 3.2** | MCP Client <-> Agent Routing | Per-agent MCP server config overrides global | Agent's `config.toml` `[mcp_servers]` section merged with global config. |
| **Contract 3.3** | Workspace <-> Routing | L2 calls `ensure_agent_workspace()` from H1 | Workspace must exist before `AgentRuntime` is initialized. |

---

## 10. Implementation Order

The recommended implementation order within this phase:

```
Week 5:
  L1.1  Define MatchCriteria, AgentRoute, RoutingResult types (clawft-types)
  L1.2  Implement AgentRouter with route(), from_config()
  L1.3  Tests 1-13 (routing logic)

Week 6:
  L2.1  Define AgentRuntime struct
  L2.2  Implement dispatch_to_agent() with workspace initialization
  L2.3  Per-agent SessionManager, ContextBuilder, ToolRegistry scoping
  L2.4  Anonymous agent permission reduction
  L2.5  Tests 14-21 (workspace isolation)

Week 7:
  L3.1  Define InterAgentMessage, MessagePayload, AgentBusError types (clawft-types)
  L3.2  Implement AgentBus with register/unregister/send
  L3.3  Tests 22-29 (bus fundamentals)

Week 8:
  L3.4  Implement SwarmCoordinator with dispatch/collect/fan_out
  L3.5  Tests 30-35 (swarm coordination)
  L3.6  Integration: wire AgentRouter into AgentLoop consumer path
  L3.7  End-to-end test: message -> route -> agent -> response
```
