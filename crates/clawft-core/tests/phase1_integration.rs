//! Phase 1 integration tests.
//!
//! Verifies that core subsystems (config loading, message flow, tool execution,
//! session persistence) work together correctly.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use clawft_core::bootstrap::AppContext;
use clawft_core::session::SessionManager;
use clawft_platform::NativePlatform;
use clawft_types::config::Config;
use clawft_types::session::Session;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("clawft_phase1_{prefix}_{pid}_{id}"))
}

/// Test 1: Config loads and initializes all subsystems.
#[tokio::test]
async fn test_config_loads_and_initializes_all_subsystems() {
    let config = Config::default();
    let platform = Arc::new(NativePlatform::new());
    let ctx = AppContext::new(config, platform).await.unwrap();

    // All subsystems should be accessible.
    assert!(ctx.tools().is_empty()); // No tools registered yet
    assert!(ctx.memory().memory_path().is_absolute());
    assert!(ctx.skills().skills_dir().is_absolute());
    let _bus = ctx.bus().clone();
}

/// Test 2: Session persistence across restarts.
#[tokio::test]
async fn test_session_persistence_across_restarts() {
    let dir = temp_dir("session_persist");
    let sessions_dir = dir.join("sessions");
    let platform = Arc::new(NativePlatform::new());

    // First manager: create and save a session.
    {
        let mgr = SessionManager::with_dir(platform.clone(), sessions_dir.clone());
        let mut session = Session::new("persist-test");
        session.add_message("user", "hello", None);
        session.add_message("assistant", "hi there", None);
        mgr.save_session(&session).await.unwrap();
    }

    // Second manager: load the session from disk.
    {
        let mgr = SessionManager::with_dir(platform.clone(), sessions_dir);
        let loaded = mgr.load_session("persist-test").await.unwrap();
        assert_eq!(loaded.key, "persist-test");
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0]["role"], "user");
        assert_eq!(loaded.messages[0]["content"], "hello");
        assert_eq!(loaded.messages[1]["role"], "assistant");
        assert_eq!(loaded.messages[1]["content"], "hi there");
    }

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

/// Test 3: Fixture config file loads correctly.
#[tokio::test]
async fn test_fixture_config_loads() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/config.json"
    );
    let content = tokio::fs::read_to_string(fixture_path).await.unwrap();
    let config: Config = serde_json::from_str(&content).unwrap();
    assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5");
    assert!(config.channels.telegram.enabled);
}

/// Test 4: Session fixture file parses correctly.
#[tokio::test]
async fn test_session_fixture_parses() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/session.jsonl"
    );
    let content = tokio::fs::read_to_string(fixture_path).await.unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // First line is metadata.
    assert!(lines.len() >= 6, "fixture should have at least 6 lines");
    let meta: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(meta["_type"], "metadata");

    // Remaining lines are messages.
    for line in &lines[1..] {
        let msg: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(msg.get("role").is_some());
        assert!(msg.get("content").is_some());
    }
}

/// Test 5: Tool fixture file parses as valid JSON.
#[tokio::test]
async fn test_tool_fixture_parses() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/tools/read_file.json"
    );
    let content = tokio::fs::read_to_string(fixture_path).await.unwrap();
    let tool_def: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(tool_def["type"], "function");
    assert_eq!(tool_def["function"]["name"], "read_file");
    assert!(tool_def["function"]["parameters"]["required"]
        .as_array()
        .unwrap()
        .contains(&serde_json::Value::String("path".into())));
}

/// Test 6: Session save/load round-trip preserves metadata.
#[tokio::test]
async fn test_session_roundtrip_preserves_metadata() {
    let dir = temp_dir("session_meta");
    let sessions_dir = dir.join("sessions");
    let platform = Arc::new(NativePlatform::new());
    let mgr = SessionManager::with_dir(platform, sessions_dir);

    let mut session = Session::new("meta-test");
    session
        .metadata
        .insert("agent".into(), serde_json::json!("test-bot"));
    session.last_consolidated = 3;
    session.add_message("user", "hi", None);
    mgr.save_session(&session).await.unwrap();

    let loaded = mgr.load_session("meta-test").await.unwrap();
    assert_eq!(loaded.last_consolidated, 3);
    assert_eq!(loaded.metadata["agent"], "test-bot");
    assert_eq!(loaded.messages.len(), 1);

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

/// Test 7: AppContext into_agent_loop produces a valid loop.
#[tokio::test]
async fn test_app_context_into_agent_loop() {
    let config = Config::default();
    let platform = Arc::new(NativePlatform::new());
    let ctx = AppContext::new(config, platform).await.unwrap();

    // Clone the bus sender before consuming the context.
    let _tx = ctx.bus().inbound_sender();

    let agent = ctx.into_agent_loop();
    assert_eq!(
        agent.config().defaults.model,
        "anthropic/claude-opus-4-5"
    );
}
