//! `weft skills` -- CLI commands for skill discovery and management.
//!
//! Provides subcommands:
//!
//! - `weft skills list` -- list all skills (workspace, user, builtin) with
//!   source annotation.
//! - `weft skills show <name>` -- show skill details (description, variables,
//!   instructions preview).
//! - `weft skills install <path>` -- copy a skill to the user skills dir.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use comfy_table::{Table, presets};

use clawft_core::agent::skills_v2::SkillRegistry;
use clawft_types::skill::{SkillDefinition, SkillFormat};

/// Arguments for the `weft skills` subcommand.
#[derive(Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub action: SkillsAction,
}

/// Subcommands for `weft skills`.
#[derive(Subcommand)]
pub enum SkillsAction {
    /// List all skills with source annotation.
    List,

    /// Show details of a specific skill.
    Show {
        /// Skill name to inspect.
        name: String,
    },

    /// Install a skill from a local path.
    Install {
        /// Path to a skill directory (containing SKILL.md or skill.json).
        path: String,
    },

    /// Remove a user-installed skill.
    Remove {
        /// Skill name to remove from ~/.clawft/skills/.
        name: String,
    },
}

/// Run the skills subcommand.
pub async fn run(args: SkillsArgs) -> anyhow::Result<()> {
    let (ws_dir, user_dir) = discover_skill_dirs();
    let registry =
        SkillRegistry::discover(ws_dir.as_deref(), user_dir.as_deref(), Vec::new())
            .await
            .map_err(|e| anyhow::anyhow!("failed to discover skills: {e}"))?;

    match args.action {
        SkillsAction::List => skills_list(&registry, ws_dir.as_deref(), user_dir.as_deref()),
        SkillsAction::Show { name } => skills_show(&registry, &name),
        SkillsAction::Install { path } => skills_install(&path, user_dir.as_deref()),
        SkillsAction::Remove { name } => skills_remove(&name, user_dir.as_deref()),
    }
}

/// Discover workspace and user skill directories.
fn discover_skill_dirs() -> (Option<PathBuf>, Option<PathBuf>) {
    let user_dir = dirs::home_dir().map(|h| h.join(".clawft").join("skills"));

    // Walk upward from cwd to find .clawft/skills/
    let ws_dir = std::env::current_dir().ok().and_then(|cwd| {
        let mut dir: &Path = cwd.as_path();
        loop {
            let candidate = dir.join(".clawft").join("skills");
            if candidate.is_dir() {
                return Some(candidate);
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => return None,
            }
        }
    });

    (ws_dir, user_dir)
}

/// List all skills in a table with source annotation.
fn skills_list(
    registry: &SkillRegistry,
    ws_dir: Option<&Path>,
    user_dir: Option<&Path>,
) -> anyhow::Result<()> {
    let skills = registry.list();

    if skills.is_empty() {
        println!("No skills found.");
        println!();
        if let Some(dir) = user_dir {
            println!("User skills directory: {}", dir.display());
        }
        if let Some(dir) = ws_dir {
            println!("Workspace skills directory: {}", dir.display());
        }
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL_CONDENSED);
    table.set_header(["NAME", "SOURCE", "FORMAT", "DESCRIPTION"]);

    // Sort by name for deterministic output.
    let mut sorted: Vec<&SkillDefinition> = skills;
    sorted.sort_by_key(|s| &s.name);

    for skill in sorted {
        let source = classify_source(skill, ws_dir, user_dir);
        let format = match skill.format {
            SkillFormat::SkillMd => "SKILL.md",
            SkillFormat::Legacy => "legacy",
        };
        let desc = truncate(&skill.description, 50);
        table.add_row([&skill.name, source, format, &desc]);
    }

    println!("{table}");
    println!();
    println!("Total: {} skill(s)", registry.len());

    Ok(())
}

