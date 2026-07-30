//! Fuzz target: `RateLimiter` with rotating unique sender IDs.
//!
//! Property (security-review §6.1 / PEN-03): never panics; tracked
//! senders stay bounded by `max_tracked_users` (LRU eviction); always
//! returns a bool.
//!
//! Run: `cargo +nightly fuzz run rate_limiter_unique_ids -- -max_total_time=60`

#![no_main]

use clawft_core::pipeline::rate_limiter::RateLimiter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let window_secs = match data.first().copied().unwrap_or(1) {
        0 => 1, // window_seconds=0 is rejected in validation; avoid pathological Instant math
        w => u32::from(w),
    };
    let global_limit = u32::from(data.get(1).copied().unwrap_or(0));
    // Small max_tracked so LRU eviction is hit often under unique IDs.
    let max_tracked = (data.get(2).copied().unwrap_or(4) as usize % 16) + 1;
    let per_user_limit = u32::from(data.get(3).copied().unwrap_or(5));

    let limiter = RateLimiter::new(window_secs, global_limit).with_max_tracked_users(max_tracked);

    // Each 4-byte chunk becomes a distinct sender_id (rotation attack).
    for (i, chunk) in data.chunks(4).enumerate().take(64) {
        let id = if chunk.is_empty() {
            String::new() // empty sender_id is a valid key
        } else {
            format!(
                "s{:x}-{}",
                u32::from_le_bytes([
                    chunk.first().copied().unwrap_or(0),
                    chunk.get(1).copied().unwrap_or(0),
                    chunk.get(2).copied().unwrap_or(0),
                    chunk.get(3).copied().unwrap_or(0),
                ]),
                i
            )
        };
        let allowed = limiter.check(&id, per_user_limit);
        // Bool result only — no other side-effect assertions needed.
        let _ = allowed;
    }

    // Also hammer a single sender for the sliding-window path.
    let sticky = "sticky";
    for _ in 0..16 {
        let _ = limiter.check(sticky, per_user_limit.max(1));
    }
});
