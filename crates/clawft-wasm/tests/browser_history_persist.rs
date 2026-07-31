//! OPFS conversation history + config persistence — WEFT-399.
//!
//! Verifies that with `--features browser-opfs`:
//! 1. Session JSONL turns written via [`LocalFileSink`] survive a fresh
//!    filesystem open (page-reload simulation).
//! 2. Per-group `CLAWFT.md` (CLAUDE.md-per-group) survives reopen.
//! 3. Config snapshot at [`config_persist_path`] survives reopen (P6.4).
//!
//! Run:
//! ```text
//! wasm-pack test --headless --chrome crates/clawft-wasm \
//!   --no-default-features --features browser-opfs \
//!   --test browser_history_persist
//! ```
//!
//! Or: `FEATURES=browser-opfs scripts/build.sh test-browser`

#![cfg(all(target_arch = "wasm32", feature = "browser-opfs"))]

use std::path::Path;
use std::sync::Arc;

use clawft_core::agent::local_file_sink::LocalFileSink;
use clawft_core::agent::sink::{ConversationSink, Turn};
use clawft_platform::browser::{
    config_persist_path, conv_id_for_group, group_clawft_md_path, sessions_dir, BrowserFileSystem,
    BrowserFsBackend, BrowserPlatform, DEFAULT_GROUP_ID,
};
use clawft_platform::fs::FileSystem;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn turn(role: &str, content: &str) -> Turn {
    Turn {
        turn_id: format!("t-{role}"),
        role: role.into(),
        content: content.into(),
        tool_calls: None,
        tool_call_id: None,
        ts_ms: 1_700_000_000_000,
        voice_analysis: None,
        audio: None,
    }
}

/// Unique group per run so parallel chrome sessions don't collide.
fn test_group() -> String {
    format!("weft399-{}", js_sys::Date::now() as u64)
}

#[wasm_bindgen_test]
async fn conversation_jsonl_survives_reopen() {
    let group = test_group();
    let conv = conv_id_for_group(&group);
    let user_text = format!("hello-{}", js_sys::Date::now());
    let asst_text = format!("hi-{}", js_sys::Date::now());

    // Session 1: open OPFS platform, append turns, drop.
    {
        let platform = Arc::new(BrowserPlatform::open().await);
        assert_eq!(
            platform.fs_backend(),
            BrowserFsBackend::Opfs,
            "secure context should expose OPFS"
        );
        let sessions = sessions_dir();
        platform
            .fs()
            .create_dir_all(&sessions)
            .await
            .expect("mkdir sessions");
        let sink = LocalFileSink::with_dir(Arc::clone(&platform), sessions);
        sink.append_turn(&conv, turn("user", &user_text))
            .await
            .expect("append user");
        sink.append_turn(&conv, turn("assistant", &asst_text))
            .await
            .expect("append assistant");
        let hist = sink.history(&conv, 0).await;
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].content, user_text);
        assert_eq!(hist[1].content, asst_text);
    }

    // Session 2: new handles (reload simulation).
    {
        let platform = Arc::new(BrowserPlatform::open().await);
        assert_eq!(platform.fs_backend(), BrowserFsBackend::Opfs);
        let sink = LocalFileSink::with_dir(Arc::clone(&platform), sessions_dir());
        let hist = sink.history(&conv, 0).await;
        assert_eq!(
            hist.len(),
            2,
            "turns must persist across OPFS reopen for conv={conv}"
        );
        assert_eq!(hist[0].role, "user");
        assert_eq!(hist[0].content, user_text);
        assert_eq!(hist[1].role, "assistant");
        assert_eq!(hist[1].content, asst_text);

        sink.delete(&conv).await.expect("cleanup history");
    }

    // Session 3: delete persisted.
    {
        let platform = Arc::new(BrowserPlatform::open().await);
        let sink = LocalFileSink::with_dir(platform, sessions_dir());
        assert!(
            sink.history(&conv, 0).await.is_empty(),
            "cleared history must stay empty after reopen"
        );
    }
}

#[wasm_bindgen_test]
async fn group_clawft_md_survives_reopen() {
    let group = test_group();
    let path = group_clawft_md_path(&group);
    let body = format!(
        "# Group {group}\n\nPrefer concise answers. WEFT-399 CLAWFT.md.\n"
    );

    {
        let fs = BrowserFileSystem::open().await;
        assert_eq!(fs.backend(), BrowserFsBackend::Opfs);
        fs.write_string(&path, &body).await.expect("write CLAWFT.md");
        assert_eq!(fs.read_to_string(&path).await.expect("read"), body);
    }

    {
        let fs = BrowserFileSystem::open().await;
        let read_back = fs
            .read_to_string(&path)
            .await
            .expect("CLAWFT.md must survive reopen");
        assert_eq!(read_back, body);
        let _ = fs.remove_file(&path).await;
    }
}

#[wasm_bindgen_test]
async fn config_snapshot_survives_reopen() {
    // P6.4: init → reload → config survives.
    let path = config_persist_path();
    let stamp = js_sys::Date::now() as u64;
    let config_json = format!(
        r#"{{"agents":{{"defaults":{{"model":"test-model-{stamp}"}}}},"weft399":true}}"#
    );

    {
        let fs = BrowserFileSystem::open().await;
        assert_eq!(fs.backend(), BrowserFsBackend::Opfs);
        fs.write_string(&path, &config_json)
            .await
            .expect("write config snapshot");
    }

    {
        let fs = BrowserFileSystem::open().await;
        let read_back = fs
            .read_to_string(&path)
            .await
            .expect("config must survive reopen (P6.4)");
        assert_eq!(read_back, config_json);
        assert!(read_back.contains(&format!("test-model-{stamp}")));
        // Leave a clean path for subsequent runs (best-effort).
        let _ = fs.remove_file(Path::new(&path)).await;
    }
}

#[wasm_bindgen_test]
async fn default_group_conv_id_is_web_browser() {
    assert_eq!(conv_id_for_group(DEFAULT_GROUP_ID), "web:browser");
    assert_eq!(
        sessions_dir().display().to_string(),
        "/clawft/.clawft/workspace/sessions"
    );
}
