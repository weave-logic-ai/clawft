# SPARC Implementation Plan: Stream 2F - Security Hardening & Integration Testing

**Timeline**: Week 8-10
**Owned Crates**: `clawft-core` (security module + integration tests), `clawft-types` (error variant)
**Dependencies**: Phase 1 complete (SessionManager, AgentLoop, MemoryStore, ToolRegistry, AppContext)

---

## 1. Agent Instructions

### Source Files to Read (Current State)
```
clawft/crates/clawft-core/src/session.rs              # SessionManager — load_session, get_or_create, session_path
clawft/crates/clawft-core/src/agent/loop_core.rs       # AgentLoop — run_tool_loop, process_message
clawft/crates/clawft-core/src/agent/memory.rs          # MemoryStore — write_long_term, append_long_term
clawft/crates/clawft-core/src/agent/context.rs         # ContextBuilder — build_messages
clawft/crates/clawft-core/src/bootstrap.rs             # AppContext — new, into_agent_loop
clawft/crates/clawft-core/src/bus.rs                   # MessageBus — consume_inbound, dispatch_outbound
clawft/crates/clawft-core/src/tools/registry.rs        # ToolRegistry — register, execute, schemas
clawft/crates/clawft-tools/src/file_tools.rs           # validate_path — existing path containment pattern
clawft/crates/clawft-types/src/error.rs                # ClawftError — add SecurityViolation variant
clawft/crates/clawft-types/src/session.rs              # Session struct — key, messages, add_message
clawft/crates/clawft-types/src/config.rs               # Config, AgentsConfig, AgentDefaults
clawft/crates/clawft-types/src/event.rs                # InboundMessage, OutboundMessage
```

### Planning Documents (MUST READ)
```
repos/nanobot/.planning/02-technical-requirements.md   # Security boundary requirements
repos/nanobot/.planning/03-development-guide.md        # Stream timeline + testing strategy
repos/nanobot/.planning/sparc/00-orchestrator.md       # Cross-stream test fixtures, coverage targets
repos/nanobot/.planning/sparc/1b-core-engine.md        # Session, AgentLoop, MemoryStore spec
```

### Module Structure
```
clawft-core/
├── src/
│   ├── security.rs                 # NEW: validate_session_id, truncate_result, sanitize_content
│   ├── session.rs                  # MODIFIED: wire validate_session_id into load/create
│   ├── agent/
│   │   ├── loop_core.rs            # MODIFIED: wire truncate_result after tool execution
│   │   └── memory.rs              # MODIFIED: wire sanitize_content into write methods
│   └── lib.rs                      # MODIFIED: add pub mod security
├── tests/
│   └── security_tests.rs           # NEW: security boundary tests

clawft-types/
├── src/
│   └── error.rs                    # MODIFIED: add SecurityViolation variant

clawft/ (workspace root)
├── tests/
│   ├── phase1_integration.rs       # NEW: end-to-end Phase 1 integration tests
│   ├── phase2_integration.rs       # NEW: end-to-end Phase 2 integration tests
│   └── fixtures/
│       ├── session.jsonl           # NEW: sample session with 5 messages
│       ├── MEMORY.md               # NEW: sample memory file
│       └── tools/
│           └── read_file.json      # NEW: sample tool definition
```

---

## 2. Specification

### 2.1 Security API Requirements

#### `ClawftError::SecurityViolation` Variant
Add to `clawft-types/src/error.rs`:
```rust
/// A security boundary was violated (path traversal, oversized input, etc.)
#[error("security violation: {reason}")]
SecurityViolation {
    /// What policy was violated.
    reason: String,
},
```

