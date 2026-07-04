//! Talk-Mode tick service — the orchestration heart of the ECC graph-walk voice
//! conversation (ADR-062 §Architecture, Phase 2.1).
//!
//! A conversation turn is a **walk that mutates a causal graph**; this loop is
//! the driver. It is modeled on the dormant [`DemocritusLoop`](crate::democritus)
//! (SENSE → … → COMMIT on a compute budget) but specialised to turn-taking:
//!
//! ```text
//! SENSE  = ImpulseQueue.drain_ready()                  (HLC-ordered turn signals)
//! FLOOR  = evaluate_floor / compute_urgency over       (a scored read, ADR-062 D4)
//!          SessionView.live_seqs(), dampened by         (coherence soft-dampener D6)
//! MUTATE = per impulse:
//!            EndOfUtterance → commit  (Frontier→Committed via mirror_state)
//!            TurnShift      → handoff (floor Open; clear the in-flight turn)
//!            Backchannel    → a Continuer cross-ref to the current speaker node,
//!                             and **never a turn node** (the load-bearing invariant)
//!            TurnClaim      → prune the in-flight node (Frontier→Pruned) +
//!                             draw a Contradicts edge from the claiming turn
//! ```
//!
//! It registers as a [`SystemService`] **alongside** the coherence
//! [`CognitiveTick`](crate::cognitive_tick::CognitiveTick) — it does **not**
//! touch that loop (which stays the slow integrity monitor). It owns its **own**
//! `CognitiveTick` instance so it shares the ADR-047 self-calibrating
//! budget/adaptive primitives (`tick_budget_ratio`, `record_tick`,
//! `current_interval_ms`) without coupling to the coherence cadence.
//!
//! The loop creates **no turn nodes** itself — turn nodes are produced by the
//! dual-write path (`session_forest::dual_write_turn`, Phase 1) and handed to
//! the loop via [`register_turn`](TalkModeLoop::register_turn). The loop only
//! advances their lifecycle and links reply/claim relationships. No audio, no
//! LLM, no embedder live here: this is pure graph orchestration.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::causal::{CausalEdgeType, CausalGraph, NodeId};
use crate::cognitive_tick::CognitiveTick;
use crate::coherence::{CoherenceDampener, CoherenceSignals};
use crate::context_graft::{NodeState, mirror_state};
use crate::crossref::{CrossRef, CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
use crate::floor::{
    ContentReadiness, FloorCandidate, FloorState, UrgencySignals, contending_count, crowd_density,
    evaluate_floor,
};
use crate::impulse::{Impulse, ImpulseQueue, ImpulseType};
use crate::view_resolver::ViewResolver;

use std::sync::Arc;

/// Configuration for the Talk-Mode loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkModeConfig {
    /// Maximum number of turn impulses to drain per tick.
    pub max_impulses_per_tick: usize,
    /// Default semantic-relevance signal fed to the floor read for a live
    /// frontier candidate (the in-loop path has no embedder; P6 supplies the
    /// real cosine via `observe_*`). Constant in v0.1 (Weaver learns it later).
    pub default_semantic_relevance: f32,
}

impl Default for TalkModeConfig {
    fn default() -> Self {
        Self {
            max_impulses_per_tick: 64,
            default_semantic_relevance: 0.2,
        }
    }
}

/// Summary of one Talk-Mode tick (SENSE → FLOOR → MUTATE).
#[derive(Debug, Clone)]
pub struct TalkTickResult {
    /// Turn impulses drained in the SENSE phase.
    pub impulses_sensed: usize,
    /// Turns committed (Frontier→Committed) this tick (EndOfUtterance).
    pub commits: usize,
    /// In-flight turns pruned this tick (TurnClaim barge-in).
    pub prunes: usize,
    /// Backchannels handled as Continuer cross-refs (never nodes).
    pub backchannels: usize,
    /// Floor handoffs (TurnShift → floor Open).
    pub handoffs: usize,
    /// The floor read for this tick.
    pub floor: FloorState,
    /// Winning urgency score before the coherence dampener.
    pub floor_score: f32,
    /// Winning urgency score after the coherence dampener (≤ `floor_score`).
    pub dampened_score: f32,
    /// Wall-clock duration of the tick in microseconds.
    pub duration_us: u64,
}

