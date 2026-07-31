//! BrowserEnvironment OPFS persistence — WEFT-14.
//!
//! Verifies that with `--features browser-opfs`, env vars written through
//! [`BrowserEnvironment::open`] survive a fresh open (simulates page
//! reload: new handles into the same origin OPFS root / env.json).
//!
//! Run:
//! ```text
//! wasm-pack test --headless --chrome crates/clawft-wasm \
//!   --no-default-features --features browser-opfs \
//!   --test browser_env_persist
//! ```
//!
//! Or: `FEATURES=browser-opfs scripts/build.sh test-browser`

#![cfg(all(target_arch = "wasm32", feature = "browser-opfs"))]

use clawft_platform::browser::{
    BrowserEnvBackend, BrowserEnvironment, ENV_PERSIST_PATH,
};
use clawft_platform::env::Environment;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn env_vars_survive_reopen() {
    let key = format!("WEFT14_PERSIST_{}", js_sys::Date::now() as u64);
    let value = format!("value-{}", js_sys::Date::now());

    // Session 1: open OPFS env, set, flush, drop.
    {
        let env = BrowserEnvironment::open().await;
        assert_eq!(
            env.backend(),
            BrowserEnvBackend::Opfs,
            "secure context should expose OPFS env backend in headless Chrome"
        );
        env.set_var(&key, &value);
        env.flush().await.expect("flush env snapshot to OPFS");
        assert_eq!(env.get_var(&key), Some(value.clone()));
    }

    // Session 2: new BrowserEnvironment (reload simulation).
    {
        let env = BrowserEnvironment::open().await;
        assert_eq!(env.backend(), BrowserEnvBackend::Opfs);
        assert_eq!(
            env.get_var(&key),
            Some(value.clone()),
            "env set before reopen must persist across new OPFS handles ({ENV_PERSIST_PATH})"
        );
        // Cleanup so subsequent runs stay tidy.
        env.remove_var(&key);
        env.flush().await.expect("flush cleanup");
        assert!(env.get_var(&key).is_none());
    }

    // Session 3: confirm remove also persisted.
    {
        let env = BrowserEnvironment::open().await;
        assert!(
            env.get_var(&key).is_none(),
            "removed key must stay gone after reopen"
        );
    }
}

#[wasm_bindgen_test]
async fn open_with_seed_overlays_persisted() {
    let base_key = format!("WEFT14_BASE_{}", js_sys::Date::now() as u64);
    let seed_key = format!("WEFT14_SEED_{}", js_sys::Date::now() as u64);

    {
        let env = BrowserEnvironment::open().await;
        env.set_var(&base_key, "from-disk");
        env.flush().await.expect("flush");
    }

    let mut seed = std::collections::HashMap::new();
    seed.insert(seed_key.clone(), "from-seed".to_string());
    seed.insert(base_key.clone(), "seed-wins".to_string());

    let env = BrowserEnvironment::open_with_seed(seed).await;
    assert_eq!(env.backend(), BrowserEnvBackend::Opfs);
    assert_eq!(env.get_var(&seed_key), Some("from-seed".to_string()));
    assert_eq!(
        env.get_var(&base_key),
        Some("seed-wins".to_string()),
        "seed must overlay persisted keys"
    );

    env.remove_var(&base_key);
    env.remove_var(&seed_key);
    env.flush().await.expect("cleanup");
}

#[wasm_bindgen_test]
async fn debug_redacts_in_browser() {
    let env = BrowserEnvironment::open().await;
    env.set_var("ANTHROPIC_API_KEY", "sk-browser-secret");
    env.set_var("MODEL", "claude");
    let dbg = format!("{env:?}");
    assert!(
        !dbg.contains("sk-browser-secret"),
        "secret must not appear in Debug: {dbg}"
    );
    assert!(dbg.contains("[REDACTED]"), "expected redaction: {dbg}");
    assert!(dbg.contains("claude"), "non-sensitive should show: {dbg}");
    env.remove_var("ANTHROPIC_API_KEY");
    env.remove_var("MODEL");
    let _ = env.flush().await;
}
