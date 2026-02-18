//! Memory bootstrap: index existing MEMORY.md into a vector store.
//!
//! On first startup (or when the index file is missing), this module reads
//! the workspace's `MEMORY.md`, splits it into sections, embeds each
//! section, and stores the results in an [`RvfStore`] for later semantic
//! search.
//!
//! This module is gated behind the `rvf` feature flag.

use std::path::Path;

use tracing::{debug, info, warn};

use crate::embeddings::Embedder;
use crate::embeddings::rvf_stub::{RvfError, RvfStore};

/// Errors that can occur during memory bootstrap.
#[derive(Debug)]
pub enum BootstrapError {
    /// An I/O error occurred reading files.
    Io(std::io::Error),
    /// An embedding error occurred.
    Embedding(crate::embeddings::EmbeddingError),
    /// An RVF store error occurred.
    Store(RvfError),
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapError::Io(e) => write!(f, "bootstrap I/O error: {e}"),
            BootstrapError::Embedding(e) => write!(f, "bootstrap embedding error: {e}"),
            BootstrapError::Store(e) => write!(f, "bootstrap store error: {e}"),
        }
    }
}

impl std::error::Error for BootstrapError {}

impl From<std::io::Error> for BootstrapError {
    fn from(e: std::io::Error) -> Self {
        BootstrapError::Io(e)
    }
}

impl From<crate::embeddings::EmbeddingError> for BootstrapError {
    fn from(e: crate::embeddings::EmbeddingError) -> Self {
        BootstrapError::Embedding(e)
    }
}

impl From<RvfError> for BootstrapError {
    fn from(e: RvfError) -> Self {
        BootstrapError::Store(e)
    }
}

/// Bootstrap the memory vector index from an existing MEMORY.md file.
///
/// # Algorithm
///
/// 1. Check if the index file at `index_path` already exists -- if so, skip.
/// 2. Read the MEMORY.md file at `memory_path`.
/// 3. Split the content into sections by `## ` headers or double-newline
///    paragraphs.
/// 4. Embed each section using the provided [`Embedder`].
/// 5. Store embeddings in an [`RvfStore`] and compact (persist) it.
/// 6. Return the number of sections indexed.
///
/// # Arguments
///
/// * `memory_path` - Path to the MEMORY.md file.
/// * `index_path` - Path where the index file will be stored (e.g. `memory.rvf.json`).
/// * `embedder` - The embedder to use for generating vectors.
///
/// # Errors
///
/// Returns [`BootstrapError`] if reading the memory file, embedding, or
/// persisting the index fails.
pub async fn bootstrap_memory_index(
    memory_path: &Path,
    index_path: &Path,
    embedder: &dyn Embedder,
) -> Result<usize, BootstrapError> {
    // 1. Check if index already exists
    if index_path.exists() {
        info!(
            path = %index_path.display(),
            "memory index already exists, skipping bootstrap"
        );
        return Ok(0);
    }

    // 2. Read MEMORY.md
    if !memory_path.exists() {
        info!(
            path = %memory_path.display(),
            "MEMORY.md not found, skipping bootstrap"
        );
        return Ok(0);
    }

    let content = std::fs::read_to_string(memory_path)?;
    if content.trim().is_empty() {
        info!("MEMORY.md is empty, skipping bootstrap");
        return Ok(0);
    }

    debug!(
        path = %memory_path.display(),
        bytes = content.len(),
        "read MEMORY.md for bootstrap"
    );

    // 3. Split into sections
    let sections = split_into_sections(&content);
    if sections.is_empty() {
        info!("no sections found in MEMORY.md");
        return Ok(0);
    }

    debug!(count = sections.len(), "split MEMORY.md into sections");

    // 4. Embed each section
    let mut store = RvfStore::create(Some(index_path));
    let mut indexed = 0;

    for (i, section) in sections.iter().enumerate() {
        let trimmed = section.trim();
        if trimmed.is_empty() {
            continue;
        }

        match embedder.embed(trimmed).await {
            Ok(embedding) => {
                let id = format!("memory-section-{i}");
                let metadata = serde_json::json!({
                    "source": "MEMORY.md",
                    "section_index": i,
                    "text": trimmed,
                    "char_count": trimmed.len(),
                });
                store.ingest(id, embedding, metadata);
                indexed += 1;
            }
            Err(e) => {
                warn!(
                    section = i,
                    error = %e,
                    "failed to embed section, skipping"
                );
            }
        }
    }

    // 5. Persist the store
    store.compact()?;

    info!(
        indexed,
        total_sections = sections.len(),
        path = %index_path.display(),
        "memory bootstrap complete"
    );

    Ok(indexed)
}

/// Split markdown content into sections.
///
/// Splitting strategy:
/// 1. First try splitting by `## ` headers (level-2 markdown headings).
/// 2. If that produces fewer than 2 sections, fall back to splitting by
///    double newlines (paragraphs).
/// 3. Filter out empty sections.
fn split_into_sections(content: &str) -> Vec<String> {
    // Try header-based splitting first
    let header_sections: Vec<String> = split_by_headers(content);
    if header_sections.len() >= 2 {
        return header_sections;
    }

    // Fall back to paragraph splitting
    split_by_paragraphs(content)
}

