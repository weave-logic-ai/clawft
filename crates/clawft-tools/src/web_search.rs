//! Web search tool.
//!
//! Provides a `web_search` tool that queries a configurable search API endpoint.
//! Falls back gracefully when no search endpoint is configured.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::Platform;
use serde_json::json;
use tracing::debug;

/// Web search tool.
///
/// Sends a search query to a configurable API endpoint and returns results.
/// If no endpoint is configured, returns a descriptive error message.
pub struct WebSearchTool<P: Platform> {
    platform: Arc<P>,
    /// Optional search API endpoint URL.
    endpoint: Option<String>,
}

impl<P: Platform> WebSearchTool<P> {
    /// Create a new web search tool.
    ///
    /// If `endpoint` is `None`, the tool will return a helpful error
    /// indicating that web search is not configured.
    pub fn new(platform: Arc<P>, endpoint: Option<String>) -> Self {
        Self { platform, endpoint }
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
            .unwrap_or(5) as usize;

        debug!(query = %query, num_results = num_results, "executing web search");

        let endpoint = match &self.endpoint {
            Some(ep) if !ep.is_empty() => ep,
            _ => {
                return Ok(json!({
                    "error": "web search not configured",
                    "message": "No search API endpoint is configured. Set 'tools.web_search.endpoint' in your config.",
                    "query": query,
                }));
            }
        };

        // Build the search request URL.
        let url = format!(
            "{}?q={}&limit={}",
            endpoint,
            urlencoding_minimal(query),
            num_results
        );

        let headers = std::collections::HashMap::new();
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

    fn make_tool(endpoint: Option<&str>) -> WebSearchTool<NativePlatform> {
        WebSearchTool::new(
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
}
