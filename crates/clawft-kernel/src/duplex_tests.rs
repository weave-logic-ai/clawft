//! Tests for the daemon-side [`DuplexChannel`] (ADR-068 D1).
//!
//! Three suites, all runnable without hardware (plan §3):
//!   1. `transition_table` — table-driven over every ADR-068 D1 row.
//!   2. `text_degenerate` — the D6 conformance gate (audio-only states unreachable).
//!   3. `loopback_sim` — `MockEcc` + `MockEdge` driving the self-cancel / barge /
//!      backchannel / link-drop benches.

use super::*;

// ---------------------------------------------------------------------------
// Loopback simulation harness (plan §3: MockStream + MockFloor + MockEdge)
// ---------------------------------------------------------------------------

/// One scripted step: either something the daemon observed on the stream, or a
/// verdict the ECC floor returned.
#[derive(Debug, Clone)]
enum Step {
    Obs(StreamObservation),
    Verdict(FloorVerdict),
}

/// A `MockEdge` records every [`EdgeCommand`] the daemon actuated, so a scenario
/// can assert exactly what reached the device (or that nothing did).
#[derive(Default)]
struct MockEdge {
    commands: Vec<EdgeCommand>,
}

/// Drive a [`DuplexChannel`] through a script, collecting the state trajectory, the
/// derived impulses, and the commands delivered to the [`MockEdge`]. This is the
/// no-hardware loopback: `Step::Obs` stands in for `MockStream`, `Step::Verdict`
/// for `MockEcc`/`MockFloor`.
struct DuplexSim {
    channel: DuplexChannel,
    edge: MockEdge,
    trajectory: Vec<DuplexState>,
    impulses: Vec<DuplexImpulse>,
}

impl DuplexSim {
    fn new(payload: PayloadKind) -> Self {
        let channel = DuplexChannel::new(payload);
        let trajectory = vec![channel.state()];
        Self {
            channel,
            edge: MockEdge::default(),
            trajectory,
            impulses: Vec::new(),
        }
    }

    fn run(&mut self, script: &[Step]) {
        for step in script {
            match step.clone() {
                Step::Obs(obs) => {
                    let (impulse, cmds) = self.channel.observe(obs);
                    if let Some(imp) = impulse {
                        self.impulses.push(imp);
                    }
                    self.edge.commands.extend(cmds);
                }
                Step::Verdict(v) => {
                    let cmds = self.channel.apply(v);
                    self.edge.commands.extend(cmds);
                }
            }
            self.trajectory.push(self.channel.state());
        }
    }

    fn entered(&self, s: DuplexState) -> bool {
        self.trajectory.contains(&s)
    }

    fn impulse_kinds(&self) -> Vec<ImpulseType> {
        self.impulses.iter().map(|i| i.kind.clone()).collect()
    }
}

// ---------------------------------------------------------------------------
// 1. Transition-table conformance (plan §3 test 1) — every ADR-068 D1 row
// ---------------------------------------------------------------------------

mod transition_table {
    use super::*;

    /// One expected D1 row. `from` + trigger → `to`, with the impulse the trigger
    /// emits and the commands it actuates.
    struct Row {
        name: &'static str,
        from: DuplexState,
        step: Step,
        to: DuplexState,
        impulse: Option<ImpulseType>,
        commands: Vec<EdgeCommand>,
    }

    /// Force `chan` into `target` from `Idle` via the shortest legal path, so each
    /// row is tested from its documented `from` state. Uses only public transitions.
    fn drive_to(chan: &mut DuplexChannel, target: DuplexState) {
        use DuplexState::*;
        // Preload an ERL so `TurnClaim`s carry a scalar where the rows expect one.
        chan.observe(StreamObservation::Erl(0.9));
        match target {
            Idle => {}
            Listening => {
                chan.observe(StreamObservation::VadOnset);
            }
            Thinking => {
                chan.observe(StreamObservation::VadOnset);
                chan.observe(StreamObservation::SmartTurnEou);
            }
            Speaking => {
                chan.observe(StreamObservation::VadOnset);
                chan.observe(StreamObservation::SmartTurnEou);
                chan.apply(FloorVerdict::GrantAgent);
            }
            Overlap => {
                chan.observe(StreamObservation::VadOnset);
                chan.observe(StreamObservation::SmartTurnEou);
                chan.apply(FloorVerdict::GrantAgent);
                chan.observe(StreamObservation::VadOnset);
            }
            Backchannel => {
                chan.observe(StreamObservation::VadOnset);
                chan.observe(StreamObservation::SmartTurnEou);
                chan.apply(FloorVerdict::GrantAgent);
                chan.apply(FloorVerdict::Backchannel);
            }
            Degraded => {
                chan.observe(StreamObservation::HeartbeatLost);
            }
        }
        assert_eq!(chan.state(), target, "drive_to failed to reach {target:?}");
    }

