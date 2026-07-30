//! OPFS persistence regression — WEFT-392.
//!
//! Verifies that with `--features browser-opfs`, a write through
//! [`BrowserFileSystem::open`] survives a fresh open (simulates page
//! reload: new handles into the same origin OPFS root).
//!
//! Run:
//! ```text
//! wasm-pack test --headless --chrome crates/clawft-wasm \
//!   --no-default-features --features browser-opfs \
//!   --test browser_opfs
//! ```
//!
//! Or: `FEATURES=browser-opfs scripts/build.sh test-browser` once the
//! suite is wired; until then use the command above.

#![cfg(all(target_arch = "wasm32", feature = "browser-opfs"))]

use std::path::Path;

use clawft_platform::browser::{BrowserFileSystem, BrowserFsBackend, BROWSER_WORKSPACE_DIR};
use clawft_platform::fs::FileSystem;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Unique path per run so parallel chrome sessions don't stomp each other.
fn test_path() -> std::path::PathBuf {
    let stamp = js_sys::Date::now() as u64;
    Path::new(BROWSER_WORKSPACE_DIR).join(format!("weft392-persist-{stamp}.txt"))
}

#[wasm_bindgen_test]
async fn opfs_write_survives_reopen() {
    let path = test_path();
    let payload = format!("weft-392-persist-{}", js_sys::Date::now());

    // Session 1: open OPFS, write, drop handle.
    {
        let fs = BrowserFileSystem::open().await;
        assert_eq!(
            fs.backend(),
            BrowserFsBackend::Opfs,
            "secure context should expose OPFS in headless Chrome"
        );
        fs.write_string(&path, &payload)
            .await
            .expect("OPFS write");
        let read_back = fs.read_to_string(&path).await.expect("OPFS read same session");
        assert_eq!(read_back, payload);
        let meta = fs.metadata(&path).await.expect("metadata");
        assert!(!meta.is_dir);
        assert_eq!(meta.len, payload.len() as u64);
    }

    // Session 2: new BrowserFileSystem (reload simulation).
    {
        let fs = BrowserFileSystem::open().await;
        assert_eq!(fs.backend(), BrowserFsBackend::Opfs);
        let read_back = fs
            .read_to_string(&path)
            .await
            .expect("OPFS read after reopen (persistence)");
        assert_eq!(
            read_back, payload,
            "content written before reopen must persist across new OPFS handles"
        );
        // Cleanup so subsequent runs stay tidy.
        let _ = fs.remove_file(&path).await;
    }
}

#[wasm_bindgen_test]
async fn opfs_list_dir_reports_is_dir_via_metadata() {
    let fs = BrowserFileSystem::open().await;
    assert_eq!(fs.backend(), BrowserFsBackend::Opfs);

    let dir = Path::new(BROWSER_WORKSPACE_DIR).join("weft392-list");
    let file = dir.join("note.txt");
    fs.create_dir_all(&dir).await.expect("mkdir");
    fs.write_string(&file, "abc").await.expect("write");

    let entries = fs.list_dir(&dir).await.expect("list");
    assert!(
        entries.iter().any(|p| p.ends_with("note.txt")),
        "list_dir should include note.txt: {entries:?}"
    );

    let file_meta = fs.metadata(&file).await.expect("file meta");
    assert!(!file_meta.is_dir);
    assert_eq!(file_meta.len, 3);

    let dir_meta = fs.metadata(&dir).await.expect("dir meta");
    assert!(dir_meta.is_dir);
    assert_eq!(dir_meta.len, 0);

    let _ = fs.remove_file(&file).await;
}
