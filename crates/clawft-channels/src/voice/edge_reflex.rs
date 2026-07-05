//! The dumb edge's minimal reflex set (ADR-068 D5) — the *only* edge-local logic.
//!
//! Everything that can tolerate one cognitive tick plus link RTT is
//! daemon-authoritative (VAD, endpointing, floor grant, barge *judgment*,
//! backchannel classification). The edge holds **only** the three reflexes for
//! things that physically cannot wait for a round trip:
//!
//! 1. **Instant mute / silence on command.** When the daemon confirms a barge the
//!    edge silences the speaker immediately, without draining its playout buffer.
//!    This is reflex *execution* of a daemon *decision* — the edge does not judge
//!    the barge ([`EdgeReflex::on_command`]).
//! 2. **Link-loss safety ([`ReflexState::Degraded`]).** On heartbeat loss the edge
//!    stops transmitting and silences output on its own, because its trigger *is*
//!    the daemon's absence ([`EdgeReflex::on_link_loss`]). The one self-firing rule.
//! 3. **The AEC/ERL seam is an *input*, not implemented here.** Sample-accurate
//!    echo cancellation lives in `clawft-voice-aec` (it owns device I/O); this
//!    module only *relays* the ERL confidence scalar up ([`EdgeReflex::observe_erl`])
//!    and executes mute against the daemon's verdict.
//!
//! ## Structurally dumb (ADR-068 D1/D5, plan §2)
//!
//! The reflex's only inputs are an [`EdgeCommand`] (from the daemon) and link
//! events. It has **no** `FloorVerdict` and **no** impulse constructor, so it is
//! structurally incapable of deciding the floor — a reviewer verifies "the edge is
//! dumb" from the types. The load-bearing safety invariant (ADR-068 D4) is that a
//! disconnected edge never speaks unprompted and never holds an open mic into a
//! dead link: [`ReflexState::Degraded`] forces both `tx` and speaker off.
//!
//! ## no_std, dependency-free
//!
//! This module uses only `core` constructs (no `alloc`, no `std`) so it later
//! compiles for `wasm32` and an ESP32-class `no_std` target unchanged. In Phase 2
//! it graduates, with the stream-lane client, into the core `clawft-duplex-edge`
//! crate (plan §5). Until then it defines its own **payload-free** [`EdgeCommand`]
//! mirror of the daemon-side `clawft_kernel::duplex::EdgeCommand` — the reflex never
//! inspects media bytes, so it needs no payload and no cross-crate dependency. The
//! Phase-1 stream boundary maps the daemon command onto this one; the two
//! discriminants must stay in sync until the shared crate lands.

/// A control-lane command the daemon actuates on the edge (ADR-068 D3), as the
/// reflex sees it: payload-free, because a reflex acts on the *kind*, not the media.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeCommand {
    /// Media is arriving on the media lane (PCM/text). The reflex re-enables the
    /// speaker path but does not itself buffer or decode audio.
    Play,
    /// Silence the speaker instantly, mid-playback (barge confirmed).
    Mute,
    /// Stop output — end-of-floor quiet / link teardown.
    Silence,
    /// Return the speaker to full playback (the "un-duck" of a denied barge).
    Resume,
    /// Cancel the in-flight render; drop any queued playout.
    Cancel,
    /// Session/capability configuration at the handshake.
    Config,
}

/// The edge's reflex shadow of the daemon floor (ADR-068 D5). Only three states —
/// the edge does not mirror the seven-state daemon machine; it holds the coarse
/// safety shadow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflexState {
    /// Normal operation: transmitting mic frames up, speaker path permitted.
    Streaming,
    /// Speaker silenced by a daemon command (barge / end). Still transmitting up.
    Muted,
    /// Link lost: transmitting stopped, speaker silenced, awaiting re-handshake.
    /// The self-firing safety state.
    Degraded,
}

