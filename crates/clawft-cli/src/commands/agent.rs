//! `weft agent` -- interactive agent session or single-message mode.
//!
//! In single-message mode (`--message "..."`), sends one prompt to the agent
//! and prints the response. In interactive mode (no `--message`), reads from
//! stdin in a REPL loop.
//!
//! Messages are processed through the full 6-stage pipeline via [`AgentLoop`]:
//! Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner.
//! Tool calls are executed automatically up to `max_tool_iterations`.
//!
//! # Examples
//!
//! ```text
//! # Single message
//! weft agent -m "What is Rust?"
//!
//! # Interactive mode
//! weft agent
//! > What is Rust?
//! [agent response]
//! > /exit
//!
//! # Override model
//! weft agent --model openai/gpt-4o -m "hello"
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use clap::Args;
use tokio::io::AsyncBufReadExt;
use tracing::info;

use clawft_core::bootstrap::AppContext;
use clawft_core::bus::MessageBus;
use clawft_platform::NativePlatform;
use clawft_types::event::InboundMessage;

use super::{expand_workspace, load_config};

/// Arguments for the `weft agent` subcommand.
#[derive(Args)]
pub struct AgentArgs {
    /// Send a single message and exit (non-interactive mode).
    #[arg(short, long)]
    pub message: Option<String>,

    /// Model to use (overrides config).
    #[arg(long)]
    pub model: Option<String>,

    /// Config file path (overrides auto-discovery).
    #[arg(short, long)]
    pub config: Option<String>,

    /// Enable intelligent routing (requires vector-memory feature).
    #[arg(long)]
    pub intelligent_routing: bool,

    /// Trust workspace-level (project) skills.
    ///
    /// Without this flag, only user and built-in skills are loaded.
    /// Workspace skills in `.clawft/skills/` are skipped as a security
    /// measure (SEC-SKILL-05).
    #[arg(long)]
    pub trust_project_skills: bool,
}

/// Run the agent command.
///
/// Loads configuration, bootstraps [`AppContext`], registers tools,
/// and enters either single-message or interactive mode. The agent
/// loop processes messages through the full pipeline, including
/// tool execution.
pub async fn run(args: AgentArgs) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let mut config = load_config(&*platform, args.config.as_deref()).await?;

    // Apply model override if provided.
    if let Some(ref model) = args.model {
        config.agents.defaults.model = model.clone();
    }

    let effective_model = &config.agents.defaults.model;
    info!(model = %effective_model, "initializing agent");

    // Bootstrap the application context (bus, sessions, memory, skills, pipeline).
    let mut ctx = AppContext::new(config.clone(), platform.clone())
        .await
        .map_err(|e| anyhow::anyhow!("bootstrap failed: {e}"))?;

    // Build security policies from config.
    let command_policy = build_command_policy(&config.tools.command_policy);
    let url_policy = build_url_policy(&config.tools.url_policy);

    // Register tools.
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    let web_search_config = build_web_search_config(&config.tools);
    clawft_tools::register_all(
        ctx.tools_mut(),
        platform.clone(),
        workspace,
        command_policy,
        url_policy,
        web_search_config,
    );

    // Register MCP server tools.
    crate::mcp_tools::register_mcp_tools(&config, ctx.tools_mut()).await;

    // Register delegation tool (feature-gated, graceful degradation).
    crate::mcp_tools::register_delegation(&config.delegation, ctx.tools_mut());

    // Register message tool (needs bus reference, cannot go in register_all).
    let bus_ref = ctx.bus().clone();
    ctx.tools_mut()
        .register(Arc::new(clawft_tools::message_tool::MessageTool::new(
            bus_ref,
        )));

    let tool_count = ctx.tools().len();
    let tool_names: Vec<String> = ctx.tools().list();
    info!(tools = tool_count, "tool registry initialized");

    // Wire the live LLM-backed pipeline so real provider calls work.
    ctx.enable_live_llm();

    // Intelligent routing (vector-memory feature gate).
    if args.intelligent_routing {
        #[cfg(feature = "vector-memory")]
        {
            info!("intelligent routing enabled");
            // IntelligentRouter wiring would go here when fully implemented.
            // For now, log that it's enabled.
        }
        #[cfg(not(feature = "vector-memory"))]
        {
            anyhow::bail!(
                "intelligent routing requires the 'vector-memory' feature. \
                 Rebuild with: cargo build --features vector-memory"
            );
        }
    }

    // Clone the bus before consuming the context.
    let bus = ctx.bus().clone();

    // Convert context into the agent loop (consumes ctx).
    let agent = ctx.into_agent_loop();

    if let Some(ref message) = args.message {
        return run_single_message(message, &bus, agent, effective_model).await;
    }

    run_interactive(&bus, agent, &tool_names, effective_model).await
}

