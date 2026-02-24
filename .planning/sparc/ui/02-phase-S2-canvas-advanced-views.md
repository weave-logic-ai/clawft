# SPARC Phase Specification: S2 -- Advanced Views + Live Canvas

| Field | Value |
|-------|-------|
| **Element** | K1 Web Dashboard -- Sprint 2: Advanced Views + Live Canvas |
| **Phase** | S2 (Specification) |
| **Timeline** | Weeks 4-6 (3 weeks) |
| **Priority** | P0 (S2.1 Canvas), P1 (S2.2-S2.5) |
| **Crates** | `clawft-types`, `clawft-tools`, `clawft-services`, `ui/` |
| **Dependencies IN** | S1.1 (Backend API Foundation), S1.2 (Frontend Scaffolding), S1.3 (Core Views), C3 (skill loader), C4 (hot-reload), H1 (per-agent workspace), H2 (HNSW VectorStore), K4 (ClawHub registry), L1 (agent routing table) |
| **Blocks** | S3.1 (Delegation Monitor), S3.2 (Advanced Canvas), S3.4 (Tauri Desktop) |
| **Status** | PLANNED |

---

## 1. Overview

Sprint 2 delivers five feature areas on top of the S1 foundation:

1. **S2.1 Live Canvas (P0, Weeks 4-5)** -- The agent-driven interactive UI workspace. Agents call `render_ui` to push UI elements (text, buttons, inputs, images, code blocks, tables, forms) to a canvas rendered in real-time via WebSocket. User interactions (clicks, form submissions) route back to agents as tool results. The canvas supports snapshot/restore for state persistence and undo/redo for history navigation.

2. **S2.2 Skill Browser (P1, Week 5)** -- Browse installed skills in a card grid, search the ClawHub registry, and install/uninstall skills with progress feedback. Proxies ClawHub API through the backend to avoid CORS issues and add auth.

3. **S2.3 Memory Explorer (P1, Week 5)** -- CRUD interface for the workspace memory store. DataTable listing with namespace/tag filtering, semantic search with similarity threshold slider, and a write dialog for creating new entries.

4. **S2.4 Configuration Editor (P1, Week 6)** -- Tabbed editor for the full `Config` schema. Schema-driven form generation with react-hook-form + zod validation. Diff viewer shows pending changes before save confirmation.

5. **S2.5 Cron + Channels (P1, Week 6)** -- Cron job dashboard with CRUD, expression editor, and next-fire preview. Channel status view with real-time WebSocket connection state updates and routing visualization.

### 1.1 Design Principles

- **Backend-first types**: All protocol types are defined in `clawft-types` first, then mirrored to TypeScript via a shared `types.ts` module. The Rust types are the source of truth.
- **MSW parity**: Every new API endpoint has a corresponding MSW mock handler so the frontend can be developed and tested without a running backend.
- **Component isolation**: Each feature area uses its own route, store slice, and component directory. Cross-feature coupling goes through shared hooks and the API client.
- **Progressive enhancement**: Canvas starts with basic element types (text, button, input). Charts, code editors, and form builders are deferred to S3.2.

---

## 2. Current Code (S1 Deliverables That S2 Builds On)

### 2.1 Backend (clawft-services)

S1.1 delivered:

- **Axum REST router factory** at `clawft-services/src/api/mod.rs` -- `build_api_router()` returns an `axum::Router` with `/api/agents/*`, `/api/sessions/*`, `/api/tools/*` routes.
- **Auth middleware** -- Bearer token validation via `tower` layer. `/api/auth/token` endpoint generates tokens.
- **WebSocket handler** at `/ws` -- Upgrade handler with topic subscription (`subscribe`/`unsubscribe` JSON commands). Uses `tokio::sync::broadcast` for fan-out.
- **CORS middleware** -- `tower-http::cors` with configurable origins from `GatewayConfig`.
- **Static file serving** -- `ui/dist/` served at `/` via `tower-http::services::ServeDir` or `rust-embed`.

### 2.2 Frontend (ui/)

S1.2 delivered:

- **Vite + React + TypeScript** project at `ui/` with `pnpm` package manager.
- **Tailwind CSS v4 + shadcn/ui** component library.
- **MSW mock layer** at `ui/src/mocks/` with handlers for agents, sessions, tools.
- **`api-client.ts`** at `ui/src/lib/api-client.ts` -- Fetch wrapper with Bearer auth, configurable `VITE_API_URL`.
- **`ws-client.ts`** at `ui/src/lib/ws-client.ts` -- Reconnecting WebSocket with exponential backoff, topic subscription, typed message handlers.
- **TanStack Router** with file-based routes at `ui/src/routes/`.
- **`MainLayout`** with collapsible sidebar at `ui/src/components/layout/`.

S1.3 delivered:

- **Dashboard Home** at `ui/src/routes/index.tsx`.
- **Agent Management** at `ui/src/routes/agents.tsx` with DataTable, status badges, start/stop actions.
- **WebChat** at `ui/src/routes/chat.tsx` with streaming message display, tool call cards.
- **Session Explorer** at `ui/src/routes/sessions.tsx` with DataTable.
- **Tool Registry** at `ui/src/routes/tools.tsx` with JSON Schema tree viewer.
- **Theme toggle** (dark/light) and **Cmd+K command palette**.

### 2.3 Existing Rust Types (clawft-types)

Types that S2 extends or references:

- `clawft_types::skill::SkillDefinition` -- Skill metadata with name, description, version, variables, allowed_tools.
- `clawft_types::cron::CronJob`, `CronSchedule`, `CronPayload`, `CronJobState`, `ScheduleKind`, `JobStatus` -- Full cron job model.
- `clawft_types::config::Config` -- Root config with agents, channels, providers, gateway, tools, delegation, routing sections.
- `clawft_types::config::AgentsConfig`, `ProvidersConfig`, `ChannelsConfig`, `GatewayConfig`, `ToolsConfig` -- Config sub-sections.
- `clawft_types::delegation::DelegationConfig`, `DelegationRule`, `DelegationTarget` -- Delegation routing.
- `clawft_types::routing::RoutingConfig` -- Tiered routing configuration.
- `clawft_types::event::InboundMessage`, `OutboundMessage` -- Channel message events.
- `clawft_types::session::Session` -- Conversation session state.

### 2.4 Existing Services (clawft-services)

- `clawft_services::clawhub::ClawHubClient` -- Registry client with `search()`, `download()`, `install()`, `publish()`.
- `clawft_services::clawhub::SkillEntry` -- Registry skill entry with id, name, description, version, author, stars, tags, signed status.
- `clawft_services::clawhub::SkillSearchResult` -- Search result with score and match type.
- `clawft_services::clawhub::SkillInstallResult` -- Install result with success, install_path, security_scan_passed.
- `clawft_services::cron_service::CronService` -- Cron scheduler with `add_job()`, `remove_job()`, `enable_job()`, `list_jobs()`, `run_job_now()`.

---

## 3. Deliverables

### 3.1 S2.1: Live Canvas

#### 3.1.1 Rust: `CanvasCommand` Protocol (clawft-types)

**File**: `crates/clawft-types/src/canvas.rs`

Add `pub mod canvas;` to `crates/clawft-types/src/lib.rs`.

```rust
//! Live Canvas protocol types.
//!
//! Defines the command protocol between agents and the Canvas frontend.
//! Agents push UI elements via `CanvasCommand::Render`, update them via
//! `CanvasCommand::Update`, and remove them via `CanvasCommand::Remove`.
//! The frontend sends interaction events back via `CanvasInteraction`.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a canvas element.
pub type ElementId = String;

/// Unique identifier for a canvas session (tied to an agent session).
pub type CanvasId = String;

// -- Element Types -----------------------------------------------------------

/// The kind of UI element to render on the canvas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanvasElement {
    /// Plain or markdown text block.
    Text {
        content: String,
        #[serde(default = "default_text_format")]
        format: String,
    },
    /// Clickable button.
    Button {
        label: String,
        action: String,
        #[serde(default = "default_button_variant")]
        variant: String,
        #[serde(default)]
        disabled: bool,
    },
    /// Text input field.
    Input {
        label: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default)]
        value: String,
        #[serde(default = "default_input_type")]
        input_type: String,
        #[serde(default)]
        submit_action: Option<String>,
    },
    /// Image element.
    Image {
        src: String,
        #[serde(default)]
        alt: String,
        #[serde(default)]
        width: Option<u32>,
        #[serde(default)]
        height: Option<u32>,
    },
    /// Syntax-highlighted code block.
    Code {
        code: String,
        #[serde(default)]
        language: String,
        #[serde(default)]
        filename: Option<String>,
        #[serde(default = "default_true")]
        line_numbers: bool,
    },
    /// Data table with headers and rows.
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        #[serde(default)]
        sortable: bool,
        #[serde(default)]
        filterable: bool,
    },
    /// Multi-field form.
    Form {
        #[serde(default)]
        title: Option<String>,
        fields: Vec<FormField>,
        submit_action: String,
        #[serde(default = "default_submit_label")]
        submit_label: String,
    },
    /// Chart placeholder (fully rendered in S3.2, stub in S2.1).
    Chart {
        chart_type: String,
        data: serde_json::Value,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        width: u32,
        #[serde(default = "default_chart_height")]
        height: u32,
    },
}

/// A single field within a Form element.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormField {
    pub name: String,
    pub label: String,
    #[serde(default = "default_input_type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Option for a select/dropdown field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

fn default_text_format() -> String { "plain".into() }
fn default_button_variant() -> String { "default".into() }
fn default_input_type() -> String { "text".into() }
fn default_submit_label() -> String { "Submit".into() }
fn default_chart_height() -> u32 { 300 }
fn default_true() -> bool { true }

// -- Canvas Commands ---------------------------------------------------------

/// Commands sent from agents to the canvas frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum CanvasCommand {
    Render {
        id: ElementId,
        element: CanvasElement,
        #[serde(default)]
        position: Option<u32>,
        #[serde(default)]
        class: Option<String>,
    },
    Update {
        id: ElementId,
        element: CanvasElement,
    },
    Remove {
        id: ElementId,
    },
    Reset,
    Snapshot {
        #[serde(default)]
        label: Option<String>,
    },
    Restore {
        snapshot_id: String,
    },
    Batch {
        commands: Vec<CanvasCommand>,
    },
}

// -- Canvas Interactions -----------------------------------------------------

/// Interaction events sent from the canvas frontend back to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "interaction", rename_all = "snake_case")]
pub enum CanvasInteraction {
    Click {
        element_id: ElementId,
        action: String,
    },
    InputSubmit {
        element_id: ElementId,
        value: String,
        action: Option<String>,
    },
    FormSubmit {
        element_id: ElementId,
        data: HashMap<String, serde_json::Value>,
        action: String,
    },
    TableSelect {
        element_id: ElementId,
        row_index: usize,
        row_data: Vec<String>,
    },
}

// -- Canvas State ------------------------------------------------------------

/// A rendered element with its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElementState {
    pub id: ElementId,
    pub element: CanvasElement,
    pub position: u32,
    #[serde(default)]
    pub class: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Complete canvas state (used for snapshots).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSnapshot {
    pub id: String,
    pub canvas_id: CanvasId,
    #[serde(default)]
    pub label: Option<String>,
    pub elements: Vec<CanvasElementState>,
    pub created_at: DateTime<Utc>,
}

// -- WebSocket Protocol Messages ---------------------------------------------

/// Messages sent over the `/ws/canvas` WebSocket connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CanvasWsMessage {
    Subscribe { canvas_id: CanvasId },
    Unsubscribe { canvas_id: CanvasId },
    Command {
        canvas_id: CanvasId,
        command: CanvasCommand,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    Interaction {
        canvas_id: CanvasId,
        interaction: CanvasInteraction,
        session_id: String,
    },
    State {
        canvas_id: CanvasId,
        elements: Vec<CanvasElementState>,
    },
    Error {
        message: String,
        #[serde(default)]
        code: Option<String>,
    },
}
```

