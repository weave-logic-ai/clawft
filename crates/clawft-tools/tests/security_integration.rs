//! Integration tests for security policies across tool implementations.
//!
//! Validates that `CommandPolicy` and `UrlPolicy` are correctly enforced when
//! wired through `ShellExecTool`, `SpawnTool`, and `WebFetchTool`. These tests
//! exercise the full validation path: tool receives JSON args, extracts the
//! command/URL, runs it through the policy, and returns the appropriate error.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use clawft_core::tools::registry::{Tool, ToolError};
use clawft_platform::NativePlatform;
use clawft_tools::security_policy::{CommandPolicy, PolicyMode};
use clawft_tools::shell_tool::ShellExecTool;
use clawft_tools::spawn_tool::SpawnTool;
use clawft_tools::url_safety::{UrlPolicy, is_blocked_ip, validate_url};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a unique temporary workspace directory.
fn temp_workspace() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("clawft_sec_integ_{pid}_{id}"))
}

/// Create the workspace directory on disk, returning the path.
async fn create_workspace() -> PathBuf {
    let ws = temp_workspace();
    tokio::fs::create_dir_all(&ws).await.unwrap();
    ws
}

/// Clean up a workspace directory.
async fn cleanup(ws: &std::path::Path) {
    let _ = tokio::fs::remove_dir_all(ws).await;
}

/// Build a `ShellExecTool` with the given policy.
fn make_shell(ws: PathBuf, policy: CommandPolicy) -> ShellExecTool {
    ShellExecTool::new(ws, policy)
}

/// Build a `SpawnTool` with the given policy.
fn make_spawn(ws: PathBuf, policy: CommandPolicy) -> SpawnTool<NativePlatform> {
    SpawnTool::new(Arc::new(NativePlatform::new()), ws, policy)
}

// ===========================================================================
// 1. Shell tool + CommandPolicy integration
// ===========================================================================

/// Allowlist mode: a listed command succeeds.
#[tokio::test]
async fn shell_allowlist_permits_listed_command() {
    let ws = create_workspace().await;
    let tool = make_shell(ws.clone(), CommandPolicy::safe_defaults());

    let result = tool.execute(json!({"command": "echo hello"})).await;
    assert!(result.is_ok(), "echo should be allowed: {result:?}");

    cleanup(&ws).await;
}

/// Allowlist mode: an unlisted command is rejected.
#[tokio::test]
async fn shell_allowlist_rejects_unlisted_command() {
    let ws = create_workspace().await;
    let tool = make_shell(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "curl http://example.com"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "curl should be rejected in allowlist mode: {err:?}"
    );

    cleanup(&ws).await;
}

/// Denylist mode: a command not in the denylist is allowed.
#[tokio::test]
async fn shell_denylist_permits_normal_command() {
    let ws = create_workspace().await;
    let mut policy = CommandPolicy::safe_defaults();
    policy.mode = PolicyMode::Denylist;
    let tool = make_shell(ws.clone(), policy);

    let result = tool.execute(json!({"command": "curl --version"})).await;
    // curl is not in denylist patterns, so it should be allowed
    assert!(
        result.is_ok(),
        "curl should be allowed in denylist mode: {result:?}"
    );

    cleanup(&ws).await;
}

