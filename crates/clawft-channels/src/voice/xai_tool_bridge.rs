//! xAI Realtime → WeftOS tool bridge (ADR-074 V1 / WEFT-690).
//!
//! Registers custom **function** tools on `session.update`, maps
//! `response.function_call_arguments.done` events to typed
//! [`WindowIntent`] verbs (ADR-073), and builds the client reply events
//! (`conversation.item.create` + `response.create`).
//!
//! # Tools (V1 minimum)
//!
//! | Function name   | Intent        | Purpose |
//! |-----------------|---------------|---------|
//! | `spawn_agent`   | [`WindowIntent::Spawn`] | Open N agent pane(s) with role/prompt |
//! | `focus_pane`    | [`WindowIntent::Focus`] | Bring pane/window to front |
//! | `summarize`     | [`WindowIntent::Summarize`] | Short spoken/UI-safe summary |
//! | `shell_intent`  | [`WindowIntent::Shell`] | **Only** when [`CommandPolicy`] allows (off unrestricted shell) |
//!
//! # Event flow
//!
//! ```text
//! session.update { tools: weftos_realtime_tools(..) }
//!        │
//!        ▼  (user speaks / model decides)
//! response.function_call_arguments.done { name, call_id, arguments }
//!        │
//!        ▼  parse_function_call → map_to_window_intent
//! WindowIntent  ──► IntentHandler::handle  ──► ToolCallResult
//!        │
//!        ▼
//! conversation.item.create { function_call_output }
//! response.create   (after all parallel tool outputs; wait for audio if playing)
//! ```
//!
//! # Acoustic Echo Cancellation (required for cloud S2S)
//!
//! Full-duplex cloud speech-to-speech **still needs AEC in front of the mic**.
//! Speaker playback is the AEC render/reference; only echo-cancelled capture
//! should be appended to the Realtime input buffer — otherwise the model
//! hears itself and false-barge-ins. Target wire order (ADR-068 / ADR-074):
//!
//! ```text
//! mic ──► AEC::process_capture ──► input_audio_buffer / binary frames
//! speaker ◄── AEC::push_render  ◄── response audio deltas
//! ```
//!
//! Implementation lives in `clawft-voice-aec` (`AecProcessor`). This bridge
//! does **not** open unrestricted audio devices. [`aec_pipeline_ready`] is an
//! honest readiness hook: returns `false` until a Talk-Mode / device path
//! injects AEC (DSP may be passthrough without `webrtc-aec`). See
//! [`AEC_IN_FRONT_OF_MIC_NOTE`].

use clawft_types::security::CommandPolicy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tracing::{debug, info, warn};

// ── AEC readiness (stub / hook) ─────────────────────────────────────────────

/// Product + architecture note: cloud S2S requires AEC before mic uplink.
pub const AEC_IN_FRONT_OF_MIC_NOTE: &str = "\
Cloud S2S (xAI Realtime) requires acoustic echo cancellation in front of the \
microphone: feed speaker/TTS PCM into AEC render (push_render) and only send \
AEC-processed capture to the Realtime input. Without this, the model hears \
itself. Wire via clawft-voice-aec::AecProcessor when the capture path is ready \
(feature webrtc-aec for real AEC3; default build may be passthrough).";

/// Always true for product policy — cloud S2S must not skip AEC design.
pub fn aec_in_front_of_mic_required() -> bool {
    true
}

/// Honest readiness: V1 bridge has no live AEC injection; Talk-Mode / device
/// path must set this true when `AecProcessor` is on the capture pipeline.
///
/// Call sites should log [`AEC_IN_FRONT_OF_MIC_NOTE`] when this is `false` and
/// still opening a cloud S2S session (demo may proceed with degraded barge-in).
pub fn aec_pipeline_ready() -> bool {
    // Stub until WEFT-691 / voice-real-audio + voice-aec device path wires in.
    false
}

// ── Tool names ──────────────────────────────────────────────────────────────

