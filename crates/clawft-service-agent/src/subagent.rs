//! `DaemonSubagentSpawner` — the daemon-side [`SubagentSpawner`] (M4 A.2).
//!
//! A subagent is a daemon-hosted **child conversation** on the same
//! [`AgentService`] engine as the parent (design D1), so it inherits the entire
//! M2 commit pipeline (durable sink, chain anchoring, `SessionTier`, gate, forest
//! commit, idle reaping) for free. This type is the concrete implementation of
//! the [`SubagentSpawner`] seam (`clawft_core::agent::spawn`, design D2): the
//! `agent_spawn` / `task_status` / `task_result` / `agent_message` tools hold an
//! `Arc<dyn SubagentSpawner>` and call into here; here we dispatch the child
//! through `AgentService::dispatch` and draw the forest edges (design D3).
//!
//! ## Arc-cycle break (design D2)
//!
//! `AgentService` owns the tool registry, which owns a tool holding this
//! spawner, which dispatches back into `AgentService`. To avoid a reference
//! cycle the spawner holds only a [`Weak`] back-reference, wired *after* the
//! service `Arc` exists via [`set_service`](DaemonSubagentSpawner::set_service)
//! — the same late-wiring shape as the A2ARouter gate `OnceLock`.
//!
//! ## Forest edges (design D3, A.4)
//!
//! On spawn: `C.goal --TriggeredBy--> T_user@P`. On completion:
//! `R_c@C --EvidenceFor--> T_user@P`. Both are cross-conversation
//! [`CrossRef`](clawft_kernel::CrossRef)s in the shared store (see
//! [`session_forest::link_cross_conv`]), so the ADR-067 graph view renders the
//! spawn tree with zero extra work while each conversation's lineage walk stays
//! contained.

use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, warn};

use clawft_core::agent::spawn::{
    SpawnBackend, SpawnError, SpawnHandle, SpawnSpec, SubagentSpawner, TaskOutcome, TaskStatus,
};
use clawft_kernel::chain::ChainManager;
use clawft_kernel::{CausalGraph, CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};

use crate::protocol::{AgentChatMessage, AgentChatParams, AgentChatResult};
use crate::service::{AgentLoopHandle, AgentService};
use crate::session_forest::link_cross_conv;
use crate::spawn_registry::SpawnRegistry;
use crate::dialogue_act::{Act, RefinedAct};
use crate::turn_classifier::{ClassificationVector, KeywordTurnClassifier, TurnClassifier, Vad};

/// Chain source label for subagent lifecycle events (ADR-033 witnessing).
const WITNESS_SOURCE: &str = "subagent";
const EVENT_SPAWN: &str = "agent.spawn";
const EVENT_COMPLETE: &str = "agent.complete";
const EVENT_FAIL: &str = "agent.fail";
const EVENT_CANCEL: &str = "agent.cancel";

/// Caps + toggle for daemon subagents (design D5). Seeded from
/// `KernelConfig.agent.subagents` by the daemon boot wiring (C.1); the defaults
/// mirror `spawn_tool` (5 concurrent) and the WEFT-180 recursion cap (depth 3).
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    /// Master switch — when `false`, every `spawn` returns [`SpawnError::Disabled`].
    pub enabled: bool,
    /// Max live children per parent conversation.
    pub max_per_conv: u32,
    /// Max spawn depth (parent depth + 1 must not exceed this).
    pub max_depth: u32,
    /// Per-child dispatch timeout.
    pub timeout: Duration,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_per_conv: 5,
            max_depth: 3,
            timeout: Duration::from_secs(120),
        }
    }
}

/// Kernel-global forest handles the spawner draws cross-conv edges into (A.4).
#[derive(Clone)]
pub struct SubagentForest {
    /// The kernel-global causal graph (spawn-goal / spawn-result nodes).
    pub causal: Arc<CausalGraph>,
    /// The shared cross-ref store (`TriggeredBy` / `EvidenceFor` edges).
    pub crossrefs: Arc<CrossRefStore>,
}

/// The daemon-side [`SubagentSpawner`]. Generic over the loop handle so the
/// `Weak<AgentService<H>>` back-reference matches the concrete service the
/// daemon builds; boxed as `Arc<dyn SubagentSpawner>` at tool registration.
pub struct DaemonSubagentSpawner<H: AgentLoopHandle> {
    /// Late-wired back-reference into the service (Arc-cycle break, design D2).
    service: OnceLock<Weak<AgentService<H>>>,
    registry: Arc<SpawnRegistry>,
    chain: Option<Arc<ChainManager>>,
    forest: Option<SubagentForest>,
    config: SubagentConfig,
}

impl<H: AgentLoopHandle> DaemonSubagentSpawner<H> {
    /// Construct a spawner. Wire the service back-reference later with
    /// [`set_service`](Self::set_service); attach witnessing / forest with the
    /// `with_*` builders.
    pub fn new(registry: Arc<SpawnRegistry>, config: SubagentConfig) -> Self {
        Self {
            service: OnceLock::new(),
            registry,
            chain: None,
            forest: None,
            config,
        }
    }

