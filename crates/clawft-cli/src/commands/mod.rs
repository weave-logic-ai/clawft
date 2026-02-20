//! CLI command implementations for `weft`.
//!
//! Each subcommand is implemented in its own module:
//!
//! - [`agent`] -- Interactive agent session or single-message mode.
//! - [`gateway`] -- Channel gateway (Telegram, Slack, etc.) + agent loop.
//! - [`help_cmd`] -- Topic-aware help (`weft help [topic]`).
//! - [`status`] -- Configuration diagnostics.

pub mod agent;
pub mod agents_cmd;
pub mod channels;
pub mod config_cmd;
pub mod cron;
pub mod gateway;
pub mod help_cmd;
#[cfg(feature = "services")]
pub mod mcp_server;
pub mod memory_cmd;
pub mod onboard;
pub mod security_cmd;
pub mod sessions;
pub mod skills_cmd;
pub mod status;
pub mod workspace_cmd;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clawft_core::tools::registry::ToolRegistry;
use clawft_platform::Platform;
use clawft_types::config::Config;

/// Load configuration from the given path override or via auto-discovery.
///
/// If `config_override` is provided, loads from that path. Otherwise,
/// uses the platform's config discovery chain:
/// 1. `CLAWFT_CONFIG` env var
/// 2. `~/.clawft/config.json`
/// 3. `~/.nanobot/config.json`
///
/// Returns a default `Config` if no config file is found.
pub async fn load_config<P: Platform>(
    platform: &P,
    config_override: Option<&str>,
) -> anyhow::Result<Config> {
    let raw = if let Some(path_str) = config_override {
        let path = Path::new(path_str);
        if !platform.fs().exists(path).await {
            anyhow::bail!("config file not found: {path_str}");
        }
        let contents = platform
            .fs()
            .read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("failed to read config: {e}"))?;
        let value: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("failed to parse config: {e}"))?;
        clawft_platform::config_loader::normalize_keys(value)
    } else {
        clawft_platform::config_loader::load_config_raw(platform.fs(), platform.env())
            .await
            .map_err(|e| anyhow::anyhow!("failed to load config: {e}"))?
    };

    let config: Config = serde_json::from_value(raw)?;
    Ok(config)
}

/// Expand `~/` prefixes in workspace paths to the user's home directory.
pub fn expand_workspace(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

/// Discover the config file path (for display in `weft status`).
pub fn discover_config_path<P: Platform>(platform: &P) -> Option<PathBuf> {
    let home = platform.fs().home_dir();
    clawft_platform::config_loader::discover_config_path(platform.env(), home)
}

/// Register the core set of tools into a [`ToolRegistry`].
///
/// This is the shared tool setup used by `weft agent`, `weft gateway`, and
/// `weft mcp-server`. It:
///
/// 1. Builds security policies (command + URL) from config.
/// 2. Registers all built-in tools via [`clawft_tools::register_all`].
/// 3. Registers MCP server tools (proxied from configured MCP servers).
/// 4. Registers the delegation tool (feature-gated).
///
/// Callers that need additional tools (e.g. `MessageTool` with a bus reference)
/// should register them separately after calling this function.
pub async fn register_core_tools<P: Platform + 'static>(
    registry: &mut ToolRegistry,
    config: &Config,
    platform: Arc<P>,
) {
    let command_policy = agent::build_command_policy(&config.tools.command_policy);
    let url_policy = agent::build_url_policy(&config.tools.url_policy);
    let workspace = expand_workspace(&config.agents.defaults.workspace);
    let web_search_config = agent::build_web_search_config(&config.tools);

    clawft_tools::register_all(
        registry,
        platform,
        workspace,
        command_policy,
        url_policy,
        web_search_config,
    );

    crate::mcp_tools::register_mcp_tools(config, registry).await;

    // Pass the Anthropic provider API key from config as a fallback for delegation.
    let anthropic_key = config.providers.anthropic.api_key.expose();
    let config_api_key = if anthropic_key.is_empty() {
        None
    } else {
        Some(anthropic_key)
    };
    crate::mcp_tools::register_delegation(&config.delegation, registry, config_api_key);
}

/// Build an `Arc<ChannelHost>` implementation that bridges the channel
/// system to a `MessageBus` inbound sender.
///
/// This is the glue between `clawft-channels::PluginHost` (which expects
/// an `Arc<dyn ChannelHost>`) and `clawft-core::bus::MessageBus`.
#[cfg(feature = "channels")]
pub fn make_channel_host(
    bus: Arc<clawft_core::bus::MessageBus>,
) -> Arc<dyn clawft_channels::ChannelHost> {
    Arc::new(BusChannelHost { bus })
}

/// A [`ChannelHost`] implementation backed by a [`MessageBus`].
///
/// Routes inbound messages from channel plugins into the message bus
/// for consumption by the agent loop.
#[cfg(feature = "channels")]
struct BusChannelHost {
    bus: Arc<clawft_core::bus::MessageBus>,
}

#[cfg(feature = "channels")]
#[async_trait::async_trait]
impl clawft_channels::ChannelHost for BusChannelHost {
    async fn deliver_inbound(
        &self,
        msg: clawft_types::event::InboundMessage,
    ) -> Result<(), clawft_types::error::ChannelError> {
        self.bus
            .publish_inbound(msg)
            .map_err(|e| clawft_types::error::ChannelError::Other(e.to_string()))
    }

    async fn register_command(
        &self,
        _cmd: clawft_channels::Command,
    ) -> Result<(), clawft_types::error::ChannelError> {
        // Command registration is a no-op for now; the agent does not
        // expose channel commands yet.
        Ok(())
    }

    async fn publish_inbound(
        &self,
        channel: &str,
        sender_id: &str,
        chat_id: &str,
        content: &str,
        media: Vec<String>,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), clawft_types::error::ChannelError> {
        let msg = clawft_types::event::InboundMessage {
            channel: channel.to_owned(),
            sender_id: sender_id.to_owned(),
            chat_id: chat_id.to_owned(),
            content: content.to_owned(),
            timestamp: chrono::Utc::now(),
            media,
            metadata,
        };
        self.deliver_inbound(msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_workspace_tilde() {
        let expanded = expand_workspace("~/.clawft/workspace");
        assert!(!expanded.to_string_lossy().starts_with('~'));
        assert!(expanded.to_string_lossy().contains(".clawft"));
    }

    #[test]
    fn expand_workspace_absolute() {
        let expanded = expand_workspace("/opt/workspace");
        assert_eq!(expanded, PathBuf::from("/opt/workspace"));
    }

    #[test]
    fn expand_workspace_relative() {
        let expanded = expand_workspace("workspace");
        assert_eq!(expanded, PathBuf::from("workspace"));
    }
}
