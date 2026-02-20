# Phase K-Security: Per-Agent Sandbox & Security Plugin

> **Element:** 10 -- Deployment & Community
> **Phase:** K-Security (K3, K3a)
> **Timeline:** Week 9-11
> **Priority:** P0 (K3 sandbox), P1 (K3a security plugin)
> **Crates:** `clawft-security` (NEW), `clawft-core`, `clawft-cli`, `clawft-wasm`, `clawft-plugin`
> **Dependencies IN:** 04/C2 (WASM host + PluginSandbox), 09/L2 (per-agent workspace isolation + config.toml)
> **Blocks:** K4 (ClawHub requires `weft security scan` for skill install), K5 (benchmark suite assumes sandbox overhead is measured)
> **Status:** Planning
> **Orchestrator Ref:** `10-deployment-community/00-orchestrator.md` Phase K-Security

---

## 1. Overview

This phase delivers two complementary security layers:

1. **K3 -- Per-Agent Sandbox**: A runtime enforcement layer that maps per-agent configuration (`~/.clawft/agents/<id>/config.toml`) to a `SandboxPolicy` struct, then enforces that policy via WASM sandboxing (cross-platform) and OS-level sandboxing (seccomp + landlock on Linux). Every agent's tool access is restricted according to its declared permissions, with all sandbox decisions audit-logged via structured tracing.

2. **K3a -- Security Plugin**: A standalone audit engine with 50+ security checks across 10 categories, plus hardening modules that auto-apply seccomp/landlock profiles, background monitors for anomalous behavior detection, and CLI integration (`weft security scan`, `weft security audit`, `weft security harden`). The security plugin runs automatically during `weft skill install` and blocks activation of skills with Critical/High findings by default.

The sandbox layer sits between the agent loop (Element 09/L2 per-agent workspace) and tool execution. When an agent invokes a tool, the sandbox intercepts the call, checks the agent's `SandboxPolicy`, and either permits or denies the operation. This is conceptually distinct from C2's WASM PluginSandbox (which restricts WASM *plugins*) -- K3 restricts *agents*, regardless of whether they execute native tools or WASM plugins.

---

## 2. Specification

### 2.1 K3: SandboxPolicy Struct

**New file:** `crates/clawft-plugin/src/sandbox.rs`

The `SandboxPolicy` is the runtime representation of an agent's security constraints, constructed from the agent's `config.toml` at agent startup.

```rust
use std::collections::HashSet;
use std::path::PathBuf;

/// Runtime sandbox policy for a single agent.
///
/// Constructed from `~/.clawft/agents/<id>/config.toml` at agent startup.
/// Immutable after construction -- if config changes, the agent must be
/// restarted to pick up new policy.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    /// Agent identifier (for audit logging).
    pub agent_id: String,
    /// Sandbox enforcement type.
    pub sandbox_type: SandboxType,
    /// Tools this agent is allowed to invoke (empty = all denied).
    pub allowed_tools: Vec<String>,
    /// Tools explicitly denied (takes precedence over allowed_tools).
    pub denied_tools: Vec<String>,
    /// Filesystem access rules.
    pub filesystem_rules: Vec<FsRule>,
    /// Network access rules.
    pub network_rules: Vec<NetRule>,
    /// Environment variable access rules.
    pub env_rules: Vec<EnvRule>,
}

/// Sandbox enforcement type.
///
/// Default selection logic:
/// - WASM plugins: `SandboxType::Wasm`
/// - Native tools on Linux: `SandboxType::OsSandbox`
/// - Native tools on macOS/Windows: `SandboxType::Wasm` (with warning)
/// - Explicit `sandbox_type = "none"` in config.toml: `SandboxType::None`
///   (requires `privileged = true` in agent config)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxType {
    /// No sandbox. Only for privileged agents with explicit config.
    /// Requires `privileged = true` in agent config.toml.
    None,
    /// WASM sandbox via wasmtime WASI capabilities.
    /// Cross-platform. Restricts filesystem, network, environment access.
    Wasm,
    /// OS-level sandbox: seccomp + landlock (Linux only).
    /// Restricts syscalls and filesystem access at the kernel level.
    OsSandbox,
    /// Both WASM + OS sandbox layers applied simultaneously.
    /// Maximum isolation. Used for untrusted third-party skills.
    Combined,
}

/// Filesystem access rule.
#[derive(Debug, Clone)]
pub struct FsRule {
    /// Canonicalized path prefix that access applies to.
    pub path: PathBuf,
    /// Whether write access is permitted (false = read-only).
    pub writable: bool,
}

/// Network access rule.
#[derive(Debug, Clone)]
pub struct NetRule {
    /// Domain or IP pattern (e.g., "api.example.com", "*.openai.com").
    pub host_pattern: String,
    /// Allowed ports. Empty = all ports allowed.
    pub allowed_ports: Vec<u16>,
    /// Whether HTTPS is required (default: true).
    pub require_tls: bool,
}

/// Environment variable access rule.
#[derive(Debug, Clone)]
pub struct EnvRule {
    /// Variable name or prefix pattern (e.g., "CLAWFT_*").
    pub pattern: String,
    /// Whether write (set/unset) is permitted (false = read-only).
    pub writable: bool,
}
```

### 2.2 K3: Secure Defaults

The default sandbox type is **NOT** `None`. Platform-specific defaults:

| Platform | WASM Plugins | Native Tools | Rationale |
|----------|-------------|-------------|-----------|
| Linux | `Wasm` | `OsSandbox` | seccomp + landlock available |
| macOS | `Wasm` | `Wasm` | No seccomp/landlock; warning emitted |
| Windows | `Wasm` | `Wasm` | No seccomp/landlock; warning emitted |

To select `SandboxType::None`, the agent config must explicitly set:
```toml
[security]
sandbox_type = "none"
privileged = true
```

Without `privileged = true`, setting `sandbox_type = "none"` is rejected at config parse time with an error.

### 2.3 K3: Config-to-Policy Mapping

**Config file location:** `~/.clawft/agents/<id>/config.toml`

Example agent config (integration with L2 from Element 09):
```toml
[agent]
id = "analyst-01"
name = "Data Analyst"

[security]
sandbox_type = "wasm"          # "none" | "wasm" | "os" | "combined"
# privileged = false           # Required for sandbox_type = "none"

[tools]
allowed = ["read_file", "write_file", "web_fetch", "memory_read", "memory_write"]
denied = ["exec_shell", "spawn"]

[filesystem]
rules = [
    { path = "~/.clawft/agents/analyst-01/workspace", writable = true },
    { path = "~/data/readonly-datasets", writable = false },
]

[network]
rules = [
    { host = "api.openai.com", ports = [443], require_tls = true },
    { host = "*.internal.corp", ports = [], require_tls = false },
]

[environment]
rules = [
    { pattern = "CLAWFT_*", writable = false },
    { pattern = "OPENAI_API_KEY", writable = false },
]
```

**Mapping function:**

```rust
impl SandboxPolicy {
    /// Construct a SandboxPolicy from an agent's config.toml.
    ///
    /// # Errors
    /// - Returns error if `sandbox_type = "none"` without `privileged = true`
    /// - Returns error if filesystem paths cannot be canonicalized
    /// - Emits warning on non-Linux when `sandbox_type = "os"` or "combined"
    pub fn from_agent_config(
        agent_id: &str,
        config: &AgentSecurityConfig,
    ) -> Result<Self, SandboxConfigError> {
        let sandbox_type = match config.sandbox_type.as_deref() {
            Some("none") => {
                if !config.privileged.unwrap_or(false) {
                    return Err(SandboxConfigError::NoneRequiresPrivileged);
                }
                tracing::warn!(
                    agent_id = agent_id,
                    "Agent configured with SandboxType::None -- no sandbox enforcement"
                );
                SandboxType::None
            }
            Some("wasm") => SandboxType::Wasm,
            Some("os") => {
                #[cfg(not(target_os = "linux"))]
                {
                    tracing::warn!(
                        agent_id = agent_id,
                        "OS sandbox unavailable on this platform; falling back to WASM sandbox"
                    );
                    SandboxType::Wasm
                }
                #[cfg(target_os = "linux")]
                SandboxType::OsSandbox
            }
            Some("combined") => {
                #[cfg(not(target_os = "linux"))]
                {
                    tracing::warn!(
                        agent_id = agent_id,
                        "OS sandbox unavailable on this platform; \
                         Combined mode falling back to WASM-only"
                    );
                    SandboxType::Wasm
                }
                #[cfg(target_os = "linux")]
                SandboxType::Combined
            }
            None => Self::platform_default(),
            Some(other) => return Err(SandboxConfigError::UnknownType(other.to_string())),
        };

        // ... construct FsRule, NetRule, EnvRule from config sections ...

        Ok(Self {
            agent_id: agent_id.to_string(),
            sandbox_type,
            allowed_tools: config.tools_allowed.clone().unwrap_or_default(),
            denied_tools: config.tools_denied.clone().unwrap_or_default(),
            filesystem_rules: /* parsed from config.filesystem */,
            network_rules: /* parsed from config.network */,
            env_rules: /* parsed from config.environment */,
        })
    }

    /// Platform-specific default sandbox type.
    fn platform_default() -> SandboxType {
        #[cfg(target_os = "linux")]
        { SandboxType::OsSandbox }
        #[cfg(not(target_os = "linux"))]
        { SandboxType::Wasm }
    }
}
```