/// A registered turn node: its causal-graph id, its universal id (the cross-ref
/// key), and the conversation it belongs to. Populated by the dual-write path
/// (Phase 1). `conv_id` is what lets the multiplexed loop (M2 D4) resolve the
/// correct per-conversation view at commit/prune time.
#[derive(Debug, Clone)]
struct TurnRef {
    node: NodeId,
    uid: UniversalNodeId,
    conv_id: String,
}

struct TalkInner {
    /// Coherence soft-dampener (ADR-062 D6); applied to floor scores.
    dampener: CoherenceDampener,
}

/// The Talk-Mode tick service (ADR-062 Phase 2.1; multiplexed per M2 D1/D4).
///
/// A single daemon-hosted instance multiplexes **all** conversations. Turn
/// lineage is keyed by the globally-unique `chain_seq`; the per-conversation
/// view is resolved through the injected [`ViewResolver`] at commit/prune time,
/// so commit lands on the same `SessionView` the tier grafts from.
pub struct TalkModeLoop {
    impulses: Arc<ImpulseQueue>,
    causal: Arc<CausalGraph>,
    crossrefs: Arc<CrossRefStore>,
    /// Per-conversation view resolution (M2 D4). Replaces the single
    /// `Arc<SessionView>` so one loop serves every conversation.
    views: Arc<dyn ViewResolver>,
    /// Own ADR-047 self-calibrating tick (separate instance from the coherence
    /// `CognitiveTick`; shares the primitives, not the cadence).
    tick: Arc<CognitiveTick>,
    config: TalkModeConfig,
    /// chain_seq → its registered turn node (set by the dual-write path). The
    /// `chain_seq` is globally unique, so one map serves all conversations.
    lineage: DashMap<u64, TurnRef>,
    /// conv_id → its in-flight turn's `chain_seq`. Read by the floor and
    /// barge-in (both degenerate for text, which commits by explicit
    /// `chain_seq`); scoped per conversation for the multiplexed loop.
    current_turn: DashMap<String, u64>,
    inner: Mutex<TalkInner>,
    total_ticks: AtomicU64,
}

impl TalkModeLoop {
    /// Wire a Talk-Mode loop to the forest substrate, a per-conversation view
    /// resolver (M2 D4), and an ADR-047 tick.
    pub fn new(
        impulses: Arc<ImpulseQueue>,
        causal: Arc<CausalGraph>,
        crossrefs: Arc<CrossRefStore>,
        views: Arc<dyn ViewResolver>,
        tick: Arc<CognitiveTick>,
        config: TalkModeConfig,
    ) -> Self {
        Self {
            impulses,
            causal,
            crossrefs,
            views,
            tick,
            config,
            lineage: DashMap::new(),
            current_turn: DashMap::new(),
            inner: Mutex::new(TalkInner {
                dampener: CoherenceDampener::new(),
            }),
            total_ticks: AtomicU64::new(0),
        }
    }

    /// The impulse queue this loop drains. The text impulse source
    /// (`SessionTier::index_turn`) emits `EndOfUtterance` onto this exact queue,
    /// so it derives the handle from the loop rather than holding a second copy.
    pub fn impulses(&self) -> &Arc<ImpulseQueue> {
        &self.impulses
    }

    /// Register a turn node produced by the dual-write path (Phase 1) and make
    /// it `conv_id`'s in-flight turn. `node`/`uid` are the causal node id and
    /// universal id from `session_forest::dual_write_turn`.
    pub fn register_turn(
        &self,
        chain_seq: u64,
        node: NodeId,
        uid: UniversalNodeId,
        conv_id: impl Into<String>,
    ) {
        let conv_id = conv_id.into();
        self.lineage.insert(
            chain_seq,
            TurnRef {
                node,
                uid,
                conv_id: conv_id.clone(),
            },
        );
        self.current_turn.insert(conv_id, chain_seq);
    }

