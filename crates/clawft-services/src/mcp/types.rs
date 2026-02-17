//! JSON-RPC 2.0 types for MCP communication.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version, always `"2.0"`.
    pub jsonrpc: String,
    /// Request identifier.
    pub id: u64,
    /// Method name.
    pub method: String,
    /// Method parameters.
    #[serde(default = "default_params")]
    pub params: serde_json::Value,
}

fn default_params() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC 2.0 request.
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version, always `"2.0"`.
    pub jsonrpc: String,
    /// Request identifier this response corresponds to.
    pub id: u64,
    /// Successful result (mutually exclusive with `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error result (mutually exclusive with `result`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Error message.
    pub message: String,
    /// Optional structured error data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serialization() {
        let req = JsonRpcRequest::new(1, "tools/list", serde_json::json!({}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn request_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":42,"method":"test","params":{"key":"value"}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 42);
        assert_eq!(req.method, "test");
        assert_eq!(req.params["key"], "value");
    }

    #[test]
    fn request_default_params() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert!(req.params.is_object());
    }

    #[test]
    fn response_with_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn response_with_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "method not found");
    }

    #[test]
    fn response_serialization_skips_none() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!(42)),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("error"));
    }

    #[test]
    fn error_with_data() {
        let json = r#"{"code":-32600,"message":"invalid","data":{"detail":"bad format"}}"#;
        let err: JsonRpcError = serde_json::from_str(json).unwrap();
        assert_eq!(err.code, -32600);
        assert!(err.data.is_some());
    }

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest::new(99, "tools/call", serde_json::json!({"name": "echo", "arguments": {}}));
        let json = serde_json::to_string(&req).unwrap();
        let restored: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, 99);
        assert_eq!(restored.method, "tools/call");
    }
}
