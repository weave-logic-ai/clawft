//! MCP IDE integration -- VS Code extension backend.
//!
//! Provides a [`ToolProvider`] implementation that exposes IDE-specific
//! tools through the MCP server. These tools allow agents to interact
//! with a connected IDE (VS Code or compatible editor) via tool calls.
//!
//! # Architecture
//!
//! `IdeToolProvider` implements [`ToolProvider`] and is registered with
//! [`CompositeToolProvider`] alongside builtin and skill providers.
//! This makes IDE tools appear in `tools/list` and callable via
//! `tools/call`.
//!
//! # Tools
//!
//! | Tool | Description |
//! |------|-------------|
//! | `ide_open_file` | Open a file in the editor |
//! | `ide_edit` | Apply a text edit to an open document |
//! | `ide_diagnostics` | Get current diagnostics (errors, warnings) |
//! | `ide_symbols` | Search workspace symbols |
//! | `ide_hover` | Get hover information for a position |

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ToolDefinition;
use super::provider::{CallToolResult, ToolError, ToolProvider};

// ---------------------------------------------------------------------------
// IDE tool definitions
// ---------------------------------------------------------------------------

/// All IDE tool definitions.
fn ide_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "ide_open_file".into(),
            description: "Open a file in the IDE editor at an optional line number".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or workspace-relative path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number to reveal (1-based, optional)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (1-based, optional)"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "ide_edit".into(),
            description: "Apply a text edit to an open document in the IDE".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "range": {
                        "type": "object",
                        "description": "Range to replace (startLine, startCol, endLine, endCol), 0-based",
                        "properties": {
                            "startLine": { "type": "integer" },
                            "startCol": { "type": "integer" },
                            "endLine": { "type": "integer" },
                            "endCol": { "type": "integer" }
                        },
                        "required": ["startLine", "startCol", "endLine", "endCol"]
                    },
                    "text": {
                        "type": "string",
                        "description": "Replacement text"
                    }
                },
                "required": ["path", "range", "text"]
            }),
        },
        ToolDefinition {
            name: "ide_diagnostics".into(),
            description: "Get current diagnostics (errors, warnings) from the IDE".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional file path to filter diagnostics"
                    },
                    "severity": {
                        "type": "string",
                        "description": "Minimum severity to include",
                        "enum": ["error", "warning", "info", "hint"]
                    }
                }
            }),
        },
        ToolDefinition {
            name: "ide_symbols".into(),
            description: "Search workspace symbols in the IDE".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Symbol search query"
                    },
                    "kind": {
                        "type": "string",
                        "description": "Filter by symbol kind (function, class, interface, etc.)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 25
                    }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "ide_hover".into(),
            description: "Get hover information for a position in a file".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (0-based)"
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (0-based)"
                    }
                },
                "required": ["path", "line", "column"]
            }),
        },
    ]
}

// ---------------------------------------------------------------------------
// IdeDispatcher callback type
// ---------------------------------------------------------------------------

/// Callback for IDE tool execution.
///
/// The IDE extension provides an implementation that forwards tool
/// calls to the actual editor. For testing, a mock can be provided.
pub type IdeDispatchFn = dyn Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
    + Send
    + Sync;

// ---------------------------------------------------------------------------
// IdeBackend (WEFT-193)
// ---------------------------------------------------------------------------

/// Pluggable IDE execution backend.
///
/// Production hosts (VS Code extension, Cursor, JetBrains) implement this
/// (or attach via [`HotSwapIdeBackend`]) so tool calls reach a live editor.
/// Listing tools does not require a connected backend; executing them does.
#[async_trait]
pub trait IdeBackend: Send + Sync {
    /// Invoke an IDE tool by name with JSON arguments.
    async fn invoke(&self, name: &str, args: Value) -> Result<String, String>;

    /// Whether a live IDE is currently attached and reachable.
    fn is_connected(&self) -> bool;

