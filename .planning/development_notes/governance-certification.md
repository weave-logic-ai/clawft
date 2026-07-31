# Governance Gate Certification

**Audit Date:** 2026-04-04
**Auditor:** Security Auditor Agent (V3)
**Scope:** All `crates/clawft-kernel/src/`, `crates/clawft-core/src/`, `crates/clawft-weave/src/`
**Feature Flag:** All governance gates are behind `#[cfg(feature = "exochain")]`

---

## Executive Summary

The governance gate system has been expanded from the original 2 call sites to **19 distinct governance check call sites** across 12 files. All 14 high-priority items from the audit checklist are covered. Two minor gaps were identified (see "Missing Gates" section below).

**Verdict: CERTIFIED with 2 minor gaps noted.**

---

## Governance Gate Inventory

### HIGH-PRIORITY Items (Required by Audit Checklist)

| File | Method | Gate Present? | Check Position | Denial Handling | EffectVector | CERTIFIED |
|------|--------|:---:|-----------|----------------|-------------|:---:|
| auth_service.rs | register_credential | YES | Before mutation (L275) | Returns `KernelError::GovernanceDenied` | GateEffectKind::AuthCredentialRegister (risk=0.5, privacy=0.6, security=0.8) | YES |
| auth_service.rs | register_hashed_credential | YES | Before mutation (L488) | Returns `KernelError::GovernanceDenied` | GateEffectKind::AuthCredentialRegister (risk=0.5, privacy=0.6, security=0.8) | YES |
| config_service.rs | set_secret | YES | Before secret write (L444) | Returns `KernelError::GovernanceDenied` | GateEffectKind::ConfigSecretSet (risk=0.55, privacy=0.85, security=0.9) | YES |
| profile_store.rs | delete_profile | YES | Before deletion (L247) | Returns `ProfileError::GovernanceDenied` | risk=0.7, privacy=0.4 | YES |
| hnsw_service.rs | clear | YES | Before bulk destruction (L212) | Returns `Err(String)` | risk=0.8 | YES |
| causal.rs | clear | YES | Before graph wipe (L376) | Returns `KernelError::GovernanceDenied` | risk=0.8 | YES |
| app.rs | install | YES | Before installation (L475) | Returns `AppError::GovernanceDenied` | risk=0.3, security=0.3 | YES |
| app.rs | remove | YES | Before removal (L621) | Returns `AppError::GovernanceDenied` | risk=0.4, security=0.2 | YES |
| cron.rs | add_job | YES | Before scheduling (L126) | Returns `CronError::GovernanceDenied` | GateEffectKind::CronAdd (risk=0.2, security=0.1) | YES |
| cluster.rs | add_peer | YES | Before trust boundary expansion (L414) | Returns `ClusterError::AuthFailed` | risk=0.4, security=0.3 | YES |
| cluster.rs | remove_peer | YES | Before trust boundary contraction (L487) | Returns `ClusterError::AuthFailed` | risk=0.3, security=0.2 | YES |
| capability.rs | request_elevation_gated | YES | Before privilege escalation (L581) | Returns `ElevationResult::Denied` | risk=0.5, security=0.5 | YES |
| environment.rs | set_active | YES | Before governance threshold change (L315) | Returns `EnvironmentError::NotFound` | risk=0.3, security=0.2 | YES |
| wasm_runner/runner.rs | execute | YES | Before arbitrary code execution (L199) | Returns `WasmError::GovernanceDenied` | risk=0.4, security=0.5 | YES |

### Additional Governance Gates (Beyond Checklist)

| File | Method | Gate Present? | Check Position | Denial Handling | EffectVector | CERTIFIED |
|------|--------|:---:|-----------|----------------|-------------|:---:|
| profile_store.rs | create_profile | YES | Before creation (L199) | Returns `ProfileError::GovernanceDenied` | risk=0.3, privacy=0.2 | YES |
| config_service.rs | set (config) | YES | Before config write (L199) | Returns `KernelError::GovernanceDenied` | GateEffectKind::ConfigSet (risk=0.35, security=0.4) | YES |
| config_service.rs | delete (config) | YES | Before config delete (L267) | Returns `KernelError::GovernanceDenied` | GateEffectKind::ConfigDelete (risk=0.4, security=0.45) | YES |
| a2a.rs | send (routing) | YES | Before message delivery (L262) | Returns `KernelError::CapabilityDenied` | GateEffectKind::A2aRouting (risk=0.25, security=0.3) | YES |
| http_api.rs | handle_request | YES | Before governance evaluation (L212) | Returns GovernResponse with decision | Extracted from context map | YES |

### Sandbox (clawft-core) -- Policy-Based (Not GovernanceGate)

| File | Method | Gate Type | Check Position | Denial Handling | CERTIFIED |
|------|--------|:---:|-----------|----------------|:---:|
| sandbox.rs | check_command | SandboxPolicy | Before command execution (L124) | Returns `Err(String)` | YES (policy-layer) |
| sandbox.rs | check_file_read | SandboxPolicy | Before file read (L91) | Returns `Err(String)` | YES (policy-layer) |
| sandbox.rs | check_file_write | SandboxPolicy | Before file write (L106) | Returns `Err(String)` | YES (policy-layer) |
| sandbox.rs | check_network | SandboxPolicy | Before network access (L77) | Returns `Err(String)` | YES (policy-layer) |
| sandbox.rs | check_tool | SandboxPolicy | Before tool invocation (L63) | Returns `Err(String)` | YES (policy-layer) |