Tests for the canvas types module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_element_text_serde() {
        let elem = CanvasElement::Text {
            content: "Hello world".into(),
            format: "markdown".into(),
        };
        let json = serde_json::to_string(&elem).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        let restored: CanvasElement = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, elem);
    }

    #[test]
    fn canvas_element_text_default_format() {
        let json = r#"{"type":"text","content":"hi"}"#;
        let elem: CanvasElement = serde_json::from_str(json).unwrap();
        match elem {
            CanvasElement::Text { format, .. } => assert_eq!(format, "plain"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn canvas_command_render_serde() {
        let cmd = CanvasCommand::Render {
            id: "el-1".into(),
            element: CanvasElement::Button {
                label: "Click me".into(),
                action: "btn_click".into(),
                variant: "primary".into(),
                disabled: false,
            },
            position: Some(0),
            class: Some("mt-4".into()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"command\":\"render\""));
        let restored: CanvasCommand = serde_json::from_str(&json).unwrap();
        match restored {
            CanvasCommand::Render { id, .. } => assert_eq!(id, "el-1"),
            _ => panic!("expected Render"),
        }
    }

    #[test]
    fn canvas_command_batch_serde() {
        let batch = CanvasCommand::Batch {
            commands: vec![
                CanvasCommand::Reset,
                CanvasCommand::Render {
                    id: "el-1".into(),
                    element: CanvasElement::Text {
                        content: "Fresh".into(),
                        format: "plain".into(),
                    },
                    position: None,
                    class: None,
                },
            ],
        };
        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("\"command\":\"batch\""));
    }

    #[test]
    fn canvas_interaction_form_submit_serde() {
        let mut data = HashMap::new();
        data.insert("name".into(), serde_json::json!("Alice"));
        let interaction = CanvasInteraction::FormSubmit {
            element_id: "form-1".into(),
            data,
            action: "submit_profile".into(),
        };
        let json = serde_json::to_string(&interaction).unwrap();
        assert!(json.contains("\"interaction\":\"form_submit\""));
    }

    #[test]
    fn canvas_ws_message_command_serde() {
        let msg = CanvasWsMessage::Command {
            canvas_id: "canvas-abc".into(),
            command: CanvasCommand::Reset,
            agent_id: "agent-1".into(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"command\""));
    }

    #[test]
    fn form_field_defaults() {
        let json = r#"{"name":"email","label":"Email"}"#;
        let field: FormField = serde_json::from_str(json).unwrap();
        assert_eq!(field.field_type, "text");
        assert!(!field.required);
        assert!(field.options.is_empty());
    }

    #[test]
    fn canvas_snapshot_serde() {
        let snap = CanvasSnapshot {
            id: "snap-1".into(),
            canvas_id: "canvas-1".into(),
            label: Some("initial".into()),
            elements: vec![],
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let restored: CanvasSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "snap-1");
    }
}
```

#### 3.1.2 Rust: `render_ui` Tool (clawft-tools)

**File**: `crates/clawft-tools/src/canvas_tool.rs`

Add `pub mod canvas_tool;` to `crates/clawft-tools/src/lib.rs`.

```rust
//! Canvas rendering tool.
//!
//! The `render_ui` tool allows agents to push UI elements to the Live
//! Canvas. It accepts a command type and element properties, constructs
//! a `CanvasCommand`, and broadcasts it through the canvas channel.

use std::sync::Arc;
use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError, ToolMetadata};
use clawft_types::canvas::{
    CanvasCommand, CanvasElement, CanvasWsMessage, FormField,
};
use serde_json::json;
use tokio::sync::broadcast;
use tracing::debug;

/// Tool that agents call to render UI elements on the Live Canvas.
pub struct RenderUiTool {
    canvas_tx: Arc<broadcast::Sender<CanvasWsMessage>>,
    agent_id: String,
    canvas_id: String,
}

impl RenderUiTool {
    pub fn new(
        canvas_tx: Arc<broadcast::Sender<CanvasWsMessage>>,
        agent_id: String,
        canvas_id: String,
    ) -> Self {
        Self { canvas_tx, agent_id, canvas_id }
    }
}

#[async_trait]
impl Tool for RenderUiTool {
    fn name(&self) -> &str { "render_ui" }

    fn description(&self) -> &str {
        "Render interactive UI elements on the Live Canvas. Supports text, \
         buttons, inputs, images, code blocks, tables, and forms."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "enum": ["render", "update", "remove", "reset"]
                },
                "id": { "type": "string" },
                "element_type": {
                    "type": "string",
                    "enum": ["text","button","input","image","code","table","form","chart"]
                },
                "props": { "type": "object" },
                "position": { "type": "integer" },
                "class": { "type": "string" }
            },
            "required": ["command"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            required_permission_level: Some(1),
            ..Default::default()
        }
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let cmd_str = args.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing: command".into()))?;

        let canvas_cmd = match cmd_str {
            "render" => {
                let id = require_str(&args, "id")?;
                let element = parse_element(&args)?;
                CanvasCommand::Render {
                    id,
                    element,
                    position: args.get("position").and_then(|v| v.as_u64()).map(|v| v as u32),
                    class: args.get("class").and_then(|v| v.as_str()).map(String::from),
                }
            }
            "update" => CanvasCommand::Update {
                id: require_str(&args, "id")?,
                element: parse_element(&args)?,
            },
            "remove" => CanvasCommand::Remove {
                id: require_str(&args, "id")?,
            },
            "reset" => CanvasCommand::Reset,
            other => return Err(ToolError::InvalidArgs(format!("unknown command: {other}"))),
        };

        let ws_msg = CanvasWsMessage::Command {
            canvas_id: self.canvas_id.clone(),
            command: canvas_cmd,
            agent_id: self.agent_id.clone(),
            timestamp: chrono::Utc::now(),
        };

        let receivers = self.canvas_tx.send(ws_msg)
            .map_err(|_| ToolError::ExecutionFailed("no canvas clients connected".into()))?;

        debug!(agent = %self.agent_id, canvas = %self.canvas_id, receivers, "render_ui");

        Ok(json!({ "success": true, "command": cmd_str, "receivers": receivers }))
    }
}

fn require_str(args: &serde_json::Value, field: &str) -> Result<String, ToolError> {
    args.get(field).and_then(|v| v.as_str()).map(String::from)
        .ok_or_else(|| ToolError::InvalidArgs(format!("missing: {field}")))
}

fn parse_element(args: &serde_json::Value) -> Result<CanvasElement, ToolError> {
    let et = args.get("element_type").and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidArgs("missing: element_type".into()))?;
    let p = args.get("props").cloned().unwrap_or(json!({}));

    match et {
        "text" => Ok(CanvasElement::Text {
            content: p.get("content").and_then(|v| v.as_str()).unwrap_or("").into(),
            format: p.get("format").and_then(|v| v.as_str()).unwrap_or("plain").into(),
        }),
        "button" => Ok(CanvasElement::Button {
            label: p.get("label").and_then(|v| v.as_str()).unwrap_or("Button").into(),
            action: p.get("action").and_then(|v| v.as_str()).unwrap_or("click").into(),
            variant: p.get("variant").and_then(|v| v.as_str()).unwrap_or("default").into(),
            disabled: p.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
        }),
        "input" => Ok(CanvasElement::Input {
            label: p.get("label").and_then(|v| v.as_str()).unwrap_or("").into(),
            placeholder: p.get("placeholder").and_then(|v| v.as_str()).unwrap_or("").into(),
            value: p.get("value").and_then(|v| v.as_str()).unwrap_or("").into(),
            input_type: p.get("input_type").and_then(|v| v.as_str()).unwrap_or("text").into(),
            submit_action: p.get("submit_action").and_then(|v| v.as_str()).map(String::from),
        }),
        "image" => Ok(CanvasElement::Image {
            src: p.get("src").and_then(|v| v.as_str()).unwrap_or("").into(),
            alt: p.get("alt").and_then(|v| v.as_str()).unwrap_or("").into(),
            width: p.get("width").and_then(|v| v.as_u64()).map(|v| v as u32),
            height: p.get("height").and_then(|v| v.as_u64()).map(|v| v as u32),
        }),
        "code" => Ok(CanvasElement::Code {
            code: p.get("code").and_then(|v| v.as_str()).unwrap_or("").into(),
            language: p.get("language").and_then(|v| v.as_str()).unwrap_or("").into(),
            filename: p.get("filename").and_then(|v| v.as_str()).map(String::from),
            line_numbers: p.get("line_numbers").and_then(|v| v.as_bool()).unwrap_or(true),
        }),
        "table" => Ok(CanvasElement::Table {
            headers: p.get("headers").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
            rows: p.get("rows").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
            sortable: p.get("sortable").and_then(|v| v.as_bool()).unwrap_or(false),
            filterable: p.get("filterable").and_then(|v| v.as_bool()).unwrap_or(false),
        }),
        "form" => {
            let fields: Vec<FormField> = p.get("fields")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            Ok(CanvasElement::Form {
                title: p.get("title").and_then(|v| v.as_str()).map(String::from),
                fields,
                submit_action: p.get("submit_action").and_then(|v| v.as_str()).unwrap_or("form_submit").into(),
                submit_label: p.get("submit_label").and_then(|v| v.as_str()).unwrap_or("Submit").into(),
            })
        }
        "chart" => Ok(CanvasElement::Chart {
            chart_type: p.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar").into(),
            data: p.get("data").cloned().unwrap_or(json!([])),
            title: p.get("title").and_then(|v| v.as_str()).map(String::from),
            width: p.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            height: p.get("height").and_then(|v| v.as_u64()).unwrap_or(300) as u32,
        }),
        other => Err(ToolError::InvalidArgs(format!("unknown element_type: {other}"))),
    }
}
```

#### 3.1.3 Rust: Canvas WebSocket Handler (clawft-services)

**File**: `crates/clawft-services/src/api/canvas_ws.rs`

Handles `/ws/canvas` upgrade. Manages per-connection subscriptions, fans out `CanvasWsMessage::Command` to subscribers, and routes `CanvasWsMessage::Interaction` back to agents. On subscribe, sends current element state via `CanvasStateManager`.

**File**: `crates/clawft-services/src/api/canvas_state.rs`

`CanvasStateManager` tracks current elements per canvas session. Exposes `apply_command(canvas_id, command)`, `get_elements(canvas_id)`, `get_snapshots(canvas_id)`. Protected by `Arc<RwLock<CanvasStateManager>>`.

Signatures:

```rust
pub struct CanvasStateManager {
    elements: HashMap<CanvasId, Vec<CanvasElementState>>,
    snapshots: HashMap<CanvasId, Vec<CanvasSnapshot>>,
    next_position: HashMap<CanvasId, u32>,
}

impl CanvasStateManager {
    pub fn new() -> Self;
    pub fn apply_command(&mut self, canvas_id: &str, command: &CanvasCommand);
    pub fn get_elements(&self, canvas_id: &str) -> Vec<CanvasElementState>;
    pub fn get_snapshots(&self, canvas_id: &str) -> Vec<CanvasSnapshot>;
}
```

#### 3.1.4 Canvas REST Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/canvas/:canvas_id/state` | `get_canvas_state` | -- | `{ canvas_id, elements: CanvasElementState[], element_count }` |
| GET | `/api/canvas/:canvas_id/snapshots` | `list_snapshots` | -- | `{ canvas_id, snapshots: [{ id, label, element_count, created_at }] }` |
| POST | `/api/canvas/:canvas_id/snapshots` | `create_snapshot` | `{ label? }` | `{ id, label, element_count, created_at }` |
| POST | `/api/canvas/:canvas_id/snapshots/:id/restore` | `restore_snapshot` | -- | `{ canvas_id, elements, element_count }` |

