//! Energy-based voice activity detector.
//!
//! Frame-by-frame RMS over an `s16le` PCM stream, gated by a dBFS
//! threshold and a silence tail. Emits [`VadEvent`]s on speech-start /
//! speech-end so the channel can slice utterances. Mirrors the model
//! `clawft-service-classify::EnergyClassifier` uses (-45 dBFS default)
//! so behaviour is consistent across the substrate.
//!
//! This is intentionally not Silero / WebRTC-VAD: those pull heavy ML
//! deps and the M5 ADR (WEFT-205) hasn't picked one. Energy-VAD is the
//! lowest-common-denominator that works without a model file and without
//! a runtime dependency on `voice_activity_detector` or sherpa-rs. If
//! WEFT-205 picks a real VAD later, this trait + module is the seam.

/// Events produced by the VAD as PCM frames are fed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadEvent {
    /// Speech onset detected at sample offset `at_sample`.
    SpeechStart {
        /// Cumulative sample index at which the utterance began.
        at_sample: u64,
    },
    /// Speech end detected at sample offset `at_sample`. The captured
    /// utterance covers `[start_sample, at_sample]`.
    SpeechEnd {
        /// Cumulative sample index at which speech started.
        start_sample: u64,
        /// Cumulative sample index at which silence reached the tail.
        at_sample: u64,
    },
}

/// Streaming energy-RMS VAD.
///
/// Hold one of these per capture stream. Push frames via [`Self::feed`];
/// it returns zero or more `VadEvent`s. State is fully internal.
#[derive(Debug)]
pub struct EnergyVad {
    sample_rate: u32,
    threshold_dbfs: f32,
    silence_tail_samples: u64,
    min_utterance_samples: u64,
    max_utterance_samples: u64,
    cumulative: u64,
    in_speech: bool,
    speech_start: u64,
    silence_run: u64,
    /// Samples of *active speech* observed since the most recent
    /// `SpeechStart`. Excludes silence frames; this is what
    /// `min_utterance_samples` is checked against, so a brief blip
    /// followed by a long silence tail does not count as an utterance.
    speech_samples: u64,
}

impl EnergyVad {
    /// Build a VAD.
    ///
    /// `silence_ms` is the trailing-silence window that ends an
    /// utterance. `min_utterance_ms` and `max_utterance_ms` clamp the
    /// emitted segment lengths.
    pub fn new(
        sample_rate: u32,
        threshold_dbfs: f32,
        silence_ms: u32,
        min_utterance_ms: u32,
        max_utterance_ms: u32,
    ) -> Self {
        let s = u64::from(sample_rate);
        Self {
            sample_rate,
            threshold_dbfs,
            silence_tail_samples: s * u64::from(silence_ms) / 1_000,
            min_utterance_samples: s * u64::from(min_utterance_ms) / 1_000,
            max_utterance_samples: s * u64::from(max_utterance_ms) / 1_000,
            cumulative: 0,
            in_speech: false,
            speech_start: 0,
            silence_run: 0,
            speech_samples: 0,
        }
    }

    /// Sample rate the VAD was constructed with.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Compute the RMS energy of `frame` in dBFS, clamped to [-100, 0].
    ///
    /// Empty frames return -100 (treated as silence).
    pub fn rms_dbfs(frame: &[i16]) -> f32 {
        if frame.is_empty() {
            return -100.0;
        }
        let sum_sq: f64 = frame.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = (sum_sq / frame.len() as f64).sqrt();
        if rms <= 0.0 {
            return -100.0;
        }
        // i16 full-scale = 32_768.
        let dbfs = 20.0 * (rms / 32_768.0).log10();
        dbfs.clamp(-100.0, 0.0) as f32
    }

    /// Whether the VAD currently believes speech is active.
    pub fn in_speech(&self) -> bool {
        self.in_speech
    }