#### `validate_session_id(id: &str) -> Result<(), ClawftError>`
- **Input**: Raw session key string (e.g. `"telegram:12345"`, `"slack:C0A1B2"`)
- **Reject with `ClawftError::SecurityViolation`** if:
  - `id` contains `..` (path traversal)
  - `id` contains `/` or `\` (directory separator)
  - `id` contains null bytes (`\0`)
  - `id.len() > 256` (excessive length)
  - `id.is_empty()` (empty key)
  - `id` contains ASCII control characters (bytes 0x00-0x1F except tab 0x09)
- **Allow**: alphanumeric, `:`, `-`, `_`, `.`, `@`, `+`, space, tab

#### `truncate_result(value: serde_json::Value, max_bytes: usize) -> serde_json::Value`
- **Input**: Arbitrary JSON `Value` from tool execution, byte limit (default: 65536)
- **Behavior**:
  - Serialize `value` to string; if `len <= max_bytes`, return unchanged
  - If `Value::String(s)` and oversized: truncate the string to fit within `max_bytes`, append `"... [truncated, original size: {n} bytes]"`
  - If `Value::Array(arr)` and oversized: take elements from the front until serialized size fits, append `{"_truncated": true, "original_count": n}`
  - Otherwise: serialize and truncate raw JSON string, wrap in `{"_truncated_json": "<prefix>...", "original_bytes": n}`
- **Return**: New `serde_json::Value` guaranteed to serialize to <= `max_bytes`

#### `sanitize_content(content: &str) -> String`
- **Input**: Arbitrary string from user/LLM/tool content
- **Behavior**:
  - Strip null bytes (`\0`)
  - Strip ASCII control characters (0x00-0x1F) except `\n` (0x0A), `\r` (0x0D), `\t` (0x09)
  - Strip DEL (0x7F)
  - Preserve all valid UTF-8 including emoji, CJK, RTL text
- **Return**: Cleaned string

### 2.2 Wiring Requirements

#### SessionManager Wiring
- `load_session(key)`: Call `validate_session_id(key)?` as the first statement
- `get_or_create(key)`: Call `validate_session_id(key)?` as the first statement (change return type to `Result<Session>`)
- `append_turn(key, ...)`: Call `validate_session_id(key)?` as the first statement
- `session_path(key)`: No change (validation happens before this is called)

#### AgentLoop Wiring
- `run_tool_loop(request)`: After each tool execution (`self.tools.execute(...)`), pass the result through `truncate_result(val, MAX_TOOL_RESULT_BYTES)` before appending to messages
- Constant: `const MAX_TOOL_RESULT_BYTES: usize = 65_536;` in `loop_core.rs`

#### MemoryStore Wiring
- `write_long_term(content)`: Apply `sanitize_content(content)` before writing
- `append_long_term(entry)`: Apply `sanitize_content(entry)` before appending
- `append_history(entry)`: Apply `sanitize_content(entry)` before appending

### 2.3 Security Test Requirements

All tests in `clawft-core/tests/security_tests.rs`:

| Test Name | Assertion |
|-----------|-----------|
| `test_session_id_no_path_traversal` | `validate_session_id("../../etc/passwd")` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_slash` | `validate_session_id("foo/bar")` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_backslash` | `validate_session_id("foo\\bar")` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_null_byte` | `validate_session_id("foo\0bar")` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_excessive_length` | `validate_session_id(&"a".repeat(257))` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_empty` | `validate_session_id("")` returns `Err(SecurityViolation)` |
| `test_session_id_rejects_control_chars` | `validate_session_id("foo\x01bar")` returns `Err(SecurityViolation)` |
| `test_session_id_accepts_valid` | `validate_session_id("telegram:12345")` returns `Ok(())` |
| `test_session_id_accepts_complex_valid` | `validate_session_id("slack:C0A1B2-thread_ts.1234")` returns `Ok(())` |
| `test_tool_result_size_limit` | `truncate_result(huge_json, 1024)` serializes to <= 1024 bytes |
| `test_tool_result_preserves_small` | `truncate_result(small_json, 65536)` returns identical value |
| `test_tool_result_truncates_string` | `truncate_result(Value::String(big_str), 100)` contains truncation marker |
| `test_tool_result_truncates_array` | `truncate_result(Value::Array(big_arr), 256)` has `_truncated: true` sentinel |
| `test_context_builder_escapes_user_input` | User input containing `<script>alert(1)</script>` appears verbatim (not executed) in assembled context |
| `test_memory_write_sanitizes_content` | Writing `"hello\0world\x01"` results in stored content `"helloworld"` |
| `test_sanitize_preserves_valid_utf8` | Emoji, CJK, accented characters pass through unchanged |
| `test_sanitize_strips_null_and_control` | Only null, control chars (except `\n\r\t`), and DEL are removed |
| `test_max_agent_iterations` | AgentLoop with `max_tool_iterations: 2` stops after 2 iterations and returns error |

### 2.4 Phase 1 Integration Test Requirements

All tests in `clawft/tests/phase1_integration.rs`:

| Test Name | What It Validates |
|-----------|-------------------|
| `test_config_loads_and_initializes_all_subsystems` | `Config::from_file("tests/fixtures/config.json")` produces valid `AppContext` with all subsystems initialized |
| `test_full_message_flow_mock_transport` | Send `InboundMessage` via bus sender -> `AgentLoop` processes it -> `OutboundMessage` appears on outbound receiver; uses mock transport returning canned text |
| `test_tool_execution_end_to_end` | Register a mock tool -> send message that triggers tool use via mock transport -> verify tool was executed and result incorporated into final response |
| `test_session_persistence_across_restarts` | Create `SessionManager` -> add messages -> save session -> create new `SessionManager` on same directory -> load session -> verify messages match |

### 2.5 Phase 2 Integration Test Requirements

All tests in `clawft/tests/phase2_integration.rs`:

| Test Name | What It Validates |
|-----------|-------------------|
| `test_slack_channel_factory_builds_from_config` | `SlackChannelFactory::from_config(&config)` returns a `SlackChannel` with correct tokens and base URL |
| `test_discord_channel_factory_builds_from_config` | `DiscordChannelFactory::from_config(&config)` returns a `DiscordChannel` with correct token and intents |
| `test_cron_service_triggers_message` | `CronService` with 1-second schedule fires job -> `InboundMessage` appears on bus within 3 seconds |
| `test_vector_memory_search_during_context_build` | Index 10 turns with distinct topics -> `ContextBuilder::build_messages` with relevant query -> verify relevant turns appear in assembled context |
| `test_mcp_client_discovers_tools` | `McpClient` with `MockTransport` -> `list_tools()` -> returns expected `Vec<ToolDefinition>` matching mock response |

### 2.6 Test Fixture Requirements

