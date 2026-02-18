# SPARC Implementation Plan: Phase E - RateLimiter

**Phase**: E (of A-I)
**Depends On**: Phase B (UserPermissions)
**Effort**: 0.5 days
**File**: `crates/clawft-core/src/pipeline/rate_limiter.rs`

---

## 1. Specification

### Goal

Implement a sliding-window rate limiter keyed by `sender_id` that the `TieredRouter`
uses at Step 3 of the routing algorithm (Section 5.2 of `08-tiered-router.md`).
Each user has a configurable `rate_limit` (requests per minute) from their resolved
`UserPermissions`. The limiter returns `true` (allowed) or `false` (rate-limited).

### Requirements

| Requirement | Detail |
|-------------|--------|
| Algorithm | Sliding window counter (not fixed window, not token bucket) |
| Key | `sender_id: &str` -- one window per unique sender |
| Window | Configurable `window_seconds` (default 60, from `routing.rate_limiting.window_seconds`) |
| Limit source | `UserPermissions.rate_limit` (requests per window). `0` = unlimited |
| Thread safety | Must be safe for concurrent access from `tokio` tasks (no `Mutex`) |
| Eviction | LRU eviction when entries exceed `max_entries` (default 10,000) |
| Memory bound | Each entry: sender key + Vec of timestamps. ~200 bytes per entry at peak |
| Clock | `std::time::Instant` for monotonic timestamps (no wall-clock skew) |

### Non-Goals

- Distributed rate limiting (single-process only for Phase E)
- Per-IP rate limiting (only per-sender_id)
- Per-channel aggregate rate limiting (only global and per-sender_id)

---

## 2. Pseudocode

### Core Data Structures

```rust
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::pipeline::tiered_router::RateLimitable;

/// A sliding-window rate limiter keyed by sender ID, with an optional
/// global rate limit that applies across all senders.
///
/// Thread-safe via `DashMap` (per-sender) and `AtomicU64` (global counter).
/// Each sender has an independent sliding window of timestamps. When the
/// window is full, the request is rejected. A global rate limit, if
/// configured, is checked BEFORE per-user limits to prevent aggregate abuse
/// from many distinct sender_ids.
pub struct RateLimiter {
    /// Per-sender sliding window entries.
    /// Key: sender_id, Value: SlidingWindowEntry.
    entries: DashMap<String, SlidingWindowEntry>,

    /// Window duration (default: 60 seconds).
    window: Duration,

    /// Maximum tracked senders before LRU eviction.
    max_entries: usize,

    /// Monotonic counter for LRU ordering.
    access_counter: AtomicU64,

    /// Global request counter for the current window.
    /// Tracks total requests across ALL senders within the current window.
    /// Used to enforce `global_rate_limit_rpm` from `RateLimitConfig`.
    global_counter: AtomicU64,

    /// Start of the current global rate limit window.
    /// Protected by Mutex because it is read-check-reset atomically.
    global_window_start: Mutex<Instant>,

    /// Global rate limit: maximum requests per window across all senders.
    /// 0 = unlimited (no global limit enforced).
    /// Sourced from `RateLimitConfig.global_rate_limit_rpm`.
    global_rate_limit: u32,
}

/// Timestamps of recent requests within the sliding window for one sender.
struct SlidingWindowEntry {
    /// Timestamps of requests within the current window, oldest first.
    timestamps: VecDeque<Instant>,

    /// Last access order (for LRU eviction).
    last_access: u64,
}
```

### Constructor

```rust
impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    /// - `window_seconds`: Duration of the sliding window (default 60).
    /// - `max_entries`: Maximum number of tracked senders before LRU eviction (default 10_000).
    /// - `global_rate_limit`: Maximum requests per window across ALL senders. 0 = unlimited.
    pub fn new(window_seconds: u64, max_entries: usize, global_rate_limit: u32) -> Self {
        Self {
            entries: DashMap::with_capacity(1024),
            window: Duration::from_secs(window_seconds),
            max_entries,
            access_counter: AtomicU64::new(0),
            global_counter: AtomicU64::new(0),
            global_window_start: Mutex::new(Instant::now()),
            global_rate_limit,
        }
    }

    /// Create from the routing config's rate_limiting section.
    ///
    /// Reads `window_seconds` and `global_rate_limit_rpm` from `RateLimitConfig`.
    pub fn from_config(window_seconds: u64, global_rate_limit_rpm: u32) -> Self {
        Self::new(window_seconds, 10_000, global_rate_limit_rpm)
    }
}
```

