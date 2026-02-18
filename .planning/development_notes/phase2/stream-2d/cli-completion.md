# Stream 2D: CLI Completion -- Development Notes

**Agent**: cli-engineer (coder)
**Date**: 2026-02-17
**Phase**: 2 / Stream 2D
**Crate**: `clawft-cli`
**Modules**: `commands/channels`, `commands/cron`, `markdown/`

## Summary

Completed the CLI binary with channel status reporting, cron management subcommands, and markdown conversion for Telegram/Slack/Discord output. All new subcommands are wired into the clap-derived CLI parser in `main.rs`.

## Files Written

| File | Lines | Purpose |
|------|-------|---------|
| `commands/channels.rs` | 147 | `weft channels status` -- tabular channel status display |
| `commands/cron.rs` | 477 | `weft cron {list,add,remove,enable,disable,run}` with JSONL storage |
| `markdown/mod.rs` | 36 | `MarkdownConverter` trait definition |
| `markdown/telegram.rs` | 205 | `TelegramMarkdownConverter` -- Markdown to Telegram HTML |
| `markdown/slack.rs` | 177 | `SlackMarkdownConverter` -- Markdown to Slack mrkdwn |
| `markdown/discord.rs` | 187 | `DiscordMarkdownConverter` -- passthrough with code block normalization |

**Total new lines**: ~1,229

## Architecture Decisions

### Channel status table with comfy-table
`weft channels status` uses `comfy-table` for terminal-friendly table output.
It displays 9 known channel types (Telegram, Slack, Discord, Email, Mochat,
Web, IRC, Matrix, Signal) plus any extra channels from the `channels.extra`
config map. Each row shows the channel name, status (Configured/Not Configured),
and a summary of key config fields.

### comfy-table pinned to 7.1.x
Version 7.2.2 of `comfy-table` uses `let` chains which are unstable on
Rust 1.85 (our MSRV). The Cargo.lock pins to 7.1.x to maintain compatibility.
This will be unlocked when the MSRV is raised or `let` chains stabilize.

### Cron CLI with normalize_cron_expr()
The `cron` crate v0.15 expects 7-field expressions (`sec min hour dom mon dow year`).
Users typically write 5-field crontabs. `normalize_cron_expr()` detects 5-field
input and expands it to 7-field by prepending `0` (seconds) and appending `*` (year).
This provides a familiar user experience while satisfying the library's parser.

### Cron storage via CronStore
The CLI's cron commands use `CronStore`, a thin wrapper around JSONL file I/O
at `~/.clawft/cron_jobs.jsonl`. It's distinct from `clawft-services::cron_service::storage`
because the CLI operates directly on the file without a running service, while
the service uses the same file format but manages the scheduler lifecycle.

### Markdown converter trait
`MarkdownConverter` is a simple trait with one method: `convert(&self, markdown: &str) -> String`.
Three implementations:
- **Telegram**: Converts to HTML (`<b>`, `<i>`, `<code>`, `<pre>`, `<a>`) with XSS-safe `escape_html()`
- **Slack**: Converts to mrkdwn (`*bold*`, `_italic_`, `` `code` ``, `>quote`, `<url|text>`)
- **Discord**: Passthrough (Discord natively supports Markdown), normalizes code block fences

### XSS-safe Telegram HTML
The `TelegramMarkdownConverter` escapes `<`, `>`, `&` in all text content before
wrapping in HTML tags. This prevents injection attacks if user-generated content
flows through the converter to Telegram's HTML parser.

### pulldown-cmark event-based conversion
All three converters use `pulldown-cmark::Parser` to get a stream of `Event`s,
then map each event to the target format. This is more robust than regex-based
conversion because it handles nested structures, code blocks with language hints,
and edge cases like nested emphasis correctly.

## Dependencies Added

- `clawft-services` (workspace): Type access for cron job definitions
- `pulldown-cmark` (workspace): Markdown parsing for all 3 converters
- `comfy-table` (workspace): Terminal table formatting for channel status
- `cron` (workspace): Expression validation in CLI (shared with services crate)

## Test Coverage (103 tests)

- **commands/channels.rs** (12 tests): Table output with various config combinations, empty config, extra channels
- **commands/cron.rs** (28 tests): Add/remove/enable/disable/list/run, normalize_cron_expr, JSONL roundtrip, duplicate names, missing jobs
- **markdown/mod.rs** (2 tests): Trait object safety, identity conversion
- **markdown/telegram.rs** (22 tests): Bold, italic, code, pre, links, headings, lists, quotes, escape_html, nested markup, empty input
- **markdown/slack.rs** (19 tests): Bold, italic, code, pre, links, headings, lists, quotes, Slack-specific escaping
- **markdown/discord.rs** (16 tests): Passthrough, code blocks, language hints, fence normalization
- **main.rs/commands integration** (4 tests): CLI arg parsing, subcommand dispatch

## Quality Gates

| Check | Result |
|-------|--------|
| `cargo build -p clawft-cli` | PASS |
| `cargo test -p clawft-cli` | PASS (103 tests, 0 failures) |
| `cargo clippy -p clawft-cli -- -D warnings` | PASS (0 warnings) |

## Integration Points

- **clawft-types::config**: Reads channel configs for status display, cron job schema
- **clawft-services::cron_service**: Shares JSONL format for cron persistence
- **clawft-channels**: Factory names used for channel status discovery
- **clawft-core::agent**: Markdown converters used when sending outbound messages to channels

## Next Steps

1. Add `weft channels test <name>` for connectivity testing
2. Add `weft mcp list` and `weft mcp call` subcommands
3. Add shell completion generation (`weft completions bash/zsh/fish`)
4. Wire markdown converters into outbound message pipeline per channel type
5. Add `weft config show` for displaying resolved configuration
