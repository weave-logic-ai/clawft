//! Browser filesystem implementation.
//!
//! ## Backends
//!
//! * **In-memory** (default) — `HashMap`-backed store. Files live only for the
//!   lifetime of the [`BrowserFileSystem`] instance and do not survive reloads.
//! * **OPFS** (`browser-opfs` feature) — Origin Private File System via
//!   `navigator.storage.getDirectory()` and web-sys `FileSystem*Handle`
//!   bindings (stable since web-sys **0.3.70**; verified on **0.3.85**). When
//!   OPFS is unavailable at runtime the constructor falls back to memory.
//!
//! ## Virtual path layout (WEFT-392 home_dir collision)
//!
//! Browser home is always the absolute virtual root [`BROWSER_HOME_DIR`]
//! (`/clawft`). Config / skills live under `/clawft/.clawft/…` — never the
//! relative cwd path `.clawft/` used by native overlays. The default agent
//! workspace is [`BROWSER_WORKSPACE_DIR`] (`/clawft/workspace`).

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::fs::{FileSystem, FsMetadata};

/// Virtual home directory for all browser / OPFS filesystem backends.
///
/// Maps to OPFS root component `clawft` (leading `/` is virtual).
pub const BROWSER_HOME_DIR: &str = "/clawft";

/// Default agent workspace under the browser home.
pub const BROWSER_WORKSPACE_DIR: &str = "/clawft/workspace";

/// Which storage backend a [`BrowserFileSystem`] is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFsBackend {
    /// In-memory `HashMap` (default / fallback).
    Memory,
    /// Origin Private File System (requires `browser-opfs` + secure context).
    Opfs,
}

/// Browser filesystem implementing [`FileSystem`].
///
/// Defaults to an in-memory store. With the `browser-opfs` feature, call
/// [`BrowserFileSystem::open`] to prefer OPFS with automatic memory fallback.
pub struct BrowserFileSystem {
    inner: BrowserFsInner,
}

enum BrowserFsInner {
    Memory(MemoryStore),
    #[cfg(feature = "browser-opfs")]
    Opfs(opfs::OpfsStore),
}

struct MemoryStore {
    files: Mutex<HashMap<PathBuf, String>>,
    dirs: Mutex<Vec<PathBuf>>,
}

impl BrowserFileSystem {
    /// Create a new empty in-memory filesystem.
    pub fn new() -> Self {
        Self {
            inner: BrowserFsInner::Memory(MemoryStore {
                files: Mutex::new(HashMap::new()),
                dirs: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Which backend is active.
    pub fn backend(&self) -> BrowserFsBackend {
        match &self.inner {
            BrowserFsInner::Memory(_) => BrowserFsBackend::Memory,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(_) => BrowserFsBackend::Opfs,
        }
    }

    /// Open the best available backend.
    ///
    /// * With `browser-opfs`: try OPFS; on failure fall back to memory.
    /// * Without the feature: always memory (same as [`new`]).
    pub async fn open() -> Self {
        #[cfg(feature = "browser-opfs")]
        {
            match opfs::OpfsStore::open().await {
                Ok(store) => {
                    tracing::info!("BrowserFileSystem: using OPFS backend");
                    return Self {
                        inner: BrowserFsInner::Opfs(store),
                    };
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "BrowserFileSystem: OPFS unavailable, falling back to in-memory store"
                    );
                }
            }
        }
        Self::new()
    }

    /// Force the in-memory backend (useful in tests).
    pub fn memory() -> Self {
        Self::new()
    }
}

impl Default for BrowserFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a path to a canonical form for consistent lookups.
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other),
        }
    }
    result
}

// ---------------------------------------------------------------------------
// In-memory backend
// ---------------------------------------------------------------------------

