# ExoChain/Governance Gap Fix Plan

**Date**: 2026-04-14
**Gaps**: 5 critical, 16 high, 30+ medium
**Strategy**: 8 agents, each handling a logical group of files
**Rule**: Every state-modifying public method gets chain logging. Critical/High paths also get governance gating.

---

## Agent Assignments (8 agents)

### Agent 1: AUTH + SECRETS (Critical)
**Files**: `auth_service.rs`, `config_service.rs`
**Gaps**: 6 (all Critical/High)
- [ ] `auth_service.rs:register_credential` → chain: `auth.credential.register`
- [ ] `auth_service.rs:rotate_credential` → chain: `auth.credential.rotate`
- [ ] `auth_service.rs:request_token` → chain: `auth.token.issue`
- [ ] `auth_service.rs:revoke_token` → chain: `auth.token.revoke`
- [ ] `auth_service.rs:authenticate` → chain: `auth.attempt` (log success/fail)
- [ ] `config_service.rs:set` → chain: `config.set` + governance gate
- [ ] `config_service.rs:delete` → chain: `config.delete` + governance gate
- [ ] `config_service.rs:set_secret` → chain: `config.secret.set` + governance gate
**Governance**: Add governance gate to credential registration and secret storage.

### Agent 2: PROFILES + VECTORS (Critical/High)
**Files**: `profile_store.rs`, `hnsw_service.rs`
**Gaps**: 8
- [ ] `profile_store.rs:create_profile` → chain: `profile.create` + governance gate
- [ ] `profile_store.rs:delete_profile` → chain: `profile.delete` + governance gate
- [ ] `profile_store.rs:switch_profile` → chain: `profile.switch`
- [ ] `profile_store.rs:insert` → chain: `profile.vector.insert`
- [ ] `hnsw_service.rs:insert` → chain: `hnsw.insert`
- [ ] `hnsw_service.rs:clear` → chain: `hnsw.clear` + governance gate
- [ ] `hnsw_service.rs:save_to_file` → chain: `hnsw.save`
- [ ] `hnsw_service.rs:load_from_file` → chain: `hnsw.load`

### Agent 3: APPS + CRON (High)
**Files**: `app.rs`, `cron.rs`
**Gaps**: 8
- [ ] `app.rs:install` → chain: `app.install` + governance gate
- [ ] `app.rs:remove` → chain: `app.remove` + governance gate
- [ ] `app.rs:start` → chain: `app.start`
- [ ] `app.rs:stop` → chain: `app.stop`
- [ ] `app.rs:transition_to` → chain: `app.transition`
- [ ] `cron.rs:add_job` → chain: `cron.add` + governance gate
- [ ] `cron.rs:remove_job` → chain: `cron.remove`
- [ ] `cron.rs:tick` → chain: `cron.execute` (log which job fired)

### Agent 4: CLUSTER + CAPABILITY + ENVIRONMENT (High)
**Files**: `cluster.rs`, `capability.rs`, `environment.rs`
**Gaps**: 8
- [ ] `cluster.rs:add_peer` → chain: `cluster.peer.add` + governance gate
- [ ] `cluster.rs:remove_peer` → chain: `cluster.peer.remove` + governance gate
- [ ] `cluster.rs:update_state` → chain: `cluster.peer.state`
- [ ] `capability.rs:request_elevation` → chain: `capability.elevate` + governance gate
- [ ] `environment.rs:register` → chain: `env.register`
- [ ] `environment.rs:set_active` → chain: `env.switch` + governance gate
- [ ] `environment.rs:remove` → chain: `env.remove`

### Agent 5: CAUSAL GRAPH + ARTIFACTS (Medium)
**Files**: `causal.rs`, `artifact_store.rs`
**Gaps**: 6
- [ ] `causal.rs:add_node` → chain: `causal.node.add`
- [ ] `causal.rs:remove_node` → chain: `causal.node.remove`
- [ ] `causal.rs:link` → chain: `causal.edge.add`
- [ ] `causal.rs:unlink` → chain: `causal.edge.remove`
- [ ] `causal.rs:clear` → chain: `causal.clear` + governance gate
- [ ] `artifact_store.rs:store` → chain: `artifact.store`
- [ ] `artifact_store.rs:remove` → chain: `artifact.remove`

### Agent 6: CONTAINERS + PROCESSES + WASM (Medium/High)
**Files**: `container.rs`, `process.rs`, `wasm_runner/runner.rs`
**Gaps**: 8
- [ ] `container.rs:start_container` → chain: `container.start`
- [ ] `container.rs:stop_container` → chain: `container.stop`
- [ ] `container.rs:configure` → chain: `container.configure`
- [ ] `process.rs:insert` → chain: `process.register`
- [ ] `process.rs:remove` → chain: `process.deregister`
- [ ] `process.rs:update_state` → chain: `process.state`
- [ ] `wasm_runner/runner.rs:execute` → chain: `wasm.execute` + governance gate
- [ ] `agency.rs:add_child/remove_child` → chain: `agent.hierarchy`

