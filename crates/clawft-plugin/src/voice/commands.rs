//! Voice command shortcuts.
//!
//! Maps spoken trigger phrases to direct tool invocations, bypassing
//! the full LLM pipeline for common commands. The
//! [`VoiceCommandRegistry`] performs exact prefix matching and
//! Levenshtein-based fuzzy matching.
//!
//! # Security plumbing (WEFT-225 / WEFT-226)
//!
//! [`VoiceCommandSession`] is the call-path that **reads**
//! [`VoiceCommand::confirm`]: rate-limit first (SC-8), then either
//! execute immediately or open an SC-6 anti-replay confirmation with
//! transcription echo. See [`crate::voice::confirm`] and
//! [`crate::voice::rate_limit`].

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::confirm::{
    command_requires_confirm, ConfirmResult, ConfirmationGate, PendingConfirmation,
};
use super::rate_limit::{VoiceRateDecision, VoiceRateLimiter};

/// A voice command shortcut that maps a spoken phrase to a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    /// Trigger phrases (any of these activates the command).
    /// Matched after STT, case-insensitive, with fuzzy tolerance.
    pub triggers: Vec<String>,
    /// Tool name to invoke.
    pub tool: String,
    /// Static parameters to pass to the tool.
    #[serde(default)]
    pub params: serde_json::Value,
    /// Whether this command requires voice confirmation before executing.
    #[serde(default)]
    pub confirm: bool,
    /// Human-readable description (for help listing).
    pub description: String,
}

/// Registry of voice command shortcuts.
///
/// Supports exact prefix matching and Levenshtein fuzzy matching
/// (edit distance <= 2) on the trigger phrase portion of transcriptions.
pub struct VoiceCommandRegistry {
    commands: Vec<VoiceCommand>,
    /// Precomputed lowercase triggers for fast matching.
    trigger_index: HashMap<String, usize>,
}

impl VoiceCommandRegistry {
    /// Build a registry from a list of voice commands.
    pub fn new(commands: Vec<VoiceCommand>) -> Self {
        let mut trigger_index = HashMap::new();
        for (idx, cmd) in commands.iter().enumerate() {
            for trigger in &cmd.triggers {
                trigger_index.insert(trigger.to_lowercase(), idx);
            }
        }
        Self {
            commands,
            trigger_index,
        }
    }

    /// Match a transcribed phrase against registered commands.
    ///
    /// Returns the matched command if the transcription starts with
    /// or closely matches a registered trigger phrase (Levenshtein
    /// distance <= 2).
    pub fn match_command(&self, transcription: &str) -> Option<&VoiceCommand> {
        let lower = transcription.to_lowercase();
        let lower = lower.trim();

        // Pass 1: exact prefix match
        for (trigger, idx) in &self.trigger_index {
            if lower.starts_with(trigger.as_str()) {
                return Some(&self.commands[*idx]);
            }
        }

        // Pass 2: fuzzy match on the first N words (Levenshtein <= 2)
        let words: Vec<&str> = lower.split_whitespace().collect();
        for (trigger, idx) in &self.trigger_index {
            let trigger_words: Vec<&str> = trigger.split_whitespace().collect();
            if words.len() >= trigger_words.len() {
                let spoken = words[..trigger_words.len()].join(" ");
                if levenshtein_distance(&spoken, trigger) <= 2 {
                    return Some(&self.commands[*idx]);
                }
            }
        }

        None
    }

    /// List all registered commands.
    pub fn list(&self) -> &[VoiceCommand] {
        &self.commands
    }

    /// Build a registry with the default built-in commands.
    pub fn with_defaults() -> Self {
        let commands = vec![
            VoiceCommand {
                triggers: vec!["stop listening".into(), "stop voice".into()],
                tool: "voice_stop".into(),
                params: serde_json::json!({}),
                confirm: false,
                description: "Stop the voice listening session.".into(),
            },
            VoiceCommand {
                triggers: vec!["what time is it".into(), "current time".into()],
                tool: "system_info".into(),
                params: serde_json::json!({"query": "time"}),
                confirm: false,
                description: "Show the current time.".into(),
            },
            VoiceCommand {
                triggers: vec!["list files".into(), "show files".into()],
                tool: "list_directory".into(),
                params: serde_json::json!({"path": "."}),
                confirm: false,
                description: "List files in the current directory.".into(),
            },
        ];
        Self::new(commands)
    }
}

