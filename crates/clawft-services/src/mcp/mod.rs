//! MCP (Model Context Protocol) client.
//!
//! Provides a client for communicating with MCP servers using
//! JSON-RPC 2.0 over pluggable transports (stdio or HTTP).

pub mod transport;
pub mod types;

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::{Result, ServiceError};
use transport::McpTransport;
use types::JsonRpcRequest;

/// Definition of an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    #[serde(rename = "inputSchema", alias = "input_schema")]
    pub input_schema: serde_json::Value,
}

/// Client for communicating with an MCP server.
pub struct McpClient {
    transport: Box<dyn McpTransport>,
    request_id: AtomicU64,
}

impl McpClient {
    /// Create a new MCP client with the given transport.
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self {
            transport,
            request_id: AtomicU64::new(1),
        }
    }

    /// List all tools available on the MCP server.
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, "tools/list", serde_json::json!({}));

        let response = self.transport.send_request(request).await?;

        if let Some(err) = response.error {
            return Err(ServiceError::McpProtocol(format!(
                "code={}, message={}",
                err.code, err.message
            )));
        }

        let result = response
            .result
            .ok_or_else(|| ServiceError::McpProtocol("empty result".into()))?;

        // MCP returns tools in a `tools` array.
        let tools_value = result
            .get("tools")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(vec![]));

        let tools: Vec<ToolDefinition> = serde_json::from_value(tools_value)?;
        Ok(tools)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(
        &self,
        name: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(
            id,
            "tools/call",
            serde_json::json!({
                "name": name,
                "arguments": params,
            }),
        );

        let response = self.transport.send_request(request).await?;

        if let Some(err) = response.error {
            return Err(ServiceError::McpProtocol(format!(
                "code={}, message={}",
                err.code, err.message
            )));
        }

        response
            .result
            .ok_or_else(|| ServiceError::McpProtocol("empty result".into()))
    }

    /// Generate the next request ID.
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use transport::MockTransport;
    use types::JsonRpcResponse;

    fn make_success_response(id: u64, result: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn make_error_response(id: u64, code: i32, message: &str) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(types::JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    #[tokio::test]
    async fn list_tools_parses_response() {
        let tools_response = make_success_response(
            1,
            serde_json::json!({
                "tools": [
                    {
                        "name": "echo",
                        "description": "Echoes input",
                        "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}}
                    },
                    {
                        "name": "calc",
                        "description": "Calculator",
                        "inputSchema": {"type": "object"}
                    }
                ]
            }),
        );

        let transport = MockTransport::new(vec![tools_response]);
        let client = McpClient::new(Box::new(transport));

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(tools[0].description, "Echoes input");
        assert_eq!(tools[1].name, "calc");
    }

    #[tokio::test]
    async fn list_tools_empty() {
        let response = make_success_response(1, serde_json::json!({"tools": []}));
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        let tools = client.list_tools().await.unwrap();
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn call_tool_sends_correct_request() {
        let response = make_success_response(1, serde_json::json!({"output": "hello"}));
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        let result = client
            .call_tool("echo", serde_json::json!({"text": "hello"}))
            .await
            .unwrap();

        assert_eq!(result["output"], "hello");
    }

    #[tokio::test]
    async fn call_tool_request_format() {
        let response = make_success_response(1, serde_json::json!({}));
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        client
            .call_tool("my_tool", serde_json::json!({"arg": 42}))
            .await
            .unwrap();

        // Request was sent successfully if we got here without error.
        // The mock transport verified we sent a valid JSON-RPC request.
    }

    #[tokio::test]
    async fn handle_jsonrpc_error_on_list_tools() {
        let response = make_error_response(1, -32601, "method not found");
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        let result = client.list_tools().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ServiceError::McpProtocol(_)));
        assert!(err.to_string().contains("method not found"));
    }

    #[tokio::test]
    async fn handle_jsonrpc_error_on_call_tool() {
        let response = make_error_response(1, -32602, "invalid params");
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        let result = client.call_tool("bad", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid params"));
    }

    #[tokio::test]
    async fn request_ids_increment() {
        let responses = vec![
            make_success_response(1, serde_json::json!({"tools": []})),
            make_success_response(2, serde_json::json!({"tools": []})),
        ];
        let transport = MockTransport::new(responses);
        let client = McpClient::new(Box::new(transport));

        client.list_tools().await.unwrap();
        client.list_tools().await.unwrap();
        // If we get here without error, IDs were generated correctly.
    }

    #[tokio::test]
    async fn empty_result_is_error() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: None,
            error: None,
        };
        let transport = MockTransport::new(vec![response]);
        let client = McpClient::new(Box::new(transport));

        let result = client.call_tool("test", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::McpProtocol(_)
        ));
    }

    #[tokio::test]
    async fn tool_definition_serde() {
        let td = ToolDefinition {
            name: "test".into(),
            description: "A test tool".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&td).unwrap();
        let restored: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.description, "A test tool");
    }

    #[tokio::test]
    async fn tool_definition_input_schema_alias() {
        // MCP uses camelCase, but we should also accept snake_case.
        let json = r#"{"name":"t","description":"d","input_schema":{"type":"object"}}"#;
        let td: ToolDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(td.name, "t");
    }
}
