# ExoChain Governance Certification Report -- CRITICAL & HIGH Items

**Date**: 2026-04-04
**Auditor**: security-auditor (automated source verification)
**Scope**: 21 items (5 CRITICAL, 16 HIGH) from exochain-governance-audit.md
**Method**: Line-by-line source code inspection of actual `.rs` files on `master` branch

## Certification Criteria

For each item, the following were verified:

- **Chain Event**: `EVENT_KIND_*` constant defined in `chain.rs`
- **Chain Logged**: `chain_manager.append()` called after the mutation
- **Governance Gated**: `GovernanceGate`/`GateBackend`/`GovernanceEngine` evaluation occurs before the mutation
- **Payload Complete**: Event payload includes identifying information (IDs, names, actions)
- **Feature Guard**: `#[cfg(feature = "exochain")]` wraps chain/governance code
- **Denial Returns Error**: Governance denial returns an `Err` variant (not silently ignored)

## CRITICAL Items (5/5)

| # | File:Method | Chain Event Constant | Chain Logged? | Governance Gated? | Payload Complete? | Denial Returns Err? | Feature Guard? | CERTIFIED |
|---|------------|---------------------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | auth_service:register_credential | `EVENT_KIND_AUTH_CREDENTIAL_REGISTER` | YES (line 310) | YES -- `GateBackend.check()` (line 275) | YES -- credential_name, credential_type | YES -- `KernelError::GovernanceDenied` | YES | PASS |
| 2 | auth_service:rotate_credential | `EVENT_KIND_AUTH_CREDENTIAL_ROTATE` | YES (line 338) | NO | YES -- credential_name | N/A | YES | **FAIL** |
| 3 | auth_service:request_token | `EVENT_KIND_AUTH_TOKEN_ISSUE` | YES (line 403) | NO | YES -- token_id, credential_name, agent_id, ttl_secs | N/A | YES | **FAIL** |
| 4 | config_service:set_secret | `EVENT_KIND_CONFIG_SECRET_SET` | YES (line 473) | YES -- `GateBackend.check()` (line 444) | YES -- namespace, key | YES -- `KernelError::GovernanceDenied` | YES | PASS |
| 5 | profile_store:delete_profile | `EVENT_KIND_PROFILE_DELETE` | YES (line 269) | YES -- `GovernanceEngine.evaluate()` (line 247) | YES -- profile_id | YES -- `ProfileError::GovernanceDenied` | YES | PASS |

### Critical Failures

**#2 -- auth_service:rotate_credential**: Chain logging is present but **no governance gate**. Credential rotation is a critical security operation that should require governance approval. An agent with IPC access could rotate any credential without policy check.

**#3 -- auth_service:request_token**: Chain logging is present but **no governance gate**. Token issuance is gated only by the `allowed_agents` check on the credential, not by governance policy. High-frequency token requests could bypass governance oversight.

## HIGH Items (16/16)

| # | File:Method | Chain Event Constant | Chain Logged? | Governance Gated? | Payload Complete? | Denial Returns Err? | Feature Guard? | CERTIFIED |
|---|------------|---------------------|:---:|:---:|:---:|:---:|:---:|:---:|
| 6 | auth_service:revoke_token | `EVENT_KIND_AUTH_TOKEN_REVOKE` | YES (line 447) | NO | YES -- token_id | N/A | YES | **PARTIAL** |
| 7 | auth_service:authenticate | `EVENT_KIND_AUTH_ATTEMPT` | YES (lines 553, 582) | NO | YES -- agent_id, success, token_id (on success) | N/A | YES | PASS |
| 8 | config_service:set | `EVENT_KIND_CONFIG_SET` | YES (line 238) | YES -- `GateBackend.check()` (line 199) | YES -- namespace, key, changed_by | YES -- `KernelError::GovernanceDenied` | YES | PASS |
| 9 | config_service:delete | `EVENT_KIND_CONFIG_DELETE` | YES (line 297) | YES -- `GateBackend.check()` (line 267) | YES -- namespace, key, changed_by | YES -- `KernelError::GovernanceDenied` | YES | PASS |
| 10 | hnsw_service:clear | `EVENT_KIND_HNSW_CLEAR` | YES (line 230) | YES -- `GovernanceEngine.evaluate()` (line 212) | MINIMAL -- empty JSON `{}` | YES -- returns `Err(String)` | YES | **PARTIAL** |
| 11 | profile_store:create_profile | `EVENT_KIND_PROFILE_CREATE` | YES (line 216) | YES -- `GovernanceEngine.evaluate()` (line 199) | YES -- profile_id, name | YES -- `ProfileError::GovernanceDenied` | YES | PASS |
| 12 | profile_store:switch_profile | `EVENT_KIND_PROFILE_SWITCH` | YES (line 309) | NO | YES -- profile_id, previous | N/A | YES | PASS |
| 13 | cron:add_job | `EVENT_KIND_CRON_ADD` | YES (line 161) | YES -- `GovernanceGate.check()` (line 126) | YES -- job_id, job_name, interval_secs, command | YES -- `CronError::GovernanceDenied` | YES | PASS |
| 14 | cluster:add_peer | `EVENT_KIND_CLUSTER_PEER_ADD` | YES (line 466) | YES -- `GovernanceGate.check()` (line 413) | YES -- node_id, name, platform | YES -- `ClusterError::AuthFailed` | YES | PASS |
| 15 | cluster:remove_peer | `EVENT_KIND_CLUSTER_PEER_REMOVE` | YES (line 513) | YES -- `GovernanceGate.check()` (line 486) | YES -- node_id, name | YES -- `ClusterError::AuthFailed` | YES | PASS |
| 16 | app:install | `EVENT_KIND_APP_INSTALL` | YES (line 506) | YES -- `GovernanceGate.check()` (line 475) | YES -- app_name, version, agent/tool/service counts | YES -- `AppError::GovernanceDenied` | YES | PASS |
| 17 | app:remove | `EVENT_KIND_APP_REMOVE` | YES (line 645) | YES -- `GovernanceGate.check()` (line 621) | YES -- app_name, version | YES -- `AppError::GovernanceDenied` | YES | PASS |
| 18 | app:start | `EVENT_KIND_APP_START` | YES (line 769) | NO | YES -- app_name, agents_spawned | N/A | YES | PASS |
| 19 | capability:request_elevation | `EVENT_KIND_CAPABILITY_ELEVATE` | YES (lines 597, 614) | YES -- `GovernanceGate.check()` (line 580) | YES -- pid, platform, result, can_spawn, can_network | YES -- returns `ElevationResult::Denied` | YES | PASS |
| 20 | environment:set_active | `EVENT_KIND_ENV_SWITCH` | YES (line 333) | YES -- `GovernanceGate.check()` (line 312) | YES -- id, previous | YES -- returns `EnvironmentError::NotFound` (reused) | YES | **PARTIAL** |
| 21 | wasm_runner/runner:execute | `EVENT_KIND_WASM_EXECUTE` | YES (line 218) | YES -- `GovernanceEngine.evaluate()` (line 199) | YES -- tool name, module_size | YES -- `WasmError::GovernanceDenied` | YES | PASS |

