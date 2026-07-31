//! Real backend broadcaster for the `voice:status` WebSocket topic (WEFT-218).
//!
//! Lives in the daemon/services stack (not the deprecated plugin
//! `VoiceWsEvent` scaffold). Producers call [`VoiceStatusHub::publish`] /
//! [`VoiceAccess::update_pipeline_state`]; dashboard clients subscribe to
//! topic [`VOICE_STATUS_TOPIC`] and receive the standard WS envelope:
//!
//! ```json
//! {"type":"event","topic":"voice:status","data":{...}}
//! ```

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::broadcaster::TopicBroadcaster;
use super::VoiceStatusInfo;

/// WebSocket / TopicBroadcaster topic for voice pipeline state.
pub const VOICE_STATUS_TOPIC: &str = "voice:status";

/// Allowed pipeline state strings (UI + REST contract).
pub const VALID_STATES: &[&str] = &["idle", "listening", "processing", "speaking"];

/// Wire payload published on [`VOICE_STATUS_TOPIC`].
///
/// Field names match the clawft-ui store (`state`, `talkModeActive`,
/// `transcript`, `response`) plus `event`/`status`/`timestamp` for
/// protocol compatibility with the historical plugin event shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceStatusEvent {
    /// Always `"voice:status"`.
    pub event: String,
    /// Snake-case pipeline state (same as [`Self::state`]).
    pub status: String,
    /// UI pipeline state: idle | listening | processing | speaking.
    pub state: String,
    /// Whether talk mode is active.
    #[serde(rename = "talkModeActive")]
    pub talk_mode_active: bool,
    /// Optional partial / final user transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    /// Optional agent response text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    /// Wake-word flag mirrored from config when known.
    #[serde(
        rename = "wakeWordEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub wake_word_enabled: Option<bool>,
    /// Unix epoch milliseconds.
    pub timestamp: u64,
}

impl VoiceStatusEvent {
    /// Build an event from runtime fields.
    pub fn new(
        state: impl Into<String>,
        talk_mode_active: bool,
        transcript: Option<String>,
        response: Option<String>,
        wake_word_enabled: Option<bool>,
    ) -> Self {
        let state = state.into();
        Self {
            event: VOICE_STATUS_TOPIC.into(),
            status: state.clone(),
            state,
            talk_mode_active,
            transcript,
            response,
            wake_word_enabled,
            timestamp: now_ms(),
        }
    }

    /// Serialize to the JSON value published on the broadcaster.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "event": VOICE_STATUS_TOPIC,
                "status": &self.state,
                "state": &self.state,
                "talkModeActive": self.talk_mode_active,
                "timestamp": self.timestamp,
            })
        })
    }
}

/// Partial update for runtime pipeline state (REST body).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VoicePipelineUpdate {
    /// New pipeline state (idle|listening|processing|speaking).
    pub state: Option<String>,
    /// Talk-mode active flag.
    #[serde(rename = "talkModeActive")]
    pub talk_mode_active: Option<bool>,
    /// Optional transcript (omit to leave unchanged; `null` clears).
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub transcript: OptionalField<String>,
    /// Optional response text.
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub response: OptionalField<String>,
}

/// Tri-state field so JSON can omit, set, or clear a string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OptionalField<T> {
    /// Field absent from the request — leave previous value.
    #[default]
    Absent,
    /// Explicit `null` — clear the value.
    Clear,
    /// New value.
    Set(T),
}

impl<T: Serialize> Serialize for OptionalField<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OptionalField::Absent | OptionalField::Clear => serializer.serialize_none(),
            OptionalField::Set(v) => v.serialize(serializer),
        }
    }
}

fn deserialize_optional_string<'de, D>(
    deserializer: D,
) -> Result<OptionalField<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(match opt {
        None => OptionalField::Clear,
        Some(s) => OptionalField::Set(s),
    })
}

/// Validate and normalize a pipeline state string.
pub fn normalize_state(state: &str) -> Result<&'static str, String> {
    let lower = state.trim().to_ascii_lowercase();
    // Map plugin-era "transcribing" onto UI "processing".
    let mapped = match lower.as_str() {
        "transcribing" => "processing",
        other => other,
    };
    VALID_STATES
        .iter()
        .copied()
        .find(|&s| s == mapped)
        .ok_or_else(|| {
            format!(
                "invalid voice state '{state}'; expected one of {}",
                VALID_STATES.join(", ")
            )
        })
}

/// Hub that owns the last published snapshot and fans out on the topic.
///
/// Cheap to clone (`Arc` internals). Safe to call from sync contexts when a
/// Tokio runtime is present (publish is fire-and-forget via `spawn`).
#[derive(Clone)]
pub struct VoiceStatusHub {
    broadcaster: Arc<TopicBroadcaster>,
    /// Last event published (for tests / late REST readers).
    last: Arc<std::sync::RwLock<Option<VoiceStatusEvent>>>,
}

