//! Markdown to Discord format converter.
//!
//! Discord uses standard Markdown for message formatting, so the
//! conversion is mostly a passthrough. This converter ensures code
//! blocks and spoilers are preserved correctly.
//!
//! See <https://support.discord.com/hc/en-us/articles/210298617>.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::MarkdownConverter;

/// Converts CommonMark to Discord-compatible Markdown (mostly passthrough).
pub struct DiscordMarkdownConverter;

impl MarkdownConverter for DiscordMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);
        let mut output = String::with_capacity(markdown.len());
        // Stack for link destination URLs so we can emit `](url)` at End(Link).
        let mut link_stack: Vec<String> = Vec::new();

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Strong => output.push_str("**"),
                    Tag::Emphasis => output.push('*'),
                    Tag::CodeBlock(_) => output.push_str("```\n"),
                    Tag::Link { dest_url, .. } => {
                        output.push('[');
                        link_stack.push(dest_url.to_string());
                    }
                    Tag::Paragraph => {}
                    Tag::Heading { level, .. } => {
                        for _ in 0..level_to_u8(level) {
                            output.push('#');
                        }
                        output.push(' ');
                    }
                    Tag::List(_) => {}
                    Tag::Item => output.push_str("- "),
                    Tag::BlockQuote(_) => output.push_str("> "),
                    Tag::Strikethrough => output.push_str("~~"),
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
                    TagEnd::Strong => output.push_str("**"),
                    TagEnd::Emphasis => output.push('*'),
                    TagEnd::CodeBlock => output.push_str("```\n"),
                    TagEnd::Link => {
                        if let Some(url) = link_stack.pop() {
                            output.push_str("](");
                            output.push_str(&url);
                            output.push(')');
                        } else {
                            output.push(']');
                        }
                    }
                    TagEnd::Paragraph => output.push('\n'),
                    TagEnd::Heading(_) => output.push('\n'),
                    TagEnd::Item => output.push('\n'),
                    TagEnd::Strikethrough => output.push_str("~~"),
                    _ => {}
                },
                Event::Text(text) => {
                    output.push_str(&text);
                }
                Event::Code(code) => {
                    output.push('`');
                    output.push_str(&code);
                    output.push('`');
                }
                Event::SoftBreak | Event::HardBreak => {
                    output.push('\n');
                }
                _ => {}
            }
        }

        output.trim_end().to_owned()
    }
}

/// Convert a pulldown-cmark `HeadingLevel` to a `u8`.
fn level_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(md: &str) -> String {
        DiscordMarkdownConverter.convert(md)
    }

    #[test]
    fn test_bold() {
        assert_eq!(convert("**bold**"), "**bold**");
    }

    #[test]
    fn test_italic() {
        assert_eq!(convert("*italic*"), "*italic*");
    }

    #[test]
    fn test_code_inline() {
        assert_eq!(convert("`code`"), "`code`");
    }

    #[test]
    fn test_code_block() {
        let result = convert("```\nhello world\n```");
        assert!(result.contains("```"));
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_link() {
        let result = convert("[text](https://example.com)");
        assert_eq!(result, "[text](https://example.com)");
    }

    #[test]
    fn test_strikethrough() {
        assert_eq!(convert("~~deleted~~"), "~~deleted~~");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(convert(""), "");
    }

    #[test]
    fn test_plain_text() {
        assert_eq!(convert("hello world"), "hello world");
    }

    #[test]
    fn test_heading() {
        let result = convert("# Title");
        assert!(result.contains("# Title"));
    }

    #[test]
    fn test_heading_h2() {
        let result = convert("## Subtitle");
        assert!(result.contains("## Subtitle"));
    }

    #[test]
    fn test_list_items() {
        let result = convert("- item one\n- item two");
        assert!(result.contains("- item one"));
        assert!(result.contains("- item two"));
    }

    #[test]
    fn test_combined() {
        let result = convert("**bold** and *italic* and `code`");
        assert!(result.contains("**bold**"));
        assert!(result.contains("*italic*"));
        assert!(result.contains("`code`"));
    }

    #[test]
    fn test_blockquote() {
        let result = convert("> quoted text");
        assert!(result.contains("> "));
        assert!(result.contains("quoted text"));
    }

    #[test]
    fn test_level_to_u8() {
        assert_eq!(level_to_u8(pulldown_cmark::HeadingLevel::H1), 1);
        assert_eq!(level_to_u8(pulldown_cmark::HeadingLevel::H6), 6);
    }
}
