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

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::causal::{CausalEdgeType, CausalGraph, NodeId};
use crate::cognitive_tick::CognitiveTick;
use crate::coherence::{CoherenceDampener, CoherenceSignals};
use crate::context_graft::{NodeState, SessionView, mirror_state};
use crate::crossref::{CrossRef, CrossRefStore, CrossRefType, StructureTag, UniversalNodeId};
use crate::floor::{
    ContentReadiness, FloorCandidate, FloorState, UrgencySignals, contending_count, crowd_density,
    evaluate_floor,
};
use crate::health::HealthStatus;
use crate::impulse::{Impulse, ImpulseQueue, ImpulseType};
use crate::service::{ServiceType, SystemService};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tick result
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Turn lineage
// ---------------------------------------------------------------------------

/// A registered turn node: its causal-graph id and its universal id (the
/// cross-ref key). Populated by the dual-write path (Phase 1).
#[derive(Debug, Clone)]
struct TurnRef {
    node: NodeId,
    uid: UniversalNodeId,
}

struct TalkInner {
    /// The in-flight turn (the active speaker's frontier `chain_seq`), if any.
    current_turn: Option<u64>,
    /// Coherence soft-dampener (ADR-062 D6); applied to floor scores.
    dampener: CoherenceDampener,
}

// ---------------------------------------------------------------------------
// TalkModeLoop
// ---------------------------------------------------------------------------

/// The Talk-Mode tick service (ADR-062 Phase 2.1).
pub struct TalkModeLoop {
    impulses: Arc<ImpulseQueue>,
    causal: Arc<CausalGraph>,
    crossrefs: Arc<CrossRefStore>,
    view: Arc<SessionView>,
    /// Own ADR-047 self-calibrating tick (separate instance from the coherence
    /// `CognitiveTick`; shares the primitives, not the cadence).
    tick: Arc<CognitiveTick>,
    config: TalkModeConfig,
    /// chain_seq → its registered turn node (set by the dual-write path).
    lineage: DashMap<u64, TurnRef>,
    inner: Mutex<TalkInner>,
    total_ticks: AtomicU64,
}

impl TalkModeLoop {
    /// Wire a Talk-Mode loop to the forest substrate and an ADR-047 tick.
    pub fn new(
        impulses: Arc<ImpulseQueue>,
        causal: Arc<CausalGraph>,
        crossrefs: Arc<CrossRefStore>,
        view: Arc<SessionView>,
        tick: Arc<CognitiveTick>,
        config: TalkModeConfig,
    ) -> Self {
        Self {
            impulses,
            causal,
            crossrefs,
            view,
            tick,
            config,
            lineage: DashMap::new(),
            inner: Mutex::new(TalkInner {
                current_turn: None,
                dampener: CoherenceDampener::new(),
            }),
            total_ticks: AtomicU64::new(0),
        }
    }

