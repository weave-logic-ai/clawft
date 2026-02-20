# Security Reference

This document covers the security policies enforced by clawft's tool execution
layer. These policies protect against prompt-injection attacks where an LLM is
tricked into running destructive commands or exfiltrating data through
server-side request forgery (SSRF).

---

## Overview

clawft adopts a **defense-in-depth** approach to tool security:

1. **Permission-Gated Tool Access** -- Per-user tool allowlists enforced by
   `AuthContext` on every request. See [Permission-Gated Tool Access](#permission-gated-tool-access).
2. **Command Execution Policy** -- Controls which shell commands the agent may
   run. Affects the `exec_shell` and `spawn` tools.
3. **URL Safety Policy** -- Controls which URLs the agent may fetch. Affects the
   `web_fetch` tool. Blocks SSRF attacks targeting private networks and cloud
   metadata endpoints.
4. **Workspace Containment** -- All file tools are sandboxed to a workspace
   directory. Covered in the [Tools Reference](tools.md#workspace-containment).
5. **Input Sanitization** -- Session IDs, tool results, and content are
   validated and sanitized before use.
6. **Output Truncation** -- Tool results are capped at 64 KB to prevent
   unbounded context growth.

Each layer operates independently. A command must pass **all** applicable
checks before it executes.

---

## Command Execution Policy

Source: `clawft-tools/src/security_policy.rs`

The `CommandPolicy` struct validates every command before `exec_shell` or
`spawn` runs it. It supports two modes and an always-on dangerous-pattern
check.

### Policy Modes

| Mode        | Behavior                                                     | Default |
|-------------|--------------------------------------------------------------|---------|
| `allowlist` | Only commands whose executable basename is in the allowlist may run. | Yes |
| `denylist`  | Any command is permitted unless it matches a denylist pattern. | No |

In both modes, dangerous patterns are **always checked first** (see below).

### Allowlist Mode (Default, Recommended)

When `mode` is `"allowlist"`, the policy extracts the first token of the
command, strips any path prefix to get the basename, and checks it against the
allowlist. Commands not on the list receive a `PermissionDenied` error.

**Default safe commands (17):**

| Command  | Purpose                     |
|----------|-----------------------------|
| `echo`   | Print text                  |
| `cat`    | Display file contents       |
| `ls`     | List directory entries      |
| `pwd`    | Print working directory     |
| `head`   | Display first lines         |
| `tail`   | Display last lines          |
| `wc`     | Word/line/byte count        |
| `grep`   | Pattern search              |
| `find`   | File search                 |
| `sort`   | Sort lines                  |
| `uniq`   | Deduplicate lines           |
| `diff`   | Compare files               |
| `date`   | Display date/time           |
| `env`    | Display environment         |
| `true`   | Exit with code 0            |
| `false`  | Exit with code 1            |
| `test`   | Evaluate conditional        |

These are read-only or informational commands that cannot modify the system.

### Denylist Mode

When `mode` is `"denylist"`, any command is allowed unless its lowercased form
contains a denylist pattern as a substring. By default the denylist contains the
same entries as the dangerous patterns list (see below). You can supply custom
denylist patterns in the configuration.

### Dangerous Patterns (Always Blocked)

Regardless of the active mode, the following patterns are **always** checked.
Matching is case-insensitive substring containment with whitespace normalized
to spaces (so `sudo\tsomething` matches `"sudo "`).

| Pattern             | Threat                        |
|---------------------|-------------------------------|
| `rm -rf /`          | Filesystem destruction        |
| `sudo `             | Privilege escalation          |
| `mkfs`              | Filesystem formatting         |
| `dd if=`            | Raw disk write                |
| `:(){ :\|:& };:`    | Fork bomb                     |
| `chmod 777 /`       | Dangerous permission change   |
| `> /dev/sd`         | Raw device write              |
| `shutdown`          | System shutdown               |
| `reboot`            | System reboot                 |
| `poweroff`          | System power off              |
| `format c:`         | Disk formatting (Windows)     |

A command that matches a dangerous pattern returns a `PermissionDenied` error
with the specific pattern that triggered it, even if the command's basename is
on the allowlist (e.g. `echo; rm -rf /`).

### Basename Extraction

The policy extracts the executable name from the first token:

```text
"echo foo"          -> "echo"     (on allowlist)
"/usr/bin/ls -la"   -> "ls"       (on allowlist)
"/usr/bin/curl"     -> "curl"     (NOT on allowlist)
"  cat file"        -> "cat"      (on allowlist)
```

---

## URL Safety / SSRF Protection

Source: `clawft-tools/src/url_safety.rs`

The `UrlPolicy` validates every URL before `web_fetch` makes an HTTP request.
This prevents SSRF attacks where an attacker causes the agent to reach internal
services, cloud metadata endpoints, or other private resources.

### How Validation Works

1. Parse the URL and extract the host.
2. If the host is in `allowed_domains`, **skip all further checks**.
3. If the host is in `blocked_domains`, **reject**.
4. If the host matches a cloud metadata endpoint, **reject**.
5. If `allow_private` is `true`, **skip IP-based checks**.
6. If the host is a literal IP address, check it against blocked CIDR ranges.
7. If the host is a domain name, resolve it via DNS and check all resolved IPs
   against blocked CIDR ranges.
8. If DNS resolution fails, allow the URL through (the downstream HTTP client
   will fail on its own).

### Blocked IP Ranges

**IPv4:**

| CIDR             | Range Description              |
|------------------|--------------------------------|
| `10.0.0.0/8`    | Private (RFC 1918 Class A)     |
| `172.16.0.0/12`  | Private (RFC 1918 Class B)     |
| `192.168.0.0/16` | Private (RFC 1918 Class C)     |
| `127.0.0.0/8`   | Loopback                       |
| `169.254.0.0/16` | Link-local / APIPA             |
| `0.0.0.0/8`     | "This" network                 |

**IPv6:**

| CIDR         | Range Description     |
|--------------|-----------------------|
| `::1/128`    | Loopback              |
| `fe80::/10`  | Link-local            |
| `fc00::/7`   | Unique local (ULA)    |

IPv4-mapped IPv6 addresses (`::ffff:x.x.x.x`) are also checked against the
IPv4 blocked ranges.

### Cloud Metadata Endpoint Blocking

The following hostnames are always blocked regardless of policy settings (unless
the host is in `allowed_domains`):

| Hostname                    | Service     |
|-----------------------------|-------------|
| `169.254.169.254`           | AWS / Azure |
| `metadata.google.internal`  | GCP         |
| `metadata.internal`         | Generic     |

### Domain Allow/Block Lists

- **Allowed domains** bypass all safety checks, including IP range checks and
  metadata endpoint checks. Use this for trusted internal services that resolve
  to private IPs.
- **Blocked domains** are rejected before any IP-based checks. Use this to
  explicitly deny specific external domains.

### Disabling URL Safety

Set `enabled: false` to skip all URL validation. This is intended for
development and testing environments only. In production, always keep URL safety
enabled.

---

## Configuration Reference

Security policies are configured in the `tools` section of the JSON
configuration file.

### Command Policy (`tools.commandPolicy`)

```json
{
  "tools": {
    "commandPolicy": {
      "mode": "allowlist",
      "allowlist": ["echo", "cat", "ls", "pwd", "head", "tail", "wc",
                     "grep", "find", "sort", "uniq", "diff", "date",
                     "env", "true", "false", "test"],
      "denylist": []
    }
  }
}
```

| Field       | Type     | Default       | Description                                          |
|-------------|----------|---------------|------------------------------------------------------|
| `mode`      | string   | `"allowlist"` | `"allowlist"` or `"denylist"`                        |
| `allowlist` | string[] | `[]`          | Permitted basenames (overrides built-in defaults when non-empty) |
| `denylist`  | string[] | `[]`          | Blocked patterns for denylist mode (overrides built-in defaults when non-empty) |

When `allowlist` is empty (default), the 17 built-in safe commands are used.
When you provide a non-empty array, it **replaces** the defaults entirely.

The 11 dangerous patterns are always active and cannot be disabled via
configuration.

### URL Policy (`tools.urlPolicy`)

```json
{
  "tools": {
    "urlPolicy": {
      "enabled": true,
      "allowPrivate": false,
      "allowedDomains": [],
      "blockedDomains": []
    }
  }
}
```

| Field            | Type     | Default | Description                                    |
|------------------|----------|---------|------------------------------------------------|
| `enabled`        | boolean  | `true`  | Whether URL safety checks are active           |
| `allowPrivate`   | boolean  | `false` | Allow requests to private/reserved IP ranges   |
| `allowedDomains` | string[] | `[]`    | Domains that bypass all safety checks          |
| `blockedDomains` | string[] | `[]`    | Domains that are always blocked                |

### Full Example

```json
{
  "tools": {
    "commandPolicy": {
      "mode": "allowlist",
      "allowlist": ["echo", "cat", "ls", "pwd", "head", "tail",
                     "wc", "grep", "find", "sort", "uniq", "diff",
                     "date", "env", "true", "false", "test",
                     "cargo", "rustc"]
    },
    "urlPolicy": {
      "enabled": true,
      "allowPrivate": false,
      "allowedDomains": ["api.internal.corp"],
      "blockedDomains": ["malicious-site.com"]
    }
  }
}
```

Both `snake_case` and `camelCase` field names are accepted in JSON
configuration files (`command_policy` / `commandPolicy`,
`url_policy` / `urlPolicy`, `allow_private` / `allowPrivate`, etc.).

---

## Permission-Gated Tool Access

Source: `clawft-core/src/tools/registry.rs`, `clawft-core/src/pipeline/permissions.rs`

The `ToolRegistry` enforces permission checks before every tool execution.
When an `AuthContext` is present on the `ChatRequest`, the registry validates
that the user has permission to invoke the requested tool.

### How It Works

```
Tool call -> Extract permissions from AuthContext -> 4-step check -> Execute or Reject
```

**Step 1: Null permissions bypass.** If no `AuthContext` is attached (e.g.,
internal pipeline calls), the tool executes without permission checks.

**Step 2: Tool allowlist.** The user's `tool_access` list is checked. If the
list is empty, all tools are denied. If it contains `"*"`, all tools are
allowed. Otherwise, the tool name must appear in the list.

**Step 3: Tool metadata level.** If the tool has a `required_permission_level`
in its metadata, the user's `level` must be >= that value.

**Step 4: Custom permissions.** If the tool requires specific custom
permissions (e.g., `"vision_enabled": true`), those must match in the user's
`custom_permissions` map.

### Error Format

Permission denials return a structured `ToolError::PermissionDenied`:

```rust
ToolError::PermissionDenied {
    tool: String,    // the tool that was denied
    reason: String,  // why it was denied
}
```

### Tool Access by Permission Level

| Level | Name | Default Tool Access |
|-------|------|---------------------|
| 0 | `zero_trust` | None (empty list) |
| 1 | `user` | `read_file`, `write_file`, `edit_file`, `list_dir`, `web_search`, `web_fetch`, `message` |
| 2 | `admin` | All (`*`) |

### Per-User Tool Overrides

Individual users can have custom tool access that differs from their level's
defaults:

```json
{
  "routing": {
    "permissions": {
      "users": {
        "bob_discord_456": {
          "level": 1,
          "tool_access": ["read_file", "list_dir", "web_search"]
        }
      }
    }
  }
}
```

Bob gets Level 1 permissions but with a restricted tool set (no `write_file`,
`edit_file`, `web_fetch`, or `message`).

### AuthContext Injection Prevention

The `auth_context` field on `ChatRequest` uses `#[serde(skip_deserializing)]`,
which means:

- Users cannot inject permissions via the gateway API JSON payload
- The field is only populated server-side by the `AgentLoop`
- CLI users automatically receive `admin` (Level 2) permissions
- Remote channel users receive `zero_trust` (Level 0) by default

This is a critical security boundary: the permission system is enforced
server-side and cannot be bypassed through crafted input.

---

## Security Model

### Threat Model

clawft's primary threat model for tool security is **LLM prompt injection
leading to tool execution**. In this scenario:

1. An attacker crafts a message (directly or via injected content in documents,
   web pages, or other data sources) that manipulates the LLM's behavior.
2. The LLM issues tool calls that the attacker intended, such as running
   destructive shell commands, exfiltrating data via HTTP requests to
   attacker-controlled servers, or probing internal infrastructure.

The security policies act as a **guardrail between the LLM and the operating
system**, ensuring that even if the LLM is compromised by prompt injection, the
damage is bounded.

### Why Allowlist Over Denylist for LLM-Controlled Execution

For systems where an LLM controls tool execution, **allowlist mode is strongly
recommended** over denylist mode:

- **Denylists are inherently incomplete.** An attacker can find commands or
  encodings that bypass known patterns. New dangerous commands may be installed
  on the system that are not covered by the denylist.
- **Allowlists fail closed.** Any command not explicitly approved is rejected.
  This prevents entire categories of attacks without needing to anticipate
  specific payloads.
- **The LLM's needs are bounded.** An AI agent typically needs a small set of
  informational commands. The 17 default commands cover most read-only use
  cases.

The denylist mode exists for environments that require broader command access
and accept the increased risk.

### Defense-in-Depth

Even in denylist mode, the 11 dangerous patterns are always checked. This
provides a safety net against the most catastrophic commands regardless of
configuration:

```
Request -> Dangerous pattern check -> Mode-specific check -> Execution
              (always on)              (allowlist/denylist)
```

Both checks must pass before a command executes. This layered approach means
that a misconfigured denylist still catches `rm -rf /`, `sudo`, fork bombs, and
similar attacks.

---

## Best Practices

1. **Keep the allowlist minimal.** Only add commands the agent genuinely needs.
   Every additional command increases the attack surface.

2. **Review custom allowlist additions.** Before adding a command, consider
   whether it can modify the system, access the network, or escalate
   privileges. Commands like `curl`, `wget`, `python`, `bash`, and `node`
   should be added with extreme caution.

3. **Do not disable URL safety in production.** The `enabled: false` setting is
   for development only. In production, always keep SSRF protection active.

4. **Use `allowedDomains` sparingly.** Each allowed domain bypasses all URL
   safety checks, including IP range validation. Only add domains you fully
   trust and control.

5. **Monitor tool execution logs.** clawft logs all command rejections and URL
   validation failures at the `warn` level. Review these logs regularly to
   detect attempted abuse.

6. **Conduct regular security audits.** Periodically review:
   - The configured allowlist/denylist for unnecessary entries.
   - The `allowedDomains` list for stale or compromised domains.
   - Tool execution logs for suspicious patterns.
   - Dependency updates that may affect security boundaries.

7. **Prefer allowlist mode.** Unless your use case requires broad command
   access, stay with the default allowlist mode. Switch to denylist mode only
   with a clear understanding of the risks.

8. **Test policy changes.** After modifying command or URL policies, verify that
   both intended commands succeed and dangerous commands are still blocked.

---

## WASM Plugin Security

Source: `clawft-plugin/src/wasm_host.rs`

The WASM sandbox prevents untrusted plugin code from escaping isolation. All host function calls pass through permission checks before any system resource is accessed.

| Layer | Mechanism | Threat Mitigated |
|-------|-----------|-----------------|
| Fuel metering | wasmtime fuel (1B default) | CPU denial-of-service |
| Memory limits | 16 MB default cap | Out-of-memory crashes |
| Epoch interruption | Wall-clock timeout | Infinite loops, CPU starvation |
| Permission allowlists | Per-function checks | Unauthorized resource access |
| Path canonicalization | `canonicalize` + symlink rejection | Directory traversal |
| SSRF check | `is_private_ip()` on resolved IPs | Internal network probing |
| Rate limiting | Token bucket on `log` and `http-request` | Resource exhaustion |
| Audit logging | All host function calls logged | Post-incident forensics |

Each of the 5 WIT host functions enforces its own gate: `http-request` validates URLs against `permissions.network` and rejects private IPs (reuses `is_private_ip()`); `read-file`/`write-file` canonicalize paths and reject symlinks outside `permissions.filesystem`; `get-env` returns only `permissions.env_vars`; `log` is rate-limited. 45 security tests cover all attack vectors including fuel exhaustion, memory limits, permission denial, path traversal, SSRF, rate limits, and epoch interruption.

---

## Plugin Permission Re-prompt (T41)

When a plugin upgrade requests new permissions, `PermissionDiff` computes the delta against the `PermissionStore`'s previously approved set. Only new permissions (new URLs, paths, env vars, or shell access) are presented to the `PermissionApprover`. If no new permissions exist, the upgrade proceeds silently. On rejection, the old version remains active.

---

## WITNESS Audit Chains (H2.6)

SHA-256 hash-chained audit trail for memory operations. Each record includes the operation, timestamp, and hash of the previous record. Sequential verification from the root detects tampering. Memory exports include the WITNESS chain; imports validate it before accepting data.

---

## Security Plugin (clawft-security)

The `clawft-security` crate provides 57 audit checks across 10 categories: authentication, authorization, input validation, filesystem, network, cryptography, dependencies, configuration, logging, and plugin security. Runs via `weft security scan` and automatically during skill install (K3a).

---

## SandboxPolicy

OS-level process sandboxing (seccomp-bpf/Landlock on Linux 5.13+) with WASM fallback on other platforms. Each agent receives its own policy restricting syscalls, filesystem access, network, and process spawning based on configuration and trust level.