### 2.4 K3: WASM Sandbox Layer

The WASM sandbox layer extends C2's `PluginSandbox` (from Element 04) to agent-level enforcement. It uses wasmtime's WASI capabilities to restrict:

1. **Filesystem access**: Only paths listed in `filesystem_rules` are mapped into the WASI preopened directories. All other paths are invisible to the sandboxed agent.
2. **Network access**: Outbound HTTP is filtered through the `network_rules`. Each request passes through `validate_http_request()` (from C2.4) with the agent's `NetRule` list as the allowlist.
3. **Environment variables**: Only variables matching `env_rules` patterns are visible inside the sandbox. The WASI environment is constructed with only the allowed variables.

**Integration point with C2:**
```rust
// crates/clawft-core/src/agent/sandbox.rs (NEW)

use clawft_plugin::sandbox::{SandboxPolicy, SandboxType};
use clawft_wasm::sandbox::PluginSandbox;

/// Agent-level sandbox enforcer.
///
/// Wraps tool execution in the appropriate sandbox layer(s)
/// based on the agent's SandboxPolicy.
pub struct AgentSandbox {
    policy: SandboxPolicy,
    /// WASM sandbox (when SandboxType includes Wasm).
    wasm_sandbox: Option<PluginSandbox>,
    /// OS sandbox handle (when SandboxType includes OsSandbox, Linux only).
    #[cfg(target_os = "linux")]
    os_sandbox: Option<OsSandboxHandle>,
}

impl AgentSandbox {
    /// Check whether a tool invocation is permitted.
    ///
    /// Returns Ok(()) if allowed, Err with denial reason if not.
    /// All decisions are audit-logged regardless of outcome.
    pub fn check_tool_access(
        &self,
        tool_name: &str,
        agent_id: &str,
    ) -> Result<(), SandboxDenied> {
        // Step 1: Check denied_tools (explicit deny takes precedence)
        if self.policy.denied_tools.contains(&tool_name.to_string()) {
            audit_log(agent_id, tool_name, "denied", "explicit_deny_list");
            return Err(SandboxDenied::ToolDenied(tool_name.to_string()));
        }

        // Step 2: Check allowed_tools (empty = deny all)
        if !self.policy.allowed_tools.is_empty()
            && !self.policy.allowed_tools.contains(&tool_name.to_string())
        {
            audit_log(agent_id, tool_name, "denied", "not_in_allow_list");
            return Err(SandboxDenied::ToolNotAllowed(tool_name.to_string()));
        }

        // Step 3: Log the allow decision
        audit_log(agent_id, tool_name, "allowed", "policy_check_passed");
        Ok(())
    }
}
```

### 2.5 K3: OS Sandbox Layer (Linux Only)

**New file:** `crates/clawft-core/src/agent/os_sandbox.rs`

The OS sandbox layer uses `seccomp` and `landlock` to enforce kernel-level restrictions on the agent process. This layer is only available on Linux and is gated behind `#[cfg(target_os = "linux")]`.

```rust
#[cfg(target_os = "linux")]
pub mod os_sandbox {
    use landlock::{
        Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr, RulesetStatus, ABI,
    };

    /// OS-level sandbox handle.
    ///
    /// Once applied, the seccomp/landlock rules cannot be removed
    /// for the lifetime of the thread.
    pub struct OsSandboxHandle {
        /// Whether landlock was successfully applied.
        landlock_active: bool,
        /// Whether seccomp was successfully applied.
        seccomp_active: bool,
    }

    impl OsSandboxHandle {
        /// Apply OS-level sandbox restrictions.
        ///
        /// Must be called early in the agent thread lifecycle,
        /// before any tool execution begins.
        pub fn apply(policy: &SandboxPolicy) -> Result<Self, OsSandboxError> {
            let landlock_active = Self::apply_landlock(policy)?;
            let seccomp_active = Self::apply_seccomp(policy)?;
            Ok(Self { landlock_active, seccomp_active })
        }

        fn apply_landlock(policy: &SandboxPolicy) -> Result<bool, OsSandboxError> {
            // Detect best available Landlock ABI
            let abi = ABI::V5; // or detect with ABI::new_current()
            let mut ruleset = Ruleset::default()
                .handle_access(AccessFs::from_all(abi))?
                .create()?;

            // Add rules for each filesystem_rule in the policy
            for rule in &policy.filesystem_rules {
                let access = if rule.writable {
                    AccessFs::from_all(abi)
                } else {
                    AccessFs::from_read(abi)
                };
                let path_fd = PathFd::new(&rule.path)?;
                ruleset = ruleset.add_rule(PathBeneath::new(path_fd, access))?;
            }

            let status = ruleset.restrict_self()?;
            Ok(status.ruleset != RulesetStatus::NotSupported)
        }

        fn apply_seccomp(policy: &SandboxPolicy) -> Result<bool, OsSandboxError> {
            // Build seccomp filter using seccompiler crate
            // Allow: read, write, open, close, mmap, brk, futex, etc.
            // Deny: ptrace, mount, reboot, sethostname, etc.
            // Conditional on policy: execve (only if exec_shell allowed)
            // ...
            Ok(true)
        }
    }
}
```

**macOS/Windows fallback:**
```rust
#[cfg(not(target_os = "linux"))]
pub mod os_sandbox {
    use super::*;

    pub struct OsSandboxHandle;

    impl OsSandboxHandle {
        pub fn apply(policy: &SandboxPolicy) -> Result<Self, OsSandboxError> {
            tracing::warn!(
                agent_id = %policy.agent_id,
                "OS sandbox not available on this platform; \
                 relying on WASM sandbox only"
            );
            Ok(Self)
        }
    }
}
```

### 2.6 K3: Audit Logging

All sandbox decisions are logged via structured tracing for forensic analysis.

```rust
/// Audit log for sandbox decisions.
///
/// Log level: `info` for allow, `warn` for deny.
/// Target: `clawft::agent::sandbox::audit`
fn audit_log(agent_id: &str, tool_name: &str, action: &str, reason: &str) {
    match action {
        "allowed" => {
            tracing::info!(
                target: "clawft::agent::sandbox::audit",
                agent_id = agent_id,
                tool = tool_name,
                action = "allow",
                reason = reason,
                "sandbox decision"
            );
        }
        "denied" => {
            tracing::warn!(
                target: "clawft::agent::sandbox::audit",
                agent_id = agent_id,
                tool = tool_name,
                action = "deny",
                reason = reason,
                "sandbox decision"
            );
        }
        _ => {}
    }
}
```

**Log format example:**
```
2026-02-25T10:30:00Z  INFO clawft::agent::sandbox::audit: sandbox decision agent_id="analyst-01" tool="read_file" action="allow" reason="policy_check_passed"
2026-02-25T10:30:01Z  WARN clawft::agent::sandbox::audit: sandbox decision agent_id="analyst-01" tool="exec_shell" action="deny" reason="explicit_deny_list"
```

### 2.7 K3a: Security Plugin -- Audit Check Categories

**New crate:** `crates/clawft-security/`

The security plugin implements 50+ audit checks organized into 10 categories. Each check is a function that inspects a `ScanTarget` (skill manifest, source code, runtime behavior) and returns a `CheckResult`.