    /// Push a frame of PCM samples and collect events.
    ///
    /// Returns the events produced by this frame (zero, one, or — in
    /// degenerate cases like a max-length flush followed by a continued
    /// frame — two).
    pub fn feed(&mut self, frame: &[i16]) -> Vec<VadEvent> {
        let mut events = Vec::new();
        if frame.is_empty() {
            return events;
        }
        let dbfs = Self::rms_dbfs(frame);
        let frame_len = frame.len() as u64;
        let is_speech = dbfs >= self.threshold_dbfs;

        if !self.in_speech {
            if is_speech {
                self.in_speech = true;
                self.speech_start = self.cumulative;
                self.silence_run = 0;
                self.speech_samples = frame_len;
                events.push(VadEvent::SpeechStart {
                    at_sample: self.speech_start,
                });
            }
        } else {
            if is_speech {
                self.silence_run = 0;
                self.speech_samples = self.speech_samples.saturating_add(frame_len);
            } else {
                self.silence_run = self.silence_run.saturating_add(frame_len);
            }
            // End-of-speech if silence tail exceeded.
            let end_at = self.cumulative + frame_len;
            let utterance_len = end_at.saturating_sub(self.speech_start);
            let end_by_silence = self.silence_run >= self.silence_tail_samples;
            let end_by_max = utterance_len >= self.max_utterance_samples;
            if end_by_silence || end_by_max {
                // `min_utterance_samples` is checked against speech-only
                // duration so brief blips followed by long silence don't
                // count as utterances.
                if self.speech_samples >= self.min_utterance_samples {
                    events.push(VadEvent::SpeechEnd {
                        start_sample: self.speech_start,
                        at_sample: end_at,
                    });
                }
                self.in_speech = false;
                self.silence_run = 0;
                self.speech_samples = 0;
            }
        }

        self.cumulative = self.cumulative.saturating_add(frame_len);
        events
    }
}

/// Adaptive noise-floor tracker (dBFS).
///
/// Room tone varies machine-to-machine (fans, HVAC, street noise) *and* with
/// the OS input-gain slider, which the user may change between sessions or
/// mid-session. A fixed dBFS gate mis-classifies a loud room as permanent
/// speech and the silence-based endpointer never finalizes a turn (observed
/// live: a -37 dBFS room vs the -45 dBFS default gate — Talk-Mode "never heard"
/// anyone). Two failure directions have to be defended at once:
///
/// * **Gate stuck CLOSED** — quiet speech drags a chasing floor up under
///   itself until only word attacks clear the gate (round-6). Defence: gate at
///   `floor + margin` (Schmitt-trigger) and **freeze the floor while voiced**,
///   learning it only from genuine silence.
/// * **Gate stuck OPEN** — room tone above the frozen floor reads as perpetual
///   speech, so no genuine-silence frame ever arrives, the floor can never
///   learn up, and the endpointer never fires (round-7, triggered the instant
///   the user raised input gain). Defence: **startup calibration** (seed the
///   floor from the first `CALIB_MS` of a session, assumed non-speech, so any
///   gain setting is handled at boot) plus a **stuck-open watchdog** (if
///   "speech" persists past `WATCHDOG_MS` with no endpoint, force-recalibrate
///   the floor from the quietest recent frames — a real monologue has
///   micro-pauses, so continuous voicing that long means the floor is wrong).
#[derive(Debug)]
pub struct NoiseFloor {
    floor_dbfs: f32,
    margin_db: f32,
    /// Whether startup calibration has completed (floor seeded from the first
    /// `CALIB_MS`). Until then `classify` buffers frames and reports non-voiced.
    calibrated: bool,
    /// dBFS of frames seen during the calibration window (median → floor seed).
    calib_frames: Vec<f32>,
    /// Samples accumulated toward the `calib_target_samples` window.
    calib_samples: u64,
    /// Calibration window length in samples (from the sample rate).
    calib_target_samples: u64,
    /// Whether the gate currently reads speech (drives the sustain hysteresis
    /// threshold and the floor freeze).
    in_speech: bool,
    /// Remaining hangover (samples): while > 0 a below-threshold frame is still
    /// treated as voiced so a brief intra-word dip doesn't close the gate.
    hangover_remaining: u64,
    /// Hangover length in samples (from the sample rate).
    hangover_samples: u64,
    /// Count of frames judged genuine silence. The floor only learns from these
    /// (see [`converged`](Self::converged)).
    quiet_frames: u32,
    /// Continuous voiced run (samples, including hangover). Reset by any
    /// genuine-silence frame; when it reaches `watchdog_samples` the stuck-open
    /// watchdog fires.
    voiced_run_samples: u64,
    /// Watchdog trip point in samples (from the sample rate).
    watchdog_samples: u64,
    /// Recent per-frame `(dbfs, len)` over ~`WATCHDOG_WINDOW_MS`; the watchdog
    /// recalibrates the floor from the p10 of this ring.
    recent: std::collections::VecDeque<(f32, u64)>,
    /// Sum of `len` in `recent`, for O(1) window eviction.
    recent_samples: u64,
    /// Rolling window length in samples (from the sample rate).
    recent_target_samples: u64,
    /// How many times the stuck-open watchdog has force-recalibrated the floor.
    /// Surfaced for diagnostics / tests.
    recalibrations: u32,
}

