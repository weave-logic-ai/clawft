//! MCP server shell that handles JSON-RPC protocol over newline-delimited
//! JSON streams.
//!
//! [`McpServerShell`] is generic over `AsyncBufRead + AsyncWrite` so it
//! can be driven by stdio, TCP, or in-memory buffers for testing. The
//! protocol logic itself lives in [`super::dispatch::McpDispatcher`],
//! shared with the gateway's HTTP `/mcp` endpoint.

use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use super::composite::CompositeToolProvider;
use super::dispatch::{INVALID_REQUEST, McpDispatcher, make_error_response};
use super::middleware::Middleware;

// ── McpServerShell ─────────────────────────────────────────────────────

/// An MCP server that reads newline-delimited JSON-RPC from a reader and
/// writes responses to a writer.
///
/// Handles the `initialize` handshake, `tools/list`, `tools/call`, and
/// `notifications/initialized` methods. Unknown methods receive a
/// `-32601 Method not found` error. Requests sent before `initialize`
/// receive a `-32002 Server not initialized` error.
pub struct McpServerShell {
    dispatcher: McpDispatcher,
    initialized: bool,
}

impl McpServerShell {
    /// Create a new server shell wrapping the given composite provider.
    pub fn new(provider: CompositeToolProvider) -> Self {
        Self {
            dispatcher: McpDispatcher::new(provider),
            initialized: false,
        }
    }

    /// Add a middleware to the processing pipeline.
    pub fn add_middleware(&mut self, middleware: Box<dyn Middleware>) {
        self.dispatcher.add_middleware(middleware);
    }

    /// Run the server loop, reading lines from `reader` and writing
    /// responses to `writer` until EOF.
    pub async fn run<R, W>(&mut self, reader: R, mut writer: W) -> std::io::Result<()>
    where
        R: AsyncBufRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            // Parse the incoming JSON.
            let msg: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => {
                    let resp = make_error_response(Value::Null, INVALID_REQUEST, "Parse error");
                    write_response(&mut writer, &resp).await?;
                    continue;
                }
            };

            if let Some(resp) = self
                .dispatcher
                .handle_message(&msg, &mut self.initialized)
                .await
            {
                write_response(&mut writer, &resp).await?;
            }
        }

        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

