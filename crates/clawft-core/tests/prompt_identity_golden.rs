//! M3 P0 — golden prompt-assembly regression harness (plan §T1).
//!
//! This is the byte-identical cutover gate for the M3 store-collapse. It records
//! a fixed multi-turn conversation, drives it through the **current** store-1
//! assembly path (`SessionManager` → `ContextBuilder::build_messages`), and
//! snapshots the assembled `Vec<LlmMessage>` as the golden. After the P3
//! sink-hydration cutover the same golden must still reproduce byte-for-byte.
//!
//! # The asymmetry this guards (design §1 / D2)
//!
//! Store 1 (SessionManager JSONL) receives only `user` + the **final**
//! `assistant` message of each exchange. The ConversationSink turn log — the
//! store P3 hydrates from — is a **superset**: it additionally holds every
//! tool-loop intermediate (assistant-with-`tool_calls` turns and `tool`
//! result turns). Any "read history from the sink" design must reproduce
//! store 1's exact subset or the assembled prompt changes. The recorded
//! conversation below therefore includes a tool-call exchange with
//! intermediates so the superset/subset asymmetry is exercised.
//!
//! # The provider seam (for P3)
//!
//! [`HistoryProvider`] is the swappable input source. Two impls:
//!   - [`SessionManagerProvider`] — the pre-collapse store-1 path (the
//!     golden's authoritative source).
//!   - [`SupersetFilterProvider`] — the **production** P3 path (wired at the
//!     cutover): it loads the recorded superset into a real
//!     [`InMemorySink`](clawft_core::agent::sink::InMemorySink) and hydrates
//!     via the production
//!     [`hydrate_session`](clawft_core::agent::hydrate::hydrate_session),
//!     which applies the production `hydrate::filter_store1_subset` (design D2).
//!
//! The equivalence test asserts both produce the identical assembled prompt —
//! i.e. read-time filtering of the superset by the production hydration path
//! reproduces the write-time subset. This test must stay green unchanged; the
//! `.snap` golden is never regenerated for the cutover.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;

use clawft_core::agent::context::{ContextBuilder, LlmMessage};
use clawft_core::agent::hydrate::hydrate_session;
use clawft_core::agent::memory::MemoryStore;
use clawft_core::agent::sink::{ConversationSink, InMemorySink, Turn};
use clawft_core::agent::skills::SkillsLoader;
use clawft_core::session::SessionManager;
use clawft_platform::Platform;
use clawft_types::config::{AgentDefaults, AgentsConfig};
use clawft_types::session::Session;

// ─────────────────────────────────────────────────────────────────────────
// Canonical recorded conversation
// ─────────────────────────────────────────────────────────────────────────

/// One turn as it exists in the sink **superset** turn log. `role`/`content`
/// are the only fields assembly consumes (see `build_messages_inner`, which
/// drops tool metadata and timestamps); the tool flags below exist purely so
/// [`filter_store1_subset`] can identify the intermediates store 1 never held.
struct RecordedTurn {
    role: &'static str,
    content: &'static str,
    /// Assistant turn that emitted `tool_calls` — a tool-loop intermediate.
    has_tool_calls: bool,
    /// A `tool` result turn — a tool-loop intermediate.
    is_tool_result: bool,
}

impl RecordedTurn {
    const fn user(content: &'static str) -> Self {
        Self { role: "user", content, has_tool_calls: false, is_tool_result: false }
    }
    /// A plain (final) assistant reply — the kind store 1 keeps.
    const fn assistant(content: &'static str) -> Self {
        Self { role: "assistant", content, has_tool_calls: false, is_tool_result: false }
    }
    /// An assistant turn that invokes tools — superset-only intermediate.
    const fn assistant_tool_call(content: &'static str) -> Self {
        Self { role: "assistant", content, has_tool_calls: true, is_tool_result: false }
    }
    /// A tool-result turn — superset-only intermediate.
    const fn tool_result(content: &'static str) -> Self {
        Self { role: "tool", content, has_tool_calls: false, is_tool_result: true }
    }

