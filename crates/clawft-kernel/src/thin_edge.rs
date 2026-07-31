//! Desktop thin edge over localhost — Phase 1 scaffold (ADR-068 / WEFT-628).
//!
//! The edge is a **dumb streaming transducer**: capture + playback + a minimal
//! reflex set + a control/media lane pair. Cognition (STT/TTS/VAD/endpoint/
//! floor/classify) lives daemon-side. This module defines the **localhost wire
//! vocabulary** and an **in-process loopback session** that proves:
//!
//! 1. The edge streams [`EdgeState`] (incl. ERL confidence) up the control lane.
//! 2. The daemon [`DuplexChannel`] folds ERL into `TurnClaim`s.
//! 3. [`compute_urgency`](crate::floor::compute_urgency) / [`floor_verdict_from_erl`]
//!    weight low-ERL claims down so self-echo cannot win the floor.
//!
//! No cpal, no network, no TalkForest: pure protocol + state-machine wiring so
//! later daemon RPC / UDS transport (plan Phase 1) plugs in without reshaping
//! types. When the daemon is up the edge holds **no** standalone in-memory
//! forest — that invariant is structural here (this session only owns
//! [`DuplexChannel`] + edge reflex shadow flags).

use crate::duplex::{
    DuplexChannel, DuplexImpulse, DuplexState, EdgeCommand, FloorVerdict, PayloadKind,
    StreamObservation,
};
use crate::floor::{ContentReadiness, UrgencySignals, compute_urgency, floor_verdict_from_erl};
use crate::impulse::ImpulseType;

// ---------------------------------------------------------------------------
// Wire vocabulary (ADR-068 D3) — payload-typed media + control lanes
// ---------------------------------------------------------------------------

/// Media-lane frame the edge streams up (mic / keystrokes) or the daemon
/// streams down (TTS / rendered text). Typed by negotiated capability, not by
/// a baked profile (ADR-068 D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaFrame {
    /// 16 kHz PCM (or host sample-rate) bytes.
    Pcm(Vec<u8>),
    /// UTF-8 text deltas (terminal / GUI chat / mid-profile STT offload).
    Text(String),
}

/// Control-lane state the edge reports up every tick / on change (ADR-068 D3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeState {
    /// Speaker path currently playing daemon media.
    pub playback_active: bool,
    /// Latest AEC echo-return-loss confidence in `[0, 1]`, when measured.
    pub erl_confidence: Option<f32>,
    /// Silence-squelch engaged (bandwidth only; no semantic authority).
    pub squelch_active: bool,
    /// Heartbeat sequence so the daemon can detect link loss.
    pub heartbeat: u64,
}

impl EdgeState {
    /// Fresh edge: no playback, no ERL, no squelch, heartbeat 0.
    pub const fn new() -> Self {
        Self {
            playback_active: false,
            erl_confidence: None,
            squelch_active: false,
            heartbeat: 0,
        }
    }
}

impl Default for EdgeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Control-lane message (either direction).
#[derive(Debug, Clone, PartialEq)]
pub enum ControlMessage {
    /// Edge → daemon: current reflex/AEC shadow.
    EdgeUp(EdgeState),
    /// Daemon → edge: concrete command (floor verdicts never leave the daemon).
    Cmd(EdgeCommand),
}

// ---------------------------------------------------------------------------
// Thin edge shadow (edge-side of the localhost session)
// ---------------------------------------------------------------------------

/// Minimal edge-side shadow of the three ADR-068 D5 reflexes, mirrored here so
/// the kernel crate can unit-test the loopback without depending on
/// `clawft-channels`. The channels `EdgeReflex` is the production type; this
/// is a byte-compatible state shadow for the scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeReflexShadow {
    /// Streaming mic up, speaker permitted.
    Streaming,
    /// Speaker silenced by daemon command.
    Muted,
    /// Link lost — no TX, no speaker.
    Degraded,
}

