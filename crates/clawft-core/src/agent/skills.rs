//! Skill discovery and loading.
//!
//! Scans a directory tree for skill definitions (each skill is a
//! subdirectory containing `skill.json` metadata and `prompt.md` LLM
//! instructions). Loaded skills are cached in an `RwLock` for concurrent
//! read access with infrequent writes.
//!
//! Directory layout:
//! ```text
//! skills/
//! +-- research/
//! |   +-- skill.json   {"name":"research","description":"...","variables":["topic"]}
//! |   +-- prompt.md    # LLM instructions text
//! +-- code_review/
//!     +-- skill.json
//!     +-- prompt.md
//! ```
//!
//! File locations follow the fallback chain:
//! `~/.clawft/workspace/skills/` then `~/.nanobot/workspace/skills/`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use clawft_platform::Platform;
use clawft_types::{ClawftError, Result};

/// A loaded skill definition.
///
/// Skills consist of JSON metadata (`skill.json`) and an optional LLM
/// prompt file (`prompt.md`). The prompt is loaded lazily on first use
/// and cached in the [`SkillsLoader`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill identifier (matches the directory name).
    pub name: String,

    /// Human-readable description for skill listing.
    pub description: String,

    /// Template variable names expected by the skill prompt.
    #[serde(default)]
    pub variables: Vec<String>,

    /// LLM instructions loaded from `prompt.md`. `None` until the
    /// prompt file has been read.
    #[serde(skip)]
    pub prompt: Option<String>,

    /// Semantic version of the skill definition.
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0.0".into()
}

/// Loads and caches skill definitions from a workspace directory.
///
/// The loader scans for skill subdirectories, parses their `skill.json`
/// metadata, and lazily loads `prompt.md` content on first use. All
/// filesystem access goes through the [`Platform`] abstraction.
///
/// # Concurrency
///
/// The skill cache is wrapped in [`RwLock`] for many-reader / rare-writer
/// access. [`list_skills`](SkillsLoader::list_skills) and
/// [`get_skill`](SkillsLoader::get_skill) acquire read locks;
/// [`load_skill`](SkillsLoader::load_skill) and
/// [`load_all`](SkillsLoader::load_all) acquire write locks.
pub struct SkillsLoader<P: Platform> {
    skills_dir: PathBuf,
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    platform: Arc<P>,
}

impl<P: Platform> SkillsLoader<P> {
    /// Create a new skills loader.
    ///
    /// Resolves the skills directory via the fallback chain:
    /// 1. `~/.clawft/workspace/skills/`
    /// 2. `~/.nanobot/workspace/skills/`
    ///
    /// If neither exists, the `.clawft` path is used.
    ///
    /// # Errors
    ///
    /// Returns [`ClawftError::ConfigInvalid`] if no home directory can
    /// be determined.
    pub fn new(platform: Arc<P>) -> Result<Self> {
        let home = platform.fs().home_dir().ok_or_else(|| {
            ClawftError::ConfigInvalid {
                reason: "could not determine home directory".into(),
            }
        })?;

        let clawft_skills = home.join(".clawft").join("workspace").join("skills");
        let nanobot_skills = home.join(".nanobot").join("workspace").join("skills");

        let skills_dir = if nanobot_skills.exists() && !clawft_skills.exists() {
            debug!(path = %nanobot_skills.display(), "using legacy nanobot skills path");
            nanobot_skills
        } else {
            debug!(path = %clawft_skills.display(), "using clawft skills path");
            clawft_skills
        };

        Ok(Self {
            skills_dir,
            skills: Arc::new(RwLock::new(HashMap::new())),
            platform,
        })
    }

    /// Create a skills loader with an explicit directory (for testing).
    #[cfg(test)]
    pub(crate) fn with_dir(skills_dir: PathBuf, platform: Arc<P>) -> Self {
        Self {
            skills_dir,
            skills: Arc::new(RwLock::new(HashMap::new())),
            platform,
        }
    }

    /// List available skill names by scanning subdirectories.
    ///
    /// A directory is considered a skill if it contains a `skill.json`
    /// file. Names are returned in filesystem order (not sorted).
    ///
    /// # Errors
    ///
    /// Returns an empty list if the skills directory does not exist.
    /// Propagates I/O errors for other failures.
    pub async fn list_skills(&self) -> Result<Vec<String>> {
        if !self.platform.fs().exists(&self.skills_dir).await {
            return Ok(Vec::new());
        }

        let entries = self
            .platform
            .fs()
            .list_dir(&self.skills_dir)
            .await
            .map_err(ClawftError::Io)?;

        let mut names = Vec::new();
        for entry in entries {
            let skill_json = entry.join("skill.json");
            if self.platform.fs().exists(&skill_json).await {
                if let Some(name) = entry.file_name() {
                    names.push(name.to_string_lossy().into_owned());
                }
            }
        }

        Ok(names)
    }