async fn write_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &Value,
) -> std::io::Result<()> {
    let mut line = serde_json::to_string(response).map_err(std::io::Error::other)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::composite::CompositeToolProvider;
    use super::super::middleware::{Middleware, ToolCallRequest};
    use super::super::dispatch::{
        INVALID_REQUEST, METHOD_NOT_FOUND, NOT_INITIALIZED, PROTOCOL_VERSION, SERVER_NAME,
    };
    use super::super::provider::{CallToolResult, ContentBlock, ToolError, ToolProvider};
    use super::McpServerShell;
    use super::super::ToolDefinition;
    use serde_json::Value;
    use async_trait::async_trait;
    use serde_json::json;
    use std::io::Cursor;

    // ── Test helpers ────────────────────────────────────────────────────

    /// Build a JSON-RPC request line (with id).
    fn request_line(id: u64, method: &str, params: Value) -> String {
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        format!("{}\n", serde_json::to_string(&req).unwrap())
    }

    /// Build a notification line (no id).
    fn notification_line(method: &str, params: Value) -> String {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        format!("{}\n", serde_json::to_string(&req).unwrap())
    }

    /// Parse response lines from the output buffer.
    fn parse_responses(output: &[u8]) -> Vec<Value> {
        let text = String::from_utf8_lossy(output);
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("invalid JSON response"))
            .collect()
    }

    /// Standard initialize request line.
    fn init_line(id: u64) -> String {
        request_line(
            id,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0.1" }
            }),
        )
    }

    // ── Mock provider ───────────────────────────────────────────────────

    struct EchoProvider;

    #[async_trait]
    impl ToolProvider for EchoProvider {
        fn namespace(&self) -> &str {
            "echo"
        }

        fn list_tools(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition {
                name: "say".to_string(),
                description: "Echoes text".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } }
                }),
            }]
        }

        async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
            if name == "say" {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(empty)");
                Ok(CallToolResult::text(text))
            } else {
                Err(ToolError::NotFound(name.to_string()))
            }
        }
    }

    fn make_server() -> McpServerShell {
        let mut provider = CompositeToolProvider::new();
        provider.register(Box::new(EchoProvider));
        McpServerShell::new(provider)
    }

    // ── Protocol tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn initialize_handshake() {
        let mut server = make_server();
        let input = init_line(1);

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 1);
        let resp = &responses[0];
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert_eq!(resp["result"]["serverInfo"]["name"], SERVER_NAME);
    }

    #[tokio::test]
    async fn not_initialized_rejection() {
        let mut server = make_server();
        let input = request_line(1, "tools/list", json!({}));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["error"]["code"], NOT_INITIALIZED);
    }

    #[tokio::test]
    async fn tools_list_returns_tools() {
        let mut server = make_server();
        let mut input = init_line(1);
        input.push_str(&notification_line("notifications/initialized", json!({})));
        input.push_str(&request_line(2, "tools/list", json!({})));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        // init response + tools/list response (notification produces none).
        assert_eq!(responses.len(), 2);

        let list_resp = &responses[1];
        assert_eq!(list_resp["id"], 2);
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo__say");
    }

    #[tokio::test]
    async fn tools_call_routes_correctly() {
        let mut server = make_server();
        let mut input = init_line(1);
        input.push_str(&notification_line("notifications/initialized", json!({})));
        input.push_str(&request_line(
            2,
            "tools/call",
            json!({
                "name": "echo__say",
                "arguments": { "text": "hello world" }
            }),
        ));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 2);

        let call_resp = &responses[1];
        assert_eq!(call_resp["id"], 2);
        let content = call_resp["result"]["content"].as_array().unwrap();
        assert_eq!(content[0]["text"], "hello world");
    }

    #[tokio::test]
    async fn tools_call_not_found() {
        let mut server = make_server();
        let mut input = init_line(1);
        input.push_str(&request_line(
            2,
            "tools/call",
            json!({
                "name": "nonexistent__tool",
                "arguments": {}
            }),
        ));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 2);

        let call_resp = &responses[1];
        // Tool-not-found is returned as a CallToolResult with is_error=true.
        assert!(call_resp["result"]["isError"].as_bool().unwrap_or(false));
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let mut server = make_server();
        let mut input = init_line(1);
        input.push_str(&request_line(2, "completions/complete", json!({})));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 2);

        let resp = &responses[1];
        assert_eq!(resp["error"]["code"], METHOD_NOT_FOUND);
        assert!(
            resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("completions/complete")
        );
    }

    #[tokio::test]
    async fn notification_produces_no_response() {
        let mut server = make_server();
        let mut input = init_line(1);
        input.push_str(&notification_line("notifications/initialized", json!({})));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        // Only the initialize response.
        assert_eq!(responses.len(), 1);
    }

    #[tokio::test]
    async fn malformed_json_returns_parse_error() {
        let mut server = make_server();
        let input = "not valid json\n".to_string();

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["error"]["code"], INVALID_REQUEST);
    }

    #[tokio::test]
    async fn empty_lines_are_skipped() {
        let mut server = make_server();
        let mut input = String::new();
        input.push('\n');
        input.push_str("   \n");
        input.push_str(&init_line(1));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["id"], 1);
    }

    // ── Middleware integration tests ────────────────────────────────────

    /// Middleware that uppercases all tool descriptions.
    struct UppercaseMiddleware;

    #[async_trait]
    impl Middleware for UppercaseMiddleware {
        async fn filter_tools(&self, tools: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
            tools
                .into_iter()
                .map(|mut t| {
                    t.description = t.description.to_uppercase();
                    t
                })
                .collect()
        }
    }

    #[tokio::test]
    async fn middleware_filter_tools_applied() {
        let mut provider = CompositeToolProvider::new();
        provider.register(Box::new(EchoProvider));
        let mut server = McpServerShell::new(provider);
        server.add_middleware(Box::new(UppercaseMiddleware));

        let mut input = init_line(1);
        input.push_str(&request_line(2, "tools/list", json!({})));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        let tools = responses[1]["result"]["tools"].as_array().unwrap();
        assert_eq!(tools[0]["description"], "ECHOES TEXT");
    }

    /// Middleware that rejects calls to a specific tool.
    struct BlockingMiddleware {
        blocked_tool: String,
    }

    #[async_trait]
    impl Middleware for BlockingMiddleware {
        async fn before_call(
            &self,
            request: ToolCallRequest,
        ) -> Result<ToolCallRequest, ToolError> {
            if request.name == self.blocked_tool {
                Err(ToolError::ExecutionFailed("tool is blocked".into()))
            } else {
                Ok(request)
            }
        }
    }

    #[tokio::test]
    async fn middleware_before_call_can_reject() {
        let mut provider = CompositeToolProvider::new();
        provider.register(Box::new(EchoProvider));
        let mut server = McpServerShell::new(provider);
        server.add_middleware(Box::new(BlockingMiddleware {
            blocked_tool: "echo__say".to_string(),
        }));

        let mut input = init_line(1);
        input.push_str(&request_line(
            2,
            "tools/call",
            json!({
                "name": "echo__say",
                "arguments": { "text": "blocked" }
            }),
        ));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        let call_resp = &responses[1];
        assert!(call_resp["result"]["isError"].as_bool().unwrap_or(false));
        let content = call_resp["result"]["content"].as_array().unwrap();
        assert!(content[0]["text"].as_str().unwrap().contains("blocked"));
    }

    /// Middleware that appends a suffix content block.
    struct SuffixMiddleware;

    #[async_trait]
    impl Middleware for SuffixMiddleware {
        async fn after_call(
            &self,
            _request: &ToolCallRequest,
            mut result: CallToolResult,
        ) -> Result<CallToolResult, ToolError> {
            result.content.push(ContentBlock::Text {
                text: "[suffix]".into(),
            });
            Ok(result)
        }
    }

    #[tokio::test]
    async fn middleware_after_call_applied() {
        let mut provider = CompositeToolProvider::new();
        provider.register(Box::new(EchoProvider));
        let mut server = McpServerShell::new(provider);
        server.add_middleware(Box::new(SuffixMiddleware));

        let mut input = init_line(1);
        input.push_str(&request_line(
            2,
            "tools/call",
            json!({
                "name": "echo__say",
                "arguments": { "text": "hello" }
            }),
        ));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        let content = responses[1]["result"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["text"], "hello");
        assert_eq!(content[1]["text"], "[suffix]");
    }

    // ── Full session integration test ───────────────────────────────────

    #[tokio::test]
    async fn full_session_flow() {
        let mut server = make_server();

        let mut input = String::new();
        // 1. Initialize.
        input.push_str(&init_line(1));
        // 2. Initialized notification.
        input.push_str(&notification_line("notifications/initialized", json!({})));
        // 3. List tools.
        input.push_str(&request_line(2, "tools/list", json!({})));
        // 4. Call a tool.
        input.push_str(&request_line(
            3,
            "tools/call",
            json!({
                "name": "echo__say",
                "arguments": { "text": "integration test" }
            }),
        ));
        // 5. Unknown method.
        input.push_str(&request_line(4, "unknown/method", json!({})));

        let reader = Cursor::new(input.into_bytes());
        let mut output = Vec::new();

        server.run(reader, &mut output).await.unwrap();

        let responses = parse_responses(&output);
        // 4 responses (init, list, call, unknown). Notification produces none.
        assert_eq!(responses.len(), 4);

        // 1: Initialize OK.
        assert!(responses[0]["result"]["protocolVersion"].is_string());

        // 2: tools/list.
        let tools = responses[1]["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);

        // 3: tools/call.
        let content = responses[2]["result"]["content"].as_array().unwrap();
        assert_eq!(content[0]["text"], "integration test");

        // 4: Unknown method error.
        assert_eq!(responses[3]["error"]["code"], METHOD_NOT_FOUND);
    }
}
