//! Web search tool.
//!
//! Provides a `web_search` tool that queries the Brave Search API or a custom
//! search API endpoint. Falls back gracefully when no credentials are configured.
//!
//! # Configuration
//!
//! The tool accepts a [`WebSearchConfig`] which can provide:
//!
//! - An API key for the Brave Search API (constructs the endpoint automatically)
//! - A custom endpoint URL (used as-is, for self-hosted or alternative search APIs)
//!
//! When both are provided, the custom endpoint takes precedence.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::Platform;
use serde_json::json;
use tracing::debug;

/// Brave Search API base URL.
const BRAVE_SEARCH_ENDPOINT: &str = "https://api.search.brave.com/res/v1/web/search";

/// Configuration for the web search tool.
///
/// Supports two modes:
///
/// 1. **Brave Search API key**: Set `api_key` and the tool constructs the
///    proper Brave Search endpoint, adding the `X-Subscription-Token` header.
///
/// 2. **Custom endpoint**: Set `endpoint` to a URL that accepts `?q=<query>&count=<n>`
///    query parameters and returns JSON search results.
///
/// When both are provided, `endpoint` takes precedence.
#[derive(Debug, Clone, Default)]
pub struct WebSearchConfig {
    /// Brave Search API key. Used to authenticate against the Brave Search API.
    pub api_key: Option<String>,
    /// Custom search API endpoint URL. Overrides Brave Search when set.
    pub endpoint: Option<String>,
    /// Maximum number of results per query (default: 5).
    pub max_results: u32,
}

impl WebSearchConfig {
    /// Returns `true` if the config has enough information to perform searches.
    pub fn is_configured(&self) -> bool {
        self.has_api_key() || self.has_endpoint()
    }

    /// Returns `true` if a non-empty API key is set.
    fn has_api_key(&self) -> bool {
        self.api_key.as_ref().is_some_and(|k| !k.is_empty())
    }

    /// Returns `true` if a non-empty custom endpoint is set.
    fn has_endpoint(&self) -> bool {
        self.endpoint.as_ref().is_some_and(|e| !e.is_empty())
    }
}

/// Web search tool.
///
/// Sends a search query to the Brave Search API or a custom search endpoint
/// and returns results. If no API key or endpoint is configured, returns a
/// descriptive error message.
pub struct WebSearchTool<P: Platform> {
    platform: Arc<P>,
    config: WebSearchConfig,
}

impl<P: Platform> WebSearchTool<P> {
    /// Create a new web search tool with the given configuration.
    pub fn new(platform: Arc<P>, config: WebSearchConfig) -> Self {
        Self { platform, config }
    }

    /// Create a web search tool from a raw endpoint URL (legacy API).
    ///
    /// Provided for backward compatibility. Prefer [`new`](Self::new) with
    /// a [`WebSearchConfig`] instead.
    pub fn from_endpoint(platform: Arc<P>, endpoint: Option<String>) -> Self {
        Self {
            platform,
            config: WebSearchConfig {
                endpoint,
                max_results: 5,
                api_key: None,
            },
        }
    }

    /// Build the request URL and headers for the search.
    fn build_request(&self, query: &str, num_results: usize) -> (String, HashMap<String, String>) {
        let mut headers = HashMap::new();

        if self.config.has_endpoint() {
            // Custom endpoint mode: no auth headers, use endpoint as-is.
            let endpoint = self.config.endpoint.as_ref().unwrap();
            let url = format!(
                "{}?q={}&count={}",
                endpoint,
                urlencoding_minimal(query),
                num_results
            );
            (url, headers)
        } else {
            // Brave Search API mode: use the standard endpoint with auth header.
            let api_key = self.config.api_key.as_ref().unwrap();
            headers.insert("X-Subscription-Token".to_string(), api_key.clone());
            headers.insert("Accept".to_string(), "application/json".to_string());
            let url = format!(
                "{}?q={}&count={}",
                BRAVE_SEARCH_ENDPOINT,
                urlencoding_minimal(query),
                num_results
            );
            (url, headers)
        }
    }
}

#[async_trait]
impl<P: Platform + 'static> Tool for WebSearchTool<P> {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns a list of search results with titles, URLs, and snippets."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query string"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("missing required field: query".into()))?;

        let num_results = args
            .get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.config.max_results as u64) as usize;

        debug!(query = %query, num_results = num_results, "executing web search");

        if !self.config.is_configured() {
            return Ok(json!({
                "error": "web search not configured",
                "message": "No search API key or endpoint is configured. Set 'tools.web.search.api_key' (Brave Search) or 'tools.web.search.endpoint' (custom) in your config.",
                "query": query,
            }));
        }

        let (url, headers) = self.build_request(query, num_results);

        let result = self
            .platform
            .http()
            .request("GET", &url, &headers, None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("search request failed: {e}")))?;

        if result.status >= 400 {
            return Err(ToolError::ExecutionFailed(format!(
                "search API returned status {}",
                result.status
            )));
        }

        // Parse the response body as JSON.
        let body = String::from_utf8_lossy(&result.body);
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(parsed) => Ok(json!({
                "query": query,
                "results": parsed,
            })),
            Err(_) => Ok(json!({
                "query": query,
                "results": body.to_string(),
            })),
        }
    }
}