/// Split by `## ` markdown headers, keeping the header with its content.
fn split_by_headers(content: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if line.starts_with("## ") && !current.trim().is_empty() {
            sections.push(current.trim().to_string());
            current = String::new();
        }
        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        sections.push(current.trim().to_string());
    }

    sections.into_iter().filter(|s| !s.is_empty()).collect()
}

/// Split by double newlines (paragraph breaks).
fn split_by_paragraphs(content: &str) -> Vec<String> {
    content
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::api_embedder::ApiEmbedder;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_bootstrap_test_{label}_{pid}_{n}"))
    }

    #[test]
    fn split_by_headers_basic() {
        let content = "# Title\n\nIntro text.\n\n## Section One\n\nContent one.\n\n## Section Two\n\nContent two.";
        let sections = split_by_headers(content);
        assert_eq!(sections.len(), 3);
        assert!(sections[0].starts_with("# Title"));
        assert!(sections[1].starts_with("## Section One"));
        assert!(sections[2].starts_with("## Section Two"));
    }

    #[test]
    fn split_by_headers_no_headers() {
        let content = "Just some plain text\nwith no headers.";
        let sections = split_by_headers(content);
        assert_eq!(sections.len(), 1);
    }

    #[test]
    fn split_by_paragraphs_basic() {
        let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let sections = split_by_paragraphs(content);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0], "First paragraph.");
        assert_eq!(sections[1], "Second paragraph.");
        assert_eq!(sections[2], "Third paragraph.");
    }

    #[test]
    fn split_into_sections_uses_headers_when_available() {
        let content = "## One\n\nContent.\n\n## Two\n\nMore content.";
        let sections = split_into_sections(content);
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn split_into_sections_falls_back_to_paragraphs() {
        let content = "Para one.\n\nPara two.\n\nPara three.";
        let sections = split_into_sections(content);
        assert_eq!(sections.len(), 3);
    }

    #[test]
    fn split_into_sections_empty() {
        let sections = split_into_sections("");
        assert!(sections.is_empty());
    }

    #[test]
    fn split_into_sections_whitespace_only() {
        let sections = split_into_sections("   \n\n   \n\n   ");
        assert!(sections.is_empty());
    }

    #[tokio::test]
    async fn bootstrap_creates_index() {
        let dir = temp_dir("creates_index");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md");
        let index_path = dir.join("memory.rvf.json");

        std::fs::write(
            &memory_path,
            "## Facts\n\nThe sky is blue.\n\n## Notes\n\nRust is fast.",
        )
        .unwrap();

        let embedder = ApiEmbedder::hash_only(64);
        let count = bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        assert_eq!(count, 2);
        assert!(index_path.exists());

        // Verify we can reopen the store
        let store = RvfStore::open(&index_path).unwrap();
        assert_eq!(store.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn bootstrap_skips_existing_index() {
        let dir = temp_dir("skips_existing");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md");
        let index_path = dir.join("memory.rvf.json");

        std::fs::write(&memory_path, "## Stuff\n\nContent.").unwrap();
        std::fs::write(&index_path, "{}").unwrap(); // pre-existing index

        let embedder = ApiEmbedder::hash_only(64);
        let count = bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        assert_eq!(count, 0, "should skip when index already exists");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn bootstrap_skips_missing_memory() {
        let dir = temp_dir("missing_memory");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md"); // does not exist
        let index_path = dir.join("memory.rvf.json");

        let embedder = ApiEmbedder::hash_only(64);
        let count = bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        assert_eq!(count, 0, "should skip when MEMORY.md is missing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn bootstrap_skips_empty_memory() {
        let dir = temp_dir("empty_memory");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md");
        let index_path = dir.join("memory.rvf.json");

        std::fs::write(&memory_path, "").unwrap();

        let embedder = ApiEmbedder::hash_only(64);
        let count = bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        assert_eq!(count, 0, "should skip when MEMORY.md is empty");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn bootstrap_paragraph_splitting() {
        let dir = temp_dir("paragraphs");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md");
        let index_path = dir.join("memory.rvf.json");

        std::fs::write(
            &memory_path,
            "First fact about something.\n\nSecond fact about another thing.\n\nThird fact.",
        )
        .unwrap();

        let embedder = ApiEmbedder::hash_only(64);
        let count = bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        assert_eq!(count, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn bootstrap_index_is_searchable() {
        let dir = temp_dir("searchable");
        std::fs::create_dir_all(&dir).unwrap();

        let memory_path = dir.join("MEMORY.md");
        let index_path = dir.join("memory.rvf.json");

        std::fs::write(
            &memory_path,
            "## Rust\n\nRust is a systems programming language.\n\n## Python\n\nPython is interpreted.",
        )
        .unwrap();

        let embedder = ApiEmbedder::hash_only(64);
        bootstrap_memory_index(&memory_path, &index_path, &embedder)
            .await
            .unwrap();

        // Reopen and search
        let store = RvfStore::open(&index_path).unwrap();
        let query_emb = embedder.hash_fallback("rust programming");
        let results = store.query(&query_emb, 2);
        assert_eq!(results.len(), 2);

        // Verify metadata contains text
        let first = &results[0];
        assert!(first.metadata.get("text").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