#### `tests/fixtures/session.jsonl`
```jsonl
{"_type":"metadata","created_at":"2025-06-15T10:00:00Z","updated_at":"2025-06-15T10:05:00Z","metadata":{},"last_consolidated":0}
{"role":"user","content":"Hello, can you help me?","timestamp":"2025-06-15T10:00:00Z"}
{"role":"assistant","content":"Of course! What do you need help with?","timestamp":"2025-06-15T10:00:01Z"}
{"role":"user","content":"What is the weather today?","timestamp":"2025-06-15T10:01:00Z"}
{"role":"assistant","content":"I do not have access to real-time weather data, but I can help you find it.","timestamp":"2025-06-15T10:01:01Z"}
{"role":"user","content":"Thanks, that is fine.","timestamp":"2025-06-15T10:02:00Z"}
```

#### `tests/fixtures/MEMORY.md`
```markdown
# Project Context

The user prefers concise answers. They work in the Pacific timezone.

# Key Decisions

- Use Rust for the rewrite
- MSRV is 1.85+
- Provider crate is clawft-llm
```

#### `tests/fixtures/tools/read_file.json`
```json
{
  "name": "read_file",
  "description": "Read the contents of a file within the workspace",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Relative path to the file within the workspace"
      }
    },
    "required": ["path"]
  }
}
```

### 2.7 Acceptance Criteria

- [ ] `ClawftError::SecurityViolation` variant added to `clawft-types`
- [ ] `security.rs` module added to `clawft-core` with all three public functions
- [ ] `validate_session_id` wired into `SessionManager::load_session`, `get_or_create`, `append_turn`
- [ ] `truncate_result` wired into `AgentLoop::run_tool_loop` after each tool execution
- [ ] `sanitize_content` wired into `MemoryStore::write_long_term`, `append_long_term`, `append_history`
- [ ] All 18 security tests pass in `clawft-core/tests/security_tests.rs`
- [ ] All 4 Phase 1 integration tests pass in `clawft/tests/phase1_integration.rs`
- [ ] All 5 Phase 2 integration tests pass in `clawft/tests/phase2_integration.rs`
- [ ] Test fixtures created in `tests/fixtures/`
- [ ] No regressions in existing `clawft-core` unit tests
- [ ] `cargo clippy --workspace` passes with no warnings
- [ ] Coverage >= 80% for `security.rs` module (measured via `cargo-tarpaulin`)

---

## 3. Pseudocode

### 3.1 Session ID Validation
```
fn validate_session_id(id: &str) -> Result<(), ClawftError> {
    if id.is_empty() {
        return Err(SecurityViolation("session id must not be empty"))
    }
    if id.len() > 256 {
        return Err(SecurityViolation("session id exceeds 256 bytes"))
    }
    if id.contains("..") {
        return Err(SecurityViolation("session id contains path traversal sequence '..'"))
    }
    if id.contains('/') || id.contains('\\') {
        return Err(SecurityViolation("session id contains directory separator"))
    }

    for byte in id.bytes() {
        if byte == 0x00 {
            return Err(SecurityViolation("session id contains null byte"))
        }
        // Control chars 0x01-0x08, 0x0B-0x0C, 0x0E-0x1F (allow \t=0x09, \n=0x0A, \r=0x0D)
        if byte <= 0x1F && byte != 0x09 {
            return Err(SecurityViolation("session id contains control character"))
        }
        if byte == 0x7F {
            return Err(SecurityViolation("session id contains DEL character"))
        }
    }

    Ok(())
}
```

### 3.2 Result Truncation
```
const DEFAULT_MAX_BYTES: usize = 65_536;

fn truncate_result(value: serde_json::Value, max_bytes: usize) -> serde_json::Value {
    let serialized = serde_json::to_string(&value).unwrap_or_default();

    if serialized.len() <= max_bytes {
        return value;
    }

    let original_len = serialized.len();

    match value {
        Value::String(ref s) => {
            // Truncate the string content, leaving room for the suffix and quotes
            let suffix = format!("... [truncated, original size: {} bytes]", original_len);
            let budget = max_bytes.saturating_sub(suffix.len() + 2); // 2 for JSON quotes
            let truncated = safe_utf8_truncate(s, budget);
            Value::String(format!("{}{}", truncated, suffix))
        }

        Value::Array(ref arr) => {
            // Take elements from front until we approach the limit
            let mut result = Vec::new();
            let mut running_size = 2; // for [ ]
            for elem in arr {
                let elem_json = serde_json::to_string(&elem).unwrap_or_default();
                let next_size = running_size + elem_json.len() + 1; // +1 for comma
                // Reserve ~100 bytes for the truncation sentinel
                if next_size + 100 > max_bytes {
                    break;
                }
                result.push(elem.clone());
                running_size = next_size;
            }
            result.push(json!({
                "_truncated": true,
                "original_count": arr.len()
            }));
            Value::Array(result)
        }

        _ => {
            // Generic: wrap truncated raw JSON in an object
            let prefix_budget = max_bytes.saturating_sub(100); // reserve for wrapper keys
            let prefix = &serialized[..prefix_budget.min(serialized.len())];
            json!({
                "_truncated_json": format!("{}...", prefix),
                "original_bytes": original_len
            })
        }
    }
}

fn safe_utf8_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk backward from max_bytes to find a char boundary
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
```

### 3.3 Content Sanitization
```
fn sanitize_content(content: &str) -> String {
    content.chars().filter(|&c| {
        match c {
            '\0'        => false,                        // null byte
            '\n' | '\r' | '\t' => true,                  // allowed whitespace
            c if c as u32 <= 0x1F => false,              // other control chars
            '\x7F'      => false,                        // DEL
            _           => true,                         // everything else (UTF-8 safe)
        }
    }).collect()
}
```

