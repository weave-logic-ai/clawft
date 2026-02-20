# Business Requirements: Plugin & Skill System + Channel Enhancements

> Draft addendum to [01-business-requirements.md](../01-business-requirements.md).
> Covers Workstreams C (C1-C7) and E (E1-E6) from the [Unified Sprint Plan](../improvements.md).

---

## 5d. Plugin & Skill System (Workstream C)

### Motivation

ClawFT's current extensibility model requires modifying core crates to add new tools, channels, or pipeline stages. This creates a maintenance bottleneck and prevents community contribution without deep knowledge of the internal architecture. The Plugin & Skill System introduces a unified trait-based plugin interface, a WASM sandbox for portable distribution, and an OpenClaw-compatible skill loader that allows users to install, create, and hot-reload skills at runtime.

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| PS-1 | As a plugin developer, I want a unified trait interface (`Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler`) so I can extend clawft with new tools, channels, and skills without modifying core crates | P0 |
| PS-2 | As a plugin developer, I want a manifest schema (JSON/YAML) that declares my plugin's capabilities, dependencies, and required permissions so clawft can validate and load it automatically | P0 |
| PS-3 | As a community member, I want to distribute plugins as portable WASM binaries that work on any platform without requiring the end user to have a Rust toolchain | P1 |
| PS-4 | As a plugin developer, I want the WASM host to provide filesystem, environment, and HTTP client access through WASI so my plugin can interact with the outside world within sandbox limits | P1 |
| PS-5 | As a user, I want to install skills from ClawHub with a single command (`weft skill install github.com/openclaw/skills/coding-agent`) and have them auto-register with my agent | P1 |
| PS-6 | As a user, I want skills defined in `SKILL.md` format (YAML frontmatter + tool description + execution hints) to be parsed and loaded automatically | P1 |
| PS-7 | As a developer, I want skill changes to take effect immediately without restarting the gateway, so I can iterate on skill logic during a live session | P1 |
| PS-8 | As a developer, I want a file-system watcher on skill directories that detects changes and triggers hot-reload automatically | P1 |
| PS-9 | As a user, I want skill precedence layering (workspace > managed/local > bundled) so I can override bundled behavior with project-specific skills | P1 |
| PS-10 | As a user, I want the agent to automatically create reusable skills when it detects repeated task patterns, writing `SKILL.md` + implementation and installing into my managed skills directory | P2 |
| PS-11 | As a developer, I want agent commands wired through a slash-command registry (not inline match blocks) so that skill-contributed commands are dynamically discoverable | P2 |
| PS-12 | As an IDE user, I want loaded skills and tools automatically exposed through the MCP server so VS Code, Copilot, and Claude Desktop can discover and invoke them | P1 |
| PS-13 | As a developer, I want plugins to declare their own skill directories in the manifest (`clawft.plugin.json`) so plugin-shipped skills participate in normal precedence resolution | P2 |
| PS-14 | As a developer, I want the PluginHost to unify channels and tools under the plugin trait system, and support `SOUL.md` / `AGENTS.md` personality injection into pipeline stages | P2 |
| PS-15 | As a user, I want WASM plugins to be size-budgeted (< 300 KB uncompressed, < 120 KB gzipped) so they remain lightweight and fast to download | P1 |

### Feature Summary

| Feature | Description | Sprint Item | Config Layer |
|---------|-------------|-------------|-------------|
| Plugin trait crate | `clawft-plugin` crate with `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler` traits + manifest schema | C1 | N/A (library) |
| WASM plugin host | `wasmtime` + WIT component model host in `clawft-wasm`; full WASI filesystem, environment, and HTTP client impls | C2 | Global |
| Skill Loader | `SKILL.md` parser (serde_yaml), auto-registration, ClawHub discovery (HTTP index + git clone) | C3 | Both |
| Hot-reload | `notify`-crate file watcher on skill directories; changes take effect mid-session | C4 | Both |
| Skill precedence | workspace > managed (`~/.clawft/skills`) > bundled; plugin-declared skill dirs | C4 | Both |
| Autonomous skill creation | Agent detects repeated patterns, generates `SKILL.md` + implementation, compiles to WASM, installs to managed dir | C4a | Global |
| Slash-command registry | Wire `interactive/builtins` and `registry` modules; dynamic skill-contributed commands | C5 | N/A (runtime) |
| MCP skill exposure | Auto-expose loaded skills/tools through MCP server for external IDE integration | C6 | Project |
| Unified PluginHost | Channels + tools unified under plugin trait system; concurrent `start_all`/`stop_all`; personality injection | C7 | Both |

### Non-Goals (Plugin & Skill System)