/// Outcome of feeding a transcription into a [`VoiceCommandSession`].
#[derive(Debug, Clone)]
pub enum CommandDispatch {
    /// No registry match and no pending confirmation consumed the turn.
    NoMatch,
    /// Command is ready to execute (rate-limit passed; confirm not required
    /// or already spoken).
    Execute {
        /// The matched (or previously pending) command.
        command: VoiceCommand,
        /// Original transcription that authorized execution.
        heard: String,
    },
    /// Command matched with `confirm: true`; speak `prompt` via TTS and wait.
    NeedConfirmation {
        /// Command awaiting confirmation.
        command: VoiceCommand,
        /// Full TTS prompt (echo + nonce).
        prompt: String,
        /// Pending confirmation metadata (nonce, deadline).
        pending: PendingConfirmation,
    },
    /// User replied but the phrase did not match the expected nonce.
    ConfirmationRejected {
        /// Hint for TTS ("Say 'confirm delta' to proceed.").
        hint: String,
    },
    /// Confirmation window expired.
    ConfirmationTimedOut,
    /// SC-8 rate limit or post-fail cooldown blocked the turn.
    RateLimited {
        /// Decision detail (bucket vs cooldown).
        decision: VoiceRateDecision,
        /// User-facing message suitable for TTS.
        message: String,
    },
}

/// Stateful voice-command path: registry + SC-8 rate limiter + SC-6 gate.
///
/// This is the surface that **consumes** [`VoiceCommand::confirm`]. Call
/// [`Self::handle_transcription`] for every finalized STT result on the
/// command channel.
pub struct VoiceCommandSession {
    registry: VoiceCommandRegistry,
    rate: VoiceRateLimiter,
    confirm: ConfirmationGate,
    /// Command waiting on spoken confirmation (cloned at `NeedConfirmation`).
    awaiting: Option<VoiceCommand>,
}

impl VoiceCommandSession {
    /// Assemble a session from explicit pieces.
    pub fn new(
        registry: VoiceCommandRegistry,
        rate: VoiceRateLimiter,
        confirm: ConfirmationGate,
    ) -> Self {
        Self {
            registry,
            rate,
            confirm,
            awaiting: None,
        }
    }

    /// Defaults: built-in registry, SC-8 defaults, SC-6 defaults.
    pub fn with_defaults() -> Self {
        Self::new(
            VoiceCommandRegistry::with_defaults(),
            VoiceRateLimiter::with_defaults(),
            ConfirmationGate::with_defaults(),
        )
    }

    /// Borrow the command registry.
    pub fn registry(&self) -> &VoiceCommandRegistry {
        &self.registry
    }

    /// Mutable rate limiter (wake path shares the same SC-8 instance).
    pub fn rate_limiter_mut(&mut self) -> &mut VoiceRateLimiter {
        &mut self.rate
    }

    /// Mutable confirmation gate.
    pub fn confirmation_gate_mut(&mut self) -> &mut ConfirmationGate {
        &mut self.confirm
    }

    /// Check a wake activation against the SC-8 wake bucket.
    pub fn check_wake(&mut self) -> VoiceRateDecision {
        self.rate.check_wake()
    }

    /// Process a finalized transcription (command match or confirmation reply).
    pub fn handle_transcription(&mut self, transcription: &str) -> CommandDispatch {
        // 1. Pending confirmation takes priority over new command matching.
        if self.awaiting.is_some() {
            return self.handle_confirm_reply(transcription);
        }

        // 2. Match a command.
        let Some(cmd) = self.registry.match_command(transcription).cloned() else {
            return CommandDispatch::NoMatch;
        };

        // 3. SC-8 command rate limit.
        let decision = self.rate.check_command();
        if !decision.is_allowed() {
            let message = VoiceRateLimiter::denial_message(&decision)
                .unwrap_or("Rate limit reached. Please wait.")
                .to_string();
            return CommandDispatch::RateLimited { decision, message };
        }

        // 4. SC-6: commands with confirm=true open the nonce gate.
        if command_requires_confirm(&cmd) {
            let pending = self.confirm.begin_for_command(&cmd, transcription);
            let prompt = pending.prompt.clone();
            self.awaiting = Some(cmd.clone());
            return CommandDispatch::NeedConfirmation {
                command: cmd,
                prompt,
                pending,
            };
        }

        // 5. Execute immediately.
        self.rate.record_success();
        CommandDispatch::Execute {
            command: cmd,
            heard: transcription.to_string(),
        }
    }