impl VoiceStatusHub {
    /// Create a hub bound to a shared [`TopicBroadcaster`].
    pub fn new(broadcaster: Arc<TopicBroadcaster>) -> Self {
        Self {
            broadcaster,
            last: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// Publish a full status event. Ensures the topic channel exists so
    /// late subscribers can attach; messages with zero receivers are still
    /// dropped by `broadcast` (ephemeral live signal).
    pub async fn publish(&self, event: VoiceStatusEvent) {
        if let Ok(mut guard) = self.last.write() {
            *guard = Some(event.clone());
        }
        // Create the topic if needed, then fan out.
        let _ = self.broadcaster.get_or_create(VOICE_STATUS_TOPIC).await;
        self.broadcaster
            .publish(VOICE_STATUS_TOPIC, event.to_json())
            .await;
    }

    /// Fire-and-forget publish when a Tokio runtime is available.
    ///
    /// No-ops silently when called outside a runtime (e.g. pure unit tests
    /// of pure helpers). Prefer [`Self::publish`] in async tests.
    pub fn publish_spawn(&self, event: VoiceStatusEvent) {
        if let Ok(mut guard) = self.last.write() {
            *guard = Some(event.clone());
        }
        let bc = self.broadcaster.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = bc.get_or_create(VOICE_STATUS_TOPIC).await;
                bc.publish(VOICE_STATUS_TOPIC, event.to_json()).await;
            });
        }
    }

    /// Last published event, if any.
    pub fn last_event(&self) -> Option<VoiceStatusEvent> {
        self.last.read().ok().and_then(|g| g.clone())
    }
}

/// Build a [`VoiceStatusInfo`] snapshot from runtime fields.
pub fn status_info(
    state: &str,
    talk_mode_active: bool,
    wake_word_enabled: bool,
) -> VoiceStatusInfo {
    VoiceStatusInfo {
        state: state.to_string(),
        talk_mode_active,
        wake_word_enabled,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_state_accepts_ui_values() {
        for s in VALID_STATES {
            assert_eq!(normalize_state(s).unwrap(), *s);
        }
        assert_eq!(normalize_state("Listening").unwrap(), "listening");
        assert_eq!(normalize_state("transcribing").unwrap(), "processing");
    }

    #[test]
    fn normalize_state_rejects_unknown() {
        assert!(normalize_state("screaming").is_err());
    }

    #[test]
    fn event_json_shape() {
        let ev = VoiceStatusEvent::new(
            "listening",
            true,
            Some("hello".into()),
            None,
            Some(false),
        );
        let json = ev.to_json();
        assert_eq!(json["event"], "voice:status");
        assert_eq!(json["status"], "listening");
        assert_eq!(json["state"], "listening");
        assert_eq!(json["talkModeActive"], true);
        assert_eq!(json["transcript"], "hello");
        assert_eq!(json["wakeWordEnabled"], false);
        assert!(json["timestamp"].as_u64().unwrap() > 0);
        assert!(json.get("response").is_none());
    }

    #[test]
    fn pipeline_update_deserialize_omit_vs_null() {
        let omit: VoicePipelineUpdate = serde_json::from_str(r#"{"state":"listening"}"#).unwrap();
        assert!(matches!(omit.transcript, OptionalField::Absent));
        assert_eq!(omit.state.as_deref(), Some("listening"));

        let clear: VoicePipelineUpdate =
            serde_json::from_str(r#"{"transcript":null}"#).unwrap();
        assert!(matches!(clear.transcript, OptionalField::Clear));

        let set: VoicePipelineUpdate =
            serde_json::from_str(r#"{"transcript":"hi"}"#).unwrap();
        assert_eq!(set.transcript, OptionalField::Set("hi".into()));
    }

    #[tokio::test]
    async fn hub_publish_reaches_subscriber() {
        let bc = Arc::new(TopicBroadcaster::new());
        let mut rx = bc.subscribe(VOICE_STATUS_TOPIC).await;
        let hub = VoiceStatusHub::new(bc);
        let ev = VoiceStatusEvent::new("speaking", true, None, Some("yo".into()), None);
        hub.publish(ev.clone()).await;

        let received = rx.recv().await.unwrap();
        let parsed: VoiceStatusEvent = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed.state, "speaking");
        assert_eq!(parsed.response.as_deref(), Some("yo"));
        assert_eq!(parsed.event, "voice:status");
        assert_eq!(hub.last_event().unwrap().state, "speaking");
    }
}