```rust
// crates/clawft-security/src/lib.rs

pub mod checks;
pub mod hardening;
pub mod monitors;

/// A single audit check result.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Unique check identifier (e.g., "PI-001").
    pub check_id: String,
    /// Human-readable title.
    pub title: String,
    /// Category this check belongs to.
    pub category: CheckCategory,
    /// Severity of the finding.
    pub severity: Severity,
    /// Whether the check passed (no finding) or failed (finding detected).
    pub passed: bool,
    /// Details of the finding (empty if passed).
    pub details: String,
    /// Remediation suggestion.
    pub remediation: String,
    /// File path and line number if applicable.
    pub location: Option<(String, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckCategory {
    PromptInjection,
    ExfiltrationUrl,
    CredentialLiteral,
    PermissionEscalation,
    UnsafeShell,
    SupplyChainRisk,
    DenialOfService,
    IndirectPromptInjection,
    InformationDisclosure,
    CrossAgentAccess,
}
```

### 2.8 K3a: Audit Check Inventory

50+ checks across 10 categories, with priority tiers:

**P0 (block activation by default):**

| ID | Category | Check | Severity |
|----|----------|-------|----------|
| PI-001 | PromptInjection | System prompt override patterns (`ignore previous`, `you are now`) | Critical |
| PI-002 | PromptInjection | Jailbreak template detection (DAN, AIM, developer mode) | Critical |
| PI-003 | PromptInjection | Role hijacking (`act as`, `pretend to be`) | High |
| PI-004 | PromptInjection | Instruction injection via delimiters (`<<<`, `>>>`, `---`) | High |
| PI-005 | PromptInjection | Context window manipulation (excessive whitespace, null bytes) | Medium |
| PI-006 | PromptInjection | Multi-turn injection (payload split across messages) | High |
| PI-007 | PromptInjection | Unicode homoglyph substitution in prompts | Medium |
| PI-008 | PromptInjection | Base64/hex encoded instruction payloads | High |
| EX-001 | ExfiltrationUrl | Webhook/callback URL patterns (ngrok, requestbin, pipedream) | Critical |
| EX-002 | ExfiltrationUrl | DNS exfiltration patterns (data encoded in subdomains) | High |
| EX-003 | ExfiltrationUrl | HTTP POST to non-allowlisted external endpoints | High |
| EX-004 | ExfiltrationUrl | Data URI encoding in outbound requests | Medium |
| EX-005 | ExfiltrationUrl | Steganographic data hiding in image URLs | Medium |
| CR-001 | CredentialLiteral | API key patterns (sk-*, AKIA*, xoxb-*, ghp_*) | Critical |
| CR-002 | CredentialLiteral | Password/secret string literals in source | High |
| CR-003 | CredentialLiteral | Private key material (PEM headers, RSA/EC key blocks) | Critical |
| CR-004 | CredentialLiteral | JWT tokens in source code | High |
| CR-005 | CredentialLiteral | Connection strings with embedded credentials | High |

**P1 (warn, recommend fix):**

| ID | Category | Check | Severity |
|----|----------|-------|----------|
| PE-001 | PermissionEscalation | Tool requests beyond declared manifest permissions | High |
| PE-002 | PermissionEscalation | Runtime sandbox escape attempts (ptrace, mount) | Critical |
| PE-003 | PermissionEscalation | Config.toml manipulation to weaken sandbox | High |
| PE-004 | PermissionEscalation | Privilege escalation via symlink race | High |
| PE-005 | PermissionEscalation | Attempts to access other agents' workspaces | High |
| US-001 | UnsafeShell | Shell metacharacter injection (`;`, `|`, `&&`, backtick) | Critical |
| US-002 | UnsafeShell | Command injection via template strings | High |
| US-003 | UnsafeShell | Path traversal in shell arguments (`../`) | High |
| US-004 | UnsafeShell | Environment variable injection via shell | Medium |
| US-005 | UnsafeShell | Unsafe use of eval/exec in skill code | High |
| SC-001 | SupplyChainRisk | Unsigned skill packages | Medium |
| SC-002 | SupplyChainRisk | Dependency with known CVE | High |
| SC-003 | SupplyChainRisk | Skill requesting network access without justification | Medium |
| SC-004 | SupplyChainRisk | WASM module exceeding size budget (>300KB) | Medium |
| SC-005 | SupplyChainRisk | Obfuscated code patterns | High |

**P2 (informational, best practice):**

| ID | Category | Check | Severity |
|----|----------|-------|----------|
| DS-001 | DenialOfService | Infinite loop patterns | High |
| DS-002 | DenialOfService | Unbounded allocation (Vec::with_capacity with user input) | Medium |
| DS-003 | DenialOfService | Recursive function without depth limit | Medium |
| DS-004 | DenialOfService | Regex with catastrophic backtracking | High |
| DS-005 | DenialOfService | Fork bomb / process spawn storm | Critical |
| IP-001 | IndirectPromptInjection | Tool output containing instruction patterns | High |
| IP-002 | IndirectPromptInjection | Fetched URL content with prompt injection markers | High |
| IP-003 | IndirectPromptInjection | Database query results with injected instructions | Medium |
| IP-004 | IndirectPromptInjection | File content with hidden instructions (comments, metadata) | Medium |
| IP-005 | IndirectPromptInjection | Cross-skill data poisoning | High |
| ID-001 | InformationDisclosure | Error messages leaking internal paths | Medium |
| ID-002 | InformationDisclosure | Stack traces in user-facing responses | Medium |
| ID-003 | InformationDisclosure | Version/dependency information exposure | Low |
| CA-001 | CrossAgentAccess | Agent A reading Agent B's session store | High |
| CA-002 | CrossAgentAccess | Agent A modifying Agent B's config.toml | Critical |
| CA-003 | CrossAgentAccess | Shared memory namespace without access control | Medium |

**Total: 53 checks (18 P0 + 20 P1 + 15 P2)**

### 2.9 K3a: Hardening Modules

**Directory:** `crates/clawft-security/src/hardening/`

```rust
// crates/clawft-security/src/hardening/mod.rs

pub mod seccomp_profiles;
pub mod landlock_profiles;
pub mod network_hardening;

/// Auto-apply hardening profiles to the current process or agent.
///
/// Called by `weft security harden` CLI command.
pub struct HardeningEngine {
    profiles: Vec<Box<dyn HardeningProfile>>,
}

pub trait HardeningProfile: Send + Sync {
    /// Profile name for logging.
    fn name(&self) -> &str;
    /// Check if this profile can be applied on the current platform.
    fn is_applicable(&self) -> bool;
    /// Apply the hardening profile.
    fn apply(&self, policy: &SandboxPolicy) -> Result<(), HardeningError>;
    /// Describe what this profile does (for audit reports).
    fn describe(&self) -> String;
}
```

**Hardening modules:**

1. **seccomp_profiles**: Pre-built seccomp BPF filter sets for common agent roles:
   - `readonly_agent`: Only read syscalls (read, pread64, readv, openat with O_RDONLY)
   - `file_worker`: Read + write + create (no exec, no network)
   - `network_agent`: Read + write + socket + connect (no exec)
   - `full_agent`: Everything except dangerous syscalls (no ptrace, mount, reboot)

2. **landlock_profiles**: Filesystem restriction profiles:
   - Map `filesystem_rules` to Landlock `PathBeneath` rules
   - Deny all filesystem access not explicitly listed
   - Separate read-only and read-write access grants

3. **network_hardening**: Network restriction enforcement:
   - Per-skill domain allowlist enforcement via iptables/nftables rules (optional)
   - Connection counting and rate limiting at the socket level
   - DNS resolution restrictions (prevent DNS rebinding)

### 2.10 K3a: Background Monitors

**Directory:** `crates/clawft-security/src/monitors/`

```rust
// crates/clawft-security/src/monitors/mod.rs

pub mod tool_usage;
pub mod api_calls;
pub mod file_access;

/// Background monitor trait.
///
/// Monitors run continuously during agent operation and emit
/// alerts when anomalous behavior is detected.
pub trait SecurityMonitor: Send + Sync {
    /// Monitor name for logging.
    fn name(&self) -> &str;
    /// Process a security event and return any alerts.
    fn process_event(&mut self, event: &SecurityEvent) -> Vec<SecurityAlert>;
    /// Get current monitoring statistics.
    fn stats(&self) -> MonitorStats;
}

/// Security event types that monitors observe.
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    ToolInvocation {
        agent_id: String,
        tool_name: String,
        timestamp: u64,
    },
    ApiCall {
        agent_id: String,
        endpoint: String,
        method: String,
        timestamp: u64,
    },
    FileAccess {
        agent_id: String,
        path: String,
        operation: FileOperation,
        timestamp: u64,
    },
}

#[derive(Debug, Clone)]
pub struct SecurityAlert {
    pub monitor: String,
    pub severity: Severity,
    pub message: String,
    pub agent_id: String,
    pub timestamp: u64,
}
```

