//! CLAWFT.md loading with import resolution.
//!
//! Loads workspace-level `CLAWFT.md` files and resolves `@path/to/file`
//! import directives. Imports are resolved relative to the CLAWFT.md
//! file's directory with security constraints (no `..` traversal, no
//! absolute paths, bounded recursion depth).
//!
//! # Import syntax
//!
//! Lines starting with `@` (after optional whitespace) are treated as
//! import directives. The remainder of the line is interpreted as a
//! relative file path:
//!
//! ```text
//! @prompts/safety.md
//! @agents/researcher.md
//! ```
//!
//! Imports are inlined recursively (up to depth 5). All other lines
//! pass through unchanged.

use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use clawft_types::{ClawftError, Result};

/// Maximum import recursion depth to prevent circular imports.
const MAX_IMPORT_DEPTH: u32 = 5;

/// Load `CLAWFT.md` from a workspace root with import resolution.
///
/// Searches for `CLAWFT.md` in the workspace root directory, reads its
/// content, and resolves any `@path/to/file` import directives. Imports
/// are resolved relative to the directory containing the CLAWFT.md file.
///
/// # Security
///
/// - Import paths containing `..` are rejected.
/// - Absolute import paths are rejected.
/// - Recursion is bounded to [`MAX_IMPORT_DEPTH`] levels.
///
/// # Errors
///
/// Returns [`ClawftError::ConfigInvalid`] if the CLAWFT.md file cannot
/// be found or read, or if an import path violates security constraints.
pub fn load_clawft_md(workspace_root: &Path) -> Result<String> {
    let clawft_md_path = workspace_root.join("CLAWFT.md");

    if !clawft_md_path.exists() {
        debug!(path = %clawft_md_path.display(), "no CLAWFT.md found");
        return Ok(String::new());
    }

    let content =
        std::fs::read_to_string(&clawft_md_path).map_err(|e| ClawftError::ConfigInvalid {
            reason: format!("failed to read CLAWFT.md: {e}"),
        })?;

    let base_dir = clawft_md_path.parent().unwrap_or(workspace_root);

    resolve_imports(&content, base_dir, 0)
}

/// Walk up from `start_dir` to find all CLAWFT.md files, bounded at
/// `.git` directory or the filesystem root.
///
/// Returns paths ordered from most specific (closest to `start_dir`)
/// to most general (highest ancestor). This enables callers to merge
/// content with the most specific file taking precedence.
pub fn find_clawft_md_chain(start_dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut dir = start_dir;

    loop {
        let candidate = dir.join("CLAWFT.md");
        if candidate.exists() {
            results.push(candidate);
        }

        // Stop at .git boundary
        if dir.join(".git").exists() {
            break;
        }

        match dir.parent() {
            Some(parent) if parent != dir => dir = parent,
            _ => break,
        }
    }

    results
}

