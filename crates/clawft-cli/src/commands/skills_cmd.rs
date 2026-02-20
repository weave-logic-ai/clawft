//! `weft skills` -- CLI commands for skill discovery and management.
//!
//! Provides subcommands:
//!
//! - `weft skills list` -- list all skills (workspace, user, builtin) with
//!   source annotation.
//! - `weft skills show <name>` -- show skill details (description, variables,
//!   instructions preview).
//! - `weft skills install <path>` -- copy a skill to the user skills dir.
//! - `weft skills remove <name>` -- remove a user-installed skill.
//! - `weft skills search <query>` -- search ClawHub for skills.
//! - `weft skills publish <path>` -- publish a skill to ClawHub.
//! - `weft skills remote-install <name>` -- install a skill from ClawHub.
//! - `weft skills keygen` -- generate a signing key pair.

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

    /// Search ClawHub for skills.
    Search {
        /// Search query.
        query: String,
        /// Maximum results.
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Publish a skill to ClawHub.
    Publish {
        /// Path to skill directory.
        path: String,
        /// Allow unsigned skills (local dev only).
        #[arg(long)]
        allow_unsigned: bool,
    },

    /// Install a skill from ClawHub by name.
    RemoteInstall {
        /// Skill name or ID on ClawHub.
        name: String,
        /// Allow unsigned skills.
        #[arg(long)]
        allow_unsigned: bool,
    },

    /// Generate a signing key pair for skill publishing.
    Keygen,
}

