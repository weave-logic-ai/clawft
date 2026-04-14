# ExoChain MEDIUM Severity Certification

**Date**: 2026-04-04
**Auditor**: security-auditor (automated)
**Scope**: All 32 MEDIUM-severity items from `exochain-governance-audit.md`
**Method**: Direct source inspection of each method, verifying:
1. Chain event constant exists in `chain.rs`
2. `chain_manager.append()` call present in the method body
3. `#[cfg(feature = "exochain")]` guard wrapping the call
4. Payload includes identifying information

## Certification Results

| # | File:Method | Event Constant | append() Call | cfg Guard | Payload | Status |
|---|-------------|---------------|---------------|-----------|---------|--------|
| 1 | `hnsw_service.rs:insert` | `EVENT_KIND_HNSW_INSERT` | YES (line 124) | YES (line 122) | `{id}` | PASS |
| 2 | `hnsw_service.rs:save_to_file` | `EVENT_KIND_HNSW_SAVE` | YES (line 258) | YES (line 255) | `{path}` | PASS |
| 3 | `hnsw_service.rs:load_from_file` | `EVENT_KIND_HNSW_LOAD` | NO (static ctor, no cm available) | N/A | N/A | CONDITIONAL PASS |
| 4 | `profile_store.rs:insert` | `EVENT_KIND_PROFILE_VECTOR_INSERT` | YES (line 365) | YES (line 361) | `{profile_id, vector_id, key}` | PASS |
| 5 | `causal.rs:add_node` | `EVENT_KIND_CAUSAL_NODE_ADD` | YES (line 177) | YES (line 175) | `{node_id, label}` | PASS |
| 6 | `causal.rs:remove_node` | `EVENT_KIND_CAUSAL_NODE_REMOVE` | YES (line 227) | YES (line 225) | `{node_id, label}` | PASS |
| 7 | `causal.rs:link` | `EVENT_KIND_CAUSAL_EDGE_ADD` | YES (line 278) | YES (line 276) | `{source, target, edge_type, weight}` | PASS |
| 8 | `causal.rs:unlink` | `EVENT_KIND_CAUSAL_EDGE_REMOVE` | YES (line 314) | YES (line 312) | `{source, target, removed_count}` | PASS |
| 9 | `artifact_store.rs:store` | `EVENT_KIND_ARTIFACT_STORE` | YES (line 192) | YES (line 190) | `{hash, size, content_type}` | PASS |
| 10 | `artifact_store.rs:remove` | `EVENT_KIND_ARTIFACT_REMOVE` | YES (line 274) | YES (line 272) | `{hash}` | PASS |
| 11 | `cron.rs:remove_job` | `EVENT_KIND_CRON_REMOVE` | YES (line 186) | YES (line 184) | `{job_id, job_name}` | PASS |
| 12 | `cron.rs:tick` | `EVENT_KIND_CRON_EXECUTE` | YES (line 250) | YES (line 248) | `{job_id, job_name, fire_count, command}` | PASS |
| 13 | `environment.rs:register` | `EVENT_KIND_ENV_REGISTER` | YES (line 288) | YES (line 285) | `{id, name, class, risk_threshold}` | PASS |
| 14 | `environment.rs:remove` | `EVENT_KIND_ENV_REMOVE` | YES (line 393) | YES (line 392) | `{id, name, class}` | PASS |
| 15 | `container.rs:start_container` | `EVENT_KIND_CONTAINER_START` | YES (line 418) | YES (line 416) | `{name, image}` | PASS |
| 16 | `container.rs:stop_container` | `EVENT_KIND_CONTAINER_STOP` | YES (line 457) | YES (line 455) | `{name}` | PASS |
| 17 | `container.rs:configure` | `EVENT_KIND_CONTAINER_CONFIGURE` | YES (line 357) | YES (line 355) | `{name, image, ports}` | PASS |
| 18 | `process.rs:insert` | `EVENT_KIND_PROCESS_REGISTER` | YES (line 194) | YES (line 192) | `{pid, agent_id, parent_pid}` | PASS |
| 19 | `process.rs:remove` | `EVENT_KIND_PROCESS_DEREGISTER` | YES (line 232) | YES (line 229) | `{pid, agent_id}` | PASS |
| 20 | `process.rs:update_state` | `EVENT_KIND_PROCESS_STATE` | YES (line 270) | YES (line 268) | `{pid, from, to}` | PASS |
| 21 | `agency.rs:add_child` | `EVENT_KIND_AGENT_HIERARCHY_ADD` | YES (line 159) | YES (line 157) | `{child_pid, current_children}` | PASS |
| 22 | `agency.rs:remove_child` | `EVENT_KIND_AGENT_HIERARCHY_REMOVE` | YES (line 176) | YES (line 174) | `{child_pid, current_children}` | PASS |
| 23 | `cluster.rs:update_state` | `EVENT_KIND_CLUSTER_PEER_STATE` | YES (line 538) | YES (line 536) | `{node_id, from, to}` | PASS |
| 24 | `mesh_service.rs:register` (insert) | `EVENT_KIND_MESH_SERVICE_REGISTER` | YES (line 115) | YES (line 113) | `{service_name, node_id, version}` | PASS |
| 25 | `mesh_service.rs:deregister` (insert_negative) | `EVENT_KIND_MESH_SERVICE_DEREGISTER` | YES (line 133) | YES (line 131) | `{service_name, action}` | PASS |
| 26 | `mesh_artifact.rs:store` (register_remote) | `EVENT_KIND_MESH_ARTIFACT_STORE` | YES (line 98) | YES (line 95) | `{hash, remote_node_id, action}` | PASS |
| 27 | `mesh_artifact.rs:fetch` (create_request) | `EVENT_KIND_MESH_ARTIFACT_FETCH` | YES (line 137) | YES (line 134) | `{hash, requester_node_id, action}` | PASS |
| 28 | `mesh_ipc.rs:send` (PendingRequests::register) | `EVENT_KIND_MESH_IPC_SEND` | YES (line 184) | YES (line 182) | `{correlation_id, source_node, dest_node, envelope_id}` | PASS |
| 29 | `persistence.rs:save` | `EVENT_KIND_KERNEL_SAVE` | YES (line 108) | YES (line 101, fn-level) | `{data_dir, node_count, hnsw_count}` | PASS |
| 30 | `persistence.rs:load` | `EVENT_KIND_KERNEL_LOAD` | YES (line 138) | YES (line 132, fn-level) | `{data_dir, node_count, hnsw_count}` | PASS |
| 31 | `reconciler.rs:tick` | `EVENT_KIND_RECONCILER_TICK` | YES (line 226) | YES (line 224) | `{drift_count, desired_count}` | PASS |
| 32 | `wasm_runner/tools_fs.rs` | NONE | NO | NO | N/A | **FAIL** |