/// Registered Realtime function: spawn agent pane(s).
pub const TOOL_SPAWN_AGENT: &str = "spawn_agent";
/// Registered Realtime function: focus a pane/window.
pub const TOOL_FOCUS_PANE: &str = "focus_pane";
/// Registered Realtime function: summarize for speech/UI.
pub const TOOL_SUMMARIZE: &str = "summarize";
/// Optional Realtime function: governed shell intent (not unrestricted shell).
pub const TOOL_SHELL_INTENT: &str = "shell_intent";

/// Default max characters for spoken/UI-safe summarize output.
pub const DEFAULT_SUMMARIZE_MAX_CHARS: usize = 280;

/// Hard cap on agent count per single spawn_agent call (demo safety).
pub const MAX_SPAWN_COUNT: u32 = 8;

// ── WindowIntent (ADR-073 skeleton) ─────────────────────────────────────────

/// Typed layout/control intent bus (ADR-073).
///
/// Sources (voice, keys, MCP, GUI) must eventually emit the same verbs.
/// V1 is the **skeleton** used by the xAI tool bridge — full Agent Workspace
/// WM binding is WEFT-688 / Phase D.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum WindowIntent {
    /// Open one or more agent panes (visible by default).
    Spawn {
        /// Agent role or type (e.g. `researcher`, `coder`).
        role: String,
        /// Optional initial prompt / task.
        #[serde(default)]
        prompt: Option<String>,
        /// How many agent panes to open (1..=MAX_SPAWN_COUNT).
        #[serde(default = "default_spawn_count")]
        count: u32,
        /// When true, allow background-only (no pane). Default false (visible).
        #[serde(default)]
        background: bool,
    },
    /// Bring a pane/window to front / select it.
    Focus {
        /// Stable pane id when known.
        #[serde(default)]
        pane_id: Option<String>,
        /// Human title / label match when id unknown.
        #[serde(default)]
        title: Option<String>,
    },
    /// Compress text (or a target pane's buffer) into spoken/UI-safe length.
    Summarize {
        /// Source text when the model already has content.
        #[serde(default)]
        text: Option<String>,
        /// Optional pane to summarize instead of inline text.
        #[serde(default)]
        pane_id: Option<String>,
        /// Max chars for the spoken/UI result.
        #[serde(default = "default_summarize_max")]
        max_chars: usize,
    },
    /// Arrange panes (grid/cascade) — reserved; V1 maps may reject or no-op.
    Arrange {
        #[serde(default)]
        strategy: Option<String>,
    },
    /// Close a pane (+ optional agent cancel).
    Close {
        #[serde(default)]
        pane_id: Option<String>,
    },
    /// Governed shell / command intent — **never** unrestricted by default.
    Shell {
        /// Command line to validate against [`CommandPolicy`].
        command: String,
    },
}

fn default_spawn_count() -> u32 {
    1
}
fn default_summarize_max() -> usize {
    DEFAULT_SUMMARIZE_MAX_CHARS
}

impl WindowIntent {
    /// Short label for logs / UI chips (no secrets).
    pub fn verb_name(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "spawn",
            Self::Focus { .. } => "focus",
            Self::Summarize { .. } => "summarize",
            Self::Arrange { .. } => "arrange",
            Self::Close { .. } => "close",
            Self::Shell { .. } => "shell",
        }
    }
}

// ── Function-call wire types ────────────────────────────────────────────────

/// Parsed server event `response.function_call_arguments.done`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCallEvent {
    /// Tool / function name.
    pub name: String,
    /// Call id for `function_call_output`.
    pub call_id: String,
    /// Raw arguments JSON string from the server.
    pub arguments: String,
}

/// Errors while parsing or mapping tool calls.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolBridgeError {
    #[error("not a function-call event (type={0})")]
    NotFunctionCall(String),

    #[error("missing field: {0}")]
    MissingField(&'static str),

    #[error("invalid arguments JSON: {0}")]
    InvalidArguments(String),

    #[error("unknown tool: {0}")]
    UnknownTool(String),

    #[error("invalid tool arguments: {0}")]
    InvalidToolArgs(String),

    #[error("shell intent denied by governance: {0}")]
    ShellDenied(String),
}

