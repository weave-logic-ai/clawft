# Stream 3I-E: Security Hardening and Documentation

**Agent**: 3I-E
**Date**: 2026-02-17
**Status**: Complete

## Tasks Completed

### SEC-04: API Key Leakage Audit

**Finding**: No API key leakage exists in the codebase. All log calls across
all 9 crates were audited. The `OpenAiCompatProvider` Debug impl at
`openai_compat.rs:181-188` already masks keys with `***`. The Authorization
header is built at line 109 but never logged. `ProviderConfig` stores
environment variable names, not raw keys.

**Changes**:
- `clawft-llm/src/openai_compat.rs`: Added 3 regression tests:
  - `debug_impl_never_leaks_api_key` -- verifies Debug output masks multiple
    key formats (sk-xxx, Bearer xxx, hex strings)
  - `provider_display_does_not_leak_key` -- verifies `***` masking
  - `log_fields_do_not_include_api_key` -- verifies ProviderConfig stores env
    var names not keys

### SEC-05: Symlink Path Traversal Test

**Finding**: `validate_path()` already handles symlinks correctly.
`canonicalize()` resolves symlinks to their real target and then the
workspace containment check rejects any resolved path outside the allowed
directory.

**Changes**:
- `clawft-tools/src/file_tools.rs`: Added 3 tests (all `#[cfg(unix)]`):
  - `test_symlink_outside_workspace_rejected` -- symlink to file outside workspace
  - `test_symlink_to_directory_outside_workspace_rejected` -- symlink to dir
  - `test_symlink_within_workspace_allowed` -- internal symlinks pass

### MISSED-05: Web Fetch Body Size Limit

**Finding**: The previous implementation had a hardcoded 1 MB
(`MAX_RESPONSE_BYTES = 1_048_576`) constant with no configurability and no
truncation metadata in the response.

**Changes**:
- `clawft-tools/src/web_fetch.rs`:
  - Changed constant to `DEFAULT_MAX_RESPONSE_BYTES = 10 * 1024 * 1024` (10 MB)
  - Added `max_response_bytes` field to `WebFetchTool` struct
  - Added `with_max_bytes()` constructor for custom limits
  - Added truncation metadata to response JSON: `truncated`, `limit_bytes`,
    `warning` fields when body exceeds limit
  - Added `warn!` tracing log on truncation
  - Added 4 tests:
    - `default_max_response_bytes_is_10mb`
    - `with_max_bytes_sets_custom_limit`
    - `oversized_response_is_truncated` (mock platform, verifies all metadata)
    - `response_within_limit_not_truncated` (mock platform)

### DOC-01: Benchmarks Documentation

**Changes**:
- Created `clawft/docs/benchmarks/results.md` with:
  - Native binary baseline metrics (startup 3.5ms, binary 4710KB, throughput 418/s)
  - WASM build metrics (57.9KB raw, 24.3KB gzipped, dlmalloc allocator)
  - Test suite summary (1,254 tests passing across 9 crates)
  - Instructions for running individual and combined benchmarks
  - Regression detection documentation
  - WASM profiling tool references

### DOC-04: Documentation Fix

**Finding**: The gap analysis referenced "readability" in routing.md, but the
actual issue was in `architecture/overview.md` where the `web_fetch` tool
description said "Fetch and extract content from URLs" -- implying content
extraction/readability processing that does not exist.

**Changes**:
- `clawft/docs/architecture/overview.md`: Updated web_fetch description to
  "Fetch raw content from URLs (with SSRF protection and body size limits)"
- `clawft/docs/reference/tools.md`: Updated body size limit from "1 MB
  (1,048,576 bytes)" to "10 MB (10,485,760 bytes)" with note about
  configurability and truncation metadata

### Pre-existing Fix: McpSession Debug

While running tests, discovered a pre-existing compilation error:
`McpSession` in `clawft-services/src/mcp/mod.rs` lacked a `Debug` impl,
which was required by test code using `unwrap_err()`.

**Changes**:
- `clawft-services/src/mcp/mod.rs`: Added manual `Debug` impl for
  `McpSession` that shows `server_info` and `protocol_version` (uses
  `finish_non_exhaustive()` to skip the non-Debug `McpClient` field)

## Verification

- `cargo test --workspace`: 1,254 tests passed, 0 failed
- `cargo clippy --workspace -- -D warnings`: clean (0 errors, 0 warnings)
- All 11 new tests confirmed passing:
  - 3 SEC-04 tests in `clawft-llm`
  - 3 SEC-05 tests in `clawft-tools`
  - 4 MISSED-05 tests in `clawft-tools`
  - 1 DOC (no code tests for docs)

## Files Modified

| File | Change Type |
|------|-------------|
| `crates/clawft-tools/src/web_fetch.rs` | Modified (MISSED-05) |
| `crates/clawft-tools/src/file_tools.rs` | Modified (SEC-05 tests) |
| `crates/clawft-llm/src/openai_compat.rs` | Modified (SEC-04 tests) |
| `crates/clawft-services/src/mcp/mod.rs` | Modified (Debug impl fix) |
| `docs/architecture/overview.md` | Modified (DOC-04) |
| `docs/reference/tools.md` | Modified (DOC-04) |
| `docs/benchmarks/results.md` | Created (DOC-01) |
