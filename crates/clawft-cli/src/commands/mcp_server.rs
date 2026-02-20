//! `weft mcp-server` -- run clawft as an MCP server over stdio.
//!
//! Exposes all registered tools (builtin + MCP-proxied) as an MCP tool
//! server, reading JSON-RPC requests from stdin and writing responses to
//! stdout. This allows MCP clients (like Claude Desktop, Cursor, etc.) to
//! use clawft tools natively.
//!
//! # Lifecycle
//!
//! ```text
//! 1. Load config & build tool registry (same as `weft agent`)
//! 2. Create BuiltinToolProvider wrapping tool registry
//! 3. Build middleware pipeline (security, permissions, audit)
//! 4. Create McpServerShell and run on stdin/stdout
//! ```
//!
//! # Example
//!
//! ```text
//! weft mcp-server
//! weft mcp-server --config /path/to/config.json
//! ```

use std::sync::Arc;

use clap::Args;
use tracing::info;

use clawft_core::tools::registry::ToolRegistry;
use clawft_platform::NativePlatform;
use clawft_services::mcp::BuiltinToolProvider;
use clawft_services::mcp::ToolDefinition;
use clawft_services::mcp::composite::CompositeToolProvider;
use clawft_services::mcp::middleware::{
    AuditLog, Middleware, PermissionFilter, ResultGuard, SecurityGuard,
};
use clawft_services::mcp::server::McpServerShell;

use super::load_config;

/// Arguments for the `weft mcp-server` subcommand.
#[derive(Args)]
pub struct McpServerArgs {
    /// Config file path (overrides auto-discovery).
    #[arg(short, long)]
    pub config: Option<String>,
}

/// Run the MCP server command.
///
/// Loads configuration, builds the tool registry (identical to `weft agent`),
/// wraps it in a [`BuiltinToolProvider`], and serves tools over stdio using
/// [`McpServerShell`] with the full middleware pipeline.
pub async fn run(args: McpServerArgs) -> anyhow::Result<()> {
    info!("starting weft mcp-server");

    let platform = Arc::new(NativePlatform::new());
    let config = load_config(&*platform, args.config.as_deref()).await?;

    // ── Build tool registry (shared core tools) ────────────────────
    let mut registry = ToolRegistry::new();
    super::register_core_tools(&mut registry, &config, platform.clone()).await;

    let tool_count = registry.len();
    let tool_names = registry.list();
    info!(
        tools = tool_count,
        "tool registry initialized for MCP server"
    );

    // ── Convert registry to ToolProvider ─────────────────────────────
    let tool_defs = build_tool_definitions(&registry);
    let provider = build_builtin_provider(tool_defs, registry);

    // ── Build CompositeToolProvider ──────────────────────────────────
    let mut composite = CompositeToolProvider::new();
    composite.register(Box::new(provider));

    // ── Build middleware pipeline ────────────────────────────────────
    let security_guard = build_security_guard(&config.tools);
    let middlewares: Vec<Box<dyn Middleware>> = vec![
        Box::new(security_guard),
        Box::new(PermissionFilter::new(None)),
        Box::new(ResultGuard::default()),
        Box::new(AuditLog),
    ];

    // ── Create McpServerShell and run on stdin/stdout ────────────────
    let mut shell = McpServerShell::new(composite);
    for mw in middlewares {
        shell.add_middleware(mw);
    }

    info!(
        tools = tool_count,
        names = ?tool_names,
        "MCP server ready, reading from stdin"
    );

    let stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let stdout = tokio::io::stdout();
    shell.run(stdin, stdout).await?;

    info!("stdin closed, MCP server shutting down");
    Ok(())
}

/// Build [`ToolDefinition`] list from a populated [`ToolRegistry`].
fn build_tool_definitions(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    let schemas = registry.schemas();
    schemas
        .into_iter()
        .filter_map(|schema| {
            let func = schema.get("function")?;
            Some(ToolDefinition {
                name: func.get("name")?.as_str()?.to_string(),
                description: func
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                input_schema: func
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object"})),
            })
        })
        .collect()
}

/// Build a [`BuiltinToolProvider`] backed by a [`ToolRegistry`].
///
/// The registry is moved into an `Arc` and shared between the provider's
/// dispatcher closure and the outer scope.
fn build_builtin_provider(
    tool_defs: Vec<ToolDefinition>,
    registry: ToolRegistry,
) -> BuiltinToolProvider {
    let registry = Arc::new(registry);
    let reg_clone = registry.clone();

    BuiltinToolProvider::new(tool_defs, move |name, args| {
        let reg = reg_clone.clone();
        let name = name.to_string();
        Box::pin(async move {
            match reg.execute(&name, args, None).await {
                Ok(value) => Ok(serde_json::to_string(&value).unwrap_or_default()),
                Err(e) => Err(e.to_string()),
            }
        })
    })
}

