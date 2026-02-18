//! Unified skill definition types.
//!
//! Skills are reusable LLM instruction bundles that can be loaded from two
//! formats:
//!
//! - **Legacy**: `skill.json` metadata + `prompt.md` instructions (per the
//!   original [`SkillsLoader`](crate) convention).
//! - **SKILL.md**: A single markdown file with YAML frontmatter containing
//!   metadata and the body containing LLM instructions.
//!
//! The [`SkillDefinition`] struct is the unified representation regardless
//! of source format.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How the skill was loaded.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillFormat {
    /// Legacy: `skill.json` + `prompt.md`.
    #[default]
    Legacy,
    /// `SKILL.md` with YAML frontmatter.
    SkillMd,
}

/// Unified skill definition.
///
/// Combines metadata (name, description, variables, tool permissions) with
/// the LLM instructions body. The [`format`](SkillDefinition::format) field
/// records which on-disk layout the skill was loaded from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Skill identifier (typically matches the directory name).
    pub name: String,

    /// Human-readable description shown in skill listings.
    pub description: String,

    /// Semantic version of the skill definition.
    #[serde(default)]
    pub version: String,

    /// Template variable names expected by the skill prompt.
    #[serde(default)]
    pub variables: Vec<String>,

    /// Hint text shown for the slash-command argument (e.g. `"PR URL or number"`).
    #[serde(default)]
    pub argument_hint: Option<String>,

    /// Tools the skill is allowed to use.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Whether end-users can invoke this skill directly (e.g. via `/skill`).
    #[serde(default)]
    pub user_invocable: bool,

    /// If true, the LLM cannot invoke this skill -- only humans can.
    #[serde(default)]
    pub disable_model_invocation: bool,

    /// The actual LLM instructions (markdown body).
    #[serde(skip)]
    pub instructions: String,

    /// Format this skill was loaded from.
    #[serde(skip)]
    pub format: SkillFormat,

    /// Source path on disk.
    #[serde(skip)]
    pub source_path: Option<PathBuf>,

    /// Additional metadata not captured by named fields.
    #[serde(default, flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SkillDefinition {
    /// Create a minimal skill definition (for testing or built-in skills).
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: String::new(),
            variables: Vec::new(),
            argument_hint: None,
            allowed_tools: Vec::new(),
            user_invocable: false,
            disable_model_invocation: false,
            instructions: String::new(),
            format: SkillFormat::default(),
            source_path: None,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_definition_new() {
        let skill = SkillDefinition::new("test", "A test skill");
        assert_eq!(skill.name, "test");
        assert_eq!(skill.description, "A test skill");
        assert!(skill.version.is_empty());
        assert!(skill.variables.is_empty());
        assert!(skill.instructions.is_empty());
        assert_eq!(skill.format, SkillFormat::Legacy);
        assert!(skill.source_path.is_none());
    }

    #[test]
    fn skill_format_default() {
        assert_eq!(SkillFormat::default(), SkillFormat::Legacy);
    }

    #[test]
    fn skill_definition_serde_roundtrip() {
        let mut skill = SkillDefinition::new("roundtrip", "Roundtrip test");
        skill.version = "2.0.0".into();
        skill.variables = vec!["var1".into(), "var2".into()];
        skill.user_invocable = true;
        skill.instructions = "These should be skipped".into();

        let json = serde_json::to_string(&skill).unwrap();
        let restored: SkillDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.name, "roundtrip");
        assert_eq!(restored.version, "2.0.0");
        assert_eq!(restored.variables, vec!["var1", "var2"]);
        assert!(restored.user_invocable);
        // instructions is #[serde(skip)] so it should be empty after deser
        assert!(restored.instructions.is_empty());
        // format is #[serde(skip)]
        assert_eq!(restored.format, SkillFormat::Legacy);
    }

    #[test]
    fn skill_definition_from_json_with_extras() {
        let json = r#"{
            "name": "extra",
            "description": "Has extra fields",
            "custom_field": "custom_value",
            "priority": 5
        }"#;
        let skill: SkillDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(skill.name, "extra");
        assert_eq!(
            skill.metadata.get("custom_field"),
            Some(&serde_json::json!("custom_value"))
        );
        assert_eq!(skill.metadata.get("priority"), Some(&serde_json::json!(5)));
    }
}
