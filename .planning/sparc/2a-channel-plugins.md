# SPARC Implementation Plan: Stream 2A - Channel Plugins

**Timeline**: Week 7-10
**Owned Crates**: `clawft-channels/slack`, `clawft-channels/discord`
**Dependencies**: Phase 1 complete (Channel trait, MessageBus, config system)

---

## 1. Agent Instructions

### Python Source Files to Read
```
repos/nanobot/nanobot/channels/slack.py        # 235 lines - Socket Mode + Web API
repos/nanobot/nanobot/channels/discord.py      # 311 lines - Gateway + REST API
repos/nanobot/nanobot/channels/base.py         # Base channel class
repos/nanobot/nanobot/channels/__init__.py     # Channel registry
```

### Planning Documents (MUST READ)
```
repos/nanobot/.planning/02-technical-requirements.md  # Channel plugin architecture
repos/nanobot/.planning/03-development-guide.md       # Stream 2A timeline
repos/nanobot/.planning/01-project-overview.md        # Channel plugin requirements
```

### Module Structure
```
clawft-channels/
├── slack/
│   ├── Cargo.toml              # Dependencies: tokio-tungstenite, reqwest, hmac, sha2
│   ├── src/
│   │   ├── lib.rs              # SlackChannel struct + Channel trait impl
│   │   ├── socket_mode.rs      # WebSocket connection management
│   │   ├── web_api.rs          # REST API client (chat.postMessage, files.upload, etc.)
│   │   ├── events.rs           # Event type definitions + parsing
│   │   ├── signature.rs        # HMAC-SHA256 signature verification
│   │   ├── markdown.rs         # Markdown → Slack mrkdwn conversion
│   │   └── error.rs            # SlackError type
│   └── tests/
│       ├── socket_mode_tests.rs
│       ├── web_api_tests.rs
│       └── markdown_tests.rs
│
├── discord/
│   ├── Cargo.toml              # Dependencies: tokio-tungstenite, reqwest, ed25519-dalek
│   ├── src/
│   │   ├── lib.rs              # DiscordChannel struct + Channel trait impl
│   │   ├── gateway.rs          # Gateway WebSocket management
│   │   ├── rest_api.rs         # REST API client with rate limiting
│   │   ├── events.rs           # Event type definitions (MESSAGE_CREATE, etc.)
│   │   ├── signature.rs        # Ed25519 signature verification
│   │   ├── rate_limit.rs       # Per-route bucket rate limiting
│   │   ├── markdown.rs         # Markdown → Discord markdown conversion
│   │   └── error.rs            # DiscordError type
│   └── tests/
│       ├── gateway_tests.rs
│       ├── rest_api_tests.rs
│       ├── rate_limit_tests.rs
│       └── markdown_tests.rs
```

---

## 2. Specification

### 2.1 Slack Plugin Requirements

#### Socket Mode WebSocket
- **Endpoint**: `wss://wss-primary.slack.com/link/?ticket=...&app_id=...`
- **Authentication**: App-level token in initial handshake
- **Protocol**: JSON messages with `envelope_id` + `type` + `payload`
- **Acknowledge**: Must send `{"envelope_id": "...", "type": "ack"}` within 3 seconds
- **Reconnection**: Exponential backoff (1s, 2s, 4s, 8s, max 30s)
- **Heartbeat**: Server sends ping frames, client responds with pong

#### Web API REST Endpoints
```
POST https://slack.com/api/chat.postMessage
  Headers: Authorization: Bearer {bot_token}
  Body: {"channel": "C123", "text": "...", "thread_ts": "..."}

POST https://slack.com/api/files.upload
  Headers: Authorization: Bearer {bot_token}
  Body: multipart/form-data with file + channels

GET https://slack.com/api/conversations.info
  Headers: Authorization: Bearer {bot_token}
  Query: ?channel=C123
```

#### Event Handling
- **message**: Direct messages, channel messages, thread replies
- **reaction_added**: React to message with emoji
- **app_mention**: @bot mentions in channels
- **file_shared**: File uploaded to channel