    /// Short backend identifier for diagnostics (`"disconnected"`,
    /// `"callback"`, `"hot-swap"`, extension id, …).
    fn backend_name(&self) -> &str;
}

/// Honest disconnected backend: tools list is available, execution fails
/// with an explicit not-connected message (WEFT-193).
#[derive(Debug, Default, Clone, Copy)]
pub struct DisconnectedIdeBackend;

#[async_trait]
impl IdeBackend for DisconnectedIdeBackend {
    async fn invoke(&self, name: &str, _args: Value) -> Result<String, String> {
        Err(format!(
            "IDE backend not connected: cannot execute '{name}'. \
             Attach a live IDE via IdeToolProvider::with_backend or \
             HotSwapIdeBackend::connect."
        ))
    }

    fn is_connected(&self) -> bool {
        false
    }

    fn backend_name(&self) -> &str {
        "disconnected"
    }
}

/// Callback-backed IDE backend (connected for the lifetime of the provider).
pub struct CallbackIdeBackend {
    dispatcher: Arc<IdeDispatchFn>,
}

impl CallbackIdeBackend {
    /// Wrap a dispatcher function as a connected backend.
    pub fn new<F>(dispatcher: F) -> Self
    where
        F: Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            dispatcher: Arc::new(dispatcher),
        }
    }
}

#[async_trait]
impl IdeBackend for CallbackIdeBackend {
    async fn invoke(&self, name: &str, args: Value) -> Result<String, String> {
        (self.dispatcher)(name, args).await
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn backend_name(&self) -> &str {
        "callback"
    }
}

/// Hot-pluggable IDE backend: starts disconnected; an extension can attach
/// and detach a live dispatcher at runtime (WEFT-193 real wire path).
pub struct HotSwapIdeBackend {
    dispatcher: tokio::sync::RwLock<Option<Arc<IdeDispatchFn>>>,
}

impl HotSwapIdeBackend {
    /// Create a disconnected hot-swap backend.
    pub fn new() -> Self {
        Self {
            dispatcher: tokio::sync::RwLock::new(None),
        }
    }

    /// Attach a live IDE dispatcher (extension connected).
    pub async fn connect<F>(&self, dispatcher: F)
    where
        F: Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let mut slot = self.dispatcher.write().await;
        *slot = Some(Arc::new(dispatcher));
    }

    /// Detach the live dispatcher (extension disconnected).
    pub async fn disconnect(&self) {
        let mut slot = self.dispatcher.write().await;
        *slot = None;
    }
}

impl Default for HotSwapIdeBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IdeBackend for HotSwapIdeBackend {
    async fn invoke(&self, name: &str, args: Value) -> Result<String, String> {
        let guard = self.dispatcher.read().await;
        match guard.as_ref() {
            Some(dispatch) => dispatch(name, args).await,
            None => Err(format!(
                "IDE backend not connected: cannot execute '{name}'. \
                 Wait for the IDE extension to attach, or call HotSwapIdeBackend::connect."
            )),
        }
    }

    fn is_connected(&self) -> bool {
        // try_read avoids async in a sync method; if locked, treat as connected
        // only when we can observe a dispatcher.
        match self.dispatcher.try_read() {
            Ok(guard) => guard.is_some(),
            Err(_) => true, // write lock held → connect/disconnect in flight
        }
    }

    fn backend_name(&self) -> &str {
        "hot-swap"
    }
}

// ---------------------------------------------------------------------------
// IdeToolProvider
// ---------------------------------------------------------------------------

/// A [`ToolProvider`] that exposes IDE-specific tools via MCP.
///
/// Created by the integration layer when a VS Code extension connects.
/// The backend is provided by the extension and routes tool calls to
/// the actual IDE.
///
/// # Connection honesty (WEFT-193)
///
/// - [`IdeToolProvider::disconnected`] / [`IdeToolProvider::stub`]: list-only;
///   execution returns an explicit not-connected error.
/// - [`IdeToolProvider::new`] / [`IdeToolProvider::with_backend`]: real backends
///   that can execute tools when connected.
pub struct IdeToolProvider {
    tools: Vec<ToolDefinition>,
    backend: Arc<dyn IdeBackend>,
}