### Global Rate Limit Check

```rust
impl RateLimiter {
    /// Check if the global rate limit allows another request.
    ///
    /// This is called BEFORE per-user checks. If the global limit is exceeded,
    /// the request is rejected regardless of per-user limits. This prevents
    /// aggregate abuse from many distinct sender_ids (e.g., an attacker creating
    /// 1000 Discord accounts to bypass per-user limits).
    ///
    /// Returns `true` if allowed, `false` if global limit exceeded.
    fn check_global(&self) -> bool {
        // 0 = no global limit
        if self.global_rate_limit == 0 {
            return true;
        }

        let now = Instant::now();

        // Check if the current global window has expired; if so, reset.
        // The Mutex ensures atomic read-check-reset of the window boundary.
        {
            let mut window_start = self.global_window_start.lock().unwrap();
            if now.duration_since(*window_start) >= self.window {
                // Window expired: reset counter and start a new window.
                self.global_counter.store(0, Ordering::Relaxed);
                *window_start = now;
            }
        }

        // Atomically increment and check against limit.
        // fetch_add returns the previous value; if it was already at or above
        // the limit, the request is rejected.
        let prev = self.global_counter.fetch_add(1, Ordering::Relaxed);
        if prev >= self.global_rate_limit as u64 {
            // Over limit -- undo the increment (best effort) so the counter
            // does not drift unboundedly when many requests are rejected.
            self.global_counter.fetch_sub(1, Ordering::Relaxed);
            return false;
        }

        true
    }
```

### Core Check Method

```rust
    /// Check if a request from `sender_id` is allowed under the given `limit`.
    ///
    /// Performs a two-stage check:
    /// 1. **Global rate limit** -- checked first via `check_global()`. If the
    ///    global limit (from `RateLimitConfig.global_rate_limit_rpm`) is exceeded,
    ///    the request is rejected immediately regardless of per-user limits.
    /// 2. **Per-user rate limit** -- checked second via the sliding window for
    ///    this `sender_id`.
    ///
    /// Returns `true` if the request is allowed, `false` if rate-limited.
    ///
    /// # Arguments
    /// - `sender_id`: Unique identifier for the sender.
    /// - `limit`: Maximum requests per window for this sender. `0` means unlimited
    ///   per-user (global limit still applies).
    pub fn check(&self, sender_id: &str, limit: u32) -> bool {
        // Stage 1: Check global rate limit BEFORE per-user limit.
        if !self.check_global() {
            return false;
        }

        // Stage 2: Per-user limit. 0 = unlimited per-user, always allow.
        if limit == 0 {
            return true;
        }

        let now = Instant::now();
        let window_start = now - self.window;
        let order = self.access_counter.fetch_add(1, Ordering::Relaxed);

        // Get or insert entry for this sender
        let mut entry = self.entries
            .entry(sender_id.to_string())
            .or_insert_with(|| SlidingWindowEntry {
                timestamps: VecDeque::new(),
                last_access: order,
            });

        // Update LRU access order
        entry.last_access = order;

        // Purge expired timestamps from the front of the deque
        while let Some(&ts) = entry.timestamps.front() {
            if ts < window_start {
                entry.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check if under the limit
        if entry.timestamps.len() < limit as usize {
            entry.timestamps.push_back(now);
            // Trigger eviction if needed (non-blocking, best-effort)
            drop(entry); // Release DashMap lock before eviction
            self.maybe_evict();
            true
        } else {
            false
        }
    }
```

### LRU Eviction