#### Signature Verification (Slash Commands)
```
timestamp = request.headers["X-Slack-Request-Timestamp"]
signature = request.headers["X-Slack-Signature"]
base_string = f"v0:{timestamp}:{request.body}"
expected = "v0=" + hmac_sha256(signing_secret, base_string)
assert signature == expected
```

#### Markdown Conversion
```
**bold** → *bold*
*italic* → _italic_
`code` → `code`
```blocks → ```blocks
[link](url) → <url|link>
@user → <@U123>
#channel → <#C123>
```

### 2.2 Discord Plugin Requirements

#### Gateway WebSocket
- **Endpoint**: `wss://gateway.discord.gg/?v=10&encoding=json`
- **Authentication**: Identify payload with bot token
- **Heartbeat**: Server sends heartbeat interval (e.g., 41250ms), client sends OP 1 at interval
- **Reconnection**: Resume with session_id + seq if available, else re-identify
- **Intents**: GUILDS (1<<0), GUILD_MESSAGES (1<<9), DIRECT_MESSAGES (1<<12)

#### REST API Endpoints
```
POST https://discord.com/api/v10/channels/{channel_id}/messages
  Headers: Authorization: Bot {token}
  Body: {"content": "...", "embeds": [...]}

POST https://discord.com/api/v10/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me
  Headers: Authorization: Bot {token}

GET https://discord.com/api/v10/channels/{channel_id}
  Headers: Authorization: Bot {token}
```

#### Rate Limiting
- **Global**: 50 requests per second across all routes
- **Per-route buckets**: Identified by `X-RateLimit-Bucket` header
- **Bucket limits**: Read from `X-RateLimit-Limit` and `X-RateLimit-Remaining`
- **Reset time**: `X-RateLimit-Reset` (Unix timestamp)
- **429 Response**: `Retry-After` header in milliseconds

#### Event Handling
- **MESSAGE_CREATE**: New message in guild or DM
- **INTERACTION_CREATE**: Slash command or button click
- **MESSAGE_REACTION_ADD**: User adds reaction
- **GUILD_CREATE**: Bot joins guild

#### Signature Verification (Interactions)
```
signature = request.headers["X-Signature-Ed25519"]
timestamp = request.headers["X-Signature-Timestamp"]
message = timestamp + request.body
public_key = ed25519.VerifyingKey.from_bytes(...)
assert public_key.verify(signature, message)
```

#### Markdown Conversion
```
**bold** → **bold** (unchanged)
*italic* → *italic* (unchanged)
`code` → `code` (unchanged)
||spoiler|| → ||spoiler||
<t:1234567890:R> → timestamp formatting
> quote → > quote
```

### 2.3 Channel Trait Implementation
Both plugins implement:
```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, host: Arc<dyn ChannelHost>, cancel: CancellationToken) -> Result<()>;
    async fn send(&self, msg: &OutboundMessage) -> Result<()>;
    fn is_running(&self) -> bool;
}
```

---

## 3. Pseudocode

### 3.1 Slack Socket Mode Connection
```
async fn connect_socket_mode(app_token: String, cancel: CancellationToken) -> Result<()> {
    let mut retry_delay = 1;
    loop {
        match establish_websocket(&app_token).await {
            Ok(ws) => {
                retry_delay = 1;
                if let Err(e) = handle_socket_messages(ws, cancel.clone()).await {
                    log error;
                    if cancel.is_cancelled() { return Ok(()); }
                }
            }
            Err(e) => {
                log error;
                sleep(retry_delay).await;
                retry_delay = min(retry_delay * 2, 30);
            }
        }
    }
}

async fn handle_socket_messages(ws: WebSocket, cancel: CancellationToken) -> Result<()> {
    loop {
        select! {
            _ = cancel.cancelled() => return Ok(()),
            msg = ws.next() => {
                let envelope = parse_envelope(msg)?;

                // Send ack within 3 seconds
                ws.send(json!({"envelope_id": envelope.id, "type": "ack"})).await?;

                // Process event
                match envelope.payload {
                    EventPayload::Message(msg) => {
                        let inbound = InboundMessage { ... };
                        host.on_inbound_message(inbound).await?;
                    }
                    EventPayload::Reaction(react) => { ... }
                    _ => {}
                }
            }
        }
    }
}
```

