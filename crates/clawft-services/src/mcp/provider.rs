//! Tool provider abstraction for MCP tool delegation.
//!
//! Defines [`ToolProvider`], the trait that unifies local (builtin) and
//! remote (MCP server) tool sources behind a single interface, and
//! [`BuiltinToolProvider`], which wraps the existing
//! [`clawft_core::tools::registry::ToolRegistry`] dispatch mechanism.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ToolDefinition;

// ---------------------------------------------------------------------------
// Result / content types
// ---------------------------------------------------------------------------

/// A single content block returned by a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    #[serde(rename = "text")]
    Text { text: String },
}

/// The result of calling a tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallToolResult {
    /// Content blocks produced by the tool.
    pub content: Vec<ContentBlock>,
    /// Whether the tool execution resulted in an error.
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

impl CallToolResult {
    /// Convenience constructor for a successful text result.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
            is_error: false,
        }
    }

    /// Convenience constructor for an error text result.
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock::Text { text: text.into() }],
            is_error: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during tool provider operations.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The requested tool was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The tool execution failed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// The caller lacks permission to invoke the tool.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

// ---------------------------------------------------------------------------
// ToolProvider trait
// ---------------------------------------------------------------------------

/// Abstraction over a source of tools.
///
/// Implementors may serve tools from a local registry (see
/// [`BuiltinToolProvider`]) or from a remote MCP server.
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// Namespace prefix for this provider's tools (e.g. `"builtin"`,
    /// `"mcp:server-name"`).
    fn namespace(&self) -> &str;

    /// List the tool definitions available from this provider.
    fn list_tools(&self) -> Vec<ToolDefinition>;

    /// Execute a tool by name with the given JSON arguments.
    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError>;
}

// ---------------------------------------------------------------------------
// BuiltinToolProvider
// ---------------------------------------------------------------------------

/// Type alias for the dispatcher function used by [`BuiltinToolProvider`].
///
/// Accepts a tool name and JSON arguments, returns a future that resolves
/// to either a success string or an error string.
type DispatchFn = dyn Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
    + Send
    + Sync;

/// A [`ToolProvider`] backed by local tool definitions and a dispatch
/// function.
///
/// Rather than depending directly on `ToolRegistry` (which lives in a
/// different crate), this provider accepts a list of tool definitions
/// and a closure that dispatches execution requests. This keeps the
/// dependency graph clean and makes testing straightforward.
pub struct BuiltinToolProvider {
    tools: Vec<ToolDefinition>,
    dispatcher: Arc<DispatchFn>,
}

impl BuiltinToolProvider {
    /// Create a new builtin tool provider.
    ///
    /// # Arguments
    ///
    /// * `tools` - Tool definitions to expose via [`ToolProvider::list_tools`].
    /// * `dispatcher` - Closure that executes a tool by name. It receives
    ///   the tool name and JSON arguments, and returns `Ok(output)` on
    ///   success or `Err(message)` on failure.
    pub fn new<F>(tools: Vec<ToolDefinition>, dispatcher: F) -> Self
    where
        F: Fn(&str, Value) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            tools,
            dispatcher: Arc::new(dispatcher),
        }
    }
}

impl std::fmt::Debug for BuiltinToolProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinToolProvider")
            .field("tool_count", &self.tools.len())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ToolProvider for BuiltinToolProvider {
    fn namespace(&self) -> &str {
        "builtin"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        self.tools.clone()
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
        // Verify the tool exists in our definition list.
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a small set of tool definitions for testing.
    fn sample_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "echo".into(),
                description: "Echoes input".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}}),
            },
            ToolDefinition {
                name: "add".into(),
                description: "Adds numbers".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}}}),
            },
        ]
    }

    /// Helper: build a [`BuiltinToolProvider`] with a simple dispatcher.
    fn make_provider() -> BuiltinToolProvider {
        BuiltinToolProvider::new(sample_tools(), |name, args| {
            let name = name.to_string();
            Box::pin(async move {
                match name.as_str() {
                    "echo" => {
                        let text = args
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(empty)");
                        Ok(format!("echo: {text}"))
                    }
                    "add" => {
                        let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        Ok(format!("{}", a + b))
                    }
                    _ => Err(format!("unknown tool: {name}")),
                }
            })
        })
    }

    #[test]
    fn namespace_returns_builtin() {
        let provider = make_provider();
        assert_eq!(provider.namespace(), "builtin");
    }

    #[test]
    fn list_tools_returns_registered_tools() {
        let provider = make_provider();
        let tools = provider.list_tools();
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"add"));
    }

    #[tokio::test]
    async fn call_tool_dispatches_correctly() {
        let provider = make_provider();

        let result = provider
            .call_tool("echo", serde_json::json!({"text": "hello"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "echo: hello"),
        }
    }

    #[tokio::test]
    async fn call_tool_add_dispatches_correctly() {
        let provider = make_provider();

        let result = provider
            .call_tool("add", serde_json::json!({"a": 3.0, "b": 4.0}))
            .await
            .unwrap();

        assert!(!result.is_error);
        match &result.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "7"),
        }
    }

    #[tokio::test]
    async fn call_tool_not_found() {
        let provider = make_provider();

        let result = provider
            .call_tool("nonexistent", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected NotFound, got: {other}"),
        }
    }

    #[tokio::test]
    async fn call_tool_dispatcher_error_returns_error_result() {
        // A dispatcher that always returns Err.
        let provider = BuiltinToolProvider::new(
            vec![ToolDefinition {
                name: "broken".into(),
                description: "Always fails".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            |_name, _args| Box::pin(async { Err("intentional failure".to_string()) }),
        );

        let result = provider
            .call_tool("broken", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.is_error);
        match &result.content[0] {
            ContentBlock::Text { text } => assert_eq!(text, "intentional failure"),
        }
    }

    #[test]
    fn call_tool_result_text_convenience() {
        let result = CallToolResult::text("hello");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(
            result.content[0],
            ContentBlock::Text {
                text: "hello".into()
            }
        );
    }

    #[test]
    fn call_tool_result_error_convenience() {
        let result = CallToolResult::error("oops");
        assert!(result.is_error);
        assert_eq!(
            result.content[0],
            ContentBlock::Text {
                text: "oops".into()
            }
        );
    }

    #[test]
    fn call_tool_result_serde_roundtrip() {
        let result = CallToolResult {
            content: vec![ContentBlock::Text {
                text: "output".into(),
            }],
            is_error: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let restored: CallToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, restored);
    }

    #[test]
    fn content_block_serde_roundtrip() {
        let block = ContentBlock::Text {
            text: "hello world".into(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains(r#""type":"text""#));

        let restored: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(block, restored);
    }

    #[test]
    fn call_tool_result_is_error_defaults_false() {
        let json = r#"{"content":[{"type":"text","text":"hi"}]}"#;
        let result: CallToolResult = serde_json::from_str(json).unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn tool_error_display() {
        let err = ToolError::NotFound("missing".into());
        assert_eq!(err.to_string(), "not found: missing");

        let err = ToolError::ExecutionFailed("boom".into());
        assert_eq!(err.to_string(), "execution failed: boom");

        let err = ToolError::PermissionDenied("nope".into());
        assert_eq!(err.to_string(), "permission denied: nope");
    }

    #[test]
    fn debug_format() {
        let provider = make_provider();
        let debug = format!("{:?}", provider);
        assert!(debug.contains("BuiltinToolProvider"));
        assert!(debug.contains("tool_count: 2"));
    }
}
