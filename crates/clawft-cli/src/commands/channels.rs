//! `weft channels` -- inspect channel configuration status.
//!
//! Reads the configuration and displays a table summarizing which
//! channels are enabled, and whether credentials are present.
//!
//! # Example
//!
//! ```text
//! weft channels status
//! ```

use comfy_table::{Table, presets::UTF8_FULL};

use clawft_types::config::Config;

/// Display a table of channel status from the given configuration.
pub fn channels_status(config: &Config) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["CHANNEL", "ENABLED", "CONFIG STATUS"]);

    // Known channels.
    add_row(
        &mut table,
        "telegram",
        config.channels.telegram.enabled,
        !config.channels.telegram.token.is_empty(),
    );
    add_row(
        &mut table,
        "slack",
        config.channels.slack.enabled,
        !config.channels.slack.bot_token.is_empty(),
    );
    add_row(
        &mut table,
        "discord",
        config.channels.discord.enabled,
        !config.channels.discord.token.is_empty(),
    );
    add_row(
        &mut table,
        "whatsapp",
        config.channels.whatsapp.enabled,
        !config.channels.whatsapp.bridge_token.is_empty(),
    );
    add_row(
        &mut table,
        "email",
        config.channels.email.enabled,
        !config.channels.email.imap_host.is_empty(),
    );
    add_row(
        &mut table,
        "feishu",
        config.channels.feishu.enabled,
        !config.channels.feishu.app_id.is_empty(),
    );
    add_row(
        &mut table,
        "dingtalk",
        config.channels.dingtalk.enabled,
        !config.channels.dingtalk.client_id.is_empty(),
    );
    add_row(
        &mut table,
        "mochat",
        config.channels.mochat.enabled,
        !config.channels.mochat.claw_token.is_empty(),
    );
    add_row(
        &mut table,
        "qq",
        config.channels.qq.enabled,
        !config.channels.qq.app_id.is_empty(),
    );

    // Unknown / extra channels from config.
    for name in config.channels.extra.keys() {
        table.add_row([name.as_str(), "unknown", "present in config"]);
    }

    println!("{table}");
}

/// Add a row for a known channel.
fn add_row(table: &mut Table, name: &str, enabled: bool, has_credentials: bool) {
    let enabled_str = if enabled { "yes" } else { "no" };
    let config_status = match (enabled, has_credentials) {
        (true, true) => "configured",
        (true, false) => "MISSING credentials",
        (false, true) => "credentials present",
        (false, false) => "not configured",
    };
    table.add_row([name, enabled_str, config_status]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::config::{Config, DiscordConfig, TelegramConfig};

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn channels_status_with_defaults_does_not_panic() {
        // Smoke test: no panic with all-defaults config.
        channels_status(&default_config());
    }

    #[test]
    fn channels_status_with_telegram_enabled() {
        let mut cfg = default_config();
        cfg.channels.telegram = TelegramConfig {
            enabled: true,
            token: "test-token-123".into(),
            token_env: None,
            allow_from: vec![],
            proxy: None,
        };
        // Should not panic; output goes to stdout.
        channels_status(&cfg);
    }

    #[test]
    fn channels_status_with_all_disabled() {
        let cfg = default_config();
        // All channels are disabled by default.
        assert!(!cfg.channels.telegram.enabled);
        assert!(!cfg.channels.slack.enabled);
        assert!(!cfg.channels.discord.enabled);
        channels_status(&cfg);
    }

    #[test]
    fn channels_status_with_extra_channels() {
        let mut cfg = default_config();
        cfg.channels.extra.insert(
            "my_custom_channel".into(),
            serde_json::json!({"url": "wss://custom.io"}),
        );
        // Should include the extra channel row.
        channels_status(&cfg);
    }

    #[test]
    fn add_row_all_combinations() {
        let mut table = Table::new();
        table.set_header(["CHANNEL", "ENABLED", "CONFIG STATUS"]);
        add_row(&mut table, "test1", true, true);
        add_row(&mut table, "test2", true, false);
        add_row(&mut table, "test3", false, true);
        add_row(&mut table, "test4", false, false);
        let rendered = table.to_string();
        assert!(rendered.contains("test1"));
        assert!(rendered.contains("configured"));
        assert!(rendered.contains("MISSING credentials"));
        assert!(rendered.contains("credentials present"));
        assert!(rendered.contains("not configured"));
    }

    #[test]
    fn add_row_enabled_column_values() {
        let mut table = Table::new();
        table.set_header(["CHANNEL", "ENABLED", "CONFIG STATUS"]);
        add_row(&mut table, "a", true, true);
        add_row(&mut table, "b", false, false);
        let rendered = table.to_string();
        assert!(rendered.contains("yes"));
        assert!(rendered.contains("no"));
    }

    #[test]
    fn channels_status_with_slack_credentials() {
        let mut cfg = default_config();
        cfg.channels.slack.enabled = true;
        cfg.channels.slack.bot_token = "xoxb-test".into();
        channels_status(&cfg);
    }

    #[test]
    fn channels_status_with_discord_credentials() {
        let mut cfg = default_config();
        cfg.channels.discord = DiscordConfig {
            enabled: true,
            token: "discord-token".into(),
            ..Default::default()
        };
        channels_status(&cfg);
    }
}
