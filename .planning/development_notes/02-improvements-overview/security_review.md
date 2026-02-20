# Security Review: Phase 5 Improvements Sprint

**Date**: 2026-02-20
**Branch**: `sprint/phase-5` (14 commits, 268 files, ~47K insertions)
**Base Commit**: `69c6112`
**Reviewer**: Automated security analysis (3-phase: discovery, validation, false-positive filtering)
**Scope**: 36 source files across clawft-wasm, clawft-core, clawft-plugin, clawft-services, clawft-cli, clawft-channels, clawft-types

---

## Executive Summary

Two HIGH-severity vulnerabilities were confirmed with confidence >= 8/10 after false-positive filtering. Five additional true-positive findings at confidence 6-7 are documented for future hardening but did not meet the reporting threshold for immediate action.

| # | Severity | Category | File | Confidence | Status |
|---|----------|----------|------|-----------|--------|
| 1 | HIGH | `path_traversal` | `clawft-plugin/src/manifest.rs:216` | 8/10 | **Fixed** (`aab3271`) |
| 2 | HIGH | `privilege_escalation` | `clawft-core/src/agent/sandbox.rs:217` | 9/10 | **Fixed** (`aab3271`) |

---

## Confirmed Vulnerabilities

### Vuln 1: Path Traversal via Unsanitized Plugin Manifest `id`

**File**: `crates/clawft-plugin/src/manifest.rs:216`
**Severity**: HIGH
**Confidence**: 8/10
**Category**: `path_traversal`

**Description**:

`PluginManifest::validate()` only checks that the `id` field is non-empty. It does not reject path-special characters (`/`, `\`, `..`, null bytes). The `id` field is subsequently used as a filesystem path component in `PermissionStore::save()` (`crates/clawft-wasm/src/permission_store.rs:66-82`), which calls `self.base_dir.join(plugin_id)` followed by `create_dir_all` and `fs::write` -- allowing directory creation and file writes outside the intended base directory.

**Exploit Scenario**:

An attacker crafts a plugin manifest with `id: "../../.config/autostart/backdoor"`. When the plugin's permissions are saved after user approval, `PermissionStore::save()` creates directories and writes JSON at `{base_dir}/../../.config/autostart/backdoor/approved_permissions.json`, placing attacker-influenced content outside the plugin permission directory.

**Affected Code Path**:

```
PluginManifest.id (untrusted input)
  -> PermissionStore::save(plugin_id)
    -> self.base_dir.join(plugin_id)  // no sanitization
    -> std::fs::create_dir_all(&dir)  // creates traversed path
    -> std::fs::write(path, json)     // writes outside base_dir
```

**Recommendation**:

Add allowlist validation to `PluginManifest::validate()`:

```rust
if !self.id.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
    return Err(PluginError::LoadFailed(
        "manifest: id must contain only [a-zA-Z0-9._-]".into()
    ));
}
```

Additionally, enforce a length bound (e.g., 128 chars) and add a post-join canonicalization check in `PermissionStore::save()` as defense-in-depth.

---

### Vuln 2: Linux OS Sandbox is a No-Op Stub with Deceptive Logging

**File**: `crates/clawft-core/src/agent/sandbox.rs:217`
**Severity**: HIGH
**Confidence**: 9/10
**Category**: `privilege_escalation`

**Description**:

`apply_linux_sandbox()` emits `tracing::info!("applying Linux OS sandbox (seccomp + landlock)")` and immediately returns `Ok(())` without invoking any seccomp-BPF filter or landlock ruleset. On Linux, `SandboxType::OsSandbox` and `SandboxType::Combined` both call this function, which means agents configured for OS-level sandboxing have **no kernel-level enforcement** despite logs indicating otherwise. Any native tool invocation or plugin capability that bypasses the WASM layer operates with full process privileges.

**Exploit Scenario**:

An operator deploys clawft on Linux with `sandbox_type: OsSandbox` for an agent handling untrusted plugin code. The INFO log confirms "applying Linux OS sandbox" -- the operator's threat model assumes `execve`, arbitrary filesystem access, and raw network sockets are blocked by seccomp. In reality, a compromised plugin can spawn subprocesses, read `/proc/self/environ`, access any file the process user can read, and open network connections without restriction.

**Affected Code Path**:

```
SandboxEnforcer::apply()
  -> match sandbox_type {
       OsSandbox | Combined => apply_linux_sandbox(policy)
     }
  -> apply_linux_sandbox():
       tracing::info!("applying Linux OS sandbox")  // DECEPTIVE
       Ok(())  // NO seccomp, NO landlock