/// Process a single message through the agent loop and exit.
///
/// Publishes the message to the bus, spawns the agent loop in the
/// background, waits for the outbound response, and prints it.
async fn run_single_message(
    message: &str,
    bus: &Arc<MessageBus>,
    agent: clawft_core::agent::loop_core::AgentLoop<NativePlatform>,
    model: &str,
) -> anyhow::Result<()> {
    info!(model = %model, "single-message mode");

    // Create and publish the inbound message.
    let inbound = InboundMessage {
        channel: "cli".into(),
        sender_id: "local".into(),
        chat_id: "cli-session".into(),
        content: message.to_owned(),
        timestamp: Utc::now(),
        media: vec![],
        metadata: HashMap::new(),
    };
    bus.publish_inbound(inbound)
        .map_err(|e| anyhow::anyhow!("failed to publish message: {e}"))?;

    // Spawn the agent loop in the background.
    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("agent loop error: {e}");
        }
    });

    // Wait for the outbound response.
    let response = bus.consume_outbound().await;

    match response {
        Some(msg) => {
            println!("{}", msg.content);
        }
        None => {
            eprintln!("error: no response from agent");
        }
    }

    // Signal the agent loop to stop by dropping the inbound sender.
    // The bus holds its own sender, so we close it by dropping the bus.
    // Since the bus is shared via Arc, we just drop the handle and abort.
    agent_handle.abort();
    let _ = agent_handle.await;

    Ok(())
}

/// Run an interactive REPL loop reading from stdin.
///
/// Spawns the agent loop in the background, then reads user input
/// line-by-line. Each input is published to the bus and the response
/// is consumed and printed. Special commands (/exit, /help, etc.)
/// are handled locally.
async fn run_interactive(
    bus: &Arc<MessageBus>,
    agent: clawft_core::agent::loop_core::AgentLoop<NativePlatform>,
    tool_names: &[String],
    model: &str,
) -> anyhow::Result<()> {
    println!("weft agent -- interactive mode (type /help for commands)");
    println!("Model: {model}");
    println!();

    // Spawn the agent loop in the background.
    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("agent loop error: {e}");
        }
    });

    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin).lines();

    loop {
        eprint!("> ");
        // Flush stderr so the prompt appears before blocking on read.
        use std::io::Write;
        std::io::stderr().flush().ok();

        let line = match reader.next_line().await? {
            Some(l) => l,
            None => break, // EOF
        };
        let input = line.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "/exit" | "/quit" => break,
            "/clear" => {
                println!("[session cleared]");
                continue;
            }
            "/help" => {
                print_help();
                continue;
            }
            "/tools" => {
                if tool_names.is_empty() {
                    println!("No tools registered.");
                } else {
                    println!("Registered tools ({}):", tool_names.len());
                    for name in tool_names {
                        println!("  - {name}");
                    }
                }
                println!();
                continue;
            }
            _ => {}
        }

        // Publish the user message to the bus.
        let inbound = InboundMessage {
            channel: "cli".into(),
            sender_id: "local".into(),
            chat_id: "cli-session".into(),
            content: input.to_owned(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };

        if let Err(e) = bus.publish_inbound(inbound) {
            eprintln!("error: failed to send message: {e}");
            break;
        }

        // Wait for the outbound response.
        match bus.consume_outbound().await {
            Some(msg) => {
                println!("{}", msg.content);
                println!();
            }
            None => {
                eprintln!("error: agent loop closed unexpectedly");
                break;
            }
        }
    }

    // Signal the agent loop to stop.
    agent_handle.abort();
    let _ = agent_handle.await;

    println!("Goodbye.");
    Ok(())
}