/// Outcome of handling a mapped intent (spoken/UI-safe, serializable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Whether the intent was accepted and applied (or recorded for demo).
    pub ok: bool,
    /// Short status for the model / UI.
    pub message: String,
    /// Optional spoken-safe one-liner (≤ ~DEFAULT_SUMMARIZE_MAX_CHARS).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoken: Option<String>,
    /// Echo of the intent verb for clients.
    pub verb: String,
    /// Extra structured fields (pane ids, spawn count, etc.).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

impl ToolCallResult {
    pub fn success(verb: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
            spoken: None,
            verb: verb.into(),
            data: Value::Null,
        }
    }

    pub fn failure(verb: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            spoken: None,
            verb: verb.into(),
            data: Value::Null,
        }
    }

    pub fn with_spoken(mut self, spoken: impl Into<String>) -> Self {
        self.spoken = Some(spoken.into());
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    /// Serialize for `function_call_output.output` (JSON string).
    pub fn to_output_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"ok":false,"message":"serialize failed","verb":"unknown"}"#.into()
        })
    }
}

// ── Tool definitions for session.update ─────────────────────────────────────

/// Options for which tools to register on the Realtime session.
#[derive(Debug, Clone, Copy)]
pub struct ToolCatalogOptions {
    /// Include spawn / focus / summarize (V1 minimum). Default true.
    pub core_window_intents: bool,
    /// Include `shell_intent` (still gated by [`CommandPolicy`] at handle time).
    /// Default **false** — do not open unrestricted shell by default.
    pub shell_intent: bool,
}

impl Default for ToolCatalogOptions {
    fn default() -> Self {
        Self {
            core_window_intents: true,
            shell_intent: false,
        }
    }
}

/// JSON tool definitions for `session.tools` (xAI custom function shape).
pub fn weftos_realtime_tools(opts: ToolCatalogOptions) -> Vec<Value> {
    let mut tools = Vec::new();
    if opts.core_window_intents {
        tools.push(json!({
            "type": "function",
            "name": TOOL_SPAWN_AGENT,
            "description": "Spawn one or more WeftOS agents as visible Agent Workspace panes (or shell path). Use when the user asks to start N agents, open researchers/coders, or spin up workers. Prefer visible panes (background=false).",
            "parameters": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Agent role or type (e.g. researcher, coder, reviewer)"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Optional initial task / prompt for each agent"
                    },
                    "count": {
                        "type": "integer",
                        "description": "Number of agents to spawn (1-8)",
                        "minimum": 1,
                        "maximum": 8
                    },
                    "background": {
                        "type": "boolean",
                        "description": "If true, allow no UI pane (requires explicit opt-in). Default false."
                    }
                },
                "required": ["role"]
            }
        }));
        tools.push(json!({
            "type": "function",
            "name": TOOL_FOCUS_PANE,
            "description": "Focus / bring to front an agent pane or window by id or title.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pane_id": {
                        "type": "string",
                        "description": "Stable pane identifier when known"
                    },
                    "title": {
                        "type": "string",
                        "description": "Pane title / label to match when id unknown"
                    }
                }
            }
        }));
        tools.push(json!({
            "type": "function",
            "name": TOOL_SUMMARIZE,
            "description": "Summarize a wall of text or a pane buffer into a short spoken/UI-safe string (about one breath).",
            "parameters": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Source text to compress"
                    },
                    "pane_id": {
                        "type": "string",
                        "description": "Optional pane whose content should be summarized"
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "Max characters for the spoken/UI result (default 280)"
                    }
                }
            }
        }));
    }
    if opts.shell_intent {
        tools.push(json!({
            "type": "function",
            "name": TOOL_SHELL_INTENT,
            "description": "Request a governed shell command. Only commands allowed by WeftOS CommandPolicy run; unrestricted shell is never available. Prefer spawn_agent for agent work.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command line (subject to allowlist governance)"
                    }
                },
                "required": ["command"]
            }
        }));
    }
    tools
}