### 3.2 Discord Gateway Connection
```
async fn connect_gateway(token: String, cancel: CancellationToken) -> Result<()> {
    let (mut ws, gateway_url) = get_gateway_url().await?;
    let mut heartbeat_interval = None;
    let mut session_id = None;
    let mut seq = 0;

    loop {
        select! {
            _ = cancel.cancelled() => return Ok(()),

            _ = tick(heartbeat_interval.unwrap_or(45000)), if heartbeat_interval.is_some() => {
                ws.send(json!({"op": 1, "d": seq})).await?;
            }

            msg = ws.next() => {
                let payload = parse_gateway_payload(msg)?;
                seq = payload.s.unwrap_or(seq);

                match payload.op {
                    10 => { // Hello
                        heartbeat_interval = Some(payload.d.heartbeat_interval);
                        ws.send(identify_payload(&token)).await?;
                    }
                    0 => { // Dispatch
                        session_id = Some(payload.d.session_id);
                        match payload.t.as_str() {
                            "MESSAGE_CREATE" => {
                                let inbound = parse_message_event(payload.d)?;
                                host.on_inbound_message(inbound).await?;
                            }
                            "INTERACTION_CREATE" => { ... }
                            _ => {}
                        }
                    }
                    7 => { // Reconnect
                        return reconnect_gateway(session_id, seq, cancel);
                    }
                    _ => {}
                }
            }
        }
    }
}
```

### 3.3 Discord Rate Limiting
```
struct RateLimiter {
    buckets: HashMap<String, Bucket>,
    global_semaphore: Semaphore,
}

struct Bucket {
    remaining: AtomicU32,
    reset_at: AtomicU64,
    semaphore: Semaphore,
}

async fn rate_limited_request(route: &str, req: Request) -> Result<Response> {
    // Global rate limit
    let _global = global_semaphore.acquire().await?;

    // Per-bucket rate limit
    let bucket = get_or_create_bucket(route);
    let _permit = bucket.semaphore.acquire().await?;

    // Check if bucket is exhausted
    if bucket.remaining.load(Ordering::SeqCst) == 0 {
        let reset_at = bucket.reset_at.load(Ordering::SeqCst);
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now < reset_at {
            sleep(Duration::from_secs(reset_at - now)).await;
        }
    }

    // Make request
    let resp = client.execute(req).await?;

    // Update bucket state from headers
    if let Some(remaining) = resp.headers().get("X-RateLimit-Remaining") {
        bucket.remaining.store(remaining.parse()?, Ordering::SeqCst);
    }
    if let Some(reset) = resp.headers().get("X-RateLimit-Reset") {
        bucket.reset_at.store(reset.parse()?, Ordering::SeqCst);
    }

    // Handle 429
    if resp.status() == 429 {
        let retry_after = resp.headers().get("Retry-After")?.parse()?;
        sleep(Duration::from_millis(retry_after)).await;
        return rate_limited_request(route, req).await; // Retry
    }

    Ok(resp)
}
```

### 3.4 Slack Markdown Conversion
```
fn markdown_to_slack(md: &str) -> String {
    let mut result = String::new();
    let mut parser = MarkdownParser::new(md);

    for event in parser {
        match event {
            Event::Text(text) => result.push_str(text),
            Event::Strong(text) => result.push_str(&format!("*{}*", text)),
            Event::Emphasis(text) => result.push_str(&format!("_{}_", text)),
            Event::Code(code) => result.push_str(&format!("`{}`", code)),
            Event::CodeBlock(lang, code) => result.push_str(&format!("```\n{}\n```", code)),
            Event::Link(url, text) => result.push_str(&format!("<{}|{}>", url, text)),
            Event::UserMention(user_id) => result.push_str(&format!("<@{}>", user_id)),
            Event::ChannelMention(channel_id) => result.push_str(&format!("<#{}>", channel_id)),
            _ => {}
        }
    }

    result
}
```

---

## 4. Architecture