impl IdeToolProvider {
    /// Create a new IDE tool provider with a custom (connected) dispatcher.
    pub fn new<F>(dispatcher: F) -> Self
    where
        F: Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self::with_backend(Arc::new(CallbackIdeBackend::new(dispatcher)))
    }

    /// Create a provider backed by an arbitrary [`IdeBackend`].
    pub fn with_backend(backend: Arc<dyn IdeBackend>) -> Self {
        Self {
            tools: ide_tool_definitions(),
            backend,
        }
    }

    /// Honest disconnected provider: tools list is available, execution fails
    /// with a clear not-connected message (WEFT-193).
    pub fn disconnected() -> Self {
        Self::with_backend(Arc::new(DisconnectedIdeBackend))
    }

    /// Alias for [`disconnected`] — kept for call-site compatibility.
    ///
    /// Prefer [`disconnected`] in new code; this is not a pretend-working
    /// stub. Execution always reports that no IDE is attached.
    pub fn stub() -> Self {
        Self::disconnected()
    }

    /// Whether the backend reports a live IDE connection.
    pub fn is_connected(&self) -> bool {
        self.backend.is_connected()
    }

    /// Backend identifier for diagnostics.
    pub fn backend_name(&self) -> &str {
        self.backend.backend_name()
    }
}

impl std::fmt::Debug for IdeToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdeToolProvider")
            .field("tool_count", &self.tools.len())
            .field("backend", &self.backend.backend_name())
            .field("connected", &self.backend.is_connected())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ToolProvider for IdeToolProvider {
    fn namespace(&self) -> &str {
        "ide"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        self.tools.clone()
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        // Verify the tool exists.
        if !self.tools.iter().any(|t| t.name == name) {
            return Err(ToolError::NotFound(name.to_string()));
        }

        match self.backend.invoke(name, args).await {
            Ok(output) => Ok(CallToolResult::text(output)),
            Err(msg) => Ok(CallToolResult::error(msg)),
        }
    }
}

// ---------------------------------------------------------------------------
// IDE types (for structured communication with the extension)
// ---------------------------------------------------------------------------

