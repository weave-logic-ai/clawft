//! Voice confirmation gate (SC-6 / WEFT-225).
//!
//! Destructive voice actions require a spoken confirmation that includes
//! a one-shot **anti-replay nonce** and (optionally) a **transcription
//! echo** of the pending action:
//!
//! ```text
//! I heard: 'delete the config file'. Say 'confirm delta' to proceed.
//! ```
//!
//! The nonce word changes every prompt so a pre-recorded "yes" / "confirm"
//! cannot replay. `VoiceCommand.confirm` is plumbed through
//! [`ConfirmationGate::begin_for_command`].
//!
//! # Timeouts
//!
//! If no valid confirmation arrives within
//! [`VoiceConfirmationConfig::timeout_seconds`] (default 10 s), the pending
//! action is cancelled.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clawft_types::config::VoiceConfirmationConfig;
use tracing::{debug, info};

use super::commands::VoiceCommand;

/// Tracing target for SC-6 confirmation audit lines (pairs with SC-9).
pub const CONFIRM_TARGET: &str = "voice.confirm";

/// Short, STT-friendly nonce vocabulary (NATO-ish + common words).
/// Kept short so ASR reliably hears them; rotated per confirmation.
const NONCE_WORDS: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
    "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa",
    "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey",
    "xray", "yankee", "zulu", "amber", "coral", "maple", "cedar", "river",
    "stone", "flame", "orbit", "prism", "comet",
];

/// Source of anti-replay nonce words. Production uses wall-clock entropy;
/// tests inject a fixed sequence.
pub trait NonceSource: Send + Sync {
    /// Produce the next nonce word (lowercase, single token).
    fn next_nonce(&self) -> String;
}

/// Default nonce source: `SystemTime` nanos XOR a process-local counter,
/// mapped into [`NONCE_WORDS`].
#[derive(Debug, Default)]
pub struct EntropyNonceSource {
    counter: AtomicU64,
}

impl NonceSource for EntropyNonceSource {
    fn next_nonce(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let idx = (nanos ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15)) as usize % NONCE_WORDS.len();
        NONCE_WORDS[idx].to_string()
    }
}

/// Fixed-sequence nonce source for deterministic tests.
#[derive(Debug)]
pub struct FixedNonceSource {
    words: Vec<String>,
    idx: AtomicU64,
}

impl FixedNonceSource {
    /// Build from an ordered list of nonce words (cycled).
    pub fn new(words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let words: Vec<String> = words.into_iter().map(Into::into).collect();
        assert!(!words.is_empty(), "FixedNonceSource needs at least one word");
        Self {
            words,
            idx: AtomicU64::new(0),
        }
    }
}

impl NonceSource for FixedNonceSource {
    fn next_nonce(&self) -> String {
        let i = self.idx.fetch_add(1, Ordering::Relaxed) as usize % self.words.len();
        self.words[i].clone()
    }
}

/// A pending confirmation that must be spoken back before an action runs.
#[derive(Debug, Clone)]
pub struct PendingConfirmation {
    /// Human-readable action being confirmed (also used for transcription echo).
    pub action: String,
    /// Tool name (when the action came from a [`VoiceCommand`]).
    pub tool: Option<String>,
    /// Anti-replay nonce word (lowercase). Empty when nonce is disabled.
    pub nonce: String,
    /// Prompt text intended for TTS playback.
    pub prompt: String,
    /// When the confirmation was issued.
    pub issued_at: Instant,
    /// Absolute deadline; after this the confirmation is stale.
    pub deadline: Instant,
}

impl PendingConfirmation {
    /// True when the confirmation window has expired.
    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }

    /// Expected spoken phrase when nonce is enabled: `confirm <nonce>`.
    pub fn expected_phrase(&self) -> Option<String> {
        if self.nonce.is_empty() {
            None
        } else {
            Some(format!("confirm {}", self.nonce))
        }
    }
}

/// Outcome of submitting a spoken reply against a pending confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmResult {
    /// User spoke the correct nonce / yes; action may proceed.
    Confirmed,
    /// Transcript did not match; keep waiting (within timeout).
    Rejected,
    /// No pending confirmation, or it already expired / was cancelled.
    NoPending,
    /// Confirmation window timed out; action cancelled.
    TimedOut,
}

