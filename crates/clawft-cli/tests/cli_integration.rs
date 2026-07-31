//! CLI integration tests for the `weft` binary.
//!
//! These tests run the actual compiled binary via `std::process::Command`
//! to verify end-to-end CLI behavior. Each test spawns a fresh process
//! with `CLAWFT_CONFIG` pointing at a nonexistent path so the config
//! loader falls back to defaults (empty JSON object).

use std::process::Command;

/// Build a `Command` pointing at the compiled `weft` binary.
///
/// Sets `CLAWFT_CONFIG` to a nonexistent path so the config loader
/// detects that the file does not exist and falls back to defaults.
/// This prevents tests from accidentally loading a real user config
/// from `~/.clawft/config.json` or `~/.nanobot/config.json`.
fn weft_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_weft"));
    cmd.env("CLAWFT_CONFIG", "/tmp/.clawft-test-nonexistent-config.json");
    // Suppress tracing output so test assertions only match program output.
    cmd.env("RUST_LOG", "off");
    cmd
}

// ── 1. Version and help ─────────────────────────────────────────────────

#[test]
fn version_output() {
    let output = weft_bin()
        .arg("--version")
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("weft") && stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output should contain 'weft' and '{}', got: {stdout}",
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn help_output() {
    let output = weft_bin()
        .arg("--help")
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // WEFT-176 white-label brand: description is "WeftOS AI assistant CLI".
    // Keep a soft match so rebrands don't break the gate.
    assert!(
        stdout.contains("AI assistant CLI") || stdout.contains("weft"),
        "help output should contain the CLI description, got: {stdout}"
    );
    // WEFT-196: delegate debug surface is registered.
    assert!(
        stdout.contains("delegate"),
        "help should list delegate subcommand, got: {stdout}"
    );
}

#[test]
fn agent_help_output() {
    let output = weft_bin()
        .args(["agent", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("agent") || stdout.contains("Agent"),
        "agent help should mention 'agent', got: {stdout}"
    );
}

#[test]
fn unknown_subcommand_fails() {
    let output = weft_bin()
        .arg("this-subcommand-does-not-exist")
        .output()
        .expect("failed to run weft");

    assert!(
        !output.status.success(),
        "unknown subcommand should return non-zero exit code"
    );
}

// ── 2. Status command ───────────────────────────────────────────────────

#[test]
fn status_succeeds() {
    let output = weft_bin()
        .arg("status")
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft status should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("weft status"),
        "status output should contain header, got: {stdout}"
    );
}

#[test]
fn status_detailed_succeeds() {
    let output = weft_bin()
        .args(["status", "--detailed"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft status --detailed should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Detailed mode prints extra sections like Gateway and Channels.
    assert!(
        stdout.contains("Agent defaults"),
        "detailed status should contain 'Agent defaults', got: {stdout}"
    );
}

#[test]
fn status_rejects_unknown_flag() {
    // The status subcommand does not accept --config; clap should reject it.
    let output = weft_bin()
        .args(["status", "--bogus-flag-that-does-not-exist"])
        .output()
        .expect("failed to run weft");

    assert!(
        !output.status.success(),
        "unknown flag should cause a non-zero exit code"
    );
}

// ── 3. Config command ───────────────────────────────────────────────────

#[test]
fn config_show_outputs_json() {
    let output = weft_bin()
        .args(["config", "show"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft config show should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The output should be valid JSON (default config serialized).
    assert!(
        stdout.contains('{') && stdout.contains('}'),
        "config show should output JSON, got: {stdout}"
    );
}

#[test]
fn config_section_agents() {
    let output = weft_bin()
        .args(["config", "section", "agents"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft config section agents should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The agents section contains a "defaults" key with model, workspace, etc.
    assert!(
        stdout.contains("defaults") || stdout.contains("model"),
        "agents section should contain defaults or model, got: {stdout}"
    );
}

#[test]
fn config_section_nonexistent_handles_gracefully() {
    let output = weft_bin()
        .args(["config", "section", "nonexistent_section_xyz"])
        .output()
        .expect("failed to run weft");

    // The command itself exits 0 but prints an error to stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown section"),
        "nonexistent section should produce error on stderr, got: {stderr}"
    );
}

// ── 4. Completions ──────────────────────────────────────────────────────

#[test]
fn completions_bash() {
    let output = weft_bin()
        .args(["completions", "bash"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft completions bash should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bash") || stdout.contains("complete") || stdout.contains("COMPREPLY"),
        "bash completions should contain shell-specific keywords, got: {stdout}"
    );
}

#[test]
fn completions_zsh() {
    let output = weft_bin()
        .args(["completions", "zsh"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft completions zsh should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compdef") || stdout.contains("_describe"),
        "zsh completions should contain zsh-specific keywords, got: {stdout}"
    );
}

#[test]
fn completions_fish() {
    let output = weft_bin()
        .args(["completions", "fish"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft completions fish should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("complete -c"),
        "fish completions should contain 'complete -c', got: {stdout}"
    );
}

#[test]
fn completions_invalid_shell() {
    let output = weft_bin()
        .args(["completions", "notashell"])
        .output()
        .expect("failed to run weft");

    assert!(
        !output.status.success(),
        "unsupported shell should return non-zero exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported shell"),
        "error should mention 'unsupported shell', got: {stderr}"
    );
}

// ── 5. Sessions and memory ──────────────────────────────────────────────

#[test]
fn sessions_list_succeeds() {
    let output = weft_bin()
        .args(["sessions", "list"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft sessions list should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With no config, it may show "No sessions found." or list an empty table.
    assert!(
        stdout.contains("session") || stdout.contains("Session") || stdout.contains("No sessions"),
        "sessions list should produce session-related output, got: {stdout}"
    );
}

#[test]
fn memory_show_succeeds() {
    let output = weft_bin()
        .args(["memory", "show"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft memory show should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With no real memory file, expect either the content or "no memory entries".
    assert!(
        stdout.contains("Memory file") || stdout.contains("no memory") || stdout.contains("memory"),
        "memory show should produce memory-related output, got: {stdout}"
    );
}

// ── 6. Channels status ─────────────────────────────────────────────────

#[test]
fn channels_status_succeeds() {
    let output = weft_bin()
        .args(["channels", "status"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft channels status should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CHANNEL") || stdout.contains("telegram") || stdout.contains("slack"),
        "channels status should show channel information, got: {stdout}"
    );
}

// ── 7. Cron list ────────────────────────────────────────────────────────

#[test]
fn cron_list_succeeds() {
    let output = weft_bin()
        .args(["cron", "list"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft cron list should exit 0");
}

// ── 8. Gateway help ─────────────────────────────────────────────────────

#[test]
fn gateway_help_output() {
    let output = weft_bin()
        .args(["gateway", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "exit code should be 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("gateway") || stdout.contains("Gateway"),
        "gateway help should mention 'gateway', got: {stdout}"
    );
}

// ── 9. Subcommand --help for every top-level command ────────────────

#[test]
fn config_help_output() {
    let output = weft_bin()
        .args(["config", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft config --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("config") || stdout.contains("Config") || stdout.contains("configuration"),
        "config help should mention 'config', got: {stdout}"
    );
}

#[test]
fn memory_help_output() {
    let output = weft_bin()
        .args(["memory", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft memory --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("memory") || stdout.contains("Memory"),
        "memory help should mention 'memory', got: {stdout}"
    );
}

#[test]
fn sessions_help_output() {
    let output = weft_bin()
        .args(["sessions", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft sessions --help should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sessions") || stdout.contains("Sessions") || stdout.contains("session"),
        "sessions help should mention 'sessions', got: {stdout}"
    );
    assert!(
        stdout.contains("gc") || stdout.contains("Gc"),
        "sessions help should list gc subcommand, got: {stdout}"
    );
}

#[test]
fn sessions_gc_dry_run_succeeds() {
    let output = weft_bin()
        .args(["sessions", "gc", "--dry-run"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft sessions gc --dry-run should exit 0, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Session GC") || stdout.contains("dry-run") || stdout.contains("nothing"),
        "sessions gc dry-run should report status, got: {stdout}"
    );
}

#[test]
fn channels_help_output() {
    let output = weft_bin()
        .args(["channels", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft channels --help should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("channels") || stdout.contains("Channels") || stdout.contains("channel"),
        "channels help should mention 'channels', got: {stdout}"
    );
}

#[test]
fn cron_help_output() {
    let output = weft_bin()
        .args(["cron", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft cron --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cron") || stdout.contains("Cron") || stdout.contains("scheduled"),
        "cron help should mention 'cron', got: {stdout}"
    );
}

// ── 10. Invalid flag handling ───────────────────────────────────────

#[test]
fn invalid_top_level_flag_fails() {
    let output = weft_bin()
        .arg("--nonexistent")
        .output()
        .expect("failed to run weft");

    assert!(
        !output.status.success(),
        "unknown top-level flag should return non-zero exit code"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("error"),
        "stderr should indicate the flag is unrecognized, got: {stderr}"
    );
}

// ── 11. Agent message without valid config ──────────────────────────

#[test]
fn agent_message_without_config_does_not_panic() {
    // Sending a message with no valid config may block waiting for the LLM,
    // so we spawn the process and give it a short window to check that it
    // at least starts without panicking. We use `spawn` + `wait_with_output`
    // with a timeout rather than `.output()` which blocks indefinitely.
    use std::time::{Duration, Instant};

    let mut child = weft_bin()
        .args(["agent", "-m", "test message"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn weft");

    let deadline = Instant::now() + Duration::from_secs(5);
    let exited = loop {
        if Instant::now() >= deadline {
            break false;
        }
        match child.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break false,
        }
    };

    if exited {
        // Process exited within the timeout -- verify no panic in output.
        let output = child.wait_with_output().expect("failed to read output");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stdout}{stderr}");
        assert!(
            !combined.contains("panicked at") && !combined.contains("RUST_BACKTRACE"),
            "agent -m should not panic without config, got: {combined}"
        );
    } else {
        // Process is still running (blocking on LLM call). That's acceptable
        // -- it means it didn't panic during bootstrap. Kill it cleanly.
        let _ = child.kill();
        let _ = child.wait();
    }
}

// ── 12. Completions subcommand help ─────────────────────────────────

#[test]
fn completions_help_output() {
    let output = weft_bin()
        .args(["completions", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(
        output.status.success(),
        "weft completions --help should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("completions")
            || stdout.contains("Completions")
            || stdout.contains("shell"),
        "completions help should mention 'completions' or 'shell', got: {stdout}"
    );
}

// ── 13. Memory subcommand variants ──────────────────────────────────

#[test]
fn memory_history_succeeds() {
    let output = weft_bin()
        .args(["memory", "history"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft memory history should exit 0");
}

#[test]
fn memory_search_succeeds() {
    let output = weft_bin()
        .args(["memory", "search", "test query"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft memory search should exit 0");
}

// ── 14. MCP add / list / remove (WEFT-188) ───────────────────────────

#[test]
fn mcp_help_output() {
    let output = weft_bin()
        .args(["mcp", "--help"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft mcp --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("add") && stdout.contains("list") && stdout.contains("remove"),
        "mcp help should list add/list/remove, got: {stdout}"
    );
}

#[test]
fn mcp_list_empty_succeeds() {
    // CLAWFT_CONFIG points at nonexistent path (see weft_bin); list should
    // still exit 0 with a friendly empty message.
    let output = weft_bin()
        .args(["mcp", "list"])
        .output()
        .expect("failed to run weft");

    assert!(output.status.success(), "weft mcp list should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No MCP servers") || stdout.contains("MCP server"),
        "mcp list should mention MCP servers, got: {stdout}"
    );
}

#[test]
fn mcp_add_list_remove_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("config.json");
    std::fs::write(&cfg, "{}\n").expect("write empty config");
    let cfg_str = cfg.to_str().expect("utf8 path");

    // add via --command / --arg / --env
    let add = weft_bin()
        .args([
            "mcp",
            "add",
            "demo-srv",
            "--command",
            "npx",
            "--arg",
            "-y",
            "--arg",
            "@example/mcp-server",
            "--env",
            "TOKEN=secret",
            "--internal-only=false",
            "--config",
            cfg_str,
        ])
        .output()
        .expect("mcp add");
    assert!(
        add.status.success(),
        "mcp add failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let add_out = String::from_utf8_lossy(&add.stdout);
    assert!(
        add_out.contains("Added MCP server") || add_out.contains("demo-srv"),
        "add output unexpected: {add_out}"
    );
    assert!(
        add_out.contains("Hint:") || add_out.contains("reload") || add_out.contains("Daemon"),
        "add should surface reload hint or daemon message: {add_out}"
    );

    // file should contain the server
    let written = std::fs::read_to_string(&cfg).expect("read config");
    assert!(
        written.contains("demo-srv") && written.contains("npx"),
        "config should contain demo-srv: {written}"
    );

    // list
    let list = weft_bin()
        .args(["mcp", "list", "--config", cfg_str])
        .output()
        .expect("mcp list");
    assert!(list.status.success(), "mcp list failed");
    let list_out = String::from_utf8_lossy(&list.stdout);
    assert!(
        list_out.contains("demo-srv"),
        "list should show demo-srv: {list_out}"
    );

    // remove
    let remove = weft_bin()
        .args(["mcp", "remove", "demo-srv", "--config", cfg_str])
        .output()
        .expect("mcp remove");
    assert!(
        remove.status.success(),
        "mcp remove failed: {}",
        String::from_utf8_lossy(&remove.stderr)
    );
    let after = std::fs::read_to_string(&cfg).expect("read config after remove");
    // name should no longer be a server key (may still appear only if leftover elsewhere)
    assert!(
        !after.contains("\"demo-srv\"") && !after.contains("demo-srv"),
        "config should no longer list demo-srv: {after}"
    );
}

#[test]
fn mcp_add_trailing_command_style() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("config.json");
    std::fs::write(&cfg, "{}\n").expect("write empty config");
    let cfg_str = cfg.to_str().expect("utf8 path");

    let add = weft_bin()
        .args([
            "mcp",
            "add",
            "trail-srv",
            "--config",
            cfg_str,
            "--",
            "uvx",
            "my-mcp",
        ])
        .output()
        .expect("mcp add trailing");
    assert!(
        add.status.success(),
        "mcp add -- failed: stdout={} stderr={}",
        String::from_utf8_lossy(&add.stdout),
        String::from_utf8_lossy(&add.stderr)
    );
    let written = std::fs::read_to_string(&cfg).expect("read config");
    assert!(
        written.contains("trail-srv") && written.contains("uvx"),
        "trailing add should write uvx server: {written}"
    );
}

#[test]
fn mcp_add_url_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("config.json");
    std::fs::write(&cfg, "{}\n").expect("write empty config");
    let cfg_str = cfg.to_str().expect("utf8 path");

    let add = weft_bin()
        .args([
            "mcp",
            "add",
            "remote",
            "--url",
            "https://mcp.example.com/sse",
            "--config",
            cfg_str,
        ])
        .output()
        .expect("mcp add url");
    assert!(
        add.status.success(),
        "mcp add --url failed: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let written = std::fs::read_to_string(&cfg).expect("read config");
    assert!(
        written.contains("https://mcp.example.com/sse"),
        "url should be persisted: {written}"
    );
}

#[test]
fn mcp_remove_missing_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("config.json");
    std::fs::write(&cfg, r#"{"tools":{"mcpServers":{}}}"#).expect("write config");
    let cfg_str = cfg.to_str().expect("utf8 path");

    let remove = weft_bin()
        .args(["mcp", "remove", "does-not-exist", "--config", cfg_str])
        .output()
        .expect("mcp remove missing");
    assert!(
        !remove.status.success(),
        "removing missing server should fail"
    );
}

#[test]
fn mcp_add_requires_transport() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("config.json");
    std::fs::write(&cfg, "{}\n").expect("write config");
    let cfg_str = cfg.to_str().expect("utf8 path");

    let add = weft_bin()
        .args(["mcp", "add", "empty", "--config", cfg_str])
        .output()
        .expect("mcp add empty");
    assert!(
        !add.status.success(),
        "add without command/url should fail"
    );
}

// ── WEFT-196: weft delegate debug ───────────────────────────────────────

#[test]
fn delegate_debug_human_output() {
    let output = weft_bin()
        .args([
            "delegate",
            "debug",
            "deploy to production",
            "--claude-available",
            "true",
        ])
        .output()
        .expect("failed to run weft delegate debug");

    assert!(
        output.status.success(),
        "delegate debug should exit 0, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("weft delegate debug"),
        "should print header, got: {stdout}"
    );
    assert!(
        stdout.contains("Target:"),
        "should print Target, got: {stdout}"
    );
    assert!(
        stdout.contains("Complexity:"),
        "should print Complexity, got: {stdout}"
    );
    assert!(
        stdout.contains("Reason:"),
        "should print Reason, got: {stdout}"
    );
}

#[test]
fn delegate_debug_json_output() {
    let output = weft_bin()
        .args([
            "delegate",
            "debug",
            "hi",
            "--json",
            "--claude-available",
            "false",
        ])
        .output()
        .expect("failed to run weft delegate debug --json");

    assert!(
        output.status.success(),
        "delegate debug --json should exit 0, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");
    assert_eq!(value["task"], "hi");
    assert_eq!(value["target"], "local");
    assert_eq!(value["claude_available"], false);
    assert!(value.get("complexity").is_some());
    assert!(value.get("reason").is_some());
}

#[test]
fn delegate_help_lists_debug() {
    let output = weft_bin()
        .args(["delegate", "--help"])
        .output()
        .expect("failed to run weft delegate --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("debug"),
        "delegate help should list debug, got: {stdout}"
    );
}
