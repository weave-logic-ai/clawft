//! Web fetch tool.
//!
//! Provides a `web_fetch` tool that retrieves content from a URL using
//! the platform HTTP client.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::Platform;
use serde_json::json;
use tracing::{debug, warn};

use crate::url_safety::{UrlPolicy, validate_url};

/// Default maximum response body size in bytes (10 MB).
const DEFAULT_MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

/// Web fetch tool.
///
/// Fetches content from a given URL and returns it as text. Enforces
/// SSRF protection via [`UrlPolicy`] and a configurable maximum response
/// size to prevent memory exhaustion.
pub struct WebFetchTool<P: Platform> {
    platform: Arc<P>,
    url_policy: UrlPolicy,
    max_response_bytes: usize,
}

impl<P: Platform> WebFetchTool<P> {
    /// Create a new `WebFetchTool` with the default body size limit (10 MB).
    pub fn new(platform: Arc<P>, url_policy: UrlPolicy) -> Self {
        Self {
            platform,
            url_policy,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    /// Create a new `WebFetchTool` with a custom body size limit.
    pub fn with_max_bytes(platform: Arc<P>, url_policy: UrlPolicy, max_bytes: usize) -> Self {
        Self {
            platform,
            url_policy,
            max_response_bytes: max_bytes,
        }
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
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: url".into()))?;

        // Basic URL validation.
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidArgs(
                "url must start with http:// or https://".into(),
            ));
        }

        // SSRF protection: validate URL against policy.
        if let Err(e) = validate_url(url, &self.url_policy) {
            warn!(url, error = %e, "URL rejected by safety policy");
            return Err(ToolError::PermissionDenied(e.to_string()));
        }

        let method = args
            .get("method")
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

        let response = self
            .platform
            .http()
            .request(&method, url, &headers, None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("fetch failed: {e}")))?;

        let status = response.status;
        let total_bytes = response.body.len();
        let was_truncated = total_bytes > self.max_response_bytes;
        let body = if was_truncated {
            let truncated = String::from_utf8_lossy(&response.body[..self.max_response_bytes]);
            warn!(
                url,
                total_bytes,
                limit = self.max_response_bytes,
                "response body exceeds size limit, truncating"
            );
            format!(
                "{truncated}\n\n[... truncated at {} bytes, total response was {} bytes]",
                self.max_response_bytes, total_bytes
            )
        } else {
            String::from_utf8_lossy(&response.body).into_owned()
        };

        let content_type = response
            .headers
            .get("content-type")
            .or_else(|| response.headers.get("Content-Type"))
            .cloned()
            .unwrap_or_default();