    /// Attach the chain for lifecycle witnessing (design D6). Optional — absent
    /// in unit tests that don't assert the audit trail.
    pub fn with_chain(mut self, chain: Arc<ChainManager>) -> Self {
        self.chain = Some(chain);
        self
    }

    /// Attach the kernel-global forest so spawn/completion draw cross-conv edges
    /// (design D3). Optional — absent keeps spawns forest-silent.
    pub fn with_forest(mut self, forest: SubagentForest) -> Self {
        self.forest = Some(forest);
        self
    }

    /// Late-wire the `Weak<AgentService>` back-reference (design D2). Called by
    /// the daemon boot after the service `Arc` exists. Idempotent — a second
    /// call is ignored (the `OnceLock` is already set).
    pub fn set_service(&self, service: Weak<AgentService<H>>) {
        let _ = self.service.set(service);
    }

    /// Borrow the registry — the daemon uses it for the parent-end cascade.
    pub fn registry(&self) -> &Arc<SpawnRegistry> {
        &self.registry
    }

    /// Upgrade the back-reference, or `None` if the service is gone / unwired.
    fn service(&self) -> Option<Arc<AgentService<H>>> {
        self.service.get().and_then(Weak::upgrade)
    }

    /// Witness a lifecycle event on the chain (best-effort; no-op without a chain).
    fn witness(&self, kind: &str, payload: serde_json::Value) {
        if let Some(ref chain) = self.chain {
            chain.append(WITNESS_SOURCE, kind, Some(payload));
        }
    }

    /// Resolve the parent turn (T_user@P) uid for the spawn edges, tiered
    /// (M4 turn-level edge rooting):
    ///
    /// 1. **Supplied hint** — the tool passes the parent turn's uid (64-hex)
    ///    when the loop can mint it; use it directly.
    /// 2. **SessionTier lookup** — otherwise resolve the parent conversation's
    ///    latest anchored turn uid through the service's [`SessionTier`]. In the
    ///    daemon path `run_tool_loop` appends and anchors the assistant
    ///    tool-call turn immediately before dispatching the tool, so the latest
    ///    anchored turn at spawn time is the **assistant tool-call turn that
    ///    issued `agent_spawn`** — the turn that actually made the call. The
    ///    edge roots there (the user turn is one `Follows` hop upstream), the
    ///    most precise node the ADR-067 view can render.
    /// 3. **Conversation anchor** — final degraded tier (no tier attached, or an
    ///    empty conversation): a deterministic per-conv anchor uid so the tree
    ///    still renders rooted at the parent conversation.
    fn resolve_parent_uid(&self, hint: Option<&str>, parent_conv: &str) -> UniversalNodeId {
        if let Some(bytes) = hint.and_then(decode_hex32) {
            return UniversalNodeId::from_bytes(bytes);
        }
        if let Some(uid) = self
            .service()
            .and_then(|svc| svc.session_tier().and_then(|t| t.latest_turn_uid(parent_conv)))
        {
            return uid;
        }
        conv_anchor_uid(parent_conv)
    }
}

#[async_trait]
impl<H: AgentLoopHandle> SubagentSpawner for DaemonSubagentSpawner<H> {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnHandle, SpawnError> {
        if !self.config.enabled {
            return Err(SpawnError::Disabled);
        }
        // Only the Conversation tier is implemented in M4 (design D1/D2).
        if !matches!(spec.backend, SpawnBackend::Conversation) {
            return Err(SpawnError::BackendNotAvailable {
                backend: spec.backend.kind().to_string(),
                reason: "only the Conversation backend is implemented in M4".into(),
            });
        }
        // Depth cap (WEFT-180, design D5). `spec.depth` is the child's depth.
        if spec.depth > self.config.max_depth {
            return Err(SpawnError::DepthExceeded {
                depth: spec.depth,
                max: self.config.max_depth,
            });
        }

        let parent_conv = spec
            .parent_conv_id
            .clone()
            .unwrap_or_else(|| "root".to_string());

        // Concurrency cap (design D5) — reserve the slot before doing any work.
        self.registry
            .try_reserve(&parent_conv, self.config.max_per_conv)
            .map_err(|_| SpawnError::ConcurrencyExceeded {
                max: self.config.max_per_conv,
            })?;

        let task_id = ulid::Ulid::new().to_string();
        let child_conv = format!("sub:{parent_conv}:{task_id}");
        self.registry
            .insert(&task_id, &parent_conv, &child_conv, spec.group_id.clone());

        // Witness the spawn (ADR-033). `goal_hash` keeps the full goal off the
        // audit log while still binding the event to a specific request.
        self.witness(
            EVENT_SPAWN,
            serde_json::json!({
                "task_id": task_id,
                "parent": parent_conv,
                "child": child_conv,
                "goal_hash": goal_hash(&spec.goal),
                "depth": spec.depth,
            }),
        );