### 3.4 SessionManager Wiring
```
// In session.rs

pub async fn get_or_create(&self, key: &str) -> clawft_types::Result<Session> {
    crate::security::validate_session_id(key)?;   // <-- ADD THIS LINE

    // ... existing logic unchanged ...
}

pub async fn load_session(&self, key: &str) -> clawft_types::Result<Session> {
    crate::security::validate_session_id(key)?;   // <-- ADD THIS LINE

    let path = self.session_path(key);
    // ... existing logic unchanged ...
}

pub async fn append_turn(&self, key: &str, role: &str, content: &str) -> clawft_types::Result<()> {
    crate::security::validate_session_id(key)?;   // <-- ADD THIS LINE

    // ... existing logic unchanged ...
}
```

### 3.5 AgentLoop Wiring
```
// In loop_core.rs

const MAX_TOOL_RESULT_BYTES: usize = 65_536;

async fn run_tool_loop(&self, mut request: ChatRequest) -> clawft_types::Result<String> {
    let max_iterations = self.config.defaults.max_tool_iterations.max(1) as usize;

    for iteration in 0..max_iterations {
        let response = self.pipeline.complete(&request).await?;

        // ... extract tool_calls as before ...

        if tool_calls.is_empty() {
            // ... return text as before ...
        }

        for (id, name, input) in tool_calls {
            let result = self.tools.execute(&name, input.clone()).await;
            let result_json = match result {
                Ok(val) => {
                    // SECURITY: truncate oversized tool results
                    let truncated = crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
                    serde_json::to_string(&truncated).unwrap_or_default()
                }
                Err(e) => {
                    error!(tool = %name, error = %e, "tool execution failed");
                    format!("{{\"error\": \"{}\"}}", e)
                }
            };

            request.messages.push(LlmMessage {
                role: "tool".into(),
                content: result_json,
                tool_call_id: Some(id),
            });
        }
    }

    Err(ClawftError::Provider {
        message: format!("max tool iterations ({}) exceeded", max_iterations),
    })
}
```

### 3.6 MemoryStore Wiring
```
// In memory.rs

pub async fn write_long_term(&self, content: &str) -> Result<()> {
    let clean = crate::security::sanitize_content(content);   // <-- ADD
    self.platform
        .fs()
        .write_string(&self.memory_path, &clean)              // <-- use clean
        .await
        .map_err(ClawftError::Io)
}

pub async fn append_long_term(&self, entry: &str) -> Result<()> {
    let clean = crate::security::sanitize_content(entry);     // <-- ADD
    let formatted = format!("{}\n\n", clean.trim_end());      // <-- use clean
    self.platform
        .fs()
        .append_string(&self.memory_path, &formatted)
        .await
        .map_err(ClawftError::Io)
}

pub async fn append_history(&self, entry: &str) -> Result<()> {
    let clean = crate::security::sanitize_content(entry);     // <-- ADD
    let formatted = format!("{}\n\n", clean.trim_end());      // <-- use clean
    self.platform
        .fs()
        .append_string(&self.history_path, &formatted)
        .await
        .map_err(ClawftError::Io)
}
```

### 3.7 Phase 1 Integration Test Pseudocode
```
// tests/phase1_integration.rs

use clawft_core::bootstrap::AppContext;
use clawft_platform::NativePlatform;
use clawft_types::config::Config;
use clawft_types::event::InboundMessage;

#[tokio::test]
async fn test_config_loads_and_initializes_all_subsystems() {
    let config = Config::from_file("tests/fixtures/config.json").unwrap();
    let platform = Arc::new(NativePlatform::new());
    let ctx = AppContext::new(config, platform).await.unwrap();

    // Verify all subsystems are accessible
    assert!(!ctx.config().agents.defaults.model.is_empty());
    assert!(ctx.bus().inbound_sender().send(...).is_ok());
    assert!(ctx.tools().is_empty()); // no tools registered yet
    assert!(ctx.memory().memory_path().is_absolute());
    assert!(ctx.skills().skills_dir().is_absolute());
}

#[tokio::test]
async fn test_full_message_flow_mock_transport() {
    // 1. Build AppContext with mock transport that returns canned text
    let ctx = build_ctx_with_mock_transport("Hello from mock LLM").await;
    let tx = ctx.bus().inbound_sender();
    let mut rx = ctx.bus().outbound_receiver();

    // 2. Spawn agent loop in background
    let agent = ctx.into_agent_loop();
    tokio::spawn(async move { agent.run().await });

    // 3. Send inbound message
    tx.send(InboundMessage {
        channel: "test".into(),
        sender_id: "user1".into(),
        chat_id: "chat1".into(),
        content: "Hi there".into(),
        ..Default::default()
    }).unwrap();

    // 4. Receive outbound message
    let outbound = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert_eq!(outbound.content, "Hello from mock LLM");
    assert_eq!(outbound.channel, "test");
    assert_eq!(outbound.chat_id, "chat1");
}

#[tokio::test]
async fn test_tool_execution_end_to_end() {
    // 1. Create mock transport that returns ToolUse on first call, text on second
    // 2. Register a mock tool that returns {"result": "42"}
    // 3. Send inbound message
    // 4. Verify outbound includes tool result
    let ctx = build_ctx_with_tool_loop_transport().await;
    ctx.tools_mut().register(Arc::new(MockCalculatorTool));
    let tx = ctx.bus().inbound_sender();
    let mut rx = ctx.bus().outbound_receiver();

    let agent = ctx.into_agent_loop();
    tokio::spawn(async move { agent.run().await });

    tx.send(test_message("calculate 6*7")).unwrap();

    let outbound = timeout(Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    assert!(outbound.content.contains("42"));
}

#[tokio::test]
async fn test_session_persistence_across_restarts() {
    let tmp_dir = tempdir().unwrap();
    let platform = Arc::new(NativePlatform::new());

    // 1. Create session manager, add messages, save
    let mgr1 = SessionManager::with_dir(platform.clone(), tmp_dir.path().to_path_buf());
    let mut session = mgr1.get_or_create("test:persistence").await.unwrap();
    session.add_message("user", "first message", None);
    session.add_message("assistant", "first reply", None);
    mgr1.save_session(&session).await.unwrap();

    // 2. Create NEW session manager on same directory, load
    let mgr2 = SessionManager::with_dir(platform.clone(), tmp_dir.path().to_path_buf());
    let loaded = mgr2.load_session("test:persistence").await.unwrap();

    // 3. Verify messages survived restart
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.messages[0]["role"], "user");
    assert_eq!(loaded.messages[0]["content"], "first message");
    assert_eq!(loaded.messages[1]["role"], "assistant");
    assert_eq!(loaded.messages[1]["content"], "first reply");
}
```

