//! Context builder for assembling LLM prompts.
//!
//! Combines the system prompt, active skill prompts, long-term memory,
//! and conversation history into a message list suitable for the LLM
//! pipeline. Ported from Python `nanobot/agent/context.py`.
//!
//! # Message assembly order
//!
//! 1. **System prompt** (role=`"system"`) -- identity and instructions
//! 2. **Active skill prompts** (role=`"system"`) -- prefixed with `# Skill: {name}`
//! 3. **Memory context** (role=`"system"`) -- prefixed with `# Relevant Memory:`
//! 4. **Conversation history** -- recent messages from the session
//!
//! The current user message is **not** added here; the caller appends it.

use std::sync::Arc;

use tracing::warn;

use clawft_platform::Platform;
use clawft_types::config::AgentsConfig;
use clawft_types::session::Session;

use super::memory::MemoryStore;
use super::skills::SkillsLoader;

// Re-export the message type from pipeline::traits. This module will
// reference the type via a `crate::pipeline::traits::LlmMessage` path.
// Until the pipeline agent writes that module, we define a local
// compatible type and re-export it.
//
// Once the pipeline module is written, this can be changed to:
//   use crate::pipeline::traits::LlmMessage;
//
// For now we define it inline so the module compiles independently.

/// An LLM message with role and content.
///
/// Compatible with the `LlmMessage` type defined in
/// `crate::pipeline::traits`. Once that module is available, this
/// definition should be replaced with a re-export.
#[derive(Debug, Clone)]
pub struct LlmMessage {
    /// Message role: `"system"`, `"user"`, `"assistant"`, or `"tool"`.
    pub role: String,
    /// Message text content.
    pub content: String,
    /// Optional tool call ID (for tool result messages).
    pub tool_call_id: Option<String>,
}

/// Builder for assembling LLM context from multiple sources.
///
/// Combines configuration, memory, skills, and session history into
/// a structured message list for the LLM pipeline. Each source is
/// read asynchronously through the platform abstraction.
///
/// # Usage
///
/// ```rust,ignore
/// let ctx = ContextBuilder::new(config, memory, skills, platform);
/// let system_prompt = ctx.build_system_prompt().await;
/// let messages = ctx.build_messages(&session, &["research"]).await;
/// ```
pub struct ContextBuilder<P: Platform> {
    config: AgentsConfig,
    memory: Arc<MemoryStore<P>>,
    skills: Arc<SkillsLoader<P>>,
    platform: Arc<P>,
}

impl<P: Platform> ContextBuilder<P> {
    /// Create a new context builder.
    ///
    /// # Arguments
    ///
    /// * `config` -- Agent configuration (system prompt, memory window, etc.)
    /// * `memory` -- Shared memory store for reading long-term memory
    /// * `skills` -- Shared skills loader for reading active skill prompts
    /// * `platform` -- Platform abstraction for filesystem/env access
    pub fn new(
        config: AgentsConfig,
        memory: Arc<MemoryStore<P>>,
        skills: Arc<SkillsLoader<P>>,
        platform: Arc<P>,
    ) -> Self {
        Self {
            config,
            memory,
            skills,
            platform,
        }
    }

    /// Build the system prompt from configuration.
    ///
    /// Assembles the core identity prompt. This includes:
    /// - A static identity header describing the assistant
    /// - Workspace path information
    /// - Memory file paths for the agent to reference
    ///
    /// The returned string is suitable for a `role="system"` message.
    pub async fn build_system_prompt(&self) -> String {
        let workspace = &self.config.defaults.workspace;
        let model = &self.config.defaults.model;

        let mut parts = Vec::new();

        parts.push(format!(
            "# clawft\n\n\
            You are clawft, a helpful AI assistant. You have access to tools that allow you to:\n\
            - Read, write, and edit files\n\
            - Execute shell commands\n\
            - Search the web and fetch web pages\n\
            - Send messages to users on chat channels\n\n\
            ## Configuration\n\
            Model: {model}\n\
            Workspace: {workspace}\n\
            Memory: {workspace}/memory/MEMORY.md\n\
            History: {workspace}/memory/HISTORY.md\n\
            Skills: {workspace}/skills/"
        ));

        // Load any bootstrap files from the workspace (AGENTS.md, SOUL.md, etc.)
        let bootstrap_files = ["AGENTS.md", "SOUL.md", "USER.md", "TOOLS.md", "IDENTITY.md"];
        for filename in &bootstrap_files {
            let home = self.platform.fs().home_dir();
            if let Some(home) = home {
                let ws_path = expand_workspace(workspace, &home);
                let file_path = ws_path.join(filename);
                if self.platform.fs().exists(&file_path).await {
                    match self.platform.fs().read_to_string(&file_path).await {
                        Ok(content) if !content.trim().is_empty() => {
                            parts.push(format!("## {filename}\n\n{content}"));
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!(file = %filename, error = %e, "failed to read bootstrap file");
                        }
                    }
                }
            }
        }

        parts.join("\n\n---\n\n")
    }