    fn rows() -> Vec<Row> {
        use DuplexState::*;
        use StreamObservation as O;
        use FloorVerdict as V;
        vec![
            Row {
                name: "Idle --VadOnset--> Listening (implicit sole-party grant)",
                from: Idle,
                step: Step::Obs(O::VadOnset),
                to: Listening,
                impulse: Some(ImpulseType::TurnClaim),
                commands: vec![],
            },
            Row {
                name: "Listening --SmartTurnEou--> Thinking",
                from: Listening,
                step: Step::Obs(O::SmartTurnEou),
                to: Thinking,
                impulse: Some(ImpulseType::EndOfUtterance),
                commands: vec![],
            },
            Row {
                name: "Thinking --GrantAgent--> Speaking",
                from: Thinking,
                step: Step::Verdict(V::GrantAgent),
                to: Speaking,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Thinking --TurnShift--> Listening",
                from: Thinking,
                step: Step::Verdict(V::TurnShift),
                to: Listening,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Speaking --PlaybackDrained--> Idle",
                from: Speaking,
                step: Step::Obs(O::PlaybackDrained),
                to: Idle,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Speaking --GrantAgent (supersede)--> Speaking",
                from: Speaking,
                step: Step::Verdict(V::GrantAgent),
                to: Speaking,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Speaking --VadOnset (+erl)--> Overlap",
                from: Speaking,
                step: Step::Obs(O::VadOnset),
                to: Overlap,
                impulse: Some(ImpulseType::TurnClaim),
                commands: vec![],
            },
            Row {
                name: "Overlap --GrantUser--> Listening (cancel+mute)",
                from: Overlap,
                step: Step::Verdict(V::GrantUser),
                to: Listening,
                impulse: None,
                commands: vec![EdgeCommand::Mute, EdgeCommand::Cancel],
            },
            Row {
                name: "Overlap --DenyBarge--> Speaking (un-duck)",
                from: Overlap,
                step: Step::Verdict(V::DenyBarge),
                to: Speaking,
                impulse: None,
                commands: vec![EdgeCommand::Resume],
            },
            Row {
                name: "Speaking --Backchannel--> Backchannel",
                from: Speaking,
                step: Step::Verdict(V::Backchannel),
                to: Backchannel,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Backchannel --VadOffset (continuer ends)--> Speaking",
                from: Backchannel,
                step: Step::Obs(O::VadOffset),
                to: Speaking,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "any --HeartbeatLost--> Degraded",
                from: Speaking,
                step: Step::Obs(O::HeartbeatLost),
                to: Degraded,
                impulse: None,
                commands: vec![],
            },
            Row {
                name: "Degraded --LinkRestored--> Idle",
                from: Degraded,
                step: Step::Obs(O::LinkRestored),
                to: Idle,
                impulse: None,
                commands: vec![],
            },
        ]
    }

    #[test]
    fn every_d1_row_conforms() {
        for row in rows() {
            let mut chan = DuplexChannel::new(PayloadKind::Audio);
            drive_to(&mut chan, row.from);

            let (impulse, commands) = match row.step.clone() {
                Step::Obs(obs) => chan.observe(obs),
                Step::Verdict(v) => (None, chan.apply(v)),
            };

            assert_eq!(chan.state(), row.to, "state mismatch: {}", row.name);
            assert_eq!(
                impulse.map(|i| i.kind),
                row.impulse,
                "impulse mismatch: {}",
                row.name
            );
            assert_eq!(commands, row.commands, "command mismatch: {}", row.name);
        }
    }

    /// `HeartbeatLost` must reach `Degraded` from *every* state (the D1 "any" row).
    #[test]
    fn heartbeat_lost_from_any_state() {
        use DuplexState::*;
        for target in [Idle, Listening, Thinking, Speaking, Overlap, Backchannel] {
            let mut chan = DuplexChannel::new(PayloadKind::Audio);
            drive_to(&mut chan, target);
            let (imp, cmds) = chan.observe(StreamObservation::HeartbeatLost);
            assert_eq!(chan.state(), Degraded, "no Degraded from {target:?}");
            assert!(imp.is_none());
            assert!(cmds.is_empty(), "daemon must not command a dead link");
        }
    }

    /// The `TurnClaim` emitted on a mid-`Speaking` onset carries the last ERL, so
    /// the floor read can discriminate self-echo (ADR-068 D1 self-cancel fix).
    #[test]
    fn overlap_claim_carries_erl() {
        let mut chan = DuplexChannel::new(PayloadKind::Audio);
        drive_to(&mut chan, DuplexState::Speaking);
        chan.observe(StreamObservation::Erl(0.12));
        let (imp, _) = chan.observe(StreamObservation::VadOnset);
        let imp = imp.expect("onset emits a TurnClaim");
        assert_eq!(imp.kind, ImpulseType::TurnClaim);
        assert_eq!(imp.erl, Some(0.12));
    }
}

