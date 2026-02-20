# Element 06: Channel Enhancements -- Execution Tracker

## Summary

- **Total items**: 9 (E1-E6, E5a, E5b, plus E5 split into Matrix/IRC)
- **Workstream**: E (Channel Enhancements)
- **Timeline**: Weeks 4-8
- **Status**: Planning -> Development
- **Dependencies**: 04/C1 (ChannelAdapter trait), 03/A4 (SecretRef), 03/B4 (CronService), 07/F6 (OAuth2 helper)
- **Blocks**: None directly

---

## Execution Schedule

Element 06 has 9 channel items across 3 phases spanning Weeks 4-8.

### Week 4-5 (E-Fix -- 2 items)

- [ ] E1 -- Discord Gateway Resume (OP 6): use stored session_id/resume_url -- clawft-channels/src/discord/channel.rs
- [ ] E6 -- Enhanced heartbeat / proactive check-in mode (depends on B4 CronService) -- clawft-services/src/heartbeat/

### Week 5-7 (E-Enterprise -- 3 items)

- [ ] E2 -- Email channel (IMAP + SMTP + OAuth2 for Gmail) -- clawft-channels/src/email/ (new, feature-gated, P0 MVP)
- [ ] E5a -- Google Chat Workspace API (depends on F6 OAuth2 helper) -- clawft-channels/src/google_chat/ (new, feature-gated) **TIMELINE RISK: F6 at Week 7-9**
- [ ] E5b -- Microsoft Teams Bot Framework -- clawft-channels/src/teams/ (new, feature-gated)

### Week 6-8 (E-Consumer -- 4 items)

- [ ] E3 -- WhatsApp Cloud API -- clawft-channels/src/whatsapp/ (new, feature-gated)
- [ ] E4 -- Signal subprocess bridge -- clawft-channels/src/signal/ (new, feature-gated)
- [ ] E5 -- Matrix channel (Matrix SDK) -- clawft-channels/src/matrix/ (new, feature-gated)
- [ ] E5 -- IRC channel (IRC protocol) -- clawft-channels/src/irc/ (new, feature-gated)

---

## Per-Item Status Table

| Item | Description | Priority | Week | Crate(s) | Status | Owner | Branch | Key Deliverable |
|------|-------------|----------|------|----------|--------|-------|--------|-----------------|
| E1 | Discord Gateway Resume (OP 6) | P1 | 4-5 | clawft-channels/src/discord/channel.rs | Pending | -- | -- | Resume via stored session_id/resume_url instead of re-Identify |
| E2 | Email channel (IMAP+SMTP+OAuth2) | P0 MVP | 5-7 | clawft-channels/src/email/ (new) | Pending | -- | -- | Receive, triage, reply; Gmail OAuth2 flow; no plaintext passwords |
| E3 | WhatsApp Cloud API | P1 | 6-8 | clawft-channels/src/whatsapp/ (new) | Pending | -- | -- | Send/receive text messages via Cloud API; webhook endpoint |
| E4 | Signal subprocess bridge | P2 | 6-8 | clawft-channels/src/signal/ (new) | Pending | -- | -- | Send/receive via signal-cli subprocess; PID tracking; crash recovery |
| E5 | Matrix / IRC channels | P2 | 6-8 | clawft-channels/src/matrix/, src/irc/ (new) | Pending | -- | -- | Matrix: join rooms + send/receive; IRC: connect + send/receive |
| E5a | Google Chat Workspace API | P1 | 5-7 | clawft-channels/src/google_chat/ (new) | Pending | -- | -- | Send/receive via Workspace API; OAuth2 via F6; Pub/Sub subscription |
| E5b | Microsoft Teams Bot Framework | P1 | 5-7 | clawft-channels/src/teams/ (new) | Pending | -- | -- | Send/receive via Bot Framework; Azure AD authentication |
| E6 | Enhanced heartbeat / check-in | P1 | 4-5 | clawft-services/src/heartbeat/ | Pending | -- | -- | Proactive check-ins on cron schedule across all configured channels |

---

## Internal Dependency Graph

```
E6 (heartbeat, Week 4-5) ──────> E2 (email triage, Week 5-7)
  Proactive email triage requires both the email channel (E2) and
  the enhanced heartbeat (E6). E6 SHOULD complete in Week 5
  before E2's triage feature in Week 6-7.

E5a (Google Chat) ──────> F6 (OAuth2 helper, Element 07)
  E5a requires the OAuth2 helper from Element 07.
  F6 is scheduled at Week 7-9 but E5a needs Week 5-7.
  TIMELINE RISK: Either coordinate with Element 07 to accelerate
  F6, or defer E5a to Week 8+.

E1 (Discord Resume) is independent.
  Can start immediately at Week 4; no internal dependencies.

E3, E4, E5 (Consumer channels) are independent of each other.
  Can be worked in parallel by multiple developers.

All new channels (E2-E5b) depend on C1 (ChannelAdapter trait)
  from Element 04. C1 must land before new channel implementation.

All channel credential fields depend on A4 (SecretRef)
  from Element 03. A4 must land before config structs are finalized.

E6 depends on B4 (CronService unification)
  from Element 03. B4 must land before heartbeat scheduling.
```

---

## Cross-Element Dependencies

| Source (Element 06) | Target (Other Element) | Type | Impact |
|---------------------|------------------------|------|--------|
| C1 (Element 04) | E2, E3, E4, E5, E5a, E5b | Blocks | All new channels implement ChannelAdapter trait from C1; cannot start without it |
| A4 (Element 03) | E2, E5a, E5b | Blocks | SecretRef type required for all credential config fields |
| B4 (Element 03) | E6 | Blocks | CronService needed for heartbeat scheduling |
| F6 (Element 07) | E5a | Blocks | OAuth2 helper needed for Google Chat authentication -- **TIMELINE RISK** |
| D8 (Element 05) | E2-E5b | Implicit | New channels must use bounded bus API from D8; D8 should stabilize before E-Enterprise |
| D6 (Element 05) | All channels | Implicit | Channels produce InboundMessage carrying sender_id; contract should be stable |

