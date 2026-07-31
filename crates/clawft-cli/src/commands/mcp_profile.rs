//! Serve profiles for `weft mcp-server` (ADR-076 C1 / WEFT-699).
//!
//! Profiles compose via comma-separated CLI tokens:
//! `default` | `control` | `workspace` | `media` | `full`.
//!
//! Product default when the flag is omitted is **`default`**
//! (`control` ∪ `workspace`) — **not** a full agent-tool dump.

use std::collections::HashSet;

use clawft_services::mcp::ToolDefinition;

/// Named serve profile (atomic unit; compose with commas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServeProfile {
    /// Instance ops: status, agents, windows, skill_list/get (when present).
    Control,
    /// Sandboxed FS, shell, file memory, web, process spawn.
    Workspace,
    /// voice_*, audio_*, render_ui (session-bound).
    Media,
    /// Entire registry + skill expansion + optional MCP re-export.
    Full,
}

impl ServeProfile {
    fn parse_atomic(s: &str) -> Result<Self, String> {
        match s {
            "control" => Ok(Self::Control),
            "workspace" => Ok(Self::Workspace),
            "media" => Ok(Self::Media),
            "full" => Ok(Self::Full),
            other => Err(format!(
                "unknown profile '{other}'; expected default, control, workspace, media, full"
            )),
        }
    }
}

/// Resolved profile set used for filtering tools at serve time.
#[derive(Debug, Clone)]
pub struct ProfileSet {
    profiles: HashSet<ServeProfile>,
    /// Original CLI string (for logs).
    raw: String,
}

impl ProfileSet {
    /// Parse a comma-separated profile string.
    ///
    /// `default` expands to `control` ∪ `workspace`. Empty / whitespace-only
    /// input also yields the product default.
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut profiles = HashSet::new();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            profiles.insert(ServeProfile::Control);
            profiles.insert(ServeProfile::Workspace);
            return Ok(Self {
                profiles,
                raw: "default".into(),
            });
        }

        for part in trimmed.split(',') {
            let part = part.trim().to_ascii_lowercase();
            if part.is_empty() {
                continue;
            }
            if part == "default" {
                profiles.insert(ServeProfile::Control);
                profiles.insert(ServeProfile::Workspace);
            } else {
                profiles.insert(ServeProfile::parse_atomic(&part)?);
            }
        }

        if profiles.is_empty() {
            profiles.insert(ServeProfile::Control);
            profiles.insert(ServeProfile::Workspace);
        }

        Ok(Self {
            profiles,
            raw: trimmed.to_string(),
        })
    }

    /// Product default: control ∪ workspace.
    #[allow(dead_code)] // used by unit tests; available for callers/logging
    pub fn product_default() -> Self {
        Self::parse("default").expect("default profile must parse")
    }

    /// Original flag value / resolved label for logging.
    pub fn label(&self) -> &str {
        &self.raw
    }

    /// Whether the set includes the `full` profile.
    pub fn includes_full(&self) -> bool {
        self.profiles.contains(&ServeProfile::Full)
    }

    /// Skills registered as one MCP tool per skill — only for `full`.
    pub fn allows_skill_tools(&self) -> bool {
        self.includes_full()
    }

    /// Proxied external MCP tools (`internal_only: false`) — only for `full`.
    pub fn allows_mcp_reexport(&self) -> bool {
        self.includes_full()
    }

    /// Whether a tool name is allowed under this profile set.
    pub fn allows_tool(&self, name: &str) -> bool {
        if self.includes_full() {
            return true;
        }
        self.profiles.iter().any(|p| tool_in_profile(name, *p))
    }
}

fn tool_in_profile(name: &str, profile: ServeProfile) -> bool {
    match profile {
        ServeProfile::Full => true,
        ServeProfile::Workspace => is_workspace_tool(name),
        ServeProfile::Control => is_control_tool(name),
        ServeProfile::Media => is_media_tool(name),
    }
}

/// Workspace profile tool names (catalog + current wire aliases).
fn is_workspace_tool(name: &str) -> bool {
    matches!(
        name,
        "read_file"
            | "write_file"
            | "edit_file"
            | "list_directory"
            | "exec_shell"
            | "memory_read"
            | "memory_write"
            | "web_search"
            | "web_fetch"
            // Public wire: `process_spawn` (ADR-076 C2). Bare `spawn` kept in
            // the allow set for internal registry names before wire rename.
            | "spawn"
            | "process_spawn"
    )
}

/// Control profile: planned façades + any agent_/window_ tools present.
fn is_control_tool(name: &str) -> bool {
    matches!(
        name,
        "status"
            | "skill_list"
            | "skill_get"
            | "mcp_list"
            | "substrate_read"
            | "substrate_list"
            | "task_status"
            | "task_result"
    ) || name.starts_with("agent_")
        || name.starts_with("window_")
}

fn is_media_tool(name: &str) -> bool {
    name == "render_ui" || name.starts_with("voice_") || name.starts_with("audio_")
}