### 3.8 Phase 2 Integration Test Pseudocode
```
// tests/phase2_integration.rs

// NOTE: Phase 2 tests depend on Stream 2A (channels), Stream 2B (vector memory),
// Stream 2C (services), and Stream 2E (MCP client). These tests should be
// #[cfg_attr(not(feature = "phase2"), ignore)] until dependencies are implemented.

#[tokio::test]
#[cfg_attr(not(feature = "phase2-channels"), ignore)]
async fn test_slack_channel_factory_builds_from_config() {
    let config = Config::from_file("tests/fixtures/config.json").unwrap();
    let factory = SlackChannelFactory::new();
    let channel = factory.build(&config.channels.slack).unwrap();

    assert_eq!(channel.name(), "slack");
    assert!(channel.is_running() == false); // not started yet
}

#[tokio::test]
#[cfg_attr(not(feature = "phase2-channels"), ignore)]
async fn test_discord_channel_factory_builds_from_config() {
    let config = Config::from_file("tests/fixtures/config.json").unwrap();
    let factory = DiscordChannelFactory::new();
    let channel = factory.build(&config.channels.discord).unwrap();

    assert_eq!(channel.name(), "discord");
    assert!(channel.is_running() == false);
}

#[tokio::test]
#[cfg_attr(not(feature = "phase2-services"), ignore)]
async fn test_cron_service_triggers_message() {
    let bus = Arc::new(MessageBus::new());
    let mut rx = bus.outbound_receiver(); // or inbound, depending on CronService design

    let cron = CronService::new(bus.clone());
    cron.add_job("test-job", "* * * * * *", || InboundMessage {
        channel: "cron".into(),
        sender_id: "system".into(),
        chat_id: "cron:test-job".into(),
        content: "scheduled ping".into(),
        ..Default::default()
    });

    cron.start().await;

    let msg = timeout(Duration::from_secs(3), bus.consume_inbound()).await.unwrap().unwrap();
    assert_eq!(msg.channel, "cron");
    assert_eq!(msg.content, "scheduled ping");

    cron.stop().await;
}

#[tokio::test]
#[cfg_attr(not(feature = "phase2-vector"), ignore)]
async fn test_vector_memory_search_during_context_build() {
    // 1. Create MemoryStore with vector index enabled
    // 2. Index 10 turns with distinct topics (weather, cooking, rust, python, ...)
    // 3. Build context with query "tell me about Rust"
    // 4. Verify the Rust-related turns appear in the assembled context
    // 5. Verify unrelated turns (cooking, weather) are NOT in context

    let store = VectorMemoryStore::new_in_memory();
    for (i, topic) in SAMPLE_TOPICS.iter().enumerate() {
        store.index_turn(i, topic).await;
    }

    let results = store.search("Rust programming language", 3).await;
    assert!(results.iter().any(|r| r.content.contains("Rust")));
    assert!(!results.iter().any(|r| r.content.contains("cooking")));
}

#[tokio::test]
#[cfg_attr(not(feature = "phase2-mcp"), ignore)]
async fn test_mcp_client_discovers_tools() {
    // 1. Create MockTransport that returns a canned tools/list response
    let transport = MockMcpTransport::new(vec![
        ToolDefinition {
            name: "read_file".into(),
            description: "Read file contents".into(),
            parameters: serde_json::from_str(include_str!("fixtures/tools/read_file.json")).unwrap(),
        },
    ]);

    // 2. Create McpClient with mock transport
    let client = McpClient::new(transport);
    let tools = client.list_tools().await.unwrap();

    // 3. Verify discovered tools
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].description, "Read file contents");
    assert!(tools[0].parameters["properties"]["path"]["type"] == "string");
}
```

---

## 4. Architecture

