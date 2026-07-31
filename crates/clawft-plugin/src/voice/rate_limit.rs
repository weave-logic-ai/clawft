//! Voice rate limiting (SC-8 / WEFT-226).
//!
//! Spec (`.planning/sparc/voice/06-voice-security-review.md` SC-8):
//!
//! | Control | Default |
//! |---------|---------|
//! | Max voice commands / min | 10 (token bucket) |
//! | Max wake activations / min | 5 (token bucket) |
//! | Post-fail cooldown | after 3 consecutive failures → 30 s |
//!
//! # Token bucket
//!
//! Each bucket has capacity = rate (burst equals one full minute of
//! allowance) and refills continuously at `rate / 60` tokens per second.
//! A successful check consumes one token; a denied check leaves the
//! balance unchanged.
//!
//! # Post-fail cooldown
//!
//! Callers report STT / command failures via [`VoiceRateLimiter::record_failure`].
//! After [`VoiceRateLimitConfig::fail_threshold`] consecutive failures,
//! both command and wake checks are refused until
//! [`VoiceRateLimitConfig::post_fail_cooldown_seconds`] elapses. A success
//! ([`record_success`]) resets the consecutive-failure counter.

use std::time::{Duration, Instant};

use clawft_types::config::VoiceRateLimitConfig;
use tracing::{debug, info, warn};

/// Tracing target for SC-8 rate-limit events.
pub const RATE_LIMIT_TARGET: &str = "voice.rate_limit";

/// Which voice surface is being rate-limited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceRateKind {
    /// Matched voice command / tool dispatch.
    Command,
    /// Wake-word activation (Talk Mode open).
    Wake,
}

impl VoiceRateKind {
    fn as_str(self) -> &'static str {
        match self {
            VoiceRateKind::Command => "command",
            VoiceRateKind::Wake => "wake",
        }
    }
}

/// Result of a rate-limit check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceRateDecision {
    /// Request is allowed; one token was consumed (unless dry-run).
    Allow,
    /// Token bucket empty for this kind.
    RateLimited {
        /// Suggested wait before the next try.
        retry_after: Duration,
        kind: VoiceRateKind,
    },
    /// Post-fail cooldown is active.
    Cooldown {
        /// Remaining cooldown duration.
        remaining: Duration,
    },
}

impl VoiceRateDecision {
    /// True when the request may proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, VoiceRateDecision::Allow)
    }
}

/// Continuous-refill token bucket.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Maximum tokens (and initial fill).
    capacity: f64,
    /// Tokens added per second.
    refill_per_sec: f64,
    /// Current balance.
    tokens: f64,
    /// Last refill timestamp.
    last: Instant,
}

impl TokenBucket {
    fn new(rate_per_minute: u32, now: Instant) -> Self {
        let capacity = f64::from(rate_per_minute.max(1));
        Self {
            capacity,
            refill_per_sec: capacity / 60.0,
            tokens: capacity,
            last: now,
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
            self.last = now;
        }
    }

    /// Try to take one token. Returns remaining wait if empty.
    fn try_take(&mut self, now: Instant) -> Result<(), Duration> {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            let need = 1.0 - self.tokens;
            let secs = if self.refill_per_sec > 0.0 {
                need / self.refill_per_sec
            } else {
                60.0
            };
            Err(Duration::from_secs_f64(secs.max(0.001)))
        }
    }
}

/// SC-8 voice rate limiter: command bucket + wake bucket + post-fail cooldown.
#[derive(Debug)]
pub struct VoiceRateLimiter {
    config: VoiceRateLimitConfig,
    commands: TokenBucket,
    wake: TokenBucket,
    consecutive_failures: u32,
    cooldown_until: Option<Instant>,
}

impl VoiceRateLimiter {
    /// Build from config using `Instant::now()` as the initial clock.
    pub fn new(config: VoiceRateLimitConfig) -> Self {
        Self::new_at(config, Instant::now())
    }

    /// Build with an explicit initial clock (tests).
    pub fn new_at(config: VoiceRateLimitConfig, now: Instant) -> Self {
        Self {
            commands: TokenBucket::new(config.commands_per_minute, now),
            wake: TokenBucket::new(config.wake_activations_per_minute, now),
            consecutive_failures: 0,
            cooldown_until: None,
            config,
        }
    }

    /// Spec defaults: 10 commands/min, 5 wake/min, 3 fails → 30 s cooldown.
    pub fn with_defaults() -> Self {
        Self::new(VoiceRateLimitConfig::default())
    }

    /// Configuration snapshot.
    pub fn config(&self) -> &VoiceRateLimitConfig {
        &self.config
    }