/// Silence frames the floor must observe before it is a trustworthy noise
/// estimate — until then SNR is flagged unreliable in the record.
const FLOOR_CONVERGE_FRAMES: u32 = 10;

/// Startup calibration window: the first this-many ms of a session are assumed
/// non-speech and seed the floor from their median. Handles any input-gain
/// setting at boot (the round-7 stuck-open cause). A user who starts talking
/// inside this window loses only the opening ~half second.
const CALIB_MS: u32 = 500;

/// Hangover tail: once voiced, stay in capture until this much CONTINUOUS
/// below-threshold audio, so brief intra-word dips (stop closures, unvoiced
/// consonants) don't split an utterance.
const HANGOVER_MS: u32 = 400;

/// Stuck-open watchdog trip: continuous voicing longer than this with no
/// intervening genuine-silence frame means the floor is miscalibrated (a real
/// monologue breathes), so recalibrate from the quietest recent frames.
const WATCHDOG_MS: u32 = 15_000;

/// Rolling window the watchdog samples for its p10 recalibration floor.
const WATCHDOG_WINDOW_MS: u32 = 5_000;

/// Percentile of the recent window used as the recalibrated floor (quietest
/// tenth ≈ the true room tone even while the mean is dominated by speech).
const WATCHDOG_PCT: usize = 10;

impl NoiseFloor {
    /// `margin_db` is the strict onset margin over the tracked floor a frame
    /// must clear to START speech (8–10 dB at conversational distance); the
    /// loose sustain threshold is half that. `sample_rate` sizes the calibration
    /// window, hangover, and watchdog timers.
    pub fn new(margin_db: f32, sample_rate: u32) -> Self {
        let sr = u64::from(sample_rate);
        Self {
            floor_dbfs: -100.0,
            margin_db,
            calibrated: false,
            calib_frames: Vec::new(),
            calib_samples: 0,
            calib_target_samples: sr * u64::from(CALIB_MS) / 1_000,
            in_speech: false,
            hangover_remaining: 0,
            hangover_samples: sr * u64::from(HANGOVER_MS) / 1_000,
            quiet_frames: 0,
            voiced_run_samples: 0,
            watchdog_samples: sr * u64::from(WATCHDOG_MS) / 1_000,
            recent: std::collections::VecDeque::new(),
            recent_samples: 0,
            recent_target_samples: sr * u64::from(WATCHDOG_WINDOW_MS) / 1_000,
            recalibrations: 0,
        }
    }

