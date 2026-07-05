//! Integration (M4 headline): an agent spawns a subagent from inside a turn and
//! the result flows back into the conversation as first-class ECC state.
//!
//! This is the text analogue of M2's `live_native_talk_session` and the M4
//! acceptance test (plan §2.4). It drives the seam the isolated tests each miss:
//!
//!   - `clawft-service-agent/tests/witness_chain.rs` drives `AgentService::dispatch`
//!     through a stub LLM + real gate/chain — but with NO forest/anchor, so turns
//!     never commit.
//!   - `clawft-service-agent/tests/text_ecc_commit.rs` drives the `KernelTurnAnchor`
//!     → `SessionTier` → `TalkModeLoop` forest commit — but by calling `anchor_turn`
//!     by hand, with NO LLM loop and NO spawn.
//!
//! M4 needs BOTH at once: the LLM emits an `agent_spawn` tool call inside the
//! parent's `run_tool_loop`, the real `DaemonSubagentSpawner` dispatches a child
//! conversation on the SAME `AgentService`, and every parent + child turn commits
//! Frontier→Committed on the ONE kernel-global forest — with the spawn tree drawn
//! as `TriggeredBy` / `EvidenceFor` cross-refs. The wiring here mirrors the daemon
//! boot (`daemon.rs` ~985-1218): construct the spawner + `SpawnRegistry`, register
//! the four subagent tools via `register_all(Some(spawner))`, build the agent loop
//! with an anchor-backed sink, then late-wire the service back-reference (the
//! Arc-cycle break, design D2). Only the LLM transport is stubbed; the spawner,
//! tools, gate, forest, chain, tier, and multiplexed loop are all real.
//!
//! Hermetic: a Mock embedder, an in-memory `ChainManager`, and a directly-`tick()`ed
//! `TalkModeLoop`. No daemon socket, no Hermes, no substrate JSONL.
//!
//! ## Spawn-tree edge rooting (turn-level, task #31)
//!
//! The `TriggeredBy` / `EvidenceFor` edges root at a REAL committed parent turn
//! node, not the synthetic per-conversation `conv_anchor`. `run_tool_loop` installs
//! the `SpawnContext` with `turn_uid: None` (the parent turn's universal id is
//! minted at the kernel/daemon layer, out of clawft-core's reach), but the
//! daemon-layer spawner closes the gap: `DaemonSubagentSpawner::resolve_parent_uid`
//! reaches the `SessionTier` through its `Weak<AgentService>` and reads the parent
//! conversation's latest *indexed* turn uid (`SessionTier::latest_turn_uid`) — which
//! is why the test attaches the tier to the `AgentService` exactly as the daemon.
//!
//! One subtlety this test pins down: `SessionTier::index_turn` skips empty-text
//! turns, so a pure `tool_use` assistant turn (no narration — the shape this stub
//! emits, and a very common real-model shape) is not indexed and does not become
//! `latest_turn_uid`. The edges therefore root at the last non-empty indexed turn —
//! here the USER turn. When a model narrates alongside the call, the target is the
//! assistant tool-call turn instead. Either way the invariant the test asserts holds:
//! a real committed turn node, never `conv_anchor`, Follows-linked into the lineage.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;

use clawft_core::agent::context::ContextBuilder;
use clawft_core::agent::loop_core::AgentLoop;
use clawft_core::agent::memory::MemoryStore;
use clawft_core::agent::sink::{ConversationSink, Turn};
use clawft_core::agent::skills::SkillsLoader;
use clawft_core::agent::spawn::SubagentSpawner;
use clawft_core::bus::MessageBus;
use clawft_core::pipeline::permissions::PermissionResolver;
use clawft_core::pipeline::traits::{
    AssembledContext, ChatRequest, ContextAssembler, LearningBackend, LearningSignal, LlmTransport,
    ModelRouter, Pipeline, PipelineRegistry, QualityScore, QualityScorer, ResponseOutcome,
    RoutingDecision, TaskClassifier, TaskProfile, TaskType, Trajectory, TransportRequest,
};
use clawft_core::session::SessionManager;
use clawft_core::tools::registry::ToolRegistry;