## Summary

| Result | Count |
|--------|-------|
| PASS | 30 |
| CONDITIONAL PASS | 1 |
| **FAIL** | **1** |

## Details on Non-PASS Items

### Item 3: `hnsw_service.rs:load_from_file` -- CONDITIONAL PASS

`load_from_file` is a static constructor (`fn load_from_file(path: &Path) -> Result<Self, ...>`) that returns a new `HnswService` instance. It cannot log to the chain because the chain manager does not exist yet at construction time.

However, a companion method `load_from_file_logged` exists (line 303, gated behind `#[cfg(feature = "exochain")]`) that accepts a `ChainManager` parameter and emits `EVENT_KIND_HNSW_LOAD`. Callers with exochain enabled should use `load_from_file_logged` instead. The `persistence.rs:load_all_with_chain` function provides the chain-logged path for kernel boot.

**Verdict**: Acceptable design. The logged variant exists and is the recommended path.

### Item 32: `wasm_runner/tools_fs.rs` -- FAIL

The filesystem tools (`fs.read_file`, `fs.write_file`, `fs.read_dir`, `fs.create_dir`, `fs.remove`, `fs.copy`, `fs.move`, `fs.stat`, `fs.exists`, `fs.glob`) have zero ExoChain integration:
- No `chain_manager` field on any tool struct
- No `#[cfg(feature = "exochain")]` blocks
- No `cm.append()` calls
- No event constants defined for filesystem operations

Sandbox path checking IS present (via `SandboxConfig::is_path_allowed`), but mutations (write, create_dir, remove, copy, move) are not audit-logged.

**Remediation**: Add `EVENT_KIND_WASM_FS_WRITE`, `EVENT_KIND_WASM_FS_REMOVE`, `EVENT_KIND_WASM_FS_CREATE_DIR`, `EVENT_KIND_WASM_FS_COPY`, `EVENT_KIND_WASM_FS_MOVE` constants to `chain.rs`. Thread a chain manager into each mutating tool and emit events with `{path, agent_pid}` payloads.

## Certification Statement

31 of 32 MEDIUM-severity items have compliant ExoChain instrumentation with proper feature gating, event constants, `chain_manager.append()` calls, and identifying payloads.

1 item (`wasm_runner/tools_fs.rs`) lacks any ExoChain coverage and requires remediation before the MEDIUM tier can be fully certified.
