//! Composite tool provider that aggregates multiple [`ToolProvider`]s
//! with namespace-based routing.
//!
//! Tool names are prefixed with `"{namespace}__"` when listed, and
//! incoming calls are split on the first `"__"` to route to the
//! correct provider.

use serde_json::Value;

use super::ToolDefinition;
use super::provider::{CallToolResult, ToolError, ToolProvider};

/// Aggregates multiple [`ToolProvider`]s and routes tool calls by namespace.
pub struct CompositeToolProvider {
    providers: Vec<Box<dyn ToolProvider>>,
}

impl CompositeToolProvider {
    /// Create an empty composite provider.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a tool provider.
    pub fn register(&mut self, provider: Box<dyn ToolProvider>) {
        self.providers.push(provider);
    }

    /// Return the number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// List tools from all providers, with names prefixed as
    /// `"{namespace}__{tool_name}"`.
    pub fn list_tools_all(&self) -> Vec<ToolDefinition> {
        let mut all = Vec::new();
        for provider in &self.providers {
            let ns = provider.namespace();
            for mut tool in provider.list_tools() {
                tool.name = format!("{ns}__{}", tool.name);
                all.push(tool);
            }
        }
        all
    }

    /// Route a tool call to the correct provider.
    ///
    /// The `namespaced_name` is split on the first `"__"` separator.
    /// If no separator is found, all providers are tried in order.
    pub async fn call_tool(
        &self,
        namespaced_name: &str,
        args: Value,
    ) -> Result<CallToolResult, ToolError> {
        if let Some((ns, local)) = namespaced_name.split_once("__") {
            // Find provider by namespace.
            for provider in &self.providers {
                if provider.namespace() == ns {
                    return provider.call_tool(local, args).await;
                }
            }
            Err(ToolError::NotFound(format!(
                "no provider for namespace \"{ns}\""
            )))
        } else {
            // No namespace separator -- try each provider in order.
            for provider in &self.providers {
                match provider.call_tool(namespaced_name, args.clone()).await {
                    Ok(result) => return Ok(result),
                    Err(ToolError::NotFound(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            Err(ToolError::NotFound(format!(
                "tool \"{namespaced_name}\" not found in any provider"
            )))
        }
    }
}

impl Default for CompositeToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    /// A mock provider for testing.
    struct MockProvider {
        ns: String,
        tools: Vec<ToolDefinition>,
    }

    impl MockProvider {
        fn new(ns: &str, tool_names: &[&str]) -> Self {
            let tools = tool_names
                .iter()
                .map(|name| ToolDefinition {
                    name: (*name).to_string(),
                    description: format!("{ns}/{name}"),
                    input_schema: json!({"type": "object"}),
                })
                .collect();
            Self {
                ns: ns.to_string(),
                tools,
            }
        }
    }

    #[async_trait]
    impl ToolProvider for MockProvider {
        fn namespace(&self) -> &str {
            &self.ns
        }

        fn list_tools(&self) -> Vec<ToolDefinition> {
            self.tools.clone()
        }

        async fn call_tool(&self, name: &str, _args: Value) -> Result<CallToolResult, ToolError> {
            if self.tools.iter().any(|t| t.name == name) {
                Ok(CallToolResult::text(format!("{}:{} called", self.ns, name)))
            } else {
                Err(ToolError::NotFound(name.to_string()))
            }
        }
    }

    #[test]
    fn default_is_empty() {
        let c = CompositeToolProvider::default();
        assert_eq!(c.provider_count(), 0);
        assert!(c.list_tools_all().is_empty());
    }

    #[test]
    fn list_tools_all_prefixes_names() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("alpha", &["foo", "bar"])));
        c.register(Box::new(MockProvider::new("beta", &["baz"])));

        let tools = c.list_tools_all();
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"alpha__foo"));
        assert!(names.contains(&"alpha__bar"));
        assert!(names.contains(&"beta__baz"));
    }

    #[test]
    fn list_tools_all_preserves_descriptions() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("ns", &["tool1"])));

        let tools = c.list_tools_all();
        assert_eq!(tools[0].description, "ns/tool1");
    }

    #[tokio::test]
    async fn call_tool_routes_by_namespace() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("alpha", &["foo"])));
        c.register(Box::new(MockProvider::new("beta", &["bar"])));

        let result = c.call_tool("alpha__foo", json!({})).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(
            result.content[0],
            super::super::provider::ContentBlock::Text {
                text: "alpha:foo called".into()
            }
        );

        let result = c.call_tool("beta__bar", json!({})).await.unwrap();
        assert_eq!(
            result.content[0],
            super::super::provider::ContentBlock::Text {
                text: "beta:bar called".into()
            }
        );
    }

    #[tokio::test]
    async fn call_tool_unknown_namespace_returns_not_found() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("alpha", &["foo"])));

        let err = c.call_tool("unknown__foo", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
        assert!(err.to_string().contains("unknown"));
    }

    #[tokio::test]
    async fn call_tool_unknown_tool_in_namespace_returns_not_found() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("alpha", &["foo"])));

        let err = c
            .call_tool("alpha__nonexistent", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn call_tool_without_namespace_tries_all() {
        let mut c = CompositeToolProvider::new();
        c.register(Box::new(MockProvider::new("alpha", &["shared"])));
        c.register(Box::new(MockProvider::new("beta", &["unique"])));

        // "shared" exists in alpha (the first provider).
        let result = c.call_tool("shared", json!({})).await.unwrap();
        assert_eq!(
            result.content[0],
            super::super::provider::ContentBlock::Text {
                text: "alpha:shared called".into()
            }
        );

        // "unique" is only in beta.
        let result = c.call_tool("unique", json!({})).await.unwrap();
        assert_eq!(
            result.content[0],
            super::super::provider::ContentBlock::Text {
                text: "beta:unique called".into()
            }
        );
    }

    #[tokio::test]
    async fn call_tool_without_namespace_not_found() {
        let c = CompositeToolProvider::new();
        let err = c.call_tool("missing", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn provider_count() {
        let mut c = CompositeToolProvider::new();
        assert_eq!(c.provider_count(), 0);
        c.register(Box::new(MockProvider::new("a", &["t"])));
        assert_eq!(c.provider_count(), 1);
        c.register(Box::new(MockProvider::new("b", &["u"])));
        assert_eq!(c.provider_count(), 2);
    }

    #[tokio::test]
    async fn call_tool_propagates_non_not_found_errors() {
        /// A provider that returns ExecutionFailed for all calls.
        struct FailProvider;

        #[async_trait]
        impl ToolProvider for FailProvider {
            fn namespace(&self) -> &str {
                "fail"
            }
            fn list_tools(&self) -> Vec<ToolDefinition> {
                vec![ToolDefinition {
                    name: "boom".into(),
                    description: "always fails".into(),
                    input_schema: json!({"type": "object"}),
                }]
            }
            async fn call_tool(
                &self,
                _name: &str,
                _args: Value,
            ) -> Result<CallToolResult, ToolError> {
                Err(ToolError::ExecutionFailed("kaboom".into()))
            }
        }

        let mut c = CompositeToolProvider::new();
        c.register(Box::new(FailProvider));

        // With namespace prefix.
        let err = c.call_tool("fail__boom", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));

        // Without prefix -- should stop at ExecutionFailed, not continue.
        let err = c.call_tool("boom", json!({})).await.unwrap_err();
        assert!(matches!(err, ToolError::ExecutionFailed(_)));
    }
}
