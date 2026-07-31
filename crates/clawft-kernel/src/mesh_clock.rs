//! Injectable clock for mesh time-dependent components (WEFT-113).
//!
//! Mesh heartbeat, dedup TTL, service-resolution cache, and related
//! logic must not hardwire `std::time::Instant::now()` / `SystemTime`
//! if deterministic tests are required. Production code binds
//! [`RealClock`]; tests bind [`MockClock`].

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Monotonic timestamp relative to an opaque origin (nanoseconds).
///
/// Comparable and subtractable without depending on wall-clock time.
/// Values from different clock instances are not comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MonoTime {
    nanos: u64,
}

impl MonoTime {
    /// Construct from nanoseconds since the clock origin.
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Nanoseconds since the clock origin.
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// Duration elapsed from `earlier` to `self` (saturating).
    pub fn duration_since(self, earlier: Self) -> Duration {
        Duration::from_nanos(self.nanos.saturating_sub(earlier.nanos))
    }

    /// Alias used by heartbeat/dedup call sites for readability.
    pub fn elapsed_since(self, earlier: Self) -> Duration {
        self.duration_since(earlier)
    }
}

/// Injectable time source for mesh components.
///
/// # Production
///
/// Bind [`RealClock`] (or `Arc::new(RealClock)`).
///
/// # Tests
///
/// Bind [`MockClock`] and call [`MockClock::advance`] to drive timeouts
/// without sleeping.
pub trait Clock: Send + Sync + 'static {
    /// Current monotonic time (for timeouts / TTL).
    fn now(&self) -> MonoTime;

    /// Wall-clock microseconds since Unix epoch (for mesh time sync).
    fn unix_time_us(&self) -> u64;
}

/// Production clock backed by `std::time`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealClock;

impl RealClock {
    /// Create a production clock.
    pub const fn new() -> Self {
        Self
    }

    /// Shared production clock handle.
    pub fn shared() -> Arc<dyn Clock> {
        Arc::new(Self)
    }
}

impl Clock for RealClock {
    fn now(&self) -> MonoTime {
        // Instant is not epoch-based; measure from a process-global origin.
        static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let origin = ORIGIN.get_or_init(Instant::now);
        MonoTime::from_nanos(origin.elapsed().as_nanos() as u64)
    }

    fn unix_time_us(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
}

/// Controllable clock for deterministic time-dependent tests (WEFT-112).
///
/// Starts at monotonic `0` and unix epoch `0` unless configured.
#[derive(Debug, Clone)]
pub struct MockClock {
    mono_nanos: Arc<AtomicU64>,
    unix_us: Arc<AtomicU64>,
}

impl MockClock {
    /// Create a mock clock at t=0 (mono and unix).
    pub fn new() -> Self {
        Self {
            mono_nanos: Arc::new(AtomicU64::new(0)),
            unix_us: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a mock clock with an initial unix time (microseconds).
    pub fn with_unix_us(unix_us: u64) -> Self {
        Self {
            mono_nanos: Arc::new(AtomicU64::new(0)),
            unix_us: Arc::new(AtomicU64::new(unix_us)),
        }
    }

    /// Advance both mono and unix time by `duration`.
    pub fn advance(&self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        let us = duration.as_micros() as u64;
        self.mono_nanos.fetch_add(nanos, Ordering::SeqCst);
        self.unix_us.fetch_add(us, Ordering::SeqCst);
    }

    /// Set absolute monotonic time (nanoseconds since origin).
    pub fn set_mono_nanos(&self, nanos: u64) {
        self.mono_nanos.store(nanos, Ordering::SeqCst);
    }

    /// Set absolute unix time (microseconds).
    pub fn set_unix_us(&self, us: u64) {
        self.unix_us.store(us, Ordering::SeqCst);
    }

    /// Shared handle as `Arc<dyn Clock>`.
    pub fn shared(self) -> Arc<dyn Clock> {
        Arc::new(self)
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> MonoTime {
        MonoTime::from_nanos(self.mono_nanos.load(Ordering::SeqCst))
    }

    fn unix_time_us(&self) -> u64 {
        self.unix_us.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_time_duration_since_saturating() {
        let a = MonoTime::from_nanos(1_000);
        let b = MonoTime::from_nanos(3_000);
        assert_eq!(b.duration_since(a), Duration::from_nanos(2_000));
        assert_eq!(a.duration_since(b), Duration::from_nanos(0));
    }

    #[test]
    fn mock_clock_advance() {
        let clock = MockClock::new();
        assert_eq!(clock.now().as_nanos(), 0);
        assert_eq!(clock.unix_time_us(), 0);
        clock.advance(Duration::from_millis(500));
        assert_eq!(clock.now().as_nanos(), 500_000_000);
        assert_eq!(clock.unix_time_us(), 500_000);
    }

    #[test]
    fn real_clock_unix_plausible() {
        let t = RealClock.unix_time_us();
        // After 2020-01-01, before 2050-01-01.
        assert!(t > 1_577_836_800_000_000);
        assert!(t < 2_524_608_000_000_000);
    }

    #[test]
    fn real_clock_mono_advances() {
        let c = RealClock;
        let a = c.now();
        let b = c.now();
        assert!(b >= a);
    }
}