#### 3.1.5 TypeScript: Canvas Types (ui/)

**File**: `ui/src/lib/canvas-types.ts`

Mirrors the Rust `CanvasElement`, `CanvasCommand`, `CanvasInteraction`, `CanvasElementState`, `CanvasSnapshot`, and `CanvasWsMessage` types as TypeScript discriminated unions. See Section 3.1.1 for the full type definitions.

#### 3.1.6 TypeScript: Canvas Zustand Store (ui/)

**File**: `ui/src/stores/canvas-store.ts`

State:
- `canvasId: CanvasId | null`
- `elements: CanvasElementState[]`
- `snapshots: CanvasSnapshot[]`
- `isConnected: boolean`
- `undoStack: CanvasHistoryEntry[]` (max 50 entries)
- `redoStack: CanvasHistoryEntry[]`

Actions:
- `setCanvasId(id)`, `setConnected(connected)`
- `applyCommand(command)` -- Pushes current state to undo stack, applies command, clears redo stack.
- `setElements(elements)`, `setSnapshots(snapshots)`, `addSnapshot(snapshot)`
- `undo()` -- Pops from undo, pushes current to redo.
- `redo()` -- Pops from redo, pushes current to undo.
- `canUndo()`, `canRedo()` -- Boolean accessors.
- `reset()` -- Clears all state.

#### 3.1.7 TypeScript: CanvasRenderer Component (ui/)

**File**: `ui/src/components/canvas/CanvasRenderer.tsx`

Props: `elements: CanvasElementState[]`, `onInteraction: (CanvasInteraction) => void`.

Renders a sorted list of elements using `ElementDispatch`, which switches on `element.type` to render the appropriate sub-component:

| Element Type | Component File | Key Props |
|-------------|---------------|-----------|
| text | `elements/CanvasTextElement.tsx` | content, format |
| button | `elements/CanvasButtonElement.tsx` | label, variant, disabled, onClick |
| input | `elements/CanvasInputElement.tsx` | label, placeholder, value, inputType, onSubmit |
| image | `elements/CanvasImageElement.tsx` | src, alt, width, height |
| code | `elements/CanvasCodeElement.tsx` | code, language, filename, lineNumbers |
| table | `elements/CanvasTableElement.tsx` | headers, rows, sortable, filterable, onRowSelect |
| form | `elements/CanvasFormElement.tsx` | title, fields, submitLabel, onSubmit |
| chart | `elements/CanvasChartPlaceholder.tsx` | chartType, title (stub: "Chart support in S3.2") |

#### 3.1.8 TypeScript: Canvas Route with Split View (ui/)

**File**: `ui/src/routes/canvas.tsx`

Layout: `ResizablePanelGroup` (horizontal) with Canvas panel (60% default) and Chat panel (40% default). Toolbar with undo, redo, snapshot, and clear buttons. Connection status indicator.

#### 3.1.9 TypeScript: `useCanvasWs` Hook (ui/)

**File**: `ui/src/hooks/use-canvas-ws.ts`

```typescript
interface UseCanvasWsReturn {
  sendInteraction: (interaction: CanvasInteraction) => void;
  takeSnapshot: (label?: string) => void;
  isConnected: boolean;
}

function useCanvasWs(canvasId: CanvasId | null): UseCanvasWsReturn;
```

Connects to `/ws/canvas`, sends `subscribe` on connect, processes incoming `command` messages by calling `canvasStore.applyCommand()`, processes `state` messages by calling `canvasStore.setElements()`. Sends `interaction` messages when user interacts with canvas elements.

---

### 3.2 S2.2: Skill Browser

#### 3.2.1 Skill API Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/skills` | `list_skills` | `?search=&limit=&offset=` | `{ skills: InstalledSkill[], total }` |
| GET | `/api/skills/:name` | `get_skill` | -- | `InstalledSkill` |
| DELETE | `/api/skills/:name` | `uninstall_skill` | -- | `{ success, message }` |
| GET | `/api/skills/hub/search` | `hub_search` | `?q=&limit=&offset=` | `{ skills: SkillEntry[], total, offset, limit }` |
| POST | `/api/skills/hub/install` | `hub_install` | `{ skill_id }` | `SkillInstallResult` |

Response types:

```rust
/// Installed skill (combines SkillDefinition with runtime info).
#[derive(Serialize)]
pub struct InstalledSkill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub format: String,           // "legacy" | "skill_md"
    pub source_path: Option<String>,
    pub variables: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub user_invocable: bool,
    pub installed_at: Option<DateTime<Utc>>,
}
```

The `hub_search` handler proxies to `ClawHubClient::search()` and returns the `ApiResponse<Vec<SkillEntry>>`. The `hub_install` handler calls `ClawHubClient::install()` and emits a `skill:installed` WS event for hot-reload notification.

#### 3.2.2 TypeScript: Skill Types (ui/)

**File**: `ui/src/lib/skill-types.ts`

```typescript
export interface InstalledSkill {
  name: string;
  description: string;
  version: string;
  format: "legacy" | "skill_md";
  source_path?: string;
  variables: string[];
  allowed_tools: string[];
  user_invocable: boolean;
  installed_at?: string;
}

export interface HubSkillEntry {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  stars: number;
  signed: boolean;
  published_at: string;
  tags: string[];
}

export interface SkillInstallResult {
  success: boolean;
  install_path?: string;
  security_scan_passed?: boolean;
  error?: string;
}
```

#### 3.2.3 TypeScript: SkillCard Component (ui/)

**File**: `ui/src/components/skills/SkillCard.tsx`

Props: `skill: InstalledSkill | HubSkillEntry`, `variant: "installed" | "hub"`, `onInstall?`, `onUninstall?`, `installing?: boolean`.

Displays: name, version badge, description (truncated to 2 lines), tags as badges, author (hub only), star count (hub only), format badge (installed only), action button (install/uninstall).

#### 3.2.4 TypeScript: Skills Route (ui/)

**File**: `ui/src/routes/skills.tsx`

Layout: Two tabs -- "Installed" and "ClawHub".

Installed tab: Search input + card grid. Fetches `GET /api/skills`. Uninstall button triggers `DELETE /api/skills/:name` with confirmation dialog.

ClawHub tab: Search input with debounced fetch to `GET /api/skills/hub/search?q=`. Results as card grid. Install button triggers `POST /api/skills/hub/install` with progress toast.

---

### 3.3 S2.3: Memory Explorer

#### 3.3.1 Memory API Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/memory` | `list_entries` | `?namespace=&limit=&offset=` | `{ entries: MemoryEntry[], total }` |
| GET | `/api/memory/:key` | `get_entry` | `?namespace=` | `MemoryEntry` |
| POST | `/api/memory` | `write_entry` | `WriteMemoryRequest` | `MemoryEntry` |
| DELETE | `/api/memory/:key` | `delete_entry` | `?namespace=` | `{ success, message }` |
| POST | `/api/memory/search` | `search_entries` | `SearchMemoryRequest` | `{ results: MemorySearchResult[], total }` |

Request/response types:

```rust
#[derive(Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub namespace: String,
    pub tags: Vec<String>,
    pub ttl_seconds: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct WriteMemoryRequest {
    pub key: String,
    pub value: String,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

fn default_namespace() -> String { "default".into() }

#[derive(Deserialize)]
pub struct SearchMemoryRequest {
    pub query: String,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_limit() -> usize { 20 }
fn default_threshold() -> f32 { 0.0 }

#[derive(Serialize)]
pub struct MemorySearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
    pub match_type: String,  // "vector" | "keyword"
}
```

#### 3.3.2 TypeScript: Memory Types (ui/)

**File**: `ui/src/lib/memory-types.ts`

```typescript
export interface MemoryEntry {
  key: string;
  value: string;
  namespace: string;
  tags: string[];
  ttl_seconds?: number;
  created_at: string;
  updated_at: string;
}

export interface WriteMemoryRequest {
  key: string;
  value: string;
  namespace?: string;
  tags?: string[];
  ttl_seconds?: number;
}

export interface SearchMemoryRequest {
  query: string;
  namespace?: string;
  limit?: number;
  threshold?: number;
}

export interface MemorySearchResult {
  entry: MemoryEntry;
  score: number;
  match_type: "vector" | "keyword";
}
```

#### 3.3.3 TypeScript: MemoryExplorer Route (ui/)

**File**: `ui/src/routes/memory.tsx`

Layout: Toolbar with search input, namespace dropdown, threshold slider (0.0-1.0), "New Entry" button. DataTable below with columns: key, namespace, tags (badge list), value (truncated), updated_at.

Component hierarchy:
- `MemoryExplorer` (route component)
  - `MemoryToolbar` -- search input, namespace filter dropdown, threshold slider, "New Entry" button
  - `MemoryTable` -- DataTable with sortable columns, row click to expand
  - `MemoryEntryDetail` -- Sheet/dialog showing full key, value (with copy button), namespace, tags, TTL, timestamps
  - `WriteMemoryDialog` -- Dialog with form: key (text input), value (textarea), namespace (select/input combo), tags (tag input), TTL (number input with unit select)

---

### 3.4 S2.4: Configuration Editor

#### 3.4.1 Config API Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/config` | `get_config` | -- | `ConfigResponse` (sanitized: no secrets) |
| PATCH | `/api/config` | `update_config` | `PatchConfigRequest` | `ConfigResponse` |
| POST | `/api/config/validate` | `validate_config` | `PatchConfigRequest` | `{ valid, errors: ValidationError[] }` |
| GET | `/api/config/schema` | `get_config_schema` | -- | JSON Schema for Config |

Types:

```rust
/// Config response with secrets redacted.
#[derive(Serialize)]
pub struct ConfigResponse {
    pub agents: AgentsConfig,
    pub channels: serde_json::Value,  // Serialized with secrets masked
    pub providers: serde_json::Value, // Serialized with secrets masked
    pub gateway: GatewayConfig,
    pub tools: serde_json::Value,     // Serialized with secrets masked
    pub delegation: DelegationConfig,
    pub routing: RoutingConfig,
}

/// Partial config update (all fields optional).
#[derive(Deserialize)]
pub struct PatchConfigRequest {
    #[serde(default)]
    pub agents: Option<serde_json::Value>,
    #[serde(default)]
    pub channels: Option<serde_json::Value>,
    #[serde(default)]
    pub providers: Option<serde_json::Value>,
    #[serde(default)]
    pub gateway: Option<serde_json::Value>,
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    #[serde(default)]
    pub delegation: Option<serde_json::Value>,
    #[serde(default)]
    pub routing: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct ValidationError {
    pub path: String,       // JSON path: "agents.defaults.model"
    pub message: String,
    pub severity: String,   // "error" | "warning"
}
```

Security: The `update_config` handler MUST reject writes to `providers.*.api_key` and `channels.*.token` fields. These can only be set via environment variables or direct config file editing.

#### 3.4.2 TypeScript: Config Types (ui/)

**File**: `ui/src/lib/config-types.ts`

Mirrors `ConfigResponse`, `PatchConfigRequest`, `ValidationError`. Each config section has its own Zod schema for client-side validation.

#### 3.4.3 TypeScript: ConfigEditor Route (ui/)

**File**: `ui/src/routes/config.tsx`

Layout: Tabs component with sections: Agents, Providers, Channels, Tools, Routing, Delegation. Each tab renders a schema-driven form using `react-hook-form` with `@hookform/resolvers/zod`.