```

**Recommendation**:

Option A (implement): Add actual seccomp-BPF filtering via `seccompiler` or `libseccomp` crate, and landlock restrictions via the `landlock` crate.

Option B (honest stub): Change the log level to `warn!` and the message to explicitly state the sandbox is **not yet implemented**. Return `Err("OS sandbox not yet implemented")` for `OsSandbox` type so callers fall back to the WASM sandbox which does provide real enforcement.

---

## Below-Threshold Findings (Confidence 6-7)

These findings are true positives but did not meet the >= 8/10 confidence threshold for the primary report. They are documented here for future hardening sprints.

### BT-1: PermissionStore Path Traversal (Downstream of Vuln 1)

**File**: `crates/clawft-wasm/src/permission_store.rs:58-82`
**Confidence**: 7/10
**Category**: `path_traversal`

`PermissionStore::save()` joins `plugin_id` directly to `base_dir` without sanitization. This is the downstream impact of Vuln 1 -- fixing manifest `id` validation closes this path. As defense-in-depth, `PermissionStore` should independently validate or canonicalize the constructed path.

### BT-2: Remote Skill Install Name Traversal

**File**: `crates/clawft-cli/src/commands/skills_cmd.rs:546-561`
**Confidence**: 6/10
**Category**: `path_traversal`

`skill.name` from a remote ClawHub registry response is used directly in `user_dir.join(&skill.name)` without sanitization. A malicious registry entry with `name: "../../../.bashrc"` could write outside the skills directory. Requires a compromised or malicious registry server.

**Recommendation**: Validate `skill.name` against `[a-zA-Z0-9._-]+` before joining to the install path.

### BT-3: FNV-1a Fallback Signature (Non-Cryptographic)

**File**: `crates/clawft-cli/src/commands/skills_cmd.rs:892-907`
**Confidence**: 7/10
**Category**: `authentication_bypass`

When the `signing` feature is not compiled (it is not in default features), skill signing falls back to `SimpleHasher` (FNV-1a, 64-bit). FNV-1a provides no collision resistance or preimage resistance. The "signature" is trivially forgeable. The `signing` feature should be made a default feature or the fallback should refuse to sign.

**Recommendation**: Either add `signing` to default features in `clawft-core/Cargo.toml`, or remove the fallback entirely and return an error when signing is requested without the feature.

### BT-4: Nested Token Bypass in `sanitize_skill_instructions`

**File**: `crates/clawft-core/src/security/mod.rs:314-338`
**Confidence**: 7/10
**Category**: `prompt_injection`

Single-pass `str::replace` per injection token allows reconstruction attacks where removing an inner token recreates an outer one (e.g., `"<sy<system>stem>"` -> `"<system>"`). The exact exploitability depends on token ordering.

**Recommendation**: Replace with a loop that repeats until the string stabilizes, or reject inputs containing any injection token instead of stripping.

### BT-5: Symlink Following in `copy_dir_recursive`

**File**: `crates/clawft-cli/src/commands/skills_cmd.rs:704-717`
**Confidence**: 3/10 (effectively false positive)
**Category**: `path_traversal`

`copy_dir_recursive` follows symlinks during copy. A skill directory containing a symlink to `/etc/passwd` would copy its contents into the destination. Risk is low -- requires attacker control of the source directory and copies to a user-controlled destination.

---

## False Positives (Filtered)

The following findings were investigated and determined to be false positives or excluded by the review rules:

| Finding | Reason for Exclusion |
|---------|---------------------|
| SandboxPolicy `starts_with` bypass | Policy layer only; WASM enforcement does canonicalize (confidence 2) |
| `validate_agent_id` traversal | Rejects `/`, `\`, `\0`, and `..` -- adequate for its use case (confidence 2) |
| FlowDelegator prompt injection | Excluded: "user-controlled content in AI prompts is not a vulnerability" (confidence 1) |
| BridgeConfig command injection | Config from user's own home directory is trusted (confidence 2) |
| MCP auto-discover commands | Only reads from `~/.clawft/mcp/` (user home), not workspace dirs (confidence 3) |
| Credential leak in API error body | Error body not directly logged; Anthropic API doesn't echo credentials (confidence 2) |
| SSRF via DNS rebinding | WASM HTTP validation is pre-flight only; no evidence it's the sole SSRF boundary (confidence 3) |

---

## Methodology

### Phase 1: Discovery

Two parallel agents scanned 36 source files across security-critical domains:
- **Agent 1**: WASM sandbox, plugin system, cryptographic signing, security policy (16 files)
- **Agent 2**: Network channels, ClawHub client, MCP bridge, delegation, CLI commands (20 files)

### Phase 2: False-Positive Filtering

Three parallel validation agents read the actual source code for each finding:
- **Validator 1**: 6 path traversal findings
- **Validator 2**: 5 injection/command findings
- **Validator 3**: 3 crypto/sandbox findings

### Phase 3: Confidence Filtering

Findings with validated confidence < 8/10 were moved to the below-threshold section. Only findings with confidence >= 8/10 appear in the primary vulnerability report.

### Scoring

- **8-10**: High confidence, true vulnerability with clear exploit path -> Primary report
- **5-7**: Medium confidence, true positive but lower exploitability -> Below-threshold section
- **1-4**: Low confidence, false positive or excluded by rules -> Filtered section

---

## Remediation Priority

| Priority | Vuln | Effort | Impact |
|----------|------|--------|--------|
| **P0** | Vuln 1 (manifest id validation) | Low (5-10 lines) | Closes path traversal for all plugin ID consumers |
| **P0** | Vuln 2 (sandbox stub) | Medium (honest stub) / High (full impl) | Prevents false security posture on Linux |
| **P1** | BT-3 (FNV-1a fallback) | Low (add feature default or remove fallback) | Prevents forgeable signatures |
| **P1** | BT-2 (remote skill name) | Low (add name validation) | Prevents traversal from malicious registry |
| **P2** | BT-4 (token sanitization) | Low (loop until stable) | Hardens prompt injection defense |
| **P2** | BT-1 (PermissionStore) | Low (canonicalize check) | Defense-in-depth for Vuln 1 |