    /// Consecutive failure count (since last success or cooldown expiry).
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// True when a post-fail cooldown is currently active.
    pub fn is_in_cooldown(&self, now: Instant) -> bool {
        self.cooldown_until
            .map(|until| now < until)
            .unwrap_or(false)
    }

    /// Remaining cooldown, if any.
    pub fn cooldown_remaining(&self, now: Instant) -> Option<Duration> {
        self.cooldown_until.and_then(|until| {
            if now < until {
                Some(until.saturating_duration_since(now))
            } else {
                None
            }
        })
    }

    /// Check (and consume) a command slot.
    pub fn check_command(&mut self) -> VoiceRateDecision {
        self.check_at(VoiceRateKind::Command, Instant::now())
    }

    /// Check (and consume) a wake-activation slot.
    pub fn check_wake(&mut self) -> VoiceRateDecision {
        self.check_at(VoiceRateKind::Wake, Instant::now())
    }

    /// Check at an explicit clock (tests).
    pub fn check_at(&mut self, kind: VoiceRateKind, now: Instant) -> VoiceRateDecision {
        // Clear expired cooldown.
        if let Some(until) = self.cooldown_until {
            if now >= until {
                self.cooldown_until = None;
                self.consecutive_failures = 0;
                debug!(
                    target: RATE_LIMIT_TARGET,
                    event = "voice.cooldown_cleared",
                    "voice post-fail cooldown expired"
                );
            }
        }

        if let Some(remaining) = self.cooldown_remaining(now) {
            warn!(
                target: RATE_LIMIT_TARGET,
                event = "voice.cooldown_block",
                kind = kind.as_str(),
                remaining_ms = remaining.as_millis() as u64,
                "voice request blocked by post-fail cooldown"
            );
            return VoiceRateDecision::Cooldown { remaining };
        }

        let bucket = match kind {
            VoiceRateKind::Command => &mut self.commands,
            VoiceRateKind::Wake => &mut self.wake,
        };

        match bucket.try_take(now) {
            Ok(()) => {
                debug!(
                    target: RATE_LIMIT_TARGET,
                    event = "voice.rate_allow",
                    kind = kind.as_str(),
                    "voice rate limit allow"
                );
                VoiceRateDecision::Allow
            }
            Err(retry_after) => {
                warn!(
                    target: RATE_LIMIT_TARGET,
                    event = "voice.rate_limited",
                    kind = kind.as_str(),
                    retry_after_ms = retry_after.as_millis() as u64,
                    "Rate limit reached. Please wait."
                );
                VoiceRateDecision::RateLimited {
                    retry_after,
                    kind,
                }
            }
        }
    }

    /// Record a failed voice command / STT attempt.
    ///
    /// After [`VoiceRateLimitConfig::fail_threshold`] consecutive failures,
    /// enters the post-fail cooldown.
    pub fn record_failure(&mut self) {
        self.record_failure_at(Instant::now());
    }

    /// Record a failure at an explicit clock (tests).
    pub fn record_failure_at(&mut self, now: Instant) {
        // Already cooling down — do not re-arm until it clears.
        if self.is_in_cooldown(now) {
            return;
        }
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let threshold = self.config.fail_threshold.max(1);
        if self.consecutive_failures >= threshold {
            let cooldown = Duration::from_secs(u64::from(
                self.config.post_fail_cooldown_seconds.max(1),
            ));
            self.cooldown_until = Some(now + cooldown);
            info!(
                target: RATE_LIMIT_TARGET,
                event = "voice.post_fail_cooldown",
                consecutive_failures = self.consecutive_failures,
                cooldown_s = self.config.post_fail_cooldown_seconds,
                "entering voice post-fail cooldown"
            );
        }
    }

    /// Record a successful command (resets the consecutive-failure counter).
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// User-facing message for a denied decision (TTS / UI).
    pub fn denial_message(decision: &VoiceRateDecision) -> Option<&'static str> {
        match decision {
            VoiceRateDecision::Allow => None,
            VoiceRateDecision::RateLimited { .. } => {
                Some("Rate limit reached. Please wait.")
            }
            VoiceRateDecision::Cooldown { .. } => {
                Some("Voice temporarily paused after repeated failures. Please wait.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_tight() -> VoiceRateLimitConfig {
        VoiceRateLimitConfig {
            commands_per_minute: 10,
            wake_activations_per_minute: 5,
            fail_threshold: 3,
            post_fail_cooldown_seconds: 30,
        }
    }

    #[test]
    fn command_bucket_allows_up_to_capacity() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(cfg_tight(), t0);
        for i in 0..10 {
            let d = lim.check_at(VoiceRateKind::Command, t0);
            assert!(d.is_allowed(), "command {i} should be allowed");
        }
        let denied = lim.check_at(VoiceRateKind::Command, t0);
        assert!(
            matches!(denied, VoiceRateDecision::RateLimited { kind: VoiceRateKind::Command, .. }),
            "11th command must be rate-limited: {denied:?}"
        );
    }

    #[test]
    fn wake_bucket_allows_up_to_capacity() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(cfg_tight(), t0);
        for _ in 0..5 {
            assert!(lim.check_at(VoiceRateKind::Wake, t0).is_allowed());
        }
        let denied = lim.check_at(VoiceRateKind::Wake, t0);
        assert!(matches!(
            denied,
            VoiceRateDecision::RateLimited {
                kind: VoiceRateKind::Wake,
                ..
            }
        ));
    }