    fn handle_confirm_reply(&mut self, transcription: &str) -> CommandDispatch {
        match self.confirm.try_confirm(transcription) {
            ConfirmResult::Confirmed => {
                let command = self
                    .awaiting
                    .take()
                    .expect("awaiting set when confirm pending");
                self.rate.record_success();
                CommandDispatch::Execute {
                    command,
                    heard: transcription.to_string(),
                }
            }
            ConfirmResult::Rejected => {
                let hint = self
                    .confirm
                    .pending()
                    .map(|p| {
                        p.expected_phrase()
                            .map(|ph| format!("Say '{ph}' to proceed."))
                            .unwrap_or_else(|| "Say 'yes' to confirm.".into())
                    })
                    .unwrap_or_else(|| "Confirmation still required.".into());
                CommandDispatch::ConfirmationRejected { hint }
            }
            ConfirmResult::TimedOut => {
                self.awaiting = None;
                CommandDispatch::ConfirmationTimedOut
            }
            ConfirmResult::NoPending => {
                // Gate cleared externally; drop awaiting and re-process as new.
                self.awaiting = None;
                self.handle_transcription(transcription)
            }
        }
    }

    /// Report an execution / STT failure for SC-8 post-fail cooldown.
    pub fn record_failure(&mut self) {
        self.rate.record_failure();
    }

    /// Cancel any open confirmation without executing.
    pub fn cancel_confirmation(&mut self) {
        self.awaiting = None;
        self.confirm.cancel();
    }

    /// Whether a confirmation is outstanding.
    pub fn is_awaiting_confirmation(&self) -> bool {
        self.awaiting.is_some()
    }
}

/// Helper: map a rate decision to a retry duration when limited.
pub fn rate_retry_after(decision: &VoiceRateDecision) -> Option<Duration> {
    match decision {
        VoiceRateDecision::RateLimited { retry_after, .. } => Some(*retry_after),
        VoiceRateDecision::Cooldown { remaining } => Some(*remaining),
        VoiceRateDecision::Allow => None,
    }
}

