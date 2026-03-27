//! WebSocket event types for voice status reporting.
//!
//! Defines the `voice:status` event that is broadcast to WebSocket
//! clients when the voice channel status changes.

use serde::{Deserialize, Serialize};

use super::channel::VoiceStatus;

/// WebSocket event for voice status changes.
///
/// Sent to connected WebSocket clients whenever the voice pipeline
/// transitions between states (idle, listening, transcribing,
/// processing, speaking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceWsEvent {
    /// Event type identifier. Always `"voice:status"`.
    pub event: String,
    /// Current voice pipeline status.
    pub status: VoiceStatus,
    /// Unix timestamp in milliseconds when the event occurred.
    pub timestamp: u64,
}

impl VoiceWsEvent {
    /// Create a new voice status event with the current timestamp.
    pub fn new(status: VoiceStatus) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            event: "voice:status".into(),
            status,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_ws_event_new() {
        let event = VoiceWsEvent::new(VoiceStatus::Listening);
        assert_eq!(event.event, "voice:status");
        assert_eq!(event.status, VoiceStatus::Listening);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn voice_ws_event_serde_roundtrip() {
        let event = VoiceWsEvent {
            event: "voice:status".into(),
            status: VoiceStatus::Speaking,
            timestamp: 1700000000000,
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: VoiceWsEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.event, "voice:status");
        assert_eq!(restored.status, VoiceStatus::Speaking);
        assert_eq!(restored.timestamp, 1700000000000);
    }

    #[test]
    fn voice_ws_event_json_structure() {
        let event = VoiceWsEvent {
            event: "voice:status".into(),
            status: VoiceStatus::Idle,
            timestamp: 1700000000000,
        };
        let json: serde_json::Value = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event"], "voice:status");
        assert_eq!(json["status"], "idle");
        assert_eq!(json["timestamp"], 1700000000000_u64);
    }

    #[test]
    fn voice_ws_event_all_statuses() {
        let statuses = vec![
            VoiceStatus::Idle,
            VoiceStatus::Listening,
            VoiceStatus::Transcribing,
            VoiceStatus::Processing,
            VoiceStatus::Speaking,
        ];
        for status in statuses {
            let event = VoiceWsEvent::new(status);
            let json = serde_json::to_string(&event).unwrap();
            let restored: VoiceWsEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.status, status);
        }
    }
}