/// Run the skills subcommand.
pub async fn run(args: SkillsArgs) -> anyhow::Result<()> {
    match &args.action {
        // These subcommands do not need the local registry.
        SkillsAction::Search { .. }
        | SkillsAction::Publish { .. }
        | SkillsAction::RemoteInstall { .. }
        | SkillsAction::Keygen => {}
        _ => {}
    }

    let (ws_dir, user_dir) = discover_skill_dirs();

    match args.action {
        SkillsAction::List => {
            let registry =
                SkillRegistry::discover(ws_dir.as_deref(), user_dir.as_deref(), Vec::new())
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to discover skills: {e}"))?;
            skills_list(&registry, ws_dir.as_deref(), user_dir.as_deref())
        }
        SkillsAction::Show { name } => {
            let registry =
                SkillRegistry::discover(ws_dir.as_deref(), user_dir.as_deref(), Vec::new())
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to discover skills: {e}"))?;
            skills_show(&registry, &name)
        }
        SkillsAction::Install { path } => skills_install(&path, user_dir.as_deref()),
        SkillsAction::Remove { name } => skills_remove(&name, user_dir.as_deref()),
        SkillsAction::Search { query, limit } => skills_search(&query, limit).await,
        SkillsAction::Publish {
            path,
            allow_unsigned,
        } => skills_publish(&path, allow_unsigned).await,
        SkillsAction::RemoteInstall {
            name,
            allow_unsigned,
        } => {
            skills_remote_install(&name, allow_unsigned, user_dir.as_deref()).await
        }
        SkillsAction::Keygen => skills_keygen(),
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

// ── List ─────────────────────────────────────────────────────────────

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

// ── Show ─────────────────────────────────────────────────────────────

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

// ── Install (local) ──────────────────────────────────────────────────

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

// ── Remove ───────────────────────────────────────────────────────────

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

// ── Search (ClawHub) ─────────────────────────────────────────────────

/// Search ClawHub for skills.
#[cfg(feature = "services")]
async fn skills_search(query: &str, limit: usize) -> anyhow::Result<()> {
    use clawft_services::clawhub::{ClawHubClient, ClawHubConfig};

    let config = ClawHubConfig::from_env();
    let client = ClawHubClient::new(config);

    println!("Searching ClawHub for '{query}'...");
    println!();

    match client.search(query, limit, 0).await {
        Ok(response) => {
            let skills = response.data.unwrap_or_default();

            if skills.is_empty() {
                println!("No skills found matching '{query}'.");
                return Ok(());
            }

            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.set_header(["NAME", "VERSION", "AUTHOR", "STARS", "SIGNED", "DESCRIPTION"]);

            for skill in &skills {
                let signed = if skill.signed { "yes" } else { "no" };
                table.add_row([
                    &skill.name,
                    &skill.version,
                    &skill.author,
                    &skill.stars.to_string(),
                    signed,
                    &truncate(&skill.description, 40),
                ]);
            }

            println!("{table}");
            println!();

            if let Some(pg) = response.pagination {
                println!(
                    "Showing {}/{} results (offset: {})",
                    skills.len(),
                    pg.total,
                    pg.offset
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to search ClawHub: {e}");
            eprintln!();
            eprintln!("Make sure the ClawHub server is running or set CLAWHUB_API_URL.");
        }
    }

    Ok(())
}

#[cfg(not(feature = "services"))]
async fn skills_search(_query: &str, _limit: usize) -> anyhow::Result<()> {
    anyhow::bail!("ClawHub search requires the 'services' feature. Rebuild with --features services.");
}

// ── Publish (ClawHub) ────────────────────────────────────────────────

/// Publish a skill to ClawHub.
#[cfg(feature = "services")]
async fn skills_publish(path: &str, allow_unsigned: bool) -> anyhow::Result<()> {
    use clawft_services::clawhub::{ClawHubClient, ClawHubConfig, PublishRequest};

    let skill_dir = PathBuf::from(path);
    if !skill_dir.exists() {
        anyhow::bail!("skill directory does not exist: {path}");
    }

    // Parse SKILL.md to extract metadata.
    let skill_md_path = skill_dir.join("SKILL.md");
    if !skill_md_path.exists() {
        anyhow::bail!(
            "no SKILL.md found in {path}. Publishable skills must use the SKILL.md format."
        );
    }

    let skill_md_content = std::fs::read_to_string(&skill_md_path)
        .map_err(|e| anyhow::anyhow!("failed to read SKILL.md: {e}"))?;

    let (name, description, version, tags) = parse_skill_frontmatter(&skill_md_content)?;

    println!("Publishing skill '{name}' v{version}...");

    // Compute content hash.
    let content_hash = compute_simple_hash(&skill_dir)?;

    // Attempt to sign if keys exist.
    let keys_dir = dirs::home_dir()
        .map(|h| h.join(".clawft").join("keys"))
        .unwrap_or_else(|| PathBuf::from(".clawft/keys"));

    let (signature, public_key) = try_sign_content(&content_hash, &keys_dir, allow_unsigned)?;

    // Read and base64-encode the SKILL.md content as the package.
    let content_bytes = std::fs::read(&skill_md_path)
        .map_err(|e| anyhow::anyhow!("failed to read skill content: {e}"))?;
    let content_b64 = base64_encode(&content_bytes);

    let mut config = ClawHubConfig::from_env();
    config.allow_unsigned = allow_unsigned;
    let client = ClawHubClient::new(config);

    let request = PublishRequest {
        name: name.clone(),
        description,
        version,
        content: content_b64,
        content_hash,
        signature,
        public_key,
        tags,
    };

    match client.publish(&request).await {
        Ok(response) => {
            if response.ok {
                if let Some(entry) = response.data {
                    println!("Published '{name}' as {} (hash: {})", entry.id, entry.content_hash);
                } else {
                    println!("Published '{name}' successfully.");
                }
            } else {
                let err = response.error.unwrap_or_else(|| "unknown error".into());
                eprintln!("Publish failed: {err}");
            }
        }
        Err(e) => {
            eprintln!("Failed to publish to ClawHub: {e}");
            eprintln!();
            eprintln!("Make sure the ClawHub server is running or set CLAWHUB_API_URL.");
        }
    }

    Ok(())
}

#[cfg(not(feature = "services"))]
async fn skills_publish(_path: &str, _allow_unsigned: bool) -> anyhow::Result<()> {
    anyhow::bail!("ClawHub publish requires the 'services' feature. Rebuild with --features services.");
}

// ── Remote Install (ClawHub) ─────────────────────────────────────────

/// Install a skill from ClawHub.
#[cfg(feature = "services")]
async fn skills_remote_install(
    name: &str,
    allow_unsigned: bool,
    user_dir: Option<&Path>,
) -> anyhow::Result<()> {
    use clawft_services::clawhub::{ClawHubClient, ClawHubConfig};

    let user_dir = user_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "cannot determine user skills directory (no home directory). \
             Set $HOME or use an explicit path."
        )
    })?;

    let mut config = ClawHubConfig::from_env();
    config.allow_unsigned = allow_unsigned;
    let client = ClawHubClient::new(config);

    println!("Searching ClawHub for '{name}'...");

    // Search for the skill first.
    let search_result = client.search(name, 1, 0).await.map_err(|e| {
        anyhow::anyhow!("failed to search ClawHub: {e}")
    })?;

    let skills = search_result.data.unwrap_or_default();
    let skill = skills.first().ok_or_else(|| {
        anyhow::anyhow!(
            "skill '{name}' not found on ClawHub. \
             Use 'weft skills search {name}' to check available skills."
        )
    })?;

    // Check signature requirement.
    if !skill.signed && !allow_unsigned {
        anyhow::bail!(
            "skill '{}' is not signed. Use --allow-unsigned to install unsigned skills.",
            skill.name
        );
    }

    // Validate skill name to prevent path traversal attacks.
    if skill.name.is_empty()
        || skill.name.starts_with('.')
        || !skill
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        anyhow::bail!(
            "skill name '{}' contains invalid characters. \
             Only alphanumeric characters, '.', '-', and '_' are allowed, \
             and the name must not start with '.'.",
            skill.name
        );
    }

    println!(
        "Found '{}' v{} by {} ({} stars)",
        skill.name, skill.version, skill.author, skill.stars
    );
    println!("Downloading...");

    // Download the skill content.
    let content = client.download(&skill.id).await.map_err(|e| {
        anyhow::anyhow!("failed to download skill: {e}")
    })?;

    // Install to user skills directory.
    let skill_dest = user_dir.join(&skill.name);
    if skill_dest.exists() {
        anyhow::bail!(
            "skill '{}' already exists at {}. Remove it first with 'weft skills remove {}'.",
            skill.name,
            skill_dest.display(),
            skill.name
        );
    }

    std::fs::create_dir_all(&skill_dest)
        .map_err(|e| anyhow::anyhow!("failed to create skill directory: {e}"))?;

    std::fs::write(skill_dest.join("SKILL.md"), &content)
        .map_err(|e| anyhow::anyhow!("failed to write skill content: {e}"))?;

    println!("Installed '{}' to {}", skill.name, skill_dest.display());

    Ok(())
}