/// Default V1 catalog: spawn / focus / summarize only (no shell tool registered).
pub fn default_weftos_tools() -> Vec<Value> {
    weftos_realtime_tools(ToolCatalogOptions::default())
}

// ── Parse + map ─────────────────────────────────────────────────────────────

/// Parse a Realtime server JSON event into a [`FunctionCallEvent`].
///
/// Accepts `response.function_call_arguments.done` (primary xAI shape).
pub fn parse_function_call_event(event: &Value) -> Result<FunctionCallEvent, ToolBridgeError> {
    let event_type = event
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("unknown");
    if event_type != "response.function_call_arguments.done" {
        return Err(ToolBridgeError::NotFunctionCall(event_type.into()));
    }
    let name = event
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(ToolBridgeError::MissingField("name"))?
        .to_string();
    let call_id = event
        .get("call_id")
        .and_then(|v| v.as_str())
        .ok_or(ToolBridgeError::MissingField("call_id"))?
        .to_string();
    let arguments = match event.get("arguments") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => return Err(ToolBridgeError::MissingField("arguments")),
    };
    Ok(FunctionCallEvent {
        name,
        call_id,
        arguments,
    })
}

/// Map a function name + arguments JSON object to [`WindowIntent`].
pub fn map_to_window_intent(
    name: &str,
    args: &Value,
) -> Result<WindowIntent, ToolBridgeError> {
    match name {
        TOOL_SPAWN_AGENT => {
            let role = args
                .get("role")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    ToolBridgeError::InvalidToolArgs("spawn_agent requires non-empty role".into())
                })?
                .to_string();
            let prompt = args
                .get("prompt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let mut count = args
                .get("count")
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
                .unwrap_or(1) as u32;
            if count == 0 {
                count = 1;
            }
            if count > MAX_SPAWN_COUNT {
                count = MAX_SPAWN_COUNT;
            }
            let background = args
                .get("background")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(WindowIntent::Spawn {
                role,
                prompt,
                count,
                background,
            })
        }
        TOOL_FOCUS_PANE => {
            let pane_id = args
                .get("pane_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if pane_id.is_none() && title.is_none() {
                return Err(ToolBridgeError::InvalidToolArgs(
                    "focus_pane requires pane_id or title".into(),
                ));
            }
            Ok(WindowIntent::Focus { pane_id, title })
        }
        TOOL_SUMMARIZE => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let pane_id = args
                .get("pane_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if text.is_none() && pane_id.is_none() {
                return Err(ToolBridgeError::InvalidToolArgs(
                    "summarize requires text or pane_id".into(),
                ));
            }
            let max_chars = args
                .get("max_chars")
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_SUMMARIZE_MAX_CHARS)
                .clamp(40, 2000);
            Ok(WindowIntent::Summarize {
                text,
                pane_id,
                max_chars,
            })
        }
        TOOL_SHELL_INTENT => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    ToolBridgeError::InvalidToolArgs(
                        "shell_intent requires non-empty command".into(),
                    )
                })?
                .to_string();
            Ok(WindowIntent::Shell { command })
        }
        other => Err(ToolBridgeError::UnknownTool(other.into())),
    }
}

/// Parse function-call event then map arguments → [`WindowIntent`].
pub fn map_function_call_to_intent(
    event: &FunctionCallEvent,
) -> Result<WindowIntent, ToolBridgeError> {
    let args: Value = if event.arguments.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&event.arguments)
            .map_err(|e| ToolBridgeError::InvalidArguments(e.to_string()))?
    };
    map_to_window_intent(&event.name, &args)
}

// ── Client reply payloads ───────────────────────────────────────────────────