    /// Convert to a production [`Turn`] as the sink superset stores it, so the
    /// cutover test can drive the **real** `hydrate::filter_store1_subset`
    /// (which keys off `tool_calls` / `tool_call_id`, never the golden's local
    /// bool flags).
    fn to_sink_turn(&self, index: usize) -> Turn {
        Turn {
            turn_id: format!("golden-{index:04}"),
            role: self.role.to_string(),
            content: self.content.to_string(),
            tool_calls: self
                .has_tool_calls
                .then(|| vec![serde_json::json!({"id": "call-golden", "name": "tool"})]),
            tool_call_id: self.is_tool_result.then(|| "call-golden".to_string()),
            ts_ms: 1_700_000_000_000 + index as u64,
        }
    }
}

/// The full sink turn log (superset). Three exchanges: a plain Q&A, a
/// tool-call exchange with an assistant tool-call turn + a tool-result turn +
/// a final assistant reply, and a closing plain exchange.
fn recorded_superset() -> Vec<RecordedTurn> {
    vec![
        // Exchange 1 — plain.
        RecordedTurn::user("What is the capital of France?"),
        RecordedTurn::assistant("The capital of France is Paris."),
        // Exchange 2 — tool-call, with intermediates (superset-only).
        RecordedTurn::user("What's the weather in Paris right now?"),
        RecordedTurn::assistant_tool_call("Let me check the current conditions."),
        RecordedTurn::tool_result("Paris: 18°C, clear skies, wind 6 km/h."),
        RecordedTurn::assistant("It's currently 18°C and clear in Paris."),
        // Exchange 3 — plain close.
        RecordedTurn::user("Thanks, that's all."),
        RecordedTurn::assistant("You're welcome! Happy to help."),
    ]
}