- GUI-based plugin management (CLI and config only)
- Plugin marketplace with payment/monetization (ClawHub is free and open)
- Runtime `.so`/`.dll` native plugin loading (WASM only for dynamic loading; native plugins are compile-time)
- Plugin signing or chain-of-trust verification in initial release (future security hardening in K3a)
- Voice handler implementation (trait placeholder only; implementation deferred to Workstream G)

### Dependencies

| Item | Depends On | Reason |
|------|-----------|--------|
| C2 (WASM host) | C1 (trait crate) | WASM host implements the plugin traits |
| C3 (Skill Loader) | C1 (trait crate) | Skills register as trait implementations |
| C3 (Skill Loader) | B3 (file splits) | `skills_v2.rs` hand-rolled YAML parser must be replaced with `serde_yaml` first |
| C4 (hot-reload) | C2 + C3 | Hot-reload requires both the WASM host and skill loader to be functional |
| C4a (autonomous creation) | C4 | Self-improving skills require the full hot-reload pipeline |
| C5 (slash-commands) | C3 | Dynamic commands come from loaded skills |
| C6 (MCP exposure) | C3 | MCP exposes loaded skills |
| C7 (unified PluginHost) | C1 | Unification targets the plugin trait system |

---

## 5e. Channel Enhancements (Workstream E)

### Motivation

ClawFT currently supports three active channels (CLI, Telegram, Discord) with Slack partially implemented. Users increasingly expect their AI assistant to meet them where they already communicate -- email inboxes, WhatsApp conversations, enterprise platforms like Microsoft Teams and Google Chat, and open protocols like Matrix and IRC. The plugin trait system (Workstream C) makes it possible to add each new channel as an isolated plugin without modifying core. This workstream also addresses reliability gaps in existing channels (Discord resume) and adds proactive agent behavior through enhanced heartbeat scheduling.

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| CE-1 | As a Discord user, I want the bot to resume its gateway session after a reconnect instead of re-identifying, so I do not miss messages during brief network interruptions | P1 |
| CE-2 | As a professional user, I want my AI assistant to triage my inbox, draft replies, and handle attachments via IMAP/SMTP so I can manage email through natural language | P1 |
| CE-3 | As a professional user, I want Gmail OAuth2 support so the email channel works with Google Workspace without storing my password in config | P1 |
| CE-4 | As a mobile-first user, I want to interact with my agent through WhatsApp so I can get help from the platform I use most | P2 |
| CE-5 | As a privacy-conscious user, I want to communicate with my agent over Signal so my conversations are end-to-end encrypted | P3 |
| CE-6 | As an Apple ecosystem user, I want to interact with my agent through iMessage on macOS | P3 |
| CE-7 | As a self-hosted community member, I want to connect my agent to Matrix rooms so it participates in my federated chat infrastructure | P2 |
| CE-8 | As a user of legacy systems, I want an IRC channel adapter so my agent is reachable in IRC channels and DMs | P3 |
| CE-9 | As a Google Workspace user, I want my agent accessible in Google Chat DMs and Spaces so I can interact with it alongside my team's existing tools | P2 |
| CE-10 | As a Google Workspace user, I want the Google Chat channel to reuse the OAuth2 flow from the email channel so I do not need to configure credentials twice | P2 |
| CE-11 | As an enterprise user, I want my agent available in Microsoft Teams channels and 1:1 chats so it integrates into my organization's communication platform | P2 |
| CE-12 | As an enterprise user, I want the Teams channel to authenticate via Azure AD so it complies with my organization's identity policies | P2 |
| CE-13 | As a user, I want proactive agent check-ins (inbox triage, status summaries, daily briefings) triggered by cron schedules without requiring me to send a message first | P2 |
| CE-14 | As an admin, I want all new channels implemented as `ChannelAdapter` plugins so they are independently deployable and do not increase the core binary size | P1 |

### Feature Summary

| Feature | Description | Sprint Item | Config Layer |
|---------|-------------|-------------|-------------|
| Discord gateway resume | Implement OP 6 resume using stored `session_id` + `resume_url` instead of always re-identifying | E1 | Global |
| Email channel (IMAP/SMTP) | Full inbox access: read, reply, forward, attachments; `lettre` + `imap` crates | E2 | Global + Project |
| Email OAuth2 (Gmail) | Gmail OAuth2 via `oauth2` crate; no plaintext passwords in config | E2 | Global |
| Proactive email triage | Cron-triggered inbox scanning with AI-generated summaries and draft replies | E2 + E6 | Global |
| WhatsApp channel | WhatsApp Cloud API wrapper; media support (images, documents, voice notes) | E3 | Global |
| Signal bridge | `signal-cli` subprocess adapter; text + attachment support | E4 | Global |
| iMessage bridge | macOS-only bridge via AppleScript/Shortcuts or `imessage-exporter` | E4 | Global |
| Matrix channel | Matrix client SDK adapter; room joins, E2E encryption optional | E5 | Global |
| IRC channel | IRC client adapter; channel and DM support; SASL auth | E5 | Global |
| Google Chat channel | Google Workspace API adapter; DMs and Spaces; OAuth2 (shared with email) | E5a | Global |
| Microsoft Teams channel | Bot Framework / Graph API adapter; channels and 1:1 chats; Azure AD auth | E5b | Global |
| Enhanced heartbeat | CronService "check-in" mode for proactive behavior across all channels | E6 | Global + Project |

