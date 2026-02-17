//! Markdown conversion for channel-specific output formats.
//!
//! Each channel has its own formatting requirements:
//!
//! - **Telegram** uses a restricted subset of HTML.
//! - **Slack** uses its own `mrkdwn` format.
//! - **Discord** uses standard Markdown (mostly passthrough).
//!
//! The [`MarkdownConverter`] trait provides a uniform interface for
//! converting CommonMark input into the target format.

pub mod discord;
pub mod dispatch;
pub mod slack;
pub mod telegram;

/// Trait for converting CommonMark input into a channel-specific format.
///
/// Implementations are used by the gateway when sending outbound messages
/// to channels via the [`dispatch::MarkdownDispatcher`].
pub trait MarkdownConverter: Send + Sync {
    /// Convert the given CommonMark `markdown` string into the target format.
    fn convert(&self, markdown: &str) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_is_object_safe() {
        // Verify `MarkdownConverter` can be used as a trait object.
        fn _accepts(_: &dyn MarkdownConverter) {}
    }
}