    /// Build the complete message list for an LLM call.
    ///
    /// Assembles messages in the canonical order:
    /// 1. System prompt (role=`"system"`)
    /// 2. Active skill prompts (role=`"system"`, one per skill)
    /// 3. Long-term memory context (role=`"system"`, if non-empty)
    /// 4. Conversation history from `session.get_history(memory_window)`
    ///
    /// The current user message is **not** included -- the caller adds
    /// it after calling this method.
    ///
    /// # Arguments
    ///
    /// * `session` -- Current conversation session
    /// * `active_skills` -- Names of skills to include in context
    pub async fn build_messages(
        &self,
        session: &Session,
        active_skills: &[String],
    ) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        // 1. System prompt
        let system_prompt = self.build_system_prompt().await;
        messages.push(LlmMessage {
            role: "system".into(),
            content: system_prompt,
            tool_call_id: None,
        });

        // 2. Active skill prompts
        for skill_name in active_skills {
            match self.skills.get_skill(skill_name).await {
                Some(skill) => {
                    if let Some(ref prompt) = skill.prompt {
                        messages.push(LlmMessage {
                            role: "system".into(),
                            content: format!("# Skill: {}\n\n{}", skill.name, prompt),
                            tool_call_id: None,
                        });
                    }
                }
                None => {
                    // Try loading the skill on demand
                    match self.skills.load_skill(skill_name).await {
                        Ok(skill) => {
                            if let Some(ref prompt) = skill.prompt {
                                messages.push(LlmMessage {
                                    role: "system".into(),
                                    content: format!("# Skill: {}\n\n{}", skill.name, prompt),
                                    tool_call_id: None,
                                });
                            }
                        }
                        Err(e) => {
                            warn!(
                                skill = %skill_name,
                                error = %e,
                                "failed to load skill for context"
                            );
                        }
                    }
                }
            }
        }

        // 3. Memory context
        match self.memory.read_long_term().await {
            Ok(memory) if !memory.trim().is_empty() => {
                messages.push(LlmMessage {
                    role: "system".into(),
                    content: format!("# Relevant Memory:\n\n{memory}"),
                    tool_call_id: None,
                });
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, "failed to read long-term memory for context");
            }
        }

        // 4. Conversation history (truncated to memory_window)
        let window = self.config.defaults.memory_window.max(0) as usize;
        let history = session.get_history(window);
        for msg in history {
            let role = msg
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();
            let content = msg
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            messages.push(LlmMessage {
                role,
                content,
                tool_call_id: None,
            });
        }

        messages
    }

    /// Get a reference to the agent configuration.
    pub fn config(&self) -> &AgentsConfig {
        &self.config
    }
}