// ---------------------------------------------------------------------------
// 2. Text-degenerate conformance (plan §3 test 2, ADR-068 D6) — REQUIRED
// ---------------------------------------------------------------------------

mod text_degenerate {
    use super::*;

    /// A text edge collapses to `Idle → Listening → Thinking → Speaking → Idle`,
    /// with `Overlap`/`Backchannel` never entered and barge/ERL structurally inert
    /// (ADR-068 D6). This mirrors M2's `index_turn` emitting `register_turn` +
    /// `EndOfUtterance` at the same seam voice uses.
    #[test]
    fn text_collapses_to_linear_path() {
        let mut sim = DuplexSim::new(PayloadKind::Text);
        sim.run(&[
            // Line begins → the floor is the user's (submit IS the floor).
            Step::Obs(StreamObservation::VadOnset),
            // Line submitted → EOU.
            Step::Obs(StreamObservation::SmartTurnEou),
            // Agent granted the floor, renders, then drains.
            Step::Verdict(FloorVerdict::GrantAgent),
            Step::Obs(StreamObservation::PlaybackDrained),
        ]);

        use DuplexState::*;
        assert_eq!(
            sim.trajectory,
            vec![Idle, Listening, Thinking, Speaking, Idle],
            "text must collapse to the linear path"
        );
        assert_eq!(
            sim.impulse_kinds(),
            vec![ImpulseType::TurnClaim, ImpulseType::EndOfUtterance],
            "text emits the same register_turn + EndOfUtterance seam as voice",
        );
    }

    /// The audio-only states are structurally unreachable for a text edge, even
    /// under an adversarial script: an onset during `Speaking` does NOT open
    /// `Overlap`, a `Backchannel` verdict is inert, a barge grant cannot fire (no
    /// `Overlap` to grant from), and ERL updates drive no transition (ADR-068 D6).
    #[test]
    fn audio_only_states_unreachable_for_text() {
        let mut sim = DuplexSim::new(PayloadKind::Text);
        sim.run(&[
            Step::Obs(StreamObservation::VadOnset),
            Step::Obs(StreamObservation::SmartTurnEou),
            Step::Verdict(FloorVerdict::GrantAgent), // -> Speaking
            // Adversarial: audio events that must all be inert in text mode.
            Step::Obs(StreamObservation::Erl(0.05)),
            Step::Obs(StreamObservation::VadOnset), // would open Overlap in audio
            Step::Verdict(FloorVerdict::Backchannel), // audio-only classification
            Step::Verdict(FloorVerdict::GrantUser), // barge: no Overlap to grant
            Step::Verdict(FloorVerdict::DenyBarge), // barge: no Overlap to deny
        ]);

        // None of the audio-only states were entered.
        assert!(!sim.entered(DuplexState::Overlap), "text reached Overlap");
        assert!(!sim.entered(DuplexState::Backchannel), "text reached Backchannel");
        // The machine held `Speaking` through the entire adversarial tail.
        assert_eq!(sim.channel.state(), DuplexState::Speaking);
        // The ERL scalar was still recorded (harmless), but drove no transition.
        assert_eq!(sim.channel.last_erl(), Some(0.05));
    }

    /// A text edge still degrades safely on link loss (Degraded is not an
    /// audio-only state — D6 only makes Overlap/Backchannel/barge unreachable).
    #[test]
    fn text_still_degrades_on_link_loss() {
        let mut sim = DuplexSim::new(PayloadKind::Text);
        sim.run(&[
            Step::Obs(StreamObservation::VadOnset),
            Step::Obs(StreamObservation::HeartbeatLost),
        ]);
        assert_eq!(sim.channel.state(), DuplexState::Degraded);
    }
}

// ---------------------------------------------------------------------------
// 3. Loopback duplex simulation (plan §3 test 3) — the four benches
// ---------------------------------------------------------------------------

mod loopback_sim {
    use super::*;

    /// Reach `Speaking` (agent holds the floor, TTS streaming down).
    fn speaking() -> DuplexSim {
        let mut sim = DuplexSim::new(PayloadKind::Audio);
        sim.run(&[
            Step::Obs(StreamObservation::VadOnset),
            Step::Obs(StreamObservation::SmartTurnEou),
            Step::Verdict(FloorVerdict::GrantAgent),
        ]);
        assert_eq!(sim.channel.state(), DuplexState::Speaking);
        sim
    }