/// Filter tool definitions by profile membership (pure; unit-tested).
pub fn filter_tools_by_profile(
    tools: Vec<ToolDefinition>,
    profiles: &ProfileSet,
) -> Vec<ToolDefinition> {
    tools
        .into_iter()
        .filter(|t| profiles.allows_tool(&t.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(tools: &[ToolDefinition]) -> Vec<&str> {
        tools.iter().map(|t| t.name.as_str()).collect()
    }

    fn def(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.into(),
            description: String::new(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    fn sample_tools() -> Vec<ToolDefinition> {
        [
            "read_file",
            "write_file",
            "exec_shell",
            "spawn",
            "web_search",
            "voice_listen",
            "voice_speak",
            "audio_transcribe",
            "render_ui",
            "delegate_task",
            "agent_spawn",
            "status",
            "skill_get",
            "claude-flow__swarm_init",
        ]
        .into_iter()
        .map(def)
        .collect()
    }

    #[test]
    fn parse_default_is_control_union_workspace() {
        let p = ProfileSet::parse("default").unwrap();
        assert!(p.profiles.contains(&ServeProfile::Control));
        assert!(p.profiles.contains(&ServeProfile::Workspace));
        assert!(!p.includes_full());
        assert!(!p.allows_skill_tools());
        assert!(!p.allows_mcp_reexport());
    }

    #[test]
    fn parse_empty_equals_default() {
        let p = ProfileSet::parse("").unwrap();
        assert!(p.allows_tool("read_file"));
        assert!(!p.allows_tool("voice_listen"));
        assert!(!p.includes_full());
    }

    #[test]
    fn parse_comma_composition() {
        let p = ProfileSet::parse("control,media").unwrap();
        assert!(p.allows_tool("status"));
        assert!(p.allows_tool("agent_list"));
        assert!(p.allows_tool("voice_listen"));
        assert!(!p.allows_tool("read_file"));
        assert!(!p.allows_tool("delegate_task"));
    }

    #[test]
    fn parse_full() {
        let p = ProfileSet::parse("full").unwrap();
        assert!(p.includes_full());
        assert!(p.allows_skill_tools());
        assert!(p.allows_mcp_reexport());
        assert!(p.allows_tool("delegate_task"));
        assert!(p.allows_tool("claude-flow__swarm_init"));
    }

    #[test]
    fn parse_unknown_errors() {
        let err = ProfileSet::parse("nope").unwrap_err();
        assert!(err.contains("unknown profile"));
    }

    #[test]
    fn default_excludes_media_and_full_only_tools() {
        let p = ProfileSet::product_default();
        let filtered = filter_tools_by_profile(sample_tools(), &p);
        let n = names(&filtered);

        assert!(n.contains(&"read_file"));
        assert!(n.contains(&"write_file"));
        assert!(n.contains(&"exec_shell"));
        assert!(n.contains(&"spawn")); // registry name; wire renames to process_spawn
        assert!(n.contains(&"web_search"));
        assert!(n.contains(&"agent_spawn"));
        assert!(n.contains(&"status"));
        assert!(n.contains(&"skill_get"));

        assert!(!n.contains(&"voice_listen"));
        assert!(!n.contains(&"voice_speak"));
        assert!(!n.contains(&"audio_transcribe"));
        assert!(!n.contains(&"render_ui"));
        assert!(!n.contains(&"delegate_task"));
        assert!(!n.contains(&"claude-flow__swarm_init"));
    }

    #[test]
    fn default_allows_skill_list_and_get_not_media() {
        let p = ProfileSet::product_default();
        assert!(p.allows_tool("skill_list"));
        assert!(p.allows_tool("skill_get"));
        assert!(!p.allows_skill_tools());
        assert!(!p.allows_mcp_reexport());
    }

    #[test]
    fn default_is_not_full() {
        let default = filter_tools_by_profile(sample_tools(), &ProfileSet::product_default());
        let full = filter_tools_by_profile(sample_tools(), &ProfileSet::parse("full").unwrap());
        assert!(default.len() < full.len());
        assert_eq!(full.len(), sample_tools().len());
    }

    #[test]
    fn workspace_only() {
        let p = ProfileSet::parse("workspace").unwrap();
        let filtered = filter_tools_by_profile(sample_tools(), &p);
        let n = names(&filtered);
        assert!(n.contains(&"read_file"));
        assert!(n.contains(&"spawn"));
        assert!(!n.contains(&"agent_spawn"));
        assert!(!n.contains(&"voice_listen"));
        assert!(!n.contains(&"status"));
    }

    #[test]
    fn control_only_excludes_workspace_and_media() {
        let p = ProfileSet::parse("control").unwrap();
        let filtered = filter_tools_by_profile(sample_tools(), &p);
        let n = names(&filtered);
        assert!(n.contains(&"agent_spawn"));
        assert!(n.contains(&"status"));
        assert!(n.contains(&"skill_get"));
        assert!(!n.contains(&"read_file"));
        assert!(!n.contains(&"voice_listen"));
    }

    #[test]
    fn media_only() {
        let p = ProfileSet::parse("media").unwrap();
        let filtered = filter_tools_by_profile(sample_tools(), &p);
        let n = names(&filtered);
        assert!(n.contains(&"voice_listen"));
        assert!(n.contains(&"render_ui"));
        assert!(!n.contains(&"read_file"));
        assert!(!n.contains(&"delegate_task"));
    }

    #[test]
    fn process_spawn_alias_is_workspace() {
        let p = ProfileSet::parse("workspace").unwrap();
        assert!(p.allows_tool("process_spawn"));
        assert!(p.allows_tool("spawn"));
    }
}
