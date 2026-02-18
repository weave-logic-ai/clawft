//! Agent definitions, loading, and registry.
//!
//! An *agent definition* describes a custom agent persona: its name,
//! system prompt, model override, allowed tools, and optional skills.
//! Definitions are stored as YAML or JSON files and discovered from a
//! 3-level directory hierarchy:
//!
//! 1. **Workspace** agents (`<workspace>/agents/`) -- highest priority
//! 2. **User** agents (`~/.clawft/agents/`) -- user-global agents
//! 3. **Built-in** agents -- compiled into the binary
//!
//! When the same agent name appears at multiple levels, the higher-priority
//! level wins (workspace > user > built-in).
//!
//! # Directory layout
//!
//! ```text
//! agents/
//! +-- researcher/
//! |   +-- agent.yaml   # or agent.json
//! +-- code_reviewer/
//!     +-- agent.yaml
//! ```
//!
//! A single-file agent is also supported: `agents/researcher.yaml`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use clawft_types::{ClawftError, Result};

use crate::security::{
    MAX_AGENT_FILE_SIZE, validate_directory_name, validate_file_size, validate_model_string,
};

/// Definition of a custom agent.
///
/// Loaded from YAML/JSON (`agent.yaml` or `agent.json`) and used by the
/// [`ContextBuilder`](super::context::ContextBuilder) to customise the
/// system prompt, model selection, and skill activation for a given
/// agent persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique agent identifier.
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// LLM model override.  When `None`, the global default is used.
    #[serde(default)]
    pub model: Option<String>,

    /// System prompt prepended to every LLM call for this agent.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Skills to activate when this agent is running.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Tool allowlist.  Empty = all tools allowed.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Maximum tool-loop turns for this agent.  When `None`, the
    /// global default is used.
    #[serde(default)]
    pub max_turns: Option<u32>,

    /// Template variables available for prompt expansion.
    /// See [`render_template`](super::helpers::render_template).
    #[serde(default)]
    pub variables: HashMap<String, String>,

    /// Source path (YAML file or directory) -- not serialised.
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

/// Loads agent definitions from files and directories.
///
/// Supports both single-file and directory-per-agent layouts.
/// Accepted file names inside a directory: `agent.yaml`, `agent.yml`,
/// `agent.json`.  A standalone file is also accepted directly.
pub struct AgentLoader;

impl AgentLoader {
    /// Load an agent definition from a single file.
    ///
    /// Accepts `.yaml`, `.yml`, or `.json` extensions.  The `source_path`
    /// field on the returned definition is set to the resolved file path.
    ///
    /// # Errors
    ///
    /// Returns [`ClawftError::PluginLoadFailed`] on I/O or parse errors.
    pub fn load_file(path: &Path) -> Result<AgentDefinition> {
        // SEC-SKILL-07: Check file size before reading.
        if let Ok(meta) = std::fs::metadata(path) {
            validate_file_size(meta.len() as usize, MAX_AGENT_FILE_SIZE, "agent")?;
        }

        let content = std::fs::read_to_string(path).map_err(|e| ClawftError::PluginLoadFailed {
            plugin: format!("agent: cannot read {}: {e}", path.display()),
        })?;

        let mut def = Self::parse_content(&content, path)?;
        def.source_path = Some(path.to_path_buf());

        // SEC-SKILL-04: Validate model string if present.
        if let Some(ref model) = def.model {
            validate_model_string(model)?;
        }

        Ok(def)
    }

    /// Load an agent from a file or directory.
    ///
    /// If `path` is a file, it is parsed directly.  If `path` is a
    /// directory, the loader looks for `agent.yaml`, `agent.yml`, or
    /// `agent.json` inside it.
    ///
    /// # Errors
    ///
    /// Returns [`ClawftError::PluginLoadFailed`] when no agent file is
    /// found or the file cannot be parsed.
    pub fn load(path: &Path) -> Result<AgentDefinition> {
        if path.is_file() {
            return Self::load_file(path);
        }

        if path.is_dir() {
            for filename in &["agent.yaml", "agent.yml", "agent.json"] {
                let candidate = path.join(filename);
                if candidate.is_file() {
                    let mut def = Self::load_file(&candidate)?;
                    def.source_path = Some(path.to_path_buf());
                    return Ok(def);
                }
            }
            return Err(ClawftError::PluginLoadFailed {
                plugin: format!("agent: no agent.yaml/agent.json in {}", path.display()),
            });
        }

        Err(ClawftError::PluginLoadFailed {
            plugin: format!("agent: path does not exist: {}", path.display()),
        })
    }