/// Build a [`SecurityGuard`] middleware from the tools configuration.
///
/// Translates the CLI-level `CommandPolicyConfig` and `UrlPolicyConfig`
/// into the canonical policy types used by [`SecurityGuard`].
fn build_security_guard(tools_config: &clawft_types::config::ToolsConfig) -> SecurityGuard {
    use clawft_types::security::{CommandPolicy, PolicyMode, UrlPolicy};

    let cmd_cfg = &tools_config.command_policy;
    let mode = match cmd_cfg.mode.as_str() {
        "denylist" => PolicyMode::Denylist,
        _ => PolicyMode::Allowlist,
    };
    let mut mw_cmd = CommandPolicy::safe_defaults();
    mw_cmd.mode = mode;
    if !cmd_cfg.allowlist.is_empty() {
        mw_cmd.allowlist = cmd_cfg.allowlist.iter().cloned().collect();
    }
    if !cmd_cfg.denylist.is_empty() {
        mw_cmd.denylist = cmd_cfg.denylist.clone();
    }

    let url_cfg = &tools_config.url_policy;
    let mw_url = UrlPolicy::new(
        url_cfg.enabled,
        url_cfg.allow_private,
        url_cfg.allowed_domains.iter().cloned().collect(),
        url_cfg.blocked_domains.iter().cloned().collect(),
    );

    SecurityGuard::new(mw_cmd, mw_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_args_defaults() {
        let args = McpServerArgs { config: None };
        assert!(args.config.is_none());
    }

    #[test]
    fn mcp_server_args_with_config() {
        let args = McpServerArgs {
            config: Some("/tmp/config.json".into()),
        };
        assert_eq!(args.config.as_deref(), Some("/tmp/config.json"));
    }

    #[test]
    fn build_tool_definitions_from_registry() {
        use clawft_core::tools::registry::{Tool, ToolError as CoreToolError, ToolRegistry};

        struct TestTool;

        #[async_trait::async_trait]
        impl Tool for TestTool {
            fn name(&self) -> &str {
                "test_tool"
            }
            fn description(&self) -> &str {
                "A test tool"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                })
            }
            async fn execute(
                &self,
                _args: serde_json::Value,
            ) -> Result<serde_json::Value, CoreToolError> {
                Ok(serde_json::json!({"output": "ok"}))
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool));

        let defs = build_tool_definitions(&registry);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "test_tool");
        assert_eq!(defs[0].description, "A test tool");
        assert_eq!(defs[0].input_schema["type"], "object");
    }

    #[test]
    fn build_tool_definitions_empty_registry() {
        let registry = ToolRegistry::new();
        let defs = build_tool_definitions(&registry);
        assert!(defs.is_empty());
    }

    #[tokio::test]
    async fn builtin_provider_dispatches_to_registry() {
        use clawft_core::tools::registry::{Tool, ToolError as CoreToolError, ToolRegistry};
        use clawft_services::mcp::ToolProvider;

        struct EchoTool;

        #[async_trait::async_trait]
        impl Tool for EchoTool {
            fn name(&self) -> &str {
                "echo"
            }
            fn description(&self) -> &str {
                "Echo input"
            }
            fn parameters(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" }
                    },
                    "required": ["text"]
                })
            }
            async fn execute(
                &self,
                args: serde_json::Value,
            ) -> Result<serde_json::Value, CoreToolError> {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CoreToolError::InvalidArgs("missing 'text'".into()))?;
                Ok(serde_json::json!({"output": text}))
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        let defs = build_tool_definitions(&registry);
        let provider = build_builtin_provider(defs, registry);

        let result = provider
            .call_tool("echo", serde_json::json!({"text": "hello"}))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            clawft_services::mcp::ContentBlock::Text { text } => {
                // The output is JSON-serialized from the tool result.
                let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
                assert_eq!(parsed["output"], "hello");
            }
        }
    }

    #[tokio::test]
    async fn builtin_provider_not_found() {
        use clawft_services::mcp::ToolProvider;

        let registry = ToolRegistry::new();
        let defs = build_tool_definitions(&registry);
        let provider = build_builtin_provider(defs, registry);

        let result = provider
            .call_tool("nonexistent", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            clawft_services::mcp::ToolError::NotFound(name) => {
                assert_eq!(name, "nonexistent");
            }
            other => panic!("expected NotFound, got: {other}"),
        }
    }
}
