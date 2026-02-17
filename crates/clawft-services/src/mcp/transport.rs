//! MCP transport implementations.
//!
//! Provides [`McpTransport`] trait and two implementations:
//! - [`StdioTransport`]: communicates with a child process over stdin/stdout
//! - [`HttpTransport`]: communicates over HTTP POST

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::debug;

use super::types::{JsonRpcRequest, JsonRpcResponse};
use crate::error::{Result, ServiceError};

/// Transport layer for MCP JSON-RPC communication.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a JSON-RPC request and return the response.
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
}

/// Transport that communicates with a child process via stdin/stdout.
///
/// Each request is written as a single JSON line to stdin, and the
/// corresponding response is read as a single JSON line from stdout.
pub struct StdioTransport {
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    stdout: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
}

impl StdioTransport {
    /// Spawn a child process and set up JSON-RPC communication.
    pub async fn new(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        let mut child = cmd.spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ServiceError::McpTransport("failed to capture stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ServiceError::McpTransport("failed to capture stdout".into()))?;

        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');

        debug!(method = %request.method, id = request.id, "sending stdio request");

        // Write to stdin.
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await.map_err(|e| {
                ServiceError::McpTransport(format!("failed to write to stdin: {e}"))
            })?;
            stdin.flush().await.map_err(|e| {
                ServiceError::McpTransport(format!("failed to flush stdin: {e}"))
            })?;
        }

        // Read from stdout.
        let mut response_line = String::new();
        {
            let mut stdout = self.stdout.lock().await;
            stdout
                .read_line(&mut response_line)
                .await
                .map_err(|e| ServiceError::McpTransport(format!("failed to read from stdout: {e}")))?;
        }

        if response_line.is_empty() {
            return Err(ServiceError::McpTransport(
                "child process closed stdout".into(),
            ));
        }

        let response: JsonRpcResponse = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }
}

/// Transport that communicates via HTTP POST.
///
/// Sends JSON-RPC requests as the body of POST requests to the
/// configured endpoint URL.
pub struct HttpTransport {
    client: reqwest::Client,
    endpoint: String,
}

impl HttpTransport {
    /// Create a new HTTP transport targeting the given endpoint URL.
    pub fn new(endpoint: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint,
        }
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        debug!(
            method = %request.method,
            id = request.id,
            endpoint = %self.endpoint,
            "sending HTTP request"
        );

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| ServiceError::McpTransport(format!("HTTP request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ServiceError::McpTransport(format!(
                "HTTP {status}: {body}"
            )));
        }

        let response: JsonRpcResponse = resp
            .json()
            .await
            .map_err(|e| ServiceError::McpTransport(format!("failed to parse response: {e}")))?;

        Ok(response)
    }
}

/// A mock transport for testing.
///
/// Allows pre-programming responses that will be returned in order.
#[cfg(test)]
pub struct MockTransport {
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
    requests: Arc<Mutex<Vec<JsonRpcRequest>>>,
}

#[cfg(test)]
impl MockTransport {
    /// Create a mock transport with pre-programmed responses.
    pub fn new(responses: Vec<JsonRpcResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all requests that were sent through this transport.
    pub async fn requests(&self) -> Vec<JsonRpcRequest> {
        self.requests.lock().await.clone()
    }
}

#[cfg(test)]
#[async_trait]
impl McpTransport for MockTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        self.requests.lock().await.push(request);
        let mut responses = self.responses.lock().await;
        if responses.is_empty() {
            Err(ServiceError::McpTransport("no more mock responses".into()))
        } else {
            Ok(responses.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_transport_construction() {
        let transport = HttpTransport::new("http://localhost:8080".into());
        assert_eq!(transport.endpoint, "http://localhost:8080");
    }

    #[tokio::test]
    async fn mock_transport_returns_responses() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!({"tools": []})),
            error: None,
        };

        let transport = MockTransport::new(vec![response]);
        let req = JsonRpcRequest::new(1, "tools/list", serde_json::json!({}));
        let resp = transport.send_request(req).await.unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
    }

    #[tokio::test]
    async fn mock_transport_records_requests() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: Some(serde_json::json!(null)),
            error: None,
        };

        let transport = MockTransport::new(vec![response]);
        let req = JsonRpcRequest::new(1, "test/method", serde_json::json!({"key": "value"}));
        transport.send_request(req).await.unwrap();

        let requests = transport.requests().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "test/method");
    }

    #[tokio::test]
    async fn mock_transport_empty_responses_errors() {
        let transport = MockTransport::new(vec![]);
        let req = JsonRpcRequest::new(1, "test", serde_json::json!({}));
        let result = transport.send_request(req).await;
        assert!(result.is_err());
    }
}