Component hierarchy:
- `ConfigEditor` (route component)
  - `ConfigTabs` -- shadcn Tabs with one tab per config section
  - `ConfigSection` -- Generic section renderer using `react-hook-form`
    - `AgentsSection` -- model, max_tokens, temperature, workspace sliders/inputs
    - `ProvidersSection` -- Provider cards with api_key (masked, read-only), api_base input
    - `ChannelsSection` -- Channel enable toggles, allow_from lists
    - `ToolsSection` -- MCP server list, command policy, URL policy toggles
    - `RoutingSection` -- Tier editor, permission level configs
    - `DelegationSection` -- Rule list editor, target selectors
  - `ConfigDiffDialog` -- Before-save dialog showing JSON diff (unified format)
  - `ConfigSaveButton` -- Validates via `POST /api/config/validate`, shows diff, submits `PATCH /api/config`

#### 3.4.4 TypeScript: Diff Viewer Component (ui/)

**File**: `ui/src/components/common/DiffViewer.tsx`

Props: `oldValue: string`, `newValue: string`, `language?: string`.

Renders a unified diff view using a lightweight diff library (e.g. `diff` npm package). Shows additions in green, removals in red, with line numbers. Used by ConfigDiffDialog.

---

### 3.5 S2.5: Cron + Channels

#### 3.5.1 Cron API Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/cron` | `list_cron_jobs` | -- | `{ jobs: CronJob[] }` |
| POST | `/api/cron` | `create_cron_job` | `CreateCronRequest` | `CronJob` |
| DELETE | `/api/cron/:id` | `delete_cron_job` | -- | `{ success, message }` |
| PATCH | `/api/cron/:id` | `update_cron_job` | `UpdateCronRequest` | `CronJob` |
| POST | `/api/cron/:id/run` | `run_cron_job` | -- | `{ success, message }` |

Types:

```rust
#[derive(Deserialize)]
pub struct CreateCronRequest {
    pub name: String,
    pub schedule: String,  // cron expression or interval
    pub prompt: String,
    #[serde(default)]
    pub payload: Option<CronPayload>,
}

#[derive(Deserialize)]
pub struct UpdateCronRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}
```

Note: These handlers delegate to the existing `CronService` methods. The response format uses the existing `clawft_types::cron::CronJob` type directly.

#### 3.5.2 Channel API Endpoints (clawft-services)

Routes added to the API router:

| Method | Path | Handler | Request | Response |
|--------|------|---------|---------|----------|
| GET | `/api/channels` | `list_channels` | -- | `{ channels: ChannelStatus[] }` |
| GET | `/api/channels/:name` | `get_channel` | -- | `ChannelStatus` |
| POST | `/api/channels/:name/restart` | `restart_channel` | -- | `{ success, message }` |

Types:

```rust
#[derive(Serialize)]
pub struct ChannelStatus {
    pub name: String,
    pub enabled: bool,
    pub connected: bool,
    pub message_count: u64,
    pub last_message_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    /// Agent routing: which agents handle this channel.
    pub assigned_agents: Vec<String>,
}
```

Real-time updates: The `/ws` handler broadcasts `channel:status` events when channel connection state changes. Frontend subscribes to `channels` topic.

WS event format:

```json
{
  "topic": "channels",
  "event": "channel:status",
  "data": {
    "name": "telegram",
    "connected": true,
    "message_count": 142,
    "last_message_at": "2026-02-23T10:30:00Z"
  }
}
```

#### 3.5.3 TypeScript: CronDashboard Route (ui/)

**File**: `ui/src/routes/cron.tsx`

Layout: "New Job" button + DataTable. Columns: name, expression, next fire (relative time), last run (relative time + status badge), enabled (toggle switch), actions (run now, delete).

Component hierarchy:
- `CronDashboard` (route component)
  - `CronJobTable` -- DataTable with columns above
  - `CreateCronDialog` -- Dialog with name input, cron expression input (with human-readable preview using `cronstrue`), prompt textarea
  - `CronExpressionInput` -- Input with real-time validation and next-5-fires preview

#### 3.5.4 TypeScript: ChannelStatus Route (ui/)

**File**: `ui/src/routes/channels.tsx`

Layout: Channel cards in a grid. Each card shows: channel name, enabled badge, connection status (green/red dot), message count, last message timestamp, assigned agents list, restart button.

Real-time: Subscribes to `channels` WS topic. Updates card state on `channel:status` events.

Routing visualization: Below the cards, a simple table mapping `channel -> agent` based on the routing configuration.

---

## 4. Tasks

| ID | Task | Phase | Est. | Depends On |
|----|------|-------|------|------------|
| S2.1.1 | Define `CanvasElement`, `CanvasCommand`, `CanvasInteraction` in `clawft-types/src/canvas.rs` | S2.1 | 3h | S1.1 |
| S2.1.2 | Define `CanvasWsMessage`, `CanvasElementState`, `CanvasSnapshot` in `canvas.rs` + tests | S2.1 | 2h | S2.1.1 |
| S2.1.3 | Implement `RenderUiTool` in `clawft-tools/src/canvas_tool.rs` + register in bootstrap | S2.1 | 4h | S2.1.1 |
| S2.1.4 | Implement `CanvasStateManager` in `clawft-services/src/api/canvas_state.rs` | S2.1 | 3h | S2.1.1 |
| S2.1.5 | Implement Canvas WS handler (`/ws/canvas`) in `clawft-services/src/api/canvas_ws.rs` | S2.1 | 5h | S2.1.2, S2.1.4 |
| S2.1.6 | Implement Canvas REST endpoints (state, snapshots) | S2.1 | 3h | S2.1.4 |
| S2.1.7 | Create TypeScript canvas types + Zustand canvas store with undo/redo | S2.1 | 4h | S2.1.1 |
| S2.1.8 | Build `CanvasRenderer` + all 8 element sub-components | S2.1 | 8h | S2.1.7 |
| S2.1.9 | Build Canvas route with split view + `useCanvasWs` hook + MSW handlers | S2.1 | 5h | S2.1.7, S2.1.8 |
| S2.2.1 | Implement skill listing endpoint (`GET /api/skills`) | S2.2 | 3h | S1.1 |
| S2.2.2 | Implement ClawHub proxy endpoint (`GET /api/skills/hub/search`) | S2.2 | 2h | S2.2.1 |
| S2.2.3 | Implement skill install endpoint (`POST /api/skills/hub/install`) | S2.2 | 3h | S2.2.2 |
| S2.2.4 | Implement skill uninstall endpoint (`DELETE /api/skills/:name`) | S2.2 | 2h | S2.2.1 |
| S2.2.5 | Build `SkillCard` component + TypeScript types | S2.2 | 3h | S2.2.1 |
| S2.2.6 | Build Skills route with installed/hub tabs + MSW handlers | S2.2 | 4h | S2.2.5 |
| S2.3.1 | Implement memory CRUD endpoints (`GET/POST/DELETE /api/memory/*`) | S2.3 | 4h | S1.1 |
| S2.3.2 | Implement memory search endpoint (`POST /api/memory/search`) with VectorStore integration | S2.3 | 4h | S2.3.1 |
| S2.3.3 | Build `MemoryTable` + `MemoryEntryDetail` components | S2.3 | 3h | S2.3.1 |
| S2.3.4 | Build `WriteMemoryDialog` with tag input + TTL selector | S2.3 | 3h | S2.3.3 |
| S2.3.5 | Build Memory route with search, namespace filter, threshold slider + MSW handlers | S2.3 | 4h | S2.3.3, S2.3.4 |
| S2.4.1 | Implement config read/write endpoints (`GET/PATCH /api/config`) with secret redaction | S2.4 | 4h | S1.1 |
| S2.4.2 | Implement config validation endpoint (`POST /api/config/validate`) | S2.4 | 3h | S2.4.1 |
| S2.4.3 | Create Zod schemas for each config section | S2.4 | 4h | S2.4.1 |
| S2.4.4 | Build `ConfigEditor` route with tabbed sections + react-hook-form | S2.4 | 6h | S2.4.3 |
| S2.4.5 | Build `DiffViewer` component + `ConfigDiffDialog` | S2.4 | 3h | S2.4.4 |
| S2.5.1 | Implement cron CRUD endpoints (`GET/POST/DELETE/PATCH /api/cron/*`) | S2.5 | 3h | S1.1 |
| S2.5.2 | Implement channel status endpoints (`GET /api/channels`) + WS events | S2.5 | 3h | S2.5.1 |
| S2.5.3 | Build `CronDashboard` route with expression editor + MSW handlers | S2.5 | 4h | S2.5.1 |
| S2.5.4 | Build `ChannelStatus` route with real-time WS updates + routing visualization | S2.5 | 4h | S2.5.2 |

**Total: 29 tasks, estimated 112 hours**

---

## 5. Tests

### 5.1 Canvas Rendering Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.1.01 | `CanvasElement::Text` serializes with `type: "text"` tag | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.02 | `CanvasElement::Text` default format is "plain" | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.03 | `CanvasElement::Button` round-trips with variant and disabled | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.04 | `CanvasElement::Form` deserializes with field defaults | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.05 | `CanvasCommand::Render` serializes with `command: "render"` tag | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.06 | `CanvasCommand::Batch` contains nested commands | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.07 | `CanvasInteraction::FormSubmit` serializes with `interaction: "form_submit"` | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.08 | `CanvasWsMessage::Command` round-trips with timestamp | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.09 | `CanvasSnapshot` serializes with elements and created_at | Unit | `clawft-types/src/canvas.rs` |
| T-2.1.10 | `FormField` defaults: field_type="text", required=false, options=[] | Unit | `clawft-types/src/canvas.rs` |

### 5.2 Canvas Tool Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.1.11 | `RenderUiTool` renders text element and broadcasts | Unit | `clawft-tools/src/canvas_tool.rs` |
| T-2.1.12 | `RenderUiTool` rejects missing command field | Unit | `clawft-tools/src/canvas_tool.rs` |
| T-2.1.13 | `RenderUiTool` rejects unknown element_type | Unit | `clawft-tools/src/canvas_tool.rs` |
| T-2.1.14 | `RenderUiTool` reset command requires no id | Unit | `clawft-tools/src/canvas_tool.rs` |
| T-2.1.15 | `parse_element` handles all 8 element types | Unit | `clawft-tools/src/canvas_tool.rs` |

### 5.3 Canvas State Manager Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.1.16 | `apply_command(Render)` adds element to state | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.17 | `apply_command(Update)` modifies existing element | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.18 | `apply_command(Remove)` removes element by id | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.19 | `apply_command(Reset)` clears all elements | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.20 | `apply_command(Snapshot)` saves and restores state | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.21 | `apply_command(Batch)` applies multiple commands in order | Unit | `clawft-services/src/api/canvas_state.rs` |
| T-2.1.22 | Position auto-increments when not specified | Unit | `clawft-services/src/api/canvas_state.rs` |

### 5.4 Canvas Frontend Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.1.23 | `CanvasRenderer` renders text element | Component | `ui/src/components/canvas/__tests__/CanvasRenderer.test.tsx` |
| T-2.1.24 | `CanvasRenderer` renders button and fires click interaction | Component | `ui/src/components/canvas/__tests__/CanvasRenderer.test.tsx` |
| T-2.1.25 | `CanvasRenderer` renders input and fires submit interaction | Component | `ui/src/components/canvas/__tests__/CanvasRenderer.test.tsx` |
| T-2.1.26 | `CanvasRenderer` renders table with sortable columns | Component | `ui/src/components/canvas/__tests__/CanvasRenderer.test.tsx` |
| T-2.1.27 | `CanvasRenderer` renders form and collects field data on submit | Component | `ui/src/components/canvas/__tests__/CanvasRenderer.test.tsx` |
| T-2.1.28 | Canvas store undo restores previous state | Unit | `ui/src/stores/__tests__/canvas-store.test.ts` |
| T-2.1.29 | Canvas store redo re-applies undone state | Unit | `ui/src/stores/__tests__/canvas-store.test.ts` |
| T-2.1.30 | Canvas store applyCommand(reset) clears elements | Unit | `ui/src/stores/__tests__/canvas-store.test.ts` |