## Summary

| Status | Count | Items |
|--------|-------|-------|
| **PASS** | 15 | #1, #4, #5, #7, #8, #9, #11, #13, #14, #15, #16, #17, #18, #19, #21 |
| **PARTIAL** | 3 | #6, #10, #20 |
| **FAIL** | 2 | #2, #3 |
| **N/A (by design)** | 1 | #12 (switch_profile is read-context change, logging-only is acceptable) |

**Overall certification: 15/21 PASS, 3 PARTIAL, 2 FAIL**

## Detailed Findings

### FAIL: #2 -- rotate_credential lacks governance gate

**File**: `crates/clawft-kernel/src/auth_service.rs`, line 325-348
**Issue**: `rotate_credential()` logs to the chain but does not consult a governance gate before performing the mutation. Credential rotation is a CRITICAL security operation -- a compromised agent could rotate credentials without any policy check.
**Fix**: Add a `governance_gate.check()` block before the mutation, mirroring the pattern in `register_credential()`. Use action `"auth.credential.rotate"`.

### FAIL: #3 -- request_token lacks governance gate

**File**: `crates/clawft-kernel/src/auth_service.rs`, line 354-424
**Issue**: `request_token()` checks `allowed_agents` (an authorization list on the credential) but does not consult the governance gate. Token issuance frequency and scope are not governed by policy.
**Fix**: Add a `governance_gate.check()` block before token issuance. Use action `"auth.token.issue"`. The context should include the credential name, requesting agent, and requested TTL.

### PARTIAL: #6 -- revoke_token logs but has no governance gate

**File**: `crates/clawft-kernel/src/auth_service.rs`, line 441-458
**Issue**: Token revocation is logged to the chain but not governance-gated. While revocation is generally less sensitive than issuance, it can still be used for denial-of-service against agents that depend on active tokens.
**Recommendation**: Consider adding a governance gate. Lower priority than #2 and #3.

### PARTIAL: #10 -- hnsw_service:clear has empty payload

**File**: `crates/clawft-kernel/src/hnsw_service.rs`, line 230
**Issue**: The chain event for `hnsw.clear` logs `Some(serde_json::json!({}))` -- an empty JSON object. The payload should include the number of entries destroyed and optionally the epoch number, to support forensic reconstruction.
**Fix**: Change payload to `serde_json::json!({ "entries_destroyed": store.len(), "epoch": self.epoch.load(Ordering::SeqCst) })` (capture `len()` before the clear).

### PARTIAL: #20 -- environment:set_active reuses NotFound error for governance denial

**File**: `crates/clawft-kernel/src/environment.rs`, line 326
**Issue**: When the governance gate denies an environment switch, the code returns `EnvironmentError::NotFound` instead of a governance-specific error. This obscures the denial reason from the caller and makes debugging difficult.
**Fix**: Add a `GovernanceDenied` variant to `EnvironmentError` (matching the pattern in `ProfileError`, `AppError`, `CronError`) and return it on gate denial.

## Remediation Priority

1. **P0** (security-critical): Fix #2 (rotate_credential governance gate) and #3 (request_token governance gate)
2. **P1** (correctness): Fix #20 (environment denial error type) and #10 (hnsw clear payload)
3. **P2** (defense-in-depth): Fix #6 (revoke_token governance gate)

## Architecture Notes

- Two governance abstractions are in use: `GateBackend` trait (used by `auth_service`, `config_service`, `cron`, `app`, `cluster`, `environment`, `capability`) and `GovernanceEngine` struct (used by `hnsw_service`, `profile_store`, `wasm_runner`). Both correctly gate mutations.
- All chain events use the `#[cfg(feature = "exochain")]` feature gate consistently.
- Chain logging always occurs AFTER the mutation (correct for audit -- if the mutation fails, no event is logged for items like revoke_token and remove_peer; successful operations are always logged).
- The `auth_service` has both `governance_gate: Option<Arc<dyn GateBackend>>` and `chain_manager` fields, correctly wired via builder methods.
