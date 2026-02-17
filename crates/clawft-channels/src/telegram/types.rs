//! Telegram Bot API types for deserialization.
//!
//! These types model the subset of the Telegram Bot API response format
//! needed by the [`TelegramClient`](super::client::TelegramClient).

use serde::{Deserialize, Serialize};

/// Wrapper for all Telegram Bot API responses.
///
/// Every API method returns `{ ok: bool, result?: T, description?: String }`.
/// When `ok` is `false`, `description` contains the error message.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramResponse<T> {
    /// Whether the request was successful.
    pub ok: bool,
    /// The result payload, present when `ok` is `true`.
    pub result: Option<T>,
    /// Human-readable error description, present when `ok` is `false`.
    pub description: Option<String>,
}

/// A single update from the `getUpdates` long-polling endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    /// Monotonically increasing update identifier.
    pub update_id: i64,
    /// The message associated with this update, if any.
    pub message: Option<Message>,
}

/// A Telegram message.
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    /// Unique message identifier within the chat.
    pub message_id: i64,
    /// Sender of the message. Absent for messages in channels.
    pub from: Option<User>,
    /// Chat the message belongs to.
    pub chat: Chat,
    /// Text content of the message, if any.
    pub text: Option<String>,
    /// Unix timestamp of when the message was sent.
    pub date: i64,
}

/// A Telegram user or bot.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    /// Unique user identifier.
    pub id: i64,
    /// Whether this user is a bot.
    pub is_bot: bool,
    /// User's first name.
    pub first_name: String,
    /// User's username (without leading `@`), if set.
    pub username: Option<String>,
}

/// A Telegram chat (private, group, supergroup, or channel).
#[derive(Debug, Clone, Deserialize)]
pub struct Chat {
    /// Unique chat identifier.
    pub id: i64,
    /// Chat type: `"private"`, `"group"`, `"supergroup"`, or `"channel"`.
    #[serde(rename = "type")]
    pub chat_type: String,
    /// Title of the chat (for groups, supergroups, channels).
    pub title: Option<String>,
    /// Username of the chat (for private chats with usernames set).
    pub username: Option<String>,
}