### 5.5 Skill Browser Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.2.01 | `GET /api/skills` returns installed skills list | Integration | `clawft-services/tests/api/skills.rs` |
| T-2.2.02 | `GET /api/skills/hub/search?q=coding` proxies to ClawHub | Integration | `clawft-services/tests/api/skills.rs` |
| T-2.2.03 | `POST /api/skills/hub/install` downloads and installs skill | Integration | `clawft-services/tests/api/skills.rs` |
| T-2.2.04 | `DELETE /api/skills/:name` removes installed skill | Integration | `clawft-services/tests/api/skills.rs` |
| T-2.2.05 | `SkillCard` displays name, version, description | Component | `ui/src/components/skills/__tests__/SkillCard.test.tsx` |
| T-2.2.06 | Skills route switches between installed and hub tabs | Component | `ui/src/routes/__tests__/skills.test.tsx` |
| T-2.2.07 | Install button shows progress toast and refreshes list | Component | `ui/src/routes/__tests__/skills.test.tsx` |

### 5.6 Memory Explorer Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.3.01 | `GET /api/memory` returns entries with pagination | Integration | `clawft-services/tests/api/memory.rs` |
| T-2.3.02 | `POST /api/memory` creates entry with key, value, namespace | Integration | `clawft-services/tests/api/memory.rs` |
| T-2.3.03 | `DELETE /api/memory/:key` removes entry | Integration | `clawft-services/tests/api/memory.rs` |
| T-2.3.04 | `POST /api/memory/search` returns results with similarity scores | Integration | `clawft-services/tests/api/memory.rs` |
| T-2.3.05 | `POST /api/memory/search` respects namespace filter | Integration | `clawft-services/tests/api/memory.rs` |
| T-2.3.06 | `MemoryTable` renders entries with key, namespace, tags columns | Component | `ui/src/routes/__tests__/memory.test.tsx` |
| T-2.3.07 | `WriteMemoryDialog` submits with all fields | Component | `ui/src/components/__tests__/WriteMemoryDialog.test.tsx` |
| T-2.3.08 | Threshold slider filters search results by minimum score | Component | `ui/src/routes/__tests__/memory.test.tsx` |

### 5.7 Config Editor Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.4.01 | `GET /api/config` returns config with secrets masked | Integration | `clawft-services/tests/api/config.rs` |
| T-2.4.02 | `PATCH /api/config` rejects writes to api_key fields | Integration | `clawft-services/tests/api/config.rs` |
| T-2.4.03 | `POST /api/config/validate` returns errors for invalid model name | Integration | `clawft-services/tests/api/config.rs` |
| T-2.4.04 | `POST /api/config/validate` returns warnings for deprecated fields | Integration | `clawft-services/tests/api/config.rs` |
| T-2.4.05 | ConfigEditor renders all 6 tabs | Component | `ui/src/routes/__tests__/config.test.tsx` |
| T-2.4.06 | AgentsSection validates temperature is 0.0-2.0 | Component | `ui/src/routes/__tests__/config.test.tsx` |
| T-2.4.07 | DiffViewer shows additions in green and removals in red | Component | `ui/src/components/__tests__/DiffViewer.test.tsx` |
| T-2.4.08 | Config save flow: edit -> validate -> diff -> confirm -> PATCH | E2E | `ui/src/routes/__tests__/config.test.tsx` |

### 5.8 Cron + Channels Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.5.01 | `GET /api/cron` returns all cron jobs | Integration | `clawft-services/tests/api/cron.rs` |
| T-2.5.02 | `POST /api/cron` creates job with valid cron expression | Integration | `clawft-services/tests/api/cron.rs` |
| T-2.5.03 | `POST /api/cron` rejects invalid cron expression with 400 | Integration | `clawft-services/tests/api/cron.rs` |
| T-2.5.04 | `DELETE /api/cron/:id` removes job | Integration | `clawft-services/tests/api/cron.rs` |
| T-2.5.05 | `POST /api/cron/:id/run` fires job immediately | Integration | `clawft-services/tests/api/cron.rs` |
| T-2.5.06 | `GET /api/channels` returns channel status list | Integration | `clawft-services/tests/api/channels.rs` |
| T-2.5.07 | CronDashboard renders job list with expression and next fire | Component | `ui/src/routes/__tests__/cron.test.tsx` |
| T-2.5.08 | CronExpressionInput validates and shows human-readable preview | Component | `ui/src/components/__tests__/CronExpressionInput.test.tsx` |
| T-2.5.09 | ChannelStatus cards update on WS `channel:status` events | Component | `ui/src/routes/__tests__/channels.test.tsx` |

### 5.9 Integration Tests

| ID | Test | Type | File |
|----|------|------|------|
| T-2.I.01 | Canvas WS: subscribe -> receive state -> render command -> interaction -> agent receives | Integration | `clawft-services/tests/ws/canvas.rs` |
| T-2.I.02 | Canvas WS: snapshot -> reset -> restore -> elements match snapshot | Integration | `clawft-services/tests/ws/canvas.rs` |
| T-2.I.03 | Skill hot-reload: install skill -> WS `skill:installed` event fires | Integration | `clawft-services/tests/api/skills.rs` |
| T-2.I.04 | Canvas renders 100 elements within 16ms (60fps performance) | Performance | `ui/src/components/canvas/__tests__/perf.test.tsx` |
| T-2.I.05 | Full flow: agent calls render_ui -> canvas WS -> frontend renders -> user clicks -> agent receives interaction | E2E | `ui/tests/e2e/canvas.spec.ts` |

**Total: 55 test specifications**

---

## 6. Exit Criteria

### 6.1 S2.1 Live Canvas

- [ ] `CanvasElement` enum with 8 variants compiles and round-trips through serde
- [ ] `CanvasCommand` enum with render/update/remove/reset/snapshot/restore/batch compiles
- [ ] `CanvasInteraction` enum with click/input_submit/form_submit/table_select compiles
- [ ] `CanvasWsMessage` protocol covers subscribe/unsubscribe/command/interaction/state/error
- [ ] `render_ui` tool is registered in ToolRegistry and callable by agents
- [ ] `/ws/canvas` handler routes commands from agents to subscribed frontends
- [ ] `/ws/canvas` handler routes interactions from frontends back to agents
- [ ] `CanvasStateManager` tracks elements per session with correct add/update/remove/reset
- [ ] Canvas REST endpoints serve state and snapshots
- [ ] `CanvasRenderer` renders all 8 element types (text, button, input, image, code, table, form, chart placeholder)
- [ ] Button clicks fire `CanvasInteraction::Click` back through WebSocket
- [ ] Form submissions fire `CanvasInteraction::FormSubmit` with collected field data
- [ ] Canvas store supports undo/redo with max 50 history entries
- [ ] Split view (Canvas + Chat) uses `ResizablePanel` with drag handle
- [ ] Canvas renders 100 elements within 16ms (60fps target)
- [ ] MSW mock handlers exist for all canvas endpoints

### 6.2 S2.2 Skill Browser

- [ ] `GET /api/skills` returns installed skills with name, version, description, format
- [ ] `GET /api/skills/hub/search` proxies to ClawHub and returns results
- [ ] `POST /api/skills/hub/install` downloads skill and emits `skill:installed` WS event
- [ ] `DELETE /api/skills/:name` removes installed skill
- [ ] `SkillCard` displays skill metadata in card format
- [ ] Installed tab shows card grid with search filtering
- [ ] ClawHub tab shows search results with install button
- [ ] Install action shows progress toast and refreshes installed list
- [ ] Uninstall action requires confirmation dialog

### 6.3 S2.3 Memory Explorer

- [ ] `GET /api/memory` returns entries with key, namespace, tags, timestamps
- [ ] `POST /api/memory` creates entry with key, value, namespace, tags, TTL
- [ ] `DELETE /api/memory/:key` removes entry
- [ ] `POST /api/memory/search` returns results ranked by similarity score
- [ ] Namespace filter restricts search and list to selected namespace
- [ ] Threshold slider (0.0-1.0) filters search results by minimum score
- [ ] `WriteMemoryDialog` creates entries with all fields
- [ ] Delete action requires confirmation dialog

### 6.4 S2.4 Configuration Editor

- [ ] `GET /api/config` returns config with all secret fields masked
- [ ] `PATCH /api/config` applies partial updates and persists to disk
- [ ] `PATCH /api/config` rejects writes to secret fields (api_key, token)
- [ ] `POST /api/config/validate` returns validation errors with JSON paths
- [ ] ConfigEditor renders all 6 tabs (agents, providers, channels, tools, routing, delegation)
- [ ] Inline editing validates against Zod schemas with field-level errors
- [ ] Diff view shows additions/removals before save
- [ ] Save flow: edit -> validate -> diff -> confirm -> PATCH

### 6.5 S2.5 Cron + Channels

- [ ] `GET /api/cron` returns job list from CronService
- [ ] `POST /api/cron` creates job with validated cron expression
- [ ] `POST /api/cron` rejects invalid expressions with 400 and error message
- [ ] `DELETE /api/cron/:id` removes job
- [ ] `POST /api/cron/:id/run` fires job immediately
- [ ] `GET /api/channels` returns channel status with connection state
- [ ] CronDashboard displays expression, next fire time, last run status
- [ ] CronExpressionInput validates and shows human-readable description
- [ ] Channel cards show real-time connection state via WS events
- [ ] Channel routing visualization maps channels to assigned agents

---

## 7. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Canvas WS message volume overwhelms browser at high element count | Medium | Medium | **6** | Batch commands reduce message count. Canvas store uses `requestAnimationFrame` batching for DOM updates. Performance test (T-2.I.04) enforces 60fps at 100 elements. |
| Canvas undo/redo stack consumes excessive memory with large snapshots | Low | Medium | **3** | History stack capped at 50 entries. Elements stored as references where possible. Older entries evicted FIFO. |
| ClawHub API unavailable during skill browser development | Medium | Low | **3** | MSW mock handlers provide full skill search and install responses. Mock data includes realistic skill entries with all fields. |
| Memory search quality depends on H2 VectorStore availability | Medium | Medium | **6** | Keyword-based fallback search (already implemented in `clawft-tools/src/memory_tool.rs`). UI shows `match_type` badge so users know whether results are semantic or keyword. |
| Config PATCH allows writing sensitive fields | Low | Critical | **5** | Server-side blocklist for `providers.*.api_key`, `channels.*.token`, and `tools.web.search.api_key` paths. Integration test T-2.4.02 verifies rejection. |
| Cron expression parsing differs between Rust and frontend display | Medium | Low | **4** | Backend validates using the `cron` crate. Frontend uses `cronstrue` for display only, not validation. All validation is server-side. |
| Canvas element XSS via agent-generated content | Low | Critical | **5** | All text content rendered through React's default escaping. Markdown rendered with `rehype-sanitize`. No `dangerouslySetInnerHTML` on agent content. Image `src` validated against allowlist (http/https/data: only). |
| ResizablePanel layout breaks on small screens | Medium | Low | **3** | Minimum panel sizes enforced (30% canvas, 20% chat). Below 768px breakpoint, panels stack vertically instead of horizontally. |
| Config schema drift between Rust types and Zod schemas | Medium | Medium | **6** | Zod schemas generated from a shared JSON Schema (served by `GET /api/config/schema`). CI test validates Zod schemas match the backend schema. |

---

## 8. Detailed Component Specifications

This section provides full code specifications for frontend components and hooks that are central to each feature area. These serve as implementation contracts.

### 8.1 Canvas Element Sub-Components