use clawft_kernel::chain::ChainManager;
use clawft_kernel::context_graft::NodeState;
use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use clawft_kernel::gate::{GateBackend, GovernanceGate};
use clawft_kernel::{
    CausalEdgeType, CausalGraph, CognitiveTick, CognitiveTickConfig, CrossRefStore, CrossRefType,
    ImpulseQueue, StructureTag, TalkModeConfig, TalkModeLoop, UniversalNodeId, ViewResolver,
};

use clawft_platform::NativePlatform;

use clawft_service_agent::{
    AgentChatMessage, AgentChatParams, AgentService, DaemonSubagentSpawner, KernelEffectGate,
    KernelTurnAnchor, SessionTier, SpawnRegistry, SubagentConfig, SubagentForest, TurnAnchor,
};

use clawft_types::config::{AgentDefaults, AgentsConfig};
use clawft_types::provider::{ContentBlock, LlmResponse, StopReason, Usage};

const DIMS: usize = 64;

/// The parent user prompt. Distinct from the child goal so the stub transport
/// can tell the parent turn from the child turn by the last user message.
const PARENT_PROMPT: &str = "Please delegate the arithmetic to a subagent.";
/// The goal the parent hands the subagent — becomes the child's first user turn.
const CHILD_GOAL: &str = "answer 2+2";
/// The child's answer.
const CHILD_ANSWER: &str = "4";
/// The parent's final text once the tool result comes back.
const PARENT_FINAL: &str = "The subagent finished the task.";

// ── Stub pipeline stages (mirror witness_chain.rs / loop_core::tests) ─────────

struct StubClassifier;
impl TaskClassifier for StubClassifier {
    fn classify(&self, _request: &ChatRequest) -> TaskProfile {
        TaskProfile {
            task_type: TaskType::Chat,
            complexity: 0.3,
            keywords: vec![],
        }
    }
}

struct StubRouter;
#[async_trait]
impl ModelRouter for StubRouter {
    async fn route(&self, _request: &ChatRequest, _profile: &TaskProfile) -> RoutingDecision {
        RoutingDecision {
            provider: "test".into(),
            model: "test-model".into(),
            reason: "stub".into(),
            ..Default::default()
        }
    }
    fn update(&self, _d: &RoutingDecision, _o: &ResponseOutcome) {}
}

struct StubAssembler;
#[async_trait]
impl ContextAssembler for StubAssembler {
    async fn assemble(&self, request: &ChatRequest, _profile: &TaskProfile) -> AssembledContext {
        AssembledContext {
            messages: request.messages.clone(),
            token_estimate: 100,
            truncated: false,
        }
    }
}

struct StubScorer;
impl QualityScorer for StubScorer {
    fn score(&self, _req: &ChatRequest, _resp: &LlmResponse) -> QualityScore {
        QualityScore {
            overall: 1.0,
            relevance: 1.0,
            coherence: 1.0,
        }
    }
}

struct StubLearner;
impl LearningBackend for StubLearner {
    fn record(&self, _t: &Trajectory) {}
    fn adapt(&self, _s: &LearningSignal) {}
}

// ── Content-keyed stub transport ─────────────────────────────────────────────
//
// Parent and child ride the SAME AgentService / AgentLoop / pipeline, so the one
// transport must serve both. It keys off the messages it is handed:
//
//   - child turn (last user message is the goal)  → answer "4"
//   - parent, tool result already present         → final text
//   - parent, first pass                          → emit the agent_spawn tool call
//
// With `await: true` the calls are strictly ordered — parent-spawn, child-answer,
// parent-final — but keying on content (not a counter) keeps the stub robust to
// any reordering.
struct SpawnThenAnswerTransport;

#[async_trait]
impl LlmTransport for SpawnThenAnswerTransport {
    async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
        let last_user = request
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        // Child conversation: its only user turn is the spawn goal.
        if last_user == CHILD_GOAL {
            return Ok(text_response("child-answer", CHILD_ANSWER));
        }

        // Parent second pass: the agent_spawn tool result (role == "tool") is in
        // the transcript. Weave a final answer.
        let saw_tool_result = request.messages.iter().any(|m| m.role == "tool");
        if saw_tool_result {
            return Ok(text_response("parent-final", PARENT_FINAL));
        }

        // Parent first pass: kick off the subagent.
        Ok(LlmResponse {
            id: "parent-spawn".into(),
            content: vec![ContentBlock::ToolUse {
                id: "call-spawn-1".into(),
                name: "agent_spawn".into(),
                input: serde_json::json!({ "goal": CHILD_GOAL, "await": true }),
            }],
            stop_reason: StopReason::ToolUse,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 0,
            },
            metadata: HashMap::new(),
        })
    }
}