/// SC-6 confirmation gate: one pending action at a time.
pub struct ConfirmationGate {
    config: VoiceConfirmationConfig,
    nonce_source: Box<dyn NonceSource>,
    pending: Option<PendingConfirmation>,
}

impl ConfirmationGate {
    /// Build a gate with the given config and entropy-backed nonces.
    pub fn new(config: VoiceConfirmationConfig) -> Self {
        Self {
            config,
            nonce_source: Box::new(EntropyNonceSource::default()),
            pending: None,
        }
    }

    /// Build a gate with an injectable nonce source (tests).
    pub fn with_nonce_source(
        config: VoiceConfirmationConfig,
        nonce_source: Box<dyn NonceSource>,
    ) -> Self {
        Self {
            config,
            nonce_source,
            pending: None,
        }
    }

    /// Default SC-6 settings (10 s timeout, nonce + echo on).
    pub fn with_defaults() -> Self {
        Self::new(VoiceConfirmationConfig::default())
    }

    /// Configuration snapshot.
    pub fn config(&self) -> &VoiceConfirmationConfig {
        &self.config
    }

    /// Currently pending confirmation, if any.
    pub fn pending(&self) -> Option<&PendingConfirmation> {
        self.pending.as_ref()
    }

    /// Drop any pending confirmation without executing.
    pub fn cancel(&mut self) {
        if let Some(p) = self.pending.take() {
            info!(
                target: CONFIRM_TARGET,
                event = "voice.confirm_cancelled",
                action = %p.action,
                nonce = %p.nonce,
                "voice confirmation cancelled"
            );
        }
    }

    /// Begin confirmation for a free-form action description.
    ///
    /// Returns the TTS prompt. Replaces any prior pending confirmation.
    pub fn begin(&mut self, action: impl Into<String>) -> PendingConfirmation {
        self.begin_inner(action.into(), None)
    }

    /// Begin confirmation for a [`VoiceCommand`] whose `confirm` flag is set.
    ///
    /// Callers should only invoke this when `cmd.confirm` is true; the gate
    /// itself does not re-check the flag so non-command destructive paths can
    /// share the same machinery.
    pub fn begin_for_command(&mut self, cmd: &VoiceCommand, heard: &str) -> PendingConfirmation {
        let action = if heard.trim().is_empty() {
            format!("{} ({})", cmd.description, cmd.tool)
        } else {
            heard.trim().to_string()
        };
        self.begin_inner(action, Some(cmd.tool.clone()))
    }

    fn begin_inner(&mut self, action: String, tool: Option<String>) -> PendingConfirmation {
        let nonce = if self.config.anti_replay_nonce {
            self.nonce_source.next_nonce().to_lowercase()
        } else {
            String::new()
        };

        let prompt = build_confirm_prompt(
            &action,
            &nonce,
            self.config.transcription_echo,
            self.config.anti_replay_nonce,
        );

        let now = Instant::now();
        let timeout = Duration::from_secs(u64::from(self.config.timeout_seconds.max(1)));
        let pending = PendingConfirmation {
            action: action.clone(),
            tool,
            nonce: nonce.clone(),
            prompt: prompt.clone(),
            issued_at: now,
            deadline: now + timeout,
        };

        info!(
            target: CONFIRM_TARGET,
            event = "voice.confirm_prompt",
            action = %pending.action,
            tool = ?pending.tool,
            nonce = %pending.nonce,
            timeout_s = self.config.timeout_seconds,
            "voice confirmation prompted"
        );

        self.pending = Some(pending.clone());
        pending
    }

    /// Evaluate a transcription against the pending confirmation.
    ///
    /// Uses `Instant::now()` for the timeout check. Prefer
    /// [`try_confirm_at`] in tests.
    pub fn try_confirm(&mut self, transcription: &str) -> ConfirmResult {
        self.try_confirm_at(transcription, Instant::now())
    }

