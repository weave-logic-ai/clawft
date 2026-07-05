//! Daemon-side full-duplex conversation-floor state machine (ADR-068 D1).
//!
//! [`DuplexChannel`] is the **actuator** of ECC floor verdicts, never a decider of
//! the floor. ECC (the [`TalkModeLoop`](crate::talk_loop::TalkModeLoop)) decides;
//! this machine executes. Every ad-hoc hold that lived inline in the old
//! half-duplex `TalkModeController` (mic-drain, prebuffer, barge grace, ack-cache)
//! becomes one transition *rule* here, in the daemon (ADR-068 §Context thesis 1).
//!
//! ## The actuator/decider boundary, enforced by construction (ADR-068 D1, plan §2)
//!
//! The two mutators are split by the type they consume:
//!
//! - [`observe`](DuplexChannel::observe) takes a [`StreamObservation`] — something
//!   the daemon derived from the media stream (VAD onset, endpoint, ERL, drain,
//!   heartbeat) — and returns `(Option<DuplexImpulse>, Vec<EdgeCommand>)`. It may
//!   perform **daemon-local** transitions and emit floor-pressure impulses, but its
//!   input and output types contain **no** [`FloorVerdict`], and a
//!   [`DuplexImpulse`] can only carry an [`ImpulseType`] (`TurnClaim`/`EndOfUtterance`)
//!   — there is no `Grant` variant. So `observe` is *structurally incapable* of
//!   granting the floor.
//! - [`apply`](DuplexChannel::apply) is the **only** method that takes a
//!   [`FloorVerdict`] (the type that carries `GrantUser`/`GrantAgent`), so it is the
//!   only path that can perform an ECC-authoritative floor transition.
//!
//! A reviewer verifies "the daemon never decides the floor from the stream" by
//! reading the two signatures — not by auditing the bodies.
//!
//! ## Resolved D1-table ambiguity: Overlap entry vs. the ERL gate
//!
//! The D1 row `Speaking | onset above gate, ERL confirms → Overlap` reads, in
//! isolation, as if a *high* ERL were required to enter `Overlap`. That is **not**
//! how this machine implements it, because the required self-cancel regression test
//! (plan §3) drives a *low*-ERL onset during `Speaking` and asserts `Overlap` is
//! entered and then reverted by an ECC `DenyBarge`. The two are reconciled per the
//! ADR's own stated mechanism (ADR-068 D1 "The self-cancel fix"): barge-in "becomes
//! a runtime floor decision", i.e. the ERL discriminates genuine barge from AEC
//! self-echo **at the floor read**, not at `Overlap` entry. So:
//!
//! - Any VAD onset during `Speaking` (the energy gate is upstream, in the edge/daemon
//!   VAD) enters the **provisional** `Overlap` and emits a `TurnClaim` carrying the
//!   last measured ERL confidence.
//! - ECC weights a low-ERL `TurnClaim` down (`compute_urgency`, ADR-062 D4, Phase 1)
//!   and returns [`FloorVerdict::DenyBarge`] → back to `Speaking`; a genuine
//!   high-ERL onset yields [`FloorVerdict::GrantUser`] → `Listening` + cancel.
//!
//! This keeps the decider in ECC and makes the self-cancel fix expressible without
//! the machine judging the barge itself. `Overlap` is explicitly provisional
//! (ADR-068 D1 states table).

use crate::impulse::ImpulseType;

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

/// The seven floor states (ADR-068 D1 states table). The edge holds only the
/// reflex shadow of these (see `clawft-channels` `edge_reflex`); the authoritative
/// machine is daemon-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexState {
    /// No active floor; the daemon watches the stream for onset. Mic streaming,
    /// speaker silent.
    Idle,
    /// The user holds the floor; the daemon accumulates the utterance.
    Listening,
    /// The daemon detected EOU; it is walking the graph and rendering.
    Thinking,
    /// The agent holds the floor; the daemon streams TTS down. Speaker playing.
    Speaking,
    /// Provisional: the daemon detected onset during `Speaking`. Playback
    /// continues pending the ECC verdict (grant → `Listening`, deny → `Speaking`).
    Overlap,
    /// The daemon classified a continuer ("mm-hmm"); playback continues.
    Backchannel,
    /// The link is lost; the edge reflex owns safety and the daemon has no view.
    Degraded,
}

