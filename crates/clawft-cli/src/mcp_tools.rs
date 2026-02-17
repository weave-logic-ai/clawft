//! MCP tool wrapper for bridging MCP server tools into the tool registry.
//!
//! Wraps MCP server tool definitions as implementations of the
//! [`Tool`](clawft_core::tools::registry::Tool) trait, allowing MCP
//! tools to be invoked by the agent loop just like built-in tools.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;

use clawft_core::tools::registry::{Tool, ToolError};
use clawft_services::mcp::{McpClient, ToolDefinition};
use clawft_services::mcp::transport::{HttpTransport, StdioTransport};
use clawft_types::config::MCPServerConfig;

/// Wraps an MCP tool definition for use in the `ToolRegistry`.
///
/// Each wrapper holds a reference to the shared [`McpClient`] for its
/// server and delegates execution to [`McpClient::call_tool`].
/// The tool name is prefixed with `{server_name}__` to avoid collisions
/// when multiple MCP servers expose tools with the same base name.
pub struct McpToolWrapper {
    /// Namespaced tool name: `"{server}__{tool}"`.
    full_name: String,
    /// The tool definition from the MCP server.
    tool_def: ToolDefinition,
    /// Shared client for this MCP server.
    client: Arc<McpClient>,
}

impl McpToolWrapper {
    /// Create a new wrapper.
    ///
    /// The tool will be registered as `"{server_name}__{tool_def.name}"`.
    pub fn new(server_name: &str, tool_def: ToolDefinition, client: Arc<McpClient>) -> Self {
        let full_name = format!("{}__{}", server_name, tool_def.name);
        Self {
            full_name,
            tool_def,
            client,
        }
    }
}

#[async_trait]
impl Tool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.tool_def.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.tool_def.input_schema.clone()
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        self.client
            .call_tool(&self.tool_def.name, args)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}

/// Create an MCP client from server configuration.
///
/// Chooses the transport based on config fields:
/// - If `command` is non-empty, spawns a child process via [`StdioTransport`].
/// - If `url` is non-empty (and command is empty), uses [`HttpTransport`].
/// - If both are empty, returns `None` with a warning log.
pub async fn create_mcp_client(
    server_name: &str,
    config: &MCPServerConfig,
) -> Option<McpClient> {
    if !config.command.is_empty() {
        match StdioTransport::new(&config.command, &config.args, &config.env).await {
            Ok(transport) => Some(McpClient::new(Box::new(transport))),
            Err(e) => {
                warn!(
                    server = %server_name,
                    error = %e,
                    "failed to spawn MCP stdio transport"
                );
                None
            }
        }
    } else if !config.url.is_empty() {
        let transport = HttpTransport::new(config.url.clone());
        Some(McpClient::new(Box::new(transport)))
    } else {
        warn!(server = %server_name, "MCP server has no command or URL, skipping");
        None
    }
}

