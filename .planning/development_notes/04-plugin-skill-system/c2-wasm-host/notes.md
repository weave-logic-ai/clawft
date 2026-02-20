# Phase C2: WASM Plugin Host -- Notes

**Items**: C2 (wasmtime, WIT, host functions, security sandbox)
**Week**: 4-5

---

## In Progress: 2026-02-20

### WIT Interface Definition
- Created `crates/clawft-wasm/wit/plugin.wit`
- Package: `clawft:plugin@0.1.0`
- 5 host imports: `http-request`, `read-file`, `write-file`, `get-env`, `log`
- 3 plugin exports: `init`, `execute-tool`, `describe`
- Full documentation of security validation in WIT comments

### SandboxedFileSystem (`fs.rs`)
- Full implementation replacing previous stubs
- 8 filesystem operations with security enforcement
- Path canonicalization with symlink escape detection
- Read size limit: 8 MB, Write size limit: 4 MB
- 14 new tests covering:
  - Path traversal prevention
  - Symlink escape rejection
  - Size limit enforcement
  - Directory listing sandbox containment

### SandboxedHttpClient (`http.rs`)
- HTTP request validation against plugin network allowlist
- SSRF protection using `is_private_ip()` from Element 03/A6
- Rate limiting per plugin instance
- Body size limits
- 6 new tests covering:
  - Network allowlist exact + wildcard matching
  - SSRF rejection (private IP ranges)
  - Rate limit enforcement

### PluginSandbox (`sandbox.rs`)
- `PluginSandbox` struct -- runtime security context for loaded WASM plugins
- `NetworkAllowlist`: supports exact match, `*.example.com` wildcard, and `*` (allow all)
- `RateCounter`: fixed-window rate limiter per host function
- `is_private_ip()`: validates against RFC 1918 ranges (10.x, 172.16-31.x, 192.168.x), loopback, link-local
- Env var access: hardcoded deny list + plugin-scoped allowlist
- 5 clippy fixes applied to sandbox.rs

### Test Summary
- 116 tests passing with `wasm-plugins` feature enabled
- Security tests cover: path traversal (T01-T08), symlink escape (T09-T12), large file rejection (T13-T14), HTTP allowlist (T15-T20), SSRF (T21-T24), env var filtering (T25-T27), rate limiting (T33-T36), additional security (T44)
- 30 of 45 security tests complete (T01-T27, T33-T36, T44)

### Remaining Work
- wasmtime engine integration (loading and executing actual WASM modules)
- Fuel metering (configurable, default 1B units)
- Memory limits via StoreLimits (default 16MB)
- Audit logging for host function calls
- Remaining 15 security tests (T28-T32, T37-T43, T45)