/// Which payload rides the media lane — the axis that decides which transitions
/// are *reachable* (ADR-068 D2/D6). The machine is identical; text simply cannot
/// reach the audio-only states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadKind {
    /// PCM audio: every state is reachable.
    Audio,
    /// UTF-8 text (a terminal, GUI chat panel, or inkpad). The floor is always
    /// granted (submit *is* the floor), so `Overlap`/`Backchannel`/barge/ERL are
    /// structurally unreachable and the machine collapses to
    /// `Idle → Listening → Thinking → Speaking → Idle` (ADR-068 D6).
    Text,
}

// ---------------------------------------------------------------------------
// Wire vocabulary
// ---------------------------------------------------------------------------

/// A media-lane payload carried by [`EdgeCommand::Play`] (ADR-068 D3): PCM when
/// the edge offers no local codec, rendered text when it advertises local TTS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaPayload {
    /// 16 kHz PCM frame bytes down to the edge speaker.
    Pcm(Vec<u8>),
    /// Rendered text down to a text edge.
    Text(String),
}

/// A concrete command down the control lane to the edge (ADR-068 D3). Floor
/// verdicts reach the edge *only* as these — the edge never sees a [`FloorVerdict`]
/// or an impulse code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeCommand {
    /// Media down the media lane (PCM or text). Emitted by the renderer in Phase 1;
    /// the floor machine itself does not synthesise `Play` in Phase 0.
    Play(MediaPayload),
    /// Silence the speaker **instantly**, without draining the playout buffer
    /// (barge confirmed). Pairs with [`EdgeCommand::Cancel`] on a grant.
    Mute,
    /// Stop output (link-loss / end). Distinct from `Mute`: `Silence` is the
    /// end-of-floor quiet, `Mute` is the mid-playback barge cut.
    Silence,
    /// Return the speaker to full playback — the "un-duck" of a denied barge
    /// (ADR-068 D1 `Overlap → Speaking`). Idempotent on a thin edge that never
    /// ducked.
    Resume,
    /// Cancel the in-flight TTS render (barge granted; the answer is superseded).
    Cancel,
    /// Session/capability configuration at the handshake (ADR-068 D2/D4). Carries
    /// no payload in Phase 0 (no capability negotiation yet).
    Config,
}

/// Something the daemon **derived from the media stream** (ADR-068 D3: "the daemon
/// derives impulses from the media stream itself"). None of these can grant the
/// floor — they are observations, not verdicts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamObservation {
    /// VAD onset detected in the up-stream — a floor claim (ADR-062 D5,
    /// `SpeechStart → TurnClaim 0x50`).
    VadOnset,
    /// VAD end-of-speech (a brief offset). Ends a `Backchannel` continuer.
    VadOffset,
    /// Smart-turn endpoint — the user's utterance is complete
    /// (`EndOfUtterance 0x51`).
    SmartTurnEou,
    /// The edge reported its TTS playout has drained.
    PlaybackDrained,
    /// The latest echo-return-loss confidence the edge measured locally and
    /// streamed up the control lane. A daemon-held scalar folded into the floor
    /// read; never a transition by itself.
    Erl(f32),
    /// The edge heartbeat was lost — the link is down.
    HeartbeatLost,
    /// The link was restored and the edge re-handshook.
    LinkRestored,
}

/// The ECC-authoritative floor verdict from the [`TalkModeLoop`](crate::talk_loop).
/// This is the **only** type carrying a floor grant, and [`apply`](DuplexChannel::apply)
/// is the only method that consumes it — the structural core of the actuator/decider
/// boundary (ADR-068 D1, plan §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorVerdict {
    /// Grant the user the floor, cancelling the agent's output (`Overlap → Listening`;
    /// TTS cancel = `Contradicts`/prune, `Mute` to the edge).
    GrantUser,
    /// Grant the agent the floor: readiness met (`Thinking → Speaking`), or new
    /// committed content supersedes an ack while already `Speaking`.
    GrantAgent,
    /// Deny a provisional barge — low ERL confidence (`Overlap → Speaking`; un-duck).
    /// The principled self-cancel fix.
    DenyBarge,
    /// Classify a mid-`Speaking` onset as a continuer — a `Continuer` cross-ref,
    /// never a turn (`Speaking → Backchannel`, ADR-062 D5).
    Backchannel,
    /// The user resumes during `Thinking`; the floor shifts back (`Thinking →
    /// Listening`).
    TurnShift,
}