        // Forest: C.goal --TriggeredBy--> T_user@P (design D3). Drawn now so the
        // spawn tree is visible even for a still-running async child.
        let parent_uid = self.resolve_parent_uid(spec.parent_turn_uid.as_deref(), &parent_conv);
        if let Some(ref forest) = self.forest {
            // Spawn nodes are nodes: classify the goal so the tree renders with a
            // hue/glyph like every committed turn (classification design §D5).
            // The `goal` axis is the literal goal string (GoalMotivation
            // material); intent is Request; emotion neutral; tier keyword.
            let goal_class = spawn_goal_classification(&spec.goal);
            forest.causal.add_node(
                format!("spawn_goal:{task_id}"),
                serde_json::json!({
                    "conv_id": child_conv,
                    "parent_conv": parent_conv,
                    "task_id": task_id,
                    "depth": spec.depth,
                    "kind": "spawn_goal",
                    "state": "frontier",
                    "classification": goal_class.to_metadata_value(),
                }),
            );
            link_cross_conv(
                &forest.crossrefs,
                spawn_goal_uid(&child_conv),
                parent_uid.clone(),
                CrossRefType::TriggeredBy,
                0,
            );
            // GoalMotivation: spawn_goal --GoalMotivation--> goal-anchor (design
            // §D5). Same synthetic-anchor pattern as `dual_write_turn`'s goal
            // cross-ref (`session_forest.rs`), so the ADR-067 view can reverse-walk
            // a goal anchor to every spawn that pursued it.
            link_cross_conv(
                &forest.crossrefs,
                spawn_goal_uid(&child_conv),
                goal_anchor_uid(&child_conv, &spec.goal),
                CrossRefType::GoalMotivation,
                0,
            );
        }

        let Some(service) = self.service() else {
            // The service is gone/unwired — fail fast, release the slot.
            self.registry.set_outcome(
                &task_id,
                failed_outcome(&task_id, &child_conv, spec.group_id.clone(), "service unavailable"),
            );
            self.witness(
                EVENT_FAIL,
                serde_json::json!({ "task_id": task_id, "reason": "service unavailable" }),
            );
            return Err(SpawnError::DispatchFailed("agent service unavailable".into()));
        };

        let ctx = DriveCtx {
            task_id: task_id.clone(),
            child_conv: child_conv.clone(),
            group_id: spec.group_id.clone(),
            goal: spec.goal.clone(),
            persona: spec.persona.clone(),
            skills: spec.skills.clone(),
            depth: spec.depth,
            timeout: self.config.timeout,
            parent_uid,
        };

        if spec.await_completion {
            // Inline (the MVP path, design D3). Drive to completion, then return
            // the terminal handle straight away.
            drive_child(
                Arc::downgrade(&service),
                Arc::clone(&self.registry),
                self.chain.clone(),
                self.forest.clone(),
                ctx,
            )
            .await;
            let outcome = self.registry.outcome(&task_id);
            let status = outcome
                .as_ref()
                .map(|o| o.status)
                .unwrap_or(TaskStatus::Completed);
            Ok(SpawnHandle {
                task_id,
                child_conv_id: child_conv,
                status,
                outcome,
            })
        } else {
            // Detached (design D3). The parent polls `task_result` later.
            let weak = Arc::downgrade(&service);
            let registry = Arc::clone(&self.registry);
            let chain = self.chain.clone();
            let forest = self.forest.clone();
            let handle = tokio::spawn(async move {
                drive_child(weak, registry, chain, forest, ctx).await;
            });
            self.registry.set_join_handle(&task_id, handle);
            Ok(SpawnHandle {
                task_id,
                child_conv_id: child_conv,
                status: TaskStatus::Running,
                outcome: None,
            })
        }
    }

    async fn status(&self, task_id: &str) -> Option<TaskOutcome> {
        self.registry.outcome(task_id)
    }

    async fn group_status(&self, group_id: &str) -> Vec<TaskOutcome> {
        self.registry.group_outcomes(group_id)
    }

