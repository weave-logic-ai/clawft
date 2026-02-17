//! Filesystem abstraction and native implementation.
//!
//! Provides a platform-agnostic [`FileSystem`] trait for file I/O and a native
//! implementation backed by [`tokio::fs`]. A WASM implementation would use
//! WASI filesystem or browser-based storage.

use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Platform-agnostic filesystem operations.
///
/// All path arguments use [`Path`] for proper cross-platform path handling.
/// Implementations should create parent directories when writing files.
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Read a file's entire contents as a UTF-8 string.
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// Write a string to a file, creating parent directories if needed.
    ///
    /// Overwrites the file if it already exists.
    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()>;

    /// Append a string to a file, creating it (and parent dirs) if it does not exist.
    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()>;

    /// Check whether a path exists (file or directory).
    async fn exists(&self, path: &Path) -> bool;

    /// List all entries in a directory.
    ///
    /// Returns the full path of each entry. Does not recurse into subdirectories.
    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>;

    /// Create a directory and all parent directories.
    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;

    /// Remove a file.
    async fn remove_file(&self, path: &Path) -> std::io::Result<()>;

    /// Get the user's home directory.
    fn home_dir(&self) -> Option<PathBuf>;
}

/// Native filesystem implementation using [`tokio::fs`].
pub struct NativeFileSystem;

#[async_trait]
impl FileSystem for NativeFileSystem {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        tokio::fs::read_to_string(path).await
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            entries.push(entry.path());
        }
        Ok(entries)
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        tokio::fs::create_dir_all(path).await
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        tokio::fs::remove_file(path).await
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Generate a unique temp path for test isolation.
    fn temp_test_path(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_test_{prefix}_{pid}_{id}"))
    }

    #[tokio::test]
    async fn test_write_and_read_string() {
        let fs = NativeFileSystem;
        let path = temp_test_path("rw");

        fs.write_string(&path, "hello clawft").await.unwrap();
        let contents = fs.read_to_string(&path).await.unwrap();
        assert_eq!(contents, "hello clawft");

        fs.remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let fs = NativeFileSystem;
        let dir = temp_test_path("parent");
        let path = dir.join("sub").join("file.txt");

        fs.write_string(&path, "nested").await.unwrap();
        assert!(fs.exists(&path).await);

        let contents = fs.read_to_string(&path).await.unwrap();
        assert_eq!(contents, "nested");

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_append_string() {
        let fs = NativeFileSystem;
        let path = temp_test_path("append");

        fs.write_string(&path, "first").await.unwrap();
        fs.append_string(&path, " second").await.unwrap();

        let contents = fs.read_to_string(&path).await.unwrap();
        assert_eq!(contents, "first second");

        fs.remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_exists_true_and_false() {
        let fs = NativeFileSystem;
        let path = temp_test_path("exists");

        assert!(!fs.exists(&path).await);

        fs.write_string(&path, "").await.unwrap();
        assert!(fs.exists(&path).await);

        fs.remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_dir() {
        let fs = NativeFileSystem;
        let dir = temp_test_path("listdir");
        fs.create_dir_all(&dir).await.unwrap();

        fs.write_string(&dir.join("a.txt"), "a").await.unwrap();
        fs.write_string(&dir.join("b.txt"), "b").await.unwrap();

        let entries = fs.list_dir(&dir).await.unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<String> = entries
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));

        // Cleanup
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_create_dir_all() {
        let fs = NativeFileSystem;
        let dir = temp_test_path("mkdirs").join("a").join("b").join("c");

        fs.create_dir_all(&dir).await.unwrap();
        assert!(fs.exists(&dir).await);

        // Cleanup
        let root = temp_test_path("mkdirs");
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn test_remove_file() {
        let fs = NativeFileSystem;
        let path = temp_test_path("remove");

        fs.write_string(&path, "delete me").await.unwrap();
        assert!(fs.exists(&path).await);

        fs.remove_file(&path).await.unwrap();
        assert!(!fs.exists(&path).await);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let fs = NativeFileSystem;
        let path = temp_test_path("nonexistent_read");
        let result = fs.read_to_string(&path).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_home_dir() {
        let fs = NativeFileSystem;
        // home_dir should return Some on all standard platforms
        assert!(fs.home_dir().is_some());
    }
}