/// A floor-pressure impulse the daemon derives from the media stream and lifts onto
/// the ECC [`ImpulseQueue`](crate::impulse::ImpulseQueue) that feeds the
/// [`TalkModeLoop`](crate::talk_loop). Named to avoid colliding with
/// [`crate::impulse::Impulse`]; the plan (§Phase 0) calls this the derived
/// "Impulse". It can only carry an [`ImpulseType`] (no grant variant exists), which
/// is what makes [`observe`](DuplexChannel::observe) unable to decide the floor.
#[derive(Debug, Clone, PartialEq)]
pub struct DuplexImpulse {
    /// The impulse kind — `TurnClaim`/`EndOfUtterance` in Phase 0.
    pub kind: ImpulseType,
    /// The last ERL confidence the edge measured, carried so the floor read can
    /// weight a low-ERL `TurnClaim` down (ADR-068 D1 self-cancel fix). `None` until
    /// the edge streams an [`StreamObservation::Erl`].
    pub erl: Option<f32>,
}

impl DuplexImpulse {
    fn new(kind: ImpulseType, erl: Option<f32>) -> Self {
        Self { kind, erl }
    }
}

// ---------------------------------------------------------------------------
// DuplexChannel
// ---------------------------------------------------------------------------

/// The daemon-side floor state machine (ADR-068 D1). One per conversation. Pure
/// logic — no I/O, no async: it turns stream observations and ECC verdicts into a
/// state trajectory plus concrete [`EdgeCommand`]s.
#[derive(Debug, Clone)]
pub struct DuplexChannel {
    state: DuplexState,
    payload: PayloadKind,
    /// The last ERL confidence streamed up by the edge, folded into emitted
    /// `TurnClaim`s so the floor read can discriminate self-echo.
    last_erl: Option<f32>,
}

impl DuplexChannel {
    /// A new channel in [`DuplexState::Idle`] for the given payload axis.
    pub fn new(payload: PayloadKind) -> Self {
        Self {
            state: DuplexState::Idle,
            payload,
            last_erl: None,
        }
    }

    /// The current floor state.
    pub fn state(&self) -> DuplexState {
        self.state
    }

    /// The payload axis this channel was constructed with.
    pub fn payload(&self) -> PayloadKind {
        self.payload
    }

    /// The last ERL confidence the edge streamed up (`None` until first observed).
    pub fn last_erl(&self) -> Option<f32> {
        self.last_erl
    }