#### 8.1.1 CanvasTextElement

**File**: `ui/src/components/canvas/elements/CanvasTextElement.tsx`

```tsx
interface CanvasTextElementProps {
  content: string;
  format?: "plain" | "markdown" | "heading";
}

export function CanvasTextElement({ content, format = "plain" }: CanvasTextElementProps) {
  // "plain" -> <p> with whitespace-pre-wrap
  // "markdown" -> rendered via react-markdown with rehype-sanitize
  // "heading" -> <h2> with font-semibold
}
```

Dependencies: `react-markdown`, `rehype-sanitize`, `remark-gfm`.

Security: Markdown rendering MUST use `rehype-sanitize` to strip `<script>`, `<iframe>`, `<object>`, `<embed>`, `onload`, `onerror`, and all event handler attributes.

#### 8.1.2 CanvasButtonElement

**File**: `ui/src/components/canvas/elements/CanvasButtonElement.tsx`

```tsx
import { Button } from "@/components/ui/button";

interface CanvasButtonElementProps {
  label: string;
  variant?: "default" | "primary" | "destructive" | "outline" | "ghost";
  disabled?: boolean;
  onClick: () => void;
}

export function CanvasButtonElement({
  label,
  variant = "default",
  disabled = false,
  onClick,
}: CanvasButtonElementProps) {
  // Maps canvas variant to shadcn Button variant.
  // "primary" maps to "default" with className="bg-primary".
  return (
    <Button
      variant={variant === "primary" ? "default" : variant}
      disabled={disabled}
      onClick={onClick}
    >
      {label}
    </Button>
  );
}
```

#### 8.1.3 CanvasInputElement

**File**: `ui/src/components/canvas/elements/CanvasInputElement.tsx`

```tsx
import { useState, useCallback, KeyboardEvent } from "react";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";

interface CanvasInputElementProps {
  label: string;
  placeholder?: string;
  value?: string;
  inputType?: "text" | "number" | "email" | "password" | "textarea";
  onSubmit: (value: string) => void;
}

export function CanvasInputElement({
  label,
  placeholder = "",
  value: initialValue = "",
  inputType = "text",
  onSubmit,
}: CanvasInputElementProps) {
  const [value, setValue] = useState(initialValue);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        onSubmit(value);
      }
    },
    [value, onSubmit]
  );

  // inputType "textarea" renders Textarea, others render Input.
  // Label is always rendered above the input.
  // Submit on Enter key (except textarea uses Shift+Enter).
}
```

#### 8.1.4 CanvasImageElement

**File**: `ui/src/components/canvas/elements/CanvasImageElement.tsx`

```tsx
interface CanvasImageElementProps {
  src: string;
  alt?: string;
  width?: number;
  height?: number;
}

export function CanvasImageElement({
  src,
  alt = "",
  width,
  height,
}: CanvasImageElementProps) {
  // Validates src starts with "http://", "https://", or "data:image/".
  // Rejects other protocols (javascript:, file:, etc.).
  // Renders <img> with optional width/height, rounded corners, max-w-full.
  // Shows fallback placeholder on load error.
}
```

Security: Image `src` MUST be validated to only allow `https://`, `http://`, and `data:image/` schemes. Any other scheme renders a warning placeholder.

#### 8.1.5 CanvasCodeElement

**File**: `ui/src/components/canvas/elements/CanvasCodeElement.tsx`

```tsx
interface CanvasCodeElementProps {
  code: string;
  language?: string;
  filename?: string;
  lineNumbers?: boolean;
}

export function CanvasCodeElement({
  code,
  language = "",
  filename,
  lineNumbers = true,
}: CanvasCodeElementProps) {
  // Renders code block with syntax highlighting via shiki or prism-react-renderer.
  // Optional filename header bar with copy button.
  // Line numbers shown in gutter when lineNumbers=true.
  // Max height 400px with overflow-y-auto scroll.
}
```

Dependencies: `shiki` (preferred, tree-shakeable) or `prism-react-renderer` as fallback.

#### 8.1.6 CanvasTableElement

**File**: `ui/src/components/canvas/elements/CanvasTableElement.tsx`

```tsx
interface CanvasTableElementProps {
  headers: string[];
  rows: string[][];
  sortable?: boolean;
  filterable?: boolean;
  onRowSelect: (rowIndex: number, rowData: string[]) => void;
}

export function CanvasTableElement({
  headers,
  rows,
  sortable = false,
  filterable = false,
  onRowSelect,
}: CanvasTableElementProps) {
  // Uses shadcn Table components.
  // If sortable, column headers are clickable with sort indicator.
  // If filterable, renders a search input above the table.
  // Rows are clickable; onClick fires onRowSelect with row index and data.
  // Max height 500px with sticky header and overflow-y-auto.
}
```

#### 8.1.7 CanvasFormElement

**File**: `ui/src/components/canvas/elements/CanvasFormElement.tsx`

```tsx
import type { FormField } from "@/lib/canvas-types";

interface CanvasFormElementProps {
  title?: string;
  fields: FormField[];
  submitLabel?: string;
  onSubmit: (data: Record<string, unknown>) => void;
}

export function CanvasFormElement({
  title,
  fields,
  submitLabel = "Submit",
  onSubmit,
}: CanvasFormElementProps) {
  // Renders a form with fields based on FormField definitions.
  // Field rendering by field_type:
  //   "text"     -> Input
  //   "number"   -> Input type="number"
  //   "email"    -> Input type="email"
  //   "textarea" -> Textarea
  //   "select"   -> Select with options from field.options
  //   "checkbox" -> Checkbox
  //
  // Validates required fields, min/max, pattern before submit.
  // Collects all field values into a Record<string, unknown> and calls onSubmit.
  // Uses react-hook-form internally for validation.
}
```

#### 8.1.8 CanvasChartPlaceholder

**File**: `ui/src/components/canvas/elements/CanvasChartPlaceholder.tsx`

```tsx
interface CanvasChartPlaceholderProps {
  chartType: string;
  title?: string;
}

export function CanvasChartPlaceholder({ chartType, title }: CanvasChartPlaceholderProps) {
  // S2.1 stub: renders a card with chart icon, chartType label, and title.
  // Shows message: "Chart rendering available in a future update."
  // Full chart rendering implemented in S3.2.
}
```

### 8.2 Canvas WebSocket Hook

**File**: `ui/src/hooks/use-canvas-ws.ts`

```typescript
import { useEffect, useRef, useCallback } from "react";
import { useCanvasStore } from "@/stores/canvas-store";
import type { CanvasInteraction, CanvasId, CanvasWsMessage } from "@/lib/canvas-types";

interface UseCanvasWsReturn {
  sendInteraction: (interaction: CanvasInteraction) => void;
  takeSnapshot: (label?: string) => void;
  isConnected: boolean;
}

export function useCanvasWs(canvasId: CanvasId | null): UseCanvasWsReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const { applyCommand, setElements, setConnected, isConnected } = useCanvasStore();

  useEffect(() => {
    if (!canvasId) return;

    const wsUrl = `${import.meta.env.VITE_WS_URL ?? "ws://localhost:18790"}/ws/canvas`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      // Subscribe to canvas session.
      ws.send(JSON.stringify({ type: "subscribe", canvas_id: canvasId }));
    };

    ws.onclose = () => {
      setConnected(false);
    };

    ws.onmessage = (event) => {
      try {
        const msg: CanvasWsMessage = JSON.parse(event.data);
        switch (msg.type) {
          case "command":
            applyCommand(msg.command);
            break;
          case "state":
            setElements(msg.elements);
            break;
          case "error":
            console.error("[canvas-ws]", msg.message);
            break;
        }
      } catch {
        // Ignore unparseable messages.
      }
    };

    return () => {
      ws.close();
      wsRef.current = null;
      setConnected(false);
    };
  }, [canvasId, applyCommand, setElements, setConnected]);

  const sendInteraction = useCallback(
    (interaction: CanvasInteraction) => {
      if (!wsRef.current || !canvasId) return;
      const msg: CanvasWsMessage = {
        type: "interaction",
        canvas_id: canvasId,
        interaction,
        session_id: canvasId, // Same as canvas_id for now.
      };
      wsRef.current.send(JSON.stringify(msg));
    },
    [canvasId]
  );

  const takeSnapshot = useCallback(
    (label?: string) => {
      if (!wsRef.current || !canvasId) return;
      // Snapshots go through REST API, not WS.
      fetch(`${import.meta.env.VITE_API_URL}/api/canvas/${canvasId}/snapshots`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ label }),
      });
    },
    [canvasId]
  );

  return { sendInteraction, takeSnapshot, isConnected };
}
```

### 8.3 MSW Mock Handlers

**File**: `ui/src/mocks/handlers/canvas.ts`

```typescript
import { http, HttpResponse } from "msw";
import type { CanvasElementState, CanvasSnapshot } from "@/lib/canvas-types";

const mockElements: CanvasElementState[] = [
  {
    id: "welcome-text",
    element: { type: "text", content: "Welcome to the Live Canvas", format: "heading" },
    position: 0,
    updated_at: new Date().toISOString(),
  },
  {
    id: "action-btn",
    element: { type: "button", label: "Get Started", action: "get_started", variant: "primary" },
    position: 1,
    updated_at: new Date().toISOString(),
  },
];

export const canvasHandlers = [
  http.get("/api/canvas/:canvasId/state", ({ params }) => {
    return HttpResponse.json({
      canvas_id: params.canvasId,
      elements: mockElements,
      element_count: mockElements.length,
    });
  }),

  http.get("/api/canvas/:canvasId/snapshots", ({ params }) => {
    return HttpResponse.json({
      canvas_id: params.canvasId,
      snapshots: [],
    });
  }),

  http.post("/api/canvas/:canvasId/snapshots", async ({ params, request }) => {
    const body = (await request.json()) as { label?: string };
    return HttpResponse.json({
      id: `snap-${Date.now()}`,
      label: body.label,
      element_count: mockElements.length,
      created_at: new Date().toISOString(),
    });
  }),
];
```

**File**: `ui/src/mocks/handlers/skills.ts`

```typescript
import { http, HttpResponse } from "msw";
import type { InstalledSkill, HubSkillEntry } from "@/lib/skill-types";

const installedSkills: InstalledSkill[] = [
  {
    name: "coding-assistant",
    description: "AI coding assistant with file editing and shell access",
    version: "2.1.0",
    format: "skill_md",
    source_path: "/home/user/.clawft/skills/coding-assistant",
    variables: ["language", "framework"],
    allowed_tools: ["read_file", "write_file", "shell_exec"],
    user_invocable: true,
    installed_at: "2026-02-20T10:00:00Z",
  },
  {
    name: "web-search",
    description: "Search the web and summarize results",
    version: "1.0.0",
    format: "legacy",
    variables: [],
    allowed_tools: ["web_search", "web_fetch"],
    user_invocable: true,
    installed_at: "2026-02-18T08:00:00Z",
  },
];

const hubSkills: HubSkillEntry[] = [
  {
    id: "hub-skill-001",
    name: "data-analyst",
    description: "Analyze datasets with charts and statistical summaries",
    version: "1.2.0",
    author: "clawft-community",
    stars: 87,
    signed: true,
    published_at: "2026-02-15T12:00:00Z",
    tags: ["data", "analytics", "charts"],
  },
  {
    id: "hub-skill-002",
    name: "devops-helper",
    description: "Manage deployments, CI/CD pipelines, and infrastructure",
    version: "3.0.1",
    author: "infrastructure-team",
    stars: 124,
    signed: true,
    published_at: "2026-02-10T09:00:00Z",
    tags: ["devops", "ci-cd", "deploy"],
  },
];

export const skillHandlers = [
  http.get("/api/skills", ({ request }) => {
    const url = new URL(request.url);
    const search = url.searchParams.get("search") ?? "";
    const filtered = search
      ? installedSkills.filter(
          (s) =>
            s.name.includes(search) || s.description.includes(search)
        )
      : installedSkills;
    return HttpResponse.json({ skills: filtered, total: filtered.length });
  }),

  http.get("/api/skills/:name", ({ params }) => {
    const skill = installedSkills.find((s) => s.name === params.name);
    if (!skill) return HttpResponse.json({ error: "not found" }, { status: 404 });
    return HttpResponse.json(skill);
  }),

  http.delete("/api/skills/:name", ({ params }) => {
    return HttpResponse.json({ success: true, message: `Uninstalled ${params.name}` });
  }),

  http.get("/api/skills/hub/search", ({ request }) => {
    const url = new URL(request.url);
    const q = url.searchParams.get("q") ?? "";
    const filtered = q
      ? hubSkills.filter(
          (s) => s.name.includes(q) || s.tags.some((t) => t.includes(q))
        )
      : hubSkills;
    return HttpResponse.json({
      skills: filtered,
      total: filtered.length,
      offset: 0,
      limit: 20,
    });
  }),

  http.post("/api/skills/hub/install", async ({ request }) => {
    const body = (await request.json()) as { skill_id: string };
    return HttpResponse.json({
      success: true,
      install_path: `/home/user/.clawft/skills/${body.skill_id}`,
      security_scan_passed: true,
    });
  }),
];
```