/// Process a CLAWFT.md content string, resolving `@import` lines.
///
/// Lines starting with `@` (after optional leading whitespace) are
/// treated as import directives. The path following `@` is resolved
/// relative to `base_dir`.
///
/// # Arguments
///
/// * `content` -- Raw CLAWFT.md content string
/// * `base_dir` -- Directory for resolving relative import paths
/// * `depth` -- Current recursion depth (starts at 0)
///
/// # Errors
///
/// Returns an error if:
/// - An import path contains `..` (path traversal)
/// - An import path is absolute
/// - Recursion depth exceeds [`MAX_IMPORT_DEPTH`]
/// - An imported file cannot be read
fn resolve_imports(content: &str, base_dir: &Path, depth: u32) -> Result<String> {
    if depth > MAX_IMPORT_DEPTH {
        return Err(ClawftError::ConfigInvalid {
            reason: format!(
                "CLAWFT.md import depth exceeded maximum of {MAX_IMPORT_DEPTH} \
                 (possible circular import)"
            ),
        });
    }

    let mut output = String::with_capacity(content.len());

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(import_path) = trimmed.strip_prefix('@') {
            let import_path = import_path.trim();
            if import_path.is_empty() {
                // Bare `@` line -- pass through
                output.push_str(line);
                output.push('\n');
                continue;
            }

            // Security: reject path traversal
            if import_path.contains("..") {
                return Err(ClawftError::ConfigInvalid {
                    reason: format!("CLAWFT.md import path contains '..': {import_path}"),
                });
            }

            // Security: reject absolute paths
            let import = Path::new(import_path);
            if import.is_absolute() {
                return Err(ClawftError::ConfigInvalid {
                    reason: format!("CLAWFT.md import path is absolute: {import_path}"),
                });
            }

            // Resolve relative to base_dir
            let resolved = base_dir.join(import);

            match std::fs::read_to_string(&resolved) {
                Ok(imported_content) => {
                    let imported_base = resolved.parent().unwrap_or(base_dir);

                    // Recursively resolve imports in the imported file
                    let processed = resolve_imports(&imported_content, imported_base, depth + 1)?;
                    output.push_str(&processed);
                    // Ensure a trailing newline after imported content
                    if !processed.ends_with('\n') {
                        output.push('\n');
                    }
                }
                Err(e) => {
                    warn!(
                        path = %resolved.display(),
                        error = %e,
                        "failed to resolve CLAWFT.md import, skipping"
                    );
                    // Include a comment noting the failed import
                    output.push_str(&format!("<!-- import failed: {import_path} ({e}) -->\n"));
                }
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("clawft-test-md-{label}-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── Basic loading ───────────────────────────────────────────────

    #[test]
    fn load_clawft_md_basic() {
        let dir = temp_dir("basic");
        std::fs::write(dir.join("CLAWFT.md"), "# My Workspace\n\nHello.\n").unwrap();

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.contains("# My Workspace"));
        assert!(result.contains("Hello."));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_clawft_md_missing_returns_empty() {
        let dir = temp_dir("missing");

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Import resolution ───────────────────────────────────────────

    #[test]
    fn import_resolves_relative_file() {
        let dir = temp_dir("import-basic");
        std::fs::create_dir_all(dir.join("prompts")).unwrap();

        std::fs::write(dir.join("prompts/safety.md"), "Be safe.\n").unwrap();
        std::fs::write(
            dir.join("CLAWFT.md"),
            "# Workspace\n@prompts/safety.md\nDone.\n",
        )
        .unwrap();

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.contains("# Workspace"));
        assert!(result.contains("Be safe."));
        assert!(result.contains("Done."));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_nested_resolution() {
        let dir = temp_dir("import-nested");
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(dir.join("a/b")).unwrap();

        std::fs::write(dir.join("a/b/deep.md"), "deep content\n").unwrap();
        std::fs::write(dir.join("a/middle.md"), "@b/deep.md\n").unwrap();
        std::fs::write(dir.join("CLAWFT.md"), "top\n@a/middle.md\nbottom\n").unwrap();

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.contains("top"));
        assert!(result.contains("deep content"));
        assert!(result.contains("bottom"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Security: path traversal rejected ───────────────────────────

    #[test]
    fn import_with_dotdot_rejected() {
        let dir = temp_dir("dotdot");
        std::fs::write(dir.join("CLAWFT.md"), "@../etc/passwd\n").unwrap();

        let result = load_clawft_md(&dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains(".."), "error should mention '..'");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Security: absolute path rejected ────────────────────────────

    #[test]
    fn import_with_absolute_path_rejected() {
        let dir = temp_dir("absolute");
        std::fs::write(dir.join("CLAWFT.md"), "@/etc/passwd\n").unwrap();

        let result = load_clawft_md(&dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("absolute"), "error should mention 'absolute'");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Security: circular import prevented (max depth) ─────────────

    #[test]
    fn import_max_depth_prevents_circular() {
        let dir = temp_dir("circular");
        // Create a file that imports itself
        std::fs::write(dir.join("CLAWFT.md"), "@loop.md\n").unwrap();
        std::fs::write(dir.join("loop.md"), "@loop.md\n").unwrap();

        let result = load_clawft_md(&dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("depth exceeded") || err.contains("maximum"),
            "error should mention depth: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Import with missing file is handled gracefully ──────────────

    #[test]
    fn import_missing_file_produces_comment() {
        let dir = temp_dir("missing-import");
        std::fs::write(dir.join("CLAWFT.md"), "before\n@nonexistent.md\nafter\n").unwrap();

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.contains("before"));
        assert!(result.contains("after"));
        assert!(result.contains("<!-- import failed"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Bare @ line passes through ──────────────────────────────────

    #[test]
    fn bare_at_sign_passes_through() {
        let dir = temp_dir("bare-at");
        std::fs::write(dir.join("CLAWFT.md"), "hello\n@\nworld\n").unwrap();

        let result = load_clawft_md(&dir).unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("@"));
        assert!(result.contains("world"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── find_clawft_md_chain ────────────────────────────────────────

    #[test]
    fn find_chain_collects_parent_clawft_md() {
        let dir = temp_dir("chain");
        let child = dir.join("project");
        std::fs::create_dir_all(&child).unwrap();

        std::fs::write(dir.join("CLAWFT.md"), "parent").unwrap();
        std::fs::write(child.join("CLAWFT.md"), "child").unwrap();

        let chain = find_clawft_md_chain(&child);
        assert!(chain.len() >= 2);
        assert_eq!(chain[0], child.join("CLAWFT.md"));
        assert_eq!(chain[1], dir.join("CLAWFT.md"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_chain_stops_at_git_boundary() {
        let dir = temp_dir("git-boundary");
        let repo = dir.join("repo");
        let sub = repo.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();

        std::fs::write(dir.join("CLAWFT.md"), "outside git").unwrap();
        std::fs::write(repo.join("CLAWFT.md"), "repo root").unwrap();
        std::fs::write(sub.join("CLAWFT.md"), "subdir").unwrap();

        let chain = find_clawft_md_chain(&sub);
        // Should find sub and repo, but NOT dir (above .git boundary)
        let paths: Vec<&Path> = chain.iter().map(|p| p.as_path()).collect();
        assert!(paths.contains(&sub.join("CLAWFT.md").as_path()));
        assert!(paths.contains(&repo.join("CLAWFT.md").as_path()));
        assert!(!paths.contains(&dir.join("CLAWFT.md").as_path()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── resolve_imports unit tests ──────────────────────────────────

    #[test]
    fn resolve_imports_no_imports() {
        let dir = temp_dir("no-imports");
        let content = "line 1\nline 2\nline 3\n";
        let result = resolve_imports(content, &dir, 0).unwrap();
        assert_eq!(result, "line 1\nline 2\nline 3\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_imports_depth_zero_success() {
        let dir = temp_dir("depth-zero");
        std::fs::write(dir.join("inc.md"), "included\n").unwrap();

        let content = "before\n@inc.md\nafter\n";
        let result = resolve_imports(content, &dir, 0).unwrap();
        assert!(result.contains("included"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_imports_at_max_depth_fails() {
        let dir = temp_dir("max-depth");
        let content = "@something.md\n";
        let result = resolve_imports(content, &dir, MAX_IMPORT_DEPTH + 1);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