impl MemoryStore {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let key = normalize(path);
        self.files
            .lock()
            .expect("BrowserFileSystem mutex poisoned")
            .get(&key)
            .cloned()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file not found: {}", key.display()),
                )
            })
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let key = normalize(path);
        if let Some(parent) = key.parent() {
            self.create_dir_all(parent).await?;
        }
        self.files
            .lock()
            .expect("BrowserFileSystem mutex poisoned")
            .insert(key, content.to_string());
        Ok(())
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let key = normalize(path);
        if let Some(parent) = key.parent() {
            self.create_dir_all(parent).await?;
        }
        let mut files = self.files.lock().expect("BrowserFileSystem mutex poisoned");
        let entry = files.entry(key).or_default();
        entry.push_str(content);
        Ok(())
    }

    async fn exists(&self, path: &Path) -> bool {
        let key = normalize(path);
        let files = self.files.lock().expect("BrowserFileSystem mutex poisoned");
        if files.contains_key(&key) {
            return true;
        }
        let dirs = self
            .dirs
            .lock()
            .expect("BrowserFileSystem dirs mutex poisoned");
        if dirs.iter().any(|d| d == &key) {
            return true;
        }
        files.keys().any(|k| k.starts_with(&key) && k != &key)
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let dir = normalize(path);
        let files = self.files.lock().expect("BrowserFileSystem mutex poisoned");

        let mut entries = std::collections::BTreeSet::new();
        for key in files.keys() {
            if let Ok(suffix) = key.strip_prefix(&dir) {
                if let Some(first) = suffix.components().next() {
                    entries.insert(dir.join(first));
                }
            }
        }

        let dirs = self
            .dirs
            .lock()
            .expect("BrowserFileSystem dirs mutex poisoned");
        for d in dirs.iter() {
            if let Ok(suffix) = d.strip_prefix(&dir) {
                if let Some(first) = suffix.components().next() {
                    let child = dir.join(first);
                    if child != dir {
                        entries.insert(child);
                    }
                }
            }
        }

        if entries.is_empty() && !self.exists(path).await {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("directory not found: {}", dir.display()),
            ));
        }

        Ok(entries.into_iter().collect())
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        let key = normalize(path);
        let mut dirs = self
            .dirs
            .lock()
            .expect("BrowserFileSystem dirs mutex poisoned");
        let mut current = PathBuf::new();
        for component in key.components() {
            current.push(component);
            if !dirs.contains(&current) {
                dirs.push(current.clone());
            }
        }
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        let key = normalize(path);
        self.files
            .lock()
            .expect("BrowserFileSystem mutex poisoned")
            .remove(&key)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file not found: {}", key.display()),
                )
            })?;
        Ok(())
    }

    async fn metadata(&self, path: &Path) -> std::io::Result<FsMetadata> {
        let key = normalize(path);
        let files = self.files.lock().expect("BrowserFileSystem mutex poisoned");
        if let Some(content) = files.get(&key) {
            return Ok(FsMetadata {
                is_dir: false,
                len: content.len() as u64,
            });
        }
        drop(files);
        if self.exists(&key).await {
            // Exists but not a file → directory (explicit or implicit).
            return Ok(FsMetadata {
                is_dir: true,
                len: 0,
            });
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("path not found: {}", key.display()),
        ))
    }
}

// ---------------------------------------------------------------------------
// FileSystem dispatch
// ---------------------------------------------------------------------------

#[async_trait(?Send)]
impl FileSystem for BrowserFileSystem {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.read_to_string(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.read_to_string(path).await,
        }
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.write_string(path, content).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.write_string(path, content).await,
        }
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.append_string(path, content).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.append_string(path, content).await,
        }
    }

    async fn exists(&self, path: &Path) -> bool {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.exists(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.exists(path).await,
        }
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.list_dir(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.list_dir(path).await,
        }
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.create_dir_all(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.create_dir_all(path).await,
        }
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.remove_file(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.remove_file(path).await,
        }
    }

    fn home_dir(&self) -> Option<PathBuf> {
        // Always the virtual absolute root — see module docs (WEFT-392).
        Some(PathBuf::from(BROWSER_HOME_DIR))
    }

    async fn metadata(&self, path: &Path) -> std::io::Result<FsMetadata> {
        match &self.inner {
            BrowserFsInner::Memory(s) => s.metadata(path).await,
            #[cfg(feature = "browser-opfs")]
            BrowserFsInner::Opfs(s) => s.metadata(path).await,
        }
    }
}