/// Edge half of the localhost duplex session (WEFT-628).
///
/// Holds only reflex state + the last ERL; no forest, no STT, no floor
/// decisions. Daemon commands arrive as [`EdgeCommand`]; ERL is observed and
/// reported, never judged.
#[derive(Debug, Clone)]
pub struct ThinEdge {
    reflex: EdgeReflexShadow,
    state: EdgeState,
    tx: bool,
    speaker: bool,
}

impl ThinEdge {
    /// Fresh thin edge: streaming, speaker on, no ERL.
    pub fn new() -> Self {
        Self {
            reflex: EdgeReflexShadow::Streaming,
            state: EdgeState::new(),
            tx: true,
            speaker: true,
        }
    }

    /// Current reflex shadow.
    pub fn reflex(&self) -> EdgeReflexShadow {
        self.reflex
    }

    /// Latest control-lane snapshot (clone of the live state).
    pub fn edge_state(&self) -> EdgeState {
        let mut s = self.state;
        s.playback_active = self.speaker && self.reflex != EdgeReflexShadow::Degraded;
        s
    }

    /// Whether the edge is transmitting mic frames up.
    pub fn is_transmitting(&self) -> bool {
        self.tx && self.reflex != EdgeReflexShadow::Degraded
    }

    /// Relay AEC ERL confidence (ADR-068 D5 #1 is the AEC itself; this is the
    /// control-lane relay only).
    pub fn observe_erl(&mut self, erl: f32) {
        self.state.erl_confidence = Some(erl);
    }

    /// Execute a daemon command (instant mute/silence/resume/cancel).
    pub fn on_command(&mut self, cmd: &EdgeCommand) {
        if self.reflex == EdgeReflexShadow::Degraded {
            return;
        }
        match cmd {
            EdgeCommand::Mute | EdgeCommand::Silence | EdgeCommand::Cancel => {
                self.reflex = EdgeReflexShadow::Muted;
                self.speaker = false;
            }
            EdgeCommand::Resume | EdgeCommand::Play(_) => {
                self.reflex = EdgeReflexShadow::Streaming;
                self.speaker = true;
            }
            EdgeCommand::Config => {}
        }
    }

    /// Link-loss safety (the one self-firing rule).
    pub fn on_link_loss(&mut self) {
        self.reflex = EdgeReflexShadow::Degraded;
        self.tx = false;
        self.speaker = false;
    }

    /// Re-handshake after link restore: TX resumes, speaker stays silent until
    /// the daemon plays (never speak unprompted).
    pub fn on_link_restored(&mut self) {
        self.reflex = EdgeReflexShadow::Streaming;
        self.tx = true;
        self.speaker = false;
        self.state.erl_confidence = None;
        self.state.heartbeat = self.state.heartbeat.saturating_add(1);
    }

    /// Tick the heartbeat (edge → daemon keep-alive).
    pub fn beat(&mut self) {
        self.state.heartbeat = self.state.heartbeat.saturating_add(1);
    }
}

impl Default for ThinEdge {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Localhost duplex session (daemon + edge, in-process)
// ---------------------------------------------------------------------------

/// Outcome of one observation step on the daemon half.
#[derive(Debug, Clone, PartialEq)]
pub struct ObserveOutcome {
    /// Impulse lifted onto the ECC queue (if any).
    pub impulse: Option<DuplexImpulse>,
    /// Commands delivered to the edge this step.
    pub commands: Vec<EdgeCommand>,
    /// Daemon duplex state after the observation.
    pub duplex_state: DuplexState,
}

/// In-process localhost thin-edge session (ADR-068 Phase 1 scaffold).
///
/// The edge half is a [`ThinEdge`]; the daemon half is a [`DuplexChannel`].
/// There is **no** private TalkForest — floor cognition rides the shared
/// daemon machine only. Transport is a pair of `Vec`s simulating the two
/// lanes; a later UDS/WebSocket carrier swaps the vectors for sockets.
#[derive(Debug)]
pub struct LocalhostDuplexSession {
    /// Edge-side reflex + ERL relay.
    pub edge: ThinEdge,
    /// Daemon-side floor actuator.
    pub duplex: DuplexChannel,
    /// Media frames the edge has streamed up (for inspection / later STT).
    pub media_up: Vec<MediaFrame>,
    /// Media frames the daemon has streamed down (TTS / text).
    pub media_down: Vec<MediaFrame>,
}

impl LocalhostDuplexSession {
    /// Open a localhost session for the given payload axis.
    pub fn open(payload: PayloadKind) -> Self {
        Self {
            edge: ThinEdge::new(),
            duplex: DuplexChannel::new(payload),
            media_up: Vec::new(),
            media_down: Vec::new(),
        }
    }

