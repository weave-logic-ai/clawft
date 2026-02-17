//! `weft gateway` -- start channels and the agent processing loop.
//!
//! The gateway initializes configured channel plugins (Telegram, Slack,
//! Discord), starts them in background tasks, wires the [`AgentLoop`] for
//! message processing, and runs an outbound dispatch loop that routes
//! responses back to the originating channel.
//!
//! # Lifecycle
//!
//! ```text
//! 1. Load config & bootstrap AppContext (bus, sessions, tools, pipeline)
//! 2. Register + init enabled channel factories
//! 3. Start all channels (each in its own tokio task)
//! 4. Start background services (CronService, HeartbeatService)
//! 5. Spawn the agent loop (consumes inbound, produces outbound)
//! 6. Spawn the outbound dispatch loop (routes outbound to channels)
//! 7. Wait for Ctrl+C, then gracefully shut everything down
//! ```
//!
//! # Example
//!
//! ```text
//! weft gateway
//! weft gateway --config /path/to/config.json
//! ```

use std::sync::Arc;

use clap::Args;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

#[cfg(feature = "channels")]
use clawft_channels::discord::DiscordChannelFactory;
#[cfg(feature = "channels")]
use clawft_channels::slack::SlackChannelFactory;
#[cfg(feature = "channels")]
use clawft_channels::telegram::TelegramChannelFactory;
#[cfg(feature = "channels")]
use clawft_channels::PluginHost;
use clawft_core::bootstrap::AppContext;
use clawft_platform::NativePlatform;
#[cfg(feature = "services")]
use clawft_services::cron_service::CronService;
#[cfg(feature = "services")]
use clawft_services::heartbeat::HeartbeatService;

#[cfg(feature = "channels")]
use crate::markdown::dispatch::MarkdownDispatcher;

use super::{expand_workspace, load_config};
#[cfg(feature = "channels")]
use super::make_channel_host;

/// Arguments for the `weft gateway` subcommand.
#[derive(Args)]
pub struct GatewayArgs {
    /// Config file path (overrides auto-discovery).
    #[arg(short, long)]
    pub config: Option<String>,

    /// Enable intelligent routing (requires vector-memory feature).
    #[arg(long)]
    pub intelligent_routing: bool,
}

/// Resolve the cron JSONL storage path.
///
/// Tries `~/.clawft/cron.jsonl`, falls back to `~/.nanobot/cron.jsonl`.
#[cfg(feature = "services")]
fn resolve_cron_storage_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let clawft_path = home.join(".clawft").join("cron.jsonl");
        if clawft_path.parent().is_some_and(|p| p.exists()) {
            return clawft_path;
        }
        let nanobot_path = home.join(".nanobot").join("cron.jsonl");
        if nanobot_path.parent().is_some_and(|p| p.exists()) {
            return nanobot_path;
        }
        return clawft_path;
    }
    std::path::PathBuf::from("cron.jsonl")
}

/// Run the gateway command.
///
/// Loads configuration, bootstraps the [`AppContext`], registers all
/// enabled channels, starts them, then runs the agent loop and outbound
/// dispatch loop until Ctrl+C triggers graceful shutdown.
pub async fn run(args: GatewayArgs) -> anyhow::Result<()> {
    // If the channels feature is disabled, bail early with a helpful message.
    #[cfg(not(feature = "channels"))]
    {
        let _ = args;
        anyhow::bail!(
            "the gateway command requires the 'channels' feature. \
             Rebuild with: cargo build -p clawft-cli --features channels"
        );
    }

    #[cfg(feature = "channels")]
    {
        run_with_channels(args).await
    }
}