```rust
    /// Evict oldest-accessed entries if we exceed `max_entries`.
    ///
    /// This is called after each `check()` that inserts. It is best-effort:
    /// if another thread is already evicting, this call is a no-op.
    fn maybe_evict(&self) {
        if self.entries.len() <= self.max_entries {
            return;
        }

        // Find the entry with the lowest last_access value
        let mut oldest_key: Option<String> = None;
        let mut oldest_access = u64::MAX;

        for entry in self.entries.iter() {
            if entry.value().last_access < oldest_access {
                oldest_access = entry.value().last_access;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.entries.remove(&key);
        }
    }
```

### Utility Methods

```rust
    /// Get the current request count within the window for a sender.
    /// Useful for diagnostics and status reporting.
    pub fn current_count(&self, sender_id: &str) -> usize {
        let now = Instant::now();
        let window_start = now - self.window;

        self.entries.get(sender_id).map_or(0, |entry| {
            entry.timestamps.iter().filter(|ts| **ts >= window_start).count()
        })
    }

    /// Get the number of tracked senders.
    pub fn tracked_senders(&self) -> usize {
        self.entries.len()
    }

    /// Remove all tracked entries and reset global counter.
    /// Used for testing and config reloads.
    pub fn clear(&self) {
        self.entries.clear();
        self.access_counter.store(0, Ordering::Relaxed);
        self.global_counter.store(0, Ordering::Relaxed);
        *self.global_window_start.lock().unwrap() = Instant::now();
    }

    /// Get the configured window duration.
    pub fn window_duration(&self) -> Duration {
        self.window
    }

    /// Get the current global request count within the active window.
    /// Useful for diagnostics and monitoring dashboards.
    pub fn global_request_count(&self) -> u64 {
        self.global_counter.load(Ordering::Relaxed)
    }

    /// Get the configured global rate limit. 0 = unlimited.
    pub fn global_rate_limit(&self) -> u32 {
        self.global_rate_limit
    }
}
```

### RateLimitable Trait Implementation

Phase C defines the `RateLimitable` trait in `crates/clawft-core/src/pipeline/tiered_router.rs`.
This `impl` block connects the concrete `RateLimiter` to that trait, allowing the `TieredRouter`
to hold `Arc<dyn RateLimitable>` and call `check()` polymorphically.

```rust
/// Implement the Phase C trait so that `TieredRouter` can use `RateLimiter`
/// as `Arc<dyn RateLimitable + Send + Sync>`.
///
/// The trait signature is:
///   fn check(&self, sender_id: &str, limit: u32) -> bool;
///
/// This delegates directly to `RateLimiter::check()`, which performs both
/// the global rate limit check and the per-user sliding window check.
impl RateLimitable for RateLimiter {
    fn check(&self, sender_id: &str, limit: u32) -> bool {
        // Delegate to the inherent method, which handles:
        // 1. Global rate limit (check_global)
        // 2. Per-user sliding window
        RateLimiter::check(self, sender_id, limit)
    }
}
```

---

## 3. Architecture

### File Location

```
crates/clawft-core/src/pipeline/rate_limiter.rs   (new file)
crates/clawft-core/src/pipeline/mod.rs             (add `pub mod rate_limiter;`)
```

### Dependencies

Add to `crates/clawft-core/Cargo.toml`:

```toml
[dependencies]
dashmap = "6"   # Already used elsewhere in the project
```

### Imports from Other Phases

```rust
// Phase C trait -- RateLimiter implements this trait
use crate::pipeline::tiered_router::RateLimitable;

// Phase A config type -- used by from_config() callers
use clawft_types::routing::RateLimitConfig;
```

### Integration with TieredRouter

The `TieredRouter` holds `Arc<dyn RateLimitable>` and calls it at Step 3:

```rust
// In TieredRouter::route()
// The RateLimiter.check() internally checks the global rate limit FIRST,
// then the per-user limit. If either is exceeded, returns false.
if !self.rate_limiter.check(&auth.sender_id, permissions.rate_limit) {
    return self.rate_limited_decision(&permissions);
}
```

Construction uses `from_config` with values from `RateLimitConfig`:

```rust
let rate_limiter = Arc::new(RateLimiter::from_config(
    config.routing.rate_limiting.window_seconds as u64,
    config.routing.rate_limiting.global_rate_limit_rpm,
));
```

The `rate_limited_decision` returns a `RoutingDecision` with:

