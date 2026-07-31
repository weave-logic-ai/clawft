# ExoChain MEDIUM Severity Certification

**Date**: 2026-04-04 (original audit)  
**Re-audit**: 2026-07-31 (WEFT-547 closeout)  
**Auditor**: security-auditor (automated) + developer re-walk  
**Scope**: All 32 MEDIUM-severity items from `exochain-governance-audit.md`  
**Method**: Direct source inspection of each method, verifying:
1. Chain event constant exists in `chain.rs`
2. `chain_manager.append()` call present in the method body **or**
   `clawft_core::chain_event::push_chain_event` for wasm builtin tools
3. `#[cfg(feature = "exochain")]` guard wrapping the call
4. Payload includes identifying information

## Certification Results

| # | File:Method | Event Constant | append() Call | cfg Guard | Payload | Status |
|---|-------------|---------------|---------------|-----------|---------|--------|
| 1 | `hnsw_service.rs:insert` | `EVENT_KIND_HNSW_INSERT` | YES | YES | `{id}` | PASS |
| 2 | `hnsw_service.rs:save_to_file` | `EVENT_KIND_HNSW_SAVE` | YES | YES | `{path}` | PASS |
| 3 | `hnsw_service.rs:load_from_file` | `EVENT_KIND_HNSW_LOAD` | NO (static ctor, no cm available) | N/A | N/A | CONDITIONAL PASS |
| 4 | `profile_store.rs:insert` | `EVENT_KIND_PROFILE_VECTOR_INSERT` | YES | YES | `{profile_id, vector_id, key}` | PASS |
| 5 | `causal.rs:add_node` | `EVENT_KIND_CAUSAL_NODE_ADD` | YES | YES | `{node_id, label}` | PASS |
| 6 | `causal.rs:remove_node` | `EVENT_KIND_CAUSAL_NODE_REMOVE` | YES | YES | `{node_id, label}` | PASS |
| 7 | `causal.rs:link` | `EVENT_KIND_CAUSAL_EDGE_ADD` | YES | YES | `{source, target, edge_type, weight}` | PASS |
| 8 | `causal.rs:unlink` | `EVENT_KIND_CAUSAL_EDGE_REMOVE` | YES | YES | `{source, target, removed_count}` | PASS |
| 9 | `artifact_store.rs:store` | `EVENT_KIND_ARTIFACT_STORE` | YES | YES | `{hash, size, content_type}` | PASS |
| 10 | `artifact_store.rs:remove` | `EVENT_KIND_ARTIFACT_REMOVE` | YES | YES | `{hash}` | PASS |
| 11 | `cron.rs:remove_job` | `EVENT_KIND_CRON_REMOVE` | YES | YES | `{job_id, job_name}` | PASS |
| 12 | `cron.rs:tick` | `EVENT_KIND_CRON_EXECUTE` | YES | YES | `{job_id, job_name, fire_count, command}` | PASS |
| 13 | `environment.rs:register` | `EVENT_KIND_ENV_REGISTER` | YES | YES | `{id, name, class, risk_threshold}` | PASS |
| 14 | `environment.rs:remove` | `EVENT_KIND_ENV_REMOVE` | YES | YES | `{id, name, class}` | PASS |
| 15 | `container.rs:start_container` | `EVENT_KIND_CONTAINER_START` | YES | YES | `{name, image}` | PASS |
| 16 | `container.rs:stop_container` | `EVENT_KIND_CONTAINER_STOP` | YES | YES | `{name}` | PASS |
| 17 | `container.rs:configure` | `EVENT_KIND_CONTAINER_CONFIGURE` | YES | YES | `{name, image, ports}` | PASS |
| 18 | `process.rs:insert` | `EVENT_KIND_PROCESS_REGISTER` | YES | YES | `{pid, agent_id, parent_pid}` | PASS |
| 19 | `process.rs:remove` | `EVENT_KIND_PROCESS_DEREGISTER` | YES | YES | `{pid, agent_id}` | PASS |
| 20 | `process.rs:update_state` | `EVENT_KIND_PROCESS_STATE` | YES | YES | `{pid, from, to}` | PASS |
| 21 | `agency.rs:add_child` | `EVENT_KIND_AGENT_HIERARCHY_ADD` | YES | YES | `{child_pid, current_children}` | PASS |
| 22 | `agency.rs:remove_child` | `EVENT_KIND_AGENT_HIERARCHY_REMOVE` | YES | YES | `{child_pid, current_children}` | PASS |
| 23 | `cluster.rs:update_state` | `EVENT_KIND_CLUSTER_PEER_STATE` | YES | YES | `{node_id, from, to}` | PASS |
| 24 | `mesh_service.rs:register` (insert) | `EVENT_KIND_MESH_SERVICE_REGISTER` | YES | YES | `{service_name, node_id, version}` | PASS |
| 25 | `mesh_service.rs:deregister` (insert_negative) | `EVENT_KIND_MESH_SERVICE_DEREGISTER` | YES | YES | `{service_name, action}` | PASS |
| 26 | `mesh_artifact.rs:store` (register_remote) | `EVENT_KIND_MESH_ARTIFACT_STORE` | YES | YES | `{hash, remote_node_id, action}` | PASS |
| 27 | `mesh_artifact.rs:fetch` (create_request) | `EVENT_KIND_MESH_ARTIFACT_FETCH` | YES | YES | `{hash, requester_node_id, action}` | PASS |
| 28 | `mesh_ipc.rs:send` (PendingRequests::register) | `EVENT_KIND_MESH_IPC_SEND` | YES | YES | `{correlation_id, source_node, dest_node, envelope_id}` | PASS |
| 29 | `persistence.rs:save` | `EVENT_KIND_KERNEL_SAVE` | YES (`save_all_with_chain`) | YES (fn-level) | `{data_dir, node_count, hnsw_count}` | PASS |
| 30 | `persistence.rs:load` | `EVENT_KIND_KERNEL_LOAD` | YES (`load_all_with_chain`) | YES (fn-level) | `{data_dir, node_count, hnsw_count}` | PASS |
| 31 | `reconciler.rs:tick` | `EVENT_KIND_RECONCILER_TICK` | YES | YES | `{drift_count, desired_count}` | PASS |
| 32 | `wasm_runner/tools_fs.rs` (mutating tools) | `EVENT_KIND_WASM_FS_*` | YES (`push_chain_event`) | YES | `{path, …}` | **PASS** |