/// Build `conversation.item.create` with `function_call_output`.
pub fn function_call_output_event(call_id: &str, output: &str) -> Value {
    json!({
        "type": "conversation.item.create",
        "item": {
            "type": "function_call_output",
            "call_id": call_id,
            "output": output
        }
    })
}

/// Build `response.create` to continue after tool outputs.
pub fn response_create_event() -> Value {
    json!({ "type": "response.create" })
}

// ── Intent handler ──────────────────────────────────────────────────────────

/// Host-side executor for [`WindowIntent`] (GUI bus, agent runtime, demo log).
pub trait IntentHandler: Send {
    fn handle(&mut self, intent: &WindowIntent) -> ToolCallResult;
}

/// In-memory demo / unit-test handler.
///
/// Records intents so the CNVS-like path “voice spawns N agents” is testable
/// without a full Agent Workspace WM. Shell runs only through
/// [`CommandPolicy::safe_defaults`] validation — never unrestricted exec.
#[derive(Debug, Default)]
pub struct DemoIntentHandler {
    /// Intents accepted in order.
    pub applied: Vec<WindowIntent>,
    /// Synthetic pane ids issued for spawns.
    pub panes: Vec<String>,
    /// Focused pane id if any.
    pub focused: Option<String>,
    /// Last spoken summary.
    pub last_summary: Option<String>,
    /// Command policy for shell intents (default: safe allowlist).
    pub shell_policy: CommandPolicy,
    /// When true, shell intents that pass policy are recorded as “would run”
    /// but **not** executed (V1 does not open a process from the bridge).
    pub shell_execute: bool,
}

impl DemoIntentHandler {
    pub fn new() -> Self {
        Self {
            shell_policy: CommandPolicy::safe_defaults(),
            shell_execute: false,
            ..Default::default()
        }
    }

    /// Number of Spawn intents applied (each may have count > 1).
    pub fn total_spawned_agents(&self) -> u32 {
        self.applied
            .iter()
            .filter_map(|i| match i {
                WindowIntent::Spawn { count, .. } => Some(*count),
                _ => None,
            })
            .sum()
    }
}