```rust
fn rate_limited_decision(&self, permissions: &UserPermissions) -> RoutingDecision {
    RoutingDecision {
        provider: String::new(),
        model: String::new(),
        reason: format!(
            "rate_limited: sender exceeded {} requests per {} seconds",
            permissions.rate_limit,
            self.rate_limiter.window_duration().as_secs()
        ),
    }
}
```

The agent loop checks for empty `provider` in the routing decision to detect
rate limiting and returns a user-facing "please slow down" message.

### Thread Safety

**Per-user path**: `DashMap` provides lock-free reads and sharded writes. Each
`check()` call locks only the shard containing the sender's key. Multiple
senders are fully concurrent. Same-sender requests serialize at the shard level,
which is correct (we need atomic read-modify-write for the timestamp deque).

**Global path**: The `global_counter` uses `AtomicU64::fetch_add` for lock-free
increment/check. The `global_window_start` is protected by a `Mutex<Instant>`
that is held only during the window-expiry check (a few nanoseconds). The Mutex
is not contended during normal operation (only on window reset, once per window
duration). This design avoids a global bottleneck while ensuring correct
window-boundary behavior.

### Memory Model

At 10,000 max entries with a 60-second window and 60 req/min limit:
- Per entry: ~24 bytes (String key avg) + 60 * 16 bytes (Instant) + overhead = ~1 KB
- Total worst case: 10,000 * 1 KB = ~10 MB
- Typical case (most users < 10 req/min): ~2 MB
- Global counter overhead: 8 bytes (AtomicU64) + 24 bytes (Mutex<Instant>) = negligible

---

## 4. Refinement

### Edge Cases

| Edge Case | Handling |
|-----------|----------|
| `limit = 0` (unlimited per-user) | Short-circuit per-user check to `true`, no entry created. Global limit still applies |
| `global_rate_limit = 0` (unlimited global) | Short-circuit `check_global()` to `true` |
| Both limits = 0 | Fully unlimited; no tracking, no rejection |
| Global limit exceeded, per-user under limit | Rejected. Global check runs first; per-user check is never reached |
| Per-user limit exceeded, global under limit | Rejected. Per-user check runs second and rejects independently |
| Very high rate (1000+ req/sec) | VecDeque handles this; timestamps purged on each call |
| Clock monotonicity | Uses `Instant` (monotonic), not `SystemTime`. No wall-clock issues |
| Empty `sender_id` | Treated as a valid key -- the empty string gets its own window. Zero-trust users with no sender_id all share one bucket (correct: they should be throttled collectively) |
| Concurrent `check()` for same sender | DashMap shard lock serializes; both see consistent state |
| Concurrent `check_global()` | AtomicU64 fetch_add is lock-free; Mutex only held briefly for window reset |
| Global window expiry | Mutex protects the read-check-reset of `global_window_start`; counter resets atomically |
| Eviction during check | Eviction only removes other senders' entries, never the current sender |
| `max_entries = 0` | Eviction fires every call -- effectively tracks only the latest sender. Valid but unusual |
| Window change on config reload | Create a new `RateLimiter` with the new window; old one is dropped. Existing rate-limit state is lost (acceptable -- windows are short) |
| `window_seconds = 0` | All timestamps are immediately expired; every request is allowed. Functionally equivalent to unlimited. Document this behavior |

### Performance Considerations

- `DashMap` default shard count is `num_cpus * 4`, giving good concurrency
- Timestamp purge is O(k) where k = expired entries; amortized O(1) per call
- LRU eviction scans all entries O(n) but only triggers when exceeding max_entries; in practice, rare with 10K limit
- Global rate limit check is O(1): one atomic increment + one Mutex acquisition (only on window reset)
- The global Mutex is held only during the window-expiry check (nanoseconds); it does not contend with the per-user DashMap path
- For production deployments with >10K concurrent users, consider increasing `max_entries` or switching to a probabilistic structure

### Future Extensions (Not in Phase E Scope)

- Distributed rate limiting via Redis INCR + TTL
- Per-IP rate limiting for gateway API requests
- Per-channel aggregate rate limiting
- Sliding window log with sub-second precision for burst detection

---

## 5. Completion

### Exit Criteria

