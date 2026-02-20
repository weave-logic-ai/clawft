//! Community features for ClawHub: stars, comments, reviews.

use serde::{Deserialize, Serialize};

/// A star (like/favorite) on a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStar {
    /// Skill identifier.
    pub skill_id: String,
    /// User who starred.
    pub user_id: String,
    /// Timestamp.
    pub starred_at: String,
}

/// A comment/review on a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillComment {
    /// Comment identifier.
    pub id: String,
    /// Skill identifier.
    pub skill_id: String,
    /// Author user ID.
    pub author_id: String,
    /// Comment body.
    pub body: String,
    /// Rating (1-5 stars, optional).
    pub rating: Option<u8>,
    /// Timestamp.
    pub created_at: String,
    /// Whether this comment has been moderated.
    pub moderated: bool,
}

/// Version entry for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    /// Version string (semver).
    pub version: String,
    /// Content hash for this version.
    pub content_hash: String,
    /// Publication timestamp.
    pub published_at: String,
    /// Whether this version is yanked.
    pub yanked: bool,
    /// Changelog entry.
    pub changelog: Option<String>,
}

/// In-memory community data store (stub for development).
///
/// In production, this would be backed by a database.
#[derive(Debug, Default)]
pub struct CommunityStore {
    stars: Vec<SkillStar>,
    comments: Vec<SkillComment>,
    versions: Vec<SkillVersion>,
}

impl CommunityStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a star to a skill.
    pub fn add_star(&mut self, skill_id: &str, user_id: &str) -> SkillStar {
        let star = SkillStar {
            skill_id: skill_id.into(),
            user_id: user_id.into(),
            starred_at: chrono::Utc::now().to_rfc3339(),
        };
        self.stars.push(star.clone());
        star
    }

    /// Remove a star from a skill.
    pub fn remove_star(&mut self, skill_id: &str, user_id: &str) -> bool {
        let before = self.stars.len();
        self.stars
            .retain(|s| !(s.skill_id == skill_id && s.user_id == user_id));
        self.stars.len() < before
    }

    /// Get star count for a skill.
    pub fn star_count(&self, skill_id: &str) -> usize {
        self.stars.iter().filter(|s| s.skill_id == skill_id).count()
    }

    /// Add a comment to a skill.
    pub fn add_comment(
        &mut self,
        skill_id: &str,
        author_id: &str,
        body: &str,
        rating: Option<u8>,
    ) -> SkillComment {
        let comment = SkillComment {
            id: format!("comment-{}", self.comments.len() + 1),
            skill_id: skill_id.into(),
            author_id: author_id.into(),
            body: body.into(),
            rating: rating.map(|r| r.clamp(1, 5)),
            created_at: chrono::Utc::now().to_rfc3339(),
            moderated: false,
        };
        self.comments.push(comment.clone());
        comment
    }

    /// Get comments for a skill.
    pub fn comments_for(&self, skill_id: &str) -> Vec<&SkillComment> {
        self.comments
            .iter()
            .filter(|c| c.skill_id == skill_id)
            .collect()
    }

    /// Register a new version of a skill.
    pub fn add_version(
        &mut self,
        version: &str,
        content_hash: &str,
        changelog: Option<&str>,
    ) -> SkillVersion {
        let v = SkillVersion {
            version: version.into(),
            content_hash: content_hash.into(),
            published_at: chrono::Utc::now().to_rfc3339(),
            yanked: false,
            changelog: changelog.map(String::from),
        };
        self.versions.push(v.clone());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_add_and_count() {
        let mut store = CommunityStore::new();
        store.add_star("skill-1", "user-a");
        store.add_star("skill-1", "user-b");
        assert_eq!(store.star_count("skill-1"), 2);
        assert_eq!(store.star_count("skill-2"), 0);
    }

    #[test]
    fn star_remove() {
        let mut store = CommunityStore::new();
        store.add_star("skill-1", "user-a");
        assert!(store.remove_star("skill-1", "user-a"));
        assert_eq!(store.star_count("skill-1"), 0);
        assert!(!store.remove_star("skill-1", "user-a")); // already removed
    }

    #[test]
    fn comment_add_and_retrieve() {
        let mut store = CommunityStore::new();
        store.add_comment("skill-1", "user-a", "Great skill!", Some(5));
        store.add_comment("skill-1", "user-b", "Good but needs docs", Some(3));
        let comments = store.comments_for("skill-1");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].rating, Some(5));
    }

    #[test]
    fn rating_clamped() {
        let mut store = CommunityStore::new();
        let c = store.add_comment("skill-1", "user-a", "Test", Some(10));
        assert_eq!(c.rating, Some(5)); // clamped to max 5
    }

    #[test]
    fn version_registration() {
        let mut store = CommunityStore::new();
        let v = store.add_version("1.0.0", "abc123", Some("Initial release"));
        assert_eq!(v.version, "1.0.0");
        assert!(!v.yanked);
        assert_eq!(v.changelog.as_deref(), Some("Initial release"));
    }
}