// ---------------------------------------------------------------------------
// OPFS backend (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "browser-opfs")]
mod opfs {
    use super::normalize;
    use crate::fs::FsMetadata;
    use std::path::{Component, Path, PathBuf};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions,
        FileSystemGetFileOptions, FileSystemHandle, FileSystemHandleKind,
        FileSystemWritableFileStream, StorageManager, WorkerGlobalScope,
    };

    /// Wrapper so OPFS handles can live behind `Platform: Send + Sync`.
    ///
    /// # Safety
    /// Browser/WASM is single-threaded; `FileSystemDirectoryHandle` is never
    /// shared across real OS threads.
    struct SendDirHandle(FileSystemDirectoryHandle);

    // SAFETY: wasm32 browser target is single-threaded; handles stay on the
    // main (or dedicated worker) event loop.
    unsafe impl Send for SendDirHandle {}
    unsafe impl Sync for SendDirHandle {}

    /// OPFS-backed store rooted at `navigator.storage.getDirectory()`.
    ///
    /// Virtual paths map 1:1 onto OPFS after dropping a leading `/`.
    /// e.g. `/clawft/workspace/a.txt` → OPFS `clawft/workspace/a.txt`.
    pub(super) struct OpfsStore {
        root: SendDirHandle,
    }

    impl OpfsStore {
        pub(super) async fn open() -> Result<Self, String> {
            let storage = storage_manager().ok_or_else(|| {
                "navigator.storage unavailable (need secure context / Window or Worker)".to_string()
            })?;
            let dir_val = JsFuture::from(storage.get_directory())
                .await
                .map_err(js_err_string)?;
            let root: FileSystemDirectoryHandle = dir_val
                .dyn_into()
                .map_err(|_| "getDirectory() did not return FileSystemDirectoryHandle".to_string())?;
            let store = Self {
                root: SendDirHandle(root),
            };
            // Ensure the virtual home layout exists so config/workspace paths resolve.
            store
                .create_dir_all(Path::new(super::BROWSER_HOME_DIR))
                .await
                .map_err(|e| e.to_string())?;
            store
                .create_dir_all(Path::new(super::BROWSER_WORKSPACE_DIR))
                .await
                .map_err(|e| e.to_string())?;
            Ok(store)
        }

        pub(super) async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
            let file_handle = self.get_file_handle(path, false).await?;
            let file_val = JsFuture::from(file_handle.get_file())
                .await
                .map_err(js_to_io)?;
            let file: File = file_val.dyn_into().map_err(js_to_io)?;
            let text_val = JsFuture::from(file.text()).await.map_err(js_to_io)?;
            text_val
                .as_string()
                .ok_or_else(|| std::io::Error::other("File.text() did not return a string"))
        }

        pub(super) async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
            let file_handle = self.get_file_handle(path, true).await?;
            let writable_val = JsFuture::from(file_handle.create_writable())
                .await
                .map_err(js_to_io)?;
            let writable: FileSystemWritableFileStream =
                writable_val.dyn_into().map_err(js_to_io)?;
            JsFuture::from(writable.write_with_u8_array(content.as_bytes()).map_err(js_to_io)?)
                .await
                .map_err(js_to_io)?;
            // FileSystemWritableFileStream extends WritableStream.
            let stream: web_sys::WritableStream = writable.unchecked_into();
            JsFuture::from(stream.close()).await.map_err(js_to_io)?;
            Ok(())
        }

        pub(super) async fn append_string(
            &self,
            path: &Path,
            content: &str,
        ) -> std::io::Result<()> {
            let existing = match self.read_to_string(path).await {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(e),
            };
            let mut combined = existing;
            combined.push_str(content);
            self.write_string(path, &combined).await
        }

        pub(super) async fn exists(&self, path: &Path) -> bool {
            let key = normalize(path);
            if is_root(&key) {
                return true;
            }
            if self.get_file_handle(&key, false).await.is_ok() {
                return true;
            }
            self.get_dir_handle(&key, false).await.is_ok()
        }

        pub(super) async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
            let dir = normalize(path);
            let handle = self.get_dir_handle(&dir, false).await?;
            let mut entries = Vec::new();
            let iter = handle.values();
            loop {
                let next_promise = iter.next().map_err(js_to_io)?;
                let next = JsFuture::from(next_promise).await.map_err(js_to_io)?;
                let done = js_sys::Reflect::get(&next, &JsValue::from_str("done"))
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if done {
                    break;
                }
                let value = js_sys::Reflect::get(&next, &JsValue::from_str("value"))
                    .map_err(js_to_io)?;
                let handle: FileSystemHandle = value.dyn_into().map_err(js_to_io)?;
                let name = handle.name();
                entries.push(dir.join(name));
            }
            Ok(entries)
        }

        pub(super) async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
            let key = normalize(path);
            if is_root(&key) {
                return Ok(());
            }
            let _ = self.get_dir_handle(&key, true).await?;
            Ok(())
        }

        pub(super) async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            let key = normalize(path);
            let (parent, name) = parent_and_name(&key)?;
            let parent_handle = self.get_dir_handle(&parent, false).await?;
            JsFuture::from(parent_handle.remove_entry(&name))
                .await
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("remove_file failed for {}: {}", key.display(), js_err_string(e)),
                    )
                })?;
            Ok(())
        }

        pub(super) async fn metadata(&self, path: &Path) -> std::io::Result<FsMetadata> {
            let key = normalize(path);
            if is_root(&key) {
                return Ok(FsMetadata {
                    is_dir: true,
                    len: 0,
                });
            }
            if let Ok(file_handle) = self.get_file_handle(&key, false).await {
                let file_val = JsFuture::from(file_handle.get_file())
                    .await
                    .map_err(js_to_io)?;
                let file: File = file_val.dyn_into().map_err(js_to_io)?;
                return Ok(FsMetadata {
                    is_dir: false,
                    len: file.size() as u64,
                });
            }
            if self.get_dir_handle(&key, false).await.is_ok() {
                return Ok(FsMetadata {
                    is_dir: true,
                    len: 0,
                });
            }
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path not found: {}", key.display()),
            ))
        }

        async fn get_dir_handle(
            &self,
            path: &Path,
            create: bool,
        ) -> std::io::Result<FileSystemDirectoryHandle> {
            let key = normalize(path);
            if is_root(&key) {
                return Ok(self.root.0.clone());
            }
            let mut current = self.root.0.clone();
            for component in path_components(&key) {
                let opts = FileSystemGetDirectoryOptions::new();
                opts.set_create(create);
                let promise =
                    current.get_directory_handle_with_options(&component, &opts);
                let val = JsFuture::from(promise).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "directory not found ({}): {}",
                            component,
                            js_err_string(e)
                        ),
                    )
                })?;
                current = val.dyn_into().map_err(js_to_io)?;
            }
            Ok(current)
        }

        async fn get_file_handle(
            &self,
            path: &Path,
            create: bool,
        ) -> std::io::Result<FileSystemFileHandle> {
            let key = normalize(path);
            let (parent, name) = parent_and_name(&key)?;
            let parent_handle = self.get_dir_handle(&parent, create).await?;
            let opts = FileSystemGetFileOptions::new();
            opts.set_create(create);
            let promise = parent_handle.get_file_handle_with_options(&name, &opts);
            let val = JsFuture::from(promise).await.map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("file not found ({}): {}", key.display(), js_err_string(e)),
                )
            })?;
            val.dyn_into().map_err(js_to_io)
        }
    }

    fn storage_manager() -> Option<StorageManager> {
        if let Some(window) = web_sys::window() {
            return Some(window.navigator().storage());
        }
        if let Some(worker) = js_sys::global().dyn_ref::<WorkerGlobalScope>() {
            return Some(worker.navigator().storage());
        }
        None
    }

    fn is_root(path: &Path) -> bool {
        path.as_os_str().is_empty()
            || path == Path::new("/")
            || path == Path::new(".")
            || path.components().all(|c| matches!(c, Component::RootDir))
    }

    fn path_components(path: &Path) -> Vec<String> {
        path.components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect()
    }

    fn parent_and_name(path: &Path) -> std::io::Result<(PathBuf, String)> {
        let key = normalize(path);
        let name = key
            .file_name()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("path has no file name: {}", key.display()),
                )
            })?
            .to_string_lossy()
            .into_owned();
        let parent = key.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("/"));
        Ok((parent, name))
    }

    fn js_err_string(value: JsValue) -> String {
        if let Some(s) = value.as_string() {
            s
        } else if let Some(obj) = value.dyn_ref::<js_sys::Error>() {
            String::from(obj.message())
        } else {
            format!("{value:?}")
        }
    }

    fn js_to_io(value: JsValue) -> std::io::Error {
        std::io::Error::other(js_err_string(value))
    }

    // Silence unused import if kind helpers expand later.
    #[allow(dead_code)]
    fn handle_is_dir(handle: &FileSystemHandle) -> bool {
        handle.kind() == FileSystemHandleKind::Directory
    }
}