fn text_response(id: &str, text: &str) -> LlmResponse {
    LlmResponse {
        id: id.into(),
        content: vec![ContentBlock::Text { text: text.into() }],
        stop_reason: StopReason::EndTurn,
        usage: Usage {
            input_tokens: 8,
            output_tokens: 4,
            total_tokens: 0,
        },
        metadata: HashMap::new(),
    }
}

// ── Anchor-backed conversation sink ──────────────────────────────────────────
//
// The daemon's `SubstrateConversationSink` writes JSONL AND anchors every turn.
// Here we skip the substrate JSONL and keep only the anchoring — the loop's
// `append_turn` drives `KernelTurnAnchor::anchor_turn`, exactly the path M2's
// text_ecc_commit test exercises by hand, so parent + child turns dual-write the
// forest and queue an `EndOfUtterance` for the hosted loop to commit.
struct AnchorSink {
    anchor: Arc<KernelTurnAnchor>,
}

#[async_trait]
impl ConversationSink for AnchorSink {
    async fn lock_conversation(&self, _conv_id: &str) {
        // AgentService already serialises per-conv via its own Mutex map.
    }

    async fn append_turn(&self, conv_id: &str, turn: Turn) -> Result<(), String> {
        self.anchor.anchor_turn(conv_id, &turn.turn_id, &turn).await;
        Ok(())
    }

    async fn publish_status(
        &self,
        _conv_id: &str,
        _status: &str,
        _payload: serde_json::Value,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn load_history(&self, _conv_id: &str) -> Result<Vec<Turn>, String> {
        Ok(Vec::new())
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("clawft_m4_spawn_{prefix}_{pid}_{id}"))
}

fn test_config() -> AgentsConfig {
    AgentsConfig {
        defaults: AgentDefaults {
            workspace: "~/.clawft/workspace".into(),
            model: "test-model".into(),
            max_tokens: 4096,
            temperature: 0.5,
            max_tool_iterations: 10,
            memory_window: 50,
        },
        ..AgentsConfig::default()
    }
}

fn make_pipeline() -> PipelineRegistry {
    PipelineRegistry::new(Pipeline {
        classifier: Arc::new(StubClassifier),
        router: Arc::new(StubRouter),
        assembler: Arc::new(StubAssembler),
        transport: Arc::new(SpawnThenAnswerTransport),
        scorer: Arc::new(StubScorer),
        learner: Arc::new(StubLearner),
    })
}

/// A forest-joined `SessionTier` + hosted `TalkModeLoop`, wired exactly as the
/// daemon's `tier_owns_forest` boot (mirrors text_ecc_commit::wire_tier) but also
/// returning the `crossrefs` handle the spawner + assertions need.
#[allow(clippy::type_complexity)] // test wiring bundle — mirrors the daemon boot handles
fn wire_forest() -> (
    Arc<SessionTier>,
    Arc<ChainManager>,
    Arc<CausalGraph>,
    Arc<CrossRefStore>,
    Arc<TalkModeLoop>,
) {
    let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(DIMS));
    let chain = Arc::new(ChainManager::new(0, 1000));
    let causal = Arc::new(CausalGraph::new());
    let crossrefs = Arc::new(CrossRefStore::new());

    let tier = Arc::new(
        SessionTier::new(embedder, chain.clone(), None)
            .with_forest(causal.clone(), crossrefs.clone()),
    );

    let resolver = SessionTier::weak_view_resolver(&tier);
    let tick = Arc::new(CognitiveTick::new(CognitiveTickConfig::default()));
    let talk_loop = Arc::new(TalkModeLoop::new(
        Arc::new(ImpulseQueue::new()),
        causal.clone(),
        crossrefs.clone(),
        resolver,
        tick,
        TalkModeConfig::default(),
    ));
    tier.set_talk_loop(talk_loop.clone());

    (tier, chain, causal, crossrefs, talk_loop)
}

/// The synthetic per-conversation anchor uid the spawner's `parent_target_uid`
/// falls back to when `parent_turn_uid` is `None` (the shipped M4 rooting — see
/// module doc). Must exactly mirror `subagent.rs::parent_target_uid`.
fn conv_anchor_uid(parent_conv: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        parent_conv.as_bytes(),
        0,
        b"",
        b"conv_anchor",
    )
}

