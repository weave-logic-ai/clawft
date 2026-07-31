//! OPFS layout for browser conversation history + per-group CLAWFT.md (WEFT-399).
//!
//! Path helpers only — no I/O. See `docs/browser/opfs-history.md` for the
//! full design (CLAUDE.md-per-group, openbrowserclaw-style groups).

use std::path::{Path, PathBuf};

use super::{BROWSER_HOME_DIR, BROWSER_WORKSPACE_DIR};

/// Default browser chat group id (matches `send_message` chat_id).
pub const DEFAULT_GROUP_ID: &str = "browser";

/// Channel used by the browser WASM entry for session keys.
pub const BROWSER_CHANNEL: &str = "web";

/// Persisted Config JSON from the last successful `init` (P6.4).
///
/// Absolute virtual path under the browser home. Readable after reload
/// via [`config_persist_path`].
pub const CONFIG_PERSIST_REL: &str = ".clawft/config.json";

/// Directory holding percent-encoded session JSONL files
/// (`LocalFileSink` / `SessionManager` layout under browser home).
pub const SESSIONS_REL: &str = ".clawft/workspace/sessions";

/// Per-group workspace root under the agent workspace
/// (`/clawft/workspace/groups/{group_id}/`).
pub const GROUPS_REL: &str = "groups";

/// Per-group identity file (CLAUDE.md analogue for openbrowserclaw parity).
pub const GROUP_CLAWFT_MD: &str = "CLAWFT.md";

/// Absolute path to the persisted config snapshot.
pub fn config_persist_path() -> PathBuf {
    Path::new(BROWSER_HOME_DIR).join(CONFIG_PERSIST_REL)
}

/// Absolute sessions directory used by `LocalFileSink` on browser
/// (`/clawft/.clawft/workspace/sessions`).
pub fn sessions_dir() -> PathBuf {
    Path::new(BROWSER_HOME_DIR).join(SESSIONS_REL)
}

/// Absolute directory for one group: `/clawft/workspace/groups/{group_id}`.
///
/// Callers must pass a validated `group_id` (no separators / `..`).
pub fn group_dir(group_id: &str) -> PathBuf {
    Path::new(BROWSER_WORKSPACE_DIR)
        .join(GROUPS_REL)
        .join(group_id)
}

/// Absolute path to the group's CLAWFT.md (CLAUDE.md-per-group).
pub fn group_clawft_md_path(group_id: &str) -> PathBuf {
    group_dir(group_id).join(GROUP_CLAWFT_MD)
}

/// Canonical conversation id for a browser group (`web:{group_id}`).
///
/// Matches `InboundMessage::session_key()` when channel=`web` and
/// chat_id=`group_id`.
pub fn conv_id_for_group(group_id: &str) -> String {
    format!("{BROWSER_CHANNEL}:{group_id}")
}

/// Validate a group id for path safety (same rules as session keys,
/// but without requiring a colon — group ids are the chat_id half only).
///
/// Rejects empty, overlong, path separators, `..`, and control bytes.
pub fn validate_group_id(group_id: &str) -> Result<(), String> {
    const MAX: usize = 200;
    if group_id.is_empty() {
        return Err("group_id must not be empty".into());
    }
    if group_id.len() > MAX {
        return Err(format!(
            "group_id too long ({} bytes, max {MAX})",
            group_id.len()
        ));
    }
    if group_id.contains("..") {
        return Err("group_id contains path traversal sequence '..'".into());
    }
    if group_id.contains('/') || group_id.contains('\\') {
        return Err("group_id contains directory separator".into());
    }
    if group_id.contains('\0') {
        return Err("group_id contains null byte".into());
    }
    for byte in group_id.bytes() {
        if byte <= 0x1F && byte != 0x09 {
            return Err("group_id contains control character".into());
        }
    }
    Ok(())
}

/// Resolve group_id: `None` / empty → [`DEFAULT_GROUP_ID`].
pub fn resolve_group_id(group_id: Option<&str>) -> Result<String, String> {
    let id = group_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_GROUP_ID);
    validate_group_id(id)?;
    Ok(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_paths_are_absolute_under_clawft() {
        assert_eq!(
            config_persist_path(),
            PathBuf::from("/clawft/.clawft/config.json")
        );
        assert_eq!(
            sessions_dir(),
            PathBuf::from("/clawft/.clawft/workspace/sessions")
        );
        assert_eq!(
            group_dir("team-a"),
            PathBuf::from("/clawft/workspace/groups/team-a")
        );
        assert_eq!(
            group_clawft_md_path("team-a"),
            PathBuf::from("/clawft/workspace/groups/team-a/CLAWFT.md")
        );
        assert_eq!(conv_id_for_group("team-a"), "web:team-a");
        assert_eq!(conv_id_for_group(DEFAULT_GROUP_ID), "web:browser");
    }

    #[test]
    fn validate_group_id_rejects_traversal() {
        assert!(validate_group_id("../evil").is_err());
        assert!(validate_group_id("a/b").is_err());
        assert!(validate_group_id("").is_err());
        assert!(validate_group_id("ok-group_1").is_ok());
    }

    #[test]
    fn resolve_group_defaults_to_browser() {
        assert_eq!(resolve_group_id(None).unwrap(), "browser");
        assert_eq!(resolve_group_id(Some("")).unwrap(), "browser");
        assert_eq!(resolve_group_id(Some("  ")).unwrap(), "browser");
        assert_eq!(resolve_group_id(Some("g1")).unwrap(), "g1");
    }

    /// Hermetic mock: memory FS can host the WEFT-399 layout paths
    /// (no OPFS / Chrome). Persistence across *reopen* requires OPFS
    /// (`browser_history_persist` suite).
    #[tokio::test]
    async fn memory_fs_hosts_history_layout_paths() {
        use crate::browser::BrowserFileSystem;
        use crate::fs::FileSystem;

        let fs = BrowserFileSystem::memory();
        let sessions = sessions_dir();
        fs.create_dir_all(&sessions).await.unwrap();
        let jsonl = sessions.join("web%3Ateam.jsonl");
        let header = r#"{"_type":"metadata","created_at":"t","updated_at":"t","metadata":{},"last_consolidated":0}"#;
        let line = r#"{"role":"user","content":"hi","timestamp":"2020-01-01T00:00:00Z"}"#;
        fs.write_string(&jsonl, &format!("{header}\n{line}\n"))
            .await
            .unwrap();
        assert!(fs.exists(&jsonl).await);
        assert!(fs.read_to_string(&jsonl).await.unwrap().contains("hi"));

        let md = group_clawft_md_path("team");
        fs.write_string(&md, "# team\n").await.unwrap();
        assert_eq!(fs.read_to_string(&md).await.unwrap(), "# team\n");

        let cfg = config_persist_path();
        fs.write_string(&cfg, r#"{"ok":true}"#).await.unwrap();
        assert_eq!(fs.read_to_string(&cfg).await.unwrap(), r#"{"ok":true}"#);
    }
}
