//! Per-conversation voice-loop decision trace — the observability feed
//! behind `weft voice watch`'s process view.
//!
//! The committed `conversation.graph` shows WHAT the conversation said; this
//! trace shows HOW the agent got there: gate decisions (and why), router
//! verdicts, the brain call's model / wall time / tool activity / reasoning,
//! and client-side process events (TTS render time, cue tones) posted by the
//! talk client over `voice.trace.append`. Everything here is ephemeral
//! surface data — never a chain anchor, never durable state.
//!
//! One bounded ring per conversation (drop-oldest), read incrementally by
//! index over the `voice.trace` RPC so a poller only sees new events.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Cap on one conversation's retained trace events. A watch poller keeps up
/// at ~1 Hz; the cap only matters when nobody is watching — drop-oldest, the
/// trace is a window, not a log.
const TRACE_CAP: usize = 512;

/// One trace event. `kind` is a small closed vocabulary the renderer keys
/// on (`"gate"`, `"route"`, `"dispatch"`, `"brain"`, `"brain_error"`,
/// `"attempt"`, plus client-posted process kinds like `"tts"` / `"cue"`);
/// `detail` is kind-specific structure rendered best-effort so new fields
/// never break an older watch binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Monotonic per-daemon index — the `since` cursor for incremental reads.
    pub idx: u64,
    /// Wall-clock milliseconds since the Unix epoch, for inter-event gaps.
    pub ts_ms: u64,
    /// Event kind (see type docs).
    pub kind: String,
    /// Kind-specific payload.
    pub detail: serde_json::Value,
}

/// The daemon-global trace: conv_id → bounded ring.
#[derive(Default)]
pub struct VoiceTrace {
    rings: DashMap<String, Mutex<VecDeque<TraceEvent>>>,
    next_idx: AtomicU64,
}

impl VoiceTrace {
    /// Append one event to `conv_id`'s ring (drop-oldest at [`TRACE_CAP`]).
    pub fn record(&self, conv_id: &str, kind: &str, detail: serde_json::Value) {
        let event = TraceEvent {
            idx: self.next_idx.fetch_add(1, Ordering::Relaxed),
            ts_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            kind: kind.to_string(),
            detail,
        };
        let ring = self
            .rings
            .entry(conv_id.to_string())
            .or_insert_with(|| Mutex::new(VecDeque::new()));
        let mut ring = ring.lock().expect("voice trace ring poisoned");
        if ring.len() >= TRACE_CAP {
            ring.pop_front();
        }
        ring.push_back(event);
    }

    /// Events for `conv_id` with `idx > since`, oldest first. `since = 0`
    /// returns the whole retained window (indices start at... whatever the
    /// daemon-global counter was — callers treat idx as opaque cursor).
    pub fn read(&self, conv_id: &str, since: Option<u64>) -> Vec<TraceEvent> {
        let Some(ring) = self.rings.get(conv_id) else {
            return Vec::new();
        };
        let ring = ring.lock().expect("voice trace ring poisoned");
        ring.iter()
            .filter(|e| since.is_none_or(|s| e.idx > s))
            .cloned()
            .collect()
    }
}

/// The daemon-global instance (same singleton idiom as the `DAEMON_*`
/// statics in `daemon.rs` — the trace must be reachable from both the voice
/// loop and the RPC arms without threading a handle through either).
static VOICE_TRACE: std::sync::OnceLock<VoiceTrace> = std::sync::OnceLock::new();

/// The daemon's voice trace.
pub fn voice_trace() -> &'static VoiceTrace {
    VOICE_TRACE.get_or_init(VoiceTrace::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_incremental_read() {
        let t = VoiceTrace::default();
        t.record("c1", "route", serde_json::json!({ "action": "Turn" }));
        t.record("c1", "brain", serde_json::json!({ "duration_ms": 1200 }));
        t.record("c2", "route", serde_json::json!({ "action": "Stop" }));

        let all = t.read("c1", None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].kind, "route");
        assert_eq!(all[1].kind, "brain");
        assert!(all[0].idx < all[1].idx, "indices are monotonic");

        // Incremental: only events after the cursor.
        let newer = t.read("c1", Some(all[0].idx));
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].kind, "brain");

        // Conversations are isolated.
        assert_eq!(t.read("c2", None).len(), 1);
        assert!(t.read("unknown", None).is_empty());
    }

    #[test]
    fn ring_drops_oldest_at_cap() {
        let t = VoiceTrace::default();
        for i in 0..(TRACE_CAP + 10) {
            t.record("c", "route", serde_json::json!({ "i": i }));
        }
        let events = t.read("c", None);
        assert_eq!(events.len(), TRACE_CAP);
        assert_eq!(
            events[0].detail.get("i").and_then(|v| v.as_u64()),
            Some(10),
            "oldest 10 dropped"
        );
    }
}