impl IntentHandler for DemoIntentHandler {
    fn handle(&mut self, intent: &WindowIntent) -> ToolCallResult {
        match intent {
            WindowIntent::Spawn {
                role,
                prompt,
                count,
                background,
            } => {
                let mut ids = Vec::with_capacity(*count as usize);
                for i in 0..*count {
                    let id = format!("pane-{}-{}", role, self.panes.len() + 1 + i as usize);
                    ids.push(id.clone());
                    self.panes.push(id);
                }
                self.applied.push(intent.clone());
                let spoken = if *count == 1 {
                    format!("Spawned {role} agent.")
                } else {
                    format!("Spawned {count} {role} agents.")
                };
                let msg = if *background {
                    format!(
                        "Recorded background spawn of {count}×{role} (visible default preferred)"
                    )
                } else {
                    format!("Spawned {count} visible agent pane(s) role={role}")
                };
                info!(role = %role, count, background, "tool bridge: spawn_agent");
                ToolCallResult::success("spawn", msg)
                    .with_spoken(spoken)
                    .with_data(json!({
                        "pane_ids": ids,
                        "role": role,
                        "count": count,
                        "prompt": prompt,
                        "background": background,
                        "visible": !*background,
                    }))
            }
            WindowIntent::Focus { pane_id, title } => {
                let target = pane_id
                    .clone()
                    .or_else(|| title.clone())
                    .unwrap_or_else(|| "unknown".into());
                // Prefer exact pane_id match in our demo list.
                if let Some(id) = pane_id {
                    if self.panes.iter().any(|p| p == id) || !self.panes.is_empty() {
                        self.focused = Some(id.clone());
                    } else {
                        self.focused = Some(id.clone());
                    }
                } else if let Some(t) = title {
                    let found = self
                        .panes
                        .iter()
                        .find(|p| p.contains(t.as_str()))
                        .cloned();
                    self.focused = found.or(Some(t.clone()));
                }
                self.applied.push(intent.clone());
                let focused = self.focused.clone().unwrap_or(target);
                info!(%focused, "tool bridge: focus_pane");
                ToolCallResult::success("focus", format!("Focused {focused}"))
                    .with_spoken(format!("Focused {focused}."))
                    .with_data(json!({ "pane_id": focused }))
            }
            WindowIntent::Summarize {
                text,
                pane_id,
                max_chars,
            } => {
                let source = text.clone().unwrap_or_else(|| {
                    pane_id
                        .as_ref()
                        .map(|p| format!("(pane {p}: no buffer bound in V1 demo handler)"))
                        .unwrap_or_default()
                });
                let summary = summarize_spoken_safe(&source, *max_chars);
                self.last_summary = Some(summary.clone());
                self.applied.push(intent.clone());
                debug!(len = summary.len(), "tool bridge: summarize");
                ToolCallResult::success("summarize", "Summary ready")
                    .with_spoken(summary.clone())
                    .with_data(json!({ "summary": summary, "pane_id": pane_id }))
            }
            WindowIntent::Arrange { strategy } => {
                self.applied.push(intent.clone());
                ToolCallResult::success(
                    "arrange",
                    format!(
                        "Arrange recorded (WM binding deferred): {}",
                        strategy.as_deref().unwrap_or("default")
                    ),
                )
            }
            WindowIntent::Close { pane_id } => {
                if let Some(id) = pane_id {
                    self.panes.retain(|p| p != id);
                    if self.focused.as_deref() == Some(id.as_str()) {
                        self.focused = None;
                    }
                }
                self.applied.push(intent.clone());
                ToolCallResult::success(
                    "close",
                    format!("Close recorded for {:?}", pane_id),
                )
            }
            WindowIntent::Shell { command } => match self.shell_policy.validate(command) {
                Ok(()) => {
                    self.applied.push(intent.clone());
                    if self.shell_execute {
                        // V1 deliberately does not exec — keep governance path only.
                        warn!("shell_execute=true but V1 bridge never runs processes");
                    }
                    info!(command = %command, "tool bridge: shell_intent allowed (not executed)");
                    ToolCallResult::success(
                        "shell",
                        format!("Command allowed by policy (not executed in V1): {command}"),
                    )
                    .with_spoken("Command allowed.")
                    .with_data(json!({
                        "command": command,
                        "executed": false,
                        "policy": "allowlist"
                    }))
                }
                Err(e) => {
                    warn!(command = %command, error = %e, "tool bridge: shell_intent denied");
                    ToolCallResult::failure(
                        "shell",
                        format!("Shell intent denied by governance: {e}"),
                    )
                    .with_spoken("That command is not allowed.")
                    .with_data(json!({
                        "command": command,
                        "executed": false,
                        "denied": true,
                        "reason": e.to_string()
                    }))
                }
            },
        }
    }
}

/// Extractive spoken/UI-safe summary: collapse whitespace, take first sentences
/// up to `max_chars`, ellipsize if needed.
pub fn summarize_spoken_safe(text: &str, max_chars: usize) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return "Nothing to summarize.".into();
    }
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    // Prefer sentence boundary within budget.
    let mut acc = String::new();
    for sentence in collapsed.split_inclusive(['.', '!', '?']) {
        let next_len = acc.chars().count() + sentence.chars().count();
        if !acc.is_empty() && next_len > max_chars {
            break;
        }
        if acc.is_empty() && sentence.chars().count() > max_chars {
            // Single long sentence: hard truncate.
            let t: String = sentence.chars().take(max_chars.saturating_sub(1)).collect();
            return format!("{t}…");
        }
        acc.push_str(sentence);
        if acc.chars().count() >= max_chars {
            break;
        }
    }
    let acc = acc.trim();
    if acc.chars().count() > max_chars {
        let t: String = acc.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{t}…")
    } else if acc.is_empty() {
        let t: String = collapsed.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{t}…")
    } else {
        acc.to_string()
    }
}