### Non-Goals (Channel Enhancements)

- Native mobile apps for WhatsApp or Signal (we use their official APIs/CLIs, not custom clients)
- Voice/video calling through any channel (text and media only; voice deferred to Workstream G)
- Chinese platform channels (Feishu, DingTalk, Mochat, QQ) -- community-contributed via plugin system
- SMS/MMS channel (carriers require compliance programs beyond scope)
- Multi-account support per channel in initial release (one credential set per channel type)

### Dependencies

| Item | Depends On | Reason |
|------|-----------|--------|
| E2 (Email) | C1 (trait crate) | Implemented as `ChannelAdapter` plugin |
| E2 (Email) | A4 (credential fix) | Email credentials must not be stored as plaintext |
| E3 (WhatsApp) | C1 (trait crate) | Implemented as plugin |
| E4 (Signal/iMessage) | C1 (trait crate) | Implemented as plugin |
| E5 (Matrix/IRC) | C1 (trait crate) | Implemented as plugin |
| E5a (Google Chat) | C1 (trait crate) | Implemented as plugin |
| E5a (Google Chat) | F6 (OAuth2 helper) | Reuses generic OAuth2 flow |
| E5b (Teams) | C1 (trait crate) | Implemented as plugin |
| E6 (heartbeat) | B4 (cron unification) | Enhanced heartbeat builds on unified CronService |

---

## Updated Channel Feature Matrix (Section 5 addendum)

### Channels (pluggable, each behind a Cargo feature flag)

| Channel | Priority | Feature Flag | Transport | Auth Method | Plugin? | Notes |
|---------|----------|--------------|-----------|-------------|---------|-------|
| CLI/Interactive | P0 | `cli` | stdin/stdout | N/A | No (core) | Built-in, not a plugin |
| Telegram | P0 | `channel-telegram` | HTTP long-poll | Bot token | Yes | Existing, functional |
| Slack | P1 | `channel-slack` | WebSocket (Socket Mode) + REST | Bot token + App token | Yes | Existing, partially implemented |
| Discord | P1 | `channel-discord` | WebSocket (Gateway) + REST | Bot token | Yes | Existing; needs resume fix (E1) |
| Email (IMAP/SMTP) | P1 | `channel-email` | IMAP + SMTP | Password or OAuth2 (Gmail) | Yes | New plugin; `lettre` + `imap` crates |
| WhatsApp | P2 | `channel-whatsapp` | HTTPS (Cloud API) | Access token | Yes | New plugin; official Cloud API |
| Google Chat | P2 | `channel-google-chat` | HTTPS (Workspace API) | OAuth2 (service account or user) | Yes | New plugin; DMs + Spaces |
| Microsoft Teams | P2 | `channel-teams` | HTTPS (Bot Framework / Graph API) | Azure AD (client credentials) | Yes | New plugin; channels + 1:1 chats |
| Matrix | P2 | `channel-matrix` | HTTPS + optional WebSocket | Access token / SSO | Yes | New plugin; E2E encryption optional |
| Signal | P3 | `channel-signal` | subprocess (`signal-cli`) | Phone number + registration | Yes | New plugin; text + attachments |
| iMessage | P3 | `channel-imessage` | macOS bridge (AppleScript) | macOS login session | Yes | New plugin; macOS-only |
| IRC | P3 | `channel-irc` | TCP/TLS | SASL / NickServ | Yes | New plugin; channels + DMs |

### Channel Capability Matrix