/// The causal-graph node id whose dual-write stamped `chain_seq` (the tier stamps
/// `metadata.chain_seq` on every turn node). Mirrors text_ecc_commit's helper.
fn node_for_seq(causal: &CausalGraph, chain_seq: u64) -> clawft_kernel::causal::NodeId {
    causal
        .node_ids()
        .into_iter()
        .find(|n| {
            causal
                .get_node(*n)
                .and_then(|node| node.metadata.get("chain_seq").and_then(|v| v.as_u64()))
                == Some(chain_seq)
        })
        .unwrap_or_else(|| panic!("no causal node for chain_seq {chain_seq}"))
}

fn params(conv_id: &str, content: &str) -> AgentChatParams {
    AgentChatParams {
        messages: vec![AgentChatMessage {
            role: "user".into(),
            content: content.into(),
        }],
        temperature: None,
        max_tokens: None,
        conv_id: conv_id.into(),
        metadata: None,
    }
}

/// Drain every queued `EndOfUtterance` so all anchored turns commit
/// Frontier→Committed. In the daemon the hosted loop ticks on a timer; here we
/// pump it by hand until the impulse queue is empty.
fn drain_commits(talk_loop: &TalkModeLoop) -> usize {
    let mut committed = 0;
    for _ in 0..64 {
        if talk_loop.impulses().is_empty() {
            break;
        }
        committed += talk_loop.tick().commits;
    }
    committed
}

// ── The headline test ────────────────────────────────────────────────────────