    /// Edge pushes a media frame up (thin client of the daemon stream).
    pub fn edge_push_media(&mut self, frame: MediaFrame) {
        if self.edge.is_transmitting() {
            self.media_up.push(frame);
        }
    }

    /// Edge reports its control state (ERL + heartbeat) into the daemon.
    ///
    /// Folds `erl_confidence` into the duplex machine as
    /// [`StreamObservation::Erl`] so subsequent `TurnClaim`s carry it.
    pub fn edge_report_state(&mut self) -> ObserveOutcome {
        self.edge.beat();
        let state = self.edge.edge_state();
        if let Some(erl) = state.erl_confidence {
            let (_imp, cmds) = self.duplex.observe(StreamObservation::Erl(erl));
            debug_assert!(cmds.is_empty(), "Erl observation is non-transitional");
        }
        ObserveOutcome {
            impulse: None,
            commands: Vec::new(),
            duplex_state: self.duplex.state(),
        }
    }

    /// Daemon-local stream observation (VAD/EOU/drain/heartbeat), delivering
    /// any resulting commands to the edge.
    pub fn daemon_observe(&mut self, obs: StreamObservation) -> ObserveOutcome {
        let (impulse, commands) = self.duplex.observe(obs);
        for cmd in &commands {
            self.edge.on_command(cmd);
        }
        ObserveOutcome {
            impulse,
            commands,
            duplex_state: self.duplex.state(),
        }
    }

    /// Apply an ECC floor verdict and deliver edge commands.
    pub fn apply_verdict(&mut self, verdict: FloorVerdict) -> Vec<EdgeCommand> {
        let commands = self.duplex.apply(verdict);
        for cmd in &commands {
            self.edge.on_command(cmd);
        }
        commands
    }

    /// Score a barge claim for the floor using the claim's ERL (WEFT-628).
    ///
    /// Builds a contender urgency signal with the ERL scale applied so a
    /// low-ERL self-echo claim cannot outscore an agent holding the floor.
    pub fn score_barge_claim(impulse: &DuplexImpulse) -> f32 {
        if impulse.kind != ImpulseType::TurnClaim {
            return 0.0;
        }
        let signals = UrgencySignals {
            semantic_relevance: 1.0, // onset is maximally relevant to the floor
            emotional_arousal: 0.5,
            wait_time: 0.0,
            crowd_density: 0.25,
            content_readiness: ContentReadiness::NotStarted,
            hard_interrupt: false,
            erl_confidence: impulse.erl,
        };
        compute_urgency(&signals)
    }

