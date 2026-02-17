//! `weft sessions` -- manage conversation sessions.
//!
//! Provides subcommands for listing, inspecting, and deleting sessions.
//! Sessions are JSONL files stored under `~/.clawft/workspace/sessions/`
//! (or `~/.nanobot/workspace/sessions/` as fallback).
//!
//! # Examples
//!
//! ```text
//! weft sessions list
//! weft sessions inspect telegram:12345
//! weft sessions delete telegram:12345
//! ```

use std::sync::Arc;

use comfy_table::{presets::UTF8_FULL, Table};

use clawft_core::session::SessionManager;
use clawft_platform::NativePlatform;
use clawft_types::config::Config;

/// Actions that can be performed on sessions (used in tests).
#[cfg(test)]
#[derive(Debug, Clone)]
pub enum SessionsAction {
    /// List all sessions.
    List,
    /// Inspect a specific session by key.
    Inspect { session_id: String },
    /// Delete a specific session by key.
    Delete { session_id: String },
}

/// Format a `chrono::DateTime<Utc>` as a human-readable string.
fn format_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Extract a display-friendly role string from a message JSON value.
fn message_role(msg: &serde_json::Value) -> &str {
    msg.get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
}

/// Extract a display-friendly content preview from a message JSON value.
///
/// Truncates to `max_len` characters and appends "..." if truncated.
fn message_content_preview(msg: &serde_json::Value, max_len: usize) -> String {
    let content = msg
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if content.len() <= max_len {
        content.to_string()
    } else {
        format!("{}...", &content[..max_len])
    }
}

/// List all sessions.
pub async fn sessions_list(_config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let mgr = SessionManager::new(platform).await?;

    let keys = mgr.list_sessions().await?;

    if keys.is_empty() {
        println!("No sessions found.");
        println!("  Dir: {}", mgr.sessions_dir().display());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["SESSION KEY", "MESSAGES", "LAST UPDATED"]);

    for key in &keys {
        match mgr.load_session(key).await {
            Ok(session) => {
                let msg_count = session.messages.len().to_string();
                let updated = format_datetime(&session.updated_at);
                table.add_row([key.as_str(), &msg_count, &updated]);
            }
            Err(_) => {
                // Session file may be corrupt; show what we can.
                table.add_row([key.as_str(), "?", "?"]);
            }
        }
    }

    println!("{table}");
    println!("  {} session(s)", keys.len());
    println!("  Dir: {}", mgr.sessions_dir().display());
    Ok(())
}

/// Inspect a single session, displaying its messages.
pub async fn sessions_inspect(session_id: String, _config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let mgr = SessionManager::new(platform).await?;

    let session = mgr
        .load_session(&session_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to load session '{}': {e}", session_id))?;

    println!("Session: {}", session.key);
    println!("  Created:  {}", format_datetime(&session.created_at));
    println!("  Updated:  {}", format_datetime(&session.updated_at));
    println!("  Messages: {}", session.messages.len());
    println!(
        "  Consolidated: {} of {}",
        session.last_consolidated,
        session.messages.len()
    );

    if !session.metadata.is_empty() {
        println!("  Metadata:");
        for (k, v) in &session.metadata {
            println!("    {k}: {v}");
        }
    }

    if session.messages.is_empty() {
        println!("\n  (no messages)");
        return Ok(());
    }

    println!();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["#", "ROLE", "CONTENT", "TIMESTAMP"]);

    for (i, msg) in session.messages.iter().enumerate() {
        let idx = (i + 1).to_string();
        let role = message_role(msg).to_string();
        let content = message_content_preview(msg, 80);
        let ts = msg
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .to_string();
        table.add_row([&idx, &role, &content, &ts]);
    }

    println!("{table}");
    Ok(())
}