    /// Load a specific skill by name.
    ///
    /// Reads `skill.json` for metadata and `prompt.md` for the LLM
    /// prompt. The loaded skill is cached for future
    /// [`get_skill`](SkillsLoader::get_skill) calls.
    ///
    /// # Errors
    ///
    /// Returns [`ClawftError::PluginLoadFailed`] if the skill directory
    /// or `skill.json` cannot be read.
    pub async fn load_skill(&self, name: &str) -> Result<Skill> {
        let skill_dir = self.skills_dir.join(name);
        let skill_json_path = skill_dir.join("skill.json");
        let prompt_path = skill_dir.join("prompt.md");

        // Read and parse skill.json
        let json_content = self
            .platform
            .fs()
            .read_to_string(&skill_json_path)
            .await
            .map_err(|e| ClawftError::PluginLoadFailed {
                plugin: format!("skill/{name}: {e}"),
            })?;

        let mut skill: Skill =
            serde_json::from_str(&json_content).map_err(|e| ClawftError::PluginLoadFailed {
                plugin: format!("skill/{name}: invalid skill.json: {e}"),
            })?;

        // Load prompt.md if it exists
        if self.platform.fs().exists(&prompt_path).await {
            match self.platform.fs().read_to_string(&prompt_path).await {
                Ok(prompt) => {
                    skill.prompt = Some(prompt);
                }
                Err(e) => {
                    warn!(skill = name, error = %e, "failed to read prompt.md");
                }
            }
        }

        // Cache the loaded skill
        {
            let mut cache = self.skills.write().await;
            cache.insert(name.to_string(), skill.clone());
        }

        debug!(skill = name, "loaded skill");
        Ok(skill)
    }

    /// Get a cached skill by name.
    ///
    /// Returns `None` if the skill has not been loaded yet. Use
    /// [`load_skill`](SkillsLoader::load_skill) to load and cache a
    /// skill first.
    pub async fn get_skill(&self, name: &str) -> Option<Skill> {
        let cache = self.skills.read().await;
        cache.get(name).cloned()
    }

    /// Load all skills from the skills directory.
    ///
    /// Scans for skill subdirectories and loads each one. Errors on
    /// individual skills are logged as warnings but do not abort the
    /// overall load.
    pub async fn load_all(&self) -> Result<()> {
        let names = self.list_skills().await?;
        for name in &names {
            if let Err(e) = self.load_skill(name).await {
                warn!(skill = %name, error = %e, "failed to load skill, skipping");
            }
        }
        debug!(count = names.len(), "loaded all skills");
        Ok(())
    }