/// The dumb edge reflex core (ADR-068 D5). Pure logic, no I/O: a real edge drives
/// its cpal capture/playback and `clawft-voice-aec` from the flags this exposes.
#[derive(Debug, Clone, Copy)]
pub struct EdgeReflex {
    state: ReflexState,
    /// Whether the edge is transmitting mic frames up the media lane.
    tx: bool,
    /// Whether the speaker path is permitted to play.
    speaker: bool,
    /// The last ERL confidence measured by the (external) AEC, relayed up.
    erl: Option<f32>,
    /// Silence-squelch: suppress transmitting pure-silence frames to save
    /// bandwidth. A bandwidth optimization with **no semantic authority** — the
    /// daemon re-runs VAD on whatever arrives (ADR-068 D3).
    squelch: bool,
}

impl EdgeReflex {
    /// A fresh edge: streaming up, speaker permitted, no squelch, no ERL yet.
    pub const fn new() -> Self {
        Self {
            state: ReflexState::Streaming,
            tx: true,
            speaker: true,
            erl: None,
            squelch: false,
        }
    }

    /// The current reflex state.
    pub fn state(&self) -> ReflexState {
        self.state
    }

    /// Whether the edge is currently transmitting mic frames up.
    pub fn is_transmitting(&self) -> bool {
        self.tx
    }

    /// Whether the speaker path is currently permitted to play.
    pub fn speaker_active(&self) -> bool {
        self.speaker
    }

    /// The last ERL confidence relayed up (`None` until the AEC reports one).
    pub fn erl(&self) -> Option<f32> {
        self.erl
    }

    /// **Reflex 1.** Execute a daemon command instantly. The edge does not judge —
    /// it actuates. A command while `Degraded` is ignored (the link is down; no
    /// daemon command can be arriving anyway).
    pub fn on_command(&mut self, cmd: EdgeCommand) {
        if self.state == ReflexState::Degraded {
            return;
        }
        match cmd {
            // Silence the speaker without draining the playout buffer.
            EdgeCommand::Mute | EdgeCommand::Silence | EdgeCommand::Cancel => {
                self.state = ReflexState::Muted;
                self.speaker = false;
            }
            // Return to full playback.
            EdgeCommand::Resume | EdgeCommand::Play => {
                self.state = ReflexState::Streaming;
                self.speaker = true;
            }
            // Configuration touches neither the speaker nor the tx path.
            EdgeCommand::Config => {}
        }
    }

    /// **Reflex 2.** The one self-firing rule: on heartbeat loss, stop transmitting
    /// and silence output. Enforces the ADR-068 D4 invariant — a disconnected edge
    /// never speaks unprompted and never holds an open mic into a dead link.
    pub fn on_link_loss(&mut self) {
        self.state = ReflexState::Degraded;
        self.tx = false;
        self.speaker = false;
    }

    /// Re-handshake completed: resume streaming. Speaker stays silent until the
    /// daemon commands playback again (`Play`/`Resume`) — the edge never speaks
    /// unprompted after a drop.
    pub fn on_link_restored(&mut self) {
        self.state = ReflexState::Streaming;
        self.tx = true;
        self.speaker = false;
        self.erl = None;
    }

    /// **Reflex 3 (seam).** Relay the ERL confidence the external AEC measured, for
    /// the edge to stream up the control lane. Not a decision — just a pass-through.
    pub fn observe_erl(&mut self, erl: f32) {
        self.erl = Some(erl);
    }

    /// Enable or disable the silence-squelch bandwidth optimization.
    pub fn set_squelch(&mut self, enabled: bool) {
        self.squelch = enabled;
    }

    /// Whether the edge should transmit this frame. `false` only when not
    /// transmitting at all (Degraded) or when squelch is on and the frame is pure
    /// silence. Carries no turn semantics — the daemon still owns VAD (ADR-068 D3).
    pub fn should_transmit(&self, frame_is_silence: bool) -> bool {
        self.tx && !(self.squelch && frame_is_silence)
    }
}