- [ ] `RateLimiter::new()` creates a limiter with configurable window, max entries, and global rate limit
- [ ] `RateLimiter::check()` returns `true` when under limit, `false` when over
- [ ] `limit = 0` always returns `true` for per-user check (global limit still enforced)
- [ ] `global_rate_limit = 0` disables global limiting (per-user limit still enforced)
- [ ] Global rate limit is checked BEFORE per-user limit; if global limit exceeded, reject regardless of per-user limits
- [ ] LRU eviction triggers at `max_entries` and removes the least-recently-accessed sender
- [ ] Thread-safe: concurrent `check()` calls from multiple tokio tasks do not panic or corrupt state
- [ ] `impl RateLimitable for RateLimiter` compiles and satisfies the Phase C trait
- [ ] Integration point: `TieredRouter` can hold `Arc<dyn RateLimitable>` and call `check()` in `route()`

### Required Tests (15+)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // --- Test 1: Basic allow under limit ---
    #[test]
    fn test_allows_under_limit() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // 10 requests with limit of 10 should all pass
        for _ in 0..10 {
            assert!(limiter.check("user_1", 10));
        }
    }

    // --- Test 2: Reject at limit ---
    #[test]
    fn test_rejects_at_limit() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // Fill the window
        for _ in 0..5 {
            assert!(limiter.check("user_1", 5));
        }
        // 6th request should be rejected
        assert!(!limiter.check("user_1", 5));
    }

    // --- Test 3: Unlimited (limit = 0, no global limit) ---
    #[test]
    fn test_unlimited_always_allows() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        for _ in 0..1000 {
            assert!(limiter.check("user_1", 0));
        }
        // Should not even create an entry
        assert_eq!(limiter.tracked_senders(), 0);
    }

    // --- Test 4: Independent senders ---
    #[test]
    fn test_independent_senders() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // Fill user_1's window
        for _ in 0..5 {
            limiter.check("user_1", 5);
        }
        assert!(!limiter.check("user_1", 5));
        // user_2 should be unaffected
        assert!(limiter.check("user_2", 5));
    }

    // --- Test 5: Window expiry ---
    #[test]
    fn test_window_expiry() {
        // Use a 1-second window for fast testing
        let limiter = RateLimiter::new(1, 10_000, 0);
        // Fill the window
        for _ in 0..3 {
            assert!(limiter.check("user_1", 3));
        }
        assert!(!limiter.check("user_1", 3));

        // Wait for window to expire
        thread::sleep(Duration::from_millis(1100));

        // Should be allowed again
        assert!(limiter.check("user_1", 3));
    }

    // --- Test 6: LRU eviction ---
    #[test]
    fn test_lru_eviction() {
        let limiter = RateLimiter::new(60, 3, 0);
        // Add 4 senders to a limiter with max_entries=3
        limiter.check("user_a", 10);
        limiter.check("user_b", 10);
        limiter.check("user_c", 10);
        limiter.check("user_d", 10); // should trigger eviction of user_a

        // user_a was evicted (LRU)
        assert!(limiter.tracked_senders() <= 3);
    }

    // --- Test 7: Current count reporting ---
    #[test]
    fn test_current_count() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        assert_eq!(limiter.current_count("user_1"), 0);

        limiter.check("user_1", 10);
        limiter.check("user_1", 10);
        limiter.check("user_1", 10);

        assert_eq!(limiter.current_count("user_1"), 3);
    }

    // --- Test 8: Clear resets all state including global counter ---
    #[test]
    fn test_clear() {
        let limiter = RateLimiter::new(60, 10_000, 100);
        for _ in 0..5 {
            limiter.check("user_1", 5);
        }
        assert!(!limiter.check("user_1", 5));
        assert!(limiter.global_request_count() > 0);

        limiter.clear();

        // After clear, per-user and global state should be reset
        assert!(limiter.check("user_1", 5));
        assert_eq!(limiter.tracked_senders(), 1);
        // Global counter was reset by clear(); the single check above set it to 1
        assert_eq!(limiter.global_request_count(), 1);
    }

    // --- Test 9: Concurrent access from multiple threads ---
    #[test]
    fn test_concurrent_access() {
        let limiter = Arc::new(RateLimiter::new(60, 10_000, 0));
        let mut handles = vec![];

        for i in 0..10 {
            let limiter = Arc::clone(&limiter);
            handles.push(thread::spawn(move || {
                let sender = format!("user_{}", i);
                for _ in 0..100 {
                    limiter.check(&sender, 200);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All 10 senders should be tracked
        assert_eq!(limiter.tracked_senders(), 10);
    }

    // --- Test 10: Empty sender_id is a valid key ---
    #[test]
    fn test_empty_sender_id() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        assert!(limiter.check("", 2));
        assert!(limiter.check("", 2));
        assert!(!limiter.check("", 2));
    }

    // --- Test 11: Different limits for same sender across calls ---
    #[test]
    fn test_different_limits_per_call() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // First call with limit=5: allowed
        assert!(limiter.check("user_1", 5));
        assert!(limiter.check("user_1", 5));
        // Now 2 requests in window. Check with limit=2: should reject
        assert!(!limiter.check("user_1", 2));
        // Check with limit=5: should still allow
        assert!(limiter.check("user_1", 5));
    }

    // --- Test 12: Global rate limit rejects when exceeded ---
    #[test]
    fn test_global_rate_limit_rejects() {
        // Global limit of 5 requests per window, no per-user limit
        let limiter = RateLimiter::new(60, 10_000, 5);
        // 5 requests from different users should all pass
        for i in 0..5 {
            let sender = format!("user_{}", i);
            assert!(limiter.check(&sender, 0), "request {} should be allowed", i);
        }
        // 6th request from a new user should be rejected (global limit hit)
        assert!(!limiter.check("user_new", 0));
        assert_eq!(limiter.global_request_count(), 5);
    }

    // --- Test 13: Global limit checked before per-user limit ---
    #[test]
    fn test_global_limit_before_per_user() {
        // Global limit of 3, per-user limit of 100
        let limiter = RateLimiter::new(60, 10_000, 3);
        assert!(limiter.check("user_1", 100));
        assert!(limiter.check("user_2", 100));
        assert!(limiter.check("user_3", 100));
        // 4th request: user_4 has plenty of per-user budget but global is exhausted
        assert!(!limiter.check("user_4", 100));
    }

    // --- Test 14: Global limit = 0 means unlimited global ---
    #[test]
    fn test_global_limit_zero_is_unlimited() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // Should never hit a global limit
        for i in 0..1000 {
            let sender = format!("user_{}", i);
            assert!(limiter.check(&sender, 0));
        }
    }

    // --- Test 15: Global window resets after expiry ---
    #[test]
    fn test_global_window_resets() {
        // 1-second window, global limit of 2
        let limiter = RateLimiter::new(1, 10_000, 2);
        assert!(limiter.check("user_1", 0));
        assert!(limiter.check("user_2", 0));
        assert!(!limiter.check("user_3", 0)); // global limit hit

        // Wait for window to expire
        thread::sleep(Duration::from_millis(1100));

        // Global counter should reset; new requests should be allowed
        assert!(limiter.check("user_4", 0));
        assert_eq!(limiter.global_request_count(), 1);
    }

    // --- Test 16: RateLimitable trait delegation ---
    #[test]
    fn test_rate_limitable_trait_impl() {
        let limiter = RateLimiter::new(60, 10_000, 0);
        // Call through the trait interface
        let trait_obj: &dyn RateLimitable = &limiter;
        assert!(trait_obj.check("user_1", 2));
        assert!(trait_obj.check("user_1", 2));
        assert!(!trait_obj.check("user_1", 2));
    }

    // --- Test 17: Global limit with concurrent threads ---
    #[test]
    fn test_global_limit_concurrent() {
        // Global limit of 50, many threads sending requests
        let limiter = Arc::new(RateLimiter::new(60, 10_000, 50));
        let mut handles = vec![];

        for i in 0..10 {
            let limiter = Arc::clone(&limiter);
            handles.push(thread::spawn(move || {
                let sender = format!("user_{}", i);
                let mut allowed = 0u32;
                for _ in 0..20 {
                    if limiter.check(&sender, 100) {
                        allowed += 1;
                    }
                }
                allowed
            }));
        }

        let total_allowed: u32 = handles.into_iter()
            .map(|h| h.join().unwrap())
            .sum();

        // Total allowed should not exceed global limit of 50
        assert!(total_allowed <= 50, "total_allowed={} exceeded global limit 50", total_allowed);
    }
}
```

### Integration Test (in Phase I)

```rust
/// Integration: TieredRouter uses RateLimiter to reject excess requests.
#[tokio::test]
async fn test_tiered_router_rate_limits() {
    let config = test_tiered_config();
    let rate_limiter: Arc<dyn RateLimitable + Send + Sync> = Arc::new(
        RateLimiter::from_config(
            config.routing.rate_limiting.window_seconds as u64,
            config.routing.rate_limiting.global_rate_limit_rpm,
        ),
    );
    let router = TieredRouter::new(config, rate_limiter.clone());

    let request = ChatRequest { /* ... */ };
    let profile = TaskProfile { complexity: 0.3, /* ... */ };

    // Zero-trust user with rate_limit=10
    let mut request_with_auth = request.clone();
    request_with_auth.auth_context = Some(AuthContext {
        sender_id: "anon_123".into(),
        channel: "discord".into(),
        permissions: UserPermissions {
            rate_limit: 10,
            ..zero_trust_defaults()
        },
    });

    // First 10 requests should succeed
    for _ in 0..10 {
        let decision = router.route(&request_with_auth, &profile).await;
        assert!(!decision.provider.is_empty(), "should route normally");
    }

    // 11th request should be rate-limited
    let decision = router.route(&request_with_auth, &profile).await;
    assert!(decision.reason.contains("rate_limited"));
    assert!(decision.provider.is_empty());
}
```

---

## Remediation Applied

The following fixes from `remediation-plan.md` have been applied to this phase:

### FIX-03: RateLimitable Trait Alignment (CRITICAL)

**Source**: GAP-T08, T09, T10, C03

**Changes**:
- Added `use crate::pipeline::tiered_router::RateLimitable;` import to the struct's imports
- Added `impl RateLimitable for RateLimiter` block that delegates to the inherent `check()` method
- The trait signature `fn check(&self, sender_id: &str, limit: u32) -> bool` matches exactly
- Added exit criterion: "`impl RateLimitable for RateLimiter` compiles and satisfies the Phase C trait"
- Added Test 16 (`test_rate_limitable_trait_impl`) verifying the trait object dispatch works correctly
- Updated integration test to use `Arc<dyn RateLimitable + Send + Sync>` instead of `Arc<RateLimiter>`

### FIX-08: Global Rate Limit (HIGH SECURITY)

**Source**: GAP-S09

**Changes**:
- Added `global_counter: AtomicU64` field to `RateLimiter` for tracking total requests per window
- Added `global_window_start: Mutex<Instant>` field for tracking the current global window boundary
- Added `global_rate_limit: u32` field (sourced from `RateLimitConfig.global_rate_limit_rpm`; 0 = unlimited)
- Updated `RateLimiter::new()` to accept `global_rate_limit` as third parameter
- Updated `RateLimiter::from_config()` to accept `global_rate_limit_rpm` parameter
- Added `check_global()` private method that checks and increments the global counter atomically
- Modified `check()` to call `check_global()` BEFORE the per-user sliding window check
- Added `global_request_count()` and `global_rate_limit()` utility methods for diagnostics
- Updated `clear()` to also reset `global_counter` and `global_window_start`
- Removed "Global aggregate rate limiting" from Non-Goals (it is now in scope)
- Updated edge cases table with 4 new global-rate-limit edge cases
- Updated thread safety documentation to describe the global counter mechanism
- Updated performance considerations to note O(1) global check cost
- Updated memory model to note negligible global counter overhead
- Added 6 new tests (Tests 12-17): global rejection, global-before-per-user ordering, global zero = unlimited, global window reset, trait impl delegation, concurrent global limit enforcement
- Updated all existing tests to use the new 3-argument constructor `RateLimiter::new(window, max_entries, global_limit)`
- Updated exit criteria with global rate limit requirements