    /// Path to the skills directory (for diagnostics).
    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_platform::NativePlatform;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_skills_test_{prefix}_{pid}_{id}"))
    }

    fn test_loader(dir: &std::path::Path) -> SkillsLoader<NativePlatform> {
        let platform = Arc::new(NativePlatform::new());
        SkillsLoader::with_dir(dir.to_path_buf(), platform)
    }

    /// Create a skill directory with skill.json and optional prompt.md.
    async fn create_skill(dir: &std::path::Path, name: &str, desc: &str, prompt: Option<&str>) {
        let skill_dir = dir.join(name);
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();

        let json = serde_json::json!({
            "name": name,
            "description": desc,
            "variables": ["topic"],
        });
        tokio::fs::write(
            skill_dir.join("skill.json"),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .await
        .unwrap();

        if let Some(prompt_text) = prompt {
            tokio::fs::write(skill_dir.join("prompt.md"), prompt_text)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn list_skills_empty_when_dir_missing() {
        let dir = temp_dir("missing");
        let loader = test_loader(&dir);
        let skills = loader.list_skills().await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn list_skills_finds_skill_dirs() {
        let dir = temp_dir("list");
        create_skill(&dir, "research", "Deep research", Some("Research prompt")).await;
        create_skill(&dir, "code_review", "Code review", None).await;

        let loader = test_loader(&dir);
        let mut skills = loader.list_skills().await.unwrap();
        skills.sort();

        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0], "code_review");
        assert_eq!(skills[1], "research");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn list_skills_ignores_dirs_without_skill_json() {
        let dir = temp_dir("no_json");
        tokio::fs::create_dir_all(dir.join("not_a_skill"))
            .await
            .unwrap();
        tokio::fs::write(dir.join("not_a_skill").join("readme.md"), "hi")
            .await
            .unwrap();

        create_skill(&dir, "valid_skill", "Valid", Some("prompt")).await;

        let loader = test_loader(&dir);
        let skills = loader.list_skills().await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "valid_skill");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_skill_parses_metadata_and_prompt() {
        let dir = temp_dir("load");
        create_skill(
            &dir,
            "research",
            "Deep research on a topic",
            Some("You are a research assistant."),
        )
        .await;

        let loader = test_loader(&dir);
        let skill = loader.load_skill("research").await.unwrap();

        assert_eq!(skill.name, "research");
        assert_eq!(skill.description, "Deep research on a topic");
        assert_eq!(skill.variables, vec!["topic"]);
        assert_eq!(
            skill.prompt.as_deref(),
            Some("You are a research assistant.")
        );
        assert_eq!(skill.version, "1.0.0");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_skill_without_prompt() {
        let dir = temp_dir("no_prompt");
        create_skill(&dir, "basic", "A basic skill", None).await;

        let loader = test_loader(&dir);
        let skill = loader.load_skill("basic").await.unwrap();

        assert_eq!(skill.name, "basic");
        assert!(skill.prompt.is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_skill_caches_result() {
        let dir = temp_dir("cache");
        create_skill(&dir, "cached", "Cached skill", Some("cached prompt")).await;

        let loader = test_loader(&dir);

        // Not cached yet
        assert!(loader.get_skill("cached").await.is_none());

        // Load it
        loader.load_skill("cached").await.unwrap();

        // Now cached
        let cached = loader.get_skill("cached").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "cached");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_skill_nonexistent_returns_error() {
        let dir = temp_dir("nonexistent");
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let loader = test_loader(&dir);
        let result = loader.load_skill("does_not_exist").await;
        assert!(result.is_err());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_skill_invalid_json_returns_error() {
        let dir = temp_dir("bad_json");
        let skill_dir = dir.join("bad");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(skill_dir.join("skill.json"), "not valid json {{{")
            .await
            .unwrap();

        let loader = test_loader(&dir);
        let result = loader.load_skill("bad").await;
        assert!(result.is_err());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_all_loads_all_skills() {
        let dir = temp_dir("load_all");
        create_skill(&dir, "skill_a", "Skill A", Some("prompt a")).await;
        create_skill(&dir, "skill_b", "Skill B", Some("prompt b")).await;

        let loader = test_loader(&dir);
        loader.load_all().await.unwrap();

        assert!(loader.get_skill("skill_a").await.is_some());
        assert!(loader.get_skill("skill_b").await.is_some());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn load_all_skips_invalid_skills() {
        let dir = temp_dir("load_all_skip");
        create_skill(&dir, "good", "Good skill", Some("prompt")).await;

        // Create a bad skill
        let bad_dir = dir.join("bad");
        tokio::fs::create_dir_all(&bad_dir).await.unwrap();
        tokio::fs::write(bad_dir.join("skill.json"), "invalid")
            .await
            .unwrap();

        let loader = test_loader(&dir);
        // Should not return error -- bad skill is skipped
        loader.load_all().await.unwrap();

        assert!(loader.get_skill("good").await.is_some());
        assert!(loader.get_skill("bad").await.is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn skill_serde_roundtrip() {
        let skill = Skill {
            name: "test".into(),
            description: "Test skill".into(),
            variables: vec!["var1".into(), "var2".into()],
            prompt: Some("prompt text".into()),
            version: "2.0.0".into(),
        };

        let json = serde_json::to_string(&skill).unwrap();
        let restored: Skill = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "test");
        assert_eq!(restored.description, "Test skill");
        assert_eq!(restored.variables, vec!["var1", "var2"]);
        // prompt is #[serde(skip)] so it should be None after deserialization
        assert!(restored.prompt.is_none());
        assert_eq!(restored.version, "2.0.0");
    }

    #[tokio::test]
    async fn skill_json_with_version() {
        let dir = temp_dir("version");
        let skill_dir = dir.join("versioned");
        tokio::fs::create_dir_all(&skill_dir).await.unwrap();
        tokio::fs::write(
            skill_dir.join("skill.json"),
            r#"{"name":"versioned","description":"With version","variables":[],"version":"3.1.0"}"#,
        )
        .await
        .unwrap();

        let loader = test_loader(&dir);
        let skill = loader.load_skill("versioned").await.unwrap();
        assert_eq!(skill.version, "3.1.0");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn get_skill_returns_none_for_unloaded() {
        let dir = temp_dir("unloaded");
        let loader = test_loader(&dir);
        assert!(loader.get_skill("anything").await.is_none());
    }

    #[tokio::test]
    async fn new_resolves_home_dir() {
        let platform = Arc::new(NativePlatform::new());
        let loader = SkillsLoader::new(platform);
        assert!(loader.is_ok());
        let loader = loader.unwrap();
        assert!(loader.skills_dir().is_absolute());
    }
}