/// Minimal URL encoding for query strings.
fn urlencoding_minimal(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => result.push_str("%20"),
            '&' => result.push_str("%26"),
            '=' => result.push_str("%3D"),
            '+' => result.push_str("%2B"),
            '#' => result.push_str("%23"),
            '%' => result.push_str("%25"),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;

    fn make_tool_configured(
        api_key: Option<&str>,
        endpoint: Option<&str>,
    ) -> WebSearchTool<NativePlatform> {
        WebSearchTool::new(
            Arc::new(NativePlatform::new()),
            WebSearchConfig {
                api_key: api_key.map(|s| s.to_string()),
                endpoint: endpoint.map(|s| s.to_string()),
                max_results: 5,
            },
        )
    }

    fn make_tool(endpoint: Option<&str>) -> WebSearchTool<NativePlatform> {
        WebSearchTool::from_endpoint(
            Arc::new(NativePlatform::new()),
            endpoint.map(|s| s.to_string()),
        )
    }

    #[test]
    fn name_is_web_search() {
        let tool = make_tool(None);
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn description_is_not_empty() {
        let tool = make_tool(None);
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn parameters_has_query() {
        let tool = make_tool(None);
        let params = tool.parameters();
        assert!(params["properties"]["query"].is_object());
        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&json!("query")));
    }

    #[tokio::test]
    async fn execute_without_endpoint_returns_not_configured() {
        let tool = make_tool(None);
        let result = tool.execute(json!({"query": "test"})).await.unwrap();
        assert_eq!(result["error"], "web search not configured");
        assert_eq!(result["query"], "test");
    }

    #[tokio::test]
    async fn execute_with_empty_endpoint_returns_not_configured() {
        let tool = make_tool(Some(""));
        let result = tool.execute(json!({"query": "test"})).await.unwrap();
        assert_eq!(result["error"], "web search not configured");
    }

    #[tokio::test]
    async fn execute_missing_query_returns_error() {
        let tool = make_tool(Some("http://example.com"));
        let err = tool.execute(json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[test]
    fn urlencoding_spaces() {
        assert_eq!(urlencoding_minimal("hello world"), "hello%20world");
    }

    #[test]
    fn urlencoding_special_chars() {
        assert_eq!(urlencoding_minimal("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn urlencoding_plain() {
        assert_eq!(urlencoding_minimal("hello"), "hello");
    }

    #[test]
    fn tool_is_object_safe() {
        fn accepts_tool(_t: &dyn Tool) {}
        let tool = make_tool(None);
        accepts_tool(&tool);
    }

    // -- WebSearchConfig tests --

    #[test]
    fn config_not_configured_when_empty() {
        let config = WebSearchConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn config_configured_with_api_key() {
        let config = WebSearchConfig {
            api_key: Some("test-key".into()),
            endpoint: None,
            max_results: 5,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn config_configured_with_endpoint() {
        let config = WebSearchConfig {
            api_key: None,
            endpoint: Some("https://search.example.com".into()),
            max_results: 5,
        };
        assert!(config.is_configured());
    }

    #[test]
    fn config_not_configured_with_empty_strings() {
        let config = WebSearchConfig {
            api_key: Some(String::new()),
            endpoint: Some(String::new()),
            max_results: 5,
        };
        assert!(!config.is_configured());
    }

    // -- build_request tests --

    #[test]
    fn build_request_brave_api_key() {
        let tool = make_tool_configured(Some("my-brave-key"), None);
        let (url, headers) = tool.build_request("rust programming", 3);

        assert!(url.starts_with(BRAVE_SEARCH_ENDPOINT));
        assert!(url.contains("q=rust%20programming"));
        assert!(url.contains("count=3"));
        assert_eq!(headers.get("X-Subscription-Token").unwrap(), "my-brave-key");
        assert_eq!(headers.get("Accept").unwrap(), "application/json");
    }

    #[test]
    fn build_request_custom_endpoint() {
        let tool = make_tool_configured(Some("my-key"), Some("https://search.example.com/api"));
        let (url, headers) = tool.build_request("test query", 10);

        // Custom endpoint takes precedence over API key
        assert!(url.starts_with("https://search.example.com/api"));
        assert!(url.contains("q=test%20query"));
        assert!(url.contains("count=10"));
        // No auth headers for custom endpoint
        assert!(headers.is_empty());
    }

    #[test]
    fn build_request_encodes_special_chars() {
        let tool = make_tool_configured(Some("key"), None);
        let (url, _) = tool.build_request("a&b=c", 5);
        assert!(url.contains("q=a%26b%3Dc"));
    }

    // -- from_endpoint backward compatibility --

    #[test]
    fn from_endpoint_with_some() {
        let tool = make_tool(Some("http://custom.search"));
        assert!(tool.config.is_configured());
        assert_eq!(
            tool.config.endpoint.as_deref(),
            Some("http://custom.search")
        );
    }

    #[test]
    fn from_endpoint_with_none() {
        let tool = make_tool(None);
        assert!(!tool.config.is_configured());
    }

    #[tokio::test]
    async fn execute_with_api_key_not_configured_message() {
        let tool = make_tool_configured(None, None);
        let result = tool.execute(json!({"query": "hello"})).await.unwrap();
        assert_eq!(result["error"], "web search not configured");
    }

    #[tokio::test]
    async fn execute_uses_config_max_results_default() {
        // When num_results is not passed, should use config's max_results
        let tool = make_tool_configured(None, None);
        let result = tool.execute(json!({"query": "test"})).await.unwrap();
        // Just check it doesn't crash; config max_results=5 is the default
        assert_eq!(result["error"], "web search not configured");
    }
}
