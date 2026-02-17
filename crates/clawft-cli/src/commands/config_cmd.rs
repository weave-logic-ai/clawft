//! `weft config` -- display resolved configuration.
//!
//! Shows the full resolved configuration as formatted JSON, or a specific
//! section by name.
//!
//! # Examples
//!
//! ```text
//! weft config show
//! weft config section agents
//! weft config section gateway
//! ```

use clawft_types::config::Config;

/// Display the resolved configuration as formatted JSON.
pub fn config_show(config: &Config) {
    match serde_json::to_string_pretty(config) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("error: failed to serialize config: {e}"),
    }
}

/// Display specific configuration sections.
pub fn config_section(config: &Config, section: &str) {
    let value = serde_json::to_value(config).unwrap_or_default();
    match value.get(section) {
        Some(v) => match serde_json::to_string_pretty(v) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("error: failed to serialize section: {e}"),
        },
        None => {
            let available: Vec<&str> = value
                .as_object()
                .map(|m| m.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            eprintln!("error: unknown section '{section}'");
            eprintln!("available sections: {}", available.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_show_does_not_panic() {
        let config = Config::default();
        config_show(&config); // should not panic
    }

    #[test]
    fn config_section_valid() {
        let config = Config::default();
        config_section(&config, "agents"); // should not panic
    }

    #[test]
    fn config_section_invalid() {
        let config = Config::default();
        config_section(&config, "nonexistent"); // should not panic
    }
}
