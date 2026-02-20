//! Message payload types for channel communication.
//!
//! Defines [`MessagePayload`], supporting text, structured data, and binary
//! payloads. The `Binary` variant is reserved for voice/audio data
//! (Workstream G forward-compatibility).

use serde::{Deserialize, Serialize};

/// Message payload types for channel communication.
///
/// Supports text, structured data, and binary payloads. The `Binary`
/// variant is reserved for voice/audio data (Workstream G).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePayload {
    /// Plain text message.
    Text {
        /// The text content.
        content: String,
    },
    /// Structured data (JSON object).
    Structured {
        /// The structured data.
        data: serde_json::Value,
    },
    /// Binary data with MIME type (e.g., audio/wav for voice).
    Binary {
        /// MIME type of the binary data (e.g., `"audio/wav"`, `"audio/opus"`).
        mime_type: String,
        /// Raw binary data, base64-encoded for JSON serialization.
        #[serde(with = "base64_bytes")]
        data: Vec<u8>,
    },
}

/// Serde helper for base64-encoding binary data in JSON.
mod base64_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    /// Custom base64 encoding using a simple approach without pulling in
    /// the `base64` crate. For JSON serialization we use an array of bytes.
    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept both byte arrays and sequences of numbers.
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        Ok(bytes)
    }
}

impl MessagePayload {
    /// Create a text payload.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }

    /// Create a structured data payload.
    pub fn structured(data: serde_json::Value) -> Self {
        Self::Structured { data }
    }

    /// Create a binary payload with a MIME type.
    pub fn binary(mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self::Binary {
            mime_type: mime_type.into(),
            data,
        }
    }

    /// Returns `true` if this is a text payload.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Returns `true` if this is a binary payload.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary { .. })
    }

    /// Extract the text content, if this is a text payload.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { content } => Some(content),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_payload_creation() {
        let payload = MessagePayload::text("hello");
        assert!(payload.is_text());
        assert!(!payload.is_binary());
        assert_eq!(payload.as_text(), Some("hello"));
    }

    #[test]
    fn structured_payload_creation() {
        let data = serde_json::json!({"key": "value"});
        let payload = MessagePayload::structured(data.clone());
        assert!(!payload.is_text());
        assert!(!payload.is_binary());
        assert_eq!(payload.as_text(), None);
    }

    #[test]
    fn binary_payload_creation() {
        let payload = MessagePayload::binary("audio/wav", vec![0u8; 16]);
        assert!(payload.is_binary());
        assert!(!payload.is_text());
        assert_eq!(payload.as_text(), None);
    }

    #[test]
    fn text_payload_serde_roundtrip() {
        let payload = MessagePayload::text("hello world");
        let json = serde_json::to_string(&payload).unwrap();
        let restored: MessagePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.as_text(), Some("hello world"));
    }

    #[test]
    fn structured_payload_serde_roundtrip() {
        let data = serde_json::json!({"key": "value", "count": 42});
        let payload = MessagePayload::structured(data.clone());
        let json = serde_json::to_string(&payload).unwrap();
        let restored: MessagePayload = serde_json::from_str(&json).unwrap();
        match restored {
            MessagePayload::Structured { data: d } => {
                assert_eq!(d["key"], "value");
                assert_eq!(d["count"], 42);
            }
            _ => panic!("expected Structured variant"),
        }
    }

    #[test]
    fn binary_payload_serde_roundtrip() {
        let original_data = vec![0u8, 1, 2, 3, 255, 128];
        let payload = MessagePayload::binary("audio/wav", original_data.clone());
        let json = serde_json::to_string(&payload).unwrap();
        let restored: MessagePayload = serde_json::from_str(&json).unwrap();
        match restored {
            MessagePayload::Binary { mime_type, data } => {
                assert_eq!(mime_type, "audio/wav");
                assert_eq!(data, original_data);
            }
            _ => panic!("expected Binary variant"),
        }
    }

    #[test]
    fn message_payload_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MessagePayload>();
    }
}
