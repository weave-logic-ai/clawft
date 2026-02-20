//! MCP transport implementations.
//!
//! Provides [`McpTransport`] trait and two implementations:
//! - [`StdioTransport`]: communicates with a child process over stdin/stdout
//!   using request-ID multiplexing for concurrent requests
//! - [`HttpTransport`]: communicates over HTTP POST

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, oneshot};
use tracing::{debug, warn};

use super::types::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::error::{Result, ServiceError};

/// Transport layer for MCP JSON-RPC communication.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a JSON-RPC request and return the response.
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Send a JSON-RPC notification (no `id`, no response expected).
    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()>;
}

/// Pending response registry: maps request IDs to oneshot senders.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>;

/// Transport that communicates with a child process via stdin/stdout.
///
/// Uses a background reader task and request-ID multiplexing to support
/// concurrent requests. Each `send_request` call registers a oneshot
/// channel keyed by the request ID, writes to stdin, and waits for the
/// background reader to deliver the matching response.
pub struct StdioTransport {
    #[allow(dead_code)]
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending: PendingMap,
    #[allow(dead_code)]
    reader_handle: Arc<tokio::task::JoinHandle<()>>,
}

impl StdioTransport {
    /// Spawn a child process and set up JSON-RPC communication with
    /// request-ID multiplexing.
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

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));

        // Spawn background reader task that reads lines from stdout and
        // dispatches responses to the matching pending oneshot sender.
        let reader_pending = Arc::clone(&pending);
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        debug!("stdio reader: child process closed stdout");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<JsonRpcResponse>(trimmed) {
                            Ok(response) => {
                                let id = response.id;
                                let mut map = reader_pending.lock().await;
                                if let Some(tx) = map.remove(&id) {
                                    let _ = tx.send(response);
                                } else {
                                    warn!(id, "stdio reader: received response with no pending request");
                                }
                            }
                            Err(e) => {
                                // Could be a notification or malformed line; skip
                                debug!(error = %e, "stdio reader: ignoring non-response line");
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "stdio reader: read error, exiting");
                        break;
                    }
                }
            }

            // Signal all pending requests that the reader has stopped.
            let mut map = reader_pending.lock().await;
            map.clear();
        });

        Ok(Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            reader_handle: Arc::new(reader_handle),
        })
    }
}

/// Default timeout for waiting on a response from the child process.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');

        let id = request.id;
        debug!(method = %request.method, id, "sending stdio request");

        // Register a oneshot channel for this request ID.
        let (tx, rx) = oneshot::channel::<JsonRpcResponse>();
        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }

        // Write to stdin.
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await.map_err(|e| {
                ServiceError::McpTransport(format!("failed to write to stdin: {e}"))
            })?;
            stdin
                .flush()
                .await
                .map_err(|e| ServiceError::McpTransport(format!("failed to flush stdin: {e}")))?;
        }

        // Wait for the background reader to deliver the response, with timeout.
        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Oneshot sender was dropped (reader task exited).
                Err(ServiceError::McpTransport(
                    "child process closed stdout before responding".into(),
                ))
            }
            Err(_) => {
                // Timeout: remove the pending entry.
                let mut map = self.pending.lock().await;
                map.remove(&id);
                Err(ServiceError::McpTransport(format!(
                    "request {id} timed out after {}s",
                    REQUEST_TIMEOUT.as_secs()
                )))
            }
        }
    }

    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let notif = JsonRpcNotification::new(method, params);
        let mut line = serde_json::to_string(&notif)?;
        line.push('\n');

        debug!(method = %method, "sending stdio notification");

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to write notification to stdin: {e}"))
        })?;
        stdin.flush().await.map_err(|e| {
            ServiceError::McpTransport(format!("failed to flush stdin after notification: {e}"))
        })?;

        // Notifications do not expect a response -- do NOT read from stdout.
        Ok(())
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
            return Err(ServiceError::McpTransport(format!("HTTP {status}: {body}")));
        }

        let response: JsonRpcResponse = resp
            .json()
            .await
            .map_err(|e| ServiceError::McpTransport(format!("failed to parse response: {e}")))?;

        Ok(response)
    }

    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let notif = JsonRpcNotification::new(method, params);

        debug!(
            method = %method,
            endpoint = %self.endpoint,
            "sending HTTP notification"
        );

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&notif)
            .send()
            .await
            .map_err(|e| ServiceError::McpTransport(format!("HTTP notification failed: {e}")))?;

        // Log non-success status but don't fail -- notifications are fire-and-forget.
        let status = resp.status();
        if !status.is_success() {
            debug!(
                method = %method,
                status = %status,
                "HTTP notification received non-success status"
            );
        }

        Ok(())
    }
}

/// A mock transport for testing.
///
/// Allows pre-programming responses that will be returned in order.
/// Also records all sent notifications for verification.
#[cfg(test)]
pub struct MockTransport {
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
    requests: Arc<Mutex<Vec<JsonRpcRequest>>>,
    notifications: Arc<Mutex<Vec<JsonRpcNotification>>>,
}

#[cfg(test)]
impl MockTransport {
    /// Create a mock transport with pre-programmed responses.
    pub fn new(responses: Vec<JsonRpcResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::new(Mutex::new(Vec::new())),
            notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all requests that were sent through this transport.
    pub async fn requests(&self) -> Vec<JsonRpcRequest> {
        self.requests.lock().await.clone()
    }

    /// Get all notifications that were sent through this transport.
    pub async fn notifications(&self) -> Vec<JsonRpcNotification> {
        self.notifications.lock().await.clone()
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

    async fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let notif = JsonRpcNotification::new(method, params);
        self.notifications.lock().await.push(notif);
        Ok(())
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

    #[tokio::test]
    async fn mock_transport_records_notifications() {
        let transport = MockTransport::new(vec![]);
        transport
            .send_notification("notifications/initialized", serde_json::json!({}))
            .await
            .unwrap();
        transport
            .send_notification(
                "notifications/progress",
                serde_json::json!({"token": "abc"}),
            )
            .await
            .unwrap();

        let notifs = transport.notifications().await;
        assert_eq!(notifs.len(), 2);
        assert_eq!(notifs[0].method, "notifications/initialized");
        assert_eq!(notifs[1].method, "notifications/progress");
        assert_eq!(notifs[1].params["token"], "abc");
    }

    #[tokio::test]
    async fn notification_has_no_id_field() {
        let notif = JsonRpcNotification::new("test/notify", serde_json::json!({}));
        let json = serde_json::to_string(&notif).unwrap();
        // JSON-RPC notifications MUST NOT have an "id" field.
        assert!(!json.contains("\"id\""));
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test/notify\""));
    }
}