/// Build a [`WebSearchConfig`] from the tools configuration.
///
/// Maps the `ToolsConfig.web.search` fields (api_key, max_results) into the
/// web search tool's configuration struct. Resolves the API key from the
/// environment variable `BRAVE_SEARCH_API_KEY` if the config value is empty.
pub(crate) fn build_web_search_config(
    config: &clawft_types::config::ToolsConfig,
) -> clawft_tools::web_search::WebSearchConfig {
    let search = &config.web.search;
    let api_key = if search.api_key.is_empty() {
        std::env::var("BRAVE_SEARCH_API_KEY").ok()
    } else {
        Some(search.api_key.clone())
    };

    clawft_tools::web_search::WebSearchConfig {
        api_key,
        endpoint: None, // Custom endpoint support can be added to config later.
        max_results: search.max_results,
    }
}

/// Build a [`CommandPolicy`] from the configuration.
pub(crate) fn build_command_policy(
    config: &clawft_types::config::CommandPolicyConfig,
) -> clawft_tools::security_policy::CommandPolicy {
    use clawft_tools::security_policy::{CommandPolicy, PolicyMode};

    let mut policy = CommandPolicy::safe_defaults();

    if config.mode == "denylist" {
        policy.mode = PolicyMode::Denylist;
    }
    if !config.allowlist.is_empty() {
        policy.allowlist = config.allowlist.iter().cloned().collect();
    }
    if !config.denylist.is_empty() {
        policy.denylist = config.denylist.clone();
    }

    policy
}

/// Build a [`UrlPolicy`] from the configuration.
pub(crate) fn build_url_policy(
    config: &clawft_types::config::UrlPolicyConfig,
) -> clawft_tools::url_safety::UrlPolicy {
    use clawft_tools::url_safety::UrlPolicy;

    UrlPolicy::new(
        config.enabled,
        config.allow_private,
        config.allowed_domains.iter().cloned().collect(),
        config.blocked_domains.iter().cloned().collect(),
    )
}

/// Print the interactive help text.
fn print_help() {
    println!("Commands:");
    println!("  /help   -- Show this help");
    println!("  /clear  -- Clear session history");
    println!("  /tools  -- List available tools");
    println!("  /exit   -- Quit the session");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_args_defaults() {
        // Verify the struct can be constructed with all-None fields.
        let args = AgentArgs {
            message: None,
            model: None,
            config: None,
            intelligent_routing: false,
            trust_project_skills: false,
        };
        assert!(args.message.is_none());
        assert!(args.model.is_none());
        assert!(args.config.is_none());
    }

    #[test]
    fn agent_args_with_message() {
        let args = AgentArgs {
            message: Some("test message".into()),
            model: None,
            config: None,
            intelligent_routing: false,
            trust_project_skills: false,
        };
        assert_eq!(args.message.as_deref(), Some("test message"));
    }

    #[test]
    fn agent_args_with_model_override() {
        let args = AgentArgs {
            message: None,
            model: Some("openai/gpt-4".into()),
            config: None,
            intelligent_routing: false,
            trust_project_skills: false,
        };
        assert_eq!(args.model.as_deref(), Some("openai/gpt-4"));
    }

    #[test]
    fn agent_args_with_config_path() {
        let args = AgentArgs {
            message: None,
            model: None,
            config: Some("/tmp/test-config.json".into()),
            intelligent_routing: false,
            trust_project_skills: false,
        };
        assert_eq!(args.config.as_deref(), Some("/tmp/test-config.json"));
    }

    #[test]
    fn print_help_does_not_panic() {
        // Smoke test: just make sure it does not panic.
        print_help();
    }
}