/// The design's D2 hydration rule: reconstruct store 1's exact subset from the
/// sink superset — keep `user` turns and **final** `assistant` replies, drop
/// tool-loop intermediates (assistant-with-`tool_calls` and `tool` results).
///
/// P0's reference implementation. P3 replaces the call site with the
/// production filter; the golden bytes are the gate that it agrees.
fn filter_store1_subset(turns: &[RecordedTurn]) -> Vec<&RecordedTurn> {
    turns
        .iter()
        .filter(|t| !t.has_tool_calls && !t.is_tool_result)
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// History provider seam
// ─────────────────────────────────────────────────────────────────────────

/// Swappable source for the assembly `Session`. See module docs.
#[async_trait]
trait HistoryProvider {
    /// Build the in-memory `Session` that `ContextBuilder` reads, populated
    /// with store 1's message subset for `conv_id`.
    async fn hydrate(&self, conv_id: &str) -> Session;
}

/// The current store-1 path: a `SessionManager` fed exactly what
/// `handle_turn` appends today (user + final assistant, no intermediates),
/// hydrated via `get_or_create`. This is the golden's authoritative source.
struct SessionManagerProvider {
    manager: SessionManager<GoldenPlatform>,
}

impl SessionManagerProvider {
    /// Replay the store-1 subset of the recorded conversation into a fresh
    /// SessionManager, mirroring `AgentLoop::handle_turn`'s append pattern.
    async fn record(platform: Arc<GoldenPlatform>, conv_id: &str) -> Self {
        let manager = SessionManager::with_dir(
            platform,
            PathBuf::from("/golden/.clawft/workspace/sessions"),
        );
        let superset = recorded_superset();
        for turn in filter_store1_subset(&superset) {
            manager
                .append_turn(conv_id, turn.role, turn.content)
                .await
                .expect("append_turn into golden SessionManager");
        }
        Self { manager }
    }
}

#[async_trait]
impl HistoryProvider for SessionManagerProvider {
    async fn hydrate(&self, conv_id: &str) -> Session {
        self.manager
            .get_or_create(conv_id)
            .await
            .expect("get_or_create from golden SessionManager")
    }
}

/// The **production** P3 path (no longer a model): loads the recorded
/// superset — tool intermediates and all — into a real
/// [`InMemorySink`], then hydrates via the production
/// [`hydrate_session`], which applies the production
/// `hydrate::filter_store1_subset`. Read-time filtering of the superset
/// here must reproduce the write-time subset the SessionManager provider
/// produced. This is what wires the golden's cutover assertion to the
/// real production hydration path.
struct SupersetFilterProvider;

#[async_trait]
impl HistoryProvider for SupersetFilterProvider {
    async fn hydrate(&self, conv_id: &str) -> Session {
        let sink = InMemorySink::new();
        for (i, turn) in recorded_superset().iter().enumerate() {
            sink.append_turn(conv_id, turn.to_sink_turn(i))
                .await
                .expect("append recorded superset turn into golden sink");
        }
        // window == 0: hydrate the full superset, filter, and let the
        // assembly stage re-window (the P1 `ConversationSink::history`
        // contract) — exactly what `AgentLoop::handle_turn` does.
        hydrate_session(&sink, conv_id, 0).await
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Deterministic assembly
// ─────────────────────────────────────────────────────────────────────────

const CONV_ID: &str = "golden:store-collapse";

/// A `ContextBuilder` whose non-history inputs are fully deterministic:
/// empty memory, no skills, and a system prompt that falls to the hardcoded
/// default identity (the mock home has no bootstrap files). The only variable
/// is the conversation history supplied by the provider.
fn deterministic_context_builder(platform: Arc<GoldenPlatform>) -> ContextBuilder<GoldenPlatform> {
    let config = AgentsConfig {
        defaults: AgentDefaults {
            workspace: "/golden/workspace".into(),
            model: "golden-model".into(),
            max_tokens: 8192,
            temperature: 0.7,
            max_tool_iterations: 20,
            // Larger than the recorded history, so nothing is truncated.
            memory_window: 100,
        },
        ..AgentsConfig::default()
    };

    // Point memory + skills at paths the mock FS reports as missing → empty
    // memory context, no skill prompts.
    let memory = Arc::new(MemoryStore::with_paths(
        PathBuf::from("/golden/memory/MEMORY.md"),
        PathBuf::from("/golden/memory/HISTORY.md"),
        platform.clone(),
    ));
    let skills = Arc::new(SkillsLoader::with_dir(
        PathBuf::from("/golden/skills"),
        platform.clone(),
    ));

    ContextBuilder::new(config, memory, skills, platform)
}

/// Assemble the prompt for a provider: hydrate the session, then run the
/// frozen `ContextBuilder::build_messages` over it with no active skills.
async fn assemble<H: HistoryProvider>(platform: Arc<GoldenPlatform>, provider: &H) -> Vec<LlmMessage> {
    let session = provider.hydrate(CONV_ID).await;
    let builder = deterministic_context_builder(platform);
    builder.build_messages(&session, &[]).await
}

/// Serialize assembled messages to the golden byte form (pretty JSON, no
/// timestamps — `LlmMessage` skips `None` tool fields and carries no time).
fn serialize_golden(messages: &[LlmMessage]) -> String {
    serde_json::to_string_pretty(messages).expect("serialize assembled prompt")
}

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join("prompt_identity_golden.snap")
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

/// The golden gate: the current store-1 assembly must reproduce the committed
/// snapshot byte-for-byte. Regenerate intentionally with `UPDATE_GOLDEN=1`;
/// any unexpected diff is a stop-the-line event, not a snapshot refresh.
#[tokio::test]
async fn golden_prompt_assembly_matches_snapshot() {
    let platform = Arc::new(GoldenPlatform::new());
    let provider = SessionManagerProvider::record(platform.clone(), CONV_ID).await;
    let messages = assemble(platform, &provider).await;
    let actual = serialize_golden(&messages);

    let path = golden_path();
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("create snapshots dir");
        std::fs::write(&path, &actual).expect("write golden");
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "golden missing at {}; regenerate with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    assert_eq!(
        actual, expected,
        "assembled prompt drifted from golden {}. If intentional, regenerate with UPDATE_GOLDEN=1",
        path.display()
    );
}

/// The P3 cutover gate: read-time filtering of the sink superset must produce
/// the identical assembled prompt as the write-time store-1 subset. This is
/// the byte-identical guarantee the whole store collapse rests on.
#[tokio::test]
async fn store1_and_sink_hydrate_produce_identical_prompt() {
    let platform = Arc::new(GoldenPlatform::new());
    let store1 = SessionManagerProvider::record(platform.clone(), CONV_ID).await;

    let from_store1 = serialize_golden(&assemble(platform.clone(), &store1).await);
    let from_sink = serialize_golden(&assemble(platform, &SupersetFilterProvider).await);

    assert_eq!(
        from_store1, from_sink,
        "sink-hydrate+filter prompt diverged from the store-1 prompt — the D2 \
         subset filter does not reproduce store 1"
    );
}

/// Documents the D2 rule in isolation: the filter drops both intermediate
/// kinds (assistant-with-tool_calls, tool results) and keeps user + final
/// assistant, in order.
#[test]
fn superset_filter_drops_tool_intermediates() {
    let superset = recorded_superset();
    assert_eq!(superset.len(), 8, "recorded superset shape changed");

    let subset = filter_store1_subset(&superset);
    let shape: Vec<(&str, &str)> = subset.iter().map(|t| (t.role, t.content)).collect();
    assert_eq!(
        shape,
        vec![
            ("user", "What is the capital of France?"),
            ("assistant", "The capital of France is Paris."),
            ("user", "What's the weather in Paris right now?"),
            ("assistant", "It's currently 18°C and clear in Paris."),
            ("user", "Thanks, that's all."),
            ("assistant", "You're welcome! Happy to help."),
        ],
        "store-1 subset must drop tool intermediates and keep user + final assistant"
    );
    assert!(
        subset.iter().all(|t| !t.has_tool_calls && !t.is_tool_result),
        "no intermediate should survive the filter"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Hermetic in-memory platform (no real filesystem, no network)
// ─────────────────────────────────────────────────────────────────────────

/// In-memory filesystem. Absent files (memory, skills, bootstrap) read as
/// `NotFound`, giving an empty memory context and the default system prompt.
struct GoldenFs {
    files: StdMutex<HashMap<PathBuf, String>>,
    dirs: StdMutex<Vec<PathBuf>>,
}

impl GoldenFs {
    fn new() -> Self {
        Self { files: StdMutex::new(HashMap::new()), dirs: StdMutex::new(Vec::new()) }
    }
}

#[async_trait]
impl clawft_platform::fs::FileSystem for GoldenFs {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        self.files.lock().unwrap().get(path).cloned().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("not found: {}", path.display()))
        })
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            let mut dirs = self.dirs.lock().unwrap();
            if !dirs.contains(&parent.to_path_buf()) {
                dirs.push(parent.to_path_buf());
            }
        }
        self.files.lock().unwrap().insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let mut files = self.files.lock().unwrap();
        files.entry(path.to_path_buf()).or_default().push_str(content);
        Ok(())
    }

    async fn exists(&self, path: &Path) -> bool {
        if self.files.lock().unwrap().contains_key(path) {
            return true;
        }
        self.dirs.lock().unwrap().contains(&path.to_path_buf())
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        Ok(files
            .keys()
            .filter(|p| p.parent() == Some(path))
            .cloned()
            .collect())
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        let mut dirs = self.dirs.lock().unwrap();
        if !dirs.contains(&path.to_path_buf()) {
            dirs.push(path.to_path_buf());
        }
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.files.lock().unwrap().remove(path);
        Ok(())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        // A path with no bootstrap files on any real disk → default system prompt.
        Some(PathBuf::from("/golden-nonexistent-home"))
    }
}