    /// Evict all per-conversation loop state for `conv_id` (M2 D6). Called when
    /// the idle reaper or an explicit `agent.chat.end` ends a conversation, so
    /// `lineage` and `current_turn` do not grow unbounded across the daemon's
    /// lifetime. Committed causal nodes stay on the graph (they are witnessed).
    pub fn end_conversation(&self, conv_id: &str) {
        self.lineage.retain(|_, turn| turn.conv_id != conv_id);
        self.current_turn.remove(conv_id);
    }

    /// Feed the latest coherence read so the dampener tracks drift (ADR-062 D6).
    /// P6 supplies the real RSTE signals; below threshold floor scores are
    /// softened ×0.8 until recovery (never a hard gate).
    pub fn observe_coherence(&self, signals: &CoherenceSignals) -> bool {
        self.inner
            .lock()
            .expect("talk loop inner poisoned")
            .dampener
            .observe(signals.score())
    }

    /// Total ticks executed.
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks.load(Ordering::Relaxed)
    }

    /// The in-flight turn's `chain_seq` for `conv_id`, if any.
    pub fn current_turn(&self, conv_id: &str) -> Option<u64> {
        self.current_turn.get(conv_id).map(|e| *e.value())
    }

    /// Execute one tick: SENSE → FLOOR → MUTATE. Records its own compute time in
    /// the ADR-047 tick system. Pure graph work — no async, no I/O.
    pub fn tick(&self) -> TalkTickResult {
        let start = Instant::now();

        // ── SENSE ────────────────────────────────────────────────────────
        // Drain at most `max_impulses_per_tick`; any overflow stays queued for
        // the next tick (M2 D8 fix — the old `drain_ready().truncate(max)`
        // dropped the tail on the floor, so flooded EOUs never committed).
        let impulses = self
            .impulses
            .drain_ready_limited(self.config.max_impulses_per_tick);

        // ── FLOOR ────────────────────────────────────────────────────────
        // A scored read over the live frontier crossed with the drained turn
        // signals; softened by the coherence dampener (state, not a new read).
        let (floor, floor_score, dampened_score) = self.read_floor(&impulses);

        // ── MUTATE ─────────────────────────────────────────────────────────
        let mut result = TalkTickResult {
            impulses_sensed: impulses.len(),
            commits: 0,
            prunes: 0,
            backchannels: 0,
            handoffs: 0,
            floor,
            floor_score,
            dampened_score,
            duration_us: 0,
        };
        for imp in &impulses {
            self.mutate(imp, &mut result);
        }

        // ── COMMIT (tick bookkeeping) ──────────────────────────────────────
        let compute_us = start.elapsed().as_micros() as u64;
        result.duration_us = compute_us;
        self.tick.record_tick(compute_us);
        self.total_ticks.fetch_add(1, Ordering::Relaxed);
        debug!(
            sensed = result.impulses_sensed,
            commits = result.commits,
            prunes = result.prunes,
            backchannels = result.backchannels,
            handoffs = result.handoffs,
            "talk-mode tick"
        );
        result
    }

    /// FLOOR: build candidates from the live frontier of every conversation
    /// touched this tick, score them, dampen the winner. Returns
    /// `(state, raw_score, dampened_score)`. Multiplexed (M2 D4): the resolver
    /// can't enumerate all views, so the conversations read are those referenced
    /// by the drained impulses plus any with an in-flight turn — degenerate for
    /// text, identical to the pre-M2 read for a single-conversation loop (voice).
    fn read_floor(&self, drained: &[Impulse]) -> (FloorState, f32, f32) {
        let crowd = crowd_density(contending_count(drained));
        let mut convs: HashSet<String> = HashSet::new();
        for imp in drained {
            if let Some(conv) = self.impulse_conv(imp) {
                convs.insert(conv);
            }
        }
        for entry in self.current_turn.iter() {
            convs.insert(entry.key().clone());
        }
        let mut candidates: Vec<FloorCandidate> = Vec::new();
        for conv in &convs {
            let current = self.current_turn(conv);
            let Some(view) = self.views.view_for(conv) else {
                continue;
            };
            for seq in view.live_seqs() {
                let readiness = if Some(seq) == current {
                    ContentReadiness::Generated
                } else {
                    ContentReadiness::NotStarted
                };
                candidates.push(FloorCandidate::new(
                    format!("seq-{seq}"),
                    seq,
                    UrgencySignals {
                        semantic_relevance: self.config.default_semantic_relevance,
                        emotional_arousal: 0.0,
                        wait_time: 0.0,
                        crowd_density: crowd,
                        content_readiness: readiness,
                        hard_interrupt: false,
                    },
                ));
            }
        }
        let decision = evaluate_floor(&candidates);
        let dampened = {
            let inner = self.inner.lock().expect("talk loop inner poisoned");
            inner.dampener.apply(decision.score)
        };
        (decision.state, decision.score, dampened)
    }

    /// Resolve the conversation an impulse acts on: explicit `conv_id` in the
    /// payload, else its `chain_seq`'s registered turn, else — for empty-payload
    /// floor-pressure impulses (voice) — the sole in-flight conversation if one
    /// exists. Text always carries `conv_id`/`chain_seq`, so the fallback is voice-only.
    fn impulse_conv(&self, imp: &Impulse) -> Option<String> {
        if let Some(conv) = imp.payload.get("conv_id").and_then(|v| v.as_str()) {
            return Some(conv.to_string());
        }
        if let Some(seq) = payload_u64(imp, "chain_seq")
            && let Some(turn) = self.lineage.get(&seq)
        {
            return Some(turn.conv_id.clone());
        }
        if self.current_turn.len() == 1 {
            return self.current_turn.iter().next().map(|e| e.key().clone());
        }
        None
    }

    /// MUTATE one turn impulse onto the graph (ADR-062 D5). Every branch scopes
    /// its `current_turn` read/write to the impulse's conversation (M2 D4).
    fn mutate(&self, imp: &Impulse, result: &mut TalkTickResult) {
        match imp.impulse_type {
            ImpulseType::EndOfUtterance => {
                // Commit the named turn (or the conversation's in-flight one):
                // Frontier→Committed on both substrates (mirror_state gates the
                // legal step). Text always carries an explicit `chain_seq`.
                let seq = payload_u64(imp, "chain_seq")
                    .or_else(|| self.impulse_conv(imp).and_then(|c| self.current_turn(&c)));
                if let Some(seq) = seq
                    && self.commit_turn(seq)
                {
                    result.commits += 1;
                    // Clear the in-flight slot for the turn's own conversation.
                    if let Some(turn) = self.lineage.get(&seq) {
                        let conv = turn.conv_id.clone();
                        drop(turn);
                        if self.current_turn(&conv) == Some(seq) {
                            self.current_turn.remove(&conv);
                        }
                    }
                }
            }
            ImpulseType::TurnShift => {
                // Floor handoff: the floor opens; the next utterance commits a
                // Follows. We drop the conversation's in-flight turn so the read
                // goes Open.
                if let Some(conv) = self.impulse_conv(imp) {
                    self.current_turn.remove(&conv);
                }
                result.handoffs += 1;
            }
            ImpulseType::Backchannel => {
                // THE load-bearing invariant: a backchannel is a Continuer
                // cross-ref to the current speaker node — NEVER a turn node.
                if let Some(conv) = self.impulse_conv(imp)
                    && let Some(current) = self.current_turn(&conv)
                    && let Some(turn) = self.lineage.get(&current)
                {
                    let listener = UniversalNodeId::from_bytes(imp.source_node);
                    self.crossrefs.insert(CrossRef {
                        source: listener,
                        source_structure: StructureTag::CausalGraph,
                        target: turn.uid.clone(),
                        target_structure: StructureTag::CausalGraph,
                        ref_type: CrossRefType::Continuer,
                        created_at: imp.hlc_timestamp,
                        chain_seq: current,
                    });
                    result.backchannels += 1;
                }
            }
            ImpulseType::TurnClaim => {
                // Barge-in: prune the conversation's in-flight node
                // (Frontier→Pruned) and draw a Contradicts edge from the claiming
                // turn (if registered).
                let Some(conv) = self.impulse_conv(imp) else {
                    return;
                };
                let pruned = self.current_turn(&conv);
                if let Some(prune_seq) = pruned
                    && self.prune_turn(prune_seq)
                {
                    result.prunes += 1;
                    let claim_seq = payload_u64(imp, "claim_seq");
                    if let (Some(claim_seq), Some(prune_node)) =
                        (claim_seq, self.lineage.get(&prune_seq).map(|r| r.node))
                        && let Some(claim_node) = self.lineage.get(&claim_seq).map(|r| r.node)
                    {
                        self.causal.link(
                            claim_node,
                            prune_node,
                            CausalEdgeType::Contradicts,
                            1.0,
                            imp.hlc_timestamp,
                            claim_seq,
                        );
                    }
                    // The claiming turn becomes the conversation's in-flight turn
                    // (if known); otherwise the floor is left open.
                    match claim_seq.filter(|s| self.lineage.contains_key(s)) {
                        Some(s) => {
                            self.current_turn.insert(conv, s);
                        }
                        None => {
                            self.current_turn.remove(&conv);
                        }
                    }
                }
            }
            // Non-turn impulses are the DEMOCRITUS loop's concern, not ours.
            _ => {}
        }
    }

    /// Commit a turn Frontier→Committed across both substrates, resolving the
    /// turn's per-conversation view (M2 D4). A no-op (logged) when the turn is
    /// unregistered or its conversation has been reaped.
    fn commit_turn(&self, seq: u64) -> bool {
        self.mirror_turn(seq, NodeState::Committed)
    }

    /// Prune a turn Frontier→Pruned (hard rebase) across both substrates.
    fn prune_turn(&self, seq: u64) -> bool {
        self.mirror_turn(seq, NodeState::Pruned)
    }

    /// Resolve `seq`'s registered node + conversation view and mirror `state`
    /// across the view and the causal graph. No-op (logged) when the lineage
    /// entry is gone (unregistered / evicted) or the view has been reaped.
    fn mirror_turn(&self, seq: u64, state: NodeState) -> bool {
        let Some(turn) = self.lineage.get(&seq) else {
            debug!(seq, ?state, "talk loop: no lineage entry for turn, no-op");
            return false;
        };
        let (node, conv_id) = (turn.node, turn.conv_id.clone());
        drop(turn);
        match self.views.view_for(&conv_id) {
            Some(view) => mirror_state(&view, seq, &self.causal, node, state),
            None => {
                debug!(seq, conv_id, ?state, "talk loop: view reaped, commit no-op");
                false
            }
        }
    }
}

/// Read an optional `u64` field from an impulse payload object.
fn payload_u64(imp: &Impulse, key: &str) -> Option<u64> {
    imp.payload.get(key).and_then(|v| v.as_u64())
}

// The `SystemService` impl + the native `run_talk_loop` live in a sibling file
// (child module, so it reaches these private fields) to keep this file under the
// 500-line ceiling. `run_talk_loop` is re-exported so the daemon path is stable.
#[path = "talk_loop_service.rs"]
mod service;
#[cfg(feature = "native")]
pub use service::run_talk_loop;

// ── Tests (split to a sibling file for the <500-line rule) ────────────────────
#[cfg(test)]
#[path = "talk_loop_tests.rs"]
mod tests;