    /// Decide a provisional Overlap using ERL only (scaffold for TalkModeLoop
    /// wiring). High ERL → grant user + mute; low ERL → deny barge.
    pub fn resolve_overlap_from_erl(&mut self, erl: Option<f32>) -> (FloorVerdict, Vec<EdgeCommand>) {
        let verdict = floor_verdict_from_erl(erl);
        let cmds = self.apply_verdict(verdict);
        (verdict, cmds)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::floor::erl_admits_barge;

    #[test]
    fn thin_edge_has_no_forest_only_reflex() {
        let e = ThinEdge::new();
        assert_eq!(e.reflex(), EdgeReflexShadow::Streaming);
        assert!(e.is_transmitting());
        // Structural: ThinEdge fields are reflex + EdgeState only — no graph.
        let _ = e.edge_state();
    }

    #[test]
    fn erl_relays_into_duplex_turn_claim() {
        let mut s = LocalhostDuplexSession::open(PayloadKind::Audio);
        s.edge.observe_erl(0.12);
        s.edge_report_state();
        assert_eq!(s.duplex.last_erl(), Some(0.12));

        // Idle → Listening on onset; claim carries the low ERL.
        let out = s.daemon_observe(StreamObservation::VadOnset);
        let claim = out.impulse.expect("TurnClaim");
        assert_eq!(claim.kind, ImpulseType::TurnClaim);
        assert_eq!(claim.erl, Some(0.12));
    }

    #[test]
    fn self_cancel_low_erl_denies_barge_no_mute() {
        let mut s = LocalhostDuplexSession::open(PayloadKind::Audio);
        // Agent speaking.
        let _ = s.daemon_observe(StreamObservation::VadOnset); // → Listening
        let _ = s.daemon_observe(StreamObservation::SmartTurnEou); // → Thinking
        let _ = s.apply_verdict(FloorVerdict::GrantAgent); // → Speaking
        assert_eq!(s.duplex.state(), DuplexState::Speaking);

        // Self-echo onset with low ERL → provisional Overlap.
        s.edge.observe_erl(0.08);
        s.edge_report_state();
        let out = s.daemon_observe(StreamObservation::VadOnset);
        assert_eq!(out.duplex_state, DuplexState::Overlap);
        let claim = out.impulse.expect("TurnClaim");
        assert!(!erl_admits_barge(claim.erl.unwrap_or(0.0)));

        // ERL-scaled urgency is tiny — cannot win a contested floor.
        let score = LocalhostDuplexSession::score_barge_claim(&claim);
        assert!(score < 0.2, "low-ERL claim must be weighted down, got {score}");

        // ECC DenyBarge → back to Speaking; no Mute/Cancel on the edge.
        let (verdict, cmds) = s.resolve_overlap_from_erl(claim.erl);
        assert_eq!(verdict, FloorVerdict::DenyBarge);
        assert!(
            !cmds.iter().any(|c| matches!(
                c,
                EdgeCommand::Mute | EdgeCommand::Cancel
            )),
            "self-cancel fix: answer not cancelled"
        );
        assert_eq!(s.duplex.state(), DuplexState::Speaking);
        assert_eq!(s.edge.reflex(), EdgeReflexShadow::Streaming);
    }

    #[test]
    fn genuine_barge_high_erl_mutes_edge() {
        let mut s = LocalhostDuplexSession::open(PayloadKind::Audio);
        let _ = s.daemon_observe(StreamObservation::VadOnset);
        let _ = s.daemon_observe(StreamObservation::SmartTurnEou);
        let _ = s.apply_verdict(FloorVerdict::GrantAgent);
        assert_eq!(s.duplex.state(), DuplexState::Speaking);

        s.edge.observe_erl(0.92);
        s.edge_report_state();
        let out = s.daemon_observe(StreamObservation::VadOnset);
        assert_eq!(out.duplex_state, DuplexState::Overlap);
        let claim = out.impulse.expect("TurnClaim");
        assert!(erl_admits_barge(claim.erl.unwrap()));

        let (verdict, cmds) = s.resolve_overlap_from_erl(claim.erl);
        assert_eq!(verdict, FloorVerdict::GrantUser);
        assert!(
            cmds.iter().any(|c| matches!(c, EdgeCommand::Mute)),
            "genuine barge must Mute the edge"
        );
        assert_eq!(s.duplex.state(), DuplexState::Listening);
        assert_eq!(s.edge.reflex(), EdgeReflexShadow::Muted);
    }

    #[test]
    fn media_up_only_while_transmitting() {
        let mut s = LocalhostDuplexSession::open(PayloadKind::Audio);
        s.edge_push_media(MediaFrame::Pcm(vec![1, 2, 3]));
        assert_eq!(s.media_up.len(), 1);
        s.edge.on_link_loss();
        s.edge_push_media(MediaFrame::Pcm(vec![4, 5, 6]));
        assert_eq!(s.media_up.len(), 1, "Degraded edge must not TX");
    }
}