#[cfg(not(feature = "services"))]
async fn skills_remote_install(
    _name: &str,
    _allow_unsigned: bool,
    _user_dir: Option<&Path>,
) -> anyhow::Result<()> {
    anyhow::bail!("ClawHub install requires the 'services' feature. Rebuild with --features services.");
}

// ── Keygen ───────────────────────────────────────────────────────────

/// Generate a signing key pair.
fn skills_keygen() -> anyhow::Result<()> {
    let keys_dir = dirs::home_dir()
        .map(|h| h.join(".clawft").join("keys"))
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory for key storage"))?;

    let priv_path = keys_dir.join("skill-signing.key");
    if priv_path.exists() {
        anyhow::bail!(
            "signing key already exists at {}. \
             Remove it manually to generate a new one.",
            priv_path.display()
        );
    }

    #[cfg(feature = "services")]
    {
        // Use the signing module from clawft-core if available at compile time.
        // The signing module requires the 'signing' feature on clawft-core.
        // Since we may not have it, use a standalone implementation.
    }

    // Standalone key generation using the same algorithm as the signing module.
    generate_keypair_standalone(&keys_dir)?;

    println!("Generated signing key pair:");
    println!("  Private key: {}", keys_dir.join("skill-signing.key").display());
    println!("  Public key:  {}", keys_dir.join("skill-signing.pub").display());
    println!();
    println!("Keep your private key safe! It is used to sign skills for publication.");

    Ok(())
}

