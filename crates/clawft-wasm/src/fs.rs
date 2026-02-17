//! WASI filesystem stub.
//!
//! Provides a [`WasiFileSystem`] with a self-contained API for filesystem operations.
//! Currently all methods return `Unsupported` errors. Once WASI filesystem APIs
//! (`wasi:filesystem/types` and `wasi:filesystem/preopens`) are stable and
//! accessible from Rust, this will be replaced with real filesystem operations
//! scoped to the pre-opened directories granted by the WASI host.
//!
//! This module is fully decoupled from `clawft-platform` so it can compile for
//! `wasm32-wasip1` without pulling in tokio or reqwest.

use std::path::{Path, PathBuf};

/// Filesystem for WASI environments.
///
/// This is a stub implementation that will use WASI filesystem capabilities
/// (`wasi:filesystem/types`, `wasi:filesystem/preopens`) once they are stable.
/// Until then, all methods return [`std::io::ErrorKind::Unsupported`] errors.
pub struct WasiFileSystem;

impl WasiFileSystem {
    /// Create a new WASI filesystem handle.
    pub fn new() -> Self {
        Self
    }

    /// Read a file's entire contents as a UTF-8 string.
    pub fn read_to_string(&self, _path: &Path) -> std::io::Result<String> {
        Err(unsupported("read_to_string"))
    }

    /// Write a string to a file, creating parent directories if needed.
    pub fn write_string(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        Err(unsupported("write_string"))
    }

    /// Append a string to a file.
    pub fn append_string(&self, _path: &Path, _content: &str) -> std::io::Result<()> {
        Err(unsupported("append_string"))
    }

    /// Check whether a path exists.
    pub fn exists(&self, _path: &Path) -> bool {
        false
    }

    /// List all entries in a directory.
    pub fn list_dir(&self, _path: &Path) -> std::io::Result<Vec<PathBuf>> {
        Err(unsupported("list_dir"))
    }

    /// Create a directory and all parent directories.
    pub fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
        Err(unsupported("create_dir_all"))
    }

    /// Remove a file.
    pub fn remove_file(&self, _path: &Path) -> std::io::Result<()> {
        Err(unsupported("remove_file"))
    }

    /// Get the user's home directory.
    ///
    /// WASI environments have no concept of a user home directory.
    pub fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

impl Default for WasiFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create an unsupported error with a descriptive message.
fn unsupported(operation: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("WASI filesystem not yet implemented: {operation}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_filesystem_can_be_created() {
        let _fs = WasiFileSystem::new();
    }

    #[test]
    fn wasi_filesystem_default() {
        let _fs = WasiFileSystem::default();
    }

    #[test]
    fn read_to_string_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.read_to_string(Path::new("/tmp/test.txt"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
        assert!(err.to_string().contains("read_to_string"));
    }

    #[test]
    fn write_string_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.write_string(Path::new("/tmp/test.txt"), "content");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn append_string_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.append_string(Path::new("/tmp/test.txt"), "more");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn exists_returns_false() {
        let fs = WasiFileSystem::new();
        assert!(!fs.exists(Path::new("/tmp/test.txt")));
    }

    #[test]
    fn list_dir_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.list_dir(Path::new("/tmp"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn create_dir_all_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.create_dir_all(Path::new("/tmp/a/b/c"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn remove_file_returns_unsupported() {
        let fs = WasiFileSystem::new();
        let result = fs.remove_file(Path::new("/tmp/test.txt"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn home_dir_returns_none() {
        let fs = WasiFileSystem::new();
        assert!(fs.home_dir().is_none());
    }

    #[test]
    fn wasi_filesystem_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasiFileSystem>();
    }
}
