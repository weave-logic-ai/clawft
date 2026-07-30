# WEFT-169 result — Discord chunker (fences, markdown, Nitro, embeds, file fallback)

**Branch:** `wave0e/weft-169-discord-chunker`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef5-7951-8346-378740bbd812`  
**Base:** `release/0.8-staging`  
**Ticket:** ws05 Discord chunker — preserve fenced code, balance markdown, support Nitro/embeds/file fallback

## What shipped

### New module: `crates/clawft-channels/src/discord/chunker.rs`

Replaces the naive newline/space hard-split in `channel.rs` with a delivery planner:

| Capability | Behavior |
|------------|----------|
| **Code-fence preservation** | Mid-fence splits append a closing fence on the current chunk and reopen the next with the original language tag (` ```rust `) |
| **Markdown re-balance** | Unpaired `**` / `__` / `*` / `_` are closed at chunk end and reopened on the next chunk (skipped inside fences and `` `inline code` ``) |
| **Nitro 4000-char** | Config-driven via `DiscordConfig.nitro` and/or `max_message_length` (clamped 1..=4000). Default remains 2000 |
| **Embed packing** | Optional (`prefer_embeds`): long content → embed descriptions (≤4096) with title/field/footer/author/total (6000) limits enforced |
| **File-upload fallback** | When text/embed count exceeds `max_chunks_before_file` (default 10), remainder becomes `OutboundChunk::File`; unit-tested decision path |

### Config (`crates/clawft-types` `DiscordConfig`)

New fields (all serde-defaulted; camelCase aliases where noted):

- `nitro: bool`
- `max_message_length: Option<usize>` (`maxMessageLength`)
- `max_chunks_before_file: usize` default 10 (`maxChunksBeforeFile`)
- `prefer_embeds: bool` (`preferEmbeds`)

### API / send path

- `DiscordApiClient::create_message_with_embeds` — JSON embeds array
- `DiscordApiClient::create_message_with_file` — multipart `payload_json` + `files[0]` (live upload wired; failures fall back to a text notice in `DiscordChannel::send`)
- `DiscordChannel::send` uses `plan_chunks` + `ChunkerOptions::from_discord_config`

## Acceptance

| Criterion | Status |
|-----------|--------|
| Code-fence preservation (close/reopen + lang) | **Done** — `fence_split_closes_and_reopens_with_lang` |
| Markdown emphasis re-balance `**` `_` `*` `__` | **Done** — bold / italic / underline tests |
| Nitro 4000-char detection (config and/or runtime) | **Done (config)** — `resolve_max_message_len`, `nitro`, `max_message_length` |
| Embed support with field/title/total limits | **Done** — `enforce_limits`, `pack_content_as_embeds`, `prefer_embeds` plan path |
| File-upload fallback past N chunks | **Done (decision + multipart wire)** — plan unit-tested; send falls back to notice if upload errors |
| Unit tests per split scenario | **Done** — 24 chunker tests |
| `scripts/build.sh test clawft-channels` | **Done** — 212 passed |

## Deferred / gaps

1. **Runtime Nitro detection** — no probe of bot premium type / guild boost; operators set `nitro: true` or `maxMessageLength: 4000`. A future enhancement could learn from Discord 40001 / length-error responses and bump the limit for the session.
2. **Emphasis edge cases** — nested `***` / mismatched multi-run markers use a toggle model, not a full CommonMark emphasis stack. Good enough for agent prose; pathological markdown may still render odd.
3. **Embeds as multi-embed single message** — Discord allows up to 10 embeds per message; packing currently emits **one embed per outbound message**. Combining several small embeds into one message is not implemented.
4. **File upload content-type / spoiler / filename sanitization** — multipart uses `text/plain` and the configured filename as-is; no virus-scan or size gate beyond Discord’s own API rejection.
5. **UTF-16 length** — Discord counts UTF-16 code units; the chunker uses Unicode scalar counts (close for BMP text; astral-plane emoji can under-count vs Discord by 1 per code point).

## Verification

```bash
scripts/build.sh test clawft-channels
# Summary: 212 tests run: 212 passed

cargo test -p clawft-channels --lib discord::chunker
# 24 passed
```

## Files touched

- `crates/clawft-types/src/config/channels.rs` — DiscordConfig knobs
- `crates/clawft-channels/src/discord/chunker.rs` — **new** planner + tests
- `crates/clawft-channels/src/discord/mod.rs` — module export
- `crates/clawft-channels/src/discord/channel.rs` — `send` uses plan
- `crates/clawft-channels/src/discord/api.rs` — embeds + file multipart
- `crates/clawft-channels/src/discord/tests.rs` — `make_config` uses `Default`
- `docs/plans/wave-0e-WEFT-169-result.md` — this file
