//! Sandboxed filesystem backed by [`PluginSandbox`] validation.
//!
//! Host-side wrapper (WEFT-398): previously lived in `clawft-wasm::fs`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::sandbox::{self, PluginSandbox};

/// Sandboxed filesystem that validates all operations against a plugin's
/// permissions before executing them.
pub struct SandboxedFileSystem {
    /// The plugin sandbox that governs all access decisions.
    pub sandbox: Arc<PluginSandbox>,
}

impl SandboxedFileSystem {
    /// Create a new sandboxed filesystem for a plugin.
    pub fn new(sandbox: Arc<PluginSandbox>) -> Self {
        Self { sandbox }
    }

    /// Read a file's entire contents as a UTF-8 string.
    ///
    /// The path is validated against the plugin's filesystem permissions,
    /// canonicalized, checked for symlink escapes, and the file size is
    /// verified against the 8 MB read limit before reading.
    pub fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, false).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        std::fs::read_to_string(canonical)
    }

    /// Write a string to a file, creating it if it does not exist.
    ///
    /// The path is validated against the plugin's filesystem permissions.
    /// The parent directory is canonicalized for sandbox containment.
    /// Content size is enforced at 4 MB.
    pub fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if content.len() > sandbox::MAX_WRITE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "write content too large: {} bytes, max {} bytes",
                    content.len(),
                    sandbox::MAX_WRITE_SIZE
                ),
            ));
        }
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, true).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        std::fs::write(canonical, content)
    }

    /// Append a string to a file.
    ///
    /// Validates the path and appends content. The file must already exist
    /// within the sandbox.
    pub fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if content.len() > sandbox::MAX_WRITE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "append content too large: {} bytes, max {} bytes",
                    content.len(),
                    sandbox::MAX_WRITE_SIZE
                ),
            ));
        }
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, false).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(canonical)?;
        file.write_all(content.as_bytes())
    }

    /// Check whether a path exists within the sandbox.
    ///
    /// Returns `false` if the path is outside the sandbox or does not exist.
    pub fn exists(&self, path: &Path) -> bool {
        sandbox::validate_file_access(&self.sandbox, path, false).is_ok()
    }

    /// List all entries in a directory within the sandbox.
    pub fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, false).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(canonical)? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    /// Create a directory and all parent directories within the sandbox.
    pub fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, true).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        std::fs::create_dir_all(canonical)
    }

    /// Remove a file within the sandbox.
    pub fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        let canonical =
            sandbox::validate_file_access(&self.sandbox, path, false).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string())
            })?;
        std::fs::remove_file(canonical)
    }

    /// Get the user's home directory.
    ///
    /// Delegates to `dirs::home_dir()` when available.
    pub fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_plugin::{PluginPermissions, PluginResourceConfig};

    fn canonical_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::create_dir_all(&dir);
        std::fs::canonicalize(&dir).expect("canonicalize temp dir")
    }

    fn sandbox_with_dir(dir: &Path) -> Arc<PluginSandbox> {
        let permissions = PluginPermissions {
            filesystem: vec![dir.to_string_lossy().to_string()],
            ..Default::default()
        };
        Arc::new(PluginSandbox::from_manifest(
            "test-plugin".into(),
            permissions,
            &PluginResourceConfig::default(),
        ))
    }

    fn sandbox_no_fs() -> Arc<PluginSandbox> {
        Arc::new(PluginSandbox::from_manifest(
            "test-plugin".into(),
            PluginPermissions::default(),
            &PluginResourceConfig::default(),
        ))
    }

    #[test]
    fn sandboxed_read_within_allowed_path() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_read");
        let file = dir.join("test.txt");
        std::fs::write(&file, "hello sandbox").unwrap();

        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.read_to_string(&file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello sandbox");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_read_outside_sandbox_denied() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_read_deny");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.read_to_string(Path::new("/etc/hosts"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_write_within_allowed_path() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_write");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let file = dir.join("output.txt");
        let result = fs.write_string(&file, "written");
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "written");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_write_outside_sandbox_denied() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_write_deny");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.write_string(Path::new("/etc/hacked.txt"), "bad");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_write_too_large_rejected() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_write_large");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let large = "x".repeat(5 * 1024 * 1024); // 5 MB > 4 MB limit
        let file = dir.join("big.txt");
        let result = fs.write_string(&file, &large);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_append_within_sandbox() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_append");
        let file = dir.join("log.txt");
        std::fs::write(&file, "line1\n").unwrap();

        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.append_string(&file, "line2\n");
        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "line1\nline2\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_exists_within_sandbox() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_exists");
        let file = dir.join("exists.txt");
        std::fs::write(&file, "yes").unwrap();

        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        assert!(fs.exists(&file));
        assert!(!fs.exists(&dir.join("nope.txt")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_exists_outside_sandbox_returns_false() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_exists_deny");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        assert!(!fs.exists(Path::new("/etc/hosts")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_list_dir_within_sandbox() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_list");
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();

        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.list_dir(&dir);
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_create_dir_within_sandbox() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_mkdir");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let sub = dir.join("sub_dir");
        let result = fs.create_dir_all(&sub);
        assert!(result.is_ok());
        assert!(sub.is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_remove_file_within_sandbox() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_rm");
        let file = dir.join("deleteme.txt");
        std::fs::write(&file, "bye").unwrap();

        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        let result = fs.remove_file(&file);
        assert!(result.is_ok());
        assert!(!file.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sandboxed_no_fs_permissions_denied() {
        let fs = SandboxedFileSystem::new(sandbox_no_fs());
        let result = fs.read_to_string(Path::new("/tmp/test.txt"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn sandboxed_home_dir_returns_some() {
        let dir = canonical_temp_dir("clawft_sandboxed_fs_home");
        let fs = SandboxedFileSystem::new(sandbox_with_dir(&dir));
        // home_dir delegates to dirs::home_dir() which should return Some on Linux/macOS
        let home = fs.home_dir();
        assert!(home.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
