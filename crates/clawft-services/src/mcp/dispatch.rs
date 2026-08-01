//! Transport-agnostic MCP JSON-RPC message dispatcher.
//!
//! [`McpDispatcher`] owns the tool provider and middleware pipeline and
//! turns one incoming JSON-RPC message into (at most) one response
//! value. Transports own the wire: [`super::server::McpServerShell`]
//! drives it over newline-delimited streams (stdio), and the gateway's
//! `/mcp` endpoint drives it over Streamable HTTP for remote MCP
//! clients such as the xAI Grok voice agent.

use serde_json::Value;

use super::ToolDefinition;
use super::composite::CompositeToolProvider;
use super::middleware::{Middleware, ToolCallRequest};
use super::provider::CallToolResult;

// ── Constants ───────────────────────────────────────────────────────────

/// Re-use the canonical protocol version from the MCP module.
pub(crate) const PROTOCOL_VERSION: &str = super::MCP_PROTOCOL_VERSION;
pub(crate) const SERVER_NAME: &str = "clawft";
pub(crate) const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC error codes.
pub(crate) const METHOD_NOT_FOUND: i32 = -32601;
pub(crate) const NOT_INITIALIZED: i32 = -32002;
pub(crate) const INVALID_REQUEST: i32 = -32600;

// ── McpDispatcher ───────────────────────────────────────────────────────

/// Dispatches MCP JSON-RPC messages against a composite tool provider
/// with a middleware pipeline.
///
/// Handles the `initialize` handshake, `tools/list`, `tools/call`, and
/// `notifications/initialized` methods. Unknown methods receive a
/// `-32601 Method not found` error.
pub struct McpDispatcher {
    provider: CompositeToolProvider,
    middlewares: Vec<Box<dyn Middleware>>,
}

impl McpDispatcher {
    /// Create a dispatcher wrapping the given composite provider.
    pub fn new(provider: CompositeToolProvider) -> Self {
        Self {
            provider,
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the processing pipeline.
    pub fn add_middleware(&mut self, middleware: Box<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    /// Handle one parsed JSON-RPC message.
    ///
    /// `initialized` carries the handshake state across calls: stream
    /// transports thread a per-connection flag through, while stateless
    /// transports (HTTP, where each request may arrive on a fresh
    /// connection) pass a flag pre-set to `true` so `tools/*` calls are
    /// not rejected between requests.
    ///
    /// Returns `Some(response)` for requests and protocol errors,
    /// `None` for notifications (which never receive a response).
    pub async fn handle_message(&self, msg: &Value, initialized: &mut bool) -> Option<Value> {
        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();
        let params = msg
            .get("params")
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()));

        // Notifications have no id -- never send a response.
        let is_notification = id.is_none();

        match method {
            "initialize" => {
                *initialized = true;
                let result = serde_json::json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": { "listChanged": true }
                    },
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": SERVER_VERSION
                    }
                });
                id.map(|id| make_success_response(id, result))
            }

            "notifications/initialized" => {
                // Notification acknowledgement -- no response.
                None
            }

            _ if !*initialized => {
                if is_notification {
                    None
                } else {
                    Some(make_error_response(
                        id.unwrap_or(Value::Null),
                        NOT_INITIALIZED,
                        "Server not initialized",
                    ))
                }
            }

            "tools/list" => {
                let mut tools = self.provider.list_tools_all();

                // Apply middleware filter_tools in order.
                for mw in &self.middlewares {
                    tools = mw.filter_tools(tools).await;
                }

                let tools_json = serialize_tools(&tools);
                let result = serde_json::json!({ "tools": tools_json });
                id.map(|id| make_success_response(id, result))
            }

            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));

                let mut request = ToolCallRequest {
                    name: name.clone(),
                    args,
                };

                // Apply middleware before_call hooks.
                let mut mw_error = None;
                for mw in &self.middlewares {
                    match mw.before_call(request).await {
                        Ok(r) => request = r,
                        Err(e) => {
                            mw_error = Some(e);
                            // Reconstruct a minimal request for the error path.
                            request = ToolCallRequest {
                                name,
                                args: Value::Object(Default::default()),
                            };
                            break;
                        }
                    }
                }

                let call_result = if let Some(err) = mw_error {
                    Err(err)
                } else {
                    self.provider
                        .call_tool(&request.name, request.args.clone())
                        .await
                };

                let result_value = match call_result {
                    Ok(mut result) => {
                        // Apply middleware after_call hooks.
                        for mw in &self.middlewares {
                            match mw.after_call(&request, result).await {
                                Ok(r) => result = r,
                                Err(e) => {
                                    result = CallToolResult::error(e.to_string());
                                    break;
                                }
                            }
                        }
                        serde_json::to_value(&result).unwrap_or(Value::Null)
                    }
                    Err(e) => {
                        let err_result = CallToolResult::error(e.to_string());
                        serde_json::to_value(&err_result).unwrap_or(Value::Null)
                    }
                };

                id.map(|id| make_success_response(id, result_value))
            }

            _ => {
                // Unknown method.
                if is_notification {
                    None
                } else {
                    Some(make_error_response(
                        id.unwrap_or(Value::Null),
                        METHOD_NOT_FOUND,
                        &format!("Method not found: {method}"),
                    ))
                }
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

pub(crate) fn make_success_response(id: Value, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub(crate) fn make_error_response(id: Value, code: i32, message: &str) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn serialize_tools(tools: &[ToolDefinition]) -> Value {
    serde_json::to_value(tools).unwrap_or_else(|_| Value::Array(vec![]))
}