### 4.1 Security Module Architecture
```
clawft-core::security
├── validate_session_id(id: &str) -> Result<(), ClawftError>
│   ├── Checks: empty, length > 256, "..", "/", "\", null byte, control chars, DEL
│   └── Returns: Ok(()) or Err(ClawftError::SecurityViolation { reason })
│
├── truncate_result(value: Value, max_bytes: usize) -> Value
│   ├── Checks serialized byte length
│   ├── Handles: String truncation, Array prefix, generic JSON wrap
│   └── Returns: Value guaranteed <= max_bytes when serialized
│
├── sanitize_content(content: &str) -> String
│   ├── Strips: \0, control chars (except \n \r \t), DEL
│   └── Returns: cleaned String preserving valid UTF-8
│
└── safe_utf8_truncate(s: &str, max_bytes: usize) -> &str
    └── Finds nearest char boundary at or before max_bytes
```

### 4.2 Wiring Integration Points
```
SessionManager::get_or_create(key)
    └─> security::validate_session_id(key)?
        └─> [existing logic]

SessionManager::load_session(key)
    └─> security::validate_session_id(key)?
        └─> [existing logic]

SessionManager::append_turn(key, role, content)
    └─> security::validate_session_id(key)?
        └─> [existing logic]

AgentLoop::run_tool_loop(request)
    └─> for each tool_call:
        └─> tools.execute(name, input)
            └─> security::truncate_result(val, MAX_TOOL_RESULT_BYTES)   // NEW
                └─> serialize to result_json
                    └─> push to request.messages

MemoryStore::write_long_term(content)
    └─> security::sanitize_content(content)    // NEW
        └─> platform.fs().write_string(...)

MemoryStore::append_long_term(entry)
    └─> security::sanitize_content(entry)      // NEW
        └─> platform.fs().append_string(...)

MemoryStore::append_history(entry)
    └─> security::sanitize_content(entry)      // NEW
        └─> platform.fs().append_string(...)
```

### 4.3 Error Type Extension
```
clawft-types::error::ClawftError
├── Retry { source, attempts }         // existing
├── Timeout { operation }              // existing
├── Provider { message }               // existing
├── RateLimited { retry_after_ms }     // existing
├── ConfigInvalid { reason }           // existing
├── PluginLoadFailed { plugin }        // existing
├── Io(std::io::Error)                 // existing
├── Json(serde_json::Error)            // existing
├── Channel(String)                    // existing
└── SecurityViolation { reason }       // NEW — fatal, no retry
```

### 4.4 Test Architecture
```
Tests
├── clawft-core/tests/security_tests.rs          # Unit tests for security module
│   ├── Session ID validation (9 tests)
│   ├── Result truncation (4 tests)
│   ├── Content sanitization (3 tests)
│   └── Max iterations enforcement (1 test)
│   └── Context builder escaping (1 test)
│
├── clawft/tests/phase1_integration.rs            # Integration tests (Phase 1)
│   ├── Config → AppContext bootstrap (1 test)
│   ├── Full message flow (1 test)
│   ├── Tool execution E2E (1 test)
│   └── Session persistence (1 test)
│
├── clawft/tests/phase2_integration.rs            # Integration tests (Phase 2)
│   ├── Slack factory (1 test, gated)
│   ├── Discord factory (1 test, gated)
│   ├── Cron trigger (1 test, gated)
│   ├── Vector search (1 test, gated)
│   └── MCP discovery (1 test, gated)
│
└── clawft/tests/fixtures/
    ├── config.json                               # existing
    ├── config_invalid.json                       # existing
    ├── session.jsonl                             # NEW
    ├── MEMORY.md                                 # NEW
    └── tools/
        └── read_file.json                        # NEW
```

### 4.5 Dependency Graph
```
clawft-types (error.rs: +SecurityViolation variant)
    ^
    |
clawft-core (security.rs: new module)
    ├── session.rs       (imports security::validate_session_id)
    ├── agent/loop_core.rs (imports security::truncate_result)
    └── agent/memory.rs  (imports security::sanitize_content)

No new external dependencies required.
serde_json is already a dependency of clawft-core.
```

---

## 5. Refinement (TDD Test Plan)

### 5.1 Security Module Tests (Red -> Green -> Refactor)

#### Round 1: Session ID Validation (Red)
```rust
// clawft-core/tests/security_tests.rs
use clawft_core::security::{validate_session_id, truncate_result, sanitize_content};
use clawft_types::error::ClawftError;

#[test]
fn test_session_id_no_path_traversal() {
    let result = validate_session_id("../../etc/passwd");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClawftError::SecurityViolation { .. }));
    assert!(err.to_string().contains("path traversal"));
}

#[test]
fn test_session_id_rejects_slash() {
    let result = validate_session_id("foo/bar");
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("directory separator"));
}

#[test]
fn test_session_id_rejects_backslash() {
    let result = validate_session_id("foo\\bar");
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("directory separator"));
}

#[test]
fn test_session_id_rejects_null_byte() {
    let result = validate_session_id("foo\0bar");
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("null byte"));
}

#[test]
fn test_session_id_rejects_excessive_length() {
    let long_id = "a".repeat(257);
    let result = validate_session_id(&long_id);
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("256"));
}

#[test]
fn test_session_id_rejects_empty() {
    let result = validate_session_id("");
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_session_id_rejects_control_chars() {
    let result = validate_session_id("foo\x01bar");
    assert!(matches!(result, Err(ClawftError::SecurityViolation { .. })));
    assert!(result.unwrap_err().to_string().contains("control character"));
}

#[test]
fn test_session_id_accepts_valid() {
    assert!(validate_session_id("telegram:12345").is_ok());
    assert!(validate_session_id("slack:C0A1B2C3").is_ok());
    assert!(validate_session_id("discord:guild_123-channel_456").is_ok());
    assert!(validate_session_id("web:user@example.com").is_ok());
}

#[test]
fn test_session_id_accepts_complex_valid() {
    assert!(validate_session_id("slack:C0A1B2-thread_ts.1234.5678").is_ok());
    assert!(validate_session_id("a".repeat(256).as_str()).is_ok()); // exactly at limit
}
```