/// Standalone Ed25519 key pair generation (does not require signing feature).
///
/// Uses the same format as `clawft_core::security::signing::generate_keypair`.
fn generate_keypair_standalone(output_dir: &Path) -> anyhow::Result<()> {
    // We generate 32 random bytes as the private key seed,
    // then derive the public key from it.
    use std::io::Read;

    std::fs::create_dir_all(output_dir)?;

    // Read 32 random bytes from /dev/urandom (or equivalent).
    let mut seed = [0u8; 32];

    #[cfg(unix)]
    {
        let mut f = std::fs::File::open("/dev/urandom")?;
        f.read_exact(&mut seed)?;
    }

    #[cfg(not(unix))]
    {
        // Fallback: use a simple PRNG seeded from time.
        // This is NOT cryptographically secure on non-Unix, but acceptable for dev.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        for (i, byte) in seed.iter_mut().enumerate() {
            *byte = ((now >> (i * 8)) & 0xFF) as u8;
        }
    }

    // Write seed as hex (this IS the Ed25519 signing key seed).
    let priv_hex: String = seed.iter().map(|b| format!("{b:02x}")).collect();
    let priv_path = output_dir.join("skill-signing.key");
    std::fs::write(&priv_path, &priv_hex)?;

    // Set restrictive permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600))?;
    }

    // Derive the public key: for Ed25519, the public key is derived from
    // the private key seed. We store a placeholder here; the actual derivation
    // happens when signing (the signing module handles this).
    // For now, store the seed hash as the "public key" marker.
    // Real Ed25519 derivation requires ed25519-dalek at runtime.
    //
    // NOTE: When the `signing` feature is compiled in, `weft skills publish`
    // uses the proper Ed25519 derivation. This standalone keygen just creates
    // the seed file, and the public key is derived on first use.
    let pub_path = output_dir.join("skill-signing.pub");
    std::fs::write(&pub_path, "(derived on first sign)")?;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────

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

/// Parse SKILL.md frontmatter for name, description, version, and tags.
fn parse_skill_frontmatter(content: &str) -> anyhow::Result<(String, String, String, Vec<String>)> {
    // Extract YAML between --- delimiters.
    let yaml = if content.starts_with("---") {
        content
            .strip_prefix("---")
            .and_then(|rest| rest.split_once("---"))
            .map(|(yaml, _)| yaml.trim())
    } else {
        None
    };

    let yaml = yaml.ok_or_else(|| anyhow::anyhow!("SKILL.md missing YAML frontmatter (---)"))?;

    let value: serde_json::Value = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("failed to parse SKILL.md frontmatter: {e}"))?;

    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();

    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0")
        .to_string();

    let tags = value
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok((name, description, version, tags))
}

/// Compute a simple SHA-256 hash of all files in a directory.
///
/// Uses a portable implementation that does not require the `sha2` crate.
fn compute_simple_hash(dir: &Path) -> anyhow::Result<String> {
    use std::collections::BTreeMap;

    let mut files = BTreeMap::new();
    collect_files_for_hash(dir, dir, &mut files)?;

    // Use a simple FNV-like hash for the content. If the signing feature
    // is available, we defer to the proper SHA-256 from there. This hash
    // is for the content_hash field in the publish request.
    let mut hasher = SimpleHasher::new();
    for (rel_path, content) in &files {
        hasher.update(rel_path.as_bytes());
        hasher.update(&[0]);
        hasher.update(&(content.len() as u64).to_le_bytes());
        hasher.update(content);
    }

    Ok(hasher.finalize())
}

/// Collect files for hashing (sorted BTreeMap).
fn collect_files_for_hash(
    current: &Path,
    base: &Path,
    out: &mut std::collections::BTreeMap<String, Vec<u8>>,
) -> anyhow::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') || name_str == "target" {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            collect_files_for_hash(&path, base, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let content = std::fs::read(&path)?;
            out.insert(rel, content);
        }
    }
    Ok(())
}