    /// Evaluate a transcription at an explicit clock instant (testable).
    pub fn try_confirm_at(&mut self, transcription: &str, now: Instant) -> ConfirmResult {
        let Some(pending) = self.pending.as_ref() else {
            return ConfirmResult::NoPending;
        };

        if pending.is_expired(now) {
            let action = pending.action.clone();
            let nonce = pending.nonce.clone();
            self.pending = None;
            info!(
                target: CONFIRM_TARGET,
                event = "voice.confirm_timeout",
                action = %action,
                nonce = %nonce,
                "voice confirmation timed out"
            );
            return ConfirmResult::TimedOut;
        }

        let matched = if pending.nonce.is_empty() {
            // Nonce disabled: accept plain affirmative phrases.
            matches_affirmative(transcription)
        } else {
            matches_confirm_nonce(transcription, &pending.nonce)
        };

        if matched {
            let action = pending.action.clone();
            let nonce = pending.nonce.clone();
            self.pending = None;
            info!(
                target: CONFIRM_TARGET,
                event = "voice.confirm_accepted",
                action = %action,
                nonce = %nonce,
                response = "confirmed",
                "voice confirmation accepted"
            );
            ConfirmResult::Confirmed
        } else {
            debug!(
                target: CONFIRM_TARGET,
                event = "voice.confirm_rejected",
                heard = %transcription,
                expected_nonce = %pending.nonce,
                "voice confirmation phrase did not match"
            );
            ConfirmResult::Rejected
        }
    }

    /// Expire a pending confirmation if past its deadline.
    ///
    /// Returns `TimedOut` if something was cleared, `NoPending` otherwise.
    pub fn poll_timeout(&mut self, now: Instant) -> ConfirmResult {
        let Some(pending) = self.pending.as_ref() else {
            return ConfirmResult::NoPending;
        };
        if pending.is_expired(now) {
            let action = pending.action.clone();
            let nonce = pending.nonce.clone();
            self.pending = None;
            info!(
                target: CONFIRM_TARGET,
                event = "voice.confirm_timeout",
                action = %action,
                nonce = %nonce,
                "voice confirmation timed out"
            );
            ConfirmResult::TimedOut
        } else {
            ConfirmResult::NoPending
        }
    }
}

/// Build the TTS prompt for a confirmation request.
pub fn build_confirm_prompt(
    action: &str,
    nonce: &str,
    transcription_echo: bool,
    anti_replay_nonce: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if transcription_echo {
        parts.push(format!("I heard: '{action}'."));
    } else {
        parts.push(format!("About to: {action}."));
    }
    if anti_replay_nonce && !nonce.is_empty() {
        parts.push(format!("Say 'confirm {nonce}' to proceed."));
    } else {
        parts.push("Say 'yes' to confirm.".into());
    }
    parts.join(" ")
}

/// True when the transcript contains `confirm <nonce>` (order-preserving,
/// case-insensitive). Allows surrounding words ("yes confirm delta please").
pub fn matches_confirm_nonce(transcription: &str, nonce: &str) -> bool {
    let lower = transcription.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let nonce = nonce.to_lowercase();
    for window in words.windows(2) {
        if window[0] == "confirm" && window[1] == nonce.as_str() {
            return true;
        }
    }
    // Also accept the compact form as a substring for ASR gluing.
    let needle = format!("confirm {nonce}");
    lower.contains(&needle)
}

/// Plain affirmative match when anti-replay nonce is disabled.
fn matches_affirmative(transcription: &str) -> bool {
    let lower = transcription.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    words.iter().any(|w| {
        matches!(
            *w,
            "yes" | "yeah" | "yep" | "confirm" | "confirmed" | "ok" | "okay" | "sure"
        )
    })
}