/// Delete a session.
pub async fn sessions_delete(session_id: String, _config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let mgr = SessionManager::new(platform).await?;

    mgr.delete_session(&session_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to delete session '{}': {e}", session_id))?;

    println!("Session '{}' deleted.", session_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SessionsAction construction ──────────────────────────────────

    #[test]
    fn action_list_variant() {
        let action = SessionsAction::List;
        assert!(matches!(action, SessionsAction::List));
    }

    #[test]
    fn action_inspect_variant() {
        let action = SessionsAction::Inspect {
            session_id: "telegram:123".into(),
        };
        match action {
            SessionsAction::Inspect { session_id } => {
                assert_eq!(session_id, "telegram:123");
            }
            _ => panic!("expected Inspect variant"),
        }
    }

    #[test]
    fn action_delete_variant() {
        let action = SessionsAction::Delete {
            session_id: "slack:456".into(),
        };
        match action {
            SessionsAction::Delete { session_id } => {
                assert_eq!(session_id, "slack:456");
            }
            _ => panic!("expected Delete variant"),
        }
    }

    #[test]
    fn action_clone() {
        let action = SessionsAction::Inspect {
            session_id: "test:1".into(),
        };
        let cloned = action.clone();
        match cloned {
            SessionsAction::Inspect { session_id } => {
                assert_eq!(session_id, "test:1");
            }
            _ => panic!("expected Inspect variant"),
        }
    }

    #[test]
    fn action_debug_format() {
        let action = SessionsAction::List;
        let debug = format!("{:?}", action);
        assert!(debug.contains("List"));
    }

    // ── format_datetime ──────────────────────────────────────────────

    #[test]
    fn format_datetime_produces_expected_format() {
        use chrono::TimeZone;
        let dt = chrono::Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 0).unwrap();
        let result = format_datetime(&dt);
        assert_eq!(result, "2025-06-15 14:30:00");
    }

    #[test]
    fn format_datetime_epoch() {
        use chrono::TimeZone;
        let dt = chrono::Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        let result = format_datetime(&dt);
        assert_eq!(result, "1970-01-01 00:00:00");
    }

    // ── message_role ─────────────────────────────────────────────────

    #[test]
    fn message_role_extracts_role() {
        let msg = serde_json::json!({"role": "user", "content": "hello"});
        assert_eq!(message_role(&msg), "user");
    }

    #[test]
    fn message_role_missing_returns_unknown() {
        let msg = serde_json::json!({"content": "hello"});
        assert_eq!(message_role(&msg), "unknown");
    }

    #[test]
    fn message_role_non_string_returns_unknown() {
        let msg = serde_json::json!({"role": 42, "content": "hello"});
        assert_eq!(message_role(&msg), "unknown");
    }

    // ── message_content_preview ──────────────────────────────────────

    #[test]
    fn content_preview_short_message() {
        let msg = serde_json::json!({"role": "user", "content": "hi"});
        assert_eq!(message_content_preview(&msg, 80), "hi");
    }

    #[test]
    fn content_preview_truncates_long_message() {
        let long_content = "a".repeat(100);
        let msg = serde_json::json!({"role": "user", "content": long_content});
        let preview = message_content_preview(&msg, 10);
        assert_eq!(preview, "aaaaaaaaaa...");
        assert_eq!(preview.len(), 13); // 10 + "..."
    }

    #[test]
    fn content_preview_exact_length() {
        let content = "a".repeat(80);
        let msg = serde_json::json!({"role": "user", "content": content});
        let preview = message_content_preview(&msg, 80);
        assert_eq!(preview.len(), 80); // No "..." appended.
        assert!(!preview.ends_with("..."));
    }

    #[test]
    fn content_preview_missing_content() {
        let msg = serde_json::json!({"role": "user"});
        assert_eq!(message_content_preview(&msg, 80), "");
    }

    #[test]
    fn content_preview_non_string_content() {
        let msg = serde_json::json!({"role": "user", "content": 42});
        assert_eq!(message_content_preview(&msg, 80), "");
    }
}
