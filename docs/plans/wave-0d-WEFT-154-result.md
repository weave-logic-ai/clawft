# WEFT-154 result — Email channel IMAP poll + SMTP send

**Branch:** `wave0d/weft-154-email-channel`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb45e-9f1f-7ed2-91ad-4181c5082960`  
**Base:** `release/0.8-staging`

## Problem

The email `ChannelAdapter` shipped with a green unit suite but a stub
runtime: `start()` logged `polling for new emails (stub)` and never
opened IMAP; `send()` fabricated a synthetic Message-ID without SMTP.
Enabling `--features email` would silently drop every message.

## What shipped

Real IMAP inbound + SMTP outbound behind the `email` feature flag.

| Surface | Implementation |
|---------|----------------|
| IMAP poll | `imap` 2.4 + `native-tls` under `spawn_blocking`: connect (implicit TLS or STARTTLS) → login → `SELECT` → `UID SEARCH UNSEEN` → `UID FETCH BODY.PEEK[]` → mark `\Seen` |
| RFC822 parse | `mailparse`; prefer `text/plain`, light HTML strip fallback; body truncated at `max_body_chars` |
| SMTP send | `lettre` 0.11 async + rustls: MIME build with client Message-ID; STARTTLS (default 587) or implicit TLS (465); cleartext only when `smtp_use_tls=false` |
| Auth | Password inline (`SecretString`) **or** `password_env`; OAuth2 config remains typed, XOAUTH2 SASL deferred |
| Resilience | Exponential backoff 1s→60s on transport errors; `CancellationToken` respected in poll and backoff sleeps |
| Dedupe | In-process Message-ID set + server `\Seen` |
| Test harness | `ImapBackend` / `SmtpBackend` traits + `MockImapBackend` / `MockSmtpBackend` |

### Files

- `crates/clawft-channels/Cargo.toml` — feature `email` pulls `imap`, `lettre`, `mailparse`, `native-tls`
- `crates/clawft-channels/src/email/channel.rs` — real poll loop + SMTP send
- `crates/clawft-channels/src/email/imap_client.rs` — **new** real IMAP backend + RFC822 helpers
- `crates/clawft-channels/src/email/smtp_client.rs` — **new** real SMTP backend
- `crates/clawft-channels/src/email/transport.rs` — **new** backend traits + mocks
- `crates/clawft-channels/src/email/types.rs` — `password_env`, `resolve_password()`
- `crates/clawft-channels/src/email/mod.rs` — re-exports
- `.planning/sparc/phase4/06-channel-enhancements/04-element-06-tracker.md` — E2 runtime status
- `scripts/build.sh` — honor `--features` for `check` / `test` / `clippy`
- `Cargo.lock` — email dep graph

## Verification

```text
cargo test -p clawft-channels --features email --lib email
# 46 passed

scripts/build.sh test clawft-channels --features email
# 242 passed (full clawft-channels suite with email feature)

cargo clippy -p clawft-channels --features email --tests -- -D warnings
# ok

scripts/build.sh check
# ok (workspace)
```

## Acceptance

| Criterion | Status |
|-----------|--------|
| `start()` opens IMAP, polls UNSEEN, parses RFC822, emits inbound | Yes |
| `send()` MIME via lettre + SMTP TLS; returns Message-ID | Yes (client-generated Message-ID on the wire; SMTP does not echo it) |
| Auth: password + password_env; STARTTLS + implicit TLS | Yes |
| Reconnect-with-backoff; cancellation respected | Yes |
| Unit + mock IMAP/SMTP harness tests | Yes (46 email tests) |
| `scripts/build.sh test` + clippy with `--features email` | Yes (build.sh now forwards FEATURES) |
| Tracker no longer “stub IMAP/SMTP runtime” | Yes (E2 Done) |

## Follow-ups

- OAuth2 / XOAUTH2 SASL for Gmail IMAP+SMTP (config already accepts OAuth2 shape)
- Optional IMAP IDLE instead of interval poll when server advertises CAPABILITY IDLE
- Attachment / media path (`supports_media` remains false)