#### Round 2: Result Truncation (Red)
```rust
#[test]
fn test_tool_result_preserves_small() {
    let small = serde_json::json!({"key": "value"});
    let result = truncate_result(small.clone(), 65_536);
    assert_eq!(result, small);
}

#[test]
fn test_tool_result_size_limit() {
    // Create a 100 KB JSON value
    let huge_string = "x".repeat(100_000);
    let huge = serde_json::json!(huge_string);
    let result = truncate_result(huge, 1024);

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(serialized.len() <= 1024, "serialized len {} > 1024", serialized.len());
}

#[test]
fn test_tool_result_truncates_string() {
    let big_str = serde_json::Value::String("x".repeat(10_000));
    let result = truncate_result(big_str, 100);

    let s = result.as_str().unwrap();
    assert!(s.contains("truncated"));
    assert!(s.contains("original size"));

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(serialized.len() <= 100);
}

#[test]
fn test_tool_result_truncates_array() {
    let big_arr: Vec<serde_json::Value> = (0..1000)
        .map(|i| serde_json::json!({"index": i, "data": "x".repeat(50)}))
        .collect();
    let value = serde_json::Value::Array(big_arr);
    let result = truncate_result(value, 256);

    let arr = result.as_array().unwrap();
    // Last element should be the truncation sentinel
    let sentinel = arr.last().unwrap();
    assert_eq!(sentinel["_truncated"], true);
    assert_eq!(sentinel["original_count"], 1000);

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(serialized.len() <= 256);
}
```

#### Round 3: Content Sanitization (Red)
```rust
#[test]
fn test_sanitize_preserves_valid_utf8() {
    // Emoji, CJK, accented characters
    let input = "Hello world! Cafe\u{0301} \u{1F600} \u{4E16}\u{754C}";
    let result = sanitize_content(input);
    assert_eq!(result, input);
}

#[test]
fn test_sanitize_strips_null_and_control() {
    let input = "hello\0world\x01test\x02end";
    let result = sanitize_content(input);
    assert_eq!(result, "helloworldtestend");
}

#[test]
fn test_memory_write_sanitizes_content() {
    // Verify null bytes and control chars are stripped but newlines preserved
    let input = "line1\nline2\0hidden\x03removed\ttab_kept";
    let result = sanitize_content(input);
    assert_eq!(result, "line1\nline2hidden\ttab_kept"); // \0 and \x03 gone, \n and \t kept
}
```

#### Round 4: Integration with Existing Components (Red)
```rust
#[test]
fn test_context_builder_escapes_user_input() {
    // User input containing XSS-like content should appear verbatim
    let malicious = "<script>alert(1)</script>";
    let sanitized = sanitize_content(malicious);
    // sanitize_content does NOT strip HTML tags (that is not its job);
    // it only strips control characters. HTML escaping happens at the
    // channel layer (Slack mrkdwn conversion, Discord markdown, etc.)
    assert_eq!(sanitized, malicious);
}

#[tokio::test]
async fn test_max_agent_iterations() {
    // Build an agent loop with max_tool_iterations = 2 and a mock transport
    // that ALWAYS returns a tool call. After 2 iterations the loop should
    // return Err with "max tool iterations (2) exceeded".

    let config = AgentsConfig {
        defaults: AgentDefaults {
            max_tool_iterations: 2,
            ..Default::default()
        },
    };

    let transport = AlwaysToolCallTransport::new();
    let tools = ToolRegistry::new();
    tools.register(Arc::new(NoopTool));

    let request = ChatRequest {
        messages: vec![LlmMessage {
            role: "user".into(),
            content: "trigger tool".into(),
            tool_call_id: None,
        }],
        tools: tools.schemas(),
        model: Some("test".into()),
        max_tokens: Some(4096),
        temperature: Some(0.5),
    };

    // Directly test run_tool_loop (need to make it pub(crate) for test)
    let result = agent_loop.run_tool_loop(request).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("max tool iterations"));
    assert!(err_msg.contains("2"));
}
```

### 5.2 Implementation Order (Green)

1. **Add `SecurityViolation` variant** to `clawft-types/src/error.rs`
2. **Create `clawft-core/src/security.rs`** with `validate_session_id`, `truncate_result`, `sanitize_content`
3. **Add `pub mod security;`** to `clawft-core/src/lib.rs`
4. **Wire `validate_session_id`** into `session.rs` (3 call sites)
5. **Wire `truncate_result`** into `loop_core.rs` (1 call site)
6. **Wire `sanitize_content`** into `memory.rs` (3 call sites)
7. **Run all security tests** -- verify 18/18 pass
8. **Create test fixtures** in `tests/fixtures/`
9. **Create `phase1_integration.rs`** -- verify 4/4 pass
10. **Create `phase2_integration.rs`** (all gated with `#[ignore]` or feature flags) -- verify compiles