/// Inner implementation when the `channels` feature is enabled.
#[cfg(feature = "channels")]
async fn run_with_channels(args: GatewayArgs) -> anyhow::Result<()> {
    info!("starting weft gateway");

    let platform = Arc::new(NativePlatform::new());
    let config = load_config(&*platform, args.config.as_deref()).await?;

    // ── Bootstrap AppContext (bus, sessions, tools, pipeline) ────────
    let mut ctx = AppContext::new(config.clone(), platform.clone())
        .await
        .map_err(|e| anyhow::anyhow!("failed to bootstrap app context: {e}"))?;

    // Build security policies from config.
    let command_policy = super::agent::build_command_policy(&config.tools.command_policy);
    let url_policy = super::agent::build_url_policy(&config.tools.url_policy);

    // Register tools.
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    clawft_tools::register_all(
        ctx.tools_mut(),
        platform.clone(),
        workspace,
        command_policy,
        url_policy,
    );

    // Register MCP server tools.
    crate::mcp_tools::register_mcp_tools(&config, ctx.tools_mut()).await;

    // Register message tool (needs bus reference, cannot go in register_all).
    let bus_ref = ctx.bus().clone();
    ctx.tools_mut().register(Arc::new(
        clawft_tools::message_tool::MessageTool::new(bus_ref),
    ));

    info!(tools = ctx.tools().len(), "tool registry initialized");

    // Wire the live LLM-backed pipeline so real provider calls work.
    ctx.enable_live_llm();

    // Intelligent routing.
    if args.intelligent_routing {
        #[cfg(feature = "vector-memory")]
        {
            info!("intelligent routing enabled for gateway");
        }
        #[cfg(not(feature = "vector-memory"))]
        {
            anyhow::bail!(
                "intelligent routing requires the 'vector-memory' feature. \
                 Rebuild with: cargo build --features vector-memory"
            );
        }
    }

    // Clone the bus reference before consuming AppContext.
    let bus = ctx.bus().clone();

    // ── Channel setup ───────────────────────────────────────────────
    let host = make_channel_host(bus.clone());
    let plugin_host = Arc::new(PluginHost::new(host));

    let mut any_channel = false;

    // Telegram
    if config.channels.telegram.enabled && !config.channels.telegram.token.is_empty() {
        plugin_host
            .register_factory(Arc::new(TelegramChannelFactory))
            .await;
        let telegram_config = serde_json::to_value(&config.channels.telegram)?;
        plugin_host
            .init_channel("telegram", &telegram_config)
            .await
            .map_err(|e| anyhow::anyhow!("failed to init telegram channel: {e}"))?;
        info!("telegram channel initialized");
        any_channel = true;
    }

    // Slack
    if config.channels.slack.enabled && !config.channels.slack.bot_token.is_empty() {
        plugin_host
            .register_factory(Arc::new(SlackChannelFactory))
            .await;
        let slack_config = serde_json::to_value(&config.channels.slack)?;
        plugin_host
            .init_channel("slack", &slack_config)
            .await
            .map_err(|e| anyhow::anyhow!("failed to init slack channel: {e}"))?;
        info!("slack channel initialized");
        any_channel = true;
    }

    // Discord
    if config.channels.discord.enabled && !config.channels.discord.token.is_empty() {
        plugin_host
            .register_factory(Arc::new(DiscordChannelFactory))
            .await;
        let discord_config = serde_json::to_value(&config.channels.discord)?;
        plugin_host
            .init_channel("discord", &discord_config)
            .await
            .map_err(|e| anyhow::anyhow!("failed to init discord channel: {e}"))?;
        info!("discord channel initialized");
        any_channel = true;
    }

    if !any_channel {
        anyhow::bail!(
            "no channels are enabled in config. \
             Enable at least one channel (e.g., telegram, slack, discord) \
             and provide credentials."
        );
    }

    // ── Start all channels ──────────────────────────────────────────
    let start_results = plugin_host.start_all().await;
    for (name, result) in &start_results {
        match result {
            Ok(()) => info!(channel = %name, "channel started"),
            Err(e) => {
                error!(channel = %name, error = %e, "channel failed to start");
            }
        }
    }

    let started_count = start_results.iter().filter(|(_, r)| r.is_ok()).count();
    if started_count == 0 {
        anyhow::bail!("no channels started successfully");
    }

    // ── Cancellation token (shared by all background tasks) ─────────
    let cancel = CancellationToken::new();

    // ── Background services ──────────────────────────────────────────

    #[cfg(feature = "services")]
    let (cron_handle, heartbeat_handle) = {
        // CronService
        let inbound_tx = bus.inbound_sender();
        let cron_storage = resolve_cron_storage_path();
        let cron_handle = match CronService::new(cron_storage, inbound_tx.clone()).await {
            Ok(cron_service) => {
                let cron_cancel = cancel.clone();
                let svc = std::sync::Arc::new(cron_service);
                let svc_clone = svc.clone();
                info!("cron service initialized");
                Some(tokio::spawn(async move {
                    if let Err(e) = svc_clone.start(cron_cancel).await {
                        error!(error = %e, "cron service exited with error");
                    }
                }))
            }
            Err(e) => {
                warn!(error = %e, "failed to initialize cron service, skipping");
                None
            }
        };

        // HeartbeatService
        let heartbeat_handle = if config.gateway.heartbeat_interval_minutes > 0 {
            let svc = HeartbeatService::new(
                config.gateway.heartbeat_interval_minutes,
                config.gateway.heartbeat_prompt.clone(),
                inbound_tx,
            );
            let hb_cancel = cancel.clone();
            info!(
                interval_minutes = config.gateway.heartbeat_interval_minutes,
                "heartbeat service started"
            );
            Some(tokio::spawn(async move {
                if let Err(e) = svc.start(hb_cancel).await {
                    error!(error = %e, "heartbeat service exited with error");
                }
            }))
        } else {
            debug!("heartbeat service disabled (interval=0)");
            None
        };

        (cron_handle, heartbeat_handle)
    };

    #[cfg(not(feature = "services"))]
    let (cron_handle, heartbeat_handle): (
        Option<tokio::task::JoinHandle<()>>,
        Option<tokio::task::JoinHandle<()>>,
    ) = {
        debug!("services feature disabled, skipping cron and heartbeat");
        (None, None)
    };

    // ── Agent loop (inbound processing) ─────────────────────────────
    let agent = ctx.into_agent_loop();

    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            error!(error = %e, "agent loop exited with error");
        }
    });

    // ── Outbound dispatch loop ──────────────────────────────────────
    let cancel_for_dispatch = cancel.clone();
    let bus_for_dispatch = bus.clone();
    let plugin_host_for_dispatch = plugin_host.clone();
    let md_dispatcher = MarkdownDispatcher::new();

    let dispatch_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;

                _ = cancel_for_dispatch.cancelled() => {
                    info!("outbound dispatch loop shutting down");
                    break;
                }

                msg = bus_for_dispatch.consume_outbound() => {
                    match msg {
                        Some(mut outbound) => {
                            debug!(
                                channel = %outbound.channel,
                                chat_id = %outbound.chat_id,
                                "dispatching outbound message"
                            );
                            // Convert markdown to channel-specific format.
                            outbound.content = md_dispatcher.convert(
                                &outbound.channel,
                                &outbound.content,
                            );
                            if let Err(e) = plugin_host_for_dispatch
                                .send_to_channel(&outbound)
                                .await
                            {
                                error!(
                                    channel = %outbound.channel,
                                    chat_id = %outbound.chat_id,
                                    error = %e,
                                    "outbound dispatch failed"
                                );
                            }
                        }
                        None => {
                            info!("outbound bus closed, dispatch loop exiting");
                            break;
                        }
                    }
                }
            }
        }
    });

    info!(
        channels = started_count,
        "gateway running -- press Ctrl+C to stop"
    );

    // ── Wait for shutdown signal ────────────────────────────────────
    tokio::signal::ctrl_c().await?;
    info!("received shutdown signal");

    // 1. Cancel the dispatch loop.
    cancel.cancel();

    // 2. Stop all channels (cancels their tasks).
    let stop_results = plugin_host.stop_all().await;
    for (name, result) in &stop_results {
        match result {
            Ok(()) => info!(channel = %name, "channel stopped"),
            Err(e) => {
                warn!(channel = %name, error = %e, "channel stop error");
            }
        }
    }

    // 3. Drop the bus so the agent loop's inbound channel closes,
    //    causing it to exit its consume loop.
    drop(bus);

    // 4. Await background services.
    if let Some(h) = cron_handle {
        let _ = h.await;
    }
    if let Some(h) = heartbeat_handle {
        let _ = h.await;
    }

    // 5. Await background tasks.
    let _ = dispatch_handle.await;
    let _ = agent_handle.await;

    info!("gateway shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_args_defaults() {
        let args = GatewayArgs {
            config: None,
            intelligent_routing: false,
        };
        assert!(args.config.is_none());
    }

    #[test]
    fn gateway_args_with_config() {
        let args = GatewayArgs {
            config: Some("/tmp/gw-config.json".into()),
            intelligent_routing: false,
        };
        assert_eq!(args.config.as_deref(), Some("/tmp/gw-config.json"));
    }

    #[cfg(feature = "services")]
    #[test]
    fn resolve_cron_storage_path_returns_valid() {
        let path = resolve_cron_storage_path();
        assert!(path.to_string_lossy().contains("cron.jsonl"));
    }
}
