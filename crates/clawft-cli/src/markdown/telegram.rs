//! Markdown to Telegram HTML converter.
//!
//! Telegram's Bot API supports a limited subset of HTML:
//! `<b>`, `<i>`, `<code>`, `<pre>`, `<a href="...">`. All other
//! text content must be HTML-escaped to prevent XSS via bot responses.
//!
//! See <https://core.telegram.org/bots/api#html-style>.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::MarkdownConverter;

/// Converts CommonMark to Telegram-compatible HTML.
pub struct TelegramMarkdownConverter;

impl MarkdownConverter for TelegramMarkdownConverter {
    fn convert(&self, markdown: &str) -> String {
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);
        let mut output = String::with_capacity(markdown.len());

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Strong => output.push_str("<b>"),
                    Tag::Emphasis => output.push_str("<i>"),
                    Tag::CodeBlock(_) => output.push_str("<pre>"),
                    Tag::Link { dest_url, .. } => {
                        output.push_str("<a href=\"");
                        output.push_str(&escape_html(&dest_url));
                        output.push_str("\">");
                    }
                    Tag::Paragraph => {}
                    Tag::Heading { .. } => output.push_str("<b>"),
                    Tag::List(_) => {}
                    Tag::Item => output.push_str("- "),
                    Tag::BlockQuote(_) => output.push_str("&gt; "),
                    Tag::Strikethrough => output.push_str("<s>"),
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
                    TagEnd::Strong => output.push_str("</b>"),
                    TagEnd::Emphasis => output.push_str("</i>"),
                    TagEnd::CodeBlock => output.push_str("</pre>"),
                    TagEnd::Link => output.push_str("</a>"),
                    TagEnd::Paragraph => output.push('\n'),
                    TagEnd::Heading(_) => {
                        output.push_str("</b>");
                        output.push('\n');
                    }
                    TagEnd::Item => output.push('\n'),
                    TagEnd::Strikethrough => output.push_str("</s>"),
                    _ => {}
                },
                Event::Text(text) => {
                    output.push_str(&escape_html(&text));
                }
                Event::Code(code) => {
                    output.push_str("<code>");
                    output.push_str(&escape_html(&code));
                    output.push_str("</code>");
                }
                Event::SoftBreak | Event::HardBreak => {
                    output.push('\n');
                }
                // Raw HTML in input must be escaped (XSS prevention).
                Event::Html(html) | Event::InlineHtml(html) => {
                    output.push_str(&escape_html(&html));
                }
                _ => {}
            }
        }

        output.trim_end().to_owned()
    }
}

/// Escape HTML special characters to prevent injection.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(md: &str) -> String {
        TelegramMarkdownConverter.convert(md)
    }

    #[test]
    fn test_bold() {
        assert_eq!(convert("**bold**"), "<b>bold</b>");
    }

    #[test]
    fn test_italic() {
        assert_eq!(convert("*italic*"), "<i>italic</i>");
    }

    #[test]
    fn test_code_inline() {
        assert_eq!(convert("`code`"), "<code>code</code>");
    }

    #[test]
    fn test_code_block() {
        let result = convert("```\nhello world\n```");
        assert!(result.contains("<pre>"));
        assert!(result.contains("</pre>"));
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_link() {
        assert_eq!(
            convert("[link](https://example.com)"),
            "<a href=\"https://example.com\">link</a>"
        );
    }

    #[test]
    fn test_html_escape() {
        let result = convert("<script>alert('xss')</script>");
        assert!(result.contains("&lt;script&gt;"));
        assert!(result.contains("&lt;/script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_combined() {
        let result = convert("**bold** and *italic* and `code`");
        assert!(result.contains("<b>bold</b>"));
        assert!(result.contains("<i>italic</i>"));
        assert!(result.contains("<code>code</code>"));
    }

    #[test]
    fn test_escape_html_ampersand() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_html_angle_brackets() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
    }

    #[test]
    fn test_escape_html_quotes() {
        assert_eq!(escape_html("a \"b\" c"), "a &quot;b&quot; c");
    }

    #[test]
    fn test_heading() {
        let result = convert("# Title");
        assert!(result.contains("<b>"));
        assert!(result.contains("Title"));
        assert!(result.contains("</b>"));
    }

    #[test]
    fn test_strikethrough() {
        let result = convert("~~deleted~~");
        assert!(result.contains("<s>"));
        assert!(result.contains("deleted"));
        assert!(result.contains("</s>"));
    }

    #[test]
    fn test_list_items() {
        let result = convert("- item one\n- item two");
        assert!(result.contains("- item one"));
        assert!(result.contains("- item two"));
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
    fn test_nested_bold_italic() {
        let result = convert("***bold italic***");
        // pulldown-cmark parses this as nested emphasis+strong
        assert!(result.contains("<b>"));
        assert!(result.contains("<i>"));
    }

    #[test]
    fn test_link_with_special_chars() {
        let result = convert("[test](https://example.com?a=1&b=2)");
        assert!(result.contains("&amp;"));
        assert!(result.contains("test"));
    }
}