/// Request body for the `sendMessage` API method.
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    /// Target chat identifier.
    pub chat_id: i64,
    /// Text of the message to send.
    pub text: String,
    /// Parse mode for formatting (e.g., `"HTML"`, `"Markdown"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    /// If set, the sent message will be a reply to this message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_successful_get_me_response() {
        let json = r#"{
            "ok": true,
            "result": {
                "id": 123456789,
                "is_bot": true,
                "first_name": "TestBot",
                "username": "test_bot"
            }
        }"#;
        let resp: TelegramResponse<User> = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        let user = resp.result.unwrap();
        assert_eq!(user.id, 123456789);
        assert!(user.is_bot);
        assert_eq!(user.first_name, "TestBot");
        assert_eq!(user.username.as_deref(), Some("test_bot"));
    }

    #[test]
    fn deserialize_error_response() {
        let json = r#"{
            "ok": false,
            "description": "Unauthorized"
        }"#;
        let resp: TelegramResponse<User> = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert!(resp.result.is_none());
        assert_eq!(resp.description.as_deref(), Some("Unauthorized"));
    }

    #[test]
    fn deserialize_update_with_message() {
        let json = r#"{
            "update_id": 100,
            "message": {
                "message_id": 42,
                "from": {
                    "id": 999,
                    "is_bot": false,
                    "first_name": "Alice",
                    "username": "alice"
                },
                "chat": {
                    "id": -1001234,
                    "type": "group",
                    "title": "Test Group"
                },
                "text": "Hello, bot!",
                "date": 1700000000
            }
        }"#;
        let update: Update = serde_json::from_str(json).unwrap();
        assert_eq!(update.update_id, 100);
        let msg = update.message.unwrap();
        assert_eq!(msg.message_id, 42);
        assert_eq!(msg.text.as_deref(), Some("Hello, bot!"));
        assert_eq!(msg.date, 1700000000);
        let from = msg.from.unwrap();
        assert_eq!(from.id, 999);
        assert!(!from.is_bot);
        assert_eq!(msg.chat.id, -1001234);
        assert_eq!(msg.chat.chat_type, "group");
        assert_eq!(msg.chat.title.as_deref(), Some("Test Group"));
    }

    #[test]
    fn deserialize_update_without_message() {
        let json = r#"{"update_id": 101}"#;
        let update: Update = serde_json::from_str(json).unwrap();
        assert_eq!(update.update_id, 101);
        assert!(update.message.is_none());
    }

    #[test]
    fn deserialize_message_without_from() {
        let json = r#"{
            "message_id": 50,
            "chat": {
                "id": -100,
                "type": "channel"
            },
            "text": "channel post",
            "date": 1700000001
        }"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert!(msg.from.is_none());
        assert_eq!(msg.chat.chat_type, "channel");
    }

    #[test]
    fn deserialize_message_without_text() {
        let json = r#"{
            "message_id": 51,
            "chat": {"id": 1, "type": "private"},
            "date": 1700000002
        }"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert!(msg.text.is_none());
    }

    #[test]
    fn deserialize_private_chat() {
        let json = r#"{
            "id": 42,
            "type": "private",
            "username": "bob"
        }"#;
        let chat: Chat = serde_json::from_str(json).unwrap();
        assert_eq!(chat.id, 42);
        assert_eq!(chat.chat_type, "private");
        assert!(chat.title.is_none());
        assert_eq!(chat.username.as_deref(), Some("bob"));
    }

    #[test]
    fn deserialize_get_updates_response() {
        let json = r#"{
            "ok": true,
            "result": [
                {
                    "update_id": 200,
                    "message": {
                        "message_id": 1,
                        "from": {"id": 10, "is_bot": false, "first_name": "Eve"},
                        "chat": {"id": 10, "type": "private"},
                        "text": "hi",
                        "date": 1700000010
                    }
                },
                {
                    "update_id": 201,
                    "message": {
                        "message_id": 2,
                        "from": {"id": 11, "is_bot": false, "first_name": "Frank"},
                        "chat": {"id": 11, "type": "private"},
                        "text": "hello",
                        "date": 1700000011
                    }
                }
            ]
        }"#;
        let resp: TelegramResponse<Vec<Update>> =
            serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        let updates = resp.result.unwrap();
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].update_id, 200);
        assert_eq!(updates[1].update_id, 201);
    }

    #[test]
    fn serialize_send_message_request_minimal() {
        let req = SendMessageRequest {
            chat_id: 42,
            text: "Hello!".into(),
            parse_mode: None,
            reply_to_message_id: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["chat_id"], 42);
        assert_eq!(json["text"], "Hello!");
        // Optional fields should be absent, not null
        assert!(json.get("parse_mode").is_none());
        assert!(json.get("reply_to_message_id").is_none());
    }

    #[test]
    fn serialize_send_message_request_full() {
        let req = SendMessageRequest {
            chat_id: 42,
            text: "Hello!".into(),
            parse_mode: Some("HTML".into()),
            reply_to_message_id: Some(10),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["chat_id"], 42);
        assert_eq!(json["text"], "Hello!");
        assert_eq!(json["parse_mode"], "HTML");
        assert_eq!(json["reply_to_message_id"], 10);
    }

    #[test]
    fn deserialize_user_without_username() {
        let json = r#"{
            "id": 777,
            "is_bot": false,
            "first_name": "NoUsername"
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 777);
        assert!(user.username.is_none());
    }

    #[test]
    fn deserialize_send_message_response() {
        let json = r#"{
            "ok": true,
            "result": {
                "message_id": 99,
                "chat": {"id": 42, "type": "private"},
                "date": 1700000099
            }
        }"#;
        let resp: TelegramResponse<Message> = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        let msg = resp.result.unwrap();
        assert_eq!(msg.message_id, 99);
        assert_eq!(msg.chat.id, 42);
    }
}