    /// **Self-cancel regression** (the no-hardware proof of the 55d33c92 fix): a
    /// low-ERL onset during `Speaking` (the bot's own AEC leakage) enters `Overlap`,
    /// but ECC `DenyBarge` reverts to `Speaking`. The answer is NOT cancelled and NO
    /// `Cancel`/`Mute` reaches the edge — only an idempotent `Resume` (un-duck).
    #[test]
    fn self_cancel_low_erl_onset_reverts() {
        let mut sim = speaking();
        sim.run(&[
            Step::Obs(StreamObservation::Erl(0.08)), // low ERL = echo leakage
            Step::Obs(StreamObservation::VadOnset),  // -> Overlap (provisional)
            Step::Verdict(FloorVerdict::DenyBarge),  // ECC denies (low ERL)
        ]);

        assert!(sim.entered(DuplexState::Overlap), "Overlap must be entered");
        assert_eq!(sim.channel.state(), DuplexState::Speaking, "must revert");
        // The load-bearing invariant: no barge command reached the edge.
        assert!(
            !sim.edge.commands.contains(&EdgeCommand::Mute),
            "self-echo must NOT mute the edge",
        );
        assert!(
            !sim.edge.commands.contains(&EdgeCommand::Cancel),
            "self-echo must NOT cancel the answer",
        );
        assert_eq!(sim.edge.commands, vec![EdgeCommand::Resume]);
        // The onset's claim carried the low ERL up for the floor read.
        assert_eq!(sim.impulses.last().and_then(|i| i.erl), Some(0.08));
    }

    /// **Genuine barge**: a high-ERL onset yields `Overlap → Listening`, the TTS is
    /// cancelled, and `Mute` is delivered to the edge.
    #[test]
    fn genuine_barge_high_erl_grants_user() {
        let mut sim = speaking();
        sim.run(&[
            Step::Obs(StreamObservation::Erl(0.95)), // strong external speech
            Step::Obs(StreamObservation::VadOnset),  // -> Overlap
            Step::Verdict(FloorVerdict::GrantUser),  // ECC grants the barge
        ]);

        assert!(sim.entered(DuplexState::Overlap));
        assert_eq!(sim.channel.state(), DuplexState::Listening);
        assert_eq!(
            sim.edge.commands,
            vec![EdgeCommand::Mute, EdgeCommand::Cancel],
            "a granted barge mutes the speaker and cancels the render",
        );
    }

    /// **Backchannel**: a brief onset ECC-classifies as a continuer; `Speaking`
    /// never yields, no `EdgeCommand` interrupts playback, and the continuer ends
    /// back into `Speaking`.
    #[test]
    fn backchannel_does_not_interrupt_playback() {
        let mut sim = speaking();
        sim.run(&[
            Step::Verdict(FloorVerdict::Backchannel), // "mm-hmm"
            Step::Obs(StreamObservation::VadOffset),  // continuer ends
        ]);

        assert!(sim.entered(DuplexState::Backchannel));
        assert_eq!(sim.channel.state(), DuplexState::Speaking, "playback resumes");
        assert!(
            sim.edge.commands.is_empty(),
            "a backchannel must not interrupt playback",
        );
    }

    /// **Link drop**: `HeartbeatLost` mid-`Speaking` drives `Degraded` with NO
    /// daemon command (the edge reflex owns safety). The daemon marks the
    /// conversation stale by holding `Degraded`.
    #[test]
    fn link_drop_mid_speaking_degrades_without_command() {
        let mut sim = speaking();
        sim.run(&[Step::Obs(StreamObservation::HeartbeatLost)]);

        assert_eq!(sim.channel.state(), DuplexState::Degraded);
        assert!(
            sim.edge.commands.is_empty(),
            "the daemon cannot (and must not) command a dead link",
        );

        // Reconnect + resync returns to Idle and clears the stale ERL.
        sim.run(&[Step::Obs(StreamObservation::LinkRestored)]);
        assert_eq!(sim.channel.state(), DuplexState::Idle);
        assert_eq!(sim.channel.last_erl(), None);
    }

    /// A full normal turn cycle with no barge: Idle → Listening → Thinking →
    /// Speaking → Idle, no edge commands beyond the (absent) render.
    #[test]
    fn normal_turn_cycle() {
        let mut sim = DuplexSim::new(PayloadKind::Audio);
        sim.run(&[
            Step::Obs(StreamObservation::VadOnset),
            Step::Obs(StreamObservation::SmartTurnEou),
            Step::Verdict(FloorVerdict::GrantAgent),
            Step::Obs(StreamObservation::PlaybackDrained),
        ]);
        use DuplexState::*;
        assert_eq!(sim.trajectory, vec![Idle, Listening, Thinking, Speaking, Idle]);
        assert!(sim.edge.commands.is_empty());
    }
}