    /// Classify one frame as voiced. Applies a **Schmitt-trigger** gate (strict
    /// onset at `floor+margin`, loose sustain at `floor+margin/2`), a
    /// **hangover** tail so intra-word dips don't close the gate, and — the key
    /// discipline — **freezes the floor while in speech/hangover**, learning it
    /// ONLY from genuine silence (fast down, very slow up). Startup calibration
    /// seeds the floor from the first [`CALIB_MS`] (assumed non-speech) so the
    /// gate is correct at any input gain from boot; a stuck-open watchdog
    /// force-recalibrates the floor from the quietest recent frames if voicing
    /// runs past [`WATCHDOG_MS`] with no genuine-silence frame — the escape
    /// hatch from a gate wedged OPEN by a mid-session gain jump.
    pub fn classify(&mut self, dbfs: f32, frame_len: u64) -> bool {
        // Startup calibration: buffer the first CALIB_MS as non-speech and seed
        // the floor from their median. Report non-voiced throughout the window.
        if !self.calibrated {
            self.calib_frames.push(dbfs);
            self.calib_samples = self.calib_samples.saturating_add(frame_len);
            if self.calib_samples >= self.calib_target_samples {
                self.floor_dbfs = median(&self.calib_frames);
                self.calibrated = true;
            }
            return false;
        }

        self.push_recent(dbfs, frame_len);

        // Stuck-open watchdog: continuous voicing past the trip point without a
        // genuine-silence frame means the floor is wrong. Recalibrate it to the
        // p10 of the recent window before this frame's gate decision so the very
        // frame that tripped the watchdog is re-judged against the true floor.
        if self.voiced_run_samples >= self.watchdog_samples {
            let recalibrated = self.recent_percentile(WATCHDOG_PCT);
            let old = self.floor_dbfs;
            self.floor_dbfs = recalibrated;
            self.in_speech = false;
            self.hangover_remaining = 0;
            self.voiced_run_samples = 0;
            self.recalibrations = self.recalibrations.saturating_add(1);
            tracing::warn!(
                old_floor_dbfs = old,
                new_floor_dbfs = recalibrated,
                "noise-floor watchdog: gate stuck open — recalibrated from quiet p10"
            );
        }

        let onset = (self.floor_dbfs + self.margin_db).clamp(-100.0, 0.0);
        let sustain = (self.floor_dbfs + self.margin_db * 0.5).clamp(-100.0, 0.0);
        let gate = if self.in_speech { sustain } else { onset };

        if dbfs >= gate {
            self.in_speech = true;
            self.hangover_remaining = self.hangover_samples;
            self.voiced_run_samples = self.voiced_run_samples.saturating_add(frame_len);
            true
        } else if self.hangover_remaining > 0 {
            self.hangover_remaining = self.hangover_remaining.saturating_sub(frame_len);
            self.voiced_run_samples = self.voiced_run_samples.saturating_add(frame_len);
            true
        } else {
            // Genuine silence: leave speech and LEARN the floor (asymmetric —
            // fast toward quieter evidence, very slow upward for ambient drift).
            self.in_speech = false;
            self.voiced_run_samples = 0;
            let delta = dbfs - self.floor_dbfs;
            self.floor_dbfs += if delta < 0.0 { delta * 0.3 } else { delta * 0.02 };
            self.quiet_frames = self.quiet_frames.saturating_add(1);
            false
        }
    }

    /// Push a frame into the rolling watchdog window, evicting stale samples.
    fn push_recent(&mut self, dbfs: f32, frame_len: u64) {
        self.recent.push_back((dbfs, frame_len));
        self.recent_samples = self.recent_samples.saturating_add(frame_len);
        while self.recent_samples > self.recent_target_samples && self.recent.len() > 1 {
            if let Some((_, len)) = self.recent.pop_front() {
                self.recent_samples = self.recent_samples.saturating_sub(len);
            }
        }
    }

    /// The `pct`-th percentile dBFS of the recent window (quietest tenth ≈ the
    /// true room tone). Falls back to the current floor if the window is empty.
    fn recent_percentile(&self, pct: usize) -> f32 {
        if self.recent.is_empty() {
            return self.floor_dbfs;
        }
        let mut vals: Vec<f32> = self.recent.iter().map(|&(d, _)| d).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (vals.len().saturating_sub(1)) * pct / 100;
        vals[idx]
    }

    /// Current voice threshold (`floor + margin`).
    pub fn threshold_dbfs(&self) -> f32 {
        (self.floor_dbfs + self.margin_db).clamp(-100.0, 0.0)
    }

    /// Current tracked floor.
    pub fn floor_dbfs(&self) -> f32 {
        self.floor_dbfs
    }

    /// How many times the stuck-open watchdog has force-recalibrated the floor
    /// this session. Non-zero means the gate had to be rescued from a wedged
    /// state (usually a mid-session input-gain jump).
    pub fn recalibrations(&self) -> u32 {
        self.recalibrations
    }

