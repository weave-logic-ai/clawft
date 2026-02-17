//! Conversation session types.
//!
//! [`Session`] stores an append-only message history for a single
//! channel + chat_id pair. It is designed for LLM cache efficiency:
//! consolidation writes summaries to external files but never mutates
//! the in-memory message list.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation session.
///
/// Messages are append-only for LLM cache efficiency. The consolidation
/// process writes summaries to `MEMORY.md` / `HISTORY.md` but does **not**
/// modify the messages list or [`get_history`](Session::get_history) output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session key, typically `"{channel}:{chat_id}"`.
    pub key: String,

    /// Ordered list of messages (append-only).
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,

    /// When the session was first created.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,

    /// When the session was last updated.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,

    /// Arbitrary session metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Number of messages already consolidated to external files.
    #[serde(default)]
    pub last_consolidated: usize,
}

impl Session {
    /// Create a new empty session with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            last_consolidated: 0,
        }
    }

    /// Append a message to the session.
    ///
    /// Extra fields can be passed via `extras` and will be merged into
    /// the message object alongside `role`, `content`, and `timestamp`.
    pub fn add_message(
        &mut self,
        role: &str,
        content: &str,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) {
        let mut msg = serde_json::json!({
            "role": role,
            "content": content,
            "timestamp": Utc::now().to_rfc3339(),
        });

        if let Some(extras) = extras {
            if let Some(obj) = msg.as_object_mut() {
                for (k, v) in extras {
                    obj.insert(k, v);
                }
            }
        }

        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Get recent messages in LLM format (`role` + `content` only).
    ///
    /// Returns at most `max_messages` entries from the end of the history.
    pub fn get_history(&self, max_messages: usize) -> Vec<serde_json::Value> {
        let start = self.messages.len().saturating_sub(max_messages);
        self.messages[start..]
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": m.get("role").and_then(|v| v.as_str()).unwrap_or("user"),
                    "content": m.get("content").and_then(|v| v.as_str()).unwrap_or(""),
                })
            })
            .collect()
    }

    /// Clear all messages and reset consolidation state.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.last_consolidated = 0;
        self.updated_at = Utc::now();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session() {
        let s = Session::new("telegram:123");
        assert_eq!(s.key, "telegram:123");
        assert!(s.messages.is_empty());
        assert_eq!(s.last_consolidated, 0);
    }

    #[test]
    fn add_message_basic() {
        let mut s = Session::new("test");
        s.add_message("user", "hello", None);
        s.add_message("assistant", "hi there", None);
        assert_eq!(s.messages.len(), 2);
        assert_eq!(s.messages[0]["role"], "user");
        assert_eq!(s.messages[1]["content"], "hi there");
    }

    #[test]
    fn add_message_with_extras() {
        let mut s = Session::new("test");
        let mut extras = HashMap::new();
        extras.insert("tool_calls".into(), serde_json::json!([{"id": "tc1"}]));
        s.add_message("assistant", "let me check", Some(extras));
        assert!(s.messages[0].get("tool_calls").is_some());
    }

    #[test]
    fn get_history_all() {
        let mut s = Session::new("test");
        s.add_message("user", "one", None);
        s.add_message("assistant", "two", None);
        let hist = s.get_history(500);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0]["role"], "user");
        assert_eq!(hist[1]["content"], "two");
    }

    #[test]
    fn get_history_truncated() {
        let mut s = Session::new("test");
        for i in 0..10 {
            s.add_message("user", &format!("msg {i}"), None);
        }
        let hist = s.get_history(3);
        assert_eq!(hist.len(), 3);
        assert_eq!(hist[0]["content"], "msg 7");
        assert_eq!(hist[2]["content"], "msg 9");
    }

    #[test]
    fn clear_resets_state() {
        let mut s = Session::new("test");
        s.add_message("user", "hello", None);
        s.last_consolidated = 1;
        s.clear();
        assert!(s.messages.is_empty());
        assert_eq!(s.last_consolidated, 0);
    }

    #[test]
    fn serde_roundtrip() {
        let mut s = Session::new("slack:C123");
        s.add_message("user", "test", None);
        s.last_consolidated = 0;

        let json = serde_json::to_string(&s).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.key, "slack:C123");
        assert_eq!(restored.messages.len(), 1);
    }

    #[test]
    fn default_session() {
        let s = Session::default();
        assert_eq!(s.key, "");
        assert!(s.messages.is_empty());
    }
}
