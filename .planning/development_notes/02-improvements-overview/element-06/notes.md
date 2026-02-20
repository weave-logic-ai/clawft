# Development Notes: Element 06 - Channel Enhancements

**Workstream**: E
**Weeks**: 4-8
**Status**: Complete (9/9 items, E5a skeleton only)
**Completed**: 2026-02-20
**Agent**: Agent-06 (a1b5b44)

---

## Implementation Log

### E1: Discord Resume (OP 6) -- DONE
- Conditional Resume (OP 6) vs Identify (OP 2) on reconnect based on stored session state
- RESUMED dispatch event handler added
- Enhanced OP_INVALID_SESSION: d:true (wait with jitter) vs d:false (clear state + re-identify)
- File: `clawft-channels/src/discord/channel.rs`

### E6: Enhanced Heartbeat -- DONE
- `HeartbeatMode` enum (Simple, CheckIn) in `clawft-services/src/heartbeat/mod.rs`
- `CheckInTarget` with per-channel prompts
- Backward-compatible API: existing `new()` + new `new_check_in()`
- 11 heartbeat tests

### E2: Email Channel -- DONE
- `EmailChannelAdapter` implementing `ChannelAdapter` trait
- `EmailAdapterConfig`, `EmailAuth` (Password/OAuth2), `ParsedEmail`
- `EmailChannelAdapterFactory` for JSON config instantiation
- Feature-gated: `email`
- File: `clawft-channels/src/email/`

### E3: WhatsApp -- DONE
- `WhatsAppChannelAdapter` implementing `ChannelAdapter`
- `WhatsAppAdapterConfig` with `SecretString` for access_token and verify_token
- Cloud API REST integration
- Feature-gated: `whatsapp`

### E4: Signal -- DONE
- `SignalChannelAdapter` implementing `ChannelAdapter` with subprocess design
- `sanitize_argument()` for command injection prevention
- Config validation sanitizes phone_number and signal_cli_path
- Feature-gated: `signal`

### E5: Matrix -- DONE
- `MatrixChannelAdapter` implementing `ChannelAdapter`
- `MatrixAdapterConfig` with `SecretString` access_token, homeserver_url, auto_join_rooms
- Feature-gated: `matrix`

### E5a: Google Chat -- SKELETON (blocked on F6 OAuth2)
- `GoogleChatChannelAdapter` skeleton with config types
- `GoogleChatAdapterConfig` with project_id, service_account_key_path
- Feature-gated: `google-chat`
- Note: F6 OAuth2 plugin is now available; can be wired in follow-up

### E5b: Microsoft Teams -- DONE
- `TeamsChannelAdapter` implementing `ChannelAdapter`
- `TeamsAdapterConfig` with Azure AD fields, `SecretString` for client_secret
- Bot Framework / Graph API integration
- Feature-gated: `teams`

### E5-IRC: IRC Channel -- DONE
- `IrcChannelAdapter` implementing `ChannelAdapter` trait (skeleton, no `irc` crate dependency)
- `IrcAdapterConfig` with server, port, use_tls, nickname, channels, auth_method, password_env, allowed_senders, reconnect_delay_secs
- `validate_config()` validates auth_method is one of "none"/"nickserv"/"sasl", requires password_env for authenticated methods
- `sanitize_irc_argument()` for general argument injection prevention (shell metacharacters, protocol injection)
- `sanitize_channel_name()` allows `#` and `&` as IRC channel prefixes while banning injection characters in the body
- `is_sender_allowed()` respects allowed_senders list (empty = allow all)
- `send()` rejects Binary and Structured payloads (IRC is text-only)
- `start()` validates config, logs connection stub, waits for cancellation
- Synthetic message IDs: `irc-{target}-{timestamp}`
- Feature-gated: `irc`
- 39 IRC-specific tests (types + channel modules)
- File: `clawft-channels/src/irc/`

### Test Results
- 211 tests in clawft-channels (with irc feature) -- all pass
- 172 tests in clawft-channels (no features, regression) -- all pass
- All existing channel tests unbroken