**File**: `ui/src/mocks/handlers/memory.ts`

```typescript
import { http, HttpResponse } from "msw";
import type { MemoryEntry } from "@/lib/memory-types";

const memoryEntries: MemoryEntry[] = [
  {
    key: "auth-pattern",
    value: "Use JWT with refresh tokens. Access token TTL 15min, refresh 7d.",
    namespace: "patterns",
    tags: ["auth", "jwt", "security"],
    created_at: "2026-02-20T10:00:00Z",
    updated_at: "2026-02-20T10:00:00Z",
  },
  {
    key: "project-goals",
    value: "Ship MVP by Q1 2026. Focus on core agent loop and WebChat.",
    namespace: "planning",
    tags: ["goals", "timeline"],
    created_at: "2026-02-18T08:00:00Z",
    updated_at: "2026-02-22T14:30:00Z",
  },
  {
    key: "rust-patterns",
    value: "Prefer Arc<RwLock<T>> for shared state. Use channels for cross-task communication.",
    namespace: "patterns",
    tags: ["rust", "concurrency"],
    created_at: "2026-02-15T12:00:00Z",
    updated_at: "2026-02-15T12:00:00Z",
  },
];

export const memoryHandlers = [
  http.get("/api/memory", ({ request }) => {
    const url = new URL(request.url);
    const ns = url.searchParams.get("namespace");
    const filtered = ns
      ? memoryEntries.filter((e) => e.namespace === ns)
      : memoryEntries;
    return HttpResponse.json({ entries: filtered, total: filtered.length });
  }),

  http.get("/api/memory/:key", ({ params, request }) => {
    const url = new URL(request.url);
    const ns = url.searchParams.get("namespace") ?? "default";
    const entry = memoryEntries.find(
      (e) => e.key === params.key && e.namespace === ns
    );
    if (!entry) return HttpResponse.json({ error: "not found" }, { status: 404 });
    return HttpResponse.json(entry);
  }),

  http.post("/api/memory", async ({ request }) => {
    const body = (await request.json()) as {
      key: string;
      value: string;
      namespace?: string;
      tags?: string[];
      ttl_seconds?: number;
    };
    const now = new Date().toISOString();
    const entry: MemoryEntry = {
      key: body.key,
      value: body.value,
      namespace: body.namespace ?? "default",
      tags: body.tags ?? [],
      ttl_seconds: body.ttl_seconds,
      created_at: now,
      updated_at: now,
    };
    return HttpResponse.json(entry, { status: 201 });
  }),

  http.delete("/api/memory/:key", () => {
    return HttpResponse.json({ success: true, message: "Entry deleted" });
  }),

  http.post("/api/memory/search", async ({ request }) => {
    const body = (await request.json()) as {
      query: string;
      namespace?: string;
      limit?: number;
      threshold?: number;
    };
    const threshold = body.threshold ?? 0.0;
    const results = memoryEntries
      .filter((e) => !body.namespace || e.namespace === body.namespace)
      .map((entry) => ({
        entry,
        score: entry.value.toLowerCase().includes(body.query.toLowerCase()) ? 0.85 : 0.2,
        match_type: "keyword" as const,
      }))
      .filter((r) => r.score >= threshold)
      .sort((a, b) => b.score - a.score)
      .slice(0, body.limit ?? 20);
    return HttpResponse.json({ results, total: results.length });
  }),
];
```

**File**: `ui/src/mocks/handlers/config.ts`

```typescript
import { http, HttpResponse } from "msw";

const mockConfig = {
  agents: {
    defaults: {
      workspace: "~/.clawft/workspace",
      model: "anthropic/claude-opus-4-5",
      max_tokens: 8192,
      temperature: 0.7,
      max_tool_iterations: 20,
      memory_window: 50,
    },
  },
  channels: {
    telegram: { enabled: false, token: "***", allow_from: [] },
    slack: { enabled: false, token: "***", mode: "socket" },
    discord: { enabled: false, token: "***" },
  },
  providers: {
    anthropic: { api_key: "***", api_base: null },
    openai: { api_key: "***", api_base: null },
    openrouter: { api_key: "***", api_base: "https://openrouter.ai/api/v1" },
  },
  gateway: { host: "0.0.0.0", port: 18790 },
  tools: {
    web: { search: { api_key: "***", max_results: 5 } },
    exec: { timeout: 60 },
    restrict_to_workspace: false,
    mcp_servers: {},
    command_policy: { mode: "allowlist", allowlist: [], denylist: [] },
    url_policy: { enabled: true, allow_private: false },
  },
  delegation: {
    claude_enabled: true,
    claude_model: "claude-sonnet-4-20250514",
    max_turns: 10,
    max_tokens: 4096,
    rules: [],
  },
  routing: {
    mode: "static",
    tiers: [],
    permissions: {},
    escalation: { enabled: false },
  },
};

export const configHandlers = [
  http.get("/api/config", () => {
    return HttpResponse.json(mockConfig);
  }),

  http.patch("/api/config", async ({ request }) => {
    const patch = (await request.json()) as Record<string, unknown>;
    // Check for forbidden secret fields.
    if (patch.providers) {
      const providers = patch.providers as Record<string, Record<string, unknown>>;
      for (const [, provider] of Object.entries(providers)) {
        if ("api_key" in provider) {
          return HttpResponse.json(
            { error: "Cannot update api_key via API. Use environment variables." },
            { status: 403 }
          );
        }
      }
    }
    return HttpResponse.json({ ...mockConfig, ...patch });
  }),

  http.post("/api/config/validate", async ({ request }) => {
    const patch = (await request.json()) as Record<string, unknown>;
    const errors: Array<{ path: string; message: string; severity: string }> = [];

    // Example validation: temperature range.
    if (patch.agents) {
      const agents = patch.agents as { defaults?: { temperature?: number } };
      const temp = agents.defaults?.temperature;
      if (temp !== undefined && (temp < 0 || temp > 2)) {
        errors.push({
          path: "agents.defaults.temperature",
          message: "Temperature must be between 0.0 and 2.0",
          severity: "error",
        });
      }
    }

    return HttpResponse.json({ valid: errors.length === 0, errors });
  }),

  http.get("/api/config/schema", () => {
    // Returns a simplified JSON Schema for the Config type.
    return HttpResponse.json({
      type: "object",
      properties: {
        agents: { type: "object" },
        channels: { type: "object" },
        providers: { type: "object" },
        gateway: { type: "object" },
        tools: { type: "object" },
        delegation: { type: "object" },
        routing: { type: "object" },
      },
    });
  }),
];
```

**File**: `ui/src/mocks/handlers/cron.ts`

```typescript
import { http, HttpResponse } from "msw";
import type { CronJob } from "@/lib/cron-types";

const mockJobs: CronJob[] = [
  {
    id: "job-001",
    name: "Daily Report",
    enabled: true,
    schedule: { kind: "cron", expr: "0 9 * * *", tz: "UTC" },
    payload: { kind: "agent_turn", message: "Generate daily summary report", deliver: false },
    state: {
      next_run_at: new Date(Date.now() + 3600000).toISOString(),
      last_run_at: new Date(Date.now() - 86400000).toISOString(),
      last_status: "ok",
    },
    created_at: "2026-02-15T10:00:00Z",
    updated_at: "2026-02-15T10:00:00Z",
    delete_after_run: false,
  },
  {
    id: "job-002",
    name: "Hourly Health Check",
    enabled: true,
    schedule: { kind: "cron", expr: "0 * * * *", tz: "UTC" },
    payload: { kind: "agent_turn", message: "Run system health check", deliver: true, channel: "slack" },
    state: {
      next_run_at: new Date(Date.now() + 1800000).toISOString(),
      last_run_at: new Date(Date.now() - 1800000).toISOString(),
      last_status: "ok",
    },
    created_at: "2026-02-10T08:00:00Z",
    updated_at: "2026-02-22T14:00:00Z",
    delete_after_run: false,
  },
];

export const cronHandlers = [
  http.get("/api/cron", () => {
    return HttpResponse.json({ jobs: mockJobs });
  }),

  http.post("/api/cron", async ({ request }) => {
    const body = (await request.json()) as {
      name: string;
      schedule: string;
      prompt: string;
    };
    // Validate cron expression (basic check).
    const parts = body.schedule.trim().split(/\s+/);
    if (parts.length < 5) {
      return HttpResponse.json(
        { error: `Invalid cron expression: expected at least 5 fields, got ${parts.length}` },
        { status: 400 }
      );
    }
    const newJob: CronJob = {
      id: `job-${Date.now()}`,
      name: body.name,
      enabled: true,
      schedule: { kind: "cron", expr: body.schedule, tz: "UTC" },
      payload: { kind: "agent_turn", message: body.prompt, deliver: false },
      state: { next_run_at: new Date(Date.now() + 3600000).toISOString() },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      delete_after_run: false,
    };
    return HttpResponse.json(newJob, { status: 201 });
  }),

  http.delete("/api/cron/:id", ({ params }) => {
    return HttpResponse.json({ success: true, message: `Deleted job ${params.id}` });
  }),

  http.patch("/api/cron/:id", async ({ params, request }) => {
    const body = (await request.json()) as { enabled?: boolean };
    const job = mockJobs.find((j) => j.id === params.id);
    if (!job) return HttpResponse.json({ error: "not found" }, { status: 404 });
    return HttpResponse.json({ ...job, ...body });
  }),

  http.post("/api/cron/:id/run", ({ params }) => {
    return HttpResponse.json({ success: true, message: `Fired job ${params.id}` });
  }),
];
```

**File**: `ui/src/mocks/handlers/channels.ts`

```typescript
import { http, HttpResponse } from "msw";

const mockChannels = [
  {
    name: "telegram",
    enabled: true,
    connected: true,
    message_count: 1247,
    last_message_at: "2026-02-23T09:45:00Z",
    error: null,
    assigned_agents: ["default"],
  },
  {
    name: "slack",
    enabled: false,
    connected: false,
    message_count: 0,
    last_message_at: null,
    error: null,
    assigned_agents: [],
  },
  {
    name: "discord",
    enabled: true,
    connected: false,
    message_count: 89,
    last_message_at: "2026-02-22T18:30:00Z",
    error: "Connection timeout after 30s",
    assigned_agents: ["default"],
  },
  {
    name: "webchat",
    enabled: true,
    connected: true,
    message_count: 456,
    last_message_at: "2026-02-23T10:12:00Z",
    error: null,
    assigned_agents: ["default"],
  },
];

export const channelHandlers = [
  http.get("/api/channels", () => {
    return HttpResponse.json({ channels: mockChannels });
  }),

  http.get("/api/channels/:name", ({ params }) => {
    const channel = mockChannels.find((c) => c.name === params.name);
    if (!channel) return HttpResponse.json({ error: "not found" }, { status: 404 });
    return HttpResponse.json(channel);
  }),

  http.post("/api/channels/:name/restart", ({ params }) => {
    return HttpResponse.json({ success: true, message: `Restarting ${params.name}` });
  }),
];
```