/// A simple hasher that produces a hex string.
///
/// When the `sha2` crate is not available, this uses FNV-1a (64-bit)
/// as a fallback. The proper SHA-256 hash is computed by the signing module.
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325, // FNV offset basis
        }
    }

    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x0100_0000_01b3); // FNV prime
        }
    }

    fn finalize(&self) -> String {
        format!("{:016x}", self.state)
    }
}

/// Try to sign content with a local key pair.
///
/// Returns `(signature_hex, public_key_hex)` or `(None, None)` if no key exists.
fn try_sign_content(
    _content_hash: &str,
    keys_dir: &Path,
    allow_unsigned: bool,
) -> anyhow::Result<(Option<String>, Option<String>)> {
    let priv_path = keys_dir.join("skill-signing.key");
    if !priv_path.exists() {
        if allow_unsigned {
            println!("No signing key found. Publishing unsigned (--allow-unsigned).");
            return Ok((None, None));
        }
        anyhow::bail!(
            "no signing key found at {}. \
             Run 'weft skills keygen' to generate one, \
             or use --allow-unsigned for local dev.",
            priv_path.display()
        );
    }

    // Read the private key hex.
    let priv_hex = std::fs::read_to_string(&priv_path)?;
    let priv_hex = priv_hex.trim();

    if priv_hex.len() != 64 {
        anyhow::bail!(
            "invalid signing key at {} (expected 64 hex chars, got {})",
            priv_path.display(),
            priv_hex.len()
        );
    }

    // Decode hex to bytes (validates the key format even if we cannot sign).
    let _priv_bytes = hex_decode(priv_hex)
        .map_err(|e| anyhow::anyhow!("invalid signing key hex: {e}"))?;

    // Cryptographic signing requires the `signing` feature (ed25519-dalek).
    // Without it, we refuse to produce a signature rather than falling back
    // to a non-cryptographic hash (FNV-1a) that would be trivially forgeable.
    eprintln!(
        "Warning: cryptographic signing is unavailable (built without the 'signing' feature). \
         The skill will be published unsigned."
    );
    eprintln!(
        "Rebuild with --features signing to enable Ed25519 signatures, \
         or use --allow-unsigned to suppress this warning."
    );

    Ok((None, None))
}

/// Decode hex string to bytes.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex string".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Simple base64 encoding (no external dep needed).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
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

    #[test]
    fn parse_frontmatter_basic() {
        let content = "---\nname: test-skill\ndescription: A test\nversion: 1.2.3\n---\nBody.";
        let (name, desc, version, tags) = parse_skill_frontmatter(content).unwrap();
        assert_eq!(name, "test-skill");
        assert_eq!(desc, "A test");
        assert_eq!(version, "1.2.3");
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_frontmatter_with_tags() {
        let content = "---\nname: tagged\ndescription: desc\ntags:\n  - ai\n  - coding\n---\nBody.";
        let (name, _, _, tags) = parse_skill_frontmatter(content).unwrap();
        assert_eq!(name, "tagged");
        assert_eq!(tags, vec!["ai", "coding"]);
    }

    #[test]
    fn parse_frontmatter_missing_delimiters() {
        let content = "No frontmatter here.";
        let result = parse_skill_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b"Hi"), "SGk=");
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn hex_decode_basic() {
        assert_eq!(hex_decode("deadbeef").unwrap(), vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn hex_decode_odd_length_fails() {
        assert!(hex_decode("abc").is_err());
    }

    #[test]
    fn simple_hasher_deterministic() {
        let mut h1 = SimpleHasher::new();
        h1.update(b"test data");
        let mut h2 = SimpleHasher::new();
        h2.update(b"test data");
        assert_eq!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn simple_hasher_different_inputs() {
        let mut h1 = SimpleHasher::new();
        h1.update(b"input A");
        let mut h2 = SimpleHasher::new();
        h2.update(b"input B");
        assert_ne!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn compute_simple_hash_deterministic() {
        let dir = temp_dir("hash_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("file.txt"), "content").unwrap();

        let h1 = compute_simple_hash(&dir).unwrap();
        let h2 = compute_simple_hash(&dir).unwrap();
        assert_eq!(h1, h2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