### 4.1 Slack Plugin Architecture
```
SlackChannel
├── config: SlackConfig { app_token, bot_token, signing_secret }
├── socket_mode: SocketModeClient
│   ├── ws: WebSocketStream<MaybeTlsStream<TcpStream>>
│   ├── cancel: CancellationToken
│   └── retry_state: RetryState
├── web_api: WebApiClient
│   ├── client: reqwest::Client
│   ├── bot_token: String
│   └── base_url: Url
├── markdown_converter: MarkdownConverter
└── running: AtomicBool

SocketModeClient
├── establish_connection() -> Result<WebSocket>
├── handle_messages(ws, host, cancel) -> Result<()>
├── send_ack(envelope_id) -> Result<()>
└── parse_event(payload) -> Result<SlackEvent>

WebApiClient
├── post_message(channel, text, thread_ts) -> Result<()>
├── upload_file(channels, file) -> Result<()>
├── get_conversation_info(channel) -> Result<ConversationInfo>
└── execute_request(req) -> Result<Response>

SignatureVerifier
├── verify(timestamp, body, signature, secret) -> Result<bool>
└── compute_signature(timestamp, body, secret) -> String

MarkdownConverter
├── to_slack_mrkdwn(markdown) -> String
└── parse_markdown(input) -> Vec<MarkdownEvent>
```

### 4.2 Discord Plugin Architecture
```
DiscordChannel
├── config: DiscordConfig { bot_token, public_key }
├── gateway: GatewayClient
│   ├── ws: WebSocketStream<MaybeTlsStream<TcpStream>>
│   ├── session_id: Option<String>
│   ├── seq: AtomicU64
│   ├── heartbeat_interval: Option<Duration>
│   └── cancel: CancellationToken
├── rest_api: RestApiClient
│   ├── client: reqwest::Client
│   ├── rate_limiter: RateLimiter
│   └── base_url: Url
├── markdown_converter: MarkdownConverter
└── running: AtomicBool

GatewayClient
├── connect(token, cancel) -> Result<()>
├── send_heartbeat(seq) -> Result<()>
├── send_identify(token) -> Result<()>
├── resume(session_id, seq) -> Result<()>
├── handle_dispatch(event, seq) -> Result<()>
└── parse_gateway_payload(msg) -> Result<GatewayPayload>

RestApiClient
├── create_message(channel_id, content, embeds) -> Result<Message>
├── add_reaction(channel_id, message_id, emoji) -> Result<()>
├── get_channel(channel_id) -> Result<Channel>
└── execute_rate_limited(route, req) -> Result<Response>

RateLimiter
├── buckets: HashMap<String, Bucket>
├── global_semaphore: Semaphore
├── acquire_permit(route) -> Result<Permit>
├── update_bucket(route, headers) -> Result<()>
└── handle_rate_limit(route, retry_after) -> Result<()>

Bucket
├── remaining: AtomicU32
├── reset_at: AtomicU64
└── semaphore: Semaphore

SignatureVerifier
├── verify_ed25519(timestamp, body, signature, public_key) -> Result<bool>
└── parse_public_key(hex) -> Result<VerifyingKey>
```

### 4.3 Dependency Graph
```
clawft-channels/slack
├── clawft-core (Channel trait, InboundMessage, OutboundMessage)
├── tokio-tungstenite 0.24 (WebSocket)
├── reqwest 0.12 (HTTP client)
├── hmac 0.12 (HMAC-SHA256)
├── sha2 0.10 (SHA256)
├── serde_json 1.0 (JSON parsing)
├── tokio-util 0.7 (CancellationToken)
└── tracing 0.1 (logging)

clawft-channels/discord
├── clawft-core (Channel trait, InboundMessage, OutboundMessage)
├── tokio-tungstenite 0.24 (WebSocket)
├── reqwest 0.12 (HTTP client)
├── ed25519-dalek 2.1 (Ed25519 signatures)
├── serde_json 1.0 (JSON parsing)
├── tokio-util 0.7 (CancellationToken)
└── tracing 0.1 (logging)
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 Slack Plugin Tests

#### Unit Tests (Red → Green → Refactor)
```rust
// tests/signature_tests.rs
#[test]
fn test_valid_signature_verification() {
    let secret = "signing_secret";
    let timestamp = "1234567890";
    let body = r#"{"type":"event_callback"}"#;
    let signature = compute_expected_signature(secret, timestamp, body);
    assert!(verify_signature(timestamp, body, &signature, secret).unwrap());
}

