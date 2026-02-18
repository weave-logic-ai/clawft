//! `weft status` -- show configuration status and diagnostics.
//!
//! Discovers the active configuration file, parses it, and displays
//! a summary of the current settings. With `--detailed`, also shows
//! channel and tool configuration.
//!
//! # Example
//!
//! ```text
//! weft status
//! weft status --detailed
//! ```

use clap::Args;

use clawft_platform::NativePlatform;

use super::{discover_config_path, load_config};

/// Arguments for the `weft status` subcommand.
#[derive(Args)]
pub struct StatusArgs {
    /// Show detailed configuration (channels, tools, providers).
    #[arg(long)]
    pub detailed: bool,
}

/// Run the status command.
///
/// Discovers the config file, loads it, and prints a summary.
pub async fn run(args: StatusArgs) -> anyhow::Result<()> {
    let platform = NativePlatform::new();

    println!("weft status");
    println!("===========");
    println!();

    // Discover config path.
    let config_path = discover_config_path(&platform);

    match config_path {
        Some(ref path) => {
            println!("Config: {}", path.display());
        }
        None => {
            println!("Config: not found");
            println!("  Searched: ~/.clawft/config.json, ~/.nanobot/config.json");
            println!("  Set CLAWFT_CONFIG env var to override");
            println!();
            println!("Using defaults:");
        }
    }

    // Load and display config (defaults if no file found).
    let config = load_config(&platform, None).await?;

    println!();
    println!("Agent defaults:");
    println!("  Model:              {}", config.agents.defaults.model);
    println!("  Workspace:          {}", config.agents.defaults.workspace);
    println!(
        "  Max tokens:         {}",
        config.agents.defaults.max_tokens
    );
    println!(
        "  Temperature:        {}",
        config.agents.defaults.temperature
    );
    println!(
        "  Max tool iterations: {}",
        config.agents.defaults.max_tool_iterations
    );
    println!(
        "  Memory window:      {}",
        config.agents.defaults.memory_window
    );

    if args.detailed {
        println!();
        println!("Gateway:");
        println!("  Listen: {}:{}", config.gateway.host, config.gateway.port);

        println!();
        println!("Channels:");
        print_channel_status(
            "  telegram",
            config.channels.telegram.enabled,
            !config.channels.telegram.token.is_empty(),
        );
        print_channel_status(
            "  slack",
            config.channels.slack.enabled,
            !config.channels.slack.bot_token.is_empty(),
        );
        print_channel_status(
            "  discord",
            config.channels.discord.enabled,
            !config.channels.discord.token.is_empty(),
        );
        print_channel_status(
            "  whatsapp",
            config.channels.whatsapp.enabled,
            !config.channels.whatsapp.bridge_token.is_empty(),
        );
        print_channel_status(
            "  email",
            config.channels.email.enabled,
            !config.channels.email.imap_host.is_empty(),
        );
        print_channel_status(
            "  feishu",
            config.channels.feishu.enabled,
            !config.channels.feishu.app_id.is_empty(),
        );
        print_channel_status(
            "  dingtalk",
            config.channels.dingtalk.enabled,
            !config.channels.dingtalk.client_id.is_empty(),
        );

        println!();
        println!("Providers:");
        print_provider("  anthropic", &config.providers.anthropic.api_key);
        print_provider("  openai", &config.providers.openai.api_key);
        print_provider("  openrouter", &config.providers.openrouter.api_key);
        print_provider("  deepseek", &config.providers.deepseek.api_key);
        print_provider("  groq", &config.providers.groq.api_key);
        print_provider("  gemini", &config.providers.gemini.api_key);
        print_provider("  custom", &config.providers.custom.api_key);

        println!();
        println!("Tools:");
        println!(
            "  Restrict to workspace: {}",
            config.tools.restrict_to_workspace
        );
        println!(
            "  Exec timeout:         {}s",
            config.tools.exec_tool.timeout
        );
        println!(
            "  Search max results:   {}",
            config.tools.web.search.max_results
        );

        if !config.tools.mcp_servers.is_empty() {
            println!();
            println!("  MCP servers:");
            for (name, mcp) in &config.tools.mcp_servers {
                if !mcp.command.is_empty() {
                    println!("    {name}: {} {}", mcp.command, mcp.args.join(" "));
                } else if !mcp.url.is_empty() {
                    println!("    {name}: {}", mcp.url);
                } else {
                    println!("    {name}: (unconfigured)");
                }
            }
        }
    }

    println!();
    Ok(())
}

/// Print channel status: name, enabled/disabled, configured/unconfigured.
fn print_channel_status(label: &str, enabled: bool, has_credentials: bool) {
    let status = match (enabled, has_credentials) {
        (true, true) => "enabled, configured",
        (true, false) => "enabled, NOT configured (missing credentials)",
        (false, true) => "disabled (credentials present)",
        (false, false) => "disabled",
    };
    println!("{label}: {status}");
}

/// Print provider status, masking the API key.
fn print_provider(label: &str, api_key: &str) {
    if api_key.is_empty() {
        println!("{label}: not configured");
    } else {
        let masked = mask_key(api_key);
        println!("{label}: {masked}");
    }
}

/// Mask an API key, showing only the first 4 and last 4 characters.
fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    let middle_len = key.len() - 8;
    format!("{prefix}{}...{suffix}", "*".repeat(middle_len.min(8)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_args_defaults() {
        let args = StatusArgs { detailed: false };
        assert!(!args.detailed);
    }

    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("abc"), "***");
        assert_eq!(mask_key("abcdefgh"), "********");
    }

    #[test]
    fn mask_key_long() {
        let masked = mask_key("sk-ant-test-key-12345");
        assert!(masked.starts_with("sk-a"));
        assert!(masked.ends_with("2345"));
        assert!(masked.contains("*"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn mask_key_medium() {
        let masked = mask_key("123456789");
        assert_eq!(masked, "1234*...6789");
    }

    #[test]
    fn print_channel_status_does_not_panic() {
        // Smoke test for all combinations.
        print_channel_status("test", true, true);
        print_channel_status("test", true, false);
        print_channel_status("test", false, true);
        print_channel_status("test", false, false);
    }

    #[test]
    fn print_provider_does_not_panic() {
        print_provider("test", "");
        print_provider("test", "sk-test-key-12345678");
    }
}
