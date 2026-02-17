//! Channel-aware markdown dispatch.
//!
//! Routes outbound message content through the appropriate
//! [`MarkdownConverter`] based on the target channel name.

use std::collections::HashMap;

use super::MarkdownConverter;
use super::discord::DiscordMarkdownConverter;
use super::slack::SlackMarkdownConverter;
use super::telegram::TelegramMarkdownConverter;

/// Dispatches markdown conversion based on channel name.
///
/// Maps channel names (e.g. "telegram", "slack", "discord") to their
/// respective [`MarkdownConverter`] implementations. Unknown channels
/// get the content passed through unchanged.
pub struct MarkdownDispatcher {
    converters: HashMap<String, Box<dyn MarkdownConverter>>,
}

impl MarkdownDispatcher {
    /// Create a dispatcher with all built-in converters registered.
    pub fn new() -> Self {
        let mut converters: HashMap<String, Box<dyn MarkdownConverter>> = HashMap::new();
        converters.insert("telegram".into(), Box::new(TelegramMarkdownConverter));
        converters.insert("slack".into(), Box::new(SlackMarkdownConverter));
        converters.insert("discord".into(), Box::new(DiscordMarkdownConverter));
        Self { converters }
    }

    /// Convert markdown content for the given channel.
    ///
    /// If the channel has a registered converter, applies it.
    /// Otherwise returns the content unchanged.
    pub fn convert(&self, channel: &str, content: &str) -> String {
        match self.converters.get(channel) {
            Some(converter) => converter.convert(content),
            None => content.to_string(),
        }
    }

    /// Register a custom converter for a channel name.
    #[allow(dead_code)]
    pub fn register(&mut self, channel: impl Into<String>, converter: Box<dyn MarkdownConverter>) {
        self.converters.insert(channel.into(), converter);
    }

    /// List all registered channel names.
    #[allow(dead_code)]
    pub fn channels(&self) -> Vec<String> {
        let mut names: Vec<String> = self.converters.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for MarkdownDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_three_converters() {
        let dispatcher = MarkdownDispatcher::new();
        assert_eq!(dispatcher.channels().len(), 3);
        assert!(dispatcher.channels().contains(&"telegram".to_string()));
        assert!(dispatcher.channels().contains(&"slack".to_string()));
        assert!(dispatcher.channels().contains(&"discord".to_string()));
    }

    #[test]
    fn convert_telegram() {
        let dispatcher = MarkdownDispatcher::new();
        let result = dispatcher.convert("telegram", "**bold**");
        assert!(result.contains("<b>"));
    }

    #[test]
    fn convert_slack() {
        let dispatcher = MarkdownDispatcher::new();
        let result = dispatcher.convert("slack", "**bold**");
        // Slack bold wraps with single asterisks: *bold*
        assert!(result.contains("*bold*"));
    }

    #[test]
    fn convert_discord() {
        let dispatcher = MarkdownDispatcher::new();
        let result = dispatcher.convert("discord", "**bold**");
        // Discord is passthrough
        assert!(result.contains("**bold**"));
    }

    #[test]
    fn convert_unknown_channel_passthrough() {
        let dispatcher = MarkdownDispatcher::new();
        let input = "# Hello **world**";
        assert_eq!(dispatcher.convert("irc", input), input);
    }

    #[test]
    fn convert_cli_passthrough() {
        let dispatcher = MarkdownDispatcher::new();
        let input = "Some text";
        assert_eq!(dispatcher.convert("cli", input), input);
    }

    #[test]
    fn register_custom_converter() {
        struct UpperConverter;
        impl MarkdownConverter for UpperConverter {
            fn convert(&self, markdown: &str) -> String {
                markdown.to_uppercase()
            }
        }

        let mut dispatcher = MarkdownDispatcher::new();
        dispatcher.register("custom", Box::new(UpperConverter));
        assert_eq!(dispatcher.convert("custom", "hello"), "HELLO");
    }

    #[test]
    fn default_is_same_as_new() {
        let d1 = MarkdownDispatcher::new();
        let d2 = MarkdownDispatcher::default();
        assert_eq!(d1.channels(), d2.channels());
    }

    #[test]
    fn empty_content_passthrough() {
        let dispatcher = MarkdownDispatcher::new();
        assert_eq!(dispatcher.convert("telegram", ""), "");
        assert_eq!(dispatcher.convert("unknown", ""), "");
    }
}