/// Whether a matched [`VoiceCommand`] needs the SC-6 confirmation gate.
#[inline]
pub fn command_requires_confirm(cmd: &VoiceCommand) -> bool {
    cmd.confirm
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_with_nonce(word: &str) -> ConfirmationGate {
        ConfirmationGate::with_nonce_source(
            VoiceConfirmationConfig::default(),
            Box::new(FixedNonceSource::new([word])),
        )
    }

    #[test]
    fn begin_includes_echo_and_nonce() {
        let mut gate = gate_with_nonce("delta");
        let pending = gate.begin("delete the config file");
        assert_eq!(pending.nonce, "delta");
        assert!(pending.prompt.contains("I heard: 'delete the config file'"));
        assert!(pending.prompt.contains("confirm delta"));
        assert_eq!(pending.expected_phrase().as_deref(), Some("confirm delta"));
    }

    #[test]
    fn confirm_accepts_phrase_with_surrounding_words() {
        let mut gate = gate_with_nonce("delta");
        gate.begin("wipe disk");
        assert_eq!(
            gate.try_confirm("yes, confirm delta please"),
            ConfirmResult::Confirmed
        );
        assert!(gate.pending().is_none());
    }

    #[test]
    fn confirm_rejects_wrong_nonce() {
        let mut gate = gate_with_nonce("delta");
        gate.begin("wipe disk");
        assert_eq!(
            gate.try_confirm("confirm alpha"),
            ConfirmResult::Rejected
        );
        // Still pending after rejection.
        assert!(gate.pending().is_some());
    }

    #[test]
    fn replay_of_prior_nonce_fails_after_rotation() {
        let mut gate = ConfirmationGate::with_nonce_source(
            VoiceConfirmationConfig::default(),
            Box::new(FixedNonceSource::new(["delta", "bravo"])),
        );
        gate.begin("first");
        assert_eq!(gate.try_confirm("confirm delta"), ConfirmResult::Confirmed);

        // Second action gets a new nonce; replaying "confirm delta" fails.
        gate.begin("second");
        assert_eq!(gate.pending().unwrap().nonce, "bravo");
        assert_eq!(
            gate.try_confirm("confirm delta"),
            ConfirmResult::Rejected
        );
        assert_eq!(
            gate.try_confirm("confirm bravo"),
            ConfirmResult::Confirmed
        );
    }

    #[test]
    fn timeout_cancels_pending() {
        let mut gate = gate_with_nonce("delta");
        let pending = gate.begin("risky");
        let after = pending.deadline + Duration::from_millis(1);
        assert_eq!(
            gate.try_confirm_at("confirm delta", after),
            ConfirmResult::TimedOut
        );
        assert!(gate.pending().is_none());
    }

    #[test]
    fn voice_command_confirm_plumbing() {
        let cmd = VoiceCommand {
            triggers: vec!["deploy now".into()],
            tool: "deploy".into(),
            params: serde_json::json!({"env": "prod"}),
            confirm: true,
            description: "Deploy to production.".into(),
        };
        assert!(command_requires_confirm(&cmd));

        let mut gate = gate_with_nonce("tango");
        let pending = gate.begin_for_command(&cmd, "deploy now production");
        assert_eq!(pending.tool.as_deref(), Some("deploy"));
        assert!(pending.prompt.contains("deploy now production"));
        assert!(pending.prompt.contains("confirm tango"));
        assert_eq!(
            gate.try_confirm("confirm tango"),
            ConfirmResult::Confirmed
        );
    }

    #[test]
    fn nonce_disabled_accepts_yes() {
        let cfg = VoiceConfirmationConfig {
            anti_replay_nonce: false,
            ..VoiceConfirmationConfig::default()
        };
        let mut gate = ConfirmationGate::new(cfg);
        let pending = gate.begin("delete all");
        assert!(pending.nonce.is_empty());
        assert!(pending.prompt.contains("Say 'yes' to confirm"));
        assert_eq!(gate.try_confirm("yes"), ConfirmResult::Confirmed);
    }

    #[test]
    fn echo_disabled_uses_about_to_prefix() {
        let cfg = VoiceConfirmationConfig {
            transcription_echo: false,
            ..VoiceConfirmationConfig::default()
        };
        let mut gate = ConfirmationGate::with_nonce_source(
            cfg,
            Box::new(FixedNonceSource::new(["kilo"])),
        );
        let pending = gate.begin("restart service");
        assert!(pending.prompt.starts_with("About to: restart service."));
        assert!(pending.prompt.contains("confirm kilo"));
    }

    #[test]
    fn no_pending_returns_no_pending() {
        let mut gate = gate_with_nonce("delta");
        assert_eq!(
            gate.try_confirm("confirm delta"),
            ConfirmResult::NoPending
        );
    }

    #[test]
    fn cancel_clears_pending() {
        let mut gate = gate_with_nonce("delta");
        gate.begin("x");
        gate.cancel();
        assert!(gate.pending().is_none());
    }

    #[test]
    fn matches_confirm_nonce_case_insensitive() {
        assert!(matches_confirm_nonce("Confirm DELTA", "delta"));
        assert!(!matches_confirm_nonce("confirm", "delta"));
        assert!(!matches_confirm_nonce("delta", "delta"));
    }
}
