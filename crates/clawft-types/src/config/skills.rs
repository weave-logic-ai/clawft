//! Skill discovery, autonomous generation, and auto-promotion settings.
//!
//! Schema (under `~/.clawft/config.json`):
//!
//! ```json
//! {
//!   "skills": {
//!     "autogen": {
//!       "enabled": false,
//!       "threshold": 3,
//!       "max_pending": 10
//!     },
//!     "promotion": {
//!       "enabled": true,
//!       "threshold": 10
//!     }
//!   }
//! }
//! ```
//!
//! Autogen is **opt-in**: defaults keep pattern detection off. Toggle via
//! `weft skills autogen enable|disable|status`.
//!
//! Promotion (WEFT-348 / Phase 4) is **on by default** with a threshold of
//! 10 successful invocations: skills used from `.claude/skills/` that cross
//! the threshold are copied into `.clawft/skills/` for faster routing.

use serde::{Deserialize, Serialize};

/// Top-level skills configuration section.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillsConfig {
    /// Autonomous skill creation (pattern detection → pending SKILL.md).
    #[serde(default)]
    pub autogen: SkillAutogenConfig,

    /// Auto-promotion of frequently used `.claude/skills/*` into
    /// `.clawft/skills/` (WEFT-348).
    #[serde(default)]
    pub promotion: SkillPromotionConfig,
}

/// Configuration for autonomous skill creation (`skills.autogen`).
///
/// Mirrors `clawft_core::agent::skill_autogen::AutogenConfig` fields that
/// operators may set in JSON. `install_dir` remains an implementation detail
/// of the detector (defaults to `~/.clawft/skills/`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillAutogenConfig {
    /// Whether autonomous skill creation is enabled.
    /// Default: `false` (disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Minimum pattern repetitions before suggesting a skill.
    /// Default: 3.
    #[serde(default = "default_threshold")]
    pub threshold: usize,

    /// Maximum pending auto-generated skills at once.
    /// Default: 10.
    #[serde(default = "default_max_pending", alias = "maxPending")]
    pub max_pending: usize,
}

fn default_threshold() -> usize {
    3
}

fn default_max_pending() -> usize {
    10
}

impl Default for SkillAutogenConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: default_threshold(),
            max_pending: default_max_pending(),
        }
    }
}

/// Configuration for Phase 4 skill auto-promotion (`skills.promotion`).
///
/// After a skill sourced from `.claude/skills/*` records at least
/// [`threshold`](Self::threshold) **successful** invocations, it is copied
/// into `.clawft/skills/` so the always-loaded floor picks it up without
/// waiting on the context router candidate path (WEFT-348 / chat-agent-v1
/// Phase 4).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPromotionConfig {
    /// Whether auto-promotion is active.
    /// Default: `true`.
    #[serde(default = "default_promotion_enabled")]
    pub enabled: bool,

    /// Successful invocations required before promoting a skill.
    /// Default: `10`.
    #[serde(default = "default_promotion_threshold")]
    pub threshold: usize,
}

fn default_promotion_enabled() -> bool {
    true
}

fn default_promotion_threshold() -> usize {
    10
}

impl Default for SkillPromotionConfig {
    fn default() -> Self {
        Self {
            enabled: default_promotion_enabled(),
            threshold: default_promotion_threshold(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_disabled() {
        let cfg = SkillAutogenConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.threshold, 3);
        assert_eq!(cfg.max_pending, 10);
    }

    #[test]
    fn empty_object_parses_to_defaults() {
        let cfg: SkillAutogenConfig = serde_json::from_str("{}").unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.threshold, 3);
        assert_eq!(cfg.max_pending, 10);
    }

    #[test]
    fn skills_section_parses_snake_and_camel() {
        let snake = r#"{
            "autogen": {
                "enabled": true,
                "threshold": 5,
                "max_pending": 7
            }
        }"#;
        let cfg: SkillsConfig = serde_json::from_str(snake).unwrap();
        assert!(cfg.autogen.enabled);
        assert_eq!(cfg.autogen.threshold, 5);
        assert_eq!(cfg.autogen.max_pending, 7);
        // Promotion defaults when omitted.
        assert!(cfg.promotion.enabled);
        assert_eq!(cfg.promotion.threshold, 10);

        let camel = r#"{
            "autogen": {
                "enabled": true,
                "threshold": 4,
                "maxPending": 2
            }
        }"#;
        let cfg: SkillsConfig = serde_json::from_str(camel).unwrap();
        assert!(cfg.autogen.enabled);
        assert_eq!(cfg.autogen.threshold, 4);
        assert_eq!(cfg.autogen.max_pending, 2);
    }

    #[test]
    fn promotion_section_parses() {
        let json = r#"{
            "promotion": {
                "enabled": false,
                "threshold": 25
            }
        }"#;
        let cfg: SkillsConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.promotion.enabled);
        assert_eq!(cfg.promotion.threshold, 25);
        assert!(!cfg.autogen.enabled);
    }

    #[test]
    fn promotion_empty_object_uses_defaults() {
        let cfg: SkillPromotionConfig = serde_json::from_str("{}").unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.threshold, 10);
    }

    #[test]
    fn serde_roundtrip() {
        let original = SkillsConfig {
            autogen: SkillAutogenConfig {
                enabled: true,
                threshold: 6,
                max_pending: 12,
            },
            promotion: SkillPromotionConfig {
                enabled: true,
                threshold: 15,
            },
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: SkillsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn missing_skills_key_defaults() {
        let cfg: SkillsConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg, SkillsConfig::default());
        assert!(cfg.promotion.enabled);
        assert_eq!(cfg.promotion.threshold, 10);
    }
}