impl Default for EdgeReflex {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_edge_streams_with_speaker_permitted() {
        let r = EdgeReflex::new();
        assert_eq!(r.state(), ReflexState::Streaming);
        assert!(r.is_transmitting());
        assert!(r.speaker_active());
        assert_eq!(r.erl(), None);
    }

    #[test]
    fn mute_silences_speaker_instantly_but_keeps_tx() {
        let mut r = EdgeReflex::new();
        r.on_command(EdgeCommand::Mute);
        assert_eq!(r.state(), ReflexState::Muted);
        assert!(!r.speaker_active(), "mute must silence the speaker");
        assert!(r.is_transmitting(), "mute must NOT stop mic uplink");
    }

    #[test]
    fn silence_and_cancel_also_mute() {
        for cmd in [EdgeCommand::Silence, EdgeCommand::Cancel] {
            let mut r = EdgeReflex::new();
            r.on_command(cmd);
            assert!(!r.speaker_active(), "{cmd:?} must silence the speaker");
        }
    }

    #[test]
    fn resume_and_play_reopen_the_speaker() {
        for cmd in [EdgeCommand::Resume, EdgeCommand::Play] {
            let mut r = EdgeReflex::new();
            r.on_command(EdgeCommand::Mute);
            assert!(!r.speaker_active());
            r.on_command(cmd);
            assert_eq!(r.state(), ReflexState::Streaming);
            assert!(r.speaker_active(), "{cmd:?} must reopen the speaker");
        }
    }

    /// The load-bearing D4 safety invariant.
    #[test]
    fn link_loss_stops_tx_and_silences_output() {
        let mut r = EdgeReflex::new();
        r.on_link_loss();
        assert_eq!(r.state(), ReflexState::Degraded);
        assert!(!r.is_transmitting(), "a dead link must not hold an open mic");
        assert!(!r.speaker_active(), "a disconnected edge must not speak");
    }

    #[test]
    fn commands_are_ignored_while_degraded() {
        let mut r = EdgeReflex::new();
        r.on_link_loss();
        r.on_command(EdgeCommand::Play); // no daemon can reach a dead link
        assert_eq!(r.state(), ReflexState::Degraded);
        assert!(!r.speaker_active());
        assert!(!r.is_transmitting());
    }

    #[test]
    fn restore_resumes_streaming_but_speaker_stays_silent() {
        let mut r = EdgeReflex::new();
        r.observe_erl(0.5);
        r.on_link_loss();
        r.on_link_restored();
        assert_eq!(r.state(), ReflexState::Streaming);
        assert!(r.is_transmitting());
        assert!(
            !r.speaker_active(),
            "edge must not speak unprompted after reconnect",
        );
        assert_eq!(r.erl(), None, "stale ERL cleared on reconnect");
    }

    #[test]
    fn erl_is_relayed_not_decided() {
        let mut r = EdgeReflex::new();
        r.observe_erl(0.42);
        assert_eq!(r.erl(), Some(0.42));
        // Relaying ERL changes no floor/speaker state — the daemon decides.
        assert_eq!(r.state(), ReflexState::Streaming);
        assert!(r.speaker_active());
    }

    #[test]
    fn squelch_suppresses_only_silence_frames_and_carries_no_authority() {
        let mut r = EdgeReflex::new();
        // Squelch off: transmit everything.
        assert!(r.should_transmit(true));
        assert!(r.should_transmit(false));
        // Squelch on: drop pure-silence frames, keep voiced frames.
        r.set_squelch(true);
        assert!(!r.should_transmit(true), "squelch drops silence frames");
        assert!(r.should_transmit(false), "squelch keeps voiced frames");
    }

    #[test]
    fn degraded_never_transmits_even_with_squelch_off() {
        let mut r = EdgeReflex::new();
        r.on_link_loss();
        assert!(!r.should_transmit(false), "Degraded never transmits");
    }
}