/// Show details of a specific skill.
fn skills_show(registry: &SkillRegistry, name: &str) -> anyhow::Result<()> {
    let skill = registry.get(name).ok_or_else(|| {
        anyhow::anyhow!("skill not found: {name}\nUse 'weft skills list' to see available skills.")
    })?;

    println!("Skill: {}", skill.name);
    println!("Description: {}", skill.description);

    if !skill.version.is_empty() {
        println!("Version: {}", skill.version);
    }

    println!(
        "Format: {}",
        match skill.format {
            SkillFormat::SkillMd => "SKILL.md",
            SkillFormat::Legacy => "legacy (skill.json)",
        }
    );

    if let Some(ref path) = skill.source_path {
        println!("Source: {}", path.display());
    }

    println!("User-invocable: {}", skill.user_invocable);

    if !skill.variables.is_empty() {
        println!("Variables: {}", skill.variables.join(", "));
    }

    if let Some(ref hint) = skill.argument_hint {
        println!("Argument hint: {hint}");
    }

    if !skill.allowed_tools.is_empty() {
        println!("Allowed tools: {}", skill.allowed_tools.join(", "));
    }

    if !skill.metadata.is_empty() {
        println!("Metadata:");
        for (key, value) in &skill.metadata {
            println!("  {key}: {value}");
        }
    }

    if !skill.instructions.is_empty() {
        println!();
        println!("Instructions (preview):");
        println!("---");
        // Show first 500 characters of instructions.
        let preview = truncate(&skill.instructions, 500);
        println!("{preview}");
        if skill.instructions.len() > 500 {
            println!("... ({} chars total)", skill.instructions.len());
        }
        println!("---");
    }

    Ok(())
}

/// Install a skill from a local path to the user skills directory.
fn skills_install(source_path: &str, user_dir: Option<&Path>) -> anyhow::Result<()> {
    let user_dir = user_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "cannot determine user skills directory (no home directory). \
             Set $HOME or use an explicit path."
        )
    })?;

    let source = PathBuf::from(source_path);
    if !source.exists() {
        anyhow::bail!("source path does not exist: {source_path}");
    }

    // Determine skill name from source directory name.
    let skill_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("cannot determine skill name from path: {source_path}"))?;

    let dest = user_dir.join(skill_name);

    // Ensure user skills directory exists.
    std::fs::create_dir_all(user_dir)
        .map_err(|e| anyhow::anyhow!("failed to create user skills directory: {e}"))?;

    if dest.exists() {
        anyhow::bail!(
            "skill '{skill_name}' already exists at {}. Remove it first.",
            dest.display()
        );
    }

    // Copy directory recursively.
    copy_dir_recursive(&source, &dest).map_err(|e| anyhow::anyhow!("failed to copy skill: {e}"))?;

    println!("Installed skill '{skill_name}' to {}", dest.display());

    Ok(())
}

/// Remove a user-installed skill from `~/.clawft/skills/<name>/`.
///
/// Only removes skills from the user directory. Workspace and built-in
/// skills cannot be removed via this command.
fn skills_remove(name: &str, user_dir: Option<&Path>) -> anyhow::Result<()> {
    let user_dir = user_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "cannot determine user skills directory (no home directory). \
             Set $HOME or use an explicit path."
        )
    })?;

    let skill_path = user_dir.join(name);

    if !skill_path.exists() {
        anyhow::bail!(
            "skill '{name}' not found in user skills directory ({}). \
             Only user-installed skills can be removed.",
            user_dir.display()
        );
    }

    std::fs::remove_dir_all(&skill_path)
        .map_err(|e| anyhow::anyhow!("failed to remove skill '{name}': {e}"))?;

    println!("Removed skill '{name}' from {}", skill_path.display());

    Ok(())
}

