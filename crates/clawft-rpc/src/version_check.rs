//! Background version check with 24-hour cache.
//!
//! On every CLI invocation, checks whether a newer version of WeftOS
//! is available. The check result is cached for 24 hours to avoid
//! hammering the GitHub API. Prints a one-line notice to stderr if
//! an update is available.
//!
//! The check is non-blocking — it runs in a spawned thread and never
//! delays the main command.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const REPO: &str = "weave-logic-ai/weftos";
const CACHE_FILE: &str = "version-check.json";
const CACHE_TTL_SECS: u64 = 86400; // 24 hours
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cached version check result.
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    latest_version: String,
    checked_at: u64,
}

fn cache_path() -> PathBuf {
    crate::runtime_dir().join(CACHE_FILE)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check if a cached result is still fresh.
fn read_cache() -> Option<CacheEntry> {
    let data = fs::read_to_string(cache_path()).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    if now_secs() - entry.checked_at < CACHE_TTL_SECS {
        Some(entry)
    } else {
        None
    }
}

/// Write a cache entry.
fn write_cache(latest: &str) {
    let entry = CacheEntry {
        latest_version: latest.to_string(),
        checked_at: now_secs(),
    };
    let dir = crate::runtime_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(
        dir.join(CACHE_FILE),
        serde_json::to_string(&entry).unwrap_or_default(),
    );
}

/// Fetch latest version from GitHub Releases API.
fn fetch_latest() -> Option<String> {
    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout", "3",
            "--max-time", "5",
            "-H", "Accept: application/vnd.github.v3+json",
            &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let tag = body["tag_name"].as_str()?;
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

/// Compare two semver-ish version strings.
/// Returns true if `latest` is newer than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };
    let c = parse(current);
    let l = parse(latest);
    l > c
}

/// Print an update notice to stderr if a newer version is available.
///
/// This is the main entry point. Call from main() before command dispatch.
/// It checks the cache first, and only hits the network if the cache is stale.
/// The network check runs in a background thread so it never blocks.
fn check_and_notify_sync() {
    // 1. Check cache
    if let Some(cached) = read_cache() {
        if is_newer(CURRENT_VERSION, &cached.latest_version) {
            eprintln!(
                "\x1b[33m↑ WeftOS v{} available (current: v{}). Run: weaver update\x1b[0m",
                cached.latest_version, CURRENT_VERSION
            );
        }
        return; // Cache is fresh, done
    }

    // 2. Cache is stale — fetch in background thread (don't block the command)
    std::thread::spawn(|| {
        if let Some(latest) = fetch_latest() {
            write_cache(&latest);
            if is_newer(CURRENT_VERSION, &latest) {
                eprintln!(
                    "\x1b[33m↑ WeftOS v{} available (current: v{}). Run: weaver update\x1b[0m",
                    latest, CURRENT_VERSION
                );
            }
        }
    });
}

/// Non-blocking version check. Call from main() before command dispatch.
///
/// Prints to stderr if an update is available:
/// ```text
/// ↑ WeftOS v0.5.6 available (current: v0.5.5). Run: weaver update
/// ```
///
/// Suppressed by setting `WEFTOS_NO_UPDATE_CHECK=1`.
pub fn check_for_updates() {
    // Allow users to disable
    if std::env::var("WEFTOS_NO_UPDATE_CHECK").map(|v| v == "1").unwrap_or(false) {
        return;
    }
    check_and_notify_sync();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_works() {
        assert!(is_newer("0.5.4", "0.5.5"));
        assert!(is_newer("0.5.5", "0.6.0"));
        assert!(is_newer("0.5.5", "1.0.0"));
        assert!(!is_newer("0.5.5", "0.5.5"));
        assert!(!is_newer("0.5.5", "0.5.4"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn current_version_is_set() {
        assert!(!CURRENT_VERSION.is_empty());
    }
}