/// Simple Levenshtein distance (edit distance) between two strings.
///
/// Standard dynamic programming implementation. Used for fuzzy matching
/// of voice commands where minor transcription errors are expected.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> VoiceCommandRegistry {
        VoiceCommandRegistry::with_defaults()
    }

    #[test]
    fn exact_match_stop_listening() {
        let registry = test_registry();
        let cmd = registry.match_command("stop listening").unwrap();
        assert_eq!(cmd.tool, "voice_stop");
    }

    #[test]
    fn exact_match_what_time() {
        let registry = test_registry();
        let cmd = registry.match_command("what time is it").unwrap();
        assert_eq!(cmd.tool, "system_info");
    }

    #[test]
    fn exact_match_list_files_with_suffix() {
        let registry = test_registry();
        let cmd = registry
            .match_command("list files in the current directory")
            .unwrap();
        assert_eq!(cmd.tool, "list_directory");
    }

    #[test]
    fn exact_match_case_insensitive() {
        let registry = test_registry();
        let cmd = registry.match_command("Stop Listening").unwrap();
        assert_eq!(cmd.tool, "voice_stop");
    }

    #[test]
    fn fuzzy_match_within_distance_2() {
        let registry = test_registry();
        // "stopp listening" has distance 1 from "stop listening"
        let cmd = registry.match_command("stopp listening");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().tool, "voice_stop");
    }

    #[test]
    fn no_match_for_unrelated_phrase() {
        let registry = test_registry();
        let cmd = registry.match_command("tell me a joke");
        assert!(cmd.is_none());
    }

    #[test]
    fn levenshtein_identical_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn levenshtein_one_insertion() {
        assert_eq!(levenshtein_distance("hell", "hello"), 1);
    }

    #[test]
    fn levenshtein_one_deletion() {
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
    }

    #[test]
    fn levenshtein_one_substitution() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    }

    #[test]
    fn levenshtein_completely_different() {
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
    }

    #[test]
    fn levenshtein_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn voice_command_serde_roundtrip() {
        let cmd = VoiceCommand {
            triggers: vec!["test".into()],
            tool: "test_tool".into(),
            params: serde_json::json!({"key": "value"}),
            confirm: true,
            description: "A test command".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let restored: VoiceCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.triggers, vec!["test"]);
        assert_eq!(restored.tool, "test_tool");
        assert!(restored.confirm);
    }

    #[test]
    fn registry_list_returns_all_commands() {
        let registry = test_registry();
        assert_eq!(registry.list().len(), 3);
    }

    #[test]
    fn custom_commands_registry() {
        let commands = vec![VoiceCommand {
            triggers: vec!["deploy now".into(), "ship it".into()],
            tool: "deploy".into(),
            params: serde_json::json!({"env": "production"}),
            confirm: true,
            description: "Deploy to production.".into(),
        }];
        let registry = VoiceCommandRegistry::new(commands);
        let cmd = registry.match_command("ship it").unwrap();
        assert_eq!(cmd.tool, "deploy");
        assert!(cmd.confirm);
    }

    // ── WEFT-225 / WEFT-226 session plumbing ────────────────────────────

    fn session_with_confirm_cmd() -> VoiceCommandSession {
        use super::super::confirm::FixedNonceSource;
        use clawft_types::config::{VoiceConfirmationConfig, VoiceRateLimitConfig};

        let registry = VoiceCommandRegistry::new(vec![
            VoiceCommand {
                triggers: vec!["list files".into()],
                tool: "list_directory".into(),
                params: serde_json::json!({}),
                confirm: false,
                description: "List files.".into(),
            },
            VoiceCommand {
                triggers: vec!["deploy now".into()],
                tool: "deploy".into(),
                params: serde_json::json!({}),
                confirm: true,
                description: "Deploy.".into(),
            },
        ]);
        let rate = VoiceRateLimiter::new(VoiceRateLimitConfig::default());
        let confirm = ConfirmationGate::with_nonce_source(
            VoiceConfirmationConfig::default(),
            Box::new(FixedNonceSource::new(["delta"])),
        );
        VoiceCommandSession::new(registry, rate, confirm)
    }

    #[test]
    fn session_executes_non_confirm_command() {
        let mut session = session_with_confirm_cmd();
        match session.handle_transcription("list files") {
            CommandDispatch::Execute { command, .. } => {
                assert_eq!(command.tool, "list_directory");
                assert!(!command.confirm);
            }
            other => panic!("expected Execute, got {other:?}"),
        }
    }

    #[test]
    fn session_confirm_flow_with_nonce() {
        let mut session = session_with_confirm_cmd();
        match session.handle_transcription("deploy now") {
            CommandDispatch::NeedConfirmation {
                command, prompt, pending, ..
            } => {
                assert_eq!(command.tool, "deploy");
                assert!(command.confirm);
                assert_eq!(pending.nonce, "delta");
                assert!(prompt.contains("confirm delta"));
                assert!(prompt.contains("deploy now"));
            }
            other => panic!("expected NeedConfirmation, got {other:?}"),
        }
        assert!(session.is_awaiting_confirmation());

        // Wrong nonce keeps waiting.
        match session.handle_transcription("confirm alpha") {
            CommandDispatch::ConfirmationRejected { .. } => {}
            other => panic!("expected ConfirmationRejected, got {other:?}"),
        }

        // Correct nonce executes.
        match session.handle_transcription("confirm delta") {
            CommandDispatch::Execute { command, .. } => {
                assert_eq!(command.tool, "deploy");
            }
            other => panic!("expected Execute after confirm, got {other:?}"),
        }
        assert!(!session.is_awaiting_confirmation());
    }

    #[test]
    fn session_rate_limits_commands() {
        use clawft_types::config::{VoiceConfirmationConfig, VoiceRateLimitConfig};
        use super::super::confirm::ConfirmationGate;

        let registry = VoiceCommandRegistry::with_defaults();
        let rate = VoiceRateLimiter::new(VoiceRateLimitConfig {
            commands_per_minute: 2,
            ..VoiceRateLimitConfig::default()
        });
        let confirm = ConfirmationGate::new(VoiceConfirmationConfig::default());
        let mut session = VoiceCommandSession::new(registry, rate, confirm);

        assert!(matches!(
            session.handle_transcription("what time is it"),
            CommandDispatch::Execute { .. }
        ));
        assert!(matches!(
            session.handle_transcription("what time is it"),
            CommandDispatch::Execute { .. }
        ));
        match session.handle_transcription("what time is it") {
            CommandDispatch::RateLimited { message, .. } => {
                assert!(message.contains("Rate limit"));
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }
}
