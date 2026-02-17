//! Tool registry and [`Tool`] trait definition.
//!
//! Defines the interface that all tool implementations must satisfy
//! ([`Tool`]) and provides a [`ToolRegistry`] that stores registered
//! tools and dispatches execution requests by name.
//!
//! Tool implementations live in the `clawft-tools` crate; this module
//! only defines the contract and registry infrastructure.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

/// Error type for tool execution.
///
/// Covers the common failure modes: unknown tool, bad arguments,
/// runtime failures, permission issues, and timeouts.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// The requested tool was not found in the registry.
    #[error("tool not found: {0}")]
    NotFound(String),

    /// The arguments provided to the tool are invalid.
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    /// The tool execution failed at runtime.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// The caller lacks permission to invoke this tool.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// A file or resource the tool needs was not found.
    #[error("not found: {0}")]
    FileNotFound(String),

    /// A filesystem path is invalid or traverses outside allowed boundaries.
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// The tool execution exceeded the allowed time limit.
    #[error("timeout after {0}s")]
    Timeout(u64),
}

/// A tool that can be invoked by the agent pipeline.
///
/// Implementations provide a name, description, JSON Schema for parameters,
/// and an async `execute` method. Tools are registered in a [`ToolRegistry`]
/// and dispatched by the agent loop when the LLM emits a tool-use request.
///
/// # Implementing a tool
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use clawft_core::tools::registry::{Tool, ToolError};
///
/// struct EchoTool;
///
/// #[async_trait]
/// impl Tool for EchoTool {
///     fn name(&self) -> &str { "echo" }
///     fn description(&self) -> &str { "Echo back the input" }
///     fn parameters(&self) -> serde_json::Value {
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "text": { "type": "string", "description": "Text to echo" }
///             },
///             "required": ["text"]
///         })
///     }
///     async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
///         let text = args.get("text")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| ToolError::InvalidArgs("missing 'text'".into()))?;
///         Ok(serde_json::json!({ "output": text }))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// The unique name of this tool (used in LLM function calling).
    fn name(&self) -> &str;

    /// A human-readable description of what this tool does.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's parameters.
    ///
    /// Should return a valid JSON Schema object (type: "object" with
    /// properties, required, etc.) suitable for OpenAI function calling.
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments.
    ///
    /// Arguments are a JSON object matching the schema from [`parameters`].
    /// Returns a JSON value representing the tool's output, or a
    /// [`ToolError`] on failure.
    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError>;
}