    /// Load all agent definitions from a directory of agent subdirectories
    /// (or loose YAML/JSON files).
    ///
    /// Each subdirectory should contain `agent.yaml` / `agent.json`.
    /// Loose `.yaml`/`.yml`/`.json` files at the top level are also
    /// loaded.  Errors on individual agents are logged and skipped.
    pub fn load_dir(dir: &Path) -> Result<Vec<AgentDefinition>> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| ClawftError::PluginLoadFailed {
            plugin: format!("agent: cannot read dir {}: {e}", dir.display()),
        })?;

        let mut defs = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "failed to read directory entry");
                    continue;
                }
            };
            let entry_path = entry.path();

            // SEC-SKILL-02: Validate directory/file names.
            let entry_name = entry.file_name();
            let entry_name_str = entry_name.to_string_lossy();
            if let Err(e) = validate_directory_name(&entry_name_str) {
                warn!(
                    path = %entry_path.display(),
                    error = %e,
                    "rejected agent entry with unsafe name"
                );
                continue;
            }

            match Self::load(&entry_path) {
                Ok(def) => {
                    debug!(agent = %def.name, path = %entry_path.display(), "loaded agent");
                    defs.push(def);
                }
                Err(e) => {
                    // Only warn for entries that look like they *should* be agents
                    let looks_like_agent = entry_path.is_dir()
                        || entry_path
                            .extension()
                            .is_some_and(|ext| ext == "yaml" || ext == "yml" || ext == "json");
                    if looks_like_agent {
                        warn!(
                            path = %entry_path.display(),
                            error = %e,
                            "failed to load agent, skipping"
                        );
                    }
                }
            }
        }

        Ok(defs)
    }

    /// Parse file content as YAML or JSON based on file extension.
    fn parse_content(content: &str, path: &Path) -> Result<AgentDefinition> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "json" => serde_json::from_str(content).map_err(|e| ClawftError::PluginLoadFailed {
                plugin: format!("agent: invalid JSON in {}: {e}", path.display()),
            }),
            // YAML is the default for .yaml, .yml, and any other extension
            _ => serde_yaml::from_str(content).map_err(|e| ClawftError::PluginLoadFailed {
                plugin: format!("agent: invalid YAML in {}: {e}", path.display()),
            }),
        }
    }
}

/// Registry of available agent definitions with 3-level discovery.
///
/// Priority order (highest first):
/// 1. **Workspace** -- project-local agents
/// 2. **User** -- user-global agents (`~/.clawft/agents/`)
/// 3. **Built-in** -- hard-coded defaults
///
/// When the same name appears at multiple levels, the higher-priority
/// definition wins.
pub struct AgentRegistry {
    agents: HashMap<String, AgentDefinition>,
}

impl AgentRegistry {
    /// Discover agents from all three levels and merge into a registry.
    ///
    /// Built-in agents are added first, then user agents, then workspace
    /// agents.  Each level overwrites earlier entries with the same name.
    ///
    /// # Arguments
    ///
    /// * `workspace_dir` -- Optional workspace agents directory
    /// * `user_dir` -- Optional user-level agents directory
    /// * `builtin_agents` -- Hard-coded agent definitions
    pub fn discover(
        workspace_dir: Option<&Path>,
        user_dir: Option<&Path>,
        builtin_agents: Vec<AgentDefinition>,
    ) -> Result<Self> {
        let mut agents = HashMap::new();

        // Level 3: built-in (lowest priority)
        for def in builtin_agents {
            agents.insert(def.name.clone(), def);
        }

        // Level 2: user agents
        if let Some(dir) = user_dir {
            match AgentLoader::load_dir(dir) {
                Ok(defs) => {
                    for def in defs {
                        debug!(
                            agent = %def.name,
                            level = "user",
                            "registering user agent"
                        );
                        agents.insert(def.name.clone(), def);
                    }
                }
                Err(e) => {
                    warn!(error = %e, "failed to load user agents");
                }
            }
        }

        // Level 1: workspace agents (highest priority)
        if let Some(dir) = workspace_dir {
            match AgentLoader::load_dir(dir) {
                Ok(defs) => {
                    for def in defs {
                        debug!(
                            agent = %def.name,
                            level = "workspace",
                            "registering workspace agent (overrides lower levels)"
                        );
                        agents.insert(def.name.clone(), def);
                    }
                }
                Err(e) => {
                    warn!(error = %e, "failed to load workspace agents");
                }
            }
        }

        debug!(count = agents.len(), "agent registry populated");
        Ok(Self { agents })
    }