/// Classify the source of a skill for display.
fn classify_source(
    skill: &SkillDefinition,
    ws_dir: Option<&Path>,
    user_dir: Option<&Path>,
) -> &'static str {
    if let Some(ref path) = skill.source_path {
        if let Some(ws) = ws_dir
            && path.starts_with(ws)
        {
            return "workspace";
        }
        if let Some(ud) = user_dir
            && path.starts_with(ud)
        {
            return "user";
        }
    }
    "builtin"
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_skills_cmd_{prefix}_{pid}_{id}"))
    }

    fn create_skill_md(dir: &Path, name: &str, desc: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let content =
            format!("---\nname: {name}\ndescription: {desc}\n---\n\nInstructions for {name}.");
        std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let result = truncate("this is a long string that should be truncated", 20);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 20);
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn classify_source_workspace() {
        let ws = PathBuf::from("/project/.clawft/skills");
        let skill = SkillDefinition {
            source_path: Some(PathBuf::from("/project/.clawft/skills/research/SKILL.md")),
            ..SkillDefinition::new("research", "desc")
        };
        assert_eq!(classify_source(&skill, Some(&ws), None), "workspace");
    }

    #[test]
    fn classify_source_user() {
        let user = PathBuf::from("/home/user/.clawft/skills");
        let skill = SkillDefinition {
            source_path: Some(PathBuf::from("/home/user/.clawft/skills/coding/SKILL.md")),
            ..SkillDefinition::new("coding", "desc")
        };
        assert_eq!(classify_source(&skill, None, Some(&user)), "user");
    }

    #[test]
    fn classify_source_builtin() {
        let skill = SkillDefinition::new("builtin", "desc");
        assert_eq!(classify_source(&skill, None, None), "builtin");
    }

    #[tokio::test]
    async fn skills_list_with_registry() {
        let dir = temp_dir("list");
        create_skill_md(&dir, "alpha", "Alpha skill");
        create_skill_md(&dir, "beta", "Beta skill");

        let registry = SkillRegistry::discover(Some(&dir), None, Vec::new())
            .await
            .unwrap();
        // Just verify it does not panic and the registry has the skills.
        assert_eq!(registry.len(), 2);
        assert!(registry.get("alpha").is_some());
        assert!(registry.get("beta").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn skills_show_found() {
        let dir = temp_dir("show");
        create_skill_md(&dir, "test_skill", "A test skill");

        let registry = SkillRegistry::discover(Some(&dir), None, Vec::new())
            .await
            .unwrap();
        let skill = registry.get("test_skill");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().description, "A test skill");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn skills_show_not_found() {
        let registry = SkillRegistry::discover(None, None, Vec::new())
            .await
            .unwrap();
        let result = skills_show(&registry, "nonexistent");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn skills_install_source_not_found() {
        let user_dir = temp_dir("install_user");
        std::fs::create_dir_all(&user_dir).unwrap();

        let result = skills_install("/nonexistent/path", Some(&user_dir));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("does not exist"));

        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn skills_install_no_user_dir() {
        let result = skills_install("/some/path", None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("cannot determine"));
    }

    #[test]
    fn skills_install_copies_directory() {
        let src = temp_dir("install_src");
        let user_dir = temp_dir("install_user_dir");

        create_skill_md(&src, "installable", "To be installed");

        let result = skills_install(src.join("installable").to_str().unwrap(), Some(&user_dir));
        assert!(result.is_ok());

        let installed = user_dir.join("installable").join("SKILL.md");
        assert!(installed.exists());

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn skills_install_already_exists() {
        let src = temp_dir("install_exists_src");
        let user_dir = temp_dir("install_exists_user");

        create_skill_md(&src, "dupe", "Original");
        create_skill_md(&user_dir, "dupe", "Existing");

        let result = skills_install(src.join("dupe").to_str().unwrap(), Some(&user_dir));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("already exists"));

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn skills_remove_success() {
        let user_dir = temp_dir("remove_user");
        create_skill_md(&user_dir, "removable", "To be removed");

        assert!(user_dir.join("removable").exists());

        let result = skills_remove("removable", Some(&user_dir));
        assert!(result.is_ok());
        assert!(!user_dir.join("removable").exists());

        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn skills_remove_not_found() {
        let user_dir = temp_dir("remove_not_found");
        std::fs::create_dir_all(&user_dir).unwrap();

        let result = skills_remove("nonexistent", Some(&user_dir));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"));

        let _ = std::fs::remove_dir_all(&user_dir);
    }

    #[test]
    fn skills_remove_no_user_dir() {
        let result = skills_remove("anything", None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("cannot determine"));
    }

    #[test]
    fn copy_dir_recursive_works() {
        let src = temp_dir("copy_src");
        let dst = temp_dir("copy_dst");

        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("file.txt"), "hello").unwrap();
        std::fs::write(src.join("sub").join("nested.txt"), "nested").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("file.txt").exists());
        assert!(dst.join("sub").join("nested.txt").exists());
        assert_eq!(
            std::fs::read_to_string(dst.join("file.txt")).unwrap(),
            "hello"
        );

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }
}