/// Registry of available tools, indexed by name.
///
/// Provides lookup, listing, schema generation in OpenAI function calling
/// format, and dispatch-by-name execution.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry.
    ///
    /// If a tool with the same name already exists, it is replaced.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        debug!(tool = %name, "registering tool");
        self.tools.insert(name, tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all registered tool names (sorted alphabetically).
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    /// Generate tool schemas in OpenAI function calling format.
    ///
    /// Returns one schema object per registered tool:
    /// ```json
    /// {
    ///   "type": "function",
    ///   "function": {
    ///     "name": "tool_name",
    ///     "description": "tool description",
    ///     "parameters": { ... json schema ... }
    ///   }
    /// }
    /// ```
    ///
    /// The returned vector is sorted by tool name for deterministic output.
    pub fn schemas(&self) -> Vec<serde_json::Value> {
        let mut schemas: Vec<(String, serde_json::Value)> = self
            .tools
            .iter()
            .map(|(name, tool)| {
                let schema = serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": tool.description(),
                        "parameters": tool.parameters(),
                    }
                });
                (name.clone(), schema)
            })
            .collect();

        schemas.sort_by(|a, b| a.0.cmp(&b.0));
        schemas.into_iter().map(|(_, v)| v).collect()
    }

    /// Execute a tool by name with the given arguments.
    ///
    /// Returns [`ToolError::NotFound`] if no tool with that name is registered.
    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        debug!(tool = %name, "executing tool");
        tool.execute(args).await
    }

    /// Return the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Return true if no tools are registered.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test tool that echoes its input.
    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echo back the input text"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to echo"
                    }
                },
                "required": ["text"]
            })
        }

        async fn execute(
            &self,
            args: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArgs("missing 'text' field".into()))?;
            Ok(serde_json::json!({ "output": text }))
        }
    }

    /// A tool that always fails for testing error paths.
    struct FailTool;

    #[async_trait]
    impl Tool for FailTool {
        fn name(&self) -> &str {
            "fail"
        }

        fn description(&self) -> &str {
            "A tool that always fails"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }

        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            Err(ToolError::ExecutionFailed("intentional failure".into()))
        }
    }

    /// A math tool for testing multiple registrations.
    struct AddTool;

    #[async_trait]
    impl Tool for AddTool {
        fn name(&self) -> &str {
            "add"
        }

        fn description(&self) -> &str {
            "Add two numbers"
        }

        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "a": { "type": "number" },
                    "b": { "type": "number" }
                },
                "required": ["a", "b"]
            })
        }

        async fn execute(
            &self,
            args: serde_json::Value,
        ) -> Result<serde_json::Value, ToolError> {
            let a = args
                .get("a")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidArgs("missing 'a'".into()))?;
            let b = args
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidArgs("missing 'b'".into()))?;
            Ok(serde_json::json!({ "result": a + b }))
        }
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.list().is_empty());
    }

    #[test]
    fn default_registry_is_empty() {
        let registry = ToolRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn register_and_get() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let tool = registry.get("echo").unwrap();
        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echo back the input text");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn list_returns_sorted_names() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(AddTool));
        registry.register(Arc::new(FailTool));

        let names = registry.list();
        assert_eq!(names, vec!["add", "echo", "fail"]);
    }

    #[test]
    fn len_reflects_registered_count() {
        let mut registry = ToolRegistry::new();
        assert_eq!(registry.len(), 0);

        registry.register(Arc::new(EchoTool));
        assert_eq!(registry.len(), 1);

        registry.register(Arc::new(AddTool));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn register_replaces_existing() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(EchoTool)); // Same name, should replace.
        assert_eq!(registry.len(), 1);
    }

    #[tokio::test]
    async fn execute_echo_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let result = registry
            .execute("echo", serde_json::json!({ "text": "hello" }))
            .await
            .unwrap();

        assert_eq!(result["output"], "hello");
    }

    #[tokio::test]
    async fn execute_add_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(AddTool));

        let result = registry
            .execute("add", serde_json::json!({ "a": 3, "b": 4 }))
            .await
            .unwrap();

        assert_eq!(result["result"], 7.0);
    }

    #[tokio::test]
    async fn execute_not_found() {
        let registry = ToolRegistry::new();
        let result = registry
            .execute("missing", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(name) => assert_eq!(name, "missing"),
            other => panic!("expected NotFound, got: {other}"),
        }
    }

    #[tokio::test]
    async fn execute_tool_that_fails() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(FailTool));

        let result = registry.execute("fail", serde_json::json!({})).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert_eq!(msg, "intentional failure");
            }
            other => panic!("expected ExecutionFailed, got: {other}"),
        }
    }

    #[tokio::test]
    async fn execute_with_invalid_args() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let result = registry
            .execute("echo", serde_json::json!({})) // missing "text"
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidArgs(msg) => {
                assert!(msg.contains("text"));
            }
            other => panic!("expected InvalidArgs, got: {other}"),
        }
    }

    #[test]
    fn schemas_openai_format() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));
        registry.register(Arc::new(AddTool));

        let schemas = registry.schemas();
        assert_eq!(schemas.len(), 2);

        // Schemas should be sorted by name: "add" before "echo".
        let first = &schemas[0];
        assert_eq!(first["type"], "function");
        assert_eq!(first["function"]["name"], "add");
        assert_eq!(first["function"]["description"], "Add two numbers");
        assert!(first["function"]["parameters"]["properties"].get("a").is_some());

        let second = &schemas[1];
        assert_eq!(second["type"], "function");
        assert_eq!(second["function"]["name"], "echo");
        assert_eq!(
            second["function"]["description"],
            "Echo back the input text"
        );
        assert!(second["function"]["parameters"]["properties"]
            .get("text")
            .is_some());
    }

    #[test]
    fn schemas_empty_registry() {
        let registry = ToolRegistry::new();
        assert!(registry.schemas().is_empty());
    }

    #[test]
    fn tool_error_display() {
        let err = ToolError::NotFound("web_search".into());
        assert_eq!(err.to_string(), "tool not found: web_search");

        let err = ToolError::InvalidArgs("missing required field".into());
        assert_eq!(err.to_string(), "invalid arguments: missing required field");

        let err = ToolError::ExecutionFailed("command failed".into());
        assert_eq!(err.to_string(), "execution failed: command failed");

        let err = ToolError::PermissionDenied("no exec access".into());
        assert_eq!(err.to_string(), "permission denied: no exec access");

        let err = ToolError::FileNotFound("/tmp/missing.txt".into());
        assert_eq!(err.to_string(), "not found: /tmp/missing.txt");

        let err = ToolError::InvalidPath("../../../etc/passwd".into());
        assert_eq!(err.to_string(), "invalid path: ../../../etc/passwd");

        let err = ToolError::Timeout(30);
        assert_eq!(err.to_string(), "timeout after 30s");
    }

    #[test]
    fn tool_trait_is_object_safe() {
        // Verify Tool can be used as a trait object.
        fn accepts_tool(_t: &dyn Tool) {}
        let tool = EchoTool;
        accepts_tool(&tool);
    }

    #[test]
    fn tool_parameters_returns_valid_json_schema() {
        let tool = EchoTool;
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"].is_object());
        assert!(params["required"].is_array());
    }
}
