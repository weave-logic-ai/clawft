# Plane Board Inventory — WeftOS

> Generated: 2026-07-31 20:36 UTC
> Source: Plane workspace `weftos`
> Machine-readable DAG: [`plane-dag.json`](./plane-dag.json)
> Wave plan: [`plane-wave-plan.md`](./plane-wave-plan.md)

## Summary

| Metric | Count |
|--------|------:|
| Total tickets | 716 |
| Open | 15 |
| In Progress | 0 |
| Done | 681 |
| Cancelled | 20 |
| Dependency edges | 43 |
| Inferred domain edges | 0 |
| Parallel waves | 2 |

### Open by cycle

- **1.0.x**: 14
- **0.8.x**: 1

### Open by workstream

- **ws09-clawft-dashboard**: 9
- **ws03-pipeline**: 3
- **ws17-research**: 2
- **ws10-voice**: 1

### Open by priority

- **medium**: 5
- **low**: 10

---

## Complete ticket table

| WEFT | State | Pri | Cycle | WS | Lane | AC | Blocked-by | Blocks | Name |
|------|-------|-----|-------|----|------|----|------------|--------|------|
| WEFT-8 | Done | high | 0.7.x | ws14-deployment | A | strong | WEFT-251 | WEFT-19 | ws14: workspace deps — migrate clawft-* path-deps to [workspace.dependencies] inheritance |
| WEFT-9 | Cancelled | high | 0.7.x | ws01-core | B | strong | — | — | ws01: foundation — reconcile ADR-044 wasip1 vs .cargo/config wasip2 alias |
| WEFT-10 | Done | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: bootstrap — split workspace from global at loader for PermissionResolver ceiling |
| WEFT-11 | Done | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — implement Windows daemon transport (named pipes) for DaemonClient |
| WEFT-12 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — replace version_check curl shell-out with reqwest |
| WEFT-13 | Done | medium | 0.8.x | ws01-core | G | strong | — | — | ws01: platform — implement OPFS-backed BrowserFileSystem persistence |
| WEFT-14 | Done | low | 0.8.x | ws01-core | G | strong | — | — | ws01: platform — land OPFS-or-equivalent BrowserEnvironment persistence |
| WEFT-15 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: kernel-config — wire LogQuantizedStubConfig + SimdDistanceStubConfig runtime |
| WEFT-16 | Done | medium | 0.7.x | ws01-core | B | strong | — | — | ws01: security — rationalize lenient validate_mcp_tool_name vs strict variant |
| WEFT-17 | Cancelled | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — add chain.append RPC for weaver soul promote |
| WEFT-18 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: foundation — run ADR-010 v0.3 cancel-correctness audit on mesh select! branches |
| WEFT-19 | Done | low | 0.7.x | ws01-core | B | strong | WEFT-8 | — | ws01,ws14: publish-policy audit — flip 16 publish=false flags or document |
| WEFT-20 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: types — decide deny_unknown_fields lint mode for Config |
| WEFT-21 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: platform — document config_loader Layer 2 sync vs Layer 3 async asymmetry |
| WEFT-22 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: cli — remove TODO(E1) and TODO(C5) markers in workstream-I notes |
| WEFT-23 | Done | medium | 0.7.x | ws01-core | B | strong | — | — | ws01: cli — replace skills_cmd derived-on-first-sign placeholder pubkey with real Ed25519 |
| WEFT-24 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: planning — close out improvements.md Phase-5 sprint-tracker |
| WEFT-25 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: planning — archive stale 00-initial-sprint codebase-map / planning-summary |
| WEFT-26 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: types — clean up panic! macros in test-only canvas/provider/agent_bus arms |
| WEFT-27 | Done | high | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — apply tier check to fallback model selection |
| WEFT-28 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — HMAC the cost-tracker persistence file |
| WEFT-29 | Done | low | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — reject window_seconds=0 in Phase H validation |
| WEFT-30 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — redact RoutingDecision.reason to avoid info disclosure |
| WEFT-31 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — audit-log model_override bypasses (escalation already logs) |
| WEFT-32 | Done | high | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — add MCP tool-name namespace validation against wildcard ['*'] |
| WEFT-33 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — scaffold fuzz targets for 8 attack surfaces |
| WEFT-34 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-002 (DashMap vs RwLock<HashMap> benchmark) |
| WEFT-35 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-003 final review (escalation security) |
| WEFT-36 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-006 (config validation boundary) |
| WEFT-37 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — wire D1 per-path advisory locks for parallel tool execution |
| WEFT-38 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — wire evolution_ready flag → mutation.rs GA loop (ADR-017 flywheel) |
| WEFT-39 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — persist RetryModel learned weights across daemon restarts |
| WEFT-40 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — surface routing-decision history via admin endpoint |
| WEFT-41 | Todo | low | 1.0.x | ws03-pipeline | B | strong | — | — | ws03: research — Iteration 3 EML attention multi-param coordinated perturbation |
| WEFT-42 | Done | low | 0.9.x | ws03-pipeline | B | strong | — | — | ws03: kernel — wire sprint-16 two-tier EML coherence cadence |
| WEFT-43 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — decide consolidation of clawft-service-llm vs clawft-llm |
| WEFT-44 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: service-llm — handle non-string content (vision blocks / structured) in LlmClient |
| WEFT-45 | Done | medium | 0.8.x | ws03-pipeline | C | strong | — | — | ws03: routing — wire MicroLoraRouter (v3) once ruvllm-wasm lifts 11-pattern HNSW cap |
| WEFT-46 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — wire v2.5 sona-backed rerank step in HybridRouter |
| WEFT-47 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — add max_grantable_level field to RoutingConfig |
| WEFT-48 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: rate-limiter — expose rate-limiter metrics via admin endpoint (Element-09) |
| WEFT-49 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: rate-limiter — expose rate-limiter LRU maintenance via admin endpoint (Element-09) |
| WEFT-50 | Done | low | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: context-router — document Some(vec![]) tool_subset contract for plugin authors |
| WEFT-51 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: context-router — exhaustively test embedding-router cargo-feature-off path |
| WEFT-52 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — verify admin user x restricted channel interaction |
| WEFT-53 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — decide EML score-fusion in scope for 0.7.0 (FitnessScorer weights) |
| WEFT-54 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — review FitnessScorer.error_indicators allowlist (localization, jailbreak) |
| WEFT-55 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — verify experimental-attention CI build/test wiring |
| WEFT-56 | Done | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — define explicit pipeline-pass step in scripts/build.sh gate |
| WEFT-57 | Todo | low | 1.0.x | ws03-pipeline | B | strong | — | — | ws03: research — track 80+ heuristics from eml-synergy-scan |
| WEFT-58 | Todo | low | 1.0.x | ws03-pipeline | C | strong | — | — | ws03: research — track HNSW EML opportunities (adaptive ef, learned distance) |
| WEFT-59 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: cli — add weft skills approve / reject CLI for autogen .pending markers |
| WEFT-60 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: cli — add weft skills pending listing with generated SKILL.md preview |
| WEFT-61 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: architecture — decide fate of 8 orphaned clawft-plugin-* crates |
| WEFT-62 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: architecture — ADR documenting clawft-core vs clawft-kernel ToolRegistry split + mig |
| WEFT-63 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — implement shell-execution skill approval prompt at install time |
| WEFT-64 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: plugin — reconcile manifest formats (clawft.plugin.json vs .weftos-plugin.toml) |
| WEFT-65 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — add per-skill allowed-tools intersection validator at load time |
| WEFT-66 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: autogen — wire improve_skill_instructions / generate_skill_md_with_learning into age |
| WEFT-67 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: cli — add weft skills autogen {enable,disable,status} CLI |
| WEFT-68 | Done | medium | 0.8.x | ws04-plugin-skills | G | strong | — | — | ws04: observability — add WASM per-plugin fuel/memory observability |
| WEFT-69 | Done | high | 0.7.x | ws04-plugin-skills | B | strong | — | — | ws04: docs — document skill signing trust root location and rotation |
| WEFT-70 | Done | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: ux — add macOS-sandbox-downgrade warning to startup banner |
| WEFT-71 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: tests — add clawft.plugin.json schema roundtrip + version-compat test |
| WEFT-72 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — verify (or close) SkillContext::Fork status post-3F review M2 |
| WEFT-73 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: tests — land T39 plugin-lifecycle tests |
| WEFT-74 | Done | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — define pending-skill review timing (interactive prompt vs CLI) |
| WEFT-75 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: autogen — define filesystem allowlist semantics for autogenerated skills |
| WEFT-76 | Done | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — add weft skills refresh CLI for headless/CI scenarios |
| WEFT-77 | Done | low | 0.8.x | ws04-plugin-skills | E | strong | — | — | ws04: voice — drop or stub VoiceHandler trait placeholder for 0.7.0 |
| WEFT-78 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: scaffold — add Rust struct/parser for .weftos-plugin.toml or remove it |
| WEFT-79 | Done | high | 0.7.x | ws06-memory | C | strong | — | — | ws06: memory — route MemoryStore + SkillsLoader through WorkspaceContext |
| WEFT-80 | Cancelled | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: bootstrap — split workspace from global at loader for PermissionResolver ceiling (MW |
| WEFT-81 | Cancelled | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: governance — implement chain.append RPC for weaver soul promote (MW-3) |
| WEFT-82 | Done | high | 0.7.x | ws06-memory | C | strong | — | — | ws06: tests — convert overlay_probe.rs from #[ignore] to hermetic temp-workspace test (MW- |
| WEFT-83 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — add agent.workspace_root config key (MW-5) |
| WEFT-84 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — rebuild memory.rvf.json when MEMORY.md changes (MW-6) |
| WEFT-85 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: substrate — emit chain_event! for session.append on every appended turn (MW-7) |
| WEFT-86 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: workspace — align WorkspaceManager::delete with FR-W06 (MW-8) |
| WEFT-87 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: sessions — ship weft session gc (or self-cleanup migration path) (MW-9) |
| WEFT-88 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: workspace — update last_accessed in WorkspaceManager::load (MW-10) |
| WEFT-89 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: planning — backfill empty 08-memory-workspace decisions/blockers notes (MW-11) |
| WEFT-90 | Done | medium | 0.7.x | ws06-memory | C | strong | — | — | ws06: planning — re-walk 3g-review.md and mark each ISSUE fixed/open/won't-do (MW-12) |
| WEFT-91 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — decide whether FileIdentityProvider needs notify watcher (MW-13) |
| WEFT-92 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — decide binding-thread-mismatch policy refuse vs annotate (MW-14) |
| WEFT-93 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: embeddings — pick fate of rvf_stub.rs vs rvf_io.rs (MW-15) |
| WEFT-94 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: workspace — document or drop per-agent tool_state/ subdirectory (MW-16) |
| WEFT-95 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — route IdentityLoader::current through Platform::fs() (MW-17) |
| WEFT-96 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — define journal substrate read-on-every-turn path (WS-D1) |
| WEFT-97 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — substrate-backed Identity::source variant set (WS-D4 / WS-D5) |
| WEFT-98 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: kernel auth — add gate.check to revoke_token (DiD) |
| WEFT-99 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — re-enable auth middleware on /api/* and /ws |
| WEFT-100 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — replace CorsLayer::permissive() with deny-by-default |
| WEFT-101 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — add tower::limit::RateLimitLayer to /api/* and token endpoints |
| WEFT-102 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — add TokenStore::revoke_token + expired-token cleanup |
| WEFT-103 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: chain — add optional idempotency_key to ChainEvent (replay protection) |
| WEFT-104 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: tooling — add cargo audit to scripts/build.sh gate and CI |
| WEFT-105 | Done | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement K6.4 chain replay (LocalChain::tail_from + append_signed) |
| WEFT-106 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement K6.4 tree Merkle diff + signed mutations |
| WEFT-107 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement S10 key-rotation chain event + verifier |
| WEFT-108 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement IpcScope::Restricted browser default + browser_policy rules |
| WEFT-109 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — decide chain merge strategy (leader vs DAG) and split-brain handling |
| WEFT-110 | Done | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — decide and freeze KernelMessage wire format (JSON vs RVF) |
| WEFT-111 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — decide full libp2p-kad vs lighter DHT |
| WEFT-112 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — add InMemoryTransport / MockPeer / MockClock / FaultyTransport |
| WEFT-113 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — define Clock trait and inject into time-dependent components |
| WEFT-114 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: ci — add cargo check --target wasm32-unknown-unknown (no mesh) to CI |
| WEFT-115 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — define missing K6 protocol struct types and msg_type enum |
| WEFT-116 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — resolve mesh_adapter.rs vs mesh_ipc.rs and mesh/handshake.rs layout |
| WEFT-117 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — wire AssessmentTransport into daemon + add weft assess mesh-status |
| WEFT-118 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — add QUIC transport (quinn + snow) alongside TCP/WS |
| WEFT-119 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — make Mesh a SystemService with start/stop/health_check |
| WEFT-120 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — wire ClusterService to mesh peer discovery |
| WEFT-121 | Done | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement mesh time-sync (authority election, offset smoothing, mesh_time) |
| WEFT-122 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: services-api — wire axum handlers to http_facade types + SSE loop |
| WEFT-123 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: services-api — add HTTP facade integration tests once profile/pairing types land |
| WEFT-124 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — wire VectorBackend into DemocritusLoop |
| WEFT-125 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — add ecc.vector-config RPC endpoint |
| WEFT-126 | Done | low | 0.9.x | ws02-kernel | B | strong | — | — | ws02: vector — ship real DiskANN backend behind diskann feature once ruvector-diskann publ |
| WEFT-127 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — persist HNSW tombstones across save/load |
| WEFT-128 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — flip LogQuantized + SimdDistance is_available once ruvector-core PR #352 la |
| WEFT-129 | Done | low | 0.8.x | ws02-kernel | A | strong | — | — | ws02: kernel — ship real Wasmtime backend for spectral_embedding (or move to deferred) |
| WEFT-130 | Done | high | 0.7.x | ws02-kernel | B | strong | WEFT-554 | WEFT-554 | ws02: exo-resource-tree — replace permission.rs always-Allow stub with K1 ACL engine |
| WEFT-131 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: exo-resource-tree — implement DelegationCert lifecycle (grant/revoke + Ed25519 + exp |
| WEFT-132 | Cancelled | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: services-api — implement bridge.rs TODOs (skill, memory, config) |
| WEFT-133 | Done | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: services-api — add CSP middleware to API tower stack |
| WEFT-134 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: tests — resolve test-suite hang in clawft-kernel --lib aggregate run |
| WEFT-135 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: workspace — clean ~150 clippy errors (pre-existing debt) |
| WEFT-136 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: kernel — persist AppManager state to disk |
| WEFT-137 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: chain — implement chain-anchored anchoring beyond MockAnchor |
| WEFT-138 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — update docs/weftos/k-phases.md (K2.1/K3/K4/K5 mis-marked) |
| WEFT-139 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: docs — write docs/guides/kernel.md (deferred from K5) |
| WEFT-140 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — renumber duplicate ADRs (two ADR-020s, two ADR-028s) |
| WEFT-141 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: docs — accept ADR-023 (assessment-as-kernel-service) |
| WEFT-142 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: kernel — add NodeId composite for cross-node uniqueness + remote inbox bridge |
| WEFT-143 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: kernel — enforce max-message-size on KernelIpc::send (16 MiB) |
| WEFT-144 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — add MutationEvent Ed25519 signing for cross-node tree mutations |
| WEFT-145 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — incremental Merkle hash updates (replace full recompute_all) |
| WEFT-146 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — replace static GovernanceRule Vec with cluster-wide distribution |
| WEFT-147 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — cross-node capability-claim verification (signed advertisement) |
| WEFT-148 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — rate-limit add_peer() and governance-evaluation requests |
| WEFT-149 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — document DEMOCRITUS 'still stuck' log-line semantics |
| WEFT-150 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: kernel — verify weftos-leaf-types push path goes through governance / chain |
| WEFT-151 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — audit mesh_log/mesh_dedup/mesh_listener/mesh_bootstrap for callers |
| WEFT-152 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: tests — confirm cognitum-gate-tilezero Permit/Defer/Deny path is exercised |
| WEFT-153 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: chain — add EVENT_KIND_* constants for minor non-kernel chain gaps |
| WEFT-154 | Done | high | 0.8.x | ws05-channels | H | strong | — | — | ws05: Email channel — implement IMAP poll loop and SMTP outbound send |
| WEFT-155 | Done | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: Google Chat channel — implement Pub/Sub subscribe and chat.spaces.messages.create |
| WEFT-156 | Done | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: Microsoft Teams channel — implement Bot Framework auth and Graph message POST |
| WEFT-157 | Done | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: WhatsApp channel — implement Cloud API webhook receiver and outbound POST |
| WEFT-158 | Done | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: Signal channel — spawn signal-cli daemon and wire JSON-RPC reader |
| WEFT-159 | Done | high | 0.8.x | ws05-channels | H | strong | — | — | ws05: Matrix channel — implement /sync long-poll, room auto-join, and m.room.message send |
| WEFT-160 | Done | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: IRC channel — implement TCP/TLS dial, CAP/NICK/USER, JOIN, and PRIVMSG read/write |
| WEFT-161 | Done | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Discord — fix edit_message rate-limit handling to actually sleep |
| WEFT-162 | Done | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Permissions — emit allow_from_match metadata in Slack and Telegram channels |
| WEFT-163 | Done | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Web channel — gate is_allowed on D-7 auth middleware enablement |
| WEFT-164 | Done | high | 0.7.x | ws05-channels | E | strong | — | — | ws05: Voice channel — replace stub start/send with real capture+VAD+STT and TTS playback |
| WEFT-165 | Cancelled | high | 0.7.x | ws05-channels | H | strong | — | — | ws05: Gateway — wire Axum auth middleware (D-7) and gate non-public routes |
| WEFT-166 | Cancelled | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Gateway — add Content-Security-Policy via tower middleware |
| WEFT-167 | Cancelled | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Gateway — add per-endpoint rate limiting on auth/delegation/monitoring |
| WEFT-168 | Done | medium | 0.7.x | ws05-channels | H | strong | — | — | ws05: Gateway bridge — implement skill install/uninstall, memory delete, config persist |
| WEFT-169 | Done | medium | 0.8.x | ws05-channels | H | strong | — | — | ws05: Discord chunker — preserve fenced code, balance markdown, support Nitro/embeds/file  |
| WEFT-170 | Done | medium | 0.8.x | ws05-channels | H | strong | — | — | ws05: PluginHost C7 unification — migrate Telegram/Discord/Slack to ChannelAdapter |
| WEFT-171 | Done | medium | 0.8.x | ws05-channels | H | strong | — | — | ws05: Slash-command surface — decide consumer for ChannelHost::register_command |
| WEFT-172 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Telegram — document or remove redundant 1s poll-interval sleep |
| WEFT-173 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Discord — document intents bitmask default and cover privileged-intent rejection |
| WEFT-174 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Slack — add unknown_envelope counter for API drift detection |
| WEFT-175 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: iMessage scope — implement AppleScript bridge or formally drop from tracker |
| WEFT-176 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: WeftOS white-label — add brand() accessor and remove hard-coded clawft strings |
| WEFT-177 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Channel failover chain — decide semantics and either implement or close as out-of-sc |
| WEFT-178 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: AgentRouter — wire routing into MessageBus inbound dispatch |
| WEFT-179 | Done | high | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: FlowDelegator — implement delegation/flow.rs or formally retire the Flow target |
| WEFT-180 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: Recursive-delegation guard — thread depth via CLAWFT_DELEGATION_DEPTH and enforce MA |
| WEFT-181 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: McpServerManager — implement drain-and-swap on remove_server |
| WEFT-182 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: McpBridge — implement real Claude Code connection (spawn, handshake, tools/list, nam |
| WEFT-183 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: PlanningRouter — implement execute_react and execute_plan_and_execute |
| WEFT-184 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: AgentRuntime (L2) — per-agent SessionManager, ContextBuilder, ToolRegistry, AgentsCo |
| WEFT-185 | Done | medium | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: AgentBus + SwarmCoordinator — spawn worker loops and ship at least one demo |
| WEFT-186 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: McpServerManager — add transport factory and url/command/tempfile validators |
| WEFT-187 | Done | medium | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: Hot-reload watcher — wire notify on [tools.mcp_servers] with 500ms debounce |
| WEFT-188 | Done | medium | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: weft mcp CLI — add add/list/remove subcommands |
| WEFT-189 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: weft mcp-server — implement allowed_tools + CommandPolicy/UrlPolicy on tools/call (3 |
| WEFT-190 | Done | high | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: Tool-execution helper — extract execute_tool_with_guards and apply truncation (3H CR |
| WEFT-191 | Done | medium | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: McpSession — keepalive, reconnect, is_alive, graceful cancel (3H MAJ-03) |
| WEFT-192 | Done | low | 0.9.x | ws07-multi-agent | D | strong | — | — | ws07: weft mcp-server — JsonRpc id Value, init-state tracking, -32002 (3H MIN-01/03) |
| WEFT-193 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: IDE provider — replace IdeToolProvider::stub() with real implementation |
| WEFT-194 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: Hybrid context-router — wire MicroLoraRouter once agent-core-v1 phase E3+ is ready |
| WEFT-195 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: delegate_tool — drop hardcoded claude_available=true, query the delegator for livene |
| WEFT-196 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: weft delegate — add debug subcommand to surface routing decisions |
| WEFT-197 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: weft doctor — add multi-agent checks (claude on PATH, auto-delegation, ≥1 route) |
| WEFT-198 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: claude-flow MCP server — decide whether to add by default to [tools.mcp_servers] |
| WEFT-199 | Done | low | 0.8.x | ws07-multi-agent | B | strong | — | — | ws07: SwarmCoordinator topology — implement mesh/hierarchical/adaptive or document as prom |
| WEFT-200 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: notifications/tools/list_changed — handle inbound and advertise outbound |
| WEFT-201 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: Auto-delegation classifier — improve regex+keyword accuracy or document fragility (3 |
| WEFT-202 | Done | low | 0.7.x | ws07-multi-agent | D | strong | — | — | ws07: Backfill phase-* decisions/notes for FlowDelegator skip and bridge stub rationale |
| WEFT-203 | Done | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: delegation_config — reconcile claude_enabled defaults across serde-default vs Defaul |
| WEFT-204 | Done | medium | 0.9.x | ws07-multi-agent | D | strong | — | — | ws07: Per-agent MCP-server config override (Contract 3.2) — implement merge logic |
| WEFT-205 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: Voice STT — record ADR choosing in-process sherpa-rs vs substrate clawft-service-whi |
| WEFT-206 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: VP gate — run validation prototype or formally cancel and remove deferral markers |
| WEFT-207 | Done | high | 0.7.x | ws10-voice | E | strong | WEFT-555 | — | ws10: SC-1 mic privacy indicator — implement before any real cpal capture lands |
| WEFT-208 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: SC-4 voice permission flags — gate voice-triggered tool execution by Level 0/1/2 |
| WEFT-209 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: SC-7 model integrity — replace placeholder SHA-256 and add Ed25519 manifest verify |
| WEFT-210 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: SC-9 voice command audit logging — record permission-check trail per transcription |
| WEFT-211 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: SC-10 plugin voice capability — gate WASM plugins on voice capability + sub-perms |
| WEFT-212 | Done | high | 0.7.x | ws10-voice | E | strong | — | — | ws10: voice umbrella feature — include voice-stt and voice-tts or rename to voice-stubs |
| WEFT-213 | Done | medium | 0.7.x | ws10-voice | E | strong | — | — | ws10: AudioConfig / CaptureConfig / PlaybackConfig — collapse into one canonical type |
| WEFT-214 | Done | medium | 0.8.x | ws10-voice | E | strong | WEFT-671 | — | ws10: voice_listen / voice_speak tools — wire to real STT/TTS with cloud fallback |
| WEFT-215 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: weft voice setup — real model download with SHA-256 verify and progress UI |
| WEFT-216 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: WakeWordDetector — wire rustpotter or document an alternative |
| WEFT-217 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: EchoCanceller and NoiseSuppressor — replace deceptive passthroughs with real DSP |
| WEFT-218 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: WS voice:status — connect a real backend broadcaster |
| WEFT-219 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: /api/voice/* — replace MSW-only mocks with real handlers in clawft-services |
| WEFT-220 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Windows install-service — automate schtasks or document manual route as final |
| WEFT-221 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: Talk Mode interruption — abort TTS when VAD trips during playback |
| WEFT-222 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: VoicePersonality — wire per-agent lookup in TTS dispatch |
| WEFT-223 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-2 audio buffer zeroization and voice.audio_retention config |
| WEFT-224 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-3 cloud-fallback transparency log line |
| WEFT-225 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-6 anti-replay nonce and transcription-echo confirmation |
| WEFT-226 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-8 voice rate limiting (commands/min, wake/min, post-fail cooldown) |
| WEFT-227 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Speaker diarization via sherpa-rs |
| WEFT-228 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Tauri-side native mic capture — replace browser-only getUserMedia path |
| WEFT-229 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Latency + WER + CPU benchmarks for voice pipeline |
| WEFT-230 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Adaptive silence timeout learning |
| WEFT-231 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: UI partial-transcription streaming and TTS word highlighting |
| WEFT-232 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Discord voice bridge — clawft-channels voice → STT → agent → TTS → VC audio |
| WEFT-233 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: audio_transcribe / audio_synthesize tools — real WAV/MP3/OGG/WebM codec support |
| WEFT-234 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Cleanup orphan voice surfaces (events, statuses, voice-chat.ts, model_path types) |
| WEFT-235 | Done | low | 0.9.x | ws10-voice | E | strong | — | — | ws10: clawft-service-classify — decide adoption (connect to W-VOICE, Explorer-only, or del |
| WEFT-236 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: clawft-service-whisper — drop legacy dual-publish path post Phase-4 migration |
| WEFT-237 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: clawft-service-whisper publish_wav example — keep or delete |
| WEFT-238 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: VoiceConfig.tts.provider="browser" — implement Web Speech dispatch or change default |
| WEFT-239 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: CloudFallbackConfig — config-string to provider router |
| WEFT-240 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: WakeConfig.sensitivity vs WakeWordConfig.threshold — unify the knob |
| WEFT-241 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Define join key between TranscriptLogger session_key and substrate source-node-id |
| WEFT-242 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — supersede ADRs 005/007/038/013 (legacy Tauri+React) |
| WEFT-243 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — bound activity HashMap (LRU/TTL) to stop session leak |
| WEFT-244 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: composer — wire ui://field in surface IR dispatch |
| WEFT-245 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: composer — wire remaining 10 canon IRIs (toggle/select/slider/sheet/modal/dock/tabs/ |
| WEFT-246 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: wasm — fix wasm-opt disabled (4.2 MB unoptimized cold-load hit) |
| WEFT-247 | Done | medium | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: wasm — factor Instant::checked_sub time-origin guard helper |
| WEFT-248 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — surface live smoke verify against running llama-server |
| WEFT-249 | Done | high | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: composer — RefCell re-entrancy class still possible (deadlock-bug class) |
| WEFT-250 | Done | medium | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode allowlist drift — document or automate substrate-method add |
| WEFT-251 | Done | medium | 0.7.x | ws08-weftos-gui | F | strong | — | WEFT-8 | ws08: wasm — image PNG feature pin documentation |
| WEFT-252 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — markdown rendering in chat bubbles |
| WEFT-253 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — inline streaming via agent.chat_stream |
| WEFT-254 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — multi-conversation sidebar UI |
| WEFT-255 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — system-prompt UI affordance |
| WEFT-256 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — model / provider switcher in chip strip |
| WEFT-257 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — heartbeat label replaces spinner occlusion |
| WEFT-258 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — real interactive defer (resume on { deferred: true }) |
| WEFT-259 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — identity-drift / binding-thread mismatch warning |
| WEFT-260 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — mouse selection + clipboard |
| WEFT-261 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — bold/italic glyph variants |
| WEFT-262 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — scrollback view + wheel handler |
| WEFT-263 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — multi-tab terminal (HashMap<SessionId, Terminal>) |
| WEFT-264 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — real WASM terminal renderer |
| WEFT-265 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: canon — implement Field::Date (DatePickerButton + chrono::NaiveDate) |
| WEFT-266 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: canon — implement Field::Code (TextEdit::multiline + syntax highlighting) |
| WEFT-267 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: canon — Select TableBuilder large-set form (ADR-001 row 5) |
| WEFT-268 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — HealthViewer for substrate/<node>/health (READS-ONLY tier) |
| WEFT-269 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — SensorViewer with raw-vs-summary child-pane switcher |
| WEFT-270 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — tree filter UI (chip row for type/status filters) |
| WEFT-271 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — sparkline embed for HealthReport scalars |
| WEFT-272 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — Sensor↔Node breadcrumb navigation intent |
| WEFT-273 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — copy-path / copy-pubkey / export-snapshot via clipboard |
| WEFT-274 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — Workshop parameterization schema sign-off + impl |
| WEFT-275 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — Lineage Object Type + viewer (metadata convention sign-off) |
| WEFT-276 | Done | medium | 0.8.x | ws08-weftos-gui | B | strong | — | — | ws08: explorer — ObjectType::applicable_actions populated for Mesh/Sensor/Node |
| WEFT-277 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: composer — honest_affordances real GEPA / governance intersection |
| WEFT-278 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — implement Grid layout (degrades to Rows today) |
| WEFT-279 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — implement Tabs layout (degrades to Rows today) |
| WEFT-280 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — wire viewer_hint overrides (today: "auto" only) |
| WEFT-281 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: graph viewer — editable Phase 3+ patch UI (egui_node_graph migration) |
| WEFT-282 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — capture sidecar (mic/camera) for vscode#303293 |
| WEFT-283 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — typed active-radar return schema (variant-id echo) |
| WEFT-284 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — ThreadDock primitive for per-agent parallel output |
| WEFT-285 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — WSP-0.1 verb support (raw RPC only today) |
| WEFT-286 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: hygiene — document or retire legacy blocks/ vs canon/ duality |
| WEFT-287 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: hygiene — decide vendored vs upstream egui_demo_lib path |
| WEFT-288 | Done | low | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: handoff — cleanup 12 agent-core/* locked worktrees |
| WEFT-289 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — confirm npm run package + .vsix flow current |
| WEFT-290 | Done | low | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: tooling — document weft-gui-egui native binary path in scripts/build.sh |
| WEFT-291 | Done | low | 0.7.x | ws08-weftos-gui | F | strong | — | — | ws08: changelog — fix [0.5.x] legacy Lego Block Engine reference |
| WEFT-292 | Done | high | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: build-script — fix cmd_ui to point at clawft-ui not ui |
| WEFT-293 | Done | high | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: weft-ui — update --ui-dir default to clawft-ui/dist |
| WEFT-294 | Done | medium | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: hygiene — remove stale ui/ legacy artefact |
| WEFT-295 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: package-json — rename name field from ui to clawft-ui |
| WEFT-296 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: index-html — set real product title for dashboard |
| WEFT-297 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: readme — replace Vite template with project README |
| WEFT-298 | Done | high | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — add CSP middleware to gateway handlers |
| WEFT-299 | Cancelled | high | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — add rate-limiting middleware with per-endpoint defaults |
| WEFT-300 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — add WebSocket heartbeat and dead-connection cleanup |
| WEFT-301 | Todo | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api-bridge — wire skill install/uninstall to real loader |
| WEFT-302 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api-bridge — implement memory delete with append-only rewrite |
| WEFT-303 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api-bridge — implement save_config persistence |
| WEFT-304 | Todo | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — replace mock delegation handlers with FlowDelegator wiring |
| WEFT-305 | Todo | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — replace mock monitoring handlers with metrics collector |
| WEFT-306 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: tools — wire render_ui to message bus and ws broadcaster |
| WEFT-307 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: wasm-adapter — implement getToolSchema introspection |
| WEFT-308 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: ui — implement real Cmd+K command palette |
| WEFT-309 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — add use-auth hook with single-use URL token |
| WEFT-310 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: browser-config — validate cors_proxy URL is HTTPS in production |
| WEFT-311 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | WEFT-560 | — | ws09: pwa — add manifest, service worker, push notifications |
| WEFT-312 | Todo | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: responsive — mobile sidebar drawer and chat input |
| WEFT-313 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: tauri — scaffold clawft-ui/src-tauri desktop shell |
| WEFT-314 | Done | medium | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: tests — add Playwright E2E suite under clawft-ui/tests |
| WEFT-315 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | WEFT-561 | — | ws09: ui — axe-core a11y audit and bundle-size budget |
| WEFT-316 | Todo | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — Tailscale provider and per-user session isolation |
| WEFT-317 | Done | low | 0.9.x | ws09-clawft-dashboard | A | strong | — | — | ws09: deploy — multi-stage Dockerfile for dashboard |
| WEFT-318 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: env — add .env and .env.mock with documented VITE vars |
| WEFT-319 | Done | low | 0.9.x | ws09-clawft-dashboard | F | strong | — | — | ws09: docs — author ADR for BackendAdapter contract |
| WEFT-320 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: planning — refresh sparc trackers to reflect shipped status |
| WEFT-321 | Done | low | 0.7.x | ws09-clawft-dashboard | F | strong | — | — | ws09: planning — note rename in step-7 phase-gate and re-run gate |
| WEFT-322 | Done | high | 0.7.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — implement per-conversation cost circuit-breaker |
| WEFT-323 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — thread per-iteration CancellationToken into run_tool_loop |
| WEFT-324 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — expose public chain.append RPC for soul promote + agent journal |
| WEFT-325 | Cancelled | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — promote workspace/global PermissionResolver split |
| WEFT-326 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — stabilize append_turns_are_monotonic flake via injectable clock |
| WEFT-327 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — promote overlay_probe + resolver_live_probe diagnostics into CI |
| WEFT-328 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — plumb tool_calls / token / model fields through OutboundMessage |
| WEFT-329 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — notify-driven hot-reload watcher for identity files |
| WEFT-330 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — agent-side SOUL.journal.md write path during chat turns |
| WEFT-331 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — interactive Defer UX prompt-and-resume in panel |
| WEFT-332 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — per-user agent_ids for multi-tenant chat |
| WEFT-333 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — register agent.chat SystemService for weft status |
| WEFT-334 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — typed error variants for agent.chat |
| WEFT-335 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — observability path logging router decisions to substrate |
| WEFT-336 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — weft routing trace + replay commands |
| WEFT-337 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v2.5 — sona-backed rerank step on HybridRouter |
| WEFT-338 | Done | low | 0.8.x | ws11-agent-core-v1 | C | strong | — | — | ws11: agent-core-v3 — MicroLoraRouter behind ruvllm-wasm 11-pattern HNSW cap lift |
| WEFT-339 | Done | low | 0.9.x | ws11-agent-core-v1 | D | strong | — | — | ws11: de-duplicate clawft_weave::protocol vs clawft_service_agent::protocol types |
| WEFT-340 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: confirm agent.chat "agent service not wired" error path has integration coverage |
| WEFT-341 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — per-tool Permit token + proof-of-permission API |
| WEFT-342 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — hard-refuse on binding-thread mismatch (governance rule) |
| WEFT-343 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — Arc<RwLock<LlmClient>> runtime swap on env rotation |
| WEFT-344 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — agent.workspace_root config key |
| WEFT-345 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — after-3-denials EscalateToHuman governance path |
| WEFT-346 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — multi-conversation sidebar UI for panel |
| WEFT-347 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 4 MemoryConsolidator periodic distillation |
| WEFT-348 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 4 skills auto-promotion from .claude/skills to .clawft/skills |
| WEFT-349 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — cross-agent delegation via existing delegate_tool |
| WEFT-350 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 2 voice + streaming chat path |
| WEFT-351 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG — replace vector_diskann.rs HashMap linear-scan stub |
| WEFT-352 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-011 — activate LogQuantized for DiskANN once shaal PR #352 merges |
| WEFT-353 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-012 — activate unified SIMD distance kernel once shaal PR #352 merges |
| WEFT-354 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-013 — spatio-temporal GNN for sonobuoy (K-STEMIT) |
| WEFT-355 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-015 — EA-Agent entity alignment for multi-repo dedup |
| WEFT-356 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-017 — knowledge distillation for edge EML (SevenNet-Nano) |
| WEFT-357 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-018 — Newman modularity scoring as alternative to cohesion |
| WEFT-358 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-2 — OWL/RDF ingestion (Turtle, JSON-LD) |
| WEFT-359 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-3 — Barnes-Hut force layout + positioned-SVG export |
| WEFT-360 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-4 — VOWL visual encoding rules in SVG export |
| WEFT-361 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-004 — benchmark RFF vs Lanczos vs EML lambda₂ on 1K/10K/100K graphs |
| WEFT-362 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: layout — implement Sugiyama layered layout (currently falls back to tree) |
| WEFT-363 | Cancelled | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — wire VectorBackend into DemocritusLoop |
| WEFT-364 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — ecc.vector-config RPC to show active backend |
| WEFT-365 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — diskann feature flag for real impl |
| WEFT-366 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | WEFT-656 | — | ws12: vector — hybrid vs pure HNSW benchmark for ECC workloads |
| WEFT-367 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: weaver graphify rebuild — full extraction-pipeline integration |
| WEFT-368 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: ingest — replace StubHttpClient with real reqwest-based HTTP client |
| WEFT-369 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — MCP server (Phase 6) |
| WEFT-370 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — extraction + graph_ops benchmarks (Phase 6) |
| WEFT-371 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — write ADR-049 (graphify port) |
| WEFT-372 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — write ADR-050..053 candidates from phase2 paper survey |
| WEFT-373 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — incremental graph updates (LightRAG set-union dedup) |
| WEFT-374 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — multi-key HNSW indexing (LightRAG P2) |
| WEFT-375 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — edge embeddings for relationship queries (LightRAG P5) |
| WEFT-376 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — graph-aware HNSW re-ranking (LightRAG P4) |
| WEFT-377 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — discover_hyperedges() pipeline step |
| WEFT-378 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — vault domain hyperedges + SUGGEST→ratify→CRDT pipeline |
| WEFT-379 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — index-based optimization for forensic gap_analysis (O(n·m) cliff) |
| WEFT-380 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — adaptive HNSW rebuild_threshold (EML coherence two-tier) |
| WEFT-381 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — vision_extract end-to-end test fixture |
| WEFT-382 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — schema-based edge validation in validation.rs |
| WEFT-383 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — clean up dead clawft-llm optional dep flag |
| WEFT-384 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — adaptive ef (HNSW-EML opportunity) |
| WEFT-385 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — search-path prediction (HNSW-EML #4, biggest single win) |
| WEFT-386 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — cosine-similarity decomposition for distance speedup |
| WEFT-387 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — verify+restore standalone export/cypher.rs and export/svg.rs |
| WEFT-388 | Done | high | 0.7.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — wasm-bindgen-test regression suite for init() + send_message() pipeline |
| WEFT-389 | Done | high | 0.7.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — binary size budget audit (1.32 MB → wasm-opt -Oz CI gate) |
| WEFT-390 | Done | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — streaming chat via ReadableStream / wasm-streams |
| WEFT-391 | Done | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — wire set_env to BrowserEnvironment via OnceLock<BrowserRuntime> |
| WEFT-392 | Done | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — implement OPFS-backed BrowserFileSystem behind browser-opfs feature |
| WEFT-393 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write ADR-027 Browser WASM Support |
| WEFT-394 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write docs/development/feature-flags.md |
| WEFT-395 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write docs/browser/cors-provider-setup.md + config-schema.md |
| WEFT-396 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — update root README.md and CLAUDE.md with browser build instructions |
| WEFT-397 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — compile_error! when both native and browser features are enabled |
| WEFT-398 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — split clawft-wasm host code into dedicated crate |
| WEFT-399 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — persistent conversation history via OPFS (CLAUDE.md-per-group) |
| WEFT-400 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — Web Worker harness variant |
| WEFT-401 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — gitignore or stub-replace pre-built www/pkg artifact |
| WEFT-402 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — fix unreachable_code warning in workspace/agent.rs:257 |
| WEFT-403 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — audit ADR-044 vs reality (wasip1 vs wasip2 + script alignment) |
| WEFT-404 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — data-driven provider-routing fallback order |
| WEFT-405 | Done | low | 0.8.x | ws16-browser-wasm | A | strong | — | — | ws16: browser — sign + version browser bundle artefact (parity with WASI release) |
| WEFT-406 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — threat-model note on api_key in JS-readable WASM memory |
| WEFT-407 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — performance profiling baseline (load, init, first-msg, memory) |
| WEFT-408 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — final regression suite + ≤10% test-duration regression check (P6.7) |
| WEFT-409 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | WEFT-562,WEFT-563 | WEFT-562,WEFT-563 | ws16: browser — retire or document scripts/check-features.sh contract |
| WEFT-410 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — decide UnknownMode validation variant fate |
| WEFT-411 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — add registry corruption quarantine path |
| WEFT-412 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — emit lifecycle teardown tombstone on uninstall-while-enabled |
| WEFT-413 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — wire ADR-015 rule 6 once clawft-adapter exists |
| WEFT-414 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — cover wasm to_toml_string failure path with negative test |
| WEFT-415 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — emit substrate/meta/adapter/<id>/health from each adapter |
| WEFT-416 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — per-id Replace/Remove deltas on processes/services topics |
| WEFT-417 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — surface Subscription closed via adapter-health topic on teardown |
| WEFT-418 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — migrate mic adapter to substrate/<node-id>/sensor/mic/{summary,pc |
| WEFT-419 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — ship a second Characterization exemplar (Enumerated or Spectral) |
| WEFT-420 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — implement cross-platform network/bluetooth or document Linux-only |
| WEFT-421 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — wire 13 stub-leaf canon primitives in the composer |
| WEFT-422 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — add .first/.last field-access shorthand support |
| WEFT-423 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — implement sort(list, key) ordering combinator |
| WEFT-424 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — accept scientific (1e5) and hex (0xff) number literals |
| WEFT-425 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — parse [compositions.*] and expand in composer |
| WEFT-426 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — drop unused egui dep from Cargo.toml |
| WEFT-427 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — extract canon types and move composer back to clawft-surface |
| WEFT-428 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — replace 14-line src/substrate.rs shim with direct re-export |
| WEFT-429 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — wire real ADR-012 governance::Gate through Substrate::subscribe_adapte |
| WEFT-430 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — implement honest affordance ∩ permit intersection in compose::honest_a |
| WEFT-431 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — drive variant_id stamping in CanonResponse from surface binding |
| WEFT-432 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — implement per-sensor healthcheck contract emitter |
| WEFT-433 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — enforce per-node-prefix write gate on substrate.publish |
| WEFT-434 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — add streaming log endpoint so kernel adapter drops poll fallback |
| WEFT-435 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — test substrate.notify consumer wakeup semantics in integration suite |
| WEFT-436 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — ship a Presence exemplar adapter |
| WEFT-437 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — implement HEALTHCHECK-CONTRACT.md as clawft-substrate::healthcheck module |
| WEFT-438 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — resolve legacy-flat-path vs node-scoped-path naming and ship migration |
| WEFT-439 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: weftos-admin — add wired Modal ("confirm restart") to admin surface |
| WEFT-440 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: weftos-admin — migrate auto-install-from-fixture flow off web-time workaround |
| WEFT-441 | Done | high | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docker — purge stale ghcr.io/clawft/clawft paths in compose, vps-deploy, docs/deploy |
| WEFT-442 | Done | high | 0.7.x | ws14-deployment | A | strong | — | — | ws14: install — decide canonical install path (~/.clawft vs ~/.weftos) and sweep callers |
| WEFT-443 | Done | high | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docs — rewrite docs/deployment/docker.md (drop FROM scratch + cargo-chef appendix) |
| WEFT-444 | Done | high | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docs — rewrite docs/deployment/release.md to match cargo-dist + sub-releases |
| WEFT-445 | Done | high | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docs-site — auto-generate releases.mdx from CHANGELOG (currently 6 versions behind) |
| WEFT-446 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: changelog — backfill compare links 0.6.7..0.6.19 and add Unreleased heading |
| WEFT-447 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: ci — convert PR-gate browser-WASM check to hard gate |
| WEFT-448 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: ci — add docs-build job to pr-gates.yml so bad MDX cannot merge |
| WEFT-449 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: scripts — decide fate of scripts/release/package*.sh (delete or relabel) |
| WEFT-450 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docker — fix scripts/build/docker-build.sh image name + arch drift |
| WEFT-451 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: install — verify cargo-dist attestations in scripts/install.sh |
| WEFT-452 | Done | low | 0.7.x | ws14-deployment | A | strong | — | — | ws14: docs — re-title ADR-044 from wasip1 to wasip2 |
| WEFT-453 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — soft-check docs-site MDX builds locally via scripts/build.sh ui |
| WEFT-454 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cdn — snapshot every cdn-assets upload by commit SHA for rollback |
| WEFT-455 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add browser-WASM size budget to wasm-browser.yml |
| WEFT-456 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deploy — add health-probe rollback path to scripts/deploy/vps-deploy.sh |
| WEFT-457 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add macOS + Windows test job to pr-gates.yml |
| WEFT-458 | Done | low | 0.9.x | ws14-deployment | A | strong | — | — | ws14: ci — add cargo-audit / cargo-deny gate to pr-gates.yml |
| WEFT-459 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add SBOM (CycloneDX) generation and attach to releases |
| WEFT-460 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: tooling — add scripts/build.sh release-dry-run subcommand |
| WEFT-461 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: build-kb — move tools/build-kb into the workspace (or document why not) |
| WEFT-462 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cargo-dist — schedule v0.31 → v1.0+ bump and regenerate release.yml |
| WEFT-463 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — bump or delete scripts/09-gate.sh stale floor + paths |
| WEFT-464 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — wire scripts/k6-gate.sh into CI or mark developer-rehearsal |
| WEFT-465 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — audit and reorganize dead scripts (wake units, py helpers, weave-init.sh) |
| WEFT-466 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: planning — populate or delete empty deployment-community phase-K stubs |
| WEFT-467 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docs — audit pass on docs/deployment/wasm.md for stale URLs and wasip1 references |
| WEFT-468 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docs — fix Fumadocs link drift for docs/deployment/*.md (move into docs/src or delet |
| WEFT-469 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docker — verify or remove crates/clawft-kernel/Dockerfile.alpine |
| WEFT-470 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docs — fix stale 0.3.1 example block in ADR-037 |
| WEFT-471 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: governance — adopt release-plz/git-cliff or amend ADR-002 to record current flow |
| WEFT-472 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: planning — reconcile Element 10 tracker (ClawHub features tangentially deployment) |
| WEFT-473 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deps — add quarterly dependency-sweep cadence (post-wasmtime-v33) |
| WEFT-474 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deploy — confirm and document assess.weavelogic.ai deploy origin |
| WEFT-475 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: homebrew — decide bottle vs source-build formula for weft-cli |
| WEFT-476 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add wasm32-wasip2 build to release.yml or cargo-dist when supported |
| WEFT-477 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: closure-sdk — re-check release-engineering implications when bridge work is proposed |
| WEFT-478 | Done | high | 0.7.x | ws15-mcp | J | strong | — | — | ws15: plane MCP — list_* endpoints return HTTP 404 (server URL bug) |
| WEFT-479 | Done | high | 0.7.x | ws15-mcp | J | strong | — | — | ws15: daemon RPC — no per-method capability gating on UDS callers |
| WEFT-480 | Done | high | 0.7.x | ws15-mcp | J | strong | — | — | ws15: mcp-server — PermissionFilter::new(None) exposes every tool |
| WEFT-481 | Done | high | 0.7.x | ws15-mcp | J | strong | — | — | ws15: ipc_tcp relay — security audit (auth + bind-address default) |
| WEFT-482 | Cancelled | high | 0.7.x | ws15-mcp | J | strong | — | — | ws15: VSCode extension ALLOWED_METHODS drift — codegen or runtime fetch |
| WEFT-483 | Done | high | 0.7.x | ws15-mcp | J | strong | WEFT-559 | — | ws15: Windows transport stub — named-pipe impl or drop from 0.7.0 matrix |
| WEFT-484 | Done | medium | 0.7.x | ws15-mcp | G | strong | — | — | ws15: WASM webview rebuild — promote into scripts/build.sh |
| WEFT-485 | Done | medium | 0.7.x | ws15-mcp | G | strong | — | — | ws15: CI smoke — ts compile + wasm rebuild on PRs touching gui-egui |
| WEFT-486 | Done | medium | 0.7.x | ws15-mcp | J | strong | WEFT-558 | WEFT-558 | ws15: VSCode extension end-to-end smoke (headless host + chip-icons assert) |
| WEFT-487 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: mcp-server CI smoke — round-trip tools/list + tools/call on builtin |
| WEFT-488 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: claude-flow integration — decide first-party vs user-installed; land or strip |
| WEFT-489 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: MCP protocol-version mismatch — log/reject foreign protocolVersion |
| WEFT-490 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: SkillToolProvider tools/call contract — document SKILL.md prompt-body return |
| WEFT-491 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: IdeToolProvider — verify backend dispatch + integration tests |
| WEFT-492 | Done | medium | 0.7.x | ws15-mcp | J | strong | — | — | ws15: docs — `weft mcp add` install path for @claude-flow/cli + sample weave.toml |
| WEFT-493 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: McpServerManager hot-reload — wire file-watcher or remove affordances |
| WEFT-494 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: `mcp.add`/`mcp.list`/`mcp.remove` daemon verbs — clarify CLI vs RPC ownership |
| WEFT-495 | Done | medium | 0.8.x | ws15-mcp | G | strong | — | — | ws15: WASM panel auth — token/capability model for webview proxy |
| WEFT-496 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: webview vs daemon allowlist — substrate.publish gating semantics |
| WEFT-497 | Done | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: agent-core-chat feature flag — schedule removal post-D3 soak |
| WEFT-498 | Done | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: AgentChatParams/Result wire types — relocate to clawft-types |
| WEFT-499 | Done | low | 0.8.x | ws15-mcp | A | strong | — | — | ws15: weft-gui-egui native bin — promote to scripts/build.sh native --gui + release artifa |
| WEFT-500 | Done | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: MCP HTTP transport — verify against real HTTP server (not just mock) |
| WEFT-501 | Cancelled | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: TODO(agent-core-v1.1) — replace soul_cmd direct call with chain.append RPC |
| WEFT-502 | Done | urgent | 0.9.x | ws17-research | I | strong | — | — | ws17: Democritus — verify idle-graph gate keeps net_change suppression on real daemon (pos |
| WEFT-503 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — wire boot_ecc() runtime function into Kernel<P> boot sequence |
| WEFT-504 | Done | medium | 0.8.x | ws17-research | G | strong | — | — | ws17: ECC — verify ecc feature exclusion on wasm32-unknown-unknown |
| WEFT-505 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — add governance gates to auth_service rotate_credential, request_token, revoke_ |
| WEFT-506 | Done | medium | 0.8.x | ws17-research | C | strong | — | — | ws17: governance — make EffectVector explicit on auth/config/a2a/cron gates |
| WEFT-507 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: weave — implement weaver ecc CLI subcommands |
| WEFT-508 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: ECC — define new RVF segment types for ECC structures and persistence |
| WEFT-509 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: ECC — resolve 5 pre-existing clippy warnings in agent_loop, chain, gate |
| WEFT-510 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: EML — incremental component-count maintenance for O(1) coherence feature extraction |
| WEFT-511 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — Iteration 1 end-to-end coordinate-descent loop for Q/K/V models |
| WEFT-512 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — drive top 5 eml-synergy-scan rows from scan to implementation |
| WEFT-513 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — RoMem phase-rotation temporal KG on CausalGraph |
| WEFT-514 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — GraphRAG community summaries in pipeline analyze |
| WEFT-515 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — CausalRAG causal_trace() over typed edges |
| WEFT-516 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: KG — SASE clustering replacing label-propagation in cluster.rs |
| WEFT-517 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: KG — LightRAG dual-level keyword retrieval |
| WEFT-518 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — process remaining Phase 2 papers into priority list |
| WEFT-519 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — codify ADR-058 decoupling-invariant runtime checks |
| WEFT-520 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create weftos-worldmodel-core crate (no_std traits) |
| WEFT-521 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create weftos-worldmodel-impls crate (candle ViT-tiny + AdaLN) |
| WEFT-522 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create weftos-worldmodel facade crate |
| WEFT-523 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create weftos-sensor-pipeline + -wire crates |
| WEFT-524 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create clawft-worldmodel-service binary (3 deployment topologies) |
| WEFT-525 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — create clawft-delegation crate |
| WEFT-526 | Done | high | 0.8.x | ws17-research | B | strong | — | — | ws17: LeWM — add mesh.sensor.v1.{encoded,consensus,control} topics on mesh wire |
| WEFT-527 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — implement LatticeApi (7 methods) via ServiceApi |
| WEFT-528 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — wire SIGReg sigreg_health Welford monitor + auto-rollback at 0.85/30s |
| WEFT-529 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — implement pred_φ predictor + LatentPlanner (CEM default) |
| WEFT-530 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — implement four-condition AND rollback gate |
| WEFT-531 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — implement two training surfaces (offline edge + online streaming-merge) |
| WEFT-532 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — per-sensor-class trainable RVF-hosted small models with hot-swap |
| WEFT-533 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — ExoChain attestation of (a_t, z_t, z_{t+1}, surprise) tuples |
| WEFT-534 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: LeWM — land /lewm-worldmodel-rs marketing page after visual confirmation |
| WEFT-535 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: sonobuoy — scaffold clawft-sonobuoy-ranging crate (G1 follow-up) |
| WEFT-536 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: sonobuoy — drive G2-G5 to closure or accept as deferred |
| WEFT-537 | Done | low | 1.0.x | ws17-research | I | strong | — | — | ws17: quantum — implement Pasqal backend skeleton |
| WEFT-538 | Todo | low | 1.0.x | ws17-research | I | strong | — | — | ws17: quantum — scaffold cuDensityMat SimulatorBackend behind quantum-nvidia feature flag |
| WEFT-539 | Todo | low | 1.0.x | ws17-research | I | strong | — | — | ws17: gaming-robotics — kick off first symposium experiment |
| WEFT-540 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: docs — cross-link orphan symposium output (compositional-ui, RLM 2512.24601) or clos |
| WEFT-541 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: docs — decide on single research → feature pipeline index vs ADR-only |
| WEFT-542 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — decide boot_ecc() fold-vs-fork into Kernel<P> |
| WEFT-543 | Done | high | 0.8.x | ws17-research | I | strong | — | — | ws17: LeWM — decide 192-dim SIGReg latent dimensionality (ADR-050) |
| WEFT-544 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: governance — decide rotate-but-not-revoke policy expression for auth_service |
| WEFT-545 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: sonobuoy — decide whether 5th branch (active-imaging / SAS) lands as feature or stay |
| WEFT-546 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: Democritus — add rate limiting on exposure surface |
| WEFT-547 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: governance — close out 8-agent / 48-task exochain-fix-plan medium-severity rows |
| WEFT-548 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: EML — numerical-stability scaffolding for nested exp/ln at scale |
| WEFT-549 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: orphans — triage OpenFang gap targets (channel breadth, Hands, Tauri, security stack |
| WEFT-550 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: ci — replace smoke-test sleep+docker-ps with HTTP health probe |
| WEFT-551 | Done | high | 0.8.x | ws02-kernel | A | strong | — | — | ws02: deps — bump wasmtime 33 → 43 to clear 14 RUSTSEC advisories |
| WEFT-552 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: deps — bump rustls-webpki via rustls/reqwest/quinn alignment |
| WEFT-553 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: deps — replace unmaintained crates and unsound rand for cargo-audit cleanup |
| WEFT-554 | Done | high | 0.8.x | ws02-kernel | B | ok | WEFT-130 | WEFT-130 | ws02: exo-resource-tree — full K1 ACL engine (Did principals, delegation, exo_consent) |
| WEFT-555 | Done | high | 0.7.x | ws10-voice | E | strong | — | WEFT-207 | ws10: voice — wire substrate STT output into agent conversation + command input |
| WEFT-556 | Cancelled | high | 0.7.x | ws10-voice | E | weak | — | — | ws10: SC-10 plugin voice capability — gate WASM plugins on voice capability + sub-perms |
| WEFT-557 | Cancelled | high | 0.7.x | ws10-voice | E | ok | — | — | ws10: SC-4 voice permission flags — gate voice-triggered tool execution by Level 0/1/2 |
| WEFT-558 | Done | medium | 0.8.x | ws15-mcp | J | ok | WEFT-486 | WEFT-486 | ws15: VSCode panel E2E — chip-icon DOM assertion (followup to WEFT-486) |
| WEFT-559 | Done | high | 0.8.x | ws15-mcp | J | weak | — | WEFT-483 | ws15: Windows named-pipe transport — implement DaemonClient + daemon listener for x86_64-p |
| WEFT-560 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | WEFT-311 | ws09: pwa — push notifications via VAPID + WS event bridge |
| WEFT-561 | Todo | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | WEFT-315,WEFT-575 | ws09: ui — axe-core + Playwright a11y suite across all routes |
| WEFT-562 | Cancelled | low | — | ws16-browser-wasm | G | strong | WEFT-409 | WEFT-409 | ws16: sparc(BW5) — retire scripts/check-features.sh references missed by WEFT-409 sweep |
| WEFT-563 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | WEFT-409 | WEFT-409 | ws16: sparc(BW5) — retire scripts/check-features.sh references missed by WEFT-409 sweep |
| WEFT-564 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: scripts — actually retire or annotate scripts/check-features.sh (still on disk) |
| WEFT-565 | Done | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — TopicBroadcaster topics map leaks empty topic Senders |
| WEFT-566 | Done | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: docs — document save_config hot-reload semantics |
| WEFT-567 | Done | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: ui — /tools route does not call BackendAdapter.getToolSchema for WASM mode |
| WEFT-568 | Done | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: ui — Cmd+K palette index missing agents/sessions/tools/skills/channels + focus trap |
| WEFT-569 | Done | high | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — switch ?token= to #token= URL fragment to prevent log leak |
| WEFT-570 | Done | high | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — logout() must invoke server-side token revoke |
| WEFT-571 | Done | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: browser-config — validate customBaseUrl is HTTPS in production (mirror WEFT-310) |
| WEFT-572 | Done | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: pwa — replace placeholder vite.svg icon with real 192/512 PNGs and maskable |
| WEFT-573 | Done | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: pwa — render an offline banner when SW serves the cached shell |
| WEFT-574 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: tauri — desktop shell functional features (tray, hotkey, side-car, Spotlight, notifi |
| WEFT-575 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | WEFT-561 | — | ws09: ui — axe-core runtime a11y scan still missing (WEFT-315 AC unmet, follow-up to WEFT- |
| WEFT-576 | Done | high | 1.0.x | ws09-clawft-dashboard | A | strong | — | — | ws09: deploy — Dockerfile must run as non-root user (security hardening) |
| WEFT-577 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel wasm bundle — trim back toward 4500/1500 KB ceiling |
| WEFT-578 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 sidebar — canonical block per DESIGN.md §5 |
| WEFT-579 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Files app — list-detail |
| WEFT-580 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Processes app — table |
| WEFT-581 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Services app — tabs + table |
| WEFT-582 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Network app — chip TOMLs wrapped |
| WEFT-583 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Settings app — schema-driven form |
| WEFT-584 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Scheduler app — table+plot stub |
| WEFT-585 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Monitor app — tile-grid dashboard |
| WEFT-586 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Logs app — System + Witness chain stream |
| WEFT-587 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Terminal app — graduate from explorer/terminal.rs |
| WEFT-588 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Chat app — graduate from explorer/chat.rs |
| WEFT-589 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Admin app — composer surface + missing states |
| WEFT-590 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Explorer app — graduate from explorer/mod.rs |
| WEFT-591 | Done | high | 0.8.x | ws08-weftos-gui | F | weak | — | — | ws08: 0.8.0 Apps launcher — tile-grid + Developer category |
| WEFT-592 | Done | low | 0.8.x | ws02-kernel | B | ok | — | — | BVH spatial-temporal index — review plan and decompose into phase work items |
| WEFT-593 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cargo-dist stopped publishing platform binaries (empty plan matrix) |
| WEFT-594 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: release Docker image strategy — download-coupling vs self-contained multi-arch |
| WEFT-595 | Done | high | 0.8.x | ws18-firmware | F | strong | — | — | ws08: leaf-display residual visual gap — single-buffer disambiguation (BUG-1) |
| WEFT-596 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: ADR-057 substrate per-path read ACLs — implement (0.8.x mesh blocker) |
| WEFT-597 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: daemon tracing→ChainManager bridge — 12 ExoChain events bypass the chain |
| WEFT-598 | Done | medium | 0.8.x | ws09-clawft-dashboard | F | strong | — | — | ws09: Dependabot — triage 142 npm-side vulnerabilities (5 critical/41 high) |
| WEFT-599 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: relax transitive wasm-bindgen =0.2.108 exact pin |
| WEFT-600 | Done | high | 0.8.x | ws14-deployment | A | weak | — | WEFT-680 | ws14: workspace reqwest rustls-tls — fix static musl release build |
| WEFT-601 | Done | medium | 0.8.x | ws01-core | B | weak | — | — | ws01: adopt cargo-nextest + fix 6 test/latent-bug flakes (gate 12/12 green) |
| WEFT-602 | Done | none | 0.8.x | ws14-deployment | A | weak | — | — | ws14: release v0.6.20 — 0.6 rollup (63 assets: binaries + WASM + KB) |
| WEFT-603 | Done | high | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | weft agent -m hangs forever after a failed turn (provider error / max-iterations) |
| WEFT-604 | Done | medium | 0.8.x | ws01-core | E | strong | — | — | Unify local-LLM endpoint/model config — one source of truth for daemon + weft agent + voic |
| WEFT-605 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | exec_shell security allowlist is invisible to the model — denial spiral burns max tool ite |
| WEFT-606 | Done | medium | 0.8.x | ws10-voice | E | ok | — | — | Voice Talk-Mode turns are not anchored to the witness chain (standalone weft voice talk) |
| WEFT-607 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | agent.turn.record RPC — voice Talk-Mode turns anchored via the existing sink+anchor path |
| WEFT-608 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Kokoro TTS spoke garbled non-English — char-level tokenization vs IPA phoneme table |
| WEFT-609 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Talk-Mode deaf in a loud room — fixed -45 dBFS VAD gate vs -37 dBFS room tone |
| WEFT-610 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Talk-Mode said only 'One sec' — silent slow TTS + self-barge-in + premature capture resume |
| WEFT-611 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: turn-taking knobs + spoken self-enrollment + persistent speaker registry |
| WEFT-612 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: Orpheus prompt + sampling (was: slow tier zero audio) |
| WEFT-613 | Done | medium | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: Chatterbox cloned-voice fast tier (native port) |
| WEFT-614 | Done | medium | 0.9.x | ws10-voice | E | weak | — | — | Voicelab parity: grounded agent LLM (web_search / tool-calling) in the voice loop |
| WEFT-615 | Done | none | 0.8.x | ws10-voice | E | strong | — | — | ws10: Re-enable barge-in — reframed as ERL-confidence-floor decision (ADR-068 D1) |
| WEFT-616 | Done | none | 0.8.x | ws06-memory | C | strong | — | WEFT-652 | ws06: Prototype agenticow COW memory checkpointing in the hermes loop |
| WEFT-617 | Done | none | 0.8.x | ws10-voice | E | strong | WEFT-714,WEFT-715 | WEFT-714,WEFT-715 | ws10: Evaluate midstream for voice/ECC mid-stream gating (50ms CognitiveTick) |
| WEFT-618 | Done | none | 0.8.x | ws05-channels | H | weak | — | — | ws13: substrate/channels ADR set informed by AgentBBS (patterns only — FSL, no code reuse) |
| WEFT-619 | Done | none | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: K6 — vendor exo-core (BLAKE3+HLC) + exo-dag (DagStore/MMR/SMT, no postgres) per ADR- |
| WEFT-620 | Done | none | 0.8.x | ws17-research | I | weak | — | — | ws17: Integrate ruvnet-brain into ruv-researcher agent + .planning/ruv/ |
| WEFT-621 | Done | none | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: Clear FSL licensing question for any AgentBBS / late.sh source reuse |
| WEFT-622 | Done | none | 0.8.x | ws01-core | B | weak | — | — | M2: one conversation engine — text commits Frontier→Committed on a shared forest |
| WEFT-623 | Done | none | 0.8.x | ws06-memory | C | weak | — | — | M3: store collapse — one store |
| WEFT-624 | Done | none | 0.8.x | ws07-multi-agent | D | weak | — | — | M4: agent-initiated work — subagent spawn tools + governance |
| WEFT-625 | Done | none | 0.8.x | ws11-agent-core-v1 | D | weak | — | — | ADR-067 + ADR-068 authored; P0 scaffolds (duplex Phase 0 + conversation.graph RPC) |
| WEFT-626 | Done | none | 0.8.x | ws03-pipeline | B | weak | — | — | Classification Phase A — turn classification pipeline (Done) |
| WEFT-627 | Done | none | 0.8.x | ws03-pipeline | B | strong | — | — | Classification Phase B — B1/B2/B3 landed (Done) |
| WEFT-628 | Done | none | 0.8.x | ws10-voice | E | strong | WEFT-649,WEFT-650 | WEFT-646,WEFT-647,WEFT-648 | ADR-068 Phase 1 — desktop thin edge over localhost + ERL-into-compute_urgency |
| WEFT-629 | Done | none | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ADR-067 P1-graph — causal.node.state chain event + fold replay |
| WEFT-630 | Done | none | 0.8.x | ws08-weftos-gui | F | strong | — | — | ADR-067 G1-G5 GUI phases — umbrella |
| WEFT-631 | Done | none | 0.8.x | ws07-multi-agent | D | strong | — | — | Per-child CostBudget enforcement (budget hint threaded, not enforced) |
| WEFT-632 | Done | none | 0.8.x | ws07-multi-agent | D | strong | — | — | M4 live-capture residual — force tool selection via tool_choice (optional) |
| WEFT-633 | Done | none | 0.8.x | ws07-multi-agent | D | strong | — | — | D6 approval-UX — spawn triggers in-conversation approval (Defer + grant); GA end-state |
| WEFT-634 | Done | none | 0.8.x | ws02-kernel | B | strong | — | — | Governance rules gain action/tool selectors (engine is pure magnitude today) |
| WEFT-635 | Done | none | 0.8.x | ws07-multi-agent | D | strong | — | — | Spawn-at-user-level permission story |
| WEFT-636 | Done | none | 0.8.x | ws02-kernel | B | strong | — | — | Per-child / per-user gate principals (attribution; control holds today) |
| WEFT-637 | Done | none | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | Tools-as-nodes enrichment — deterministic spawn-edge rooting (M2 D3 seam) |
| WEFT-638 | Done | none | 0.8.x | ws10-voice | E | strong | — | — | Voice cutover eventually retires TalkForest (ADR-068) |
| WEFT-639 | Done | none | 0.8.x | ws15-mcp | J | strong | — | — | plane.sh wrapper fixes: WEFT-N resolution, real assignee lookup, cycle-membership via issu |
| WEFT-640 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | Real embedder: e5-small-v2 + record verbalization (replace SimHash placeholder) |
| WEFT-641 | Done | high | 0.8.x | ws06-memory | C | ok | — | — | AtomRegistry + atom.locate resolver + cross-index consistency audit (ADR-069 Panopticon) |
| WEFT-642 | Done | high | 0.8.x | ws06-memory | C | weak | — | — | ECC brain HNSW cannot join back to the atom spine (chain_seq hardcoded 0) |
| WEFT-643 | Done | high | 0.8.x | ws14-deployment | A | ok | — | — | Installer/version DX: build.sh install + SHA-stamped binaries + CLI-daemon mismatch warnin |
| WEFT-644 | Done | medium | 0.8.x | ws10-voice | E | weak | — | — | SileroVoiceness: neural VAD behind the Voiceness trait (model staging + stateful ONNX + fa |
| WEFT-645 | Done | medium | 0.8.x | ws01-core | J | ok | — | — | tests: make clawft-rpc no-daemon client tests hermetic (fail when a live daemon is up) |
| WEFT-646 | Done | high | 0.8.x | ws10-voice | E | weak | WEFT-628 | — | voice-wave2 §W2.2: interrupt taxonomy + router (InterruptAction, busy×intent×paralinguisti |
| WEFT-647 | Done | high | 0.8.x | ws10-voice | E | weak | WEFT-628 | — | voice-wave2 §W2.3: cancel→prune→Contradicts + witness executor (M2-D8 forest record) |
| WEFT-648 | Done | high | 0.8.x | ws10-voice | E | weak | WEFT-628 | — | voice-wave2 §W2.4: Refine executor — conservative cancel-and-resubmit + ReplySubmitter sea |
| WEFT-649 | Done | high | 0.8.x | ws10-voice | E | ok | WEFT-628 | WEFT-628 | voice-wave2 §W2.1: non-blocking capture loop — route voice turns into agent.chat + wire ro |
| WEFT-650 | Done | medium | 0.8.x | ws10-voice | E | weak | WEFT-628 | WEFT-628 | voice-wave2 §W2.6/W2.7: interrupt surface on voice watch + Wave 2 exit test |
| WEFT-651 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | ok | WEFT-665 | — | agent loop: runaway identical tool-call retries (canvas missing-content ×20) — schema-echo |
| WEFT-652 | Done | low | 0.8.x | ws06-memory | C | strong | WEFT-616 | — | ws06: hermes loop — cubecow additions: event-level snapshot cadence + AutoPause/AutoResume |
| WEFT-653 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: hermes loop — retained-output review gate: M2 design amendment (Speculative→review→C |
| WEFT-654 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: hermes loop — TurnProposal view + agent.proposal.{list,accept,discard} RPCs |
| WEFT-655 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | WEFT-673 | ws11: hermes loop — cancellation alignment: discarded turns retain witnessed partial trace |
| WEFT-656 | Done | medium | 0.8.x | ws12-knowledge-graph | C | strong | — | WEFT-366 | ws12: vector — fail loud on diskann config/feature mismatch (vector.strict) + build.sh nam |
| WEFT-657 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: voice — pocket-tts watch: adopt as fast-tier engine when official ONNX/Candle export |
| WEFT-658 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: ack variance + contextual 'what I'm looking at' filler (talk-mode UX, hot-mic feedb |
| WEFT-659 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: unclear-input gate — consume STT confidence/SNR/paralinguistics before engaging the |
| WEFT-660 | Done | high | 0.8.x | ws12-knowledge-graph | C | weak | — | — | vector: real DiskAnnBackend::search hardcodes SearchResult.id=0 for every hit |
| WEFT-661 | Done | high | 0.8.x | ws12-knowledge-graph | C | weak | — | — | vector: HybridBackend merges cosine (hot) and sqeuclidean (cold) raw distances — recall 0. |
| WEFT-662 | Done | medium | 0.8.x | ws06-memory | C | ok | — | — | upstream rvf-runtime 0.2: report 3 bugs (macOS __errno_location link failure; open() reset |
| WEFT-663 | Done | medium | 0.8.x | ws16-browser-wasm | G | ok | — | WEFT-672 | clawft-core browser target: 10 Send-future errors in agent/local_file_sink.rs (pre-existin |
| WEFT-664 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: replace spoken ack/filler with light cue tones |
| WEFT-665 | Done | high | 0.8.x | ws06-memory | C | weak | — | WEFT-651 | memory: graft debris poisoning MEMORY.md + contentless graft rendering |
| WEFT-666 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice watch: decision trace (gates, router, model, timings, reasoning) |
| WEFT-667 | Done | medium | 0.8.x | ws18-firmware | F | strong | — | — | ws13: edge-pad firmware — tilde-pin esp-hal/esp-radio (unstable feature + caret pin is a l |
| WEFT-668 | Done | low | 0.8.x | ws18-firmware | F | strong | — | — | ws13: edge-pad firmware — set-wise esp-* version bump (all one minor behind; esp-radio 0.1 |
| WEFT-669 | Done | high | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — AgentDB store split left 188 legacy entries stranded (clawft-knowledge, ruv |
| WEFT-670 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — memory_import drops the tags column (128 legacy entries migrated without th |
| WEFT-671 | Done | medium | 0.8.x | ws10-voice | E | strong | — | WEFT-214 | ws10: voice — decide the disposition of clawft-plugin/src/voice (blocks 12 audit-era items |
| WEFT-672 | Done | high | 0.8.x | ws16-browser-wasm | D | strong | WEFT-663 | — | ws16: browser target — clawft_llm::hermes::strip_think called ungated from pipeline/transp |
| WEFT-673 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | WEFT-655 | — | ws11: hermes loop — voice-review-gate residual gaps self-documented by WEFT-655 (forest-co |
| WEFT-674 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: CI — pr-gates.yml is PR-triggered only, so hard gates never run on feature branches  |
| WEFT-675 | Done | medium | 0.8.x | ws08-weftos-gui | C | strong | — | — | ws08/ws18: leaf display + ESP32-S3 firmware rewrite — vector-first scene pipeline, 7 new c |
| WEFT-676 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06/ws11: ADR-058/059 memory + context tier — Qwen3 ONNX embedder, L2 SessionTier, graft/ |
| WEFT-677 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: ADR-060 Track A — LocalProvider Hermes serving provider, tool_call round-trip + thin |
| WEFT-678 | Done | medium | 0.8.x | ws10-voice | C | strong | — | — | ws10: ADR-061 Track D voice-front initial build — native AEC, ECAPA embedder, dual-layer T |
| WEFT-679 | Done | medium | 0.8.x | ws14-deployment | A | strong | — | — | ws14: dependency-advisory patch round — quinn-proto/memmap2/rkyv CVEs patched, wasmtime DE |
| WEFT-680 | Done | medium | 0.8.x | ws14-deployment | A | strong | WEFT-600 | — | ws14: Docker/release hardening — v0.6.20 cut, v0.6.21 cut-then-reverted, Alpine + non-root |
| WEFT-681 | Done | medium | 0.8.x | ws14-deployment | A | strong | — | — | ws14: security — wasmtime advisory deferred during the 2026-06-28 patch round and never tr |
| WEFT-682 | Done | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: tracker — enforce the two-label rule at item creation (36 items unlabeled, all post- |
| WEFT-683 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — ADR-031 drift: RVF encoding declared the production default but only JSON was |
| WEFT-684 | Done | high | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — MCP server runs 'npx ruflo@latest', so a schema-bearing dep is unpinned (ro |
| WEFT-685 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | WEFT-694 | ws08: ADR-073 Phase A — stock desktop + agents inventory (Agent Workspace foundation) |
| WEFT-686 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | — | WEFT-695 | ws08: ADR-073 Phase B — freeform window manager v1 (multi-pane stage) |
| WEFT-687 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: ADR-073 Phase C — Agent Workspace mode (spawn-opens-pane + attention bus) |
| WEFT-688 | Done | high | 0.8.x | ws08-weftos-gui | E | strong | — | WEFT-695,WEFT-702 | ws08: ADR-073 Phase D — WindowIntent bus (voice/keys/MCP conductor demo) |
| WEFT-689 | Done | high | 0.8.x | ws10-voice | E | strong | — | — | ws10: ADR-074 V0 — xAI Realtime client + VoiceConfig (interim primary when key) |
| WEFT-690 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws10: ADR-074 V1 — xAI S2S tool bridge → WindowIntent (spawn/focus/summarize) |
| WEFT-691 | Done | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: ADR-074 V2 — hybrid modes + metrics + graceful local fallback |
| WEFT-692 | Done | high | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-075 G0 — Grok↔WeftOS MCP docs + project config + doctor path |
| WEFT-693 | Cancelled | high | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-075 G1 — curated MCP serve profile (control vs full) |
| WEFT-694 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | WEFT-685 | — | ws15: ADR-075 G2 — control tools (status, agents list/spawn) over MCP |
| WEFT-695 | Done | high | 0.9.x | ws08-weftos-gui | F | strong | WEFT-686,WEFT-688 | — | ws15: ADR-075 G3 — MCP tools → WindowIntent (Grok conductor) |
| WEFT-696 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-075 G4 — HTTP/SSE mcp-server listen + auth (remote Grok) |
| WEFT-697 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-075 G5 — MCP session capability tokens + audit client label |
| WEFT-698 | Done | high | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-076 C0 — capability catalog doc + surface principles lock-in |
| WEFT-699 | Done | high | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-076 C1 — weft mcp-server --profile (default ≠ full) |
| WEFT-700 | Done | high | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-076 C2 — public names, skill_list/get, process_spawn, reexport guard |
| WEFT-701 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws15: ADR-076 C3 — mcp-server --attach weave façade (live agents/status) |
| WEFT-702 | Done | medium | 0.9.x | ws08-weftos-gui | F | strong | WEFT-688 | — | ws15: ADR-076 C4 — window_* MCP + media profile gating |
| WEFT-703 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: ADR-076 C5 — catalog↔code drift CI |
| WEFT-704 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ADR-077 A0 — capture protocol v1 + splatd image-set ingest |
| WEFT-705 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ADR-077 A1 — Android capture MVP (CameraX + IMU coverage + ZIP) |
| WEFT-706 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ADR-077 A2 — pair + upload to Mac/cloud splatd + SOG review |
| WEFT-707 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws14: ADR-077 A3 — WeftOS edge core in Android (identity UniFFI) |
| WEFT-708 | Done | high | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws17: ADR-078 W0 — publish SPLAT_SCENE + world_model stub on job done |
| WEFT-709 | Done | medium | 0.8.x | ws17-research | I | strong | WEFT-721,WEFT-722 | WEFT-723 | ws17: ADR-078 W1 — geometric partition (surfaces, object AABBs, free space) |
| WEFT-710 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: free-form quilt Q0 — region/contribution + camera-stats schema |
| WEFT-711 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: multi-cam rig M0 — multi-camera session ingest (known poses) |
| WEFT-712 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: splat train backends T0 — registry + docs (Brush/NuRec/collection) |
| WEFT-713 | Done | medium | 0.8.x | ws17-research | I | strong | — | — | ws17: ADR-079 E0 — Earth digital twin doctrine + world-builder expert |
| WEFT-714 | Done | low | 0.8.x | ws10-voice | E | weak | WEFT-617 | WEFT-617 | ws10: Vendor midstream temporal-compare for IU partial prefix-diff (WEFT-617 Phase A) |
| WEFT-715 | Done | low | 0.8.x | ws10-voice | E | weak | WEFT-617 | WEFT-617 | ws10: Midstream stall analyzer → CoherenceAlert on CognitiveTick (WEFT-617 Phase B) |
| WEFT-716 | Done | low | 0.8.x | ws02-kernel | B | ok | — | WEFT-721 | ws02: BVH Phase A — finish clawft-bvh broad-phase (no chain) |
| WEFT-717 | Done | low | 0.8.x | ws02-kernel | B | strong | — | WEFT-721 | ws02: BVH Phase B — weftos-leaf-types spatial tag registry |
| WEFT-718 | Done | low | 0.8.x | ws02-kernel | B | strong | — | WEFT-721 | ws02: BVH Phase C — SpatialBackend + SpatialService + ChainSink |
| WEFT-719 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: BVH Phase D — determinism phase seal + COW branch_diff |
| WEFT-720 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: BVH Phase E — weaver ecc spatial CLI + E2E |
| WEFT-721 | Done | medium | 0.8.x | ws02-kernel | B | strong | WEFT-716,WEFT-717,WEFT-718 | WEFT-709 | ws02/ws17: BVH leaf payload schema — typed payload + embedding-field decision before WEFT- |
| WEFT-722 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | WEFT-709 | ws02/ws17: BVH leaf embedding decision — should a spatial leaf reference a feature vector, |
| WEFT-723 | Done | low | 0.8.x | ws02-kernel | B | strong | WEFT-709 | — | ws02: BVH × HNSW Phase F join — dual-index composition after ADR-088 VectorRef |

---

## Open tickets by workstream

### ws03-pipeline (3 open)

#### WEFT-41 — ws03: research — Iteration 3 EML attention multi-param coordinated perturbation

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Iteration 3 gate: multi-param coordinated perturbation on SafeTree; target ≥80% MSE reduction at (seq_len=4, d_model=8) and final_mse < 5e-2. Not attempted yet. Iterations 4-5+ (full EML-Transformer, hybrid scoring) explicitly aspirational. No tracking issue or plan stub.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-57 — ws03: research — track 80+ heuristics from eml-synergy-scan

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: 80+ hardcoded heuristics across graphify, kernel, LLM, assessment, and bench subsystems are listed as EML candidates. Notable pipeline-relevant ones: assessment/effects.rs weighted scores, pipeline/scorer.rs weights (covered by Q10), LLM provider cost-vs-quality trade-off ($0.01/1K threshold). Scan is exploratory.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-58 — ws03: research — track HNSW EML opportunities (adaptive ef, learned distance)

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: 10 EML opportunities for HNSW (adaptive ef, learned distance, cosine decomposition, search-path prediction, etc.). Items 3-10 are research; items 1-2 (adaptive beam, learned distance) are not yet implemented.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws09-clawft-dashboard (9 open)

#### WEFT-301 — ws09: api-bridge — wire skill install/uninstall to real loader

- **State**: Todo · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: bridge.rs:282 (// TODO: implement skill installation via ClawHub registry) and :287 (// TODO: implement skill uninstallation) both currently return Err("not implemented"). The /skills UI route renders an Install/Uninstall button that always fails. Real implementation depends on Skill loader (C3) and ClawHub registry (K
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-304 — ws09: api — replace mock delegation handlers with FlowDelegator wiring

- **State**: Todo · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: All four delegation handlers in crates/clawft-services/src/api/delegation.rs return hardcoded fixtures (three mock delegations, mock rules). The /delegation UI's Active / Rules / History tabs render that fake data. Real wiring depends on FlowDelegator events from M1/M2.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-305 — ws09: api — replace mock monitoring handlers with metrics collector

- **State**: Todo · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: crates/clawft-services/src/api/monitoring.rs returns hardcoded provider/session totals and ADR-026 tier costs. The /monitoring UI renders those fakes. Real wiring depends on metrics collector + sender_id thread (D5/D6).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-316 — ws09: auth — Tailscale provider and per-user session isolation

- **State**: Todo · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The gateway only supports the local Bearer-token flow today. SPARC plan calls for a Tailscale auth provider validating X-Tailscale-User-* headers plus source-IP / cert verification, and per-user session isolation so memory / sessions / config scopes do not leak across users.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-574 — ws09: tauri — desktop shell functional features (tray, hotkey, side-car, Spotlight, notifications, build.sh)

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-313 was closed as "scaffold shipped", but none of the six functional ACs landed. clawft-ui/src-tauri/src/lib.rs:1-16 explicitly lists the gaps: System tray with agent-status colour states (not shipped). Global hotkey Cmd+Shift+W / Ctrl+Shift+W (not shipped). weft-gateway side-car launch on app start, terminate on 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-312 — ws09: responsive — mobile sidebar drawer and chat input

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: MainLayout.tsx has no responsive drawer for screens < 768 px. Mobile WebChat lacks a bottom-anchored input and swipe nav between sessions. S3.3 is entirely unstarted.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-560 — ws09: pwa — push notifications via VAPID + WS event bridge

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: gap, ws09-clawft-dashboard
- **Blocked by**: none
- **Blocks**: WEFT-311
- **Gap**: Followup from WEFT-311 (manifest + offline shell shipped). Push notifications need: gateway-side push subscription endpoints (POST /api/push/subscribe, DELETE /api/push/unsubscribe) VAPID keypair generation and server-side dispatch on configured WS events (e.g. agent_done, channe
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-561 — ws09: ui — axe-core + Playwright a11y suite across all routes

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: tests, ws09-clawft-dashboard
- **Blocked by**: none
- **Blocks**: WEFT-315, WEFT-575
- **Gap**: Followup from WEFT-315 (jsx-a11y static lint + bundle-size gate shipped). The full runtime a11y audit needs: Set up Playwright suite for clawft-ui (no test infra exists yet) Integrate @axe-core/playwright; visit each of the 14 routes (/, /agents, /canvas, /chat, /sessions, /too
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-575 — ws09: ui — axe-core runtime a11y scan still missing (WEFT-315 AC unmet, follow-up to WEFT-561)

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W1 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: WEFT-561
- **Blocks**: none
- **Gap**: WEFT-315 shipped the bundle-size budget gate (scripts/bench/check-ui-bundle-size.sh) and an eslint-plugin-jsx-a11y static lint pass. The original AC explicitly required "axe-core integrated into the Playwright suite or run as a standalone script across all 14 routes" — that runtime axe-core scan is not in the tree. The
- **Plan**: Wait for WEFT-561

### ws10-voice (1 open)

#### WEFT-657 — ws10: voice — pocket-tts watch: adopt as fast-tier engine when official ONNX/Candle export ships

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Watch item. pocket-tts (100M-param Mimi-codec streaming TTS; ~200ms TTFA and ~6x real-time on 2 CPU cores claimed; voice cloning; MIT code / gated CC-BY-4.0 weights) is a strong candidate to replace Kokoro in the fast-ack tier — and in-process synthesis would make barge-in cancel instantaneous (drop the generator) vs H
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws17-research (2 open)

#### WEFT-538 — ws17: quantum — scaffold cuDensityMat SimulatorBackend behind quantum-nvidia feature flag

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: cuDensityMat-backed SimulatorBackend queued for v0.7.x post-GUI behind quantum-nvidia feature flag. Python-sidecar first, FFI later. Not started.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-539 — ws17: gaming-robotics — kick off first symposium experiment

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: 8 experiments planned ($1,102 hardware budget, 7-week timeline). None executed. PERCEIVE-THINK-ACT (PTA) framework + 'DEMOCRITUS = servo control loop' thesis sit unproven.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

