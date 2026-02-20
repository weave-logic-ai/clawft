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
// IdeDispatcher trait
// ---------------------------------------------------------------------------

/// Callback for IDE tool execution.
///
/// The IDE extension provides an implementation that forwards tool
/// calls to the actual editor. For testing, a mock can be provided.
pub type IdeDispatchFn =
    dyn Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync;

// ---------------------------------------------------------------------------
// IdeToolProvider
// ---------------------------------------------------------------------------

/// A [`ToolProvider`] that exposes IDE-specific tools via MCP.
///
/// Created by the integration layer when a VS Code extension connects.
/// The dispatcher is provided by the extension backend and routes
/// tool calls to the actual IDE.
pub struct IdeToolProvider {
    tools: Vec<ToolDefinition>,
    dispatcher: Arc<IdeDispatchFn>,
}

impl IdeToolProvider {
    /// Create a new IDE tool provider with a custom dispatcher.
    pub fn new<F>(dispatcher: F) -> Self
    where
        F: Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            tools: ide_tool_definitions(),
            dispatcher: Arc::new(dispatcher),
        }
    }

    /// Create a provider with a no-op dispatcher (useful for listing tools
    /// without an actual IDE connection).
    pub fn stub() -> Self {
        Self::new(|name, _args| {
            let name = name.to_string();
            Box::pin(async move {
                Err(format!("IDE not connected: cannot execute '{name}'"))
            })
        })
    }
}

impl std::fmt::Debug for IdeToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdeToolProvider")
            .field("tool_count", &self.tools.len())
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

        let fut = (self.dispatcher)(name, args);
        match fut.await {
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
}