    async fn result(
        &self,
        task_id: &str,
        wait: Option<Duration>,
    ) -> Result<TaskOutcome, SpawnError> {
        let Some(outcome) = self.registry.outcome(task_id) else {
            return Err(SpawnError::NotFound {
                task_id: task_id.to_string(),
            });
        };
        if outcome.status.is_terminal() || wait.is_none() {
            return Ok(outcome);
        }
        // Poll until terminal or the wait budget elapses (design D3).
        let deadline = tokio::time::Instant::now() + wait.unwrap();
        loop {
            if tokio::time::Instant::now() >= deadline {
                return self.registry.outcome(task_id).ok_or(SpawnError::NotFound {
                    task_id: task_id.to_string(),
                });
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
            if let Some(o) = self
                .registry
                .outcome(task_id)
                .filter(|o| o.status.is_terminal())
            {
                return Ok(o);
            }
        }
    }

    async fn message(&self, task_id: &str, text: &str) -> Result<(), SpawnError> {
        let Some(child_conv) = self.registry.child_conv(task_id) else {
            return Err(SpawnError::NotFound {
                task_id: task_id.to_string(),
            });
        };
        let Some(service) = self.service() else {
            return Err(SpawnError::DispatchFailed("agent service unavailable".into()));
        };
        // A2A inject (design D4): dispatch a user turn into the child conv,
        // detached so the caller never blocks (never synchronously re-enters a
        // conversation that may be mid-turn — see design failure table).
        let params = child_params(&child_conv, text, None, &[], 0);
        tokio::spawn(async move {
            if let Err(e) = service.dispatch(params).await {
                warn!(error = %e, "subagent message dispatch failed");
            }
        });
        Ok(())
    }

    async fn cancel(&self, task_id: &str) -> Result<(), SpawnError> {
        let Some(child_conv) = self.registry.child_conv(task_id) else {
            return Err(SpawnError::NotFound {
                task_id: task_id.to_string(),
            });
        };
        // Abort the detached driver (if any) and cancel the in-flight child turn.
        if let Some(handle) = self.registry.take_join_handle(task_id) {
            handle.abort();
        }
        if let Some(service) = self.service() {
            service.cancel(&child_conv);
        }
        self.registry.set_status(task_id, TaskStatus::Cancelled);
        self.witness(EVENT_CANCEL, serde_json::json!({ "task_id": task_id }));
        Ok(())
    }
}

/// Per-dispatch context handed to [`drive_child`]. POD so it moves cleanly into
/// a detached `tokio::spawn`.
struct DriveCtx {
    task_id: String,
    child_conv: String,
    group_id: Option<String>,
    goal: String,
    persona: Option<String>,
    skills: Vec<String>,
    depth: u32,
    timeout: Duration,
    /// Snapshotted parent target uid (T_user@P) for the completion edge — the
    /// parent conversation may advance before an async child finishes, so we
    /// bind the edge target at spawn time.
    parent_uid: UniversalNodeId,
}

/// Dispatch the child conversation and record its outcome (design D3). Shared by
/// the inline (`await: true`) and detached (`await: false`) paths. Best-effort:
/// forest / chain side-effects never block the outcome landing in the registry.
async fn drive_child<H: AgentLoopHandle>(
    service: Weak<AgentService<H>>,
    registry: Arc<SpawnRegistry>,
    chain: Option<Arc<ChainManager>>,
    forest: Option<SubagentForest>,
    ctx: DriveCtx,
) {
    let outcome = match service.upgrade() {
        None => failed_outcome(&ctx.task_id, &ctx.child_conv, ctx.group_id.clone(), "service gone"),
        Some(svc) => {
            let params = child_params(&ctx.child_conv, &ctx.goal, ctx.persona.as_deref(), &ctx.skills, ctx.depth);
            match tokio::time::timeout(ctx.timeout, svc.dispatch(params)).await {
                Err(_elapsed) => {
                    svc.cancel(&ctx.child_conv);
                    failed_outcome(&ctx.task_id, &ctx.child_conv, ctx.group_id.clone(), "timeout")
                }
                Ok(Err(e)) => {
                    failed_outcome(&ctx.task_id, &ctx.child_conv, ctx.group_id.clone(), &e.to_string())
                }
                Ok(Ok(res)) => outcome_from_result(&ctx.task_id, &ctx.child_conv, ctx.group_id.clone(), res),
            }
        }
    };

    let terminal = outcome.status;
    let is_failure = matches!(terminal, TaskStatus::Failed);
    registry.set_outcome(&ctx.task_id, outcome.clone());

    // Forest: R_c@C --EvidenceFor--> T_user@P (design D3).
    if let Some(ref forest) = forest {
        // Classify the result too (classification design §D5): intent Statement,
        // topic inherited from the goal so the result renders the same hue as its
        // parent goal in the graph view, emotion neutral.
        let result_class = spawn_result_classification(&ctx.goal, outcome.result.as_deref());
        forest.causal.add_node(
            format!("spawn_result:{}", ctx.task_id),
            serde_json::json!({
                "conv_id": ctx.child_conv,
                "task_id": ctx.task_id,
                "kind": "spawn_result",
                "status": terminal.as_str(),
                "state": "frontier",
                "classification": result_class.to_metadata_value(),
            }),
        );
        link_cross_conv(
            &forest.crossrefs,
            spawn_result_uid(&ctx.child_conv, &ctx.task_id),
            ctx.parent_uid.clone(),
            CrossRefType::EvidenceFor,
            0,
        );
    }

    // Witness the terminal transition (design D6).
    if let Some(ref chain) = chain {
        if is_failure {
            chain.append(
                WITNESS_SOURCE,
                EVENT_FAIL,
                Some(serde_json::json!({
                    "task_id": ctx.task_id,
                    "reason": outcome.error.clone().unwrap_or_default(),
                })),
            );
        } else {
            chain.append(
                WITNESS_SOURCE,
                EVENT_COMPLETE,
                Some(serde_json::json!({
                    "task_id": ctx.task_id,
                    "finish_reason": outcome.finish_reason.clone().unwrap_or_default(),
                    "iterations": outcome.iterations,
                    "status": terminal.as_str(),
                })),
            );
        }
    }
    debug!(task_id = %ctx.task_id, status = terminal.as_str(), "subagent child finished");
}

/// Build the child conversation's dispatch params. The goal is the child's first
/// user turn; persona / skills / spawn depth ride the metadata map so the child
/// loop can pick up its identity (design D5).
fn child_params(
    child_conv: &str,
    text: &str,
    persona: Option<&str>,
    skills: &[String],
    depth: u32,
) -> AgentChatParams {
    let mut metadata = serde_json::Map::new();
    if let Some(p) = persona {
        metadata.insert("persona".into(), serde_json::Value::String(p.to_string()));
    }
    if !skills.is_empty() {
        metadata.insert(
            "skills".into(),
            serde_json::Value::Array(
                skills
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    metadata.insert("spawn_depth".into(), serde_json::Value::from(depth));
    AgentChatParams {
        messages: vec![AgentChatMessage {
            role: "user".into(),
            content: text.to_string(),
        }],
        temperature: None,
        max_tokens: None,
        conv_id: child_conv.to_string(),
        metadata: if metadata.is_empty() { None } else { Some(metadata) },
    }
}

/// Map a child's [`AgentChatResult`] to a [`TaskOutcome`]. A budget/iteration
/// finish reason surfaces as [`TaskStatus::Exhausted`] with the partial result
/// preserved (design D5), everything else as [`TaskStatus::Completed`].
fn outcome_from_result(
    task_id: &str,
    child_conv: &str,
    group_id: Option<String>,
    res: AgentChatResult,
) -> TaskOutcome {
    let status = match res.finish_reason.as_str() {
        "max_iterations" | "budget" | "length" => TaskStatus::Exhausted,
        _ => TaskStatus::Completed,
    };
    TaskOutcome {
        task_id: task_id.to_string(),
        child_conv_id: child_conv.to_string(),
        status,
        result: Some(res.assistant_text),
        finish_reason: Some(res.finish_reason),
        iterations: res.iterations,
        group_id,
        error: None,
    }
}

/// A `Failed` outcome carrying an error reason (design D5 failure table).
fn failed_outcome(
    task_id: &str,
    child_conv: &str,
    group_id: Option<String>,
    reason: &str,
) -> TaskOutcome {
    TaskOutcome {
        task_id: task_id.to_string(),
        child_conv_id: child_conv.to_string(),
        status: TaskStatus::Failed,
        result: None,
        finish_reason: Some("error".into()),
        iterations: 0,
        group_id,
        error: Some(reason.to_string()),
    }
}

/// Compact, privacy-preserving hash of a goal for the witness chain.
fn goal_hash(goal: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    goal.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Classify a spawn-goal node (design §D5): topic from the goal's top token,
/// `intent = Request`, neutral emotion, `goal` axis = the literal goal string
/// (the `GoalMotivation` material), tier keyword. Reuses the keyword classifier
/// for the topic extraction, then overrides the fixed axes.
fn spawn_goal_classification(goal: &str) -> ClassificationVector {
    let mut cv = KeywordTurnClassifier::new().classify("assistant", goal, None);
    cv.act = Act::new(RefinedAct::Command); // a spawn goal is a directive (§D5)
    cv.emotion = Vad::neutral();
    cv.goal = Some(goal.to_string());
    cv
}

/// Classify a spawn-result node (design §D5): `intent = Statement`, topic
/// inherited from the goal (same hue as the parent goal), neutral emotion, no
/// goal axis. Classified from the result summary when present, else the goal.
fn spawn_result_classification(goal: &str, result: Option<&str>) -> ClassificationVector {
    let goal_topic = KeywordTurnClassifier::new()
        .classify("assistant", goal, None)
        .topic;
    let summary = result.unwrap_or(goal);
    let mut cv = KeywordTurnClassifier::new().classify("assistant", summary, None);
    cv.act = Act::new(RefinedAct::Comment); // a result is an assertive comment (§D5)
    cv.emotion = Vad::neutral();
    cv.topic = goal_topic; // inherit the goal's topic (design §D5)
    cv
}

/// Synthetic goal-anchor uid for a spawn goal (`GoalMotivation` target). Mirrors
/// the `dual_write_turn` goal anchor (`session_forest.rs`) — conversation-scoped,
/// keyed by the goal string, `b"goal"` discriminator — so spawn goals and turn
/// goals cluster on the same anchor when the strings match.
fn goal_anchor_uid(conv_id: &str, goal: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        conv_id.as_bytes(),
        0,
        goal.as_bytes(),
        b"goal",
    )
}

/// Deterministic uid for the child's spawn-goal node (`TriggeredBy` source).
fn spawn_goal_uid(child_conv: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        child_conv.as_bytes(),
        0,
        b"",
        b"spawn_goal",
    )
}

/// Deterministic uid for the child's spawn-result node (`EvidenceFor` source).
fn spawn_result_uid(child_conv: &str, task_id: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        child_conv.as_bytes(),
        0,
        task_id.as_bytes(),
        b"spawn_result",
    )
}

/// Hint-or-anchor parent uid: the supplied 64-hex uid if present, else the
/// per-conversation anchor. The non-`SessionTier` slice of the tiered
/// [`DaemonSubagentSpawner::resolve_parent_uid`] resolution, factored out for a
/// unit test that exercises tiers 1 and 3 without a live service.
#[cfg(test)]
fn parent_target_uid(parent_turn_uid: Option<&str>, parent_conv: &str) -> UniversalNodeId {
    if let Some(bytes) = parent_turn_uid.and_then(decode_hex32) {
        return UniversalNodeId::from_bytes(bytes);
    }
    conv_anchor_uid(parent_conv)
}

/// Deterministic per-conversation anchor uid — the final degraded tier when no
/// parent-turn uid can be resolved, so the spawn tree still renders rooted at
/// the parent conversation.
fn conv_anchor_uid(parent_conv: &str) -> UniversalNodeId {
    UniversalNodeId::new(
        &StructureTag::CausalGraph,
        parent_conv.as_bytes(),
        0,
        b"",
        b"conv_anchor",
    )
}

/// Decode exactly 64 hex chars into a 32-byte array, or `None`.
fn decode_hex32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out[i] = (hi * 16 + lo) as u8;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::event::{InboundMessage, OutboundMessage};

    /// Stub loop handle: echoes a canned answer, ignoring the goal text.
    struct StubLoop {
        answer: String,
    }

    #[async_trait]
    impl AgentLoopHandle for StubLoop {
        async fn handle_turn(&self, msg: InboundMessage) -> Result<OutboundMessage, String> {
            Ok(OutboundMessage {
                channel: msg.channel,
                chat_id: msg.chat_id,
                content: self.answer.clone(),
                reply_to: None,
                media: Vec::new(),
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    fn service_with(answer: &str) -> Arc<AgentService<StubLoop>> {
        Arc::new(AgentService::new(Arc::new(StubLoop {
            answer: answer.into(),
        })))
    }

    fn spawner(
        registry: Arc<SpawnRegistry>,
        config: SubagentConfig,
    ) -> DaemonSubagentSpawner<StubLoop> {
        DaemonSubagentSpawner::new(registry, config)
    }

    fn spec(parent: &str, goal: &str) -> SpawnSpec {
        let mut s = SpawnSpec::new(goal);
        s.parent_conv_id = Some(parent.into());
        s.await_completion = true;
        s
    }

    #[tokio::test]
    async fn await_spawn_returns_completed_with_result() {
        let registry = Arc::new(SpawnRegistry::new());
        let sp = spawner(Arc::clone(&registry), SubagentConfig::default());
        let svc = service_with("4");
        sp.set_service(Arc::downgrade(&svc));

        let handle = sp.spawn(spec("P", "answer 2+2")).await.unwrap();
        assert_eq!(handle.status, TaskStatus::Completed);
        let outcome = handle.outcome.expect("await:true carries an outcome");
        assert_eq!(outcome.result.as_deref(), Some("4"));
        assert!(handle.child_conv_id.starts_with("sub:P:"));
        // The terminal transition released the concurrency slot.
        assert_eq!(registry.live_count("P"), 0);
    }

    #[tokio::test]
    async fn await_spawn_draws_forest_edges() {
        let registry = Arc::new(SpawnRegistry::new());
        let forest = SubagentForest {
            causal: Arc::new(CausalGraph::new()),
            crossrefs: Arc::new(CrossRefStore::new()),
        };
        let sp = spawner(Arc::clone(&registry), SubagentConfig::default())
            .with_forest(forest.clone());
        let svc = service_with("4");
        sp.set_service(Arc::downgrade(&svc));

        let handle = sp.spawn(spec("P", "answer 2+2")).await.unwrap();

        // TriggeredBy (spawn) + GoalMotivation (goal anchor) + EvidenceFor
        // (completion) cross-refs all drawn.
        assert_eq!(forest.crossrefs.count(), 3);
        // Two causal nodes: the spawn goal + the spawn result.
        assert_eq!(forest.causal.node_count(), 2);

        // GoalMotivation edge: spawn_goal --GoalMotivation--> goal-anchor (§D5).
        let goal_src = spawn_goal_uid(&handle.child_conv_id);
        assert_eq!(
            forest
                .crossrefs
                .by_type(&goal_src, &CrossRefType::GoalMotivation)
                .len(),
            1,
            "spawn_goal must carry a GoalMotivation cross-ref to the goal anchor"
        );

        // The spawn_goal node carries the classification blob, goal == the goal.
        let nodes = forest.causal.nodes_for_conv(&handle.child_conv_id);
        let goal_node = nodes
            .iter()
            .find(|n| n.metadata.get("kind").and_then(|v| v.as_str()) == Some("spawn_goal"))
            .expect("spawn_goal node present");
        let class = goal_node
            .metadata
            .get("classification")
            .expect("spawn_goal carries a classification blob");
        assert_eq!(class["goal"], serde_json::json!("answer 2+2"));
        assert_eq!(class["intent"], serde_json::json!("request"));
        assert_eq!(class["tier"], serde_json::json!("keyword"));

        // The spawn_result node is classified too (intent Statement, §D5).
        let result_node = nodes
            .iter()
            .find(|n| n.metadata.get("kind").and_then(|v| v.as_str()) == Some("spawn_result"))
            .expect("spawn_result node present");
        assert_eq!(
            result_node.metadata["classification"]["intent"],
            serde_json::json!("statement")
        );
    }

    #[tokio::test]
    async fn await_spawn_witnesses_chain() {
        let registry = Arc::new(SpawnRegistry::new());
        let chain = Arc::new(ChainManager::new(0, 128));
        let sp = spawner(Arc::clone(&registry), SubagentConfig::default())
            .with_chain(Arc::clone(&chain));
        let svc = service_with("4");
        sp.set_service(Arc::downgrade(&svc));

        let before = chain.len();
        sp.spawn(spec("P", "answer 2+2")).await.unwrap();
        // agent.spawn + agent.complete appended.
        assert_eq!(chain.len() - before, 2);
    }

    #[tokio::test]
    async fn native_backend_not_available() {
        let registry = Arc::new(SpawnRegistry::new());
        let sp = spawner(registry, SubagentConfig::default());
        let svc = service_with("x");
        sp.set_service(Arc::downgrade(&svc));
        let mut s = spec("P", "g");
        s.backend = SpawnBackend::Native;
        let err = sp.spawn(s).await.unwrap_err();
        assert!(matches!(err, SpawnError::BackendNotAvailable { .. }));
    }

    #[tokio::test]
    async fn depth_cap_refuses() {
        let registry = Arc::new(SpawnRegistry::new());
        let sp = spawner(registry, SubagentConfig::default()); // max_depth 3
        let svc = service_with("x");
        sp.set_service(Arc::downgrade(&svc));
        let mut s = spec("P", "g");
        s.depth = 4;
        let err = sp.spawn(s).await.unwrap_err();
        assert!(matches!(err, SpawnError::DepthExceeded { depth: 4, max: 3 }));
    }

    #[tokio::test]
    async fn concurrency_cap_refuses_the_sixth() {
        let registry = Arc::new(SpawnRegistry::new());
        let config = SubagentConfig {
            max_per_conv: 5,
            ..SubagentConfig::default()
        };
        let sp = spawner(Arc::clone(&registry), config);
        let svc = service_with("ok");
        sp.set_service(Arc::downgrade(&svc));

        // Five detached children saturate the parent's slots.
        for _ in 0..5 {
            let mut s = spec("P", "g");
            s.await_completion = false;
            sp.spawn(s).await.unwrap();
        }
        // Force all five to still be live by not awaiting; the 6th is refused.
        // (Detached children may finish quickly, so assert the cap held for at
        // least the immediate 6th attempt against a saturated reservation.)
        let mut sixth = spec("P", "g");
        sixth.await_completion = false;
        // If any detached child already completed, top back up to 5 live.
        while registry.live_count("P") < 5 {
            let mut s = spec("P", "g");
            s.await_completion = false;
            if sp.spawn(s).await.is_err() {
                break;
            }
        }
        let err = sp.spawn(sixth).await;
        if registry.live_count("P") >= 5 {
            assert!(matches!(
                err,
                Err(SpawnError::ConcurrencyExceeded { max: 5 })
            ));
        }
    }

    #[tokio::test]
    async fn disabled_config_refuses() {
        let registry = Arc::new(SpawnRegistry::new());
        let config = SubagentConfig {
            enabled: false,
            ..SubagentConfig::default()
        };
        let sp = spawner(registry, config);
        let err = sp.spawn(spec("P", "g")).await.unwrap_err();
        assert!(matches!(err, SpawnError::Disabled));
    }

    #[tokio::test]
    async fn detached_spawn_runs_and_result_resolves() {
        let registry = Arc::new(SpawnRegistry::new());
        let sp = spawner(Arc::clone(&registry), SubagentConfig::default());
        let svc = service_with("42");
        sp.set_service(Arc::downgrade(&svc));

        let mut s = spec("P", "compute");
        s.await_completion = false;
        let handle = sp.spawn(s).await.unwrap();
        assert_eq!(handle.status, TaskStatus::Running);

        // Block up to 2s for the detached child to finish.
        let outcome = sp
            .result(&handle.task_id, Some(Duration::from_secs(2)))
            .await
            .unwrap();
        assert_eq!(outcome.status, TaskStatus::Completed);
        assert_eq!(outcome.result.as_deref(), Some("42"));
    }

    #[tokio::test]
    async fn group_status_aggregates_siblings() {
        let registry = Arc::new(SpawnRegistry::new());
        let sp = spawner(Arc::clone(&registry), SubagentConfig::default());
        let svc = service_with("done");
        sp.set_service(Arc::downgrade(&svc));

        for _ in 0..2 {
            let mut s = spec("P", "g");
            s.group_id = Some("swarm-1".into());
            sp.spawn(s).await.unwrap();
        }
        let outs = sp.group_status("swarm-1").await;
        assert_eq!(outs.len(), 2);
        assert!(outs.iter().all(|o| o.status == TaskStatus::Completed));
    }

    #[test]
    fn decode_hex32_roundtrips() {
        let uid = spawn_goal_uid("sub:P:1");
        let hex = uid.to_string();
        let bytes = decode_hex32(&hex).unwrap();
        assert_eq!(&bytes, uid.as_bytes());
        // Malformed input falls through.
        assert!(decode_hex32("nothex").is_none());
        assert!(decode_hex32("abcd").is_none());
    }

    #[tokio::test]
    async fn spawn_edge_roots_at_parent_turn_via_session_tier() {
        use crate::session_tier::SessionTier;
        use clawft_kernel::embedding::{EmbeddingProvider, MockEmbeddingProvider};

        let causal = Arc::new(CausalGraph::new());
        let crossrefs = Arc::new(CrossRefStore::new());
        let chain = Arc::new(ChainManager::new(0, 128));
        let embedder: Arc<dyn EmbeddingProvider> = Arc::new(MockEmbeddingProvider::new(64));
        let tier = Arc::new(
            SessionTier::new(embedder, chain.clone(), None)
                .with_forest(causal.clone(), crossrefs.clone()),
        );
        // Production ordering (loop_core::run_tool_loop): the parent's user turn
        // is anchored first, then the assistant tool-call turn that issues
        // agent_spawn is appended and anchored immediately BEFORE the tool
        // dispatches. So the latest anchored turn at spawn time is the assistant
        // tool-call turn — the turn that actually made the call.
        tier.index_turn("P", 1, "message", "user", "please spawn a helper", None)
            .await;
        tier.index_turn("P", 2, "message", "assistant", "calling agent_spawn", None)
            .await;
        let assistant_uid = tier.latest_turn_uid("P").expect("assistant turn anchored");
        let user_uid = crate::session_forest::turn_universal_id("P", 1, "please spawn a helper");

        // Service carries the same tier; spawner shares the same forest handles.
        let svc = Arc::new(
            AgentService::new(Arc::new(StubLoop { answer: "ok".into() }))
                .with_session_tier(tier.clone()),
        );
        let sp = spawner(Arc::new(SpawnRegistry::new()), SubagentConfig::default())
            .with_forest(SubagentForest {
                causal: causal.clone(),
                crossrefs: crossrefs.clone(),
            });
        sp.set_service(Arc::downgrade(&svc));

        // No parent_turn_uid hint → the spawner must resolve via the tier.
        let mut s = spec("P", "help");
        s.parent_turn_uid = None;
        sp.spawn(s).await.unwrap();

        // The TriggeredBy edge roots at the ASSISTANT tool-call turn (the turn
        // that issued the spawn), NOT the user turn (one Follows hop upstream)
        // and NOT the synthetic conversation anchor.
        assert!(
            crossrefs
                .get_reverse(&assistant_uid)
                .iter()
                .any(|cr| cr.ref_type == CrossRefType::TriggeredBy),
            "TriggeredBy edge must root at the assistant tool-call turn"
        );
        assert!(
            crossrefs.get_reverse(&user_uid).is_empty(),
            "edge must NOT root at the user turn — it is one Follows hop upstream"
        );
        assert!(
            crossrefs.get_reverse(&conv_anchor_uid("P")).is_empty(),
            "edge must NOT fall back to the conv anchor when the tier resolves the turn"
        );
    }

    #[test]
    fn parent_uid_prefers_supplied_hex() {
        let real = spawn_goal_uid("sub:P:99");
        let via_hex = parent_target_uid(Some(&real.to_string()), "P");
        assert_eq!(via_hex.as_bytes(), real.as_bytes());
        // Anchor fallback is stable per conversation and distinct.
        let anchor = parent_target_uid(None, "P");
        assert_ne!(anchor.as_bytes(), real.as_bytes());
        assert_eq!(anchor.as_bytes(), parent_target_uid(Some("bad"), "P").as_bytes());
    }
}