Note: The sandbox uses `SandboxPolicy` (capability-based enforcement), not `GovernanceGate`. It emits `chain_event!` markers for ExoChain auditing. This is the correct design -- sandbox is an enforcement layer, governance is a policy layer.

---

## Governance Architecture

### Gate Backends

Two patterns are used:

1. **`GateBackend` trait** (used by auth_service, config_service, app, cluster, capability, environment, a2a, cron): The `GovernanceGate` struct implements `GateBackend::check()` which takes `(agent_id, action, context_json)` and returns `GateDecision::{Permit, Deny, Defer, Warn}`.

2. **`GovernanceEngine::evaluate()`** (used by profile_store, hnsw_service, causal, wasm_runner, http_api): Takes a `GovernanceRequest` with `EffectVector` dimensions and returns `GovernanceResult` with `GovernanceDecision::{Permit, PermitWithWarning, Deny, EscalateToHuman}`.

### Daemon Integration (clawft-weave/src/daemon.rs)

The daemon constructs a `GovernanceGate` at L1192 with:
- Threshold: 0.8
- Human approval: false
- Rules: `exec-guard` (Blocking/Judicial), `cron-warn` (Warning/Executive)
- Chain manager attached for audit trail

### EffectVector Dimensions Used

| Dimension | Files Using It |
|-----------|---------------|
| risk | profile_store (0.3, 0.7), causal (0.8), hnsw_service (0.8), app (0.3, 0.4), cluster (0.3, 0.4), capability (0.5), environment (0.3), cron (0.2), wasm_runner (0.4) |
| security | app (0.2, 0.3), cluster (0.2, 0.3), capability (0.5), environment (0.2), cron (0.1), wasm_runner (0.5) |
| privacy | profile_store (0.2, 0.4) |
| fairness | (not used in production gates) |
| novelty | (not used in production gates) |

---

## Missing Gates (Gaps Identified)

### GAP-1: `config_service.rs::delete_typed` (MEDIUM severity) — **CLOSED (WEFT-547 / 2026-07-31)**

**File:** `crates/clawft-kernel/src/config_service.rs`  
**Issue (historical):** The `delete_typed` method removed typed configuration entries without any governance check while sibling `delete` was gated.  
**Resolution:** `delete_typed` now calls `GateBackend::check("config-service", "config.delete_typed", …)` before mutation and returns `KernelError::GovernanceDenied` on deny. Medium gap count → 0.

### GAP-2: Sandbox `check_command` lacks GovernanceGate integration (LOW severity) — **DEFERRED**

**File:** `crates/clawft-core/src/agent/sandbox.rs`  
**Issue:** The sandbox uses `SandboxPolicy` for enforcement and emits `chain_event!` markers, but does not consult a `GovernanceGate` or `GovernanceEngine`. This is architecturally intentional (sandbox = enforcement, governance = policy), but means governance policies cannot dynamically override sandbox decisions.
**Risk:** Low. The sandbox is a strict enforcement layer. GovernanceGate integration would add defense-in-depth but is not a gap in the current architecture.
**Deferral:** Optional defense-in-depth; not required for medium-tier certification.

### GAP-3: `cron.rs::remove_job` lacks governance (LOW severity) — **DEFERRED**

**File:** `crates/clawft-kernel/src/cron.rs`  
**Issue:** `add_job` is gated but `remove_job` is not. Removing a governance-mandated cron job (e.g., audit rotation) could weaken the security posture.
**Mitigation present:** `EVENT_KIND_CRON_REMOVE` chain audit on remove.  
**Deferral:** Governance parity with `add_job` deferred to 0.9.x polish (low severity).

---

## Verification Methodology

1. Full-text search for `GovernanceGate`, `GateBackend`, `governance_gate`, `governance_engine`, `EffectVector`, `governance.evaluate`, `GovernanceDenied`, `GovernanceRequest` across all `.rs` files in kernel, core, and weave crates.
2. Cross-referenced every `gate.check()` and `engine.evaluate()` call site against the method it resides in.
3. Verified each check occurs BEFORE the mutation (correct placement).
4. Verified each check returns an error on denial (not silently ignored).
5. Scanned for destructive/sensitive methods (`delete`, `remove`, `clear`, `wipe`, `install`, `execute`) that lack governance.

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Total governance check call sites (production code) | 19+ (`delete_typed` included) |
| Files with governance gates | 12 |
| High-priority items covered | 14/14 (100%) |
| Open gaps | 2 low (GAP-2, GAP-3 deferred); **0 medium** |
| EffectVector dimensions actively used | 3 of 5 (risk, security, privacy) |
| Gate patterns | 2 (GateBackend trait, GovernanceEngine) |
| Feature flag | `exochain` (compile-time opt-in) |

**Certification Status: PASS** (reconfirmed WEFT-547 / 2026-07-31)

All 14 high-priority governance gates are present, correctly positioned before mutations, and return errors on denial. The sole medium gap (GAP-1 `delete_typed`) is closed. Remaining items are explicit low-severity deferrals.
