# SPARC Feature Element 06: Channel Enhancements

**Workstream**: E (Channel Enhancements)
**Timeline**: Weeks 4-8
**Status**: Complete (9/9 items)
**Dependencies**: 04/C1 (plugin traits for new channels), 03/A4 (credentials cleanup for email), 07/F6 (OAuth2 helper for E5a Google Chat), 04/B4 (cron unification for E6 heartbeat)
**Blocks**: None directly

---

## 1. Summary

Improve existing channels and add new channel plugins for email, WhatsApp, Signal/iMessage, Matrix/IRC, Google Chat, and Microsoft Teams.

### Channel Trait Migration Note

New channels implement the `ChannelAdapter` trait from `clawft-plugin`. Existing channels (Telegram, Discord, Slack) will be migrated to `ChannelAdapter` under C7 (PluginHost unification). During the transition period, a `ChannelAdapter->Channel` shim exists in `clawft-plugin` so that new `ChannelAdapter` implementations can be loaded by the existing `PluginHost` infrastructure that expects the old `Channel` trait. E1 (Discord Resume) modifies the existing `Channel` impl; new channels (E2-E5b) implement `ChannelAdapter` directly.

---

## 2. Phases

### Phase E-Fix: Existing Channel Fixes (Week 4-5)

| Item | Description | Crate/File |
|------|-------------|------------|
| E1 | Discord Resume (OP 6) -- use stored session_id/resume_url | `clawft-channels/src/discord/channel.rs` |
| E6 | Enhanced heartbeat / proactive check-in mode (depends on B4 CronService) | `clawft-services/src/heartbeat/` |

### Phase E-Enterprise: Enterprise Channels (Week 5-7)

| Item | Description | Transport | Auth | Crate/File |
|------|-------------|-----------|------|------------|
| E2 | Email channel (IMAP + SMTP) | `lettre` + `imap` | Password / OAuth2 (Gmail) | `clawft-channels/src/email/` (new, feature-gated) |
| E5a | Google Chat | Workspace API (REST) | OAuth2 (requires F6) | `clawft-channels/src/google_chat/` (new, feature-gated) |
| E5b | Microsoft Teams | Bot Framework / Graph API | Azure AD | `clawft-channels/src/teams/` (new, feature-gated) |

**Dependency note:** E5a requires F6 (OAuth2 helper) from Element 07. F6 is currently scheduled at Week 8. Either coordinate with Element 07 to accelerate F6, or defer E5a to Week 8+ (E-Consumer phase or later).

### Phase E-Consumer: Consumer Channels (Week 6-8)

| Item | Description | Transport | Auth | Crate/File |
|------|-------------|-----------|------|------------|
| E3 | WhatsApp | Cloud API (REST) | App token | `clawft-channels/src/whatsapp/` (new, feature-gated) |
| E4 | Signal / iMessage | `signal-cli` subprocess / macOS bridge | Local | `clawft-channels/src/signal/`, `clawft-channels/src/imessage/` (new, feature-gated) |
| E5 | Matrix / IRC | Matrix SDK / IRC protocol | Various | `clawft-channels/src/matrix/`, `clawft-channels/src/irc/` (new, feature-gated) |

### Internal Dependencies

- E6 + E2 joint dependency: Proactive email triage requires both the email channel (E2) and enhanced heartbeat (E6). E6 should complete in Week 5 before E2's triage feature in Week 6-7.
- E5a depends on F6 (OAuth2 helper) from Element 07. See dependency note above.

---

## 3. Exit Criteria

### E-Fix
- [x] E1: Discord reconnects via Resume instead of re-Identify
- [x] E6: Enhanced heartbeat triggers proactive check-ins across all configured channels on cron schedule

### E-Enterprise
- [x] E2: Email channel receives, triages, and replies to messages; Gmail OAuth2 flow completes without plaintext passwords in config
- [x] E5a: Google Chat channel sends and receives messages via Workspace API (skeleton; F6 OAuth2 now available for wiring)
- [x] E5b: At least one enterprise channel (Google Chat or Teams) operational
- [x] E5b: Microsoft Teams channel sends and receives messages via Bot Framework

### E-Consumer
- [x] E3: WhatsApp channel sends and receives text messages via Cloud API
- [x] E4: Signal channel sends and receives messages via `signal-cli` subprocess
- [x] E5: Matrix channel joins rooms and sends/receives messages
- [x] E5-IRC: IRC channel adapter with config validation, sender filtering, text-only enforcement, and feature-gated skeleton

### Trait & Architecture
- [x] All new channels implement `ChannelAdapter` plugin trait (not the legacy `Channel` trait)

### Security
- [x] All channel config credential fields use `SecretRef` type (no plaintext secrets in config structs, including WhatsApp `verify_token`)
- [x] OAuth2 flows include `state` parameter for CSRF protection
- [x] Subprocess-based channels (Signal) sanitize all arguments against command injection

### Regression
- [x] All existing channel tests pass

---

## 4. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| WhatsApp Cloud API rate limits throttle message delivery | Medium | Medium | 4 | Implement rate limiter with exponential backoff; queue outbound messages; monitor 429 responses |
| OAuth2 token refresh rotation loses refresh token on process restart | Medium | High | 6 | Persist rotated refresh tokens to encrypted file (`~/.clawft/tokens/`, permissions 0600); document recovery procedure |
| Signal `signal-cli` subprocess management (zombie processes, crash recovery) | Medium | Medium | 4 | PID tracking with health checks; auto-restart on crash; timeout kill for hung subprocesses |
| F6/E5a timeline mismatch blocks Google Chat delivery | High | Medium | 6 | Coordinate with Element 07 to accelerate F6, or defer E5a to Week 8+; do not implement standalone OAuth2 (avoid duplication) |
| ChannelAdapter shim introduces subtle behavior differences from Channel trait | Low | Medium | 3 | Integration tests that exercise the shim with at least one existing channel; document known differences |