struct GoldenEnv;

impl clawft_platform::env::Environment for GoldenEnv {
    fn get_var(&self, _name: &str) -> Option<String> {
        None
    }
    fn set_var(&self, _name: &str, _value: &str) {}
    fn remove_var(&self, _name: &str) {}
}

struct GoldenHttp;

#[async_trait]
impl clawft_platform::http::HttpClient for GoldenHttp {
    async fn request(
        &self,
        _method: &str,
        _url: &str,
        _headers: &HashMap<String, String>,
        _body: Option<&[u8]>,
    ) -> Result<clawft_platform::http::HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        Err("GoldenHttp makes no network calls".into())
    }
}

struct GoldenPlatform {
    fs: GoldenFs,
    env: GoldenEnv,
    http: GoldenHttp,
}

impl GoldenPlatform {
    fn new() -> Self {
        Self { fs: GoldenFs::new(), env: GoldenEnv, http: GoldenHttp }
    }
}

#[async_trait]
impl Platform for GoldenPlatform {
    fn http(&self) -> &dyn clawft_platform::http::HttpClient {
        &self.http
    }
    fn fs(&self) -> &dyn clawft_platform::fs::FileSystem {
        &self.fs
    }
    fn env(&self) -> &dyn clawft_platform::env::Environment {
        &self.env
    }
    fn process(&self) -> Option<&dyn clawft_platform::process::ProcessSpawner> {
        None
    }
}
