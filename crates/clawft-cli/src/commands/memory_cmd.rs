//! `weft memory` -- read and search agent memory files.
//!
//! Provides commands to inspect the long-term memory (`MEMORY.md`) and
//! session history (`HISTORY.md`) files managed by the agent, as well
//! as a substring search across both.
//!
//! # Examples
//!
//! ```text
//! weft memory show
//! weft memory history
//! weft memory search "authentication" --limit 5
//! ```

use std::sync::Arc;

use clawft_core::agent::memory::MemoryStore;
use clawft_platform::NativePlatform;
use clawft_types::config::Config;

/// Read and display the contents of `MEMORY.md`.
///
/// Initialises a [`MemoryStore`] via the native platform, prints the
/// resolved file path, then outputs the file contents (or a placeholder
/// if the file is empty / does not exist).
pub async fn memory_show(_config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let store = MemoryStore::new(platform)
        .map_err(|e| anyhow::anyhow!("failed to initialize memory store: {e}"))?;

    println!("Memory file: {}", store.memory_path().display());
    println!();

    let content = store
        .read_long_term()
        .await
        .map_err(|e| anyhow::anyhow!("failed to read memory: {e}"))?;

    if content.is_empty() {
        println!("(no memory entries)");
    } else {
        println!("{content}");
    }
    Ok(())
}

/// Read and display the contents of `HISTORY.md`.
///
/// Initialises a [`MemoryStore`] via the native platform, prints the
/// resolved file path, then outputs the file contents (or a placeholder
/// if the file is empty / does not exist).
pub async fn memory_history(_config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let store = MemoryStore::new(platform)
        .map_err(|e| anyhow::anyhow!("failed to initialize memory store: {e}"))?;

    println!("History file: {}", store.history_path().display());
    println!();

    let content = store
        .read_history()
        .await
        .map_err(|e| anyhow::anyhow!("failed to read history: {e}"))?;

    if content.is_empty() {
        println!("(no history entries)");
    } else {
        println!("{content}");
    }
    Ok(())
}

/// Search memory and history files for paragraphs matching `query`.
///
/// Results are printed numbered, one per paragraph, capped at `limit`.
pub async fn memory_search(query: &str, limit: usize, _config: &Config) -> anyhow::Result<()> {
    let platform = Arc::new(NativePlatform::new());
    let store = MemoryStore::new(platform)
        .map_err(|e| anyhow::anyhow!("failed to initialize memory store: {e}"))?;

    let results = store.search(query, limit).await;

    if results.is_empty() {
        println!("No results for \"{query}\"");
    } else {
        println!(
            "Found {} result{} for \"{}\":\n",
            results.len(),
            if results.len() == 1 { "" } else { "s" },
            query,
        );
        for (i, paragraph) in results.iter().enumerate() {
            println!("{}. {}", i + 1, paragraph);
            println!();
        }
    }
    Ok(())
}

// ── Formatting helpers (pure, used by tests) ────────────────────────────

/// Format the search results header line.
#[cfg(test)]
fn format_search_header(query: &str, count: usize) -> String {
    if count == 0 {
        format!("No results for \"{query}\"")
    } else {
        format!(
            "Found {} result{} for \"{}\":",
            count,
            if count == 1 { "" } else { "s" },
            query,
        )
    }
}

/// Format a single numbered search result.
#[cfg(test)]
fn format_search_result(index: usize, paragraph: &str) -> String {
    format!("{}. {}", index + 1, paragraph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_search_header_zero_results() {
        let header = format_search_header("missing", 0);
        assert_eq!(header, "No results for \"missing\"");
    }

    #[test]
    fn format_search_header_single_result() {
        let header = format_search_header("auth", 1);
        assert_eq!(header, "Found 1 result for \"auth\":");
    }

    #[test]
    fn format_search_header_multiple_results() {
        let header = format_search_header("config", 5);
        assert_eq!(header, "Found 5 results for \"config\":");
    }

    #[test]
    fn format_search_result_first() {
        let line = format_search_result(0, "first paragraph");
        assert_eq!(line, "1. first paragraph");
    }

    #[test]
    fn format_search_result_tenth() {
        let line = format_search_result(9, "tenth paragraph");
        assert_eq!(line, "10. tenth paragraph");
    }

    #[test]
    fn format_search_result_preserves_content() {
        let content = "multi word paragraph with special chars: &<>";
        let line = format_search_result(2, content);
        assert!(line.contains(content));
        assert!(line.starts_with("3. "));
    }

    #[test]
    fn format_search_header_query_with_quotes() {
        let header = format_search_header("it's a \"test\"", 3);
        assert!(header.contains("it's a \"test\""));
        assert!(header.contains("3 results"));
    }

    #[test]
    fn format_search_header_empty_query() {
        let header = format_search_header("", 0);
        assert_eq!(header, "No results for \"\"");
    }
}