## Summary

| Result | Count (2026-04-04) | Count (2026-07-31) |
|--------|-------------------:|-------------------:|
| PASS | 30 | **31** |
| CONDITIONAL PASS | 1 | 1 |
| **FAIL** | **1** | **0** |

## Details on Non-PASS Items

### Item 3: `hnsw_service.rs:load_from_file` -- CONDITIONAL PASS

`load_from_file` is a static constructor (`fn load_from_file(path: &Path) -> Result<Self, ...>`) that returns a new `HnswService` instance. It cannot log to the chain because the chain manager does not exist yet at construction time.

However, a companion method `load_from_file_logged` exists (gated behind `#[cfg(feature = "exochain")]`) that accepts a `ChainManager` parameter and emits `EVENT_KIND_HNSW_LOAD`. Callers with exochain enabled should use `load_from_file_logged` instead. The `persistence.rs:load_all_with_chain` function provides the chain-logged path for kernel boot.

**Verdict**: Acceptable design. The logged variant exists and is the recommended path. **No change required for WEFT-547.**

### Item 32: `wasm_runner/tools_fs.rs` -- PASS (remediated)

**Previously FAIL (2026-04-04).** As of the 2026-07-31 re-walk, mutating filesystem tools emit chain events:

| Tool | Kind | Mechanism |
|------|------|-----------|
| `fs.write_file` | `wasm.fs.write` | `push_chain_event` |
| `fs.create_dir` | `wasm.fs.create_dir` | `push_chain_event` |
| `fs.remove` | `wasm.fs.remove` | `push_chain_event` |
| `fs.copy` | `wasm.fs.copy` | `push_chain_event` |
| `fs.move` | `wasm.fs.move` | `push_chain_event` |

Constants live in `chain.rs` as `EVENT_KIND_WASM_FS_*`. Sandbox path checks remain. Read-only tools are not mutation-audited (by design).

**Verdict**: PASS. MEDIUM tier fully certifiable.

## Certification Statement

**32 of 32** MEDIUM-severity items are certified (31 PASS + 1 CONDITIONAL PASS).  
**0 FAIL** remains. See `exochain-security-plan.md` for the WEFT-547 closeout ledger and explicit low-severity deferrals.