| Channel | Text | Rich Text | Images | Files | Reactions | Threads | Group Chat | Proactive Outbound |
|---------|------|-----------|--------|-------|-----------|---------|------------|-------------------|
| CLI/Interactive | Yes | Markdown | No | No | No | No | No | No |
| Telegram | Yes | Markdown | Yes | Yes | No | Yes (reply-to) | Yes | Yes (via chat_id) |
| Slack | Yes | Blocks/Markdown | Yes | Yes | Yes | Yes | Yes | Yes (via channel_id) |
| Discord | Yes | Markdown | Yes | Yes | Yes | Yes (forum) | Yes | Yes (via channel_id) |
| Email | Yes | HTML | Yes (inline) | Yes (attach) | No | Yes (In-Reply-To) | Yes (CC/BCC) | Yes (compose new) |
| WhatsApp | Yes | Limited | Yes | Yes | Yes | No | Yes | Yes (template msg) |
| Google Chat | Yes | Cards | Yes | No | Yes | Yes | Yes (Spaces) | Yes (via Space) |
| Teams | Yes | Adaptive Cards | Yes | Yes | Yes | Yes | Yes | Yes (via channel) |
| Matrix | Yes | HTML/Markdown | Yes | Yes | Yes | Yes | Yes (rooms) | Yes (via room_id) |
| Signal | Yes | No | Yes | Yes | Yes | No | Yes | Yes (via phone) |
| iMessage | Yes | No | Yes | Yes | Yes (tapback) | No | Yes | Yes (via contact) |
| IRC | Yes | No | No | No (DCC only) | No | No | Yes | Yes (via channel) |

---

## Success Criteria Additions

### Plugin & Skill System (Phase 5: Extend)

- [ ] `clawft-plugin` crate compiles independently with all six trait definitions
- [ ] Plugin manifest schema validates JSON and YAML manifests
- [ ] At least one channel (email) implemented entirely as a `ChannelAdapter` plugin, not touching `clawft-core`
- [ ] WASM plugin host loads and executes a `.wasm` plugin with filesystem and HTTP access
- [ ] WASM plugins enforced under size budget (< 300 KB uncompressed)
- [ ] `weft skill install <url>` downloads, validates, and registers a skill from ClawHub
- [ ] `SKILL.md` files parsed correctly (YAML frontmatter + markdown body) and auto-registered
- [ ] Skill precedence layering works: workspace skill overrides managed skill overrides bundled skill
- [ ] Hot-reload detects file changes in skill directories and updates the running agent within 2 seconds
- [ ] Autonomous skill creation generates valid `SKILL.md` + implementation from repeated patterns
- [ ] Loaded skills appear in `weft skill list` output
- [ ] MCP server exposes all loaded skills as callable tools
- [ ] Slash-command registry discovers commands from loaded skills
- [ ] `PluginHost.start_all()` and `stop_all()` execute concurrently, not sequentially

### Channel Enhancements (Phase 5: Extend)

- [ ] Discord gateway correctly resumes sessions using stored `session_id` and `resume_url`
- [ ] Email channel reads inbox via IMAP, sends replies via SMTP, handles attachments
- [ ] Gmail OAuth2 flow completes without plaintext passwords in config
- [ ] Proactive email triage runs on cron schedule and produces summaries
- [ ] WhatsApp channel sends and receives text messages via Cloud API
- [ ] Google Chat channel handles DMs and Space messages
- [ ] Microsoft Teams channel handles channel messages and 1:1 chats
- [ ] Matrix channel joins rooms and sends/receives messages
- [ ] Signal channel sends and receives messages via `signal-cli`
- [ ] IRC channel connects, authenticates (SASL), and handles channel + DM messages
- [ ] All new channels implemented as `ChannelAdapter` plugins (zero changes to `clawft-core`)
- [ ] Enhanced heartbeat triggers proactive check-ins across all configured channels
- [ ] `weft channels status` shows all new channels with connection state
- [ ] Each new channel has a Cargo feature flag and can be excluded from the build

---

## Risk Register Additions

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| WASM plugin sandbox escape | Low | Critical | 5 | wasmtime's security model is well-tested; add capability-based permissions in manifest; defense in depth via K3 |
| Skill hot-reload race condition | Medium | Medium | 4 | Atomic swap of skill registry; brief lock during reload; version counter for cache invalidation |
| Autonomous skill creation produces unsafe code | Medium | High | 6 | Generated skills run in WASM sandbox; require human approval for skills touching filesystem or network |
| WhatsApp Cloud API rate limits / compliance | High | Medium | 6 | Template message pre-approval; exponential backoff; clear docs on Meta's business verification requirements |
| Email OAuth2 token refresh failure | Medium | Medium | 4 | Automatic token refresh with retry; fall back to polling; alert user if refresh fails repeatedly |
| Google/Microsoft API breaking changes | Low | Medium | 3 | Pin API versions; abstract behind ChannelAdapter trait so core is unaffected |
| Signal CLI subprocess management | Medium | Medium | 4 | Health check pings; automatic restart on crash; timeout on unresponsive subprocess |
| iMessage bridge macOS-only limitation | Certain | Low | 3 | Clearly document as macOS-only; feature flag prevents compilation on other platforms |
| Plugin manifest schema evolution | Medium | Medium | 4 | Version field in manifest; backward-compatible schema changes; migration tool for breaking changes |
| Skill precedence conflicts confuse users | Medium | Low | 3 | `weft skill list --verbose` shows active skill and which layer it came from; warn on shadowing |