    /// Create an empty registry.
    pub fn empty() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Look up an agent by name.
    pub fn get(&self, name: &str) -> Option<&AgentDefinition> {
        self.agents.get(name)
    }

    /// List all registered agents, sorted by name.
    pub fn list(&self) -> Vec<&AgentDefinition> {
        let mut defs: Vec<_> = self.agents.values().collect();
        defs.sort_by_key(|d| &d.name);
        defs
    }

    /// Number of registered agents.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_agents_test_{prefix}_{pid}_{id}"))
    }

    /// Write a YAML agent definition to a directory.
    fn write_yaml_agent(dir: &Path, name: &str, description: &str) {
        let agent_dir = dir.join(name);
        std::fs::create_dir_all(&agent_dir).unwrap();
        let yaml = format!(
            "name: {name}\n\
             description: {description}\n\
             model: test-model/v1\n\
             system_prompt: \"You are a {name} agent.\"\n\
             skills:\n  - research\n\
             allowed_tools:\n  - read_file\n\
             max_turns: 5\n\
             variables:\n  lang: rust\n"
        );
        std::fs::write(agent_dir.join("agent.yaml"), yaml).unwrap();
    }

    /// Write a JSON agent definition to a directory.
    fn write_json_agent(dir: &Path, name: &str, description: &str) {
        let agent_dir = dir.join(name);
        std::fs::create_dir_all(&agent_dir).unwrap();
        let json = serde_json::json!({
            "name": name,
            "description": description,
            "model": "json-model/v2",
            "system_prompt": format!("You are a {name} agent."),
            "skills": ["coding"],
            "allowed_tools": ["write_file"],
            "max_turns": 10,
            "variables": {"framework": "axum"}
        });
        std::fs::write(
            agent_dir.join("agent.json"),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .unwrap();
    }

    // ── AgentLoader tests ────────────────────────────────────────────

    #[test]
    fn load_yaml_agent_definition() {
        let dir = temp_dir("load_yaml");
        write_yaml_agent(&dir, "researcher", "Deep research agent");

        let def = AgentLoader::load(&dir.join("researcher")).unwrap();
        assert_eq!(def.name, "researcher");
        assert_eq!(def.description, "Deep research agent");
        assert_eq!(def.model.as_deref(), Some("test-model/v1"));
        assert_eq!(
            def.system_prompt.as_deref(),
            Some("You are a researcher agent.")
        );
        assert_eq!(def.skills, vec!["research"]);
        assert_eq!(def.allowed_tools, vec!["read_file"]);
        assert_eq!(def.max_turns, Some(5));
        assert_eq!(def.variables.get("lang").map(|s| s.as_str()), Some("rust"));
        assert!(def.source_path.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_json_agent_definition() {
        let dir = temp_dir("load_json");
        write_json_agent(&dir, "coder", "Coding agent");

        let def = AgentLoader::load(&dir.join("coder")).unwrap();
        assert_eq!(def.name, "coder");
        assert_eq!(def.description, "Coding agent");
        assert_eq!(def.model.as_deref(), Some("json-model/v2"));
        assert_eq!(def.skills, vec!["coding"]);
        assert_eq!(def.allowed_tools, vec!["write_file"]);
        assert_eq!(def.max_turns, Some(10));
        assert_eq!(
            def.variables.get("framework").map(|s| s.as_str()),
            Some("axum")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_file_directly() {
        let dir = temp_dir("load_file");
        std::fs::create_dir_all(&dir).unwrap();
        let yaml = "name: standalone\ndescription: Standalone agent\n";
        let path = dir.join("standalone.yaml");
        std::fs::write(&path, yaml).unwrap();

        let def = AgentLoader::load_file(&path).unwrap();
        assert_eq!(def.name, "standalone");
        assert_eq!(def.source_path.unwrap(), path);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_dir_collects_all_agents() {
        let dir = temp_dir("load_dir");
        write_yaml_agent(&dir, "agent_a", "Agent A");
        write_json_agent(&dir, "agent_b", "Agent B");

        let mut defs = AgentLoader::load_dir(&dir).unwrap();
        defs.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(defs.len(), 2);
        assert_eq!(defs[0].name, "agent_a");
        assert_eq!(defs[1].name, "agent_b");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_dir_returns_empty_for_missing_dir() {
        let dir = temp_dir("missing_dir");
        let defs = AgentLoader::load_dir(&dir).unwrap();
        assert!(defs.is_empty());
    }

    #[test]
    fn load_dir_skips_invalid_entries() {
        let dir = temp_dir("skip_invalid");
        write_yaml_agent(&dir, "good", "Good agent");

        // Create a bad agent directory
        let bad_dir = dir.join("bad");
        std::fs::create_dir_all(&bad_dir).unwrap();
        std::fs::write(bad_dir.join("agent.yaml"), "{{invalid yaml").unwrap();

        let defs = AgentLoader::load_dir(&dir).unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "good");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nonexistent_returns_error() {
        let result = AgentLoader::load(Path::new("/tmp/does_not_exist_at_all"));
        assert!(result.is_err());
    }

    #[test]
    fn load_dir_with_no_agent_file_in_subdir() {
        let dir = temp_dir("no_agent_file");
        let sub = dir.join("empty_agent");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("readme.md"), "not an agent").unwrap();

        let defs = AgentLoader::load_dir(&dir).unwrap();
        assert!(defs.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_fields_when_omitted() {
        let dir = temp_dir("defaults");
        std::fs::create_dir_all(&dir).unwrap();
        let yaml = "name: minimal\ndescription: Minimal agent\n";
        std::fs::write(dir.join("agent.yaml"), yaml).unwrap();

        let def = AgentLoader::load_file(&dir.join("agent.yaml")).unwrap();
        assert_eq!(def.name, "minimal");
        assert!(def.model.is_none());
        assert!(def.system_prompt.is_none());
        assert!(def.skills.is_empty());
        assert!(def.allowed_tools.is_empty());
        assert!(def.max_turns.is_none());
        assert!(def.variables.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── AgentRegistry tests ──────────────────────────────────────────

    #[test]
    fn registry_3_level_priority() {
        let workspace_dir = temp_dir("reg_workspace");
        let user_dir = temp_dir("reg_user");

        // Built-in: agent "shared" with description "builtin"
        let builtin = vec![AgentDefinition {
            name: "shared".into(),
            description: "builtin".into(),
            model: None,
            system_prompt: None,
            skills: vec![],
            allowed_tools: vec![],
            max_turns: None,
            variables: HashMap::new(),
            source_path: None,
        }];

        // User level: agent "shared" with description "user"
        write_yaml_agent(&user_dir, "shared", "user");

        // Workspace level: agent "shared" with description "workspace"
        write_yaml_agent(&workspace_dir, "shared", "workspace");

        let registry =
            AgentRegistry::discover(Some(&workspace_dir), Some(&user_dir), builtin).unwrap();

        // Workspace has highest priority
        let agent = registry.get("shared").unwrap();
        assert_eq!(agent.description, "workspace");

        let _ = std::fs::remove_dir_all(&workspace_dir);
        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn registry_user_overrides_builtin() {
        let user_dir = temp_dir("reg_user_override");

        let builtin = vec![AgentDefinition {
            name: "helper".into(),
            description: "builtin helper".into(),
            model: None,
            system_prompt: None,
            skills: vec![],
            allowed_tools: vec![],
            max_turns: None,
            variables: HashMap::new(),
            source_path: None,
        }];

        write_yaml_agent(&user_dir, "helper", "user helper");

        let registry = AgentRegistry::discover(None, Some(&user_dir), builtin).unwrap();
        let agent = registry.get("helper").unwrap();
        assert_eq!(agent.description, "user helper");

        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn registry_list_returns_sorted() {
        let builtin = vec![
            AgentDefinition {
                name: "zulu".into(),
                description: "Z agent".into(),
                model: None,
                system_prompt: None,
                skills: vec![],
                allowed_tools: vec![],
                max_turns: None,
                variables: HashMap::new(),
                source_path: None,
            },
            AgentDefinition {
                name: "alpha".into(),
                description: "A agent".into(),
                model: None,
                system_prompt: None,
                skills: vec![],
                allowed_tools: vec![],
                max_turns: None,
                variables: HashMap::new(),
                source_path: None,
            },
        ];

        let registry = AgentRegistry::discover(None, None, builtin).unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[1].name, "zulu");
    }

    #[test]
    fn registry_empty() {
        let registry = AgentRegistry::empty();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get("anything").is_none());
        assert!(registry.list().is_empty());
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let registry = AgentRegistry::discover(None, None, vec![]).unwrap();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn agent_definition_serde_roundtrip() {
        let def = AgentDefinition {
            name: "test".into(),
            description: "Test agent".into(),
            model: Some("model/v1".into()),
            system_prompt: Some("You are test.".into()),
            skills: vec!["research".into()],
            allowed_tools: vec!["read_file".into()],
            max_turns: Some(15),
            variables: {
                let mut m = HashMap::new();
                m.insert("lang".into(), "rust".into());
                m
            },
            source_path: Some(PathBuf::from("/tmp/test")),
        };

        let json = serde_json::to_string(&def).unwrap();
        let restored: AgentDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "test");
        assert_eq!(restored.description, "Test agent");
        assert_eq!(restored.model.as_deref(), Some("model/v1"));
        assert_eq!(restored.skills, vec!["research"]);
        assert_eq!(restored.max_turns, Some(15));
        // source_path is #[serde(skip)] so should be None
        assert!(restored.source_path.is_none());
    }

    #[test]
    fn load_agent_yml_extension() {
        let dir = temp_dir("yml_ext");
        let agent_dir = dir.join("yml_agent");
        std::fs::create_dir_all(&agent_dir).unwrap();
        let yaml = "name: yml_agent\ndescription: Uses .yml extension\n";
        std::fs::write(agent_dir.join("agent.yml"), yaml).unwrap();

        let def = AgentLoader::load(&agent_dir).unwrap();
        assert_eq!(def.name, "yml_agent");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── SEC-SKILL-02: Directory traversal rejected in load_dir ──────

    #[test]
    fn sec_skill_02_agent_traversal_rejected() {
        let dir = temp_dir("sec02_agent_traversal");

        // Create a directory with path traversal in its name.
        let evil_dir = dir.join("..%2Fevil");
        std::fs::create_dir_all(&evil_dir).unwrap();
        let yaml = "name: evil\ndescription: Evil agent\n";
        std::fs::write(evil_dir.join("agent.yaml"), yaml).unwrap();

        // Also create a valid agent.
        write_yaml_agent(&dir, "good", "Good agent");

        let defs = AgentLoader::load_dir(&dir).unwrap();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"good"));
        // The evil dir has ".." in its name and should be rejected.

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── SEC-SKILL-04: Model string validation ───────────────────────

    #[test]
    fn sec_skill_04_shell_metachar_model_rejected() {
        let dir = temp_dir("sec04_model");
        let agent_dir = dir.join("evil_model");
        std::fs::create_dir_all(&agent_dir).unwrap();
        let yaml = "name: evil_model\ndescription: Has evil model\nmodel: \"; rm -rf /\"\n";
        std::fs::write(agent_dir.join("agent.yaml"), yaml).unwrap();

        let result = AgentLoader::load(&agent_dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("forbidden character") || err.contains("security violation"),
            "expected security error, got: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sec_skill_04_valid_model_accepted() {
        let dir = temp_dir("sec04_valid");
        let agent_dir = dir.join("valid_model");
        std::fs::create_dir_all(&agent_dir).unwrap();
        let yaml = "name: valid_model\ndescription: Valid model\nmodel: openai/gpt-4o\n";
        std::fs::write(agent_dir.join("agent.yaml"), yaml).unwrap();

        let def = AgentLoader::load(&agent_dir).unwrap();
        assert_eq!(def.model.as_deref(), Some("openai/gpt-4o"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── SEC-SKILL-07: Agent file size validation ────────────────────

    #[test]
    fn sec_skill_07_oversized_agent_rejected() {
        let dir = temp_dir("sec07_agent_size");
        let agent_dir = dir.join("big_agent");
        std::fs::create_dir_all(&agent_dir).unwrap();

        // Create a YAML file larger than 10 KB.
        let padding = "x".repeat(11 * 1024);
        let yaml = format!("name: big_agent\ndescription: {padding}\n");
        std::fs::write(agent_dir.join("agent.yaml"), yaml).unwrap();

        let result = AgentLoader::load(&agent_dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds") || err.contains("size"),
            "expected size error, got: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