/// A diagnostic entry from the IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeDiagnostic {
    /// File path.
    pub path: String,
    /// Diagnostic severity.
    pub severity: DiagnosticSeverity,
    /// Diagnostic message.
    pub message: String,
    /// Start line (0-based).
    pub line: usize,
    /// Start column (0-based).
    pub column: usize,
    /// Source (e.g., "typescript", "rust-analyzer").
    #[serde(default)]
    pub source: String,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// A workspace symbol from the IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeSymbol {
    /// Symbol name.
    pub name: String,
    /// Symbol kind (function, class, etc.).
    pub kind: String,
    /// File path containing the symbol.
    pub path: String,
    /// Line number (0-based).
    pub line: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ide_tool_definitions_count() {
        let tools = ide_tool_definitions();
        assert_eq!(tools.len(), 5);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"ide_open_file"));
        assert!(names.contains(&"ide_edit"));
        assert!(names.contains(&"ide_diagnostics"));
        assert!(names.contains(&"ide_symbols"));
        assert!(names.contains(&"ide_hover"));
    }

    #[test]
    fn ide_tool_schemas_are_objects() {
        let tools = ide_tool_definitions();
        for tool in &tools {
            assert!(
                tool.input_schema.is_object(),
                "schema not object for {}",
                tool.name
            );
            assert_eq!(tool.input_schema["type"], "object");
        }
    }

    #[test]
    fn namespace_is_ide() {
        let provider = IdeToolProvider::stub();
        assert_eq!(provider.namespace(), "ide");
    }

    #[test]
    fn list_tools_returns_five() {
        let provider = IdeToolProvider::stub();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 5);
    }

    #[tokio::test]
    async fn call_tool_not_found() {
        let provider = IdeToolProvider::stub();
        let result = provider
            .call_tool("nonexistent", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn call_tool_stub_returns_error_result() {
        let provider = IdeToolProvider::stub();
        let result = provider
            .call_tool("ide_open_file", serde_json::json!({"path": "/test.rs"}))
            .await
            .unwrap();
        assert!(result.is_error);
        match &result.content[0] {
            super::super::provider::ContentBlock::Text { text } => {
                assert!(
                    text.contains("not connected"),
                    "honest disconnected error expected, got: {text}"
                );
            }
        }
    }

    #[test]
    fn disconnected_is_not_connected() {
        let provider = IdeToolProvider::disconnected();
        assert!(!provider.is_connected());
        assert_eq!(provider.backend_name(), "disconnected");
    }

    #[test]
    fn callback_backend_is_connected() {
        let provider = IdeToolProvider::new(|_name, _args| {
            Box::pin(async move { Ok("ok".into()) })
        });
        assert!(provider.is_connected());
        assert_eq!(provider.backend_name(), "callback");
    }

    #[tokio::test]
    async fn hot_swap_backend_connect_and_disconnect() {
        let backend = Arc::new(HotSwapIdeBackend::new());
        assert!(!backend.is_connected());

        let provider = IdeToolProvider::with_backend(backend.clone());
        assert!(!provider.is_connected());

        let result = provider
            .call_tool("ide_open_file", serde_json::json!({"path": "/x.rs"}))
            .await
            .unwrap();
        assert!(result.is_error);

        backend
            .connect(|name, args| {
                let name = name.to_string();
                Box::pin(async move {
                    let path = args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    Ok(format!("{name}:{path}"))
                })
            })
            .await;
        assert!(provider.is_connected());

        let result = provider
            .call_tool("ide_open_file", serde_json::json!({"path": "/x.rs"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            super::super::provider::ContentBlock::Text { text } => {
                assert_eq!(text, "ide_open_file:/x.rs");
            }
        }

        backend.disconnect().await;
        assert!(!provider.is_connected());
    }

    #[tokio::test]
    async fn call_tool_with_custom_dispatcher() {
        let provider = IdeToolProvider::new(|name, args| {
            let name = name.to_string();
            Box::pin(async move {
                if name == "ide_open_file" {
                    let path = args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Ok(format!("opened: {path}"))
                } else {
                    Err(format!("unhandled: {name}"))
                }
            })
        });

        let result = provider
            .call_tool("ide_open_file", serde_json::json!({"path": "/src/main.rs"}))
            .await
            .unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            super::super::provider::ContentBlock::Text { text } => {
                assert_eq!(text, "opened: /src/main.rs");
            }
        }
    }

    #[test]
    fn debug_format() {
        let provider = IdeToolProvider::stub();
        let debug = format!("{:?}", provider);
        assert!(debug.contains("IdeToolProvider"));
        assert!(debug.contains("tool_count: 5"));
    }

    #[test]
    fn diagnostic_severity_serde() {
        let json = serde_json::to_string(&DiagnosticSeverity::Error).unwrap();
        assert_eq!(json, "\"error\"");
        let restored: DiagnosticSeverity = serde_json::from_str("\"warning\"").unwrap();
        assert_eq!(restored, DiagnosticSeverity::Warning);
    }

    #[test]
    fn ide_diagnostic_serde() {
        let diag = IdeDiagnostic {
            path: "/src/main.rs".into(),
            severity: DiagnosticSeverity::Error,
            message: "unused variable".into(),
            line: 10,
            column: 5,
            source: "rust-analyzer".into(),
        };
        let json = serde_json::to_string(&diag).unwrap();
        let restored: IdeDiagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.path, "/src/main.rs");
        assert_eq!(restored.severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn ide_symbol_serde() {
        let sym = IdeSymbol {
            name: "main".into(),
            kind: "function".into(),
            path: "/src/main.rs".into(),
            line: 1,
        };
        let json = serde_json::to_string(&sym).unwrap();
        let restored: IdeSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "main");
    }

    // ── Integration tests: backend dispatch + MCP result shape (WEFT-491) ─

    /// Verify each IDE tool routes to the right handler in a custom
    /// dispatcher and the result content is text-shaped per MCP.
    #[tokio::test]
    async fn ide_dispatcher_routes_each_tool_to_correct_handler() {
        use std::sync::Arc;
        use std::sync::Mutex;

        // Record which tool names were invoked, in order.
        let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let calls_dispatcher = calls.clone();

        let provider = IdeToolProvider::new(move |name, _args| {
            let calls = calls_dispatcher.clone();
            let name = name.to_string();
            Box::pin(async move {
                calls.lock().unwrap().push(name.clone());
                // Echo the tool name back so we can also assert the
                // payload routing.
                Ok(format!("dispatched:{name}"))
            })
        });

        for tool in [
            "ide_open_file",
            "ide_edit",
            "ide_diagnostics",
            "ide_symbols",
            "ide_hover",
        ] {
            let result = provider
                .call_tool(tool, serde_json::json!({}))
                .await
                .expect("call must not error");
            assert!(!result.is_error, "tool {tool} unexpectedly errored");
            // MCP schema: result.content is a vec; text result has
            // exactly one ContentBlock::Text.
            assert_eq!(result.content.len(), 1);
            match &result.content[0] {
                super::super::provider::ContentBlock::Text { text } => {
                    assert_eq!(text, &format!("dispatched:{tool}"));
                }
            }
        }

        // All five tools dispatched, in the order called above.
        let invoked = calls.lock().unwrap().clone();
        assert_eq!(
            invoked,
            vec![
                "ide_open_file".to_string(),
                "ide_edit".to_string(),
                "ide_diagnostics".to_string(),
                "ide_symbols".to_string(),
                "ide_hover".to_string(),
            ]
        );
    }

    /// Backend-side errors must be surfaced as `is_error=true` MCP
    /// results, not as `Err(ToolError)`. The LLM needs to see the
    /// failure in-band so it can reason about it.
    #[tokio::test]
    async fn ide_dispatcher_errors_surface_as_in_band_mcp_error() {
        let provider = IdeToolProvider::new(|name, _args| {
            let name = name.to_string();
            Box::pin(async move { Err(format!("ide-backend bailed on {name}")) })
        });

        let result = provider
            .call_tool("ide_diagnostics", serde_json::json!({"path": "/x.rs"}))
            .await
            .expect("call returns Ok with is_error=true, never Err");
        assert!(result.is_error);
        match &result.content[0] {
            super::super::provider::ContentBlock::Text { text } => {
                assert!(
                    text.contains("ide-backend bailed on ide_diagnostics"),
                    "unexpected error text: {text}"
                );
            }
        }
    }

    /// The dispatcher receives the raw `arguments` JSON object from
    /// the MCP `tools/call` request and is expected to pull fields out
    /// itself. This test verifies arguments survive the round-trip
    /// for a representative tool.
    #[tokio::test]
    async fn ide_dispatcher_receives_arguments_verbatim() {
        let provider = IdeToolProvider::new(|name, args| {
            let name = name.to_string();
            Box::pin(async move {
                if name != "ide_edit" {
                    return Err("wrong tool routed".into());
                }
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("missing path")?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or("missing text")?;
                Ok(format!("edit {path} -> {} bytes", text.len()))
            })
        });

        let result = provider
            .call_tool(
                "ide_edit",
                serde_json::json!({
                    "path": "/src/lib.rs",
                    "range": { "startLine": 0, "startCol": 0, "endLine": 0, "endCol": 0 },
                    "text": "// header"
                }),
            )
            .await
            .unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            super::super::provider::ContentBlock::Text { text } => {
                assert_eq!(text, "edit /src/lib.rs -> 9 bytes");
            }
        }
    }
}