**Monitor implementations:**

1. **tool_usage**: Detects anomalous tool usage patterns:
   - Tool invocation frequency exceeding baseline (3x rolling average)
   - Tools invoked outside normal working context
   - Sequential tool chain that matches known attack patterns (e.g., read_file -> web_fetch = potential exfiltration)

2. **api_calls**: Detects excessive API call patterns:
   - LLM API calls exceeding per-agent budget threshold
   - Burst detection (>10 calls in 5 seconds)
   - Calls to unexpected endpoints not in agent's network rules

3. **file_access**: Detects unexpected file access patterns:
   - Access to files outside the agent's declared workspace
   - Reading sensitive file paths (/etc/shadow, ~/.ssh/*, credential files)
   - Rapid sequential reads across many directories (directory enumeration)

### 2.11 K3a: CLI Integration

**New file:** `crates/clawft-cli/src/commands/security.rs`

```rust
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SecurityCommand {
    /// Run all audit checks against a skill or the workspace.
    Scan {
        /// Path to skill directory or manifest to scan.
        #[arg(short, long)]
        path: Option<String>,
        /// Output format: text, json, sarif.
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Only show findings at or above this severity.
        #[arg(long, default_value = "low")]
        min_severity: String,
        /// Exit with non-zero if any finding at or above this severity.
        #[arg(long, default_value = "high")]
        fail_on: String,
    },
    /// Generate a detailed audit report.
    Audit {
        /// Agent ID to audit (or "all" for workspace-wide).
        #[arg(short, long, default_value = "all")]
        agent: String,
        /// Include sandbox policy analysis.
        #[arg(long)]
        include_policy: bool,
        /// Output format: text, json, html.
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Auto-apply hardening profiles.
    Harden {
        /// Agent ID to harden (or "all" for all agents).
        #[arg(short, long, default_value = "all")]
        agent: String,
        /// Dry run -- show what would be applied without applying.
        #[arg(long)]
        dry_run: bool,
        /// Force apply even if some checks fail.
        #[arg(long)]
        force: bool,
    },
}
```

**CLI integration with `weft skill install`:**

When `weft skill install` is invoked, the security scan runs automatically before activation:

```rust
// In skills_cmd.rs install handler (pseudocode)
async fn install_skill(skill_path: &str, allow_unsigned: bool) -> Result<()> {
    // Step 1: Download/locate skill
    let skill_dir = resolve_skill(skill_path)?;

    // Step 2: Signature verification
    if !allow_unsigned {
        verify_skill_signature(&skill_dir)?;
    } else {
        tracing::warn!("Installing unsigned skill -- use --allow-unsigned only for local dev");
    }

    // Step 3: Security scan (automatic)
    let scan_results = security::scan(&skill_dir, ScanConfig::default())?;

    // Step 4: Block on Critical/High findings
    let blockers: Vec<_> = scan_results.iter()
        .filter(|r| !r.passed && matches!(r.severity, Severity::Critical | Severity::High))
        .collect();

    if !blockers.is_empty() {
        eprintln!("Security scan found {} Critical/High issues:", blockers.len());
        for result in &blockers {
            eprintln!("  [{}] {}: {}", result.check_id, result.severity, result.title);
            eprintln!("       Remediation: {}", result.remediation);
        }
        return Err(anyhow!("Skill installation blocked by security findings"));
    }

    // Step 5: Proceed with installation
    install_skill_files(&skill_dir)?;
    Ok(())
}
```

---

## 3. Pseudocode

### 3.1 SandboxPolicy Construction from Config

```
FUNCTION build_sandbox_policy(agent_id: str, config_path: Path) -> Result<SandboxPolicy>:
    // Step 1: Read and parse config.toml
    config_content = fs::read_to_string(config_path)?
    config = toml::from_str::<AgentConfig>(config_content)?

    // Step 2: Determine sandbox type
    security = config.security.unwrap_or_default()
    sandbox_type = MATCH security.sandbox_type:
        "none":
            IF NOT security.privileged:
                RETURN Err("sandbox_type=none requires privileged=true")
            SandboxType::None
        "wasm":
            SandboxType::Wasm
        "os":
            IF cfg!(target_os = "linux"):
                SandboxType::OsSandbox
            ELSE:
                WARN "OS sandbox not available, falling back to WASM"
                SandboxType::Wasm
        "combined":
            IF cfg!(target_os = "linux"):
                SandboxType::Combined
            ELSE:
                WARN "Combined sandbox falling back to WASM-only"
                SandboxType::Wasm
        absent:
            platform_default()  // OsSandbox on Linux, Wasm elsewhere
        unknown:
            RETURN Err("unknown sandbox type")

    // Step 3: Parse filesystem rules
    fs_rules = []
    FOR rule IN config.filesystem.rules:
        expanded_path = shellexpand::tilde(rule.path)
        canonical = fs::canonicalize(expanded_path)?
        fs_rules.push(FsRule { path: canonical, writable: rule.writable })

    // Step 4: Parse network rules
    net_rules = []
    FOR rule IN config.network.rules:
        net_rules.push(NetRule {
            host_pattern: rule.host.to_lowercase(),
            allowed_ports: rule.ports.clone(),
            require_tls: rule.require_tls.unwrap_or(true),
        })

    // Step 5: Parse environment rules
    env_rules = []
    FOR rule IN config.environment.rules:
        env_rules.push(EnvRule {
            pattern: rule.pattern.clone(),
            writable: rule.writable.unwrap_or(false),
        })

    // Step 6: Parse tool restrictions
    allowed_tools = config.tools.allowed.unwrap_or_default()
    denied_tools = config.tools.denied.unwrap_or_default()

    RETURN Ok(SandboxPolicy {
        agent_id, sandbox_type, allowed_tools, denied_tools,
        filesystem_rules: fs_rules,
        network_rules: net_rules,
        env_rules: env_rules,
    })
```

### 3.2 Sandbox Enforcement Flow

```
FUNCTION enforce_sandbox(sandbox: &AgentSandbox, tool_name: str, args: Value) -> Result<Value>:
    // Step 1: Check tool-level access
    sandbox.check_tool_access(tool_name)?

    // Step 2: Apply resource-specific checks based on tool type
    MATCH tool_name:
        "read_file" | "write_file" | "edit_file" | "list_directory":
            path = extract_path(args)
            check_filesystem_access(sandbox.policy, path, is_write(tool_name))?

        "web_fetch" | "web_search":
            url = extract_url(args)
            check_network_access(sandbox.policy, url)?

        "exec_shell" | "spawn":
            // These tools may be entirely denied by sandbox
            IF sandbox.policy.sandbox_type != SandboxType::None:
                IF "exec_shell" IN sandbox.policy.denied_tools:
                    RETURN Err(SandboxDenied::ToolDenied("exec_shell"))
                check_shell_command_safety(args)?

        "memory_read" | "memory_write":
            check_memory_namespace_access(sandbox.policy, args)?

    // Step 3: Execute the tool within the sandbox
    result = MATCH sandbox.policy.sandbox_type:
        SandboxType::None:
            execute_tool_native(tool_name, args)

        SandboxType::Wasm:
            execute_tool_in_wasm(sandbox.wasm_sandbox, tool_name, args)

        SandboxType::OsSandbox:
            // OS sandbox is already applied at thread level
            execute_tool_native(tool_name, args)

        SandboxType::Combined:
            execute_tool_in_wasm(sandbox.wasm_sandbox, tool_name, args)
            // OS sandbox is also active at thread level

    // Step 4: Post-execution monitoring
    sandbox.emit_security_event(SecurityEvent::ToolInvocation {
        agent_id: sandbox.policy.agent_id,
        tool_name: tool_name.to_string(),
        timestamp: now(),
    })

    RETURN result
```

### 3.3 Audit Check Pipeline

```
FUNCTION run_security_scan(target: ScanTarget, config: ScanConfig) -> Vec<CheckResult>:
    results = []

    // Step 1: Load all check modules
    checkers = [
        PromptInjectionChecker::new(),     // PI-001 through PI-008
        ExfiltrationChecker::new(),         // EX-001 through EX-005
        CredentialChecker::new(),           // CR-001 through CR-005
        PermissionEscalationChecker::new(), // PE-001 through PE-005
        UnsafeShellChecker::new(),          // US-001 through US-005
        SupplyChainChecker::new(),          // SC-001 through SC-005
        DenialOfServiceChecker::new(),      // DS-001 through DS-005
        IndirectInjectionChecker::new(),    // IP-001 through IP-005
        InfoDisclosureChecker::new(),       // ID-001 through ID-003
        CrossAgentChecker::new(),           // CA-001 through CA-003
    ]

    // Step 2: Run each checker's checks
    FOR checker IN checkers:
        IF config.categories_enabled.is_empty()
            OR checker.category() IN config.categories_enabled:
            check_results = checker.run(&target)
            results.extend(check_results)

    // Step 3: Sort by severity (Critical first)
    results.sort_by(|a, b| a.severity.cmp(&b.severity))

    // Step 4: Filter by minimum severity if configured
    IF config.min_severity IS SOME(min):
        results.retain(|r| r.severity >= min)

    RETURN results

FUNCTION check_skill_install_gate(results: &[CheckResult]) -> Result<()>:
    blockers = results.iter()
        .filter(|r| !r.passed)
        .filter(|r| r.severity == Critical OR r.severity == High)
        .collect()

    IF blockers.is_empty():
        RETURN Ok(())

    FOR blocker IN &blockers:
        PRINT "[{blocker.check_id}] {blocker.severity}: {blocker.title}"
        PRINT "  Remediation: {blocker.remediation}"

    RETURN Err("Skill blocked: {blockers.len()} Critical/High findings")
```

---

## 4. Architecture

### 4.1 File Map

| File | Action | Phase | Description |
|------|--------|-------|-------------|
| `crates/clawft-plugin/src/sandbox.rs` | NEW | K3 | `SandboxPolicy`, `SandboxType`, `FsRule`, `NetRule`, `EnvRule` structs |
| `crates/clawft-core/src/agent/sandbox.rs` | NEW | K3 | `AgentSandbox` enforcement, tool access checks, audit logging |
| `crates/clawft-core/src/agent/os_sandbox.rs` | NEW | K3 | OS sandbox (seccomp + landlock, Linux only) |
| `crates/clawft-core/src/agent/mod.rs` | MODIFY | K3 | Add `pub mod sandbox; pub mod os_sandbox;` |
| `crates/clawft-core/src/agent/loop_core.rs` | MODIFY | K3 | Inject sandbox check before tool execution |
| `crates/clawft-security/Cargo.toml` | NEW | K3a | New crate manifest |
| `crates/clawft-security/src/lib.rs` | NEW | K3a | Crate root: `CheckResult`, `Severity`, `CheckCategory` |
| `crates/clawft-security/src/checks/mod.rs` | NEW | K3a | Check module registry |
| `crates/clawft-security/src/checks/prompt_injection.rs` | NEW | K3a | PI-001 through PI-008 |
| `crates/clawft-security/src/checks/exfiltration.rs` | NEW | K3a | EX-001 through EX-005 |
| `crates/clawft-security/src/checks/credentials.rs` | NEW | K3a | CR-001 through CR-005 |
| `crates/clawft-security/src/checks/escalation.rs` | NEW | K3a | PE-001 through PE-005 |
| `crates/clawft-security/src/checks/unsafe_shell.rs` | NEW | K3a | US-001 through US-005 |
| `crates/clawft-security/src/checks/supply_chain.rs` | NEW | K3a | SC-001 through SC-005 |
| `crates/clawft-security/src/checks/dos.rs` | NEW | K3a | DS-001 through DS-005 |
| `crates/clawft-security/src/checks/indirect_injection.rs` | NEW | K3a | IP-001 through IP-005 |
| `crates/clawft-security/src/checks/info_disclosure.rs` | NEW | K3a | ID-001 through ID-003 |
| `crates/clawft-security/src/checks/cross_agent.rs` | NEW | K3a | CA-001 through CA-003 |
| `crates/clawft-security/src/hardening/mod.rs` | NEW | K3a | Hardening engine |
| `crates/clawft-security/src/hardening/seccomp_profiles.rs` | NEW | K3a | Seccomp BPF filter sets |
| `crates/clawft-security/src/hardening/landlock_profiles.rs` | NEW | K3a | Landlock filesystem rules |
| `crates/clawft-security/src/hardening/network_hardening.rs` | NEW | K3a | Network restriction enforcement |
| `crates/clawft-security/src/monitors/mod.rs` | NEW | K3a | Monitor trait and types |
| `crates/clawft-security/src/monitors/tool_usage.rs` | NEW | K3a | Anomalous tool usage monitor |
| `crates/clawft-security/src/monitors/api_calls.rs` | NEW | K3a | Excessive API call monitor |
| `crates/clawft-security/src/monitors/file_access.rs` | NEW | K3a | Unexpected file access monitor |
| `crates/clawft-cli/src/commands/security.rs` | NEW | K3a | `weft security scan/audit/harden` commands |
| `crates/clawft-cli/src/commands/mod.rs` | MODIFY | K3a | Add `pub mod security;` and register commands |

### 4.2 Sandbox Layer Diagram

```
                      Inbound Tool Request
                             |
                             v
                  +---------------------+
                  | AgentSandbox        |
                  | .enforce_sandbox()  |
                  +---------------------+
                             |
              +--------------+--------------+
              |                             |
        check_tool_access()          resource-specific checks
        (allowed/denied lists)       (filesystem, network, env)
              |                             |
              v                             v
        +----------+              +-----------------+
        | allow or |              | FsRule / NetRule |
        | deny     |              | / EnvRule check  |
        +----------+              +-----------------+
              |                             |
              +------------ if allowed -----+
                             |
              +--------------+--------------+
              |              |              |
         Wasm sandbox   OS sandbox     No sandbox
         (wasmtime      (seccomp +     (privileged
          WASI caps)     landlock)      agents only)
              |              |              |
              +--------------+--------------+
                             |
                             v
                     Tool Execution
                             |
                             v
                  +---------------------+
                  | SecurityMonitor     |
                  | .process_event()    |
                  +---------------------+
                             |
                             v
                    emit SecurityEvent
                    (audit log + alerts)
```

### 4.3 Security Plugin Structure

```
crates/clawft-security/
  Cargo.toml
  src/
    lib.rs                  -- CheckResult, Severity, CheckCategory, scan()
    checks/
      mod.rs                -- SecurityChecker trait, check registry
      prompt_injection.rs   -- 8 checks (PI-001..PI-008)
      exfiltration.rs       -- 5 checks (EX-001..EX-005)
      credentials.rs        -- 5 checks (CR-001..CR-005)
      escalation.rs         -- 5 checks (PE-001..PE-005)
      unsafe_shell.rs       -- 5 checks (US-001..US-005)
      supply_chain.rs       -- 5 checks (SC-001..SC-005)
      dos.rs                -- 5 checks (DS-001..DS-005)
      indirect_injection.rs -- 5 checks (IP-001..IP-005)
      info_disclosure.rs    -- 3 checks (ID-001..ID-003)
      cross_agent.rs        -- 3 checks (CA-001..CA-003)
    hardening/
      mod.rs                -- HardeningEngine, HardeningProfile trait
      seccomp_profiles.rs   -- Pre-built seccomp filter sets
      landlock_profiles.rs  -- Landlock filesystem rule generators
      network_hardening.rs  -- Domain allowlist enforcement
    monitors/
      mod.rs                -- SecurityMonitor trait, SecurityEvent, SecurityAlert
      tool_usage.rs         -- Anomalous tool invocation detector
      api_calls.rs          -- API call rate anomaly detector
      file_access.rs        -- Unexpected file access detector
```

### 4.4 CLI Command Flow

```
weft security scan [--path <dir>]
    |
    v
Load ScanTarget (skill manifest + source files)
    |
    v
Run all 53 audit checks (parallelized by category)
    |
    v
Collect CheckResult list
    |
    v
Format output (text / json / sarif)
    |
    v
Exit code: 0 if no findings >= fail_on severity, 1 otherwise


weft security audit [--agent <id>]
    |
    v
Load agent config.toml
    |
    v
Construct SandboxPolicy
    |
    v
Analyze policy for weaknesses
    |
    v
Run audit checks against agent workspace
    |
    v
Generate audit report (policy + findings + recommendations)


weft security harden [--agent <id>] [--dry-run]
    |
    v
Load agent's SandboxPolicy
    |
    v
Select applicable HardeningProfile(s)
    |
    v
IF dry_run: print what would be applied
ELSE: apply seccomp + landlock + network hardening
    |
    v
Report applied profiles
```

### 4.5 Dependency Graph

```
clawft-cli
    |
    +-- commands/security.rs  (NEW: scan, audit, harden)
    +-- commands/skills_cmd.rs (MODIFY: auto-scan on install)
    |
    +-- clawft-security (NEW crate)
    |       |
    |       +-- checks/ (53 audit checks)
    |       +-- hardening/ (seccomp, landlock, network)
    |       +-- monitors/ (tool, api, file)
    |       |
    |       +-- depends on: regex, clawft-types
    |
    +-- clawft-core
    |       |
    |       +-- agent/sandbox.rs (NEW: AgentSandbox)
    |       +-- agent/os_sandbox.rs (NEW: seccomp + landlock)
    |       +-- agent/loop_core.rs (MODIFY: inject sandbox)
    |       |
    |       +-- depends on: clawft-plugin (for SandboxPolicy)
    |
    +-- clawft-plugin
            |
            +-- sandbox.rs (NEW: SandboxPolicy, SandboxType)
            |
            +-- depends on: clawft-types

Platform-specific dependencies:
    clawft-core [target.'cfg(target_os = "linux")'.dependencies]
        +-- landlock = "0.4"
        +-- seccompiler = "0.4"  (or linux-audit/seccomp-sys)
```

---

## 5. Refinement

### 5.1 Platform-Specific Edge Cases

#### 5.1.1 Linux Kernel Version Requirements

- **Landlock**: Requires Linux 5.13+ (ABI V1). Full functionality (network restrictions) requires 5.19+ (ABI V3). Clawft should detect the available ABI version at runtime using `ABI::new_current()` and degrade gracefully:
  - ABI V1: Filesystem restrictions only
  - ABI V2: Filesystem + file truncation
  - ABI V3+: Filesystem + network
  - Pre-5.13 or config disabled: Emit warning, fall back to WASM-only

- **Seccomp**: Requires Linux 3.17+ (seccomp BPF with TSYNC). This is available on all supported distributions. If seccomp fails to apply (e.g., container without CAP_SYS_ADMIN), emit warning and continue with WASM sandbox only.

```rust
fn detect_landlock_abi() -> Option<ABI> {
    // Try the latest ABI first, fall back to older versions
    for abi in [ABI::V5, ABI::V4, ABI::V3, ABI::V2, ABI::V1] {
        if Ruleset::default().handle_access(AccessFs::from_all(abi)).is_ok() {
            return Some(abi);
        }
    }
    None
}
```

#### 5.1.2 macOS Fallback

macOS has no seccomp or landlock equivalent. The sandbox layer falls back to WASM-only:

- The `OsSandboxHandle::apply()` on macOS logs a warning and returns `Ok(Self)` without applying any restrictions.
- The `SandboxPolicy::from_agent_config()` automatically converts `sandbox_type = "os"` to `SandboxType::Wasm` on macOS with a warning.
- Apple's sandbox-exec (deprecated since macOS 10.15) is intentionally NOT used due to deprecation and lack of stable API.

#### 5.1.3 Container Environments

Containers (Docker, Kubernetes) may have restricted capabilities:
- **seccomp**: Docker applies a default seccomp profile. Additional seccomp filters may conflict. Detect Docker via `/.dockerenv` or cgroup inspection. If in Docker, skip additional seccomp (Docker's own profile is likely sufficient) but still apply landlock.
- **landlock**: Works in unprivileged containers on kernels 5.13+. No special handling needed.
- **Nested WASM**: The WASM sandbox works everywhere regardless of container environment.

### 5.2 Performance Impact

#### 5.2.1 Sandbox Overhead Measurements

Expected overhead per-tool-invocation:

| Layer | Overhead | When Applied |
|-------|----------|-------------|
| Tool access check (allow/deny list) | <1 us | Every tool call |
| Filesystem rule check (path canonicalize + prefix match) | 5-50 us | File operations |
| Network rule check (host match + SSRF check) | 1-5 us | Network operations |
| WASM sandbox context switch | 50-200 us | WASM-sandboxed tools |
| Seccomp filter evaluation | <1 us | Every syscall (kernel-level) |
| Landlock path check | 1-10 us | Every filesystem syscall |
| Audit log emission | 5-20 us | Every sandbox decision |

**Total worst case**: ~300 us per tool call with Combined sandbox. This is negligible compared to typical tool execution time (10ms-10s for LLM calls, 1-100ms for filesystem operations).

#### 5.2.2 Startup Overhead

| Operation | Overhead |
|-----------|----------|
| Parse config.toml | 1-5 ms |
| Canonicalize filesystem paths | 1-10 ms (depends on path count) |
| Apply landlock rules | 1-5 ms |
| Apply seccomp BPF | <1 ms |
| Initialize WASM sandbox | 10-50 ms (wasmtime engine) |

**Total**: ~70 ms worst case at agent startup. Acceptable for a one-time cost.

### 5.3 False Positive Mitigation for Audit Checks

#### 5.3.1 Pattern Matching Strategy

Audit checks that rely on regex pattern matching (PI-001, CR-001, US-001) are prone to false positives. Mitigation strategies:

1. **Contextual analysis**: Don't just match patterns in isolation. Check surrounding context:
   - CR-001 (API key patterns): Verify the match is in a string literal, not a regex pattern or documentation comment.
   - PI-001 (prompt injection): Check if the text is in a user-facing prompt template vs. security documentation.

2. **Allowlisting**: Each check supports a per-project allowlist in `.clawft/security-allowlist.toml`:
   ```toml
   [[allowlist]]
   check_id = "CR-001"
   file = "tests/fixtures/credential_patterns.rs"
   reason = "Test fixtures for credential detection tests"
   ```

3. **Severity tuning**: Checks can be downgraded per-project:
   ```toml
   [[severity_override]]
   check_id = "SC-001"
   severity = "info"
   reason = "Internal skills are unsigned during development"
   ```

4. **Confidence scoring**: Each check internally computes a confidence score (0.0-1.0). Findings below a configurable threshold (default: 0.5) are demoted to `Info` severity:
   - High confidence (>0.8): Known exact patterns (e.g., `sk-` prefix with 48 alphanumeric chars)
   - Medium confidence (0.5-0.8): Heuristic matches (e.g., `password` variable near string literal)
   - Low confidence (<0.5): Broad regex matches that frequently false-positive

### 5.4 Check Implementation Strategy

Each check category module implements the `SecurityChecker` trait:

```rust
pub trait SecurityChecker: Send + Sync {
    /// The category this checker covers.
    fn category(&self) -> CheckCategory;
    /// Run all checks in this category against the target.
    fn run(&self, target: &ScanTarget) -> Vec<CheckResult>;
    /// Number of individual checks in this category.
    fn check_count(&self) -> usize;
}

/// Scan target -- what the checks inspect.
pub struct ScanTarget {
    /// Skill manifest (if scanning a skill).
    pub manifest: Option<SkillManifest>,
    /// Source files to analyze.
    pub source_files: Vec<SourceFile>,
    /// Agent config (if scanning an agent's workspace).
    pub agent_config: Option<AgentConfig>,
    /// Runtime behavior samples (if available from monitors).
    pub behavior_samples: Vec<SecurityEvent>,
}

pub struct SourceFile {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
}
```

### 5.5 SARIF Output Support

The `weft security scan --format sarif` output supports SARIF v2.1.0 (Static Analysis Results Interchange Format) for integration with GitHub Code Scanning, VS Code SARIF Viewer, and other SARIF-compatible tools:

```json
{
    "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
    "version": "2.1.0",
    "runs": [{
        "tool": {
            "driver": {
                "name": "clawft-security",
                "version": "0.1.0",
                "rules": [
                    {
                        "id": "PI-001",
                        "name": "PromptInjectionSystemOverride",
                        "shortDescription": { "text": "System prompt override pattern detected" },
                        "defaultConfiguration": { "level": "error" }
                    }
                ]
            }
        },
        "results": []
    }]
}
```

### 5.6 Integration with C2 WASM PluginSandbox

K3's `AgentSandbox` and C2's `PluginSandbox` are complementary but distinct:

| Aspect | C2 PluginSandbox | K3 AgentSandbox |
|--------|-----------------|-----------------|
| Scope | Individual WASM plugins | Entire agent (native + WASM) |
| Config source | Plugin manifest (`skill.toml`) | Agent config (`config.toml`) |
| Enforcement | WASI capability restriction | Tool-level access control + OS sandbox |
| Lifecycle | Per-plugin instance | Per-agent instance |
| Rate limiting | Per-plugin counters | Per-agent counters |

When an agent executes a WASM plugin, **both** layers apply:
1. `AgentSandbox` checks if the agent is allowed to invoke the plugin (tool-level check)
2. `PluginSandbox` restricts what the plugin can do within its WASI environment (capability check)

This is defense-in-depth: even if a plugin escapes its WASI sandbox, the OS-level sandbox (seccomp + landlock) provides a second containment layer.

---

## 6. Implementation Tasks

### 6.1 K3 Tasks

| Task | Description | Estimated Effort |
|------|-------------|-----------------|
| K3.1 | Create `clawft-plugin/src/sandbox.rs` with `SandboxPolicy`, `SandboxType`, `FsRule`, `NetRule`, `EnvRule` | 1 day |
| K3.2 | Implement `SandboxPolicy::from_agent_config()` with config.toml parsing | 1 day |
| K3.3 | Create `clawft-core/src/agent/sandbox.rs` with `AgentSandbox` and `check_tool_access()` | 1 day |
| K3.4 | Create `clawft-core/src/agent/os_sandbox.rs` with landlock + seccomp (Linux) and fallback stubs | 2 days |
| K3.5 | Integrate `AgentSandbox` into `loop_core.rs` tool execution path | 1 day |
| K3.6 | Implement audit logging with structured tracing | 0.5 day |
| K3.7 | Write unit tests for SandboxPolicy construction, tool access checks, and platform fallback | 1 day |
| K3.8 | Write integration tests for end-to-end sandbox enforcement | 1 day |

### 6.2 K3a Tasks

| Task | Description | Estimated Effort |
|------|-------------|-----------------|
| K3a.1 | Create `clawft-security/` crate structure with `CheckResult`, `Severity`, `CheckCategory` | 0.5 day |
| K3a.2 | Implement P0 checks: prompt injection (8), exfiltration (5), credentials (5) | 2 days |
| K3a.3 | Implement P1 checks: escalation (5), shell (5), supply chain (5) | 1.5 days |
| K3a.4 | Implement P2 checks: DoS (5), indirect injection (5), disclosure (3), cross-agent (3) | 1.5 days |
| K3a.5 | Implement hardening modules (seccomp profiles, landlock profiles, network) | 1.5 days |
| K3a.6 | Implement background monitors (tool usage, API calls, file access) | 1.5 days |
| K3a.7 | Create CLI commands (`weft security scan/audit/harden`) | 1 day |
| K3a.8 | Integrate security scan into `weft skill install` pipeline | 0.5 day |
| K3a.9 | Implement SARIF output format | 0.5 day |
| K3a.10 | Write unit tests for all 53 checks (including false positive tests) | 2 days |
| K3a.11 | Write integration tests for CLI commands and install gate | 1 day |

### 6.3 Concurrency Plan

```
K3.1 (SandboxPolicy struct)
    |
    +-- K3.2 (config parsing) --|
    +-- K3.3 (AgentSandbox)    --|-- parallel (both depend on K3.1)
    |         |
    |         +-- K3.4 (OS sandbox) -- depends on K3.3 for types
    |         |
    |         +-- K3.5 (loop_core integration) -- depends on K3.3
    |
    +-- K3.6 (audit logging) -- parallel with K3.2-K3.5
    |
    +-- K3.7 + K3.8 (tests) -- after K3.2-K3.6

K3a.1 (crate structure) -- can start parallel with K3
    |
    +-- K3a.2 (P0 checks) --|
    +-- K3a.3 (P1 checks) --|-- parallel (independent check categories)
    +-- K3a.4 (P2 checks) --|
    |
    +-- K3a.5 (hardening) -- parallel with K3a.2-K3a.4
    +-- K3a.6 (monitors) -- parallel with K3a.2-K3a.4
    |
    +-- K3a.7 (CLI) -- after K3a.1 (needs types)
    +-- K3a.8 (skill install gate) -- after K3a.2 (needs check engine)
    +-- K3a.9 (SARIF) -- after K3a.1 (needs CheckResult)
    |
    +-- K3a.10 + K3a.11 (tests) -- after K3a.2-K3a.9
```

---

## 7. Completion

### 7.1 Acceptance Criteria Checklist

#### K3: Per-Agent Sandbox

- [ ] `SandboxPolicy` struct defined in `crates/clawft-plugin/src/sandbox.rs` with all fields
- [ ] `SandboxType` enum with `None`, `Wasm`, `OsSandbox`, `Combined` variants
- [ ] `SandboxPolicy::from_agent_config()` parses `~/.clawft/agents/<id>/config.toml`
- [ ] `SandboxType::None` requires `privileged = true` in config; rejected otherwise
- [ ] Platform default: `OsSandbox` on Linux, `Wasm` on macOS/Windows
- [ ] macOS/Windows fallback: `sandbox_type = "os"` or `"combined"` falls back to `Wasm` with warning
- [ ] `AgentSandbox::check_tool_access()` enforces `allowed_tools` and `denied_tools`
- [ ] `denied_tools` takes precedence over `allowed_tools` (deny wins)
- [ ] Filesystem rules enforce path-prefix containment with canonicalization
- [ ] Network rules enforce host pattern matching with port and TLS checks
- [ ] Environment rules enforce pattern matching with read-only/read-write distinction
- [ ] OS sandbox (seccomp) applied on Linux for `OsSandbox` and `Combined` types
- [ ] OS sandbox (landlock) applied on Linux for `OsSandbox` and `Combined` types
- [ ] Landlock ABI version detected at runtime with graceful degradation
- [ ] seccomp failure in containers detected and handled (warning + WASM fallback)
- [ ] All sandbox decisions audit-logged: `info` for allow, `warn` for deny
- [ ] Structured tracing fields: `agent_id`, `tool_name`, `action`, `reason`
- [ ] `AgentSandbox` integrated into `loop_core.rs` tool execution path
- [ ] Both `AgentSandbox` and `PluginSandbox` (C2) apply when agent runs WASM plugins

#### K3a: Security Plugin

- [ ] `clawft-security` crate created with `checks/`, `hardening/`, `monitors/` modules
- [ ] `CheckResult` struct with `check_id`, `title`, `category`, `severity`, `passed`, `details`, `remediation`
- [ ] All 53 audit checks implemented (18 P0 + 20 P1 + 15 P2)
- [ ] Check counts by category: PI(8), EX(5), CR(5), PE(5), US(5), SC(5), DS(5), IP(5), ID(3), CA(3)
- [ ] P0 checks (Critical/High) block skill activation by default on `weft skill install`
- [ ] Allowlist support via `.clawft/security-allowlist.toml`
- [ ] Severity override support per-project
- [ ] `weft security scan` runs all checks with text/json/sarif output
- [ ] `weft security audit` generates detailed agent audit report
- [ ] `weft security harden` auto-applies hardening profiles with `--dry-run` support
- [ ] SARIF v2.1.0 output format for CI/CD integration
- [ ] Hardening modules: seccomp profiles (4 role-based), landlock profiles, network
- [ ] Background monitors: tool usage anomaly, API call anomaly, file access anomaly
- [ ] `SecurityMonitor` trait with `process_event()` and `stats()` methods
- [ ] `weft skill install` automatically runs security scan before activation
- [ ] Unsigned skills rejected by default; `--allow-unsigned` logs warning

### 7.2 Test Plan

#### K3 Unit Tests

| Test | File | Description |
|------|------|-------------|
| `sandbox_policy_from_config_defaults` | `sandbox.rs` | Default config produces `OsSandbox` on Linux, `Wasm` elsewhere |
| `sandbox_policy_none_requires_privileged` | `sandbox.rs` | `sandbox_type=none` without `privileged=true` returns error |
| `sandbox_policy_os_fallback_macos` | `sandbox.rs` | `sandbox_type=os` on macOS falls back to `Wasm` |
| `sandbox_policy_combined_fallback` | `sandbox.rs` | `sandbox_type=combined` on non-Linux falls back to `Wasm` |
| `sandbox_policy_unknown_type_error` | `sandbox.rs` | Unknown `sandbox_type` string returns error |
| `check_tool_access_allowed` | `agent/sandbox.rs` | Tool in `allowed_tools` passes check |
| `check_tool_access_denied` | `agent/sandbox.rs` | Tool in `denied_tools` fails check |
| `check_tool_access_deny_precedence` | `agent/sandbox.rs` | Tool in both lists is denied (deny wins) |
| `check_tool_access_empty_allowed` | `agent/sandbox.rs` | Empty `allowed_tools` denies all |
| `fs_rule_path_containment` | `agent/sandbox.rs` | Path within allowed prefix passes |
| `fs_rule_path_escape` | `agent/sandbox.rs` | Path outside allowed prefix is denied |
| `net_rule_host_match` | `agent/sandbox.rs` | Exact host match passes |
| `net_rule_wildcard_match` | `agent/sandbox.rs` | Wildcard pattern matches subdomains |
| `net_rule_port_restriction` | `agent/sandbox.rs` | Disallowed port is denied |
| `env_rule_pattern_match` | `agent/sandbox.rs` | Pattern matching (CLAWFT_* matches CLAWFT_FOO) |
| `audit_log_allow_info_level` | `agent/sandbox.rs` | Allow decisions logged at info level |
| `audit_log_deny_warn_level` | `agent/sandbox.rs` | Deny decisions logged at warn level |

#### K3 Integration Tests

| Test | Description |
|------|-------------|
| `agent_tool_denied_e2e` | Agent with `denied_tools = ["exec_shell"]` cannot run shell commands |
| `agent_fs_sandbox_e2e` | Agent can only read files within its workspace |
| `agent_network_sandbox_e2e` | Agent can only make HTTP calls to allowed hosts |
| `wasm_plugin_double_sandbox` | WASM plugin within sandboxed agent respects both sandbox layers |
| `os_sandbox_applied_linux` | On Linux, seccomp + landlock are applied for `OsSandbox` type |
| `os_sandbox_fallback_macos` | On macOS, OS sandbox gracefully degrades to WASM-only |

#### K3a Unit Tests

| Test | Description |
|------|-------------|
| `pi_001_system_override_detected` | "ignore previous instructions" triggers PI-001 |
| `pi_001_normal_text_passes` | Normal text does not trigger PI-001 |
| `cr_001_api_key_detected` | `sk-proj-abc123...` string triggers CR-001 |
| `cr_001_regex_pattern_not_detected` | API key pattern in regex context does not trigger CR-001 |
| `ex_001_webhook_detected` | ngrok/requestbin URLs trigger EX-001 |
| `us_001_shell_metachar_detected` | Shell metacharacters in arguments trigger US-001 |
| `ds_004_regex_backtracking_detected` | Catastrophic regex patterns trigger DS-004 |
| `scan_returns_53_checks` | Full scan with default config returns exactly 53 check results |
| `scan_filter_by_severity` | `min_severity=high` filters out Medium/Low/Info results |
| `scan_sarif_output_valid` | SARIF output validates against SARIF v2.1.0 schema |
| `allowlist_suppresses_finding` | Allowlisted file:check_id combo does not appear in results |
| `install_gate_blocks_critical` | Critical finding blocks `weft skill install` |
| `install_gate_allows_low` | Low-severity-only findings do not block install |

#### K3a Integration Tests

| Test | Description |
|------|-------------|
| `weft_security_scan_e2e` | CLI command scans a test skill directory and produces output |
| `weft_security_audit_e2e` | CLI command audits a test agent config and produces report |
| `weft_security_harden_dry_run` | CLI command with `--dry-run` prints profiles without applying |
| `skill_install_with_security_gate` | `weft skill install` runs scan and blocks on Critical finding |
| `monitor_tool_usage_anomaly` | Tool usage monitor detects burst exceeding 3x baseline |
| `monitor_file_access_anomaly` | File access monitor detects access outside agent workspace |

### 7.3 Exit Criteria

This phase is complete when:

1. **All K3 acceptance criteria checked** (Section 7.1, K3 subsection)
2. **All K3a acceptance criteria checked** (Section 7.1, K3a subsection)
3. **All K3 unit tests pass:** `cargo test -p clawft-plugin` and `cargo test -p clawft-core`
4. **All K3a unit tests pass:** `cargo test -p clawft-security`
5. **All integration tests pass:** `cargo test --workspace`
6. **Clippy clean:** `cargo clippy --workspace -- -D warnings`
7. **Default sandbox is NOT None:** Verified via unit test that `SandboxPolicy::platform_default()` returns `OsSandbox` on Linux and `Wasm` elsewhere
8. **50+ audit checks verified:** `weft security scan` on test skill reports 53 checks run
9. **CLI commands functional:** `weft security scan`, `weft security audit`, `weft security harden` all produce expected output
10. **Audit logging verified:** Structured logs contain `agent_id`, `tool_name`, `action`, `reason` fields
11. **Existing tests unbroken:** All pre-existing workspace tests continue to pass

---

## 8. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Landlock ABI not available on older Linux kernels (<5.13) | Medium | Medium | 6 | Runtime ABI detection with graceful fallback to WASM-only; warn user |
| Seccomp filter conflicts in containerized environments | Medium | Medium | 6 | Detect container environment; skip additional seccomp if Docker's default profile is present |
| 53 audit checks in 2 weeks is aggressive | Medium | Medium | 6 | Prioritize P0 checks first (18 checks); P2 checks can be deferred to post-sprint if needed |
| False positive rate on pattern-matching checks | High | Medium | 8 | Confidence scoring, contextual analysis, per-project allowlists, and severity overrides |
| WASM sandbox overhead on hot path | Low | Low | 2 | Overhead is <300us per tool call; negligible compared to tool execution time |
| SandboxPolicy config.toml format changes breaking agents | Low | High | 4 | Version the config schema; provide migration tool for format changes |
| Defense-in-depth creates complexity for debugging | Medium | Low | 3 | Clear audit logging with decision reasons; `weft security audit` shows full policy analysis |
| OS sandbox applies irreversibly to thread | Low | Medium | 3 | Apply sandbox early in agent thread lifecycle; document that agent restart is needed for config changes |
| Apple sandbox-exec deprecation on macOS | N/A | N/A | 0 | Intentionally NOT used; WASM-only on macOS is the design choice |

---

## 9. Cross-Element Dependencies

### 9.1 C2 (Element 04): WASM Plugin Host

K3's `AgentSandbox` depends on C2's `PluginSandbox` for WASM-level sandboxing. The integration is through composition: `AgentSandbox` owns an optional `PluginSandbox` that is activated when the sandbox type includes `Wasm`.

**C2 must deliver:** Functional `PluginSandbox::from_manifest()`, wasmtime WASI capability restriction, and the validation functions (`validate_http_request`, `validate_file_access`, `validate_env_access`).

**K3 consumes:** `PluginSandbox` struct, validation functions (reused for agent-level checks with different policy sources).

### 9.2 L2 (Element 09): Per-Agent Workspace Isolation

K3's `SandboxPolicy` reads from the same `~/.clawft/agents/<id>/config.toml` that L2's `WorkspaceManager` creates and maintains. The `[tools]` section's `tool_restrictions` field (from L2) maps directly to K3's `allowed_tools` and `denied_tools`.

**L2 must deliver:** `WorkspaceManager::ensure_agent_workspace()` that creates `config.toml` with a `[tools]` section.

**K3 consumes:** `config.toml` with `[security]`, `[tools]`, `[filesystem]`, `[network]`, `[environment]` sections. K3 adds the `[security]` section to the config schema.

### 9.3 K4 (Element 10): ClawHub Skill Registry

K3a's `weft security scan` runs automatically during `weft skill install`, which is the primary way users install skills from ClawHub. K4 depends on K3a for the install gate.

**K3a delivers:** `security::scan()` function and `check_skill_install_gate()`.

**K4 consumes:** The scan results to block or allow skill activation.

---

## 10. References

- Orchestrator: `10-deployment-community/00-orchestrator.md`
- WASM Security Spec: `04-plugin-skill-system/01-wasm-security-spec.md`
- WASM Plugin Host (C2): `04-plugin-skill-system/02-phase-C2-wasm-host.md`
- Per-Agent Workspace (L2): `09-multi-agent-routing/02-phase-LRouting-agents-swarming.md`
- WASM Crate Source: `crates/clawft-wasm/src/lib.rs`
- Agent Loop: `crates/clawft-core/src/agent/loop_core.rs`
- CLI Commands: `crates/clawft-cli/src/commands/`
- Landlock crate: https://crates.io/crates/landlock
- Seccompiler crate: https://crates.io/crates/seccompiler
- SARIF Specification: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