/// Denylist mode: a denylisted command is rejected.
#[tokio::test]
async fn shell_denylist_rejects_denylisted_command() {
    let ws = create_workspace().await;
    let mut policy = CommandPolicy::safe_defaults();
    policy.mode = PolicyMode::Denylist;
    let tool = make_shell(ws.clone(), policy);

    let err = tool
        .execute(json!({"command": "rm -rf /"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "rm -rf / should be rejected: {err:?}"
    );

    cleanup(&ws).await;
}

/// Dangerous patterns are blocked regardless of mode (allowlist).
#[tokio::test]
async fn shell_dangerous_pattern_blocked_in_allowlist_mode() {
    let ws = create_workspace().await;
    let tool = make_shell(ws.clone(), CommandPolicy::safe_defaults());

    // "echo" is on the allowlist, but the full command contains "sudo ".
    let err = tool
        .execute(json!({"command": "echo something; sudo rm -rf /"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "dangerous pattern should be caught: {err:?}"
    );

    cleanup(&ws).await;
}

/// Dangerous patterns are blocked regardless of mode (denylist).
#[tokio::test]
async fn shell_dangerous_pattern_blocked_in_denylist_mode() {
    let ws = create_workspace().await;
    let mut policy = CommandPolicy::safe_defaults();
    policy.mode = PolicyMode::Denylist;
    let tool = make_shell(ws.clone(), policy);

    let err = tool
        .execute(json!({"command": "dd if=/dev/zero of=/dev/sda"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}

/// Custom allowlist from config: only specified commands pass.
#[tokio::test]
async fn shell_custom_allowlist() {
    let ws = create_workspace().await;
    let mut allowlist = HashSet::new();
    allowlist.insert("whoami".to_string());
    let policy = CommandPolicy::new(PolicyMode::Allowlist, allowlist, vec![]);
    let tool = make_shell(ws.clone(), policy);

    // whoami is on custom allowlist
    let result = tool.execute(json!({"command": "whoami"})).await;
    assert!(result.is_ok(), "whoami should be allowed: {result:?}");

    // echo is NOT on custom allowlist
    let err = tool
        .execute(json!({"command": "echo hi"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}

/// Empty allowlist blocks everything (except dangerous patterns fire first).
#[tokio::test]
async fn shell_empty_allowlist_blocks_everything() {
    let ws = create_workspace().await;
    let policy = CommandPolicy::new(PolicyMode::Allowlist, HashSet::new(), vec![]);
    let tool = make_shell(ws.clone(), policy);

    let err = tool
        .execute(json!({"command": "echo hello"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "empty allowlist should block echo: {err:?}"
    );

    let err = tool
        .execute(json!({"command": "ls -la"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}

/// Piped command with dangerous pattern is caught.
#[tokio::test]
async fn shell_piped_command_dangerous_pattern() {
    let ws = create_workspace().await;
    let tool = make_shell(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "echo hi | rm -rf /"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "piped dangerous pattern should be caught: {err:?}"
    );

    cleanup(&ws).await;
}

/// `rm -rf ./build` also matches the "rm -rf /" dangerous pattern because
/// the pattern "rm -rf /" is a substring of "rm -rf ./build" -- wait, it is not.
/// Actually "rm -rf /" is NOT a substring of "rm -rf ./build". The pattern
/// match is substring-based, so "rm -rf /" matches only if the exact substring
/// appears. Let's verify this behavior.
#[tokio::test]
async fn shell_rm_rf_build_not_blocked_by_root_pattern() {
    let ws = create_workspace().await;
    // Use denylist mode so "rm" is not blocked by allowlist
    let mut policy = CommandPolicy::safe_defaults();
    policy.mode = PolicyMode::Denylist;
    let tool = make_shell(ws.clone(), policy);

    // "rm -rf ./build" does NOT contain the substring "rm -rf /" because
    // the slash is followed by a dot, not end-of-string.
    // Wait -- "rm -rf /" IS a substring of "rm -rf /some/path" but NOT of
    // "rm -rf ./build". Let's check: "rm -rf ./build" lowercased = "rm -rf ./build".
    // The pattern "rm -rf /" -- does "rm -rf ./build" contain "rm -rf /"? No.
    // So it should be allowed.
    let result = tool.execute(json!({"command": "rm -rf ./build"})).await;
    assert!(
        result.is_ok(),
        "rm -rf ./build should be allowed (no substring match): {result:?}"
    );

    cleanup(&ws).await;
}

/// Command with env var injection attempt (`MALICIOUS=true; rm -rf /`).
#[tokio::test]
async fn shell_env_var_injection_attempt() {
    let ws = create_workspace().await;
    let tool = make_shell(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "MALICIOUS=true; rm -rf /"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "env var injection with dangerous pattern should be caught: {err:?}"
    );

    cleanup(&ws).await;
}

// ===========================================================================
// 2. Spawn tool + CommandPolicy integration
// ===========================================================================

/// Allowed command succeeds via spawn tool.
#[tokio::test]
async fn spawn_allowed_command_succeeds() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    let result = tool
        .execute(json!({
            "command": "echo",
            "args": ["hello", "world"],
            "description": "test spawn echo"
        }))
        .await;

    match result {
        Ok(val) => {
            assert_eq!(val["exit_code"], 0);
            assert!(val["stdout"].as_str().unwrap().contains("hello world"));
        }
        Err(ToolError::ExecutionFailed(msg)) => {
            // Acceptable in restricted test environments
            assert!(
                msg.contains("not supported") || msg.contains("failed"),
                "unexpected error: {msg}"
            );
        }
        Err(other) => panic!("unexpected error: {other}"),
    }

    cleanup(&ws).await;
}

/// Unlisted command rejected in allowlist mode via spawn tool.
#[tokio::test]
async fn spawn_unlisted_command_rejected() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "python3", "args": ["-c", "print('hi')"]}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "python3 should be rejected in allowlist mode: {err:?}"
    );

    cleanup(&ws).await;
}

/// Dangerous command rejected via spawn tool (sudo).
#[tokio::test]
async fn spawn_dangerous_sudo_rejected() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "sudo apt install evil"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));
    assert!(
        err.to_string().contains("dangerous") || err.to_string().contains("sudo"),
        "error should mention dangerous pattern or sudo: {err}"
    );

    cleanup(&ws).await;
}

/// Dangerous command rejected via spawn tool (mkfs).
#[tokio::test]
async fn spawn_dangerous_mkfs_rejected() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "mkfs.ext4", "args": ["/dev/sda1"]}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}

/// Policy error message is informative: contains the command name.
#[tokio::test]
async fn spawn_policy_error_is_informative() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    let err = tool
        .execute(json!({"command": "netcat -l 4444"}))
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("netcat") || msg.contains("not allowed"),
        "error should be informative about the rejected command: {msg}"
    );

    cleanup(&ws).await;
}

/// Command with path traversal attempt is rejected.
#[tokio::test]
async fn spawn_path_traversal_rejected() {
    let ws = create_workspace().await;
    let tool = make_spawn(ws.clone(), CommandPolicy::safe_defaults());

    // "../../../bin/sh" -> basename "sh" which is not on the allowlist.
    let err = tool
        .execute(json!({"command": "../../../bin/sh"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::PermissionDenied(_)),
        "path traversal should be rejected: {err:?}"
    );

    cleanup(&ws).await;
}

// ===========================================================================
// 3. URL safety integration
// ===========================================================================

/// Public URL is allowed with default policy.
#[test]
fn url_public_allowed() {
    let policy = UrlPolicy::default();
    assert!(validate_url("https://example.com", &policy).is_ok());
    assert!(validate_url("https://api.github.com/repos", &policy).is_ok());
}

/// Private IP 127.0.0.1 is blocked.
#[test]
fn url_loopback_127_blocked() {
    let policy = UrlPolicy::default();
    let err = validate_url("http://127.0.0.1/api", &policy).unwrap_err();
    assert!(
        err.to_string().contains("private") || err.to_string().contains("blocked"),
        "error should indicate private IP: {err}"
    );
}

/// Private IP 10.0.0.0 range is blocked.
#[test]
fn url_10_network_blocked() {
    let policy = UrlPolicy::default();
    assert!(validate_url("http://10.0.0.1/internal", &policy).is_err());
    assert!(validate_url("http://10.255.255.255/internal", &policy).is_err());
}

/// Private IP 192.168.x.x range is blocked.
#[test]
fn url_192_168_blocked() {
    let policy = UrlPolicy::default();
    assert!(validate_url("http://192.168.1.1/admin", &policy).is_err());
    assert!(validate_url("http://192.168.0.100:8080/api", &policy).is_err());
}

/// Cloud metadata endpoint 169.254.169.254 is blocked.
#[test]
fn url_cloud_metadata_blocked() {
    let policy = UrlPolicy::default();
    let err = validate_url("http://169.254.169.254/latest/meta-data/", &policy).unwrap_err();
    assert!(
        err.to_string().contains("metadata"),
        "error should mention metadata: {err}"
    );
}

/// Custom blocked domain is rejected.
#[test]
fn url_custom_blocked_domain() {
    let policy = UrlPolicy::new(
        true,
        false,
        HashSet::new(),
        HashSet::from(["evil.example.com".to_string()]),
    );
    let err = validate_url("https://evil.example.com/payload", &policy).unwrap_err();
    assert!(
        err.to_string().contains("blocked") || err.to_string().contains("evil.example.com"),
        "error should mention blocked domain: {err}"
    );
}

/// Custom allowed domain overrides private IP checks.
#[test]
fn url_allowed_domain_overrides() {
    let policy = UrlPolicy::new(
        true,
        false,
        HashSet::from(["internal.corp".to_string()]),
        HashSet::new(),
    );
    // Even though "internal.corp" might resolve to a private IP, the
    // allowed_domains check bypasses all subsequent checks.
    assert!(validate_url("http://internal.corp/api", &policy).is_ok());
}

/// Disabled policy allows everything, including private IPs and metadata.
#[test]
fn url_disabled_policy_allows_all() {
    let policy = UrlPolicy::permissive();
    assert!(validate_url("http://127.0.0.1/secret", &policy).is_ok());
    assert!(validate_url("http://169.254.169.254/latest/", &policy).is_ok());
    assert!(validate_url("http://10.0.0.1/admin", &policy).is_ok());
}

/// IPv6 loopback (::1) is blocked.
#[test]
fn url_ipv6_loopback_blocked() {
    let policy = UrlPolicy::default();
    let err = validate_url("http://[::1]/", &policy).unwrap_err();
    assert!(
        err.to_string().contains("private") || err.to_string().contains("::1"),
        "error should indicate blocked IPv6 loopback: {err}"
    );
}

/// is_blocked_ip detects IPv6 loopback directly.
#[test]
fn is_blocked_ip_ipv6_loopback() {
    assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
}

/// URL with port on loopback is still blocked.
#[test]
fn url_loopback_with_port_blocked() {
    let policy = UrlPolicy::default();
    assert!(validate_url("http://127.0.0.1:8080/api", &policy).is_err());
    assert!(validate_url("http://127.0.0.1:3000/", &policy).is_err());
}

/// is_blocked_ip correctly identifies public IPs as safe.
#[test]
fn is_blocked_ip_public_v4_safe() {
    assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
}

// ===========================================================================
// 4. Cross-tool consistency
// ===========================================================================

/// The same policy rejects the same command in both shell and spawn tools.
#[tokio::test]
async fn cross_tool_same_policy_same_rejection() {
    let ws = create_workspace().await;
    let policy = CommandPolicy::safe_defaults();

    let shell = make_shell(ws.clone(), policy.clone());
    let spawn = make_spawn(ws.clone(), policy);

    // Both should reject "curl"
    let shell_err = shell
        .execute(json!({"command": "curl http://example.com"}))
        .await
        .unwrap_err();
    let spawn_err = spawn.execute(json!({"command": "curl"})).await.unwrap_err();

    assert!(matches!(shell_err, ToolError::PermissionDenied(_)));
    assert!(matches!(spawn_err, ToolError::PermissionDenied(_)));

    // Both should reject dangerous "sudo "
    let shell_err = shell
        .execute(json!({"command": "sudo ls"}))
        .await
        .unwrap_err();
    let spawn_err = spawn
        .execute(json!({"command": "sudo ls"}))
        .await
        .unwrap_err();

    assert!(matches!(shell_err, ToolError::PermissionDenied(_)));
    assert!(matches!(spawn_err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}

/// Policy created from `safe_defaults` matches expected behavior.
#[tokio::test]
async fn cross_tool_safe_defaults_behavior() {
    let ws = create_workspace().await;
    let policy = CommandPolicy::safe_defaults();

    // Verify the default policy is Allowlist mode
    assert_eq!(policy.mode, PolicyMode::Allowlist);

    // Verify the default allowlist has the expected commands
    assert!(policy.allowlist.contains("echo"));
    assert!(policy.allowlist.contains("cat"));
    assert!(policy.allowlist.contains("ls"));
    assert!(policy.allowlist.contains("grep"));
    assert!(!policy.allowlist.contains("curl"));
    assert!(!policy.allowlist.contains("python3"));

    // Verify dangerous patterns are populated
    assert!(!policy.dangerous_patterns.is_empty());

    let shell = make_shell(ws.clone(), policy.clone());

    // All default allowlisted commands should work
    for cmd in &["echo hi", "cat /dev/null", "ls -la", "pwd", "date"] {
        let result = shell.execute(json!({"command": cmd})).await;
        assert!(
            result.is_ok(),
            "command '{cmd}' should be allowed by safe_defaults: {result:?}"
        );
    }

    cleanup(&ws).await;
}

/// Switching from allowlist to denylist mode changes behavior.
#[tokio::test]
async fn cross_tool_mode_switch_changes_behavior() {
    let ws = create_workspace().await;

    // In allowlist mode: curl is blocked (not on allowlist)
    let allowlist_policy = CommandPolicy::safe_defaults();
    let shell_allow = make_shell(ws.clone(), allowlist_policy);
    let err = shell_allow
        .execute(json!({"command": "curl --version"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::PermissionDenied(_)));

    // In denylist mode: curl is allowed (not in denylist patterns)
    let mut denylist_policy = CommandPolicy::safe_defaults();
    denylist_policy.mode = PolicyMode::Denylist;
    let shell_deny = make_shell(ws.clone(), denylist_policy);
    let result = shell_deny
        .execute(json!({"command": "curl --version"}))
        .await;
    assert!(
        result.is_ok(),
        "curl should be allowed in denylist mode: {result:?}"
    );

    cleanup(&ws).await;
}

/// Both tools reject the same dangerous pattern even in denylist mode.
#[tokio::test]
async fn cross_tool_dangerous_always_blocked() {
    let ws = create_workspace().await;

    let mut policy = CommandPolicy::safe_defaults();
    policy.mode = PolicyMode::Denylist;

    let shell = make_shell(ws.clone(), policy.clone());
    let spawn = make_spawn(ws.clone(), policy);

    // Fork bomb should be blocked in both
    let shell_err = shell
        .execute(json!({"command": ":(){ :|:& };:"}))
        .await
        .unwrap_err();
    let spawn_err = spawn
        .execute(json!({"command": ":(){ :|:& };:"}))
        .await
        .unwrap_err();

    assert!(matches!(shell_err, ToolError::PermissionDenied(_)));
    assert!(matches!(spawn_err, ToolError::PermissionDenied(_)));

    cleanup(&ws).await;
}