#[tokio::test]
async fn agent_spawns_subagent_result_commits_into_parent() {
    const PARENT_CONV: &str = "m4-parent";

    // ── Forest + hosted loop (kernel-global, shared by parent AND child) ──────
    let (tier, chain, causal, crossrefs, talk_loop) = wire_forest();

    // Anchor wired the daemon `tier_owns_forest` way: chain on, causal None on the
    // anchor (the tier dual-writes the forest), session tier attached.
    let anchor =
        Arc::new(KernelTurnAnchor::new(Some(chain.clone()), None, None).with_session_tier(tier.clone()));
    let sink = Arc::new(AnchorSink { anchor });

    // ── Subagent spawn substrate (mirrors daemon.rs C.1) ──────────────────────
    let spawn_registry = Arc::new(SpawnRegistry::new());
    let spawner: Arc<DaemonSubagentSpawner<AgentLoop<NativePlatform>>> = Arc::new(
        DaemonSubagentSpawner::new(Arc::clone(&spawn_registry), SubagentConfig::default())
            .with_chain(chain.clone())
            .with_forest(SubagentForest {
                causal: causal.clone(),
                crossrefs: crossrefs.clone(),
            }),
    );

    // ── Tool registry with the four subagent tools registered against it ──────
    let workspace = temp_dir("ws");
    let mut tool_registry = ToolRegistry::new();
    clawft_tools::register_all(
        &mut tool_registry,
        Arc::new(NativePlatform::new()),
        workspace.clone(),
        clawft_tools::security_policy::CommandPolicy::safe_defaults(),
        clawft_tools::url_safety::UrlPolicy::default(),
        clawft_tools::web_search::WebSearchConfig::default(),
        Some(Arc::clone(&spawner) as Arc<dyn clawft_core::agent::spawn::SubagentSpawner>),
    );
    let tool_registry = Arc::new(tool_registry);

    // ── The agent loop (open gate — agent_spawn's ~0.93 effect would otherwise
    //     be denied; we want the happy path) ───────────────────────────────────
    let governance: Arc<dyn GateBackend> =
        Arc::new(GovernanceGate::open().with_chain(Arc::clone(&chain)));
    let gate: Arc<dyn clawft_core::agent::gate::EffectGate> =
        Arc::new(KernelEffectGate::new(governance));

    let dir = temp_dir("loop");
    let platform = Arc::new(NativePlatform::new());
    let bus = Arc::new(MessageBus::new());
    let sessions = SessionManager::with_dir(platform.clone(), dir.join("sessions"));
    let memory = Arc::new(MemoryStore::with_paths(
        dir.join("memory").join("MEMORY.md"),
        dir.join("memory").join("HISTORY.md"),
        platform.clone(),
    ));
    let skills = Arc::new(SkillsLoader::with_dir(dir.join("skills"), platform.clone()));
    let context = ContextBuilder::new(test_config(), memory, skills, platform.clone());

    // Permission resolver mirroring the operator config (.clawft/config.json):
    // the `agent.chat` channel is granted admin so its tools (incl. agent_spawn)
    // are permitted. `default_resolver()` grants admin only to the `cli` channel,
    // so `agent.chat` would fall to zero-trust and deny every tool — the daemon
    // relies on this channel override, so the test must too.
    let routing: clawft_types::routing::RoutingConfig = serde_json::from_value(serde_json::json!({
        "permissions": { "channels": { "agent.chat": { "level": 2, "tool_access": ["*"] } } }
    }))
    .expect("routing config builds");
    let resolver = PermissionResolver::new(&routing, None);

    let agent = AgentLoop::new(
        test_config(),
        platform,
        bus,
        make_pipeline(),
        tool_registry,
        context,
        Arc::new(sessions),
        resolver,
    )
    .with_gate(gate)
    .with_sink(sink);

    // ── Service + late-wire the spawner back-reference (Arc-cycle break) ──────
    // The tier is attached to the service (as the daemon does, daemon.rs ~1448)
    // so the spawner can reach it through its Weak<AgentService> to resolve the
    // parent turn uid for turn-level edge rooting (design D3 / task #31).
    let service = Arc::new(AgentService::new(Arc::new(agent)).with_session_tier(tier.clone()));
    spawner.set_service(Arc::downgrade(&service));

    // ── Drive the parent turn ─────────────────────────────────────────────────
    let chain_before = chain.len();
    let result = service
        .dispatch(params(PARENT_CONV, PARENT_PROMPT))
        .await
        .expect("parent dispatch succeeds under the open gate");

    // The parent LLM wove its final answer after the tool came back.
    assert_eq!(
        result.assistant_text, PARENT_FINAL,
        "parent reply is the post-tool final text"
    );

    // (5) spawned_tasks[] is populated on the parent's AgentChatResult (D8).
    assert_eq!(
        result.spawned_tasks.len(),
        1,
        "exactly one subagent spawned during the parent turn"
    );
    let task = &result.spawned_tasks[0];
    assert_eq!(task.status, "completed", "the awaited subagent completed");
    let task_id = task.task_id.clone();
    let child_conv = task.child_conv_id.clone();
    assert!(
        child_conv.starts_with(&format!("sub:{PARENT_CONV}:")),
        "child conv id is namespaced under the parent: {child_conv}"
    );

    // (4) The result flowed back: the spawner's registered outcome carries "4".
    let outcome = spawner
        .status(&task_id)
        .await
        .expect("spawner tracks the completed task");
    assert_eq!(
        outcome.result.as_deref(),
        Some(CHILD_ANSWER),
        "the child's answer is the task outcome"
    );

    // (6) The chain witnessed spawn + complete for this task (ADR-033, D6).
    let events = chain.tail(0);
    let spawn_witnessed = events.iter().any(|e| {
        e.source == "subagent"
            && e.kind == "agent.spawn"
            && e.payload
                .as_ref()
                .and_then(|p| p.get("task_id"))
                .and_then(|v| v.as_str())
                == Some(task_id.as_str())
    });
    let complete_witnessed = events.iter().any(|e| {
        e.source == "subagent"
            && e.kind == "agent.complete"
            && e.payload
                .as_ref()
                .and_then(|p| p.get("task_id"))
                .and_then(|v| v.as_str())
                == Some(task_id.as_str())
    });
    assert!(spawn_witnessed, "agent.spawn witnessed on the chain");
    assert!(complete_witnessed, "agent.complete witnessed on the chain");
    assert!(
        chain.len() > chain_before,
        "the chain advanced across the spawn"
    );

    // (1)+(3) Spawn tree edges root at a REAL committed parent turn node (task #31:
    // the spawner resolves the parent conv's latest anchored turn uid via the
    // SessionTier, NOT the synthetic conv_anchor fallback), and the tree is
    // lineage-connected to the user turn.
    //
    // Asserted the role-agnostic way pA recommends: locate the edges by their
    // deterministic child-side SOURCE uids (subagent.rs), read their target, and
    // assert it is NOT the conv_anchor phantom. That is the load-bearing proof
    // that production turn-level rooting worked (a conv_anchor target would mean
    // the SessionTier wasn't reachable through the service — the C.1 shared-
    // instance failure). Survives whichever turn is "latest" at spawn time.
    let spawn_goal_uid = UniversalNodeId::new(
        &StructureTag::CausalGraph,
        child_conv.as_bytes(),
        0,
        b"",
        b"spawn_goal",
    );
    let spawn_result_uid = UniversalNodeId::new(
        &StructureTag::CausalGraph,
        child_conv.as_bytes(),
        0,
        task_id.as_bytes(),
        b"spawn_result",
    );
    let triggered: Vec<_> = crossrefs
        .get_forward(&spawn_goal_uid)
        .into_iter()
        .filter(|cr| cr.ref_type == CrossRefType::TriggeredBy)
        .collect();
    let evidence: Vec<_> = crossrefs
        .get_forward(&spawn_result_uid)
        .into_iter()
        .filter(|cr| cr.ref_type == CrossRefType::EvidenceFor)
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "one TriggeredBy edge out of the child spawn-goal node (child goal → parent turn)"
    );
    assert_eq!(
        evidence.len(),
        1,
        "one EvidenceFor edge out of the child spawn-result node (child result → parent turn)"
    );
    let trig_target = triggered[0].target.clone();
    let evid_target = evidence[0].target.clone();
    assert_eq!(
        trig_target, evid_target,
        "TriggeredBy and EvidenceFor root the spawn tree at the SAME parent turn"
    );
    // LOAD-BEARING: a real committed turn, NOT the synthetic per-conversation
    // anchor fallback — proves the SessionTier turn-level resolution ran.
    assert_ne!(
        trig_target,
        conv_anchor_uid(PARENT_CONV),
        "edge roots at a real turn, not the conv_anchor phantom"
    );

    // Concrete identity for THIS stub: it emits a pure tool_use with no assistant
    // text, so `index_turn` skips the empty assistant tool-call turn
    // (session_tier.rs) and `latest_turn_uid` at spawn time is the USER turn — the
    // edges target it. (A model that narrates alongside the call would instead
    // root at the assistant tool-call turn; the lead's ruling is the invariant
    // "latest non-empty committed turn", not a fixed role.)
    let parent_view = tier
        .view_for(PARENT_CONV)
        .expect("parent conversation view exists");
    let user_seq = *parent_view
        .chain_seqs()
        .iter()
        .min()
        .expect("parent has at least the user turn anchored");
    let user_turn_uid = UniversalNodeId::new(
        &StructureTag::CausalGraph,
        PARENT_CONV.as_bytes(),
        user_seq,
        PARENT_PROMPT.as_bytes(),
        b"turn",
    );
    assert_eq!(
        trig_target, user_turn_uid,
        "text-less tool_use roots the edge at the parent user turn (empty-text skip)"
    );
    // Lineage intact: the spawning turn is Follows-linked forward into the
    // conversation, so the spawn tree is reachable along the turn chain.
    let spawn_turn_node = node_for_seq(&causal, user_seq);
    let follows_out = causal
        .get_forward_edges(spawn_turn_node)
        .into_iter()
        .filter(|e| e.edge_type == CausalEdgeType::Follows)
        .count();
    assert!(
        follows_out >= 1,
        "spawning turn is Follows-linked into the conversation lineage"
    );

    // (2) Both parent AND child turns commit Frontier→Committed on the shared
    // forest — the M2 pipeline held for spawned work.
    let commits = drain_commits(&talk_loop);
    assert!(commits > 0, "the hosted loop committed the anchored turns");

    for seq in parent_view.chain_seqs() {
        assert_eq!(
            parent_view.state(seq),
            Some(NodeState::Committed),
            "parent turn {seq} committed"
        );
    }
    assert!(
        parent_view.chain_seqs().len() >= 2,
        "parent has at least a user turn and a reply committed"
    );

    let child_view = tier
        .view_for(&child_conv)
        .expect("child conversation view exists on the SAME tier");
    for seq in child_view.chain_seqs() {
        assert_eq!(
            child_view.state(seq),
            Some(NodeState::Committed),
            "child turn {seq} committed"
        );
    }
    assert!(
        child_view.chain_seqs().len() >= 2,
        "child has at least a goal turn and an answer committed"
    );

    // Cleanup best-effort.
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&workspace);
}