### Channel Trait Migration Note

New channels (E2-E5b) implement the `ChannelAdapter` trait from `clawft-plugin` directly. Existing channels (Telegram, Discord, Slack) will be migrated under C7 (PluginHost unification). During the transition, a `ChannelAdapter->Channel` shim in `clawft-plugin` allows new `ChannelAdapter` implementations to be loaded by the existing `PluginHost`. E1 (Discord Resume) modifies the existing `Channel` impl; it does NOT use `ChannelAdapter`.

---

## Exit Criteria

### E-Fix

- [ ] **E1**: Discord reconnects via Resume (OP 6) instead of re-Identify, using stored session_id and resume_url
- [ ] **E6**: Enhanced heartbeat triggers proactive check-ins across all configured channels on cron schedule

### E-Enterprise

- [ ] **E2**: Email channel receives, triages, and replies to messages; Gmail OAuth2 flow completes without plaintext passwords in config
- [ ] **E5a**: Google Chat channel sends and receives messages via Workspace API (blocked until F6 is available)
- [ ] **E5b**: Microsoft Teams channel sends and receives messages via Bot Framework

### E-Consumer

- [ ] **E3**: WhatsApp channel sends and receives text messages via Cloud API
- [ ] **E4**: Signal channel sends and receives messages via `signal-cli` subprocess
- [ ] **E5**: Matrix channel joins rooms and sends/receives messages

### Trait & Architecture

- [ ] All new channels (E2-E5b) implement `ChannelAdapter` plugin trait (not the legacy `Channel` trait)
- [ ] All new channel crates are feature-gated (disabled by default)

### Security

- [ ] All channel config credential fields use `SecretRef` type (no plaintext secrets in config structs, including WhatsApp `verify_token`)
- [ ] OAuth2 flows include `state` parameter for CSRF protection
- [ ] Subprocess-based channels (Signal) sanitize all arguments against command injection
- [ ] OAuth2 tokens persisted to encrypted file (`~/.clawft/tokens/`, permissions 0600)

### Regression

- [ ] All existing channel tests pass
- [ ] All 2,075+ existing tests still pass

---

## Security Checklist

| Check | Items Affected | Requirement |
|-------|---------------|-------------|
| SecretRef for all credentials | E2, E3, E4, E5, E5a, E5b | No plaintext `String` credential fields in config structs; use `SecretRef` type from A4 |
| OAuth2 state parameter | E2 (Gmail), E5a (Google Chat) | CSRF protection via `state` parameter in OAuth2 authorization URL |
| Subprocess command injection | E4 (Signal) | All arguments to `signal-cli` subprocess must be sanitized; no shell interpolation |
| Token persistence permissions | E2, E5a, E5b | OAuth2 refresh tokens persisted with 0600 file permissions in `~/.clawft/tokens/` |
| Webhook signature verification | E3 (WhatsApp) | Verify `X-Hub-Signature-256` header on incoming webhooks |
| Bot Framework token validation | E5b (Teams) | Validate JWT from Azure AD before processing activity |

---

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| E-Fix Review | E1, E6 | Code review; E1 reconnect test with simulated disconnect; E6 cron schedule test |
| E-Enterprise Security Review | E2, E5a, E5b | Security-focused review; OAuth2 flow tests; SecretRef usage verified; CSRF state parameter |
| E-Enterprise Functional Review | E2, E5a, E5b | Standard code review; end-to-end message flow tests per channel |
| E-Consumer Review | E3, E4, E5 | Code review; E4 subprocess management tests (zombie prevention, crash recovery) |
| Trait Compliance Review | E2-E5b | Verify all new channels implement `ChannelAdapter`, not legacy `Channel` trait |

---

## Risk Register

Scoring: Likelihood (Low=1, Medium=2, High=3) x Impact (Low=1, Medium=2, High=3, Critical=4)

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| F6/E5a timeline mismatch blocks Google Chat delivery | High | Medium | 6 | Coordinate with Element 07 to accelerate F6; if not possible, defer E5a to Week 8+. Do NOT implement standalone OAuth2 (avoid duplication). |
| OAuth2 token refresh rotation loses refresh token on process restart | Medium | High | 6 | Persist rotated refresh tokens to encrypted file (`~/.clawft/tokens/`, permissions 0600); document recovery procedure for token loss. |
| WhatsApp Cloud API rate limits throttle message delivery | Medium | Medium | 4 | Implement rate limiter with exponential backoff; queue outbound messages; monitor 429 responses. |
| Signal `signal-cli` subprocess management (zombie processes, crash recovery) | Medium | Medium | 4 | PID tracking with health checks; auto-restart on crash with exponential backoff; timeout kill for hung subprocesses. |
| ChannelAdapter shim introduces subtle behavior differences from Channel trait | Low | Medium | 3 | Integration tests that exercise the shim with at least one existing channel; document known differences between traits. |

---

## Progress Summary

| Phase | Items | Pending | In Progress | Completed | % Done |
|-------|-------|---------|-------------|-----------|--------|
| E-Fix (E1, E6) | 2 | 2 | 0 | 0 | 0% |
| E-Enterprise (E2, E5a, E5b) | 3 | 3 | 0 | 0 | 0% |
| E-Consumer (E3, E4, E5) | 4 | 4 | 0 | 0 | 0% |
| **Total** | **9** | **9** | **0** | **0** | **0%** |