    /// Whether the floor has observed enough real silence to be a trustworthy
    /// noise estimate. Until this is true (e.g. the very first turn of a
    /// speech-first session), any SNR derived from the floor is unreliable and
    /// should be flagged in the record.
    pub fn converged(&self) -> bool {
        self.quiet_frames >= FLOOR_CONVERGE_FRAMES
    }
}

/// Median of a dBFS slice (used to seed the floor from the calibration window).
fn median(vals: &[f32]) -> f32 {
    if vals.is_empty() {
        return -100.0;
    }
    let mut v = vals.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tone frame at the given amplitude.
    fn tone(len: usize, amp: i16) -> Vec<i16> {
        (0..len)
            .map(|i| if i % 2 == 0 { amp } else { -amp })
            .collect()
    }

    #[test]
    fn rms_silence_is_minus_100() {
        assert_eq!(EnergyVad::rms_dbfs(&[0i16; 256]), -100.0);
    }

    #[test]
    fn rms_full_scale_is_zero() {
        let v = vec![i16::MAX; 256];
        let dbfs = EnergyVad::rms_dbfs(&v);
        assert!(dbfs > -1.0, "expected near-zero dBFS, got {dbfs}");
    }

    #[test]
    fn rms_half_scale_is_around_minus_six() {
        let v = vec![16_000i16; 256];
        let dbfs = EnergyVad::rms_dbfs(&v);
        assert!(dbfs > -8.0 && dbfs < -4.0, "got {dbfs}");
    }

    #[test]
    fn vad_speech_then_silence_tail_emits_pair() {
        // 16 kHz, -45 dBFS threshold, 300 ms silence tail, 100 ms min,
        // 10 s max.
        let mut vad = EnergyVad::new(16_000, -45.0, 300, 100, 10_000);
        let frame = 1_600; // 100 ms
        // ~3000 amplitude → ~-20 dBFS, well above -45.
        let speech = tone(frame, 3_000);
        let silence = vec![0i16; frame];

        // 200 ms speech.
        let e1 = vad.feed(&speech);
        assert!(matches!(e1.first(), Some(VadEvent::SpeechStart { .. })));
        let e2 = vad.feed(&speech);
        assert!(e2.is_empty());
        assert!(vad.in_speech());

        // 400 ms silence — should cross the 300 ms tail and emit end.
        let e3 = vad.feed(&silence);
        let e4 = vad.feed(&silence);
        let e5 = vad.feed(&silence);
        let e6 = vad.feed(&silence);
        let ended = [e3, e4, e5, e6]
            .iter()
            .flatten()
            .any(|ev| matches!(ev, VadEvent::SpeechEnd { .. }));
        assert!(ended, "expected SpeechEnd within silence frames");
        assert!(!vad.in_speech());
    }

    #[test]
    fn vad_drops_below_min_utterance() {
        let mut vad = EnergyVad::new(16_000, -45.0, 200, 500, 10_000);
        let frame = 800; // 50 ms
        let speech = tone(frame, 3_000);
        let silence = vec![0i16; 16_000]; // 1 s
        // 50 ms speech (< 500 ms min) then silence.
        let e1 = vad.feed(&speech);
        assert!(matches!(e1.first(), Some(VadEvent::SpeechStart { .. })));
        let e2 = vad.feed(&silence);
        let no_end = !e2.iter().any(|ev| matches!(ev, VadEvent::SpeechEnd { .. }));
        assert!(no_end, "should drop sub-min utterance silently");
        assert!(!vad.in_speech());
    }

    #[test]
    fn vad_max_utterance_forces_flush() {
        // 200 ms max, 500 ms silence tail. Continuous speech should hit
        // the max-utterance ceiling and flush even though no silence
        // was observed. After the flush the next speech frame starts a
        // fresh utterance, so we just assert the flush happened.
        let mut vad = EnergyVad::new(16_000, -45.0, 500, 50, 200);
        let frame = 1_600; // 100 ms
        let speech = tone(frame, 3_000);
        let mut all_events = Vec::new();
        for _ in 0..3 {
            all_events.extend(vad.feed(&speech));
        }
        let ended_count = all_events
            .iter()
            .filter(|ev| matches!(ev, VadEvent::SpeechEnd { .. }))
            .count();
        assert!(
            ended_count >= 1,
            "max-len ceiling should force at least one flush"
        );
    }

    #[test]
    fn vad_does_not_emit_on_pure_silence() {
        let mut vad = EnergyVad::new(16_000, -45.0, 300, 100, 10_000);
        let silence = vec![0i16; 16_000];
        let events = vad.feed(&silence);
        assert!(events.is_empty());
        assert!(!vad.in_speech());
    }

    const F100: u64 = 1_600; // 100 ms @ 16 kHz

    #[test]
    fn floor_frozen_through_quiet_speech_no_truncation() {
        // Round-6 acceptance: at low SNR a full sentence stays ONE open
        // utterance and the floor stays flat across it. Learn a low floor from
        // leading silence, then feed 3 s of quiet speech (~10 dB over floor)
        // with periodic intra-word dips — every frame must read voiced and the
        // floor must not creep up under the speech.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..20 {
            nf.classify(-65.0, F100); // leading silence
        }
        let floor_before = nf.floor_dbfs();
        assert!(floor_before < -60.0, "floor learned from silence: {floor_before}");

        let mut voiced = 0;
        for i in 0..30 {
            // Speech at -55 with a brief dip to -63 (below the sustain gate) —
            // the hangover must bridge it.
            let dbfs = if i % 5 == 4 { -63.0 } else { -55.0 };
            if nf.classify(dbfs, F100) {
                voiced += 1;
            }
        }
        assert_eq!(voiced, 30, "every quiet-speech frame (incl. dips) stays voiced");
        let floor_after = nf.floor_dbfs();
        assert!(
            (floor_after - floor_before).abs() < 2.0,
            "floor must stay flat through speech: {floor_before} -> {floor_after}"
        );
    }

    #[test]
    fn floor_flat_across_speech_heavy_session() {
        // Acceptance: floor delta < 2 dB before vs after 10 utterances.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..15 {
            nf.classify(-65.0, F100); // establish the floor
        }
        let before = nf.floor_dbfs();
        for _ in 0..10 {
            for _ in 0..15 {
                nf.classify(-55.0, F100); // ~1.5 s of speech
            }
            for _ in 0..8 {
                nf.classify(-65.0, F100); // inter-utterance silence
            }
        }
        let after = nf.floor_dbfs();
        assert!(
            (after - before).abs() < 2.0,
            "floor drifted across a speech-heavy session: {before} -> {after}"
        );
    }

    #[test]
    fn hysteresis_strict_onset_loose_sustain() {
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..10 {
            nf.classify(-65.0, F100); // floor ~-65 at rest
        }
        // At rest a -60 frame (+5 over floor) is BELOW the strict onset gate
        // (floor+8 ≈ -57) → not voiced.
        assert!(!nf.classify(-60.0, F100), "onset gate is strict");
        // A -55 frame (+10) clears onset and enters speech.
        assert!(nf.classify(-55.0, F100), "-55 clears onset");
        // Now a -60 frame stays voiced — the sustain gate (floor+4) is looser.
        assert!(nf.classify(-60.0, F100), "sustain gate is loose");
    }

    #[test]
    fn hangover_bridges_dips_then_closes() {
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..10 {
            nf.classify(-65.0, F100);
        }
        nf.classify(-55.0, F100); // enter speech → 400 ms hangover
        // A deep dip well below the sustain gate stays voiced for <400 ms.
        for ms in 1..=4 {
            assert!(nf.classify(-90.0, F100), "dip at {}00ms within hangover", ms);
        }
        // 5th consecutive 100 ms silence exceeds the 400 ms tail → gate closes.
        assert!(!nf.classify(-90.0, F100), "gate closes after 400ms continuous silence");
    }

    #[test]
    fn noise_floor_converges_only_after_seeing_silence() {
        let mut nf = NoiseFloor::new(8.0, 16_000);
        assert!(!nf.converged(), "fresh floor is unconverged");
        nf.classify(-20.0, F100); // still inside the calibration window → not silence
        assert!(!nf.converged(), "calibration alone does not converge the floor");
        // Enough genuine-silence frames past calibration converge it.
        for _ in 0..(FLOOR_CONVERGE_FRAMES + 5) {
            nf.classify(-70.0, F100);
        }
        assert!(nf.converged(), "floor converges once it has seen silence");
    }

    #[test]
    fn startup_calibration_seeds_floor_from_first_500ms() {
        // Round-7: with raised input gain the room tone sits at -40 dBFS, above
        // the old -53 init cap. Calibration must seat the floor at the ACTUAL
        // level, not a fixed cap — five 100 ms frames fill the 500 ms window.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..5 {
            assert!(!nf.classify(-40.0, F100), "calibration window reads non-speech");
        }
        assert!(
            (nf.floor_dbfs() - (-40.0)).abs() < 1.0,
            "floor seeded to room tone, got {}",
            nf.floor_dbfs()
        );
    }

    #[test]
    fn calibration_window_reports_non_voiced_even_if_loud() {
        // The first 500 ms are assumed non-speech regardless of level, so the
        // floor seed is never poisoned by a session that starts mid-utterance.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for i in 0..5 {
            assert!(!nf.classify(-10.0, F100), "frame {i} in calib window is non-voiced");
        }
    }

    #[test]
    fn high_gain_room_reads_silence_not_perpetual_speech() {
        // The round-7 blocker: at raised gain, -40 dBFS room tone under the
        // pre-calibration gate (init cap -53, onset -45) read as PERPETUAL
        // speech and the endpointer never fired → zero renders. Calibration
        // seats the floor at -40; continued room tone must now read SILENCE and
        // real speech must still voice.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..5 {
            nf.classify(-40.0, F100); // calibration
        }
        let mut voiced = 0;
        for _ in 0..20 {
            if nf.classify(-40.0, F100) {
                voiced += 1;
            }
        }
        assert_eq!(voiced, 0, "high-gain room tone must not read as speech");
        assert!(
            nf.classify(-25.0, F100),
            "speech above the calibrated floor still voices"
        );
    }

    #[test]
    fn stuck_open_watchdog_recovers_within_15s() {
        // Inverse failure: a mid-session input-gain jump lifts room tone above
        // the FROZEN floor+margin, wedging the gate OPEN — no genuine-silence
        // frame ever arrives, so only the watchdog can recover it. Calibrate to
        // a quiet floor, then feed continuous -40 (jumped room tone) past the
        // 15 s trip point; the watchdog must recalibrate to the quiet p10 and
        // the gate must read silence again so the endpointer can finalize.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..5 {
            nf.classify(-65.0, F100); // calibrate to a quiet -65 floor
        }
        // 15 s = 150 frames of 100 ms; the watchdog trips on the 151st voiced
        // frame. Run a little past it and confirm recovery.
        let mut last = true;
        for _ in 0..160 {
            last = nf.classify(-40.0, F100);
        }
        assert!(
            nf.recalibrations() >= 1,
            "watchdog must fire on 15 s+ continuous voicing"
        );
        assert!(!last, "gate recovers to silence after watchdog recalibration");
        assert!(
            (nf.floor_dbfs() - (-40.0)).abs() < 2.0,
            "floor recalibrated to the room tone, got {}",
            nf.floor_dbfs()
        );
    }

    #[test]
    fn watchdog_does_not_defeat_the_freeze_on_normal_turns() {
        // Discriminator: sustained level WITH endpoint-firing turns (speech +
        // micro-pauses) must NOT trip the watchdog — only never-ending voicing
        // does. Ten 1.5 s utterances separated by silence: zero recalibrations.
        let mut nf = NoiseFloor::new(8.0, 16_000);
        for _ in 0..15 {
            nf.classify(-65.0, F100);
        }
        for _ in 0..10 {
            for _ in 0..15 {
                nf.classify(-55.0, F100); // 1.5 s speech
            }
            for _ in 0..8 {
                nf.classify(-65.0, F100); // 800 ms silence — resets voiced run
            }
        }
        assert_eq!(nf.recalibrations(), 0, "normal turns must not trip the watchdog");
    }
}