// ---------------------------------------------------------------------------
// Tests (host + browser feature; OPFS path needs wasm32 + secure context)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileSystem;

    #[tokio::test]
    async fn memory_write_read_round_trip() {
        let fs = BrowserFileSystem::memory();
        assert_eq!(fs.backend(), BrowserFsBackend::Memory);
        let path = Path::new("/clawft/workspace/hello.txt");
        fs.write_string(path, "hi").await.unwrap();
        assert_eq!(fs.read_to_string(path).await.unwrap(), "hi");
        let meta = fs.metadata(path).await.unwrap();
        assert!(!meta.is_dir);
        assert_eq!(meta.len, 2);
    }

    #[tokio::test]
    async fn memory_list_dir_metadata_dirs() {
        let fs = BrowserFileSystem::memory();
        fs.write_string(Path::new("/clawft/workspace/a.txt"), "aa")
            .await
            .unwrap();
        fs.create_dir_all(Path::new("/clawft/workspace/sub"))
            .await
            .unwrap();
        let entries = fs
            .list_dir(Path::new("/clawft/workspace"))
            .await
            .unwrap();
        assert!(entries.iter().any(|p| p.ends_with("a.txt")));
        assert!(entries.iter().any(|p| p.ends_with("sub")));

        let file_meta = fs
            .metadata(Path::new("/clawft/workspace/a.txt"))
            .await
            .unwrap();
        assert!(!file_meta.is_dir);
        assert_eq!(file_meta.len, 2);

        let dir_meta = fs
            .metadata(Path::new("/clawft/workspace/sub"))
            .await
            .unwrap();
        assert!(dir_meta.is_dir);
        assert_eq!(dir_meta.len, 0);
    }

    #[tokio::test]
    async fn memory_append_and_remove() {
        let fs = BrowserFileSystem::memory();
        let path = Path::new("/clawft/workspace/log.txt");
        fs.append_string(path, "one").await.unwrap();
        fs.append_string(path, "two").await.unwrap();
        assert_eq!(fs.read_to_string(path).await.unwrap(), "onetwo");
        fs.remove_file(path).await.unwrap();
        assert!(!fs.exists(path).await);
    }

    #[test]
    fn home_dir_is_virtual_absolute_root() {
        let fs = BrowserFileSystem::memory();
        assert_eq!(
            fs.home_dir().as_deref(),
            Some(Path::new(BROWSER_HOME_DIR))
        );
        // Config path collision resolved: home/.clawft is absolute under /clawft.
        let config = fs.home_dir().unwrap().join(".clawft").join("config.json");
        assert_eq!(
            config,
            PathBuf::from("/clawft/.clawft/config.json")
        );
        assert_eq!(BROWSER_WORKSPACE_DIR, "/clawft/workspace");
    }

    #[test]
    fn normalize_collapses_dot_and_parent() {
        assert_eq!(
            normalize(Path::new("/clawft/workspace/../.clawft/skills")),
            PathBuf::from("/clawft/.clawft/skills")
        );
    }
}