#[test]
fn test_invalid_signature_rejected() {
    assert!(!verify_signature("123", "body", "v0=bad", "secret").unwrap());
}

#[test]
fn test_expired_timestamp_rejected() {
    let old_timestamp = (SystemTime::now() - Duration::from_secs(400)).duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();
    assert!(verify_signature(&old_timestamp, "body", "v0=sig", "secret").is_err());
}

// tests/markdown_tests.rs
#[test]
fn test_markdown_to_slack_bold() {
    assert_eq!(markdown_to_slack("**bold**"), "*bold*");
}

#[test]
fn test_markdown_to_slack_italic() {
    assert_eq!(markdown_to_slack("*italic*"), "_italic_");
}

#[test]
fn test_markdown_to_slack_link() {
    assert_eq!(markdown_to_slack("[text](https://example.com)"), "<https://example.com|text>");
}

#[test]
fn test_markdown_to_slack_mention() {
    assert_eq!(markdown_to_slack("@U123"), "<@U123>");
}

#[test]
fn test_markdown_to_slack_code_block() {
    assert_eq!(markdown_to_slack("```\ncode\n```"), "```\ncode\n```");
}

// tests/web_api_tests.rs
#[tokio::test]
async fn test_post_message_success() {
    let mock_server = MockServer::start();
    mock_server.expect_post("/chat.postMessage")
        .with_json_body(json!({"channel": "C123", "text": "test"}))
        .respond_with(json!({"ok": true, "ts": "1234.5678"}));

    let client = WebApiClient::new("token", mock_server.url());
    let result = client.post_message("C123", "test", None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_post_message_rate_limit_retry() {
    let mock_server = MockServer::start();
    mock_server.expect_post("/chat.postMessage")
        .respond_with_status(429, json!({"ok": false, "error": "rate_limited"}))
        .times(1);
    mock_server.expect_post("/chat.postMessage")
        .respond_with(json!({"ok": true, "ts": "1234.5678"}))
        .times(1);

    let client = WebApiClient::new("token", mock_server.url());
    let result = client.post_message("C123", "test", None).await;
    assert!(result.is_ok());
}
```

#### Integration Tests
```rust
// tests/socket_mode_integration.rs
#[tokio::test]
async fn test_slack_channel_connect_and_receive_message() {
    let mock_ws_server = MockWebSocketServer::start();
    let slack = SlackChannel::new(config_with_mock_url(mock_ws_server.url()));
    let host = Arc::new(TestChannelHost::new());
    let cancel = CancellationToken::new();

    tokio::spawn(async move {
        slack.start(host.clone(), cancel.clone()).await.unwrap();
    });

    // Simulate server sending message event
    mock_ws_server.send_event(json!({
        "envelope_id": "abc",
        "type": "events_api",
        "payload": {
            "event": {
                "type": "message",
                "channel": "C123",
                "text": "test message"
            }
        }
    }));

    // Wait for host to receive message
    let received = timeout(Duration::from_secs(5), host.wait_for_message()).await.unwrap();
    assert_eq!(received.text, "test message");
}

#[tokio::test]
async fn test_slack_channel_reconnect_on_disconnect() {
    let mock_ws_server = MockWebSocketServer::with_disconnect();
    let slack = SlackChannel::new(config_with_mock_url(mock_ws_server.url()));
    let cancel = CancellationToken::new();

    // Should reconnect with exponential backoff
    let start = Instant::now();
    tokio::spawn(slack.start(host, cancel.clone()));

    // Wait for reconnection
    timeout(Duration::from_secs(10), mock_ws_server.wait_for_reconnect()).await.unwrap();
    assert!(start.elapsed() >= Duration::from_secs(1)); // At least 1 retry
}
```

### 5.2 Discord Plugin Tests

#### Unit Tests
```rust
// tests/rate_limit_tests.rs
#[tokio::test]
async fn test_rate_limiter_respects_bucket_limit() {
    let limiter = RateLimiter::new();
    limiter.update_bucket("POST /messages", 2, Instant::now() + Duration::from_secs(60));

    // First 2 requests succeed
    assert!(limiter.acquire_permit("POST /messages").await.is_ok());
    assert!(limiter.acquire_permit("POST /messages").await.is_ok());

    // Third request waits until reset
    let start = Instant::now();
    limiter.acquire_permit("POST /messages").await.unwrap();
    assert!(start.elapsed() >= Duration::from_secs(60));
}

#[tokio::test]
async fn test_rate_limiter_global_limit() {
    let limiter = RateLimiter::new_with_global_limit(2);

    // First 2 requests succeed
    assert!(limiter.acquire_global_permit().await.is_ok());
    assert!(limiter.acquire_global_permit().await.is_ok());

    // Third request waits for permit release
    let permit3 = limiter.acquire_global_permit();
    assert!(timeout(Duration::from_millis(100), permit3).await.is_err());
}

// tests/signature_tests.rs
#[test]
fn test_ed25519_signature_verification() {
    let keypair = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let public_key = keypair.verifying_key();

    let timestamp = "1234567890";
    let body = r#"{"type":1}"#;
    let message = format!("{}{}", timestamp, body);
    let signature = keypair.sign(message.as_bytes());

    assert!(verify_ed25519(timestamp, body, &hex::encode(signature), &hex::encode(public_key)).unwrap());
}

// tests/markdown_tests.rs
#[test]
fn test_discord_markdown_unchanged() {
    assert_eq!(markdown_to_discord("**bold**"), "**bold**");
    assert_eq!(markdown_to_discord("*italic*"), "*italic*");
}

#[test]
fn test_discord_markdown_spoiler() {
    assert_eq!(markdown_to_discord("||spoiler||"), "||spoiler||");
}
```

#### Integration Tests
```rust
// tests/gateway_integration.rs
#[tokio::test]
async fn test_discord_channel_connect_and_heartbeat() {
    let mock_gateway = MockGatewayServer::start();
    let discord = DiscordChannel::new(config_with_mock_url(mock_gateway.url()));
    let cancel = CancellationToken::new();

    tokio::spawn(discord.start(host, cancel.clone()));

    // Wait for identify
    let identify = timeout(Duration::from_secs(5), mock_gateway.wait_for_identify()).await.unwrap();
    assert_eq!(identify.token, "test_token");

    // Send Hello with heartbeat interval
    mock_gateway.send_payload(json!({
        "op": 10,
        "d": {"heartbeat_interval": 1000}
    }));

    // Wait for heartbeat
    timeout(Duration::from_millis(1500), mock_gateway.wait_for_heartbeat()).await.unwrap();
}

#[tokio::test]
async fn test_discord_channel_resume_after_disconnect() {
    let mock_gateway = MockGatewayServer::with_disconnect();
    let discord = DiscordChannel::new(config_with_mock_url(mock_gateway.url()));

    // Initial connection
    tokio::spawn(discord.start(host, cancel.clone()));
    mock_gateway.send_hello();
    mock_gateway.send_ready(json!({"session_id": "session123", "user": {...}}));

    // Disconnect
    mock_gateway.disconnect();

    // Wait for resume
    let resume = timeout(Duration::from_secs(5), mock_gateway.wait_for_resume()).await.unwrap();
    assert_eq!(resume.session_id, "session123");
}
```

### 5.3 Test Coverage Requirements
- **Unit test coverage**: >85% for all modules
- **Integration test coverage**: >70% for end-to-end flows
- **Critical paths**: 100% coverage for signature verification, rate limiting, reconnection logic

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [x] All unit tests passing (>85% coverage)
- [ ] All integration tests passing (>70% coverage)
- [ ] Slack Socket Mode connection tested with real Slack workspace
- [ ] Discord Gateway connection tested with real Discord bot
- [x] Markdown conversion validated against Slack/Discord rendering
- [ ] Rate limiting tested under load (Discord)
- [x] Signature verification tested with rotated keys
- [ ] Reconnection logic tested with network failures

### 6.2 Configuration Integration
- [x] SlackConfig added to clawft-core Config::channels
- [x] DiscordConfig added to clawft-core Config::channels
- [x] Environment variable loading for SLACK_APP_TOKEN, SLACK_BOT_TOKEN, DISCORD_BOT_TOKEN
- [x] Config validation (ensure required fields present)

### 6.3 Channel Registry Integration
- [x] SlackChannel registered in clawft-core ChannelManager
- [x] DiscordChannel registered in clawft-core ChannelManager
- [x] Dynamic channel loading based on config

### 6.4 MessageBus Integration
- [x] Slack events posted to MessageBus as InboundMessage
- [x] Discord events posted to MessageBus as InboundMessage
- [x] OutboundMessage routed to Slack via Web API
- [x] OutboundMessage routed to Discord via REST API

### 6.5 CLI Integration (Stream 2D dependency)
- [x] `weft channels status` shows Slack/Discord status
- [ ] `weft channels reload` restarts Slack/Discord connections

### 6.6 Documentation
- [ ] README.md for clawft-channels/slack with setup instructions
- [ ] README.md for clawft-channels/discord with setup instructions
- [ ] API documentation for Channel trait implementations
- [ ] Migration guide from nanobot/channels to clawft-channels

### 6.7 Performance Benchmarks
- [ ] Slack message latency <100ms (Socket Mode → Agent)
- [ ] Discord message latency <150ms (Gateway → Agent)
- [ ] Discord rate limiting prevents 429 errors under load
- [ ] Reconnection recovery time <5 seconds

### 6.8 Error Handling Validation
- [x] Slack invalid signature returns 401
- [x] Discord invalid signature returns 401
- [x] Network errors trigger reconnection
- [x] Rate limit errors trigger backoff
- [x] Malformed events logged and skipped

### 6.9 Final Review
- [ ] Code review by at least 2 reviewers
- [ ] Security audit of signature verification
- [ ] Performance profiling (flamegraph)
- [ ] Memory leak testing (valgrind/miri)
- [ ] Changelog updated with new features
- [ ] Version bumped (clawft-channels 0.2.0)

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockChannelHost, MockEnvironment, MockFileSystem};`
- **Import mocks from 1B**: `use clawft_core::test_utils::MockMessageBus;`
- **Use shared fixtures**: Load `tests/fixtures/config.json` (created by Stream 1A) for all config tests
- **Use shared fixtures**: Load `tests/fixtures/session.jsonl` for session integration tests

### Security Tests (Required)
- Webhook replay attack prevention (Slack: expired timestamp, Discord: invalid nonce)
- XSS prevention in message content (sanitize before rendering)
- Rate limit enforcement under load testing
- Signature verification with rotated/expired keys

### Coverage Target
- Unit test coverage: >= 80% (measured via `cargo-tarpaulin`)
- Critical paths (signature verification, rate limiting, reconnection): 100%

### Plugin Registry Feature Flag Tests
```rust
#[test]
fn test_slack_plugin_registered_with_feature() {
    #[cfg(feature = "slack")]
    {
        let registry = PluginRegistry::default();
        assert!(registry.has("slack"));
    }
}

#[test]
fn test_discord_plugin_registered_with_feature() {
    #[cfg(feature = "discord")]
    {
        let registry = PluginRegistry::default();
        assert!(registry.has("discord"));
    }
}
```

---

## Notes for Implementation Agent

1. **Read Python source files first** to understand exact event structures and API usage
2. **Use TDD London School**: Mock WebSocket/HTTP clients, write failing tests, then implement
3. **Parallel file operations**: Create all test files + implementation files in single message
4. **Signature verification is critical**: Must match Python implementations exactly
5. **Rate limiting is Discord-specific**: Slack uses simpler retry logic
6. **Markdown conversion**: Port existing slackify-markdown patterns
7. **WebSocket reconnection**: Use exponential backoff with max 30s delay
8. **Error types**: Define SlackError and DiscordError with From<reqwest::Error> impls
9. **Logging**: Use tracing for debug logs, structured fields for events
10. **Feature flags**: Consider `slack` and `discord` feature flags for optional builds