### Agent 7: MESH + PERSISTENCE (Medium)
**Files**: `mesh.rs`, `mesh_service.rs`, `mesh_artifact.rs`, `mesh_ipc.rs`, `persistence.rs`, `reconciler.rs`
**Gaps**: 8
- [ ] `mesh.rs:*` → chain: `mesh.peer.*`
- [ ] `mesh_service.rs:register` → chain: `mesh.service.register`
- [ ] `mesh_service.rs:deregister` → chain: `mesh.service.deregister`
- [ ] `mesh_artifact.rs:store` → chain: `mesh.artifact.store`
- [ ] `mesh_artifact.rs:fetch` → chain: `mesh.artifact.fetch`
- [ ] `mesh_ipc.rs:send` → chain: `mesh.ipc.send`
- [ ] `persistence.rs:save` → chain: `kernel.save`
- [ ] `persistence.rs:load` → chain: `kernel.load`
- [ ] `reconciler.rs:reconcile` → chain: `reconciler.action`

### Agent 8: CORE + GRAPHIFY + WEAVE (Medium)
**Files**: `agent/sandbox.rs`, `session.rs`, `workspace/mod.rs`, `tools/registry.rs`, `graphify/build.rs`, `graphify/ingest.rs`, `graphify/pipeline.rs`, `graphify/hooks.rs`, `init_cmd.rs`
**Gaps**: 10
- [ ] `sandbox.rs:execute` → chain: `sandbox.execute` + governance gate
- [ ] `session.rs:create` → chain: `session.create`
- [ ] `session.rs:destroy` → chain: `session.destroy`
- [ ] `workspace/mod.rs:create` → chain: `workspace.create`
- [ ] `workspace/config.rs:update` → chain: `workspace.config`
- [ ] `tools/registry.rs:register` → chain: `tool.register`
- [ ] `graphify/build.rs:build_graph` → chain: `graphify.build`
- [ ] `graphify/ingest.rs:ingest` → chain: `graphify.ingest`
- [ ] `graphify/pipeline.rs:run` → chain: `graphify.pipeline`
- [ ] `graphify/hooks.rs:register` → chain: `graphify.hook`
- [ ] `init_cmd.rs:init` → chain: `project.init`

---

## Implementation Pattern (all agents follow this)

### Chain Logging
```rust
// At the START of each state-modifying method:
#[cfg(feature = "exochain")]
if let Some(cm) = self.chain_manager() {
    cm.append(
        "subsystem_name",
        "event.kind",
        Some(serde_json::json!({
            "target": target_id,
            "action": "what_happened",
            // relevant parameters
        })),
    );
}
```

### Governance Gating (Critical/High only)
```rust
// BEFORE the mutation:
#[cfg(feature = "exochain")]
if let Some(gate) = self.governance_gate() {
    let effect = EffectVector::new(risk, fairness, privacy, novelty, security);
    if !gate.evaluate("action_name", &effect) {
        return Err(KernelError::GovernanceDenied("action_name".into()));
    }
}
```

### Chain Event Constants
Each agent adds `EVENT_KIND_*` constants to `chain.rs`:
```rust
pub const EVENT_KIND_AUTH_CREDENTIAL_REGISTER: &str = "auth.credential.register";
pub const EVENT_KIND_PROFILE_CREATE: &str = "profile.create";
// etc.
```

---

## Rules
1. NEVER break existing tests
2. ALL chain logging is behind `#[cfg(feature = "exochain")]`
3. ALL governance gates are behind `#[cfg(feature = "exochain")]`
4. Methods that currently return `()` may need to return `Result<()>` for governance denial
5. Each agent runs `scripts/build.sh check` before finishing
6. Each agent writes to isolated worktree

---

## Summary

| Agent | Severity | Files | Gaps Fixed |
|-------|----------|-------|------------|
| 1 | Critical | auth, config | 8 |
| 2 | Critical/High | profiles, hnsw | 8 |
| 3 | High | apps, cron | 8 |
| 4 | High | cluster, capability, env | 7 |
| 5 | Medium | causal, artifacts | 7 |
| 6 | Medium/High | containers, process, wasm | 8 |
| 7 | Medium | mesh, persistence | 9 |
| 8 | Medium | core, graphify, weave | 11 |
| **Total** | | **30+ files** | **66 gaps** |
