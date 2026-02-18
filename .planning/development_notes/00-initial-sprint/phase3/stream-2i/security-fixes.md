# Stream 2I: Security Fixes -- Development Notes

**Status**: Complete
**Started**: 2026-02-17
**Completed**: 2026-02-17
**Agents**: 8 (part of Phase 3 hive)
**Test Results**: 960 tests passing, 0 failures, 0 clippy warnings

## Objective

Fix 3 HIGH security findings from the Phase 2 security review before Phase 3 features ship.

## Decisions

### SEC-1 + SEC-2: Command Allowlist

- **Decision**: Allowlist mode as default, denylist available via config
- **Rationale**: Default-deny is the only safe approach for LLM-controlled shell execution. Prompt injection can easily bypass denylists.
- **Default allowlist**: 17 safe commands (echo, cat, ls, pwd, head, tail, wc, grep, find, sort, uniq, diff, date, env, true, false, test)
- **Config override**: Operators can switch to denylist mode or expand the allowlist via `tools.commandPolicy` in config

### SEC-3: SSRF Protection

- **Decision**: Build from scratch with `url` + `ipnet` crates instead of using immature SSRF-specific crates
- **Rationale**: `agent-fetch` (49 downloads) and `url-preview` (7K downloads) are too immature for security-critical code. Using battle-tested components gives us full control.
- **Blocked ranges**: RFC 1918 private, loopback, link-local, cloud metadata endpoints
- **Configurable**: Domain allow/block lists via `tools.urlPolicy` in config

## Architecture

```
Config -> ToolsConfig -> CommandPolicyConfig -> CommandPolicy
                      -> UrlPolicyConfig     -> UrlPolicy

LLM Agent -> Tool::execute() -> Policy::validate() -> System Operation
```

Shared security policy layer between tool interface and system operations.

## Challenges

- Balancing security with usability (too restrictive = unusable, too permissive = insecure)
- DNS resolution for SSRF: async resolution adds complexity, opted for sync std::net initially
- Config type placement: types in clawft-types, policy logic in clawft-tools

## Files Modified

| File | Change |
|------|--------|
| `clawft-types/src/config.rs` | Added CommandPolicyConfig, UrlPolicyConfig |
| `clawft-tools/src/security_policy.rs` | NEW: CommandPolicy module |
| `clawft-tools/src/url_safety.rs` | NEW: SSRF protection module |
| `clawft-tools/src/shell_tool.rs` | Wired CommandPolicy |
| `clawft-tools/src/spawn_tool.rs` | Wired CommandPolicy |
| `clawft-tools/src/web_fetch.rs` | Wired UrlPolicy |
| `clawft-tools/src/lib.rs` | Updated register_all signature |
| `clawft-cli/src/commands/agent.rs` | Passes policies from config |
| `clawft-cli/src/commands/gateway.rs` | Passes policies from config |

## Round 2 Wiring

The security policy modules were implemented in Round 1 but not fully wired into the tool execution paths. In Round 2, the coordinator manually completed the wiring:

1. **shell_tool.rs**: `CommandPolicy::validate()` called before any command execution. Rejects commands not on the allowlist.
2. **spawn_tool.rs**: `CommandPolicy::validate()` called before process spawning. Same allowlist enforcement as shell.
3. **web_fetch.rs**: `UrlPolicy::validate()` called before any HTTP request. Blocks private IPs, loopback, link-local, and cloud metadata endpoints.
4. **agent.rs / gateway.rs**: CLI entry points construct `CommandPolicy` and `UrlPolicy` from config and pass them through to `register_all`.

This two-round approach (implement policies, then wire them) was necessary because the swarm agents that built the policy modules did not have enough context about the tool execution call sites to wire them correctly in a single pass.

## Lessons Learned

1. **Security modules need manual wiring**: Swarm agents can write isolated modules well, but connecting them into existing call chains requires understanding the full call graph. The coordinator had to do this manually in Round 2.

2. **Default-deny is worth the friction**: The allowlist approach required more upfront work (enumerating safe commands) but is fundamentally more secure than denylists for LLM-controlled execution. Denylists are trivially bypassed via prompt injection.

3. **Config types vs policy logic separation works well**: Keeping `CommandPolicyConfig` / `UrlPolicyConfig` in clawft-types and the enforcement logic in clawft-tools maintains clean crate boundaries. The types crate stays dependency-light while the tools crate owns the security behavior.

4. **DNS-based SSRF checks are incomplete**: The current sync `std::net` resolution catches obvious cases (private IPs, loopback) but TOCTOU (time-of-check-time-of-use) attacks via DNS rebinding remain a theoretical concern. A future iteration should consider adding connection-time IP validation.

5. **Test count as a health signal**: Maintaining 960 tests with 0 failures and 0 clippy warnings after wiring security policies across 3 tools confirms the changes are non-breaking. Running the full test suite after each wiring step caught regressions immediately.