### 8.4 TypeScript: Cron Types

**File**: `ui/src/lib/cron-types.ts`

```typescript
export interface CronJob {
  id: string;
  name: string;
  enabled: boolean;
  schedule: CronSchedule;
  payload: CronPayload;
  state: CronJobState;
  created_at: string;
  updated_at: string;
  delete_after_run: boolean;
}

export interface CronSchedule {
  kind: "at" | "every" | "cron";
  at_ms?: number;
  every_ms?: number;
  expr?: string;
  tz?: string;
}

export interface CronPayload {
  kind: "system_event" | "agent_turn";
  message: string;
  deliver: boolean;
  channel?: string;
  to?: string;
}

export interface CronJobState {
  next_run_at?: string;
  last_run_at?: string;
  last_status?: "ok" | "error" | "skipped";
  last_error?: string;
}

export interface CreateCronRequest {
  name: string;
  schedule: string;
  prompt: string;
}
```

### 8.5 TypeScript: Zod Schemas for Config Validation

**File**: `ui/src/lib/config-schemas.ts`

```typescript
import { z } from "zod";

export const agentDefaultsSchema = z.object({
  workspace: z.string().min(1),
  model: z.string().min(1),
  max_tokens: z.number().int().min(1).max(200000),
  temperature: z.number().min(0).max(2),
  max_tool_iterations: z.number().int().min(1).max(100),
  memory_window: z.number().int().min(1).max(500),
});

export const agentsConfigSchema = z.object({
  defaults: agentDefaultsSchema,
});

export const gatewayConfigSchema = z.object({
  host: z.string().min(1),
  port: z.number().int().min(1).max(65535),
  heartbeat_interval_minutes: z.number().int().min(0).optional(),
  heartbeat_prompt: z.string().optional(),
});

export const delegationRuleSchema = z.object({
  pattern: z.string().min(1),
  target: z.enum(["local", "claude", "flow", "auto"]),
});

export const delegationConfigSchema = z.object({
  claude_enabled: z.boolean(),
  claude_model: z.string().min(1),
  max_turns: z.number().int().min(1).max(100),
  max_tokens: z.number().int().min(1).max(200000),
  claude_flow_enabled: z.boolean(),
  rules: z.array(delegationRuleSchema),
  excluded_tools: z.array(z.string()),
});

export const commandPolicySchema = z.object({
  mode: z.enum(["allowlist", "denylist"]),
  allowlist: z.array(z.string()),
  denylist: z.array(z.string()),
});

export const urlPolicySchema = z.object({
  enabled: z.boolean(),
  allow_private: z.boolean(),
  allowed_domains: z.array(z.string()),
  blocked_domains: z.array(z.string()),
});

// Maps config section names to their schemas.
export const configSectionSchemas: Record<string, z.ZodSchema> = {
  agents: agentsConfigSchema,
  gateway: gatewayConfigSchema,
  delegation: delegationConfigSchema,
};
```

### 8.6 TypeScript: DiffViewer Component

**File**: `ui/src/components/common/DiffViewer.tsx`

```tsx
import { useMemo } from "react";
import { diffLines, type Change } from "diff";
import { cn } from "@/lib/utils";

interface DiffViewerProps {
  oldValue: string;
  newValue: string;
  oldLabel?: string;
  newLabel?: string;
}

export function DiffViewer({
  oldValue,
  newValue,
  oldLabel = "Current",
  newLabel = "Proposed",
}: DiffViewerProps) {
  const changes = useMemo(
    () => diffLines(oldValue, newValue),
    [oldValue, newValue]
  );

  const hasChanges = changes.some((c) => c.added || c.removed);

  if (!hasChanges) {
    return (
      <div className="text-muted-foreground rounded-md border p-4 text-sm">
        No changes detected.
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-md border">
      <div className="flex border-b bg-muted/50 px-3 py-1.5 text-xs">
        <span className="flex-1 text-red-600">{oldLabel}</span>
        <span className="flex-1 text-green-600">{newLabel}</span>
      </div>
      <pre className="overflow-x-auto p-0 text-sm">
        {changes.map((change, i) => (
          <DiffLine key={i} change={change} />
        ))}
      </pre>
    </div>
  );
}

function DiffLine({ change }: { change: Change }) {
  const bg = change.added
    ? "bg-green-50 dark:bg-green-950/30"
    : change.removed
    ? "bg-red-50 dark:bg-red-950/30"
    : "";

  const prefix = change.added ? "+" : change.removed ? "-" : " ";
  const textColor = change.added
    ? "text-green-700 dark:text-green-400"
    : change.removed
    ? "text-red-700 dark:text-red-400"
    : "";

  return (
    <div className={cn("px-3 py-0", bg)}>
      {change.value.split("\n").filter(Boolean).map((line, j) => (
        <div key={j} className={cn("font-mono", textColor)}>
          <span className="mr-2 select-none opacity-50">{prefix}</span>
          {line}
        </div>
      ))}
    </div>
  );
}
```

Dependencies: `diff` npm package (ISC license, ~7KB gzipped).

### 8.7 TypeScript: CronExpressionInput Component

**File**: `ui/src/components/cron/CronExpressionInput.tsx`

```tsx
import { useState, useEffect, useMemo } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface CronExpressionInputProps {
  value: string;
  onChange: (value: string) => void;
  error?: string;
}

export function CronExpressionInput({
  value,
  onChange,
  error,
}: CronExpressionInputProps) {
  // Validates the cron expression format (5 or 6 fields).
  // Shows human-readable description using cronstrue.
  // Shows next 5 fire times calculated client-side.
  // Highlights invalid fields in red.

  const description = useMemo(() => {
    try {
      // Uses cronstrue for human-readable: "At 09:00 AM, every day"
      // Lazy-imported to reduce bundle size.
      return describeExpression(value);
    } catch {
      return null;
    }
  }, [value]);

  const isValid = useMemo(() => {
    const parts = value.trim().split(/\s+/);
    return parts.length >= 5 && parts.length <= 7;
  }, [value]);

  return (
    <div className="space-y-2">
      <Label>Cron Expression</Label>
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="0 9 * * *"
        className={!isValid && value ? "border-red-500" : ""}
      />
      {description && (
        <p className="text-muted-foreground text-sm">{description}</p>
      )}
      {error && <p className="text-sm text-red-500">{error}</p>}
    </div>
  );
}

function describeExpression(expr: string): string {
  // Placeholder: in implementation, use cronstrue package.
  const parts = expr.trim().split(/\s+/);
  if (parts.length < 5) throw new Error("Invalid");
  return `Cron: ${expr}`;
}
```

Dependencies: `cronstrue` npm package (MIT license, ~5KB gzipped). Lazy-imported via dynamic `import()` to avoid adding to initial bundle.

---

## 9. Dependency Summary

### 9.1 Rust Crate Dependencies (new in S2)

| Crate | Version | Used By | Purpose |
|-------|---------|---------|---------|
| (none) | -- | -- | All new types use existing serde/chrono/uuid deps |

No new Rust dependencies required. The canvas types use `serde`, `chrono`, and `uuid` which are already in the dependency tree.

### 9.2 Frontend Dependencies (new in S2)

| Package | Version | Size (gzip) | Purpose |
|---------|---------|-------------|---------|
| `react-markdown` | ^9.x | ~12KB | Markdown rendering in canvas text elements |
| `rehype-sanitize` | ^6.x | ~3KB | XSS protection for markdown |
| `remark-gfm` | ^4.x | ~2KB | GitHub Flavored Markdown support |
| `shiki` | ^1.x | ~15KB (core) | Syntax highlighting for code elements |
| `diff` | ^7.x | ~7KB | Unified diff generation for config editor |
| `cronstrue` | ^2.x | ~5KB | Human-readable cron descriptions |
| `react-hook-form` | ^7.x | ~9KB | Form management for config editor + canvas forms |
| `@hookform/resolvers` | ^3.x | ~2KB | Zod resolver for react-hook-form |
| `zod` | ^3.x | ~13KB | Schema validation for config sections |

Total additional bundle impact: ~68KB gzipped. All packages support tree-shaking. `shiki` and `cronstrue` are lazy-loaded via dynamic imports to keep the initial bundle under the 200KB budget.

### 9.3 File Inventory

New files created in S2:

**Rust (clawft-types)**:
- `crates/clawft-types/src/canvas.rs`

**Rust (clawft-tools)**:
- `crates/clawft-tools/src/canvas_tool.rs`

**Rust (clawft-services)**:
- `crates/clawft-services/src/api/canvas_ws.rs`
- `crates/clawft-services/src/api/canvas_state.rs`
- `crates/clawft-services/src/api/canvas_handlers.rs`
- `crates/clawft-services/src/api/skill_handlers.rs`
- `crates/clawft-services/src/api/memory_handlers.rs`
- `crates/clawft-services/src/api/config_handlers.rs`
- `crates/clawft-services/src/api/cron_handlers.rs`
- `crates/clawft-services/src/api/channel_handlers.rs`

**TypeScript (ui/src/lib)**:
- `ui/src/lib/canvas-types.ts`
- `ui/src/lib/skill-types.ts`
- `ui/src/lib/memory-types.ts`
- `ui/src/lib/config-types.ts`
- `ui/src/lib/config-schemas.ts`
- `ui/src/lib/cron-types.ts`

**TypeScript (ui/src/stores)**:
- `ui/src/stores/canvas-store.ts`

**TypeScript (ui/src/hooks)**:
- `ui/src/hooks/use-canvas-ws.ts`

**TypeScript (ui/src/components)**:
- `ui/src/components/canvas/CanvasRenderer.tsx`
- `ui/src/components/canvas/elements/CanvasTextElement.tsx`
- `ui/src/components/canvas/elements/CanvasButtonElement.tsx`
- `ui/src/components/canvas/elements/CanvasInputElement.tsx`
- `ui/src/components/canvas/elements/CanvasImageElement.tsx`
- `ui/src/components/canvas/elements/CanvasCodeElement.tsx`
- `ui/src/components/canvas/elements/CanvasTableElement.tsx`
- `ui/src/components/canvas/elements/CanvasFormElement.tsx`
- `ui/src/components/canvas/elements/CanvasChartPlaceholder.tsx`
- `ui/src/components/skills/SkillCard.tsx`
- `ui/src/components/common/DiffViewer.tsx`
- `ui/src/components/cron/CronExpressionInput.tsx`

**TypeScript (ui/src/routes)**:
- `ui/src/routes/canvas.tsx`
- `ui/src/routes/skills.tsx`
- `ui/src/routes/memory.tsx`
- `ui/src/routes/config.tsx`
- `ui/src/routes/cron.tsx`
- `ui/src/routes/channels.tsx`

**TypeScript (ui/src/mocks)**:
- `ui/src/mocks/handlers/canvas.ts`
- `ui/src/mocks/handlers/skills.ts`
- `ui/src/mocks/handlers/memory.ts`
- `ui/src/mocks/handlers/config.ts`
- `ui/src/mocks/handlers/cron.ts`
- `ui/src/mocks/handlers/channels.ts`

**Total new files: ~42**
