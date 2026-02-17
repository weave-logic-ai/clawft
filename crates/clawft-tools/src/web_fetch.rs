//! Web fetch tool.
//!
//! Provides a `web_fetch` tool that retrieves content from a URL using
//! the platform HTTP client.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::Platform;
use serde_json::json;
use tracing::debug;

/// Maximum response body size in bytes (1 MB).
const MAX_RESPONSE_BYTES: usize = 1_048_576;

/// Web fetch tool.
///
/// Fetches content from a given URL and returns it as text. Enforces
/// a maximum response size to prevent memory exhaustion.
pub struct WebFetchTool<P: Platform> {
    platform: Arc<P>,
}

impl<P: Platform> WebFetchTool<P> {
    pub fn new(platform: Arc<P>) -> Self {
        Self { platform }
    }
}

#[async_trait]
impl<P: Platform + 'static> Tool for WebFetchTool<P> {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL. Returns the response body as text (HTML, JSON, etc.)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, etc.). Defaults to GET."
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers as key-value pairs"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let url = args.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: url".into()))?;

        // Basic URL validation.
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidArgs(
                "url must start with http:// or https://".into(),
            ));
        }

        let method = args.get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let headers: std::collections::HashMap<String, String> = args
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        debug!(url = %url, method = %method, "fetching web content");

        let response = self.platform
            .http()
            .request(&method, url, &headers, None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("fetch failed: {e}")))?;

        let status = response.status;
        let body = if response.body.len() > MAX_RESPONSE_BYTES {
            let truncated = String::from_utf8_lossy(&response.body[..MAX_RESPONSE_BYTES]);
            format!("{truncated}\n\n[... truncated at {MAX_RESPONSE_BYTES} bytes]")
        } else {
            String::from_utf8_lossy(&response.body).into_owned()
        };

        let content_type = response.headers
            .get("content-type")
            .or_else(|| response.headers.get("Content-Type"))
            .cloned()
            .unwrap_or_default();

        Ok(json!({
            "status": status,
            "content_type": content_type,
            "body": body,
            "url": url,
            "bytes": response.body.len(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;

    fn make_tool() -> WebFetchTool<NativePlatform> {
        WebFetchTool::new(Arc::new(NativePlatform::new()))
    }

    #[test]
    fn name_is_web_fetch() {
        assert_eq!(make_tool().name(), "web_fetch");
    }

    #[test]
    fn description_not_empty() {
        assert!(!make_tool().description().is_empty());
    }

    #[test]
    fn parameters_has_url() {
        let params = make_tool().parameters();
        assert!(params["properties"]["url"].is_object());
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("url")));
    }

    #[tokio::test]
    async fn missing_url_returns_error() {
        let err = make_tool().execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn invalid_url_scheme_returns_error() {
        let err = make_tool()
            .execute(json!({"url": "ftp://example.com"}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
        assert!(err.to_string().contains("http"));
    }

    #[test]
    fn tool_is_object_safe() {
        fn accepts_tool(_t: &dyn Tool) {}
        accepts_tool(&make_tool());
    }
}