/// Discover and register MCP tools from all configured servers.
///
/// For each MCP server in the config, creates a client, lists available
/// tools, and wraps each as an [`McpToolWrapper`] registered in the
/// tool registry.
pub async fn register_mcp_tools(
    config: &clawft_types::config::Config,
    registry: &mut clawft_core::tools::registry::ToolRegistry,
) {
    for (server_name, server_config) in &config.tools.mcp_servers {
        match create_mcp_client(server_name, server_config).await {
            Some(client) => {
                let client = Arc::new(client);
                match client.list_tools().await {
                    Ok(tools) => {
                        let count = tools.len();
                        for tool_def in tools {
                            let wrapper = McpToolWrapper::new(
                                server_name,
                                tool_def,
                                client.clone(),
                            );
                            registry.register(Arc::new(wrapper));
                        }
                        tracing::info!(
                            server = %server_name,
                            tools = count,
                            "registered MCP tools"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            server = %server_name,
                            error = %e,
                            "failed to list MCP tools, skipping"
                        );
                    }
                }
            }
            None => {
                // Warning already logged by create_mcp_client.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use clawft_services::mcp::transport::McpTransport;
    use clawft_services::mcp::types::{JsonRpcRequest, JsonRpcResponse};

    /// A minimal mock transport for testing within this crate.
    ///
    /// The `MockTransport` in `clawft-services` is `#[cfg(test)]`-gated
    /// and therefore unavailable outside that crate, so we provide our own.
    struct TestTransport {
        responses: tokio::sync::Mutex<Vec<JsonRpcResponse>>,
    }

    impl TestTransport {
        fn new(responses: Vec<JsonRpcResponse>) -> Self {
            Self {
                responses: tokio::sync::Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl McpTransport for TestTransport {
        async fn send_request(
            &self,
            _request: JsonRpcRequest,
        ) -> clawft_services::error::Result<JsonRpcResponse> {
            let mut responses = self.responses.lock().await;
            if responses.is_empty() {
                Err(clawft_services::error::ServiceError::McpTransport(
                    "no more mock responses".into(),
                ))
            } else {
                Ok(responses.remove(0))
            }
        }
    }

    fn make_tool_def() -> ToolDefinition {
        ToolDefinition {
            name: "echo".into(),
            description: "Echo input".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                }
            }),
        }
    }

    fn make_client(responses: Vec<JsonRpcResponse>) -> Arc<McpClient> {
        let transport = TestTransport::new(responses);
        Arc::new(McpClient::new(Box::new(transport)))
    }

    // ── McpToolWrapper unit tests ───────────────────────────────────────

    #[test]
    fn wrapper_name_is_namespaced() {
        let client = make_client(vec![]);
        let wrapper = McpToolWrapper::new("myserver", make_tool_def(), client);

        assert_eq!(wrapper.name(), "myserver__echo");
    }

    #[test]
    fn wrapper_description_delegates() {
        let client = make_client(vec![]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);

        assert_eq!(wrapper.description(), "Echo input");
    }

    #[test]
    fn wrapper_parameters_returns_schema() {
        let client = make_client(vec![]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);

        let params = wrapper.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["text"].is_object());
    }

    #[tokio::test]
    async fn wrapper_execute_delegates_to_client() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({"output": "hello"})),
            error: None,
        };
        let client = make_client(vec![response]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);

        let result = wrapper
            .execute(serde_json::json!({"text": "hello"}))
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["output"], "hello");
    }

    #[tokio::test]
    async fn wrapper_execute_maps_transport_error() {
        // Client with no responses will produce a transport error.
        let client = make_client(vec![]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);

        let result = wrapper
            .execute(serde_json::json!({"text": "hello"}))
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert!(msg.contains("no more mock responses"));
            }
            other => panic!("expected ExecutionFailed, got: {other}"),
        }
    }

    #[tokio::test]
    async fn wrapper_execute_maps_protocol_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: None,
            error: Some(clawft_services::mcp::types::JsonRpcError {
                code: -32601,
                message: "method not found".into(),
                data: None,
            }),
        };
        let client = make_client(vec![response]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);

        let result = wrapper.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert!(msg.contains("method not found"));
            }
            other => panic!("expected ExecutionFailed, got: {other}"),
        }
    }

    #[test]
    fn wrapper_is_object_safe() {
        // Verify McpToolWrapper can be used as a `dyn Tool` trait object.
        fn accepts_tool(_t: &dyn Tool) {}
        let client = make_client(vec![]);
        let wrapper = McpToolWrapper::new("srv", make_tool_def(), client);
        accepts_tool(&wrapper);
    }

    // ── create_mcp_client tests ─────────────────────────────────────────

    #[tokio::test]
    async fn create_client_with_url() {
        let config = MCPServerConfig {
            command: String::new(),
            args: vec![],
            env: HashMap::new(),
            url: "http://localhost:8080".into(),
        };
        let client = create_mcp_client("test", &config).await;
        assert!(client.is_some());
    }

    #[tokio::test]
    async fn create_client_empty_returns_none() {
        let config = MCPServerConfig::default();
        let client = create_mcp_client("test", &config).await;
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn create_client_bad_command_returns_none() {
        // A command that does not exist should fail gracefully.
        let config = MCPServerConfig {
            command: "__nonexistent_binary_clawft_test__".into(),
            args: vec![],
            env: HashMap::new(),
            url: String::new(),
        };
        let client = create_mcp_client("test", &config).await;
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn create_client_prefers_command_over_url() {
        // When both command and url are set, command takes priority.
        // We use a bad command to verify it tried the command path
        // (returns None from spawn failure) rather than the URL path
        // (which would succeed).
        let config = MCPServerConfig {
            command: "__nonexistent_binary_clawft_test__".into(),
            args: vec![],
            env: HashMap::new(),
            url: "http://localhost:9999".into(),
        };
        let client = create_mcp_client("test", &config).await;
        // If it had used the URL path, this would be Some.
        assert!(client.is_none());
    }
}