    /// Apply a **daemon-local** stream observation (ADR-068 D1). Returns the
    /// floor-pressure impulse to lift onto the ECC queue (if any) plus the edge
    /// commands to send. Cannot grant the floor — see the module docs.
    pub fn observe(
        &mut self,
        obs: StreamObservation,
    ) -> (Option<DuplexImpulse>, Vec<EdgeCommand>) {
        use DuplexState::*;
        use StreamObservation::*;

        match obs {
            // Continuous ERL update from the control lane: no transition, no
            // impulse — just refresh the scalar the next `TurnClaim` carries.
            Erl(v) => {
                self.last_erl = Some(v);
                (None, Vec::new())
            }

            // An onset is always a floor claim (`SpeechStart → TurnClaim 0x50`).
            // The state transition depends on where we are; the impulse is emitted
            // so ECC can arbitrate (Idle: implicit sole-party grant; Speaking:
            // provisional Overlap pending grant/deny; else: floor pressure only).
            VadOnset => match self.state {
                Degraded => (None, Vec::new()), // no link, no stream
                Idle => {
                    self.state = Listening;
                    (self.turn_claim(), Vec::new())
                }
                Speaking if self.payload == PayloadKind::Audio => {
                    // Provisional Overlap. Playback continues — the canonical thin
                    // edge does NOT duck (the provisional-duck reflex is opt-in and
                    // off by default, ADR-068 D5), so no edge command is emitted
                    // until ECC rules. The ERL rides the claim for the floor read.
                    self.state = Overlap;
                    (self.turn_claim(), Vec::new())
                }
                // Listening (onset within the utterance), Thinking (a physical
                // resume — ECC decides TurnShift), Overlap/Backchannel (already
                // handling an onset), or Speaking-in-text (no barge): stay put, but
                // still surface the floor claim to ECC.
                _ => (self.turn_claim(), Vec::new()),
            },

            // Smart-turn endpoint: only meaningful while the user holds the floor.
            SmartTurnEou => match self.state {
                Listening => {
                    self.state = Thinking;
                    (
                        Some(DuplexImpulse::new(ImpulseType::EndOfUtterance, self.last_erl)),
                        Vec::new(),
                    )
                }
                _ => (None, Vec::new()),
            },

            // The edge reports playout drained: the agent's turn is over.
            PlaybackDrained => match self.state {
                Speaking | Overlap | Backchannel => {
                    self.state = Idle;
                    (None, Vec::new())
                }
                _ => (None, Vec::new()),
            },

            // A brief offset ends a continuer; playback reclaims the floor.
            VadOffset => match self.state {
                Backchannel => {
                    self.state = Speaking;
                    (None, Vec::new())
                }
                _ => (None, Vec::new()),
            },

            // Link loss fires from any state. The daemon marks stale; it emits no
            // edge command (the link is down — the edge's own reflex owns safety,
            // ADR-068 D5 the one self-firing rule).
            HeartbeatLost => {
                self.state = Degraded;
                (None, Vec::new())
            }

            // Reconnect + resync: only meaningful out of Degraded.
            LinkRestored => match self.state {
                Degraded => {
                    self.state = Idle;
                    self.last_erl = None;
                    (None, Vec::new())
                }
                _ => (None, Vec::new()),
            },
        }
    }

    /// Apply an ECC floor verdict (ADR-068 D1). The **only** floor-bearing mutator:
    /// it consumes the one type that carries a grant. Returns the edge commands the
    /// verdict actuates.
    pub fn apply(&mut self, verdict: FloorVerdict) -> Vec<EdgeCommand> {
        use DuplexState::*;

        match verdict {
            // Barge granted: cancel the agent's answer and mute the speaker instantly.
            FloorVerdict::GrantUser => match self.state {
                Overlap => {
                    self.state = Listening;
                    vec![EdgeCommand::Mute, EdgeCommand::Cancel]
                }
                _ => Vec::new(),
            },

            // Barge denied (low ERL — the self-cancel fix): un-duck, keep speaking.
            // No Mute/Cancel reaches the edge; the answer is NOT cancelled.
            FloorVerdict::DenyBarge => match self.state {
                Overlap => {
                    self.state = Speaking;
                    vec![EdgeCommand::Resume]
                }
                _ => Vec::new(),
            },

            // Agent granted the floor: readiness met (Thinking) or new content
            // supersedes an ack (already Speaking). The renderer streams `Play`
            // frames down the media lane; the machine emits no synthetic command.
            FloorVerdict::GrantAgent => match self.state {
                Thinking | Speaking => {
                    self.state = Speaking;
                    Vec::new()
                }
                _ => Vec::new(),
            },

            // Mid-Speaking onset classified as a continuer: playback continues.
            // Audio-only: a text edge produces no backchannel classification, so
            // this is structurally inert in `Text` mode and `Backchannel` stays
            // unreachable (ADR-068 D6).
            FloorVerdict::Backchannel if self.payload == PayloadKind::Audio => {
                match self.state {
                    Speaking => {
                        self.state = Backchannel;
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            FloorVerdict::Backchannel => Vec::new(),

            // The user resumes during Thinking; the floor shifts back.
            FloorVerdict::TurnShift => match self.state {
                Thinking => {
                    self.state = Listening;
                    Vec::new()
                }
                _ => Vec::new(),
            },
        }
    }

    /// A `TurnClaim` impulse carrying the current ERL (ADR-062 D5 / ADR-068 D1).
    fn turn_claim(&self) -> Option<DuplexImpulse> {
        Some(DuplexImpulse::new(ImpulseType::TurnClaim, self.last_erl))
    }
}

// ── Tests (split to a sibling file for the <500-line rule) ────────────────────
#[cfg(test)]
#[path = "duplex_tests.rs"]
mod tests;