/// Expand a workspace path, replacing `~/` with the actual home directory.
fn expand_workspace(workspace: &str, home: &std::path::Path) -> std::path::PathBuf {
    if let Some(rest) = workspace.strip_prefix("~/") {
        home.join(rest)
    } else {
        std::path::PathBuf::from(workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::memory::MemoryStore;
    use crate::agent::skills::SkillsLoader;
    use clawft_platform::NativePlatform;
    use clawft_types::config::AgentDefaults;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_ctx_test_{prefix}_{pid}_{id}"))
    }

    fn test_config() -> AgentsConfig {
        AgentsConfig {
            defaults: AgentDefaults {
                workspace: "~/.clawft/workspace".into(),
                model: "test-model/v1".into(),
                max_tokens: 4096,
                temperature: 0.5,
                max_tool_iterations: 10,
                memory_window: 5,
            },
        }
    }

    /// Helper to create a ContextBuilder backed by temp directories.
    async fn setup(
        prefix: &str,
    ) -> (
        ContextBuilder<NativePlatform>,
        PathBuf,
        Arc<MemoryStore<NativePlatform>>,
        Arc<SkillsLoader<NativePlatform>>,
    ) {
        let dir = temp_dir(prefix);
        let mem_dir = dir.join("memory");
        let skills_dir = dir.join("skills");

        let platform = Arc::new(NativePlatform::new());

        let memory = Arc::new(MemoryStore::with_paths(
            mem_dir.join("MEMORY.md"),
            mem_dir.join("HISTORY.md"),
            platform.clone(),
        ));

        let skills = Arc::new(SkillsLoader::with_dir(skills_dir.clone(), platform.clone()));

        let ctx = ContextBuilder::new(test_config(), memory.clone(), skills.clone(), platform);

        (ctx, dir, memory, skills)
    }

    #[tokio::test]
    async fn build_system_prompt_contains_identity() {
        let (ctx, dir, _, _) = setup("prompt").await;

        let prompt = ctx.build_system_prompt().await;

        assert!(prompt.contains("clawft"));
        assert!(prompt.contains("test-model/v1"));
        assert!(prompt.contains("MEMORY.md"));
        assert!(prompt.contains("HISTORY.md"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_includes_system_prompt() {
        let (ctx, dir, _, _) = setup("sys_msg").await;
        let session = Session::new("test:1");

        let messages = ctx.build_messages(&session, &[]).await;

        assert!(!messages.is_empty());
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("clawft"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_includes_memory() {
        let (ctx, dir, memory, _) = setup("memory").await;

        memory.write_long_term("important fact: Rust is fast").await.unwrap();

        let session = Session::new("test:2");
        let messages = ctx.build_messages(&session, &[]).await;

        // Should have system prompt + memory message
        let memory_msg = messages
            .iter()
            .find(|m| m.content.contains("Relevant Memory"));
        assert!(memory_msg.is_some());
        assert!(memory_msg.unwrap().content.contains("Rust is fast"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_skips_empty_memory() {
        let (ctx, dir, _, _) = setup("no_mem").await;
        let session = Session::new("test:3");

        let messages = ctx.build_messages(&session, &[]).await;

        // No "Relevant Memory" message when memory is empty
        let memory_msg = messages
            .iter()
            .find(|m| m.content.contains("Relevant Memory"));
        assert!(memory_msg.is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_includes_skill_prompts() {
        let (ctx, dir, _, skills) = setup("skills").await;

        // Create a skill on disk and load it
        let skills_dir = dir.join("skills");
        tokio::fs::create_dir_all(skills_dir.join("research"))
            .await
            .unwrap();
        tokio::fs::write(
            skills_dir.join("research").join("skill.json"),
            r#"{"name":"research","description":"Research skill","variables":["topic"]}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            skills_dir.join("research").join("prompt.md"),
            "You are a research expert.",
        )
        .await
        .unwrap();

        // Load it into cache
        skills.load_skill("research").await.unwrap();

        let session = Session::new("test:4");
        let messages = ctx
            .build_messages(&session, &["research".into()])
            .await;

        let skill_msg = messages
            .iter()
            .find(|m| m.content.contains("# Skill: research"));
        assert!(skill_msg.is_some());
        assert!(skill_msg.unwrap().content.contains("research expert"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_includes_conversation_history() {
        let (ctx, dir, _, _) = setup("history").await;

        let mut session = Session::new("test:5");
        session.add_message("user", "What is Rust?", None);
        session.add_message("assistant", "Rust is a systems language.", None);
        session.add_message("user", "Tell me more.", None);

        let messages = ctx.build_messages(&session, &[]).await;

        // Should have system prompt + 3 history messages
        // (memory_window=5, so all 3 fit)
        let history_roles: Vec<&str> = messages
            .iter()
            .skip(1) // skip system prompt
            .map(|m| m.role.as_str())
            .collect();

        assert_eq!(history_roles, vec!["user", "assistant", "user"]);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_truncates_history_to_window() {
        let (ctx, dir, _, _) = setup("truncate").await;

        let mut session = Session::new("test:6");
        for i in 0..20 {
            session.add_message("user", &format!("message {i}"), None);
        }

        let messages = ctx.build_messages(&session, &[]).await;

        // memory_window=5, so only last 5 history messages
        let history_msgs: Vec<&LlmMessage> =
            messages.iter().filter(|m| m.role == "user").collect();
        assert_eq!(history_msgs.len(), 5);
        assert!(history_msgs[0].content.contains("message 15"));
        assert!(history_msgs[4].content.contains("message 19"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_order_is_correct() {
        let (ctx, dir, memory, skills) = setup("order").await;

        // Set up memory
        memory.write_long_term("a long-term fact").await.unwrap();

        // Set up a skill
        let skills_dir = dir.join("skills");
        tokio::fs::create_dir_all(skills_dir.join("test_skill"))
            .await
            .unwrap();
        tokio::fs::write(
            skills_dir.join("test_skill").join("skill.json"),
            r#"{"name":"test_skill","description":"Test","variables":[]}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            skills_dir.join("test_skill").join("prompt.md"),
            "skill prompt content",
        )
        .await
        .unwrap();
        skills.load_skill("test_skill").await.unwrap();

        // Set up session
        let mut session = Session::new("test:7");
        session.add_message("user", "hello", None);

        let messages = ctx
            .build_messages(&session, &["test_skill".into()])
            .await;

        // Order: system, skill, memory, history
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("clawft"));
        assert_eq!(messages[1].role, "system");
        assert!(messages[1].content.contains("# Skill: test_skill"));
        assert_eq!(messages[2].role, "system");
        assert!(messages[2].content.contains("Relevant Memory"));
        assert_eq!(messages[3].role, "user");
        assert_eq!(messages[3].content, "hello");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_loads_uncached_skill_on_demand() {
        let (ctx, dir, _, _) = setup("on_demand").await;

        // Create skill on disk but do NOT pre-load it
        let skills_dir = dir.join("skills");
        tokio::fs::create_dir_all(skills_dir.join("lazy_skill"))
            .await
            .unwrap();
        tokio::fs::write(
            skills_dir.join("lazy_skill").join("skill.json"),
            r#"{"name":"lazy_skill","description":"Lazy","variables":[]}"#,
        )
        .await
        .unwrap();
        tokio::fs::write(
            skills_dir.join("lazy_skill").join("prompt.md"),
            "loaded on demand",
        )
        .await
        .unwrap();

        let session = Session::new("test:8");
        let messages = ctx
            .build_messages(&session, &["lazy_skill".into()])
            .await;

        let skill_msg = messages
            .iter()
            .find(|m| m.content.contains("# Skill: lazy_skill"));
        assert!(skill_msg.is_some());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn build_messages_handles_missing_skill_gracefully() {
        let (ctx, dir, _, _) = setup("missing_skill").await;
        let session = Session::new("test:9");

        // Request a skill that does not exist
        let messages = ctx
            .build_messages(&session, &["nonexistent".into()])
            .await;

        // Should just have system prompt, no error
        assert!(!messages.is_empty());
        assert_eq!(messages[0].role, "system");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[test]
    fn expand_workspace_with_tilde() {
        let home = PathBuf::from("/home/testuser");
        let expanded = expand_workspace("~/.clawft/workspace", &home);
        assert_eq!(expanded, PathBuf::from("/home/testuser/.clawft/workspace"));
    }

    #[test]
    fn expand_workspace_absolute() {
        let home = PathBuf::from("/home/testuser");
        let expanded = expand_workspace("/opt/workspace", &home);
        assert_eq!(expanded, PathBuf::from("/opt/workspace"));
    }

    #[test]
    fn config_accessor() {
        let config = test_config();
        let platform = Arc::new(NativePlatform::new());
        let memory = Arc::new(MemoryStore::with_paths(
            PathBuf::from("/tmp/m.md"),
            PathBuf::from("/tmp/h.md"),
            platform.clone(),
        ));
        let skills = Arc::new(SkillsLoader::with_dir(
            PathBuf::from("/tmp/skills"),
            platform.clone(),
        ));
        let ctx = ContextBuilder::new(config.clone(), memory, skills, platform);
        assert_eq!(ctx.config().defaults.memory_window, 5);
    }

    #[tokio::test]
    async fn llm_message_has_expected_fields() {
        let msg = LlmMessage {
            role: "system".into(),
            content: "test content".into(),
            tool_call_id: Some("tc-1".into()),
        };
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "test content");
        assert_eq!(msg.tool_call_id.as_deref(), Some("tc-1"));
    }
}