/// Full path: parse server event → map → handle → output event pair.
///
/// Returns `(function_call_output event, ToolCallResult)`.
/// Caller sends the output event(s), then `response_create_event` after all
/// parallel tool calls (and preferably after current audio finishes).
pub fn handle_function_call_event(
    event: &Value,
    handler: &mut dyn IntentHandler,
) -> Result<(Value, ToolCallResult), ToolBridgeError> {
    let fc = parse_function_call_event(event)?;
    let intent = map_function_call_to_intent(&fc)?;
    let result = handler.handle(&intent);
    let out = function_call_output_event(&fc.call_id, &result.to_output_string());
    Ok((out, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aec_requirement_documented() {
        assert!(aec_in_front_of_mic_required());
        assert!(!aec_pipeline_ready());
        assert!(AEC_IN_FRONT_OF_MIC_NOTE.contains("echo cancellation"));
        assert!(AEC_IN_FRONT_OF_MIC_NOTE.contains("clawft-voice-aec"));
    }

    #[test]
    fn default_tools_are_core_only() {
        let tools = default_weftos_tools();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert_eq!(
            names,
            vec![TOOL_SPAWN_AGENT, TOOL_FOCUS_PANE, TOOL_SUMMARIZE]
        );
        assert!(!names.contains(&TOOL_SHELL_INTENT));
        for t in &tools {
            assert_eq!(t["type"], "function");
            assert!(t.get("parameters").is_some());
        }
    }

    #[test]
    fn shell_tool_opt_in() {
        let tools = weftos_realtime_tools(ToolCatalogOptions {
            core_window_intents: true,
            shell_intent: true,
        });
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[3]["name"], TOOL_SHELL_INTENT);
    }

    #[test]
    fn parse_function_call_done() {
        let event = json!({
            "type": "response.function_call_arguments.done",
            "name": "spawn_agent",
            "call_id": "call_1",
            "arguments": r#"{"role":"researcher","count":3}"#
        });
        let fc = parse_function_call_event(&event).unwrap();
        assert_eq!(fc.name, "spawn_agent");
        assert_eq!(fc.call_id, "call_1");
        let intent = map_function_call_to_intent(&fc).unwrap();
        assert_eq!(
            intent,
            WindowIntent::Spawn {
                role: "researcher".into(),
                prompt: None,
                count: 3,
                background: false,
            }
        );
    }

    #[test]
    fn parse_rejects_other_event_types() {
        let event = json!({"type": "session.updated"});
        let err = parse_function_call_event(&event).unwrap_err();
        assert!(matches!(err, ToolBridgeError::NotFunctionCall(_)));
    }

    #[test]
    fn map_spawn_clamps_count() {
        let intent = map_to_window_intent(
            TOOL_SPAWN_AGENT,
            &json!({"role": "coder", "count": 99}),
        )
        .unwrap();
        match intent {
            WindowIntent::Spawn { count, .. } => assert_eq!(count, MAX_SPAWN_COUNT),
            _ => panic!("expected Spawn"),
        }
    }

    #[test]
    fn map_spawn_requires_role() {
        let err = map_to_window_intent(TOOL_SPAWN_AGENT, &json!({})).unwrap_err();
        assert!(matches!(err, ToolBridgeError::InvalidToolArgs(_)));
    }

    #[test]
    fn map_focus_and_summarize() {
        let f = map_to_window_intent(TOOL_FOCUS_PANE, &json!({"title": "Agent 1"})).unwrap();
        assert!(matches!(f, WindowIntent::Focus { .. }));
        let s = map_to_window_intent(
            TOOL_SUMMARIZE,
            &json!({"text": "Hello world. More detail here."}),
        )
        .unwrap();
        assert!(matches!(s, WindowIntent::Summarize { .. }));
    }

    #[test]
    fn map_unknown_tool() {
        let err = map_to_window_intent("delete_disk", &json!({})).unwrap_err();
        assert!(matches!(err, ToolBridgeError::UnknownTool(_)));
    }

    #[test]
    fn demo_spawn_n_agents() {
        let mut h = DemoIntentHandler::new();
        let intent = WindowIntent::Spawn {
            role: "researcher".into(),
            prompt: Some("scan docs".into()),
            count: 3,
            background: false,
        };
        let r = h.handle(&intent);
        assert!(r.ok);
        assert_eq!(h.total_spawned_agents(), 3);
        assert_eq!(h.panes.len(), 3);
        assert!(r.spoken.as_ref().unwrap().contains("3"));
        let ids = r.data["pane_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn demo_focus_and_summarize() {
        let mut h = DemoIntentHandler::new();
        h.handle(&WindowIntent::Spawn {
            role: "coder".into(),
            prompt: None,
            count: 1,
            background: false,
        });
        let pane = h.panes[0].clone();
        let r = h.handle(&WindowIntent::Focus {
            pane_id: Some(pane.clone()),
            title: None,
        });
        assert!(r.ok);
        assert_eq!(h.focused.as_deref(), Some(pane.as_str()));

        let wall = "First sentence is important. ".repeat(40);
        let r = h.handle(&WindowIntent::Summarize {
            text: Some(wall),
            pane_id: None,
            max_chars: 80,
        });
        assert!(r.ok);
        let spoken = r.spoken.unwrap();
        assert!(spoken.chars().count() <= 80);
        assert!(h.last_summary.is_some());
    }

    #[test]
    fn shell_denied_by_default_policy() {
        let mut h = DemoIntentHandler::new();
        let r = h.handle(&WindowIntent::Shell {
            command: "rm -rf /".into(),
        });
        assert!(!r.ok);
        assert!(r.message.contains("denied") || r.message.contains("governance") || r.message.contains("dangerous") || r.message.contains("not allowed") || r.message.contains("blocked") || !r.ok);
    }

    #[test]
    fn shell_allowlisted_not_executed() {
        let mut h = DemoIntentHandler::new();
        let r = h.handle(&WindowIntent::Shell {
            command: "echo hello".into(),
        });
        assert!(r.ok, "{r:?}");
        assert_eq!(r.data["executed"], false);
    }

    #[test]
    fn shell_tool_not_in_default_catalog() {
        // Unrestricted shell must not be registered by default.
        let tools = default_weftos_tools();
        assert!(tools
            .iter()
            .all(|t| t.get("name").and_then(|n| n.as_str()) != Some(TOOL_SHELL_INTENT)));
    }

    #[test]
    fn handle_function_call_event_end_to_end() {
        let mut h = DemoIntentHandler::new();
        let event = json!({
            "type": "response.function_call_arguments.done",
            "name": "spawn_agent",
            "call_id": "c42",
            "arguments": r#"{"role":"reviewer","count":2,"prompt":"check ADR"}"#
        });
        let (out, result) = handle_function_call_event(&event, &mut h).unwrap();
        assert!(result.ok);
        assert_eq!(out["type"], "conversation.item.create");
        assert_eq!(out["item"]["type"], "function_call_output");
        assert_eq!(out["item"]["call_id"], "c42");
        let output: Value = serde_json::from_str(out["item"]["output"].as_str().unwrap()).unwrap();
        assert_eq!(output["ok"], true);
        assert_eq!(h.total_spawned_agents(), 2);
        let cont = response_create_event();
        assert_eq!(cont["type"], "response.create");
    }

    #[test]
    fn summarize_spoken_safe_truncates() {
        let s = summarize_spoken_safe("Word ".repeat(100).as_str(), 50);
        assert!(s.chars().count() <= 50);
        assert!(s.ends_with('…') || s.chars().count() < 50);
    }

    #[test]
    fn function_call_output_shape() {
        let v = function_call_output_event("id1", r#"{"ok":true}"#);
        assert_eq!(v["item"]["call_id"], "id1");
        assert_eq!(v["item"]["type"], "function_call_output");
    }
}