### 5.3 Refactoring Pass

After all tests pass:
- Extract `MAX_SESSION_ID_LEN = 256` as a public constant in `security.rs`
- Extract `MAX_TOOL_RESULT_BYTES = 65_536` as a public constant in `security.rs`
- Add `#[inline]` to `sanitize_content` (hot path)
- Add `#[must_use]` to `truncate_result` and `sanitize_content`
- Ensure `cargo clippy --workspace` produces zero warnings
- Run `cargo tarpaulin -p clawft-core --out Html` and verify `security.rs` >= 80% coverage

---

## 6. Completion (Integration Checklist)

### 6.1 Pre-Integration Validation
- [ ] All 18 security tests passing
- [ ] All 4 Phase 1 integration tests passing
- [ ] Phase 2 integration tests compile (gated with `#[ignore]`)
- [ ] `cargo test --workspace` passes with no regressions
- [ ] `cargo clippy --workspace` clean
- [ ] Coverage >= 80% for `security.rs`

### 6.2 Error Type Integration
- [ ] `ClawftError::SecurityViolation` variant added to `clawft-types/src/error.rs`
- [ ] `#[non_exhaustive]` preserved on `ClawftError` enum
- [ ] Display message format: `"security violation: {reason}"`
- [ ] Downstream crates compile without changes (non-exhaustive ensures this)

### 6.3 Security Module Integration
- [ ] `clawft-core/src/security.rs` created with all 3 public functions + 1 helper
- [ ] `pub mod security;` added to `clawft-core/src/lib.rs`
- [ ] Module is `#[cfg(not(target_arch = "wasm32"))]`-free (works in WASM too)
- [ ] No new external dependencies added

### 6.4 SessionManager Wiring
- [ ] `validate_session_id` called at top of `load_session`
- [ ] `validate_session_id` called at top of `get_or_create` (return type changed to `Result<Session>`)
- [ ] `validate_session_id` called at top of `append_turn`
- [ ] All existing `SessionManager` tests still pass
- [ ] `process_message` in `loop_core.rs` updated if `get_or_create` now returns `Result`

### 6.5 AgentLoop Wiring
- [ ] `MAX_TOOL_RESULT_BYTES` constant defined
- [ ] `truncate_result` called after each successful `tools.execute()` in `run_tool_loop`
- [ ] Existing tool loop tests still pass
- [ ] Error branch (`Err(e)`) in tool execution is NOT truncated (error messages stay as-is)

### 6.6 MemoryStore Wiring
- [ ] `sanitize_content` called in `write_long_term` before `write_string`
- [ ] `sanitize_content` called in `append_long_term` before `append_string`
- [ ] `sanitize_content` called in `append_history` before `append_string`
- [ ] Existing `MemoryStore` tests still pass

### 6.7 Test Fixtures
- [ ] `tests/fixtures/session.jsonl` created (6 lines: 1 metadata + 5 messages)
- [ ] `tests/fixtures/MEMORY.md` created (sample memory content)
- [ ] `tests/fixtures/tools/read_file.json` created (valid tool definition)
- [ ] Fixtures loadable from both unit and integration tests

### 6.8 Phase 2 Test Gating
- [ ] Phase 2 tests use `#[cfg_attr(not(feature = "phase2-channels"), ignore)]` or equivalent
- [ ] Feature flags documented in workspace `Cargo.toml`
- [ ] Tests compile even when gated features are not enabled

### 6.9 Documentation
- [ ] `security.rs` has module-level doc comment explaining threat model
- [ ] Each public function has `///` doc with `# Errors` section
- [ ] `# Security` section added to `clawft-core` crate-level docs (in `lib.rs`)

### 6.10 Performance Validation
- [ ] `sanitize_content` handles 1 MB input in < 10ms
- [ ] `truncate_result` handles 1 MB JSON in < 10ms
- [ ] `validate_session_id` is O(n) in id length (no allocation on success path)

### 6.11 Final Review
- [ ] No hardcoded secrets in test code
- [ ] Test fixtures do not contain real API keys
- [ ] `cargo test --workspace` passes on CI
- [ ] Version bump if warranted (patch: security hardening)

---

## Cross-Stream Integration Requirements

### Reuse Stream 1 Test Infrastructure
- **Import mocks from 1A**: `use clawft_platform::test_utils::{MockFileSystem, MockEnvironment};`
- **Import mocks from 1B**: `use clawft_core::test_utils::MockMessageBus;` (if available)
- **Use shared fixtures**: Load `tests/fixtures/config.json` (created by Stream 1A) for config tests
- **Use shared fixtures**: Load `tests/fixtures/session.jsonl` for session persistence tests

### Security Tests (Required)
- Session ID path traversal prevention (`../../etc/passwd`, `foo/bar`, `foo\bar`)
- Null byte injection prevention (`"foo\0bar"`)
- Tool result size bomb prevention (100 KB+ results truncated to 64 KB)
- Memory content sanitization (control characters stripped)
- Agent loop iteration cap enforcement (`max_tool_iterations`)

### Coverage Target
- Unit test coverage: >= 80% for `security.rs` (measured via `cargo-tarpaulin`)
- Integration test coverage: >= 70% for wired call sites
- Critical paths (validation, truncation, sanitization): 100%