        let mut result = json!({
            "status": status,
            "content_type": content_type,
            "body": body,
            "url": url,
            "bytes": total_bytes,
        });
        if was_truncated {
            result["truncated"] = json!(true);
            result["limit_bytes"] = json!(self.max_response_bytes);
            result["warning"] = json!(format!(
                "Response body ({total_bytes} bytes) exceeded the {} byte limit and was truncated",
                self.max_response_bytes
            ));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;

    fn make_tool() -> WebFetchTool<NativePlatform> {
        WebFetchTool::new(Arc::new(NativePlatform::new()), UrlPolicy::default())
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

    #[test]
    fn default_max_response_bytes_is_10mb() {
        let tool = make_tool();
        assert_eq!(tool.max_response_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn with_max_bytes_sets_custom_limit() {
        let tool = WebFetchTool::with_max_bytes(
            Arc::new(NativePlatform::new()),
            UrlPolicy::default(),
            500,
        );
        assert_eq!(tool.max_response_bytes, 500);
    }

    /// Test that oversized responses are truncated with warning metadata.
    ///
    /// Uses a mock platform that returns a large response body to verify
    /// that the tool truncates the body and includes truncation metadata
    /// in the result JSON.
    #[tokio::test]
    async fn oversized_response_is_truncated() {
        use clawft_platform::http::HttpResponse;
        use std::collections::HashMap;

        // Create a mock platform that returns a large body.
        struct MockHttp {
            body: Vec<u8>,
        }

        #[async_trait]
        impl clawft_platform::http::HttpClient for MockHttp {
            async fn request(
                &self,
                _method: &str,
                _url: &str,
                _headers: &HashMap<String, String>,
                _body: Option<&[u8]>,
            ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
                Ok(HttpResponse {
                    status: 200,
                    headers: HashMap::new(),
                    body: self.body.clone(),
                })
            }
        }

        struct MockPlatform {
            http: MockHttp,
            native: clawft_platform::NativePlatform,
        }

        #[async_trait]
        impl clawft_platform::Platform for MockPlatform {
            fn http(&self) -> &dyn clawft_platform::http::HttpClient {
                &self.http
            }
            fn fs(&self) -> &dyn clawft_platform::fs::FileSystem {
                self.native.fs()
            }
            fn env(&self) -> &dyn clawft_platform::env::Environment {
                self.native.env()
            }
            fn process(&self) -> Option<&dyn clawft_platform::process::ProcessSpawner> {
                self.native.process()
            }
        }

        let large_body = vec![b'A'; 2000]; // 2000 bytes
        let platform = Arc::new(MockPlatform {
            http: MockHttp { body: large_body },
            native: clawft_platform::NativePlatform::new(),
        });

        // Set limit to 500 bytes
        let tool = WebFetchTool::with_max_bytes(
            platform,
            UrlPolicy::permissive(), // Skip SSRF checks for test
            500,
        );

        let result = tool
            .execute(json!({"url": "https://example.com/large"}))
            .await
            .unwrap();

        // Verify truncation metadata is present.
        assert_eq!(result["truncated"], true);
        assert_eq!(result["limit_bytes"], 500);
        assert_eq!(result["bytes"], 2000);
        assert!(result["warning"].as_str().unwrap().contains("2000 bytes"));
        assert!(
            result["warning"]
                .as_str()
                .unwrap()
                .contains("500 byte limit")
        );

        // Verify the body contains the truncation marker.
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("[... truncated at 500 bytes"));
    }

    /// Test that responses within the limit are NOT truncated.
    #[tokio::test]
    async fn response_within_limit_not_truncated() {
        use clawft_platform::http::HttpResponse;
        use std::collections::HashMap;

        struct MockHttp;

        #[async_trait]
        impl clawft_platform::http::HttpClient for MockHttp {
            async fn request(
                &self,
                _method: &str,
                _url: &str,
                _headers: &HashMap<String, String>,
                _body: Option<&[u8]>,
            ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
                Ok(HttpResponse {
                    status: 200,
                    headers: HashMap::new(),
                    body: b"small body".to_vec(),
                })
            }
        }

        struct MockPlatform {
            http: MockHttp,
            native: clawft_platform::NativePlatform,
        }

        #[async_trait]
        impl clawft_platform::Platform for MockPlatform {
            fn http(&self) -> &dyn clawft_platform::http::HttpClient {
                &self.http
            }
            fn fs(&self) -> &dyn clawft_platform::fs::FileSystem {
                self.native.fs()
            }
            fn env(&self) -> &dyn clawft_platform::env::Environment {
                self.native.env()
            }
            fn process(&self) -> Option<&dyn clawft_platform::process::ProcessSpawner> {
                self.native.process()
            }
        }

        let platform = Arc::new(MockPlatform {
            http: MockHttp,
            native: clawft_platform::NativePlatform::new(),
        });

        let tool = WebFetchTool::with_max_bytes(platform, UrlPolicy::permissive(), 10000);

        let result = tool
            .execute(json!({"url": "https://example.com/small"}))
            .await
            .unwrap();

        // Should NOT have truncation metadata.
        assert!(result.get("truncated").is_none());
        assert!(result.get("warning").is_none());
        assert_eq!(result["body"], "small body");
        assert_eq!(result["bytes"], 10);
    }
}