    #[test]
    fn buckets_are_independent() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(cfg_tight(), t0);
        for _ in 0..10 {
            assert!(lim.check_at(VoiceRateKind::Command, t0).is_allowed());
        }
        // Commands exhausted, wake still full.
        assert!(lim.check_at(VoiceRateKind::Wake, t0).is_allowed());
        assert!(matches!(
            lim.check_at(VoiceRateKind::Command, t0),
            VoiceRateDecision::RateLimited {
                kind: VoiceRateKind::Command,
                ..
            }
        ));
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(
            VoiceRateLimitConfig {
                commands_per_minute: 2,
                ..cfg_tight()
            },
            t0,
        );
        assert!(lim.check_at(VoiceRateKind::Command, t0).is_allowed());
        assert!(lim.check_at(VoiceRateKind::Command, t0).is_allowed());
        assert!(!lim.check_at(VoiceRateKind::Command, t0).is_allowed());

        // 30 s later → ~1 token refilled (2/min → 1/30s).
        let t1 = t0 + Duration::from_secs(30);
        assert!(lim.check_at(VoiceRateKind::Command, t1).is_allowed());
        assert!(!lim.check_at(VoiceRateKind::Command, t1).is_allowed());
    }

    #[test]
    fn post_fail_cooldown_after_threshold() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(cfg_tight(), t0);

        lim.record_failure_at(t0);
        lim.record_failure_at(t0);
        assert!(!lim.is_in_cooldown(t0));
        assert_eq!(lim.consecutive_failures(), 2);

        lim.record_failure_at(t0);
        assert!(lim.is_in_cooldown(t0));
        assert_eq!(lim.consecutive_failures(), 3);

        let blocked = lim.check_at(VoiceRateKind::Command, t0);
        assert!(matches!(blocked, VoiceRateDecision::Cooldown { .. }));
        let blocked_wake = lim.check_at(VoiceRateKind::Wake, t0);
        assert!(matches!(blocked_wake, VoiceRateDecision::Cooldown { .. }));

        // After 30 s cooldown, requests flow again.
        let t1 = t0 + Duration::from_secs(30);
        assert!(lim.check_at(VoiceRateKind::Command, t1).is_allowed());
        assert_eq!(lim.consecutive_failures(), 0);
    }

    #[test]
    fn success_resets_failure_counter() {
        let t0 = Instant::now();
        let mut lim = VoiceRateLimiter::new_at(cfg_tight(), t0);
        lim.record_failure_at(t0);
        lim.record_failure_at(t0);
        lim.record_success();
        assert_eq!(lim.consecutive_failures(), 0);
        lim.record_failure_at(t0);
        lim.record_failure_at(t0);
        // Only 2 since success — no cooldown yet.
        assert!(!lim.is_in_cooldown(t0));
    }

    #[test]
    fn denial_messages() {
        assert!(VoiceRateLimiter::denial_message(&VoiceRateDecision::Allow).is_none());
        assert_eq!(
            VoiceRateLimiter::denial_message(&VoiceRateDecision::RateLimited {
                retry_after: Duration::from_secs(1),
                kind: VoiceRateKind::Command,
            }),
            Some("Rate limit reached. Please wait.")
        );
        assert!(
            VoiceRateLimiter::denial_message(&VoiceRateDecision::Cooldown {
                remaining: Duration::from_secs(5),
            })
            .unwrap()
            .contains("paused")
        );
    }

    #[test]
    fn defaults_match_spec() {
        let lim = VoiceRateLimiter::with_defaults();
        assert_eq!(lim.config().commands_per_minute, 10);
        assert_eq!(lim.config().wake_activations_per_minute, 5);
        assert_eq!(lim.config().fail_threshold, 3);
        assert_eq!(lim.config().post_fail_cooldown_seconds, 30);
    }
}