    /// Register a turn node produced by the dual-write path (Phase 1) and make
    /// it the in-flight turn. `node`/`uid` are the causal node id and universal
    /// id from `session_forest::dual_write_turn`.
    pub fn register_turn(&self, chain_seq: u64, node: NodeId, uid: UniversalNodeId) {
        self.lineage.insert(chain_seq, TurnRef { node, uid });
        self.inner
            .lock()
            .expect("talk loop inner poisoned")
            .current_turn = Some(chain_seq);
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

    /// The in-flight turn's `chain_seq`, if any.
    pub fn current_turn(&self) -> Option<u64> {
        self.inner
            .lock()
            .expect("talk loop inner poisoned")
            .current_turn
    }

    /// Execute one tick: SENSE → FLOOR → MUTATE. Records its own compute time in
    /// the ADR-047 tick system. Pure graph work — no async, no I/O.
    pub fn tick(&self) -> TalkTickResult {
        let start = Instant::now();

        // ── SENSE ────────────────────────────────────────────────────────
        let mut impulses = self.impulses.drain_ready();
        impulses.truncate(self.config.max_impulses_per_tick);

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

    /// FLOOR: build candidates from the live frontier, score them, apply the
    /// coherence dampener to the winning score. Returns
    /// `(state, raw_score, dampened_score)`.
    fn read_floor(&self, drained: &[Impulse]) -> (FloorState, f32, f32) {
        let crowd = crowd_density(contending_count(drained));
        let current = self.current_turn();
        let candidates: Vec<FloorCandidate> = self
            .view
            .live_seqs()
            .into_iter()
            .map(|seq| {
                let readiness = if Some(seq) == current {
                    ContentReadiness::Generated
                } else {
                    ContentReadiness::NotStarted
                };
                FloorCandidate::new(
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
                )
            })
            .collect();
        let decision = evaluate_floor(&candidates);
        let dampened = {
            let inner = self.inner.lock().expect("talk loop inner poisoned");
            inner.dampener.apply(decision.score)
        };
        (decision.state, decision.score, dampened)
    }

    /// MUTATE one turn impulse onto the graph (ADR-062 D5).
    fn mutate(&self, imp: &Impulse, result: &mut TalkTickResult) {
        match imp.impulse_type {
            ImpulseType::EndOfUtterance => {
                // Commit the named turn (or the in-flight one): Frontier→Committed
                // on both substrates (mirror_state gates the legal step).
                let seq = payload_u64(imp, "chain_seq").or_else(|| self.current_turn());
                if let Some(seq) = seq
                    && self.commit_turn(seq)
                {
                    result.commits += 1;
                    let mut inner = self.inner.lock().expect("talk loop inner poisoned");
                    if inner.current_turn == Some(seq) {
                        inner.current_turn = None;
                    }
                }
            }
            ImpulseType::TurnShift => {
                // Floor handoff: the floor opens; the next utterance commits a
                // Follows. We drop the in-flight turn so the read goes Open.
                self.inner
                    .lock()
                    .expect("talk loop inner poisoned")
                    .current_turn = None;
                result.handoffs += 1;
            }
            ImpulseType::Backchannel => {
                // THE load-bearing invariant: a backchannel is a Continuer
                // cross-ref to the current speaker node — NEVER a turn node.
                if let Some(current) = self.current_turn()
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
                // Barge-in: prune the in-flight node (Frontier→Pruned) and draw a
                // Contradicts edge from the claiming turn (if registered).
                let pruned = self.current_turn();
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
                    // The claiming turn becomes the in-flight turn (if known).
                    let mut inner = self.inner.lock().expect("talk loop inner poisoned");
                    inner.current_turn = claim_seq.filter(|s| self.lineage.contains_key(s));
                }
            }
            // Non-turn impulses are the DEMOCRITUS loop's concern, not ours.
            _ => {}
        }
    }

    /// Commit a turn Frontier→Committed across both substrates.
    fn commit_turn(&self, seq: u64) -> bool {
        match self.lineage.get(&seq).map(|r| r.node) {
            Some(node) => mirror_state(&self.view, seq, &self.causal, node, NodeState::Committed),
            // No registered causal node — advance the view alone (it is the
            // source of truth; the mirror is best-effort).
            None => self.view.commit(seq),
        }
    }

    /// Prune a turn Frontier→Pruned (hard rebase) across both substrates.
    fn prune_turn(&self, seq: u64) -> bool {
        match self.lineage.get(&seq).map(|r| r.node) {
            Some(node) => mirror_state(&self.view, seq, &self.causal, node, NodeState::Pruned),
            None => self.view.transition(seq, NodeState::Pruned),
        }
    }
}

/// Read an optional `u64` field from an impulse payload object.
fn payload_u64(imp: &Impulse, key: &str) -> Option<u64> {
    imp.payload.get(key).and_then(|v| v.as_u64())
}

// ---------------------------------------------------------------------------
// SystemService
// ---------------------------------------------------------------------------

#[async_trait]
impl SystemService for TalkModeLoop {
    fn name(&self) -> &str {
        "voice.talk_mode"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.tick.set_running(true);
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.tick.set_running(false);
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.tick.is_running() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded("talk-mode tick not running".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Async run loop (native only — needs tokio/tokio-util)
// ---------------------------------------------------------------------------

/// Drive the Talk-Mode loop on the ADR-047 self-calibrating cadence until
/// `cancel` fires or the tick is stopped. Modeled on
/// [`run_democritus_loop`](crate::cognitive_tick::run_democritus_loop) but for
/// turn-taking. Spawned by the daemon (Phase 6).
#[cfg(feature = "native")]
pub async fn run_talk_loop(loop_: Arc<TalkModeLoop>, cancel: tokio_util::sync::CancellationToken) {
    loop_.tick.set_running(true);
    tracing::info!("Talk-Mode loop started");
    loop {
        let interval_ms = loop_.tick.current_interval_ms();
        if interval_ms == 0 {
            tracing::warn!("Talk-Mode loop: tick interval is 0, exiting");
            break;
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms as u64)) => {}
        }
        if !loop_.tick.is_running() {
            break;
        }
        loop_.tick();
    }
    loop_.tick.set_running(false);
    tracing::info!("Talk-Mode loop exited");
}

// ── Tests (split to a sibling file for the <500-line rule) ────────────────────
#[cfg(test)]
#[path = "talk_loop_tests.rs"]
mod tests;
