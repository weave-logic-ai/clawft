# Plane Board Inventory — WeftOS

> Generated: 2026-07-30 21:10 UTC
> Source: Plane workspace `weftos`
> Machine-readable DAG: [`plane-dag.json`](./plane-dag.json)
> Wave plan: [`plane-wave-plan.md`](./plane-wave-plan.md)

## Summary

| Metric | Count |
|--------|------:|
| Total tickets | 677 |
| Open | 272 |
| In Progress | 1 |
| Done | 386 |
| Cancelled | 19 |
| Dependency edges | 52 |
| Inferred domain edges | 23 |
| Parallel waves | 7 |

### Open by cycle

- **0.8.x**: 181
- **0.9.x**: 70
- **1.0.x**: 21

### Open by workstream

- **ws10-voice**: 32
- **ws17-research**: 31
- **ws02-kernel**: 25
- **ws12-knowledge-graph**: 25
- **ws14-deployment**: 20
- **ws11-agent-core-v1**: 17
- **ws16-browser-wasm**: 17
- **ws09-clawft-dashboard**: 17
- **ws03-pipeline**: 16
- **ws08-weftos-gui**: 14
- **ws07-multi-agent**: 12
- **ws13-app-substrate**: 10
- **ws15-mcp**: 9
- **ws06-memory**: 9
- **ws01-core**: 8
- **ws05-channels**: 7
- **ws04-plugin-skills**: 3

### Open by priority

- **high**: 29
- **medium**: 56
- **low**: 172
- **none**: 15

---

## Complete ticket table

| WEFT | State | Pri | Cycle | WS | Lane | AC | Blocked-by | Blocks | Name |
|------|-------|-----|-------|----|------|----|------------|--------|------|
| WEFT-8 | Done | high | 0.7.x | ws14-deployment | A | strong | WEFT-251 | WEFT-19 | ws14: workspace deps — migrate clawft-* path-deps to [workspace.dependencies] inheritance |
| WEFT-9 | Cancelled | high | 0.7.x | ws01-core | B | strong | — | — | ws01: foundation — reconcile ADR-044 wasip1 vs .cargo/config wasip2 alias |
| WEFT-10 | Done | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: bootstrap — split workspace from global at loader for PermissionResolver ceiling |
| WEFT-11 | Todo | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — implement Windows daemon transport (named pipes) for DaemonClient |
| WEFT-12 | Done | low | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — replace version_check curl shell-out with reqwest |
| WEFT-13 | Todo | medium | 0.8.x | ws01-core | G | strong | — | — | ws01: platform — implement OPFS-backed BrowserFileSystem persistence |
| WEFT-14 | Todo | low | 0.8.x | ws01-core | G | strong | — | — | ws01: platform — land OPFS-or-equivalent BrowserEnvironment persistence |
| WEFT-15 | Todo | low | 0.8.x | ws01-core | B | strong | — | — | ws01: kernel-config — wire LogQuantizedStubConfig + SimdDistanceStubConfig runtime |
| WEFT-16 | Done | medium | 0.7.x | ws01-core | B | strong | — | — | ws01: security — rationalize lenient validate_mcp_tool_name vs strict variant |
| WEFT-17 | Cancelled | medium | 0.8.x | ws01-core | B | strong | — | — | ws01: rpc — add chain.append RPC for weaver soul promote |
| WEFT-18 | Todo | low | 0.8.x | ws01-core | B | strong | — | — | ws01: foundation — run ADR-010 v0.3 cancel-correctness audit on mesh select! branches |
| WEFT-19 | Done | low | 0.7.x | ws01-core | B | strong | WEFT-8 | — | ws01,ws14: publish-policy audit — flip 16 publish=false flags or document |
| WEFT-20 | Todo | low | 0.9.x | ws01-core | B | strong | — | — | ws01: types — decide deny_unknown_fields lint mode for Config |
| WEFT-21 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: platform — document config_loader Layer 2 sync vs Layer 3 async asymmetry |
| WEFT-22 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: cli — remove TODO(E1) and TODO(C5) markers in workstream-I notes |
| WEFT-23 | Done | medium | 0.7.x | ws01-core | B | strong | — | — | ws01: cli — replace skills_cmd derived-on-first-sign placeholder pubkey with real Ed25519 |
| WEFT-24 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: planning — close out improvements.md Phase-5 sprint-tracker |
| WEFT-25 | Done | low | 0.7.x | ws01-core | B | strong | — | — | ws01: planning — archive stale 00-initial-sprint codebase-map / planning-summary |
| WEFT-26 | Todo | low | 0.8.x | ws01-core | B | strong | — | — | ws01: types — clean up panic! macros in test-only canvas/provider/agent_bus arms |
| WEFT-27 | Done | high | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — apply tier check to fallback model selection |
| WEFT-28 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — HMAC the cost-tracker persistence file |
| WEFT-29 | Done | low | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — reject window_seconds=0 in Phase H validation |
| WEFT-30 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — redact RoutingDecision.reason to avoid info disclosure |
| WEFT-31 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — audit-log model_override bypasses (escalation already logs) |
| WEFT-32 | Done | high | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — add MCP tool-name namespace validation against wildcard ['*'] |
| WEFT-33 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — scaffold fuzz targets for 8 attack surfaces |
| WEFT-34 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-002 (DashMap vs RwLock<HashMap> benchmark) |
| WEFT-35 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-003 final review (escalation security) |
| WEFT-36 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — resolve CONS-006 (config validation boundary) |
| WEFT-37 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — wire D1 per-path advisory locks for parallel tool execution |
| WEFT-38 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — wire evolution_ready flag → mutation.rs GA loop (ADR-017 flywheel) |
| WEFT-39 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — persist RetryModel learned weights across daemon restarts |
| WEFT-40 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — surface routing-decision history via admin endpoint |
| WEFT-41 | Todo | low | 1.0.x | ws03-pipeline | B | strong | — | — | ws03: research — Iteration 3 EML attention multi-param coordinated perturbation |
| WEFT-42 | Done | low | 0.9.x | ws03-pipeline | B | strong | — | — | ws03: kernel — wire sprint-16 two-tier EML coherence cadence |
| WEFT-43 | Todo | low | 0.9.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — decide consolidation of clawft-service-llm vs clawft-llm |
| WEFT-44 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: service-llm — handle non-string content (vision blocks / structured) in LlmClient |
| WEFT-45 | Todo | medium | 0.9.x | ws03-pipeline | C | strong | — | — | ws03: routing — wire MicroLoraRouter (v3) once ruvllm-wasm lifts 11-pattern HNSW cap |
| WEFT-46 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — wire v2.5 sona-backed rerank step in HybridRouter |
| WEFT-47 | Done | medium | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: routing — add max_grantable_level field to RoutingConfig |
| WEFT-48 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: rate-limiter — expose rate-limiter metrics via admin endpoint (Element-09) |
| WEFT-49 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: rate-limiter — expose rate-limiter LRU maintenance via admin endpoint (Element-09) |
| WEFT-50 | Done | low | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: context-router — document Some(vec![]) tool_subset contract for plugin authors |
| WEFT-51 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: context-router — exhaustively test embedding-router cargo-feature-off path |
| WEFT-52 | Done | medium | 0.7.x | ws03-pipeline | B | strong | — | — | ws03: routing — verify admin user x restricted channel interaction |
| WEFT-53 | Todo | low | 0.9.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — decide EML score-fusion in scope for 0.7.0 (FitnessScorer weights) |
| WEFT-54 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — review FitnessScorer.error_indicators allowlist (localization, jailbreak) |
| WEFT-55 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — verify experimental-attention CI build/test wiring |
| WEFT-56 | Todo | low | 0.8.x | ws03-pipeline | B | strong | — | — | ws03: pipeline — define explicit pipeline-pass step in scripts/build.sh gate |
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
| WEFT-70 | Todo | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: ux — add macOS-sandbox-downgrade warning to startup banner |
| WEFT-71 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: tests — add clawft.plugin.json schema roundtrip + version-compat test |
| WEFT-72 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — verify (or close) SkillContext::Fork status post-3F review M2 |
| WEFT-73 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: tests — land T39 plugin-lifecycle tests |
| WEFT-74 | Todo | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — define pending-skill review timing (interactive prompt vs CLI) |
| WEFT-75 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: autogen — define filesystem allowlist semantics for autogenerated skills |
| WEFT-76 | Todo | low | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: skills — add weft skills refresh CLI for headless/CI scenarios |
| WEFT-77 | Done | low | 0.8.x | ws04-plugin-skills | E | strong | — | — | ws04: voice — drop or stub VoiceHandler trait placeholder for 0.7.0 |
| WEFT-78 | Done | medium | 0.8.x | ws04-plugin-skills | B | strong | — | — | ws04: scaffold — add Rust struct/parser for .weftos-plugin.toml or remove it |
| WEFT-79 | Done | high | 0.7.x | ws06-memory | C | strong | — | — | ws06: memory — route MemoryStore + SkillsLoader through WorkspaceContext |
| WEFT-80 | Cancelled | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: bootstrap — split workspace from global at loader for PermissionResolver ceiling (MW |
| WEFT-81 | Cancelled | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: governance — implement chain.append RPC for weaver soul promote (MW-3) |
| WEFT-82 | Done | high | 0.7.x | ws06-memory | C | strong | — | — | ws06: tests — convert overlay_probe.rs from #[ignore] to hermetic temp-workspace test (MW- |
| WEFT-83 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — add agent.workspace_root config key (MW-5) |
| WEFT-84 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — rebuild memory.rvf.json when MEMORY.md changes (MW-6) |
| WEFT-85 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: substrate — emit chain_event! for session.append on every appended turn (MW-7) |
| WEFT-86 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06: workspace — align WorkspaceManager::delete with FR-W06 (MW-8) |
| WEFT-87 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: sessions — ship weft session gc (or self-cleanup migration path) (MW-9) |
| WEFT-88 | Done | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: workspace — update last_accessed in WorkspaceManager::load (MW-10) |
| WEFT-89 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: planning — backfill empty 08-memory-workspace decisions/blockers notes (MW-11) |
| WEFT-90 | Done | medium | 0.7.x | ws06-memory | C | strong | — | — | ws06: planning — re-walk 3g-review.md and mark each ISSUE fixed/open/won't-do (MW-12) |
| WEFT-91 | Todo | low | 0.9.x | ws06-memory | C | strong | — | — | ws06: identity — decide whether FileIdentityProvider needs notify watcher (MW-13) |
| WEFT-92 | Todo | medium | 0.9.x | ws06-memory | C | strong | — | — | ws06: identity — decide binding-thread-mismatch policy refuse vs annotate (MW-14) |
| WEFT-93 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: embeddings — pick fate of rvf_stub.rs vs rvf_io.rs (MW-15) |
| WEFT-94 | Done | low | 0.7.x | ws06-memory | C | strong | — | — | ws06: workspace — document or drop per-agent tool_state/ subdirectory (MW-16) |
| WEFT-95 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — route IdentityLoader::current through Platform::fs() (MW-17) |
| WEFT-96 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — define journal substrate read-on-every-turn path (WS-D1) |
| WEFT-97 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: identity — substrate-backed Identity::source variant set (WS-D4 / WS-D5) |
| WEFT-98 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: kernel auth — add gate.check to revoke_token (DiD) |
| WEFT-99 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — re-enable auth middleware on /api/* and /ws |
| WEFT-100 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — replace CorsLayer::permissive() with deny-by-default |
| WEFT-101 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — add tower::limit::RateLimitLayer to /api/* and token endpoints |
| WEFT-102 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: services-api — add TokenStore::revoke_token + expired-token cleanup |
| WEFT-103 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: chain — add optional idempotency_key to ChainEvent (replay protection) |
| WEFT-104 | Done | medium | 0.7.x | ws02-kernel | B | strong | — | — | ws02: tooling — add cargo audit to scripts/build.sh gate and CI |
| WEFT-105 | Todo | high | 0.9.x | ws02-kernel | B | strong | WEFT-109,WEFT-112,WEFT-115 | — | ws02: mesh — implement K6.4 chain replay (LocalChain::tail_from + append_signed) |
| WEFT-106 | Todo | high | 0.9.x | ws02-kernel | B | strong | WEFT-115,WEFT-144 | — | ws02: mesh — implement K6.4 tree Merkle diff + signed mutations |
| WEFT-107 | Todo | high | 0.8.x | ws02-kernel | B | strong | WEFT-113 | — | ws02: mesh — implement S10 key-rotation chain event + verifier |
| WEFT-108 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement IpcScope::Restricted browser default + browser_policy rules |
| WEFT-109 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-105 | ws02: mesh — decide chain merge strategy (leader vs DAG) and split-brain handling |
| WEFT-110 | Done | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — decide and freeze KernelMessage wire format (JSON vs RVF) |
| WEFT-111 | Todo | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — decide full libp2p-kad vs lighter DHT |
| WEFT-112 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-105 | ws02: mesh — add InMemoryTransport / MockPeer / MockClock / FaultyTransport |
| WEFT-113 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-107 | ws02: mesh — define Clock trait and inject into time-dependent components |
| WEFT-114 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: ci — add cargo check --target wasm32-unknown-unknown (no mesh) to CI |
| WEFT-115 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-105,WEFT-106 | ws02: mesh — define missing K6 protocol struct types and msg_type enum |
| WEFT-116 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — resolve mesh_adapter.rs vs mesh_ipc.rs and mesh/handshake.rs layout |
| WEFT-117 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — wire AssessmentTransport into daemon + add weft assess mesh-status |
| WEFT-118 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — add QUIC transport (quinn + snow) alongside TCP/WS |
| WEFT-119 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — make Mesh a SystemService with start/stop/health_check |
| WEFT-120 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — wire ClusterService to mesh peer discovery |
| WEFT-121 | Done | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — implement mesh time-sync (authority election, offset smoothing, mesh_time) |
| WEFT-122 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: services-api — wire axum handlers to http_facade types + SSE loop |
| WEFT-123 | Todo | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: services-api — add HTTP facade integration tests once profile/pairing types land |
| WEFT-124 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — wire VectorBackend into DemocritusLoop |
| WEFT-125 | Todo | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — add ecc.vector-config RPC endpoint |
| WEFT-126 | Done | low | 0.9.x | ws02-kernel | B | strong | — | — | ws02: vector — ship real DiskANN backend behind diskann feature once ruvector-diskann publ |
| WEFT-127 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: vector — persist HNSW tombstones across save/load |
| WEFT-128 | Todo | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: vector — flip LogQuantized + SimdDistance is_available once ruvector-core PR #352 la |
| WEFT-129 | Todo | low | 0.8.x | ws02-kernel | A | strong | — | — | ws02: kernel — ship real Wasmtime backend for spectral_embedding (or move to deferred) |
| WEFT-130 | Done | high | 0.7.x | ws02-kernel | B | strong | WEFT-554 | WEFT-554 | ws02: exo-resource-tree — replace permission.rs always-Allow stub with K1 ACL engine |
| WEFT-131 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-554 | ws02: exo-resource-tree — implement DelegationCert lifecycle (grant/revoke + Ed25519 + exp |
| WEFT-132 | Cancelled | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: services-api — implement bridge.rs TODOs (skill, memory, config) |
| WEFT-133 | Done | medium | 0.9.x | ws02-kernel | B | strong | — | — | ws02: services-api — add CSP middleware to API tower stack |
| WEFT-134 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: tests — resolve test-suite hang in clawft-kernel --lib aggregate run |
| WEFT-135 | Todo | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: workspace — clean ~150 clippy errors (pre-existing debt) |
| WEFT-136 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: kernel — persist AppManager state to disk |
| WEFT-137 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: chain — implement chain-anchored anchoring beyond MockAnchor |
| WEFT-138 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — update docs/weftos/k-phases.md (K2.1/K3/K4/K5 mis-marked) |
| WEFT-139 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: docs — write docs/guides/kernel.md (deferred from K5) |
| WEFT-140 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — renumber duplicate ADRs (two ADR-020s, two ADR-028s) |
| WEFT-141 | Done | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: docs — accept ADR-023 (assessment-as-kernel-service) |
| WEFT-142 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: kernel — add NodeId composite for cross-node uniqueness + remote inbox bridge |
| WEFT-143 | Done | high | 0.7.x | ws02-kernel | B | strong | — | — | ws02: kernel — enforce max-message-size on KernelIpc::send (16 MiB) |
| WEFT-144 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | WEFT-106 | ws02: mesh — add MutationEvent Ed25519 signing for cross-node tree mutations |
| WEFT-145 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — incremental Merkle hash updates (replace full recompute_all) |
| WEFT-146 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — replace static GovernanceRule Vec with cluster-wide distribution |
| WEFT-147 | Todo | high | 0.9.x | ws02-kernel | B | strong | — | — | ws02: mesh — cross-node capability-claim verification (signed advertisement) |
| WEFT-148 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — rate-limit add_peer() and governance-evaluation requests |
| WEFT-149 | Done | low | 0.7.x | ws02-kernel | B | strong | — | — | ws02: docs — document DEMOCRITUS 'still stuck' log-line semantics |
| WEFT-150 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: kernel — verify weftos-leaf-types push path goes through governance / chain |
| WEFT-151 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — audit mesh_log/mesh_dedup/mesh_listener/mesh_bootstrap for callers |
| WEFT-152 | Todo | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: tests — confirm cognitum-gate-tilezero Permit/Defer/Deny path is exercised |
| WEFT-153 | Todo | low | 0.8.x | ws02-kernel | B | strong | — | — | ws02: chain — add EVENT_KIND_* constants for minor non-kernel chain gaps |
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
| WEFT-170 | Todo | medium | 0.8.x | ws05-channels | H | strong | — | — | ws05: PluginHost C7 unification — migrate Telegram/Discord/Slack to ChannelAdapter |
| WEFT-171 | Todo | medium | 0.9.x | ws05-channels | H | strong | — | — | ws05: Slash-command surface — decide consumer for ChannelHost::register_command |
| WEFT-172 | Done | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Telegram — document or remove redundant 1s poll-interval sleep |
| WEFT-173 | Todo | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Discord — document intents bitmask default and cover privileged-intent rejection |
| WEFT-174 | Todo | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: Slack — add unknown_envelope counter for API drift detection |
| WEFT-175 | Todo | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: iMessage scope — implement AppleScript bridge or formally drop from tracker |
| WEFT-176 | Todo | low | 0.8.x | ws05-channels | H | strong | — | — | ws05: WeftOS white-label — add brand() accessor and remove hard-coded clawft strings |
| WEFT-177 | Todo | low | 0.9.x | ws05-channels | H | strong | — | — | ws05: Channel failover chain — decide semantics and either implement or close as out-of-sc |
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
| WEFT-193 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: IDE provider — replace IdeToolProvider::stub() with real implementation |
| WEFT-194 | Todo | low | 0.9.x | ws07-multi-agent | D | strong | — | — | ws07: Hybrid context-router — wire MicroLoraRouter once agent-core-v1 phase E3+ is ready |
| WEFT-195 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: delegate_tool — drop hardcoded claude_available=true, query the delegator for livene |
| WEFT-196 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: weft delegate — add debug subcommand to surface routing decisions |
| WEFT-197 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: weft doctor — add multi-agent checks (claude on PATH, auto-delegation, ≥1 route) |
| WEFT-198 | Todo | low | 0.9.x | ws07-multi-agent | D | strong | — | — | ws07: claude-flow MCP server — decide whether to add by default to [tools.mcp_servers] |
| WEFT-199 | Todo | low | 0.8.x | ws07-multi-agent | B | strong | — | — | ws07: SwarmCoordinator topology — implement mesh/hierarchical/adaptive or document as prom |
| WEFT-200 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: notifications/tools/list_changed — handle inbound and advertise outbound |
| WEFT-201 | Todo | low | 0.8.x | ws07-multi-agent | D | strong | — | — | ws07: Auto-delegation classifier — improve regex+keyword accuracy or document fragility (3 |
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
| WEFT-214 | Todo | medium | 0.8.x | ws10-voice | E | strong | WEFT-671 | — | ws10: voice_listen / voice_speak tools — wire to real STT/TTS with cloud fallback |
| WEFT-215 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: weft voice setup — real model download with SHA-256 verify and progress UI |
| WEFT-216 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: WakeWordDetector — wire rustpotter or document an alternative |
| WEFT-217 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: EchoCanceller and NoiseSuppressor — replace deceptive passthroughs with real DSP |
| WEFT-218 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: WS voice:status — connect a real backend broadcaster |
| WEFT-219 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: /api/voice/* — replace MSW-only mocks with real handlers in clawft-services |
| WEFT-220 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Windows install-service — automate schtasks or document manual route as final |
| WEFT-221 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: Talk Mode interruption — abort TTS when VAD trips during playback |
| WEFT-222 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: VoicePersonality — wire per-agent lookup in TTS dispatch |
| WEFT-223 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-2 audio buffer zeroization and voice.audio_retention config |
| WEFT-224 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-3 cloud-fallback transparency log line |
| WEFT-225 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-6 anti-replay nonce and transcription-echo confirmation |
| WEFT-226 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: SC-8 voice rate limiting (commands/min, wake/min, post-fail cooldown) |
| WEFT-227 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Speaker diarization via sherpa-rs |
| WEFT-228 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Tauri-side native mic capture — replace browser-only getUserMedia path |
| WEFT-229 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Latency + WER + CPU benchmarks for voice pipeline |
| WEFT-230 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Adaptive silence timeout learning |
| WEFT-231 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: UI partial-transcription streaming and TTS word highlighting |
| WEFT-232 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Discord voice bridge — clawft-channels voice → STT → agent → TTS → VC audio |
| WEFT-233 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: audio_transcribe / audio_synthesize tools — real WAV/MP3/OGG/WebM codec support |
| WEFT-234 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: Cleanup orphan voice surfaces (events, statuses, voice-chat.ts, model_path types) |
| WEFT-235 | Done | low | 0.9.x | ws10-voice | E | strong | — | — | ws10: clawft-service-classify — decide adoption (connect to W-VOICE, Explorer-only, or del |
| WEFT-236 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: clawft-service-whisper — drop legacy dual-publish path post Phase-4 migration |
| WEFT-237 | Done | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: clawft-service-whisper publish_wav example — keep or delete |
| WEFT-238 | Todo | medium | 0.8.x | ws10-voice | E | strong | — | — | ws10: VoiceConfig.tts.provider="browser" — implement Web Speech dispatch or change default |
| WEFT-239 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: CloudFallbackConfig — config-string to provider router |
| WEFT-240 | Todo | low | 0.8.x | ws10-voice | E | strong | — | — | ws10: WakeConfig.sensitivity vs WakeWordConfig.threshold — unify the knob |
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
| WEFT-254 | Todo | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — multi-conversation sidebar UI |
| WEFT-255 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — system-prompt UI affordance |
| WEFT-256 | Todo | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — model / provider switcher in chip strip |
| WEFT-257 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — heartbeat label replaces spinner occlusion |
| WEFT-258 | Todo | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — real interactive defer (resume on { deferred: true }) |
| WEFT-259 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: chat panel — identity-drift / binding-thread mismatch warning |
| WEFT-260 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — mouse selection + clipboard |
| WEFT-261 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — bold/italic glyph variants |
| WEFT-262 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — scrollback view + wheel handler |
| WEFT-263 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — multi-tab terminal (HashMap<SessionId, Terminal>) |
| WEFT-264 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: terminal panel — real WASM terminal renderer |
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
| WEFT-275 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: explorer — Lineage Object Type + viewer (metadata convention sign-off) |
| WEFT-276 | Done | medium | 0.8.x | ws08-weftos-gui | B | strong | — | — | ws08: explorer — ObjectType::applicable_actions populated for Mesh/Sensor/Node |
| WEFT-277 | Todo | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: composer — honest_affordances real GEPA / governance intersection |
| WEFT-278 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — implement Grid layout (degrades to Rows today) |
| WEFT-279 | Done | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — implement Tabs layout (degrades to Rows today) |
| WEFT-280 | Done | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: workshop — wire viewer_hint overrides (today: "auto" only) |
| WEFT-281 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: graph viewer — editable Phase 3+ patch UI (egui_node_graph migration) |
| WEFT-282 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — capture sidecar (mic/camera) for vscode#303293 |
| WEFT-283 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — typed active-radar return schema (variant-id echo) |
| WEFT-284 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — ThreadDock primitive for per-agent parallel output |
| WEFT-285 | Todo | low | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel — WSP-0.1 verb support (raw RPC only today) |
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
| WEFT-326 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — stabilize append_turns_are_monotonic flake via injectable clock |
| WEFT-327 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — promote overlay_probe + resolver_live_probe diagnostics into CI |
| WEFT-328 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — plumb tool_calls / token / model fields through OutboundMessage |
| WEFT-329 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — notify-driven hot-reload watcher for identity files |
| WEFT-330 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — agent-side SOUL.journal.md write path during chat turns |
| WEFT-331 | Todo | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — interactive Defer UX prompt-and-resume in panel |
| WEFT-332 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — per-user agent_ids for multi-tenant chat |
| WEFT-333 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — register agent.chat SystemService for weft status |
| WEFT-334 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — typed error variants for agent.chat |
| WEFT-335 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — observability path logging router decisions to substrate |
| WEFT-336 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — weft routing trace + replay commands |
| WEFT-337 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v2.5 — sona-backed rerank step on HybridRouter |
| WEFT-338 | Todo | low | 0.9.x | ws11-agent-core-v1 | C | strong | — | — | ws11: agent-core-v3 — MicroLoraRouter behind ruvllm-wasm 11-pattern HNSW cap lift |
| WEFT-339 | Done | low | 0.9.x | ws11-agent-core-v1 | D | strong | — | — | ws11: de-duplicate clawft_weave::protocol vs clawft_service_agent::protocol types |
| WEFT-340 | Done | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: confirm agent.chat "agent service not wired" error path has integration coverage |
| WEFT-341 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — per-tool Permit token + proof-of-permission API |
| WEFT-342 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — hard-refuse on binding-thread mismatch (governance rule) |
| WEFT-343 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — Arc<RwLock<LlmClient>> runtime swap on env rotation |
| WEFT-344 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core-v1.1 — agent.workspace_root config key |
| WEFT-345 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — after-3-denials EscalateToHuman governance path |
| WEFT-346 | Cancelled | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — multi-conversation sidebar UI for panel |
| WEFT-347 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 4 MemoryConsolidator periodic distillation |
| WEFT-348 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 4 skills auto-promotion from .claude/skills to .clawft/skills |
| WEFT-349 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — cross-agent delegation via existing delegate_tool |
| WEFT-350 | Todo | low | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: agent-core — Phase 2 voice + streaming chat path |
| WEFT-351 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG — replace vector_diskann.rs HashMap linear-scan stub |
| WEFT-352 | Todo | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-011 — activate LogQuantized for DiskANN once shaal PR #352 merges |
| WEFT-353 | Todo | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-012 — activate unified SIMD distance kernel once shaal PR #352 merges |
| WEFT-354 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-013 — spatio-temporal GNN for sonobuoy (K-STEMIT) |
| WEFT-355 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-015 — EA-Agent entity alignment for multi-repo dedup |
| WEFT-356 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-017 — knowledge distillation for edge EML (SevenNet-Nano) |
| WEFT-357 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-018 — Newman modularity scoring as alternative to cohesion |
| WEFT-358 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-2 — OWL/RDF ingestion (Turtle, JSON-LD) |
| WEFT-359 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-3 — Barnes-Hut force layout + positioned-SVG export |
| WEFT-360 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: OG-4 — VOWL visual encoding rules in SVG export |
| WEFT-361 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: KG-004 — benchmark RFF vs Lanczos vs EML lambda₂ on 1K/10K/100K graphs |
| WEFT-362 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: layout — implement Sugiyama layered layout (currently falls back to tree) |
| WEFT-363 | Cancelled | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — wire VectorBackend into DemocritusLoop |
| WEFT-364 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — ecc.vector-config RPC to show active backend |
| WEFT-365 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: vector — diskann feature flag for real impl |
| WEFT-366 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | WEFT-656 | — | ws12: vector — hybrid vs pure HNSW benchmark for ECC workloads |
| WEFT-367 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: weaver graphify rebuild — full extraction-pipeline integration |
| WEFT-368 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: ingest — replace StubHttpClient with real reqwest-based HTTP client |
| WEFT-369 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — MCP server (Phase 6) |
| WEFT-370 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — extraction + graph_ops benchmarks (Phase 6) |
| WEFT-371 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — write ADR-049 (graphify port) |
| WEFT-372 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — write ADR-050..053 candidates from phase2 paper survey |
| WEFT-373 | Done | medium | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — incremental graph updates (LightRAG set-union dedup) |
| WEFT-374 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — multi-key HNSW indexing (LightRAG P2) |
| WEFT-375 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — edge embeddings for relationship queries (LightRAG P5) |
| WEFT-376 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — graph-aware HNSW re-ranking (LightRAG P4) |
| WEFT-377 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — discover_hyperedges() pipeline step |
| WEFT-378 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — vault domain hyperedges + SUGGEST→ratify→CRDT pipeline |
| WEFT-379 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — index-based optimization for forensic gap_analysis (O(n·m) cliff) |
| WEFT-380 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — adaptive HNSW rebuild_threshold (EML coherence two-tier) |
| WEFT-381 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — vision_extract end-to-end test fixture |
| WEFT-382 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — schema-based edge validation in validation.rs |
| WEFT-383 | Done | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — clean up dead clawft-llm optional dep flag |
| WEFT-384 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — adaptive ef (HNSW-EML opportunity) |
| WEFT-385 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — search-path prediction (HNSW-EML #4, biggest single win) |
| WEFT-386 | Done | low | 0.9.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — cosine-similarity decomposition for distance speedup |
| WEFT-387 | Todo | low | 0.8.x | ws12-knowledge-graph | C | strong | — | — | ws12: graphify — verify+restore standalone export/cypher.rs and export/svg.rs |
| WEFT-388 | Done | high | 0.7.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — wasm-bindgen-test regression suite for init() + send_message() pipeline |
| WEFT-389 | Done | high | 0.7.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — binary size budget audit (1.32 MB → wasm-opt -Oz CI gate) |
| WEFT-390 | Todo | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — streaming chat via ReadableStream / wasm-streams |
| WEFT-391 | Done | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — wire set_env to BrowserEnvironment via OnceLock<BrowserRuntime> |
| WEFT-392 | Done | medium | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — implement OPFS-backed BrowserFileSystem behind browser-opfs feature |
| WEFT-393 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write ADR-027 Browser WASM Support |
| WEFT-394 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write docs/development/feature-flags.md |
| WEFT-395 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — write docs/browser/cors-provider-setup.md + config-schema.md |
| WEFT-396 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — update root README.md and CLAUDE.md with browser build instructions |
| WEFT-397 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — compile_error! when both native and browser features are enabled |
| WEFT-398 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — split clawft-wasm host code into dedicated crate |
| WEFT-399 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — persistent conversation history via OPFS (CLAUDE.md-per-group) |
| WEFT-400 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — Web Worker harness variant |
| WEFT-401 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — gitignore or stub-replace pre-built www/pkg artifact |
| WEFT-402 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — fix unreachable_code warning in workspace/agent.rs:257 |
| WEFT-403 | Done | low | 0.9.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — audit ADR-044 vs reality (wasip1 vs wasip2 + script alignment) |
| WEFT-404 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — data-driven provider-routing fallback order |
| WEFT-405 | Todo | low | 0.8.x | ws16-browser-wasm | A | strong | — | — | ws16: browser — sign + version browser bundle artefact (parity with WASI release) |
| WEFT-406 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — threat-model note on api_key in JS-readable WASM memory |
| WEFT-407 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — performance profiling baseline (load, init, first-msg, memory) |
| WEFT-408 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: browser — final regression suite + ≤10% test-duration regression check (P6.7) |
| WEFT-409 | Done | low | 0.8.x | ws16-browser-wasm | G | strong | WEFT-562,WEFT-563 | WEFT-562,WEFT-563 | ws16: browser — retire or document scripts/check-features.sh contract |
| WEFT-410 | Todo | low | 0.9.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — decide UnknownMode validation variant fate |
| WEFT-411 | Todo | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — add registry corruption quarantine path |
| WEFT-412 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — emit lifecycle teardown tombstone on uninstall-while-enabled |
| WEFT-413 | Todo | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — wire ADR-015 rule 6 once clawft-adapter exists |
| WEFT-414 | Todo | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-app — cover wasm to_toml_string failure path with negative test |
| WEFT-415 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — emit substrate/meta/adapter/<id>/health from each adapter |
| WEFT-416 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — per-id Replace/Remove deltas on processes/services topics |
| WEFT-417 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — surface Subscription closed via adapter-health topic on teardown |
| WEFT-418 | Todo | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — migrate mic adapter to substrate/<node-id>/sensor/mic/{summary,pc |
| WEFT-419 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — ship a second Characterization exemplar (Enumerated or Spectral) |
| WEFT-420 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-substrate — implement cross-platform network/bluetooth or document Linux-only |
| WEFT-421 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — wire 13 stub-leaf canon primitives in the composer |
| WEFT-422 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — add .first/.last field-access shorthand support |
| WEFT-423 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — implement sort(list, key) ordering combinator |
| WEFT-424 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — accept scientific (1e5) and hex (0xff) number literals |
| WEFT-425 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — parse [compositions.*] and expand in composer |
| WEFT-426 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — drop unused egui dep from Cargo.toml |
| WEFT-427 | Todo | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — extract canon types and move composer back to clawft-surface |
| WEFT-428 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: clawft-surface — replace 14-line src/substrate.rs shim with direct re-export |
| WEFT-429 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — wire real ADR-012 governance::Gate through Substrate::subscribe_adapte |
| WEFT-430 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — implement honest affordance ∩ permit intersection in compose::honest_a |
| WEFT-431 | Todo | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — drive variant_id stamping in CanonResponse from surface binding |
| WEFT-432 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: integration — implement per-sensor healthcheck contract emitter |
| WEFT-433 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — enforce per-node-prefix write gate on substrate.publish |
| WEFT-434 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — add streaming log endpoint so kernel adapter drops poll fallback |
| WEFT-435 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: substrate-rpc — test substrate.notify consumer wakeup semantics in integration suite |
| WEFT-436 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — ship a Presence exemplar adapter |
| WEFT-437 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — implement HEALTHCHECK-CONTRACT.md as clawft-substrate::healthcheck module |
| WEFT-438 | Done | medium | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: sensors — resolve legacy-flat-path vs node-scoped-path naming and ship migration |
| WEFT-439 | Done | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: weftos-admin — add wired Modal ("confirm restart") to admin surface |
| WEFT-440 | Todo | low | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: weftos-admin — migrate auto-install-from-fixture flow off web-time workaround |
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
| WEFT-453 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — soft-check docs-site MDX builds locally via scripts/build.sh ui |
| WEFT-454 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cdn — snapshot every cdn-assets upload by commit SHA for rollback |
| WEFT-455 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add browser-WASM size budget to wasm-browser.yml |
| WEFT-456 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deploy — add health-probe rollback path to scripts/deploy/vps-deploy.sh |
| WEFT-457 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add macOS + Windows test job to pr-gates.yml |
| WEFT-458 | Done | low | 0.9.x | ws14-deployment | A | strong | — | — | ws14: ci — add cargo-audit / cargo-deny gate to pr-gates.yml |
| WEFT-459 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add SBOM (CycloneDX) generation and attach to releases |
| WEFT-460 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: tooling — add scripts/build.sh release-dry-run subcommand |
| WEFT-461 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: build-kb — move tools/build-kb into the workspace (or document why not) |
| WEFT-462 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cargo-dist — schedule v0.31 → v1.0+ bump and regenerate release.yml |
| WEFT-463 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — bump or delete scripts/09-gate.sh stale floor + paths |
| WEFT-464 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — wire scripts/k6-gate.sh into CI or mark developer-rehearsal |
| WEFT-465 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: scripts — audit and reorganize dead scripts (wake units, py helpers, weave-init.sh) |
| WEFT-466 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: planning — populate or delete empty deployment-community phase-K stubs |
| WEFT-467 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docs — audit pass on docs/deployment/wasm.md for stale URLs and wasip1 references |
| WEFT-468 | Todo | low | 0.9.x | ws14-deployment | A | strong | — | — | ws14: docs — fix Fumadocs link drift for docs/deployment/*.md (move into docs/src or delet |
| WEFT-469 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docker — verify or remove crates/clawft-kernel/Dockerfile.alpine |
| WEFT-470 | Done | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: docs — fix stale 0.3.1 example block in ADR-037 |
| WEFT-471 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: governance — adopt release-plz/git-cliff or amend ADR-002 to record current flow |
| WEFT-472 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: planning — reconcile Element 10 tracker (ClawHub features tangentially deployment) |
| WEFT-473 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deps — add quarterly dependency-sweep cadence (post-wasmtime-v33) |
| WEFT-474 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: deploy — confirm and document assess.weavelogic.ai deploy origin |
| WEFT-475 | Todo | low | 0.9.x | ws14-deployment | A | strong | — | — | ws14: homebrew — decide bottle vs source-build formula for weft-cli |
| WEFT-476 | Todo | low | 0.8.x | ws14-deployment | A | strong | — | — | ws14: ci — add wasm32-wasip2 build to release.yml or cargo-dist when supported |
| WEFT-477 | Todo | low | 0.9.x | ws14-deployment | A | strong | — | — | ws14: closure-sdk — re-check release-engineering implications when bridge work is proposed |
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
| WEFT-495 | Todo | medium | 0.8.x | ws15-mcp | G | strong | — | — | ws15: WASM panel auth — token/capability model for webview proxy |
| WEFT-496 | Done | medium | 0.8.x | ws15-mcp | J | strong | — | — | ws15: webview vs daemon allowlist — substrate.publish gating semantics |
| WEFT-497 | Todo | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: agent-core-chat feature flag — schedule removal post-D3 soak |
| WEFT-498 | Done | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: AgentChatParams/Result wire types — relocate to clawft-types |
| WEFT-499 | Todo | low | 0.8.x | ws15-mcp | A | strong | — | — | ws15: weft-gui-egui native bin — promote to scripts/build.sh native --gui + release artifa |
| WEFT-500 | Todo | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: MCP HTTP transport — verify against real HTTP server (not just mock) |
| WEFT-501 | Cancelled | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: TODO(agent-core-v1.1) — replace soul_cmd direct call with chain.append RPC |
| WEFT-502 | Done | urgent | 0.9.x | ws17-research | I | strong | — | — | ws17: Democritus — verify idle-graph gate keeps net_change suppression on real daemon (pos |
| WEFT-503 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — wire boot_ecc() runtime function into Kernel<P> boot sequence |
| WEFT-504 | Todo | medium | 0.9.x | ws17-research | G | strong | — | — | ws17: ECC — verify ecc feature exclusion on wasm32-unknown-unknown |
| WEFT-505 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — add governance gates to auth_service rotate_credential, request_token, revoke_ |
| WEFT-506 | Todo | medium | 0.9.x | ws17-research | C | strong | — | — | ws17: governance — make EffectVector explicit on auth/config/a2a/cron gates |
| WEFT-507 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: weave — implement weaver ecc CLI subcommands |
| WEFT-508 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — define new RVF segment types for ECC structures and persistence |
| WEFT-509 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — resolve 5 pre-existing clippy warnings in agent_loop, chain, gate |
| WEFT-510 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — incremental component-count maintenance for O(1) coherence feature extraction |
| WEFT-511 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — Iteration 1 end-to-end coordinate-descent loop for Q/K/V models |
| WEFT-512 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — drive top 5 eml-synergy-scan rows from scan to implementation |
| WEFT-513 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — RoMem phase-rotation temporal KG on CausalGraph |
| WEFT-514 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — GraphRAG community summaries in pipeline analyze |
| WEFT-515 | Done | high | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — CausalRAG causal_trace() over typed edges |
| WEFT-516 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — SASE clustering replacing label-propagation in cluster.rs |
| WEFT-517 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — LightRAG dual-level keyword retrieval |
| WEFT-518 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: KG — process remaining Phase 2 papers into priority list |
| WEFT-519 | Todo | high | 0.9.x | ws17-research | I | strong | — | WEFT-520 | ws17: LeWM — codify ADR-058 decoupling-invariant runtime checks |
| WEFT-520 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-519,WEFT-543 | WEFT-521 | ws17: LeWM — create weftos-worldmodel-core crate (no_std traits) |
| WEFT-521 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-520 | WEFT-522 | ws17: LeWM — create weftos-worldmodel-impls crate (candle ViT-tiny + AdaLN) |
| WEFT-522 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-521 | WEFT-523,WEFT-524,WEFT-525 | ws17: LeWM — create weftos-worldmodel facade crate |
| WEFT-523 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-522 | WEFT-526 | ws17: LeWM — create weftos-sensor-pipeline + -wire crates |
| WEFT-524 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-522 | — | ws17: LeWM — create clawft-worldmodel-service binary (3 deployment topologies) |
| WEFT-525 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-522 | — | ws17: LeWM — create clawft-delegation crate |
| WEFT-526 | Todo | high | 0.9.x | ws17-research | B | strong | WEFT-523 | — | ws17: LeWM — add mesh.sensor.v1.{encoded,consensus,control} topics on mesh wire |
| WEFT-527 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-522 | WEFT-528,WEFT-529 | ws17: LeWM — implement LatticeApi (7 methods) via ServiceApi |
| WEFT-528 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-527 | WEFT-530 | ws17: LeWM — wire SIGReg sigreg_health Welford monitor + auto-rollback at 0.85/30s |
| WEFT-529 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-527 | WEFT-530 | ws17: LeWM — implement pred_φ predictor + LatentPlanner (CEM default) |
| WEFT-530 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-528,WEFT-529 | — | ws17: LeWM — implement four-condition AND rollback gate |
| WEFT-531 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: LeWM — implement two training surfaces (offline edge + online streaming-merge) |
| WEFT-532 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: LeWM — per-sensor-class trainable RVF-hosted small models with hot-swap |
| WEFT-533 | Todo | high | 0.9.x | ws17-research | I | strong | WEFT-522 | — | ws17: LeWM — ExoChain attestation of (a_t, z_t, z_{t+1}, surprise) tuples |
| WEFT-534 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: LeWM — land /lewm-worldmodel-rs marketing page after visual confirmation |
| WEFT-535 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: sonobuoy — scaffold clawft-sonobuoy-ranging crate (G1 follow-up) |
| WEFT-536 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: sonobuoy — drive G2-G5 to closure or accept as deferred |
| WEFT-537 | Done | low | 1.0.x | ws17-research | I | strong | — | — | ws17: quantum — implement Pasqal backend skeleton |
| WEFT-538 | Todo | low | 1.0.x | ws17-research | I | strong | — | — | ws17: quantum — scaffold cuDensityMat SimulatorBackend behind quantum-nvidia feature flag |
| WEFT-539 | Todo | low | 1.0.x | ws17-research | I | strong | — | — | ws17: gaming-robotics — kick off first symposium experiment |
| WEFT-540 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: docs — cross-link orphan symposium output (compositional-ui, RLM 2512.24601) or clos |
| WEFT-541 | Done | low | 0.8.x | ws17-research | I | strong | — | — | ws17: docs — decide on single research → feature pipeline index vs ADR-only |
| WEFT-542 | Done | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: ECC — decide boot_ecc() fold-vs-fork into Kernel<P> |
| WEFT-543 | Todo | high | 0.9.x | ws17-research | I | strong | — | WEFT-520 | ws17: LeWM — decide 192-dim SIGReg latent dimensionality (ADR-050) |
| WEFT-544 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: governance — decide rotate-but-not-revoke policy expression for auth_service |
| WEFT-545 | Done | low | 0.9.x | ws17-research | I | strong | — | — | ws17: sonobuoy — decide whether 5th branch (active-imaging / SAS) lands as feature or stay |
| WEFT-546 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: Democritus — add rate limiting on exposure surface |
| WEFT-547 | Todo | medium | 0.9.x | ws17-research | I | strong | — | — | ws17: governance — close out 8-agent / 48-task exochain-fix-plan medium-severity rows |
| WEFT-548 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: EML — numerical-stability scaffolding for nested exp/ln at scale |
| WEFT-549 | Todo | low | 0.9.x | ws17-research | I | strong | — | — | ws17: orphans — triage OpenFang gap targets (channel breadth, Hands, Tauri, security stack |
| WEFT-550 | Done | medium | 0.7.x | ws14-deployment | A | strong | — | — | ws14: ci — replace smoke-test sleep+docker-ps with HTTP health probe |
| WEFT-551 | Done | high | 0.8.x | ws02-kernel | A | strong | — | — | ws02: deps — bump wasmtime 33 → 43 to clear 14 RUSTSEC advisories |
| WEFT-552 | Done | high | 0.8.x | ws02-kernel | B | strong | — | — | ws02: deps — bump rustls-webpki via rustls/reqwest/quinn alignment |
| WEFT-553 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: deps — replace unmaintained crates and unsound rand for cargo-audit cleanup |
| WEFT-554 | Todo | high | 0.9.x | ws02-kernel | B | ok | WEFT-130,WEFT-131 | WEFT-130 | ws02: exo-resource-tree — full K1 ACL engine (Did principals, delegation, exo_consent) |
| WEFT-555 | Done | high | 0.7.x | ws10-voice | E | strong | — | WEFT-207 | ws10: voice — wire substrate STT output into agent conversation + command input |
| WEFT-556 | Cancelled | high | 0.7.x | ws10-voice | E | weak | — | — | ws10: SC-10 plugin voice capability — gate WASM plugins on voice capability + sub-perms |
| WEFT-557 | Cancelled | high | 0.7.x | ws10-voice | E | ok | — | — | ws10: SC-4 voice permission flags — gate voice-triggered tool execution by Level 0/1/2 |
| WEFT-558 | Done | medium | 0.8.x | ws15-mcp | J | ok | WEFT-486 | WEFT-486 | ws15: VSCode panel E2E — chip-icon DOM assertion (followup to WEFT-486) |
| WEFT-559 | Backlog | high | 0.9.x | ws15-mcp | J | weak | — | WEFT-483 | ws15: Windows named-pipe transport — implement DaemonClient + daemon listener for x86_64-p |
| WEFT-560 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | WEFT-311 | ws09: pwa — push notifications via VAPID + WS event bridge |
| WEFT-561 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | WEFT-315,WEFT-575 | ws09: ui — axe-core + Playwright a11y suite across all routes |
| WEFT-562 | Cancelled | low | — | ws16-browser-wasm | G | strong | WEFT-409 | WEFT-409 | ws16: sparc(BW5) — retire scripts/check-features.sh references missed by WEFT-409 sweep |
| WEFT-563 | Backlog | low | 0.8.x | ws16-browser-wasm | G | strong | WEFT-409 | WEFT-409 | ws16: sparc(BW5) — retire scripts/check-features.sh references missed by WEFT-409 sweep |
| WEFT-564 | Backlog | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: scripts — actually retire or annotate scripts/check-features.sh (still on disk) |
| WEFT-565 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: api — TopicBroadcaster topics map leaks empty topic Senders |
| WEFT-566 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: docs — document save_config hot-reload semantics |
| WEFT-567 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: ui — /tools route does not call BackendAdapter.getToolSchema for WASM mode |
| WEFT-568 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: ui — Cmd+K palette index missing agents/sessions/tools/skills/channels + focus trap |
| WEFT-569 | Done | high | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — switch ?token= to #token= URL fragment to prevent log leak |
| WEFT-570 | Done | high | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: auth — logout() must invoke server-side token revoke |
| WEFT-571 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: browser-config — validate customBaseUrl is HTTPS in production (mirror WEFT-310) |
| WEFT-572 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: pwa — replace placeholder vite.svg icon with real 192/512 PNGs and maskable |
| WEFT-573 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: pwa — render an offline banner when SW serves the cached shell |
| WEFT-574 | Backlog | medium | 1.0.x | ws09-clawft-dashboard | F | strong | — | — | ws09: tauri — desktop shell functional features (tray, hotkey, side-car, Spotlight, notifi |
| WEFT-575 | Backlog | low | 1.0.x | ws09-clawft-dashboard | F | strong | WEFT-561 | — | ws09: ui — axe-core runtime a11y scan still missing (WEFT-315 AC unmet, follow-up to WEFT- |
| WEFT-576 | Done | high | 1.0.x | ws09-clawft-dashboard | A | strong | — | — | ws09: deploy — Dockerfile must run as non-root user (security hardening) |
| WEFT-577 | Todo | medium | 0.8.x | ws08-weftos-gui | F | strong | — | — | ws08: vscode panel wasm bundle — trim back toward 4500/1500 KB ceiling |
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
| WEFT-592 | Backlog | low | 0.8.x | ws02-kernel | B | ok | — | — | BVH spatial-temporal index — review plan and decompose into phase work items |
| WEFT-593 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: cargo-dist stopped publishing platform binaries (empty plan matrix) |
| WEFT-594 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: release Docker image strategy — download-coupling vs self-contained multi-arch |
| WEFT-595 | Done | high | 0.8.x | ws18-firmware | F | strong | — | — | ws08: leaf-display residual visual gap — single-buffer disambiguation (BUG-1) |
| WEFT-596 | Done | high | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: ADR-057 substrate per-path read ACLs — implement (0.8.x mesh blocker) |
| WEFT-597 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: daemon tracing→ChainManager bridge — 12 ExoChain events bypass the chain |
| WEFT-598 | Todo | medium | 0.8.x | ws09-clawft-dashboard | F | strong | — | — | ws09: Dependabot — triage 142 npm-side vulnerabilities (5 critical/41 high) |
| WEFT-599 | Todo | low | 0.8.x | ws16-browser-wasm | G | strong | — | — | ws16: relax transitive wasm-bindgen =0.2.108 exact pin |
| WEFT-600 | Done | high | 0.8.x | ws14-deployment | A | weak | — | WEFT-680 | ws14: workspace reqwest rustls-tls — fix static musl release build |
| WEFT-601 | Done | medium | 0.8.x | ws01-core | B | weak | — | — | ws01: adopt cargo-nextest + fix 6 test/latent-bug flakes (gate 12/12 green) |
| WEFT-602 | Done | none | 0.8.x | ws14-deployment | A | weak | — | — | ws14: release v0.6.20 — 0.6 rollup (63 assets: binaries + WASM + KB) |
| WEFT-603 | Done | high | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | weft agent -m hangs forever after a failed turn (provider error / max-iterations) |
| WEFT-604 | Backlog | medium | 0.8.x | ws01-core | E | strong | — | — | Unify local-LLM endpoint/model config — one source of truth for daemon + weft agent + voic |
| WEFT-605 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | exec_shell security allowlist is invisible to the model — denial spiral burns max tool ite |
| WEFT-606 | Done | medium | 0.8.x | ws10-voice | E | ok | — | — | Voice Talk-Mode turns are not anchored to the witness chain (standalone weft voice talk) |
| WEFT-607 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | agent.turn.record RPC — voice Talk-Mode turns anchored via the existing sink+anchor path |
| WEFT-608 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Kokoro TTS spoke garbled non-English — char-level tokenization vs IPA phoneme table |
| WEFT-609 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Talk-Mode deaf in a loud room — fixed -45 dBFS VAD gate vs -37 dBFS room tone |
| WEFT-610 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Talk-Mode said only 'One sec' — silent slow TTS + self-barge-in + premature capture resume |
| WEFT-611 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: turn-taking knobs + spoken self-enrollment + persistent speaker registry |
| WEFT-612 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: Orpheus prompt + sampling (was: slow tier zero audio) |
| WEFT-613 | Backlog | medium | 0.8.x | ws10-voice | E | weak | — | — | Voicelab parity: Chatterbox cloned-voice fast tier (native port) |
| WEFT-614 | Done | medium | 0.9.x | ws10-voice | E | weak | — | — | Voicelab parity: grounded agent LLM (web_search / tool-calling) in the voice loop |
| WEFT-615 | In Progress | none | 0.8.x | ws10-voice | E | strong | WEFT-628 | WEFT-638 | ws10: Re-enable barge-in — reframed as ERL-confidence-floor decision (ADR-068 D1) |
| WEFT-616 | Done | none | 0.8.x | ws06-memory | C | strong | — | WEFT-652 | ws06: Prototype agenticow COW memory checkpointing in the hermes loop |
| WEFT-617 | Todo | none | 0.8.x | ws10-voice | E | strong | — | — | ws10: Evaluate midstream for voice/ECC mid-stream gating (50ms CognitiveTick) |
| WEFT-618 | Done | none | 0.8.x | ws05-channels | H | weak | — | — | ws13: substrate/channels ADR set informed by AgentBBS (patterns only — FSL, no code reuse) |
| WEFT-619 | Todo | none | 0.9.x | ws13-app-substrate | B | strong | — | — | ws13: K6 — vendor exo-core (BLAKE3+HLC) + exo-dag (DagStore/MMR/SMT, no postgres) per ADR- |
| WEFT-620 | Done | none | 0.8.x | ws17-research | I | weak | — | — | ws17: Integrate ruvnet-brain into ruv-researcher agent + .planning/ruv/ |
| WEFT-621 | Backlog | none | 0.8.x | ws13-app-substrate | B | strong | — | — | ws13: Clear FSL licensing question for any AgentBBS / late.sh source reuse |
| WEFT-622 | Done | none | 0.8.x | ws01-core | B | weak | — | — | M2: one conversation engine — text commits Frontier→Committed on a shared forest |
| WEFT-623 | Done | none | 0.8.x | ws06-memory | C | weak | — | — | M3: store collapse — one store |
| WEFT-624 | Done | none | 0.8.x | ws07-multi-agent | D | weak | — | — | M4: agent-initiated work — subagent spawn tools + governance |
| WEFT-625 | Done | none | 0.8.x | ws11-agent-core-v1 | D | weak | — | — | ADR-067 + ADR-068 authored; P0 scaffolds (duplex Phase 0 + conversation.graph RPC) |
| WEFT-626 | Done | none | 0.8.x | ws03-pipeline | B | weak | — | — | Classification Phase A — turn classification pipeline (Done) |
| WEFT-627 | Done | none | 0.8.x | ws03-pipeline | B | strong | — | — | Classification Phase B — B1/B2/B3 landed (Done) |
| WEFT-628 | Todo | none | 0.8.x | ws10-voice | E | strong | WEFT-649,WEFT-650 | WEFT-615,WEFT-646,WEFT-647 | ADR-068 Phase 1 — desktop thin edge over localhost + ERL-into-compute_urgency |
| WEFT-629 | Todo | none | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ADR-067 P1-graph — causal.node.state chain event + fold replay |
| WEFT-630 | Todo | none | 0.8.x | ws08-weftos-gui | F | strong | — | — | ADR-067 G1-G5 GUI phases — umbrella |
| WEFT-631 | Backlog | none | 0.8.x | ws07-multi-agent | D | strong | — | — | Per-child CostBudget enforcement (budget hint threaded, not enforced) |
| WEFT-632 | Done | none | 0.8.x | ws07-multi-agent | D | strong | — | — | M4 live-capture residual — force tool selection via tool_choice (optional) |
| WEFT-633 | Backlog | none | 0.9.x | ws07-multi-agent | D | strong | — | — | D6 approval-UX — spawn triggers in-conversation approval (Defer + grant); GA end-state |
| WEFT-634 | Backlog | none | 0.9.x | ws02-kernel | B | strong | — | — | Governance rules gain action/tool selectors (engine is pure magnitude today) |
| WEFT-635 | Backlog | none | 0.9.x | ws07-multi-agent | D | strong | — | — | Spawn-at-user-level permission story |
| WEFT-636 | Backlog | none | 0.9.x | ws02-kernel | B | strong | — | — | Per-child / per-user gate principals (attribution; control holds today) |
| WEFT-637 | Backlog | none | 0.9.x | ws11-agent-core-v1 | D | strong | — | — | Tools-as-nodes enrichment — deterministic spawn-edge rooting (M2 D3 seam) |
| WEFT-638 | Backlog | none | 0.9.x | ws10-voice | E | strong | WEFT-615 | — | Voice cutover eventually retires TalkForest (ADR-068) |
| WEFT-639 | Todo | none | 0.8.x | ws15-mcp | J | strong | — | — | plane.sh wrapper fixes: WEFT-N resolution, real assignee lookup, cycle-membership via issu |
| WEFT-640 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | Real embedder: e5-small-v2 + record verbalization (replace SimHash placeholder) |
| WEFT-641 | Done | high | 0.8.x | ws06-memory | C | ok | — | — | AtomRegistry + atom.locate resolver + cross-index consistency audit (ADR-069 Panopticon) |
| WEFT-642 | Done | high | 0.8.x | ws06-memory | C | weak | — | — | ECC brain HNSW cannot join back to the atom spine (chain_seq hardcoded 0) |
| WEFT-643 | Done | high | 0.8.x | ws14-deployment | A | ok | — | — | Installer/version DX: build.sh install + SHA-stamped binaries + CLI-daemon mismatch warnin |
| WEFT-644 | Todo | medium | 0.8.x | ws10-voice | E | weak | — | — | SileroVoiceness: neural VAD behind the Voiceness trait (model staging + stateful ONNX + fa |
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
| WEFT-657 | Todo | low | 0.9.x | ws10-voice | E | strong | — | — | ws10: voice — pocket-tts watch: adopt as fast-tier engine when official ONNX/Candle export |
| WEFT-658 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: ack variance + contextual 'what I'm looking at' filler (talk-mode UX, hot-mic feedb |
| WEFT-659 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: unclear-input gate — consume STT confidence/SNR/paralinguistics before engaging the |
| WEFT-660 | Done | high | 0.8.x | ws12-knowledge-graph | C | weak | — | — | vector: real DiskAnnBackend::search hardcodes SearchResult.id=0 for every hit |
| WEFT-661 | Done | high | 0.8.x | ws12-knowledge-graph | C | weak | — | — | vector: HybridBackend merges cosine (hot) and sqeuclidean (cold) raw distances — recall 0. |
| WEFT-662 | Todo | medium | 0.8.x | ws06-memory | C | ok | — | — | upstream rvf-runtime 0.2: report 3 bugs (macOS __errno_location link failure; open() reset |
| WEFT-663 | Done | medium | 0.8.x | ws16-browser-wasm | G | ok | — | WEFT-672 | clawft-core browser target: 10 Send-future errors in agent/local_file_sink.rs (pre-existin |
| WEFT-664 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice: replace spoken ack/filler with light cue tones |
| WEFT-665 | Done | high | 0.8.x | ws06-memory | C | weak | — | WEFT-651 | memory: graft debris poisoning MEMORY.md + contentless graft rendering |
| WEFT-666 | Done | high | 0.8.x | ws10-voice | E | weak | — | — | voice watch: decision trace (gates, router, model, timings, reasoning) |
| WEFT-667 | Done | medium | 0.8.x | ws18-firmware | F | strong | — | — | ws13: edge-pad firmware — tilde-pin esp-hal/esp-radio (unstable feature + caret pin is a l |
| WEFT-668 | Done | low | 0.8.x | ws18-firmware | F | strong | — | — | ws13: edge-pad firmware — set-wise esp-* version bump (all one minor behind; esp-radio 0.1 |
| WEFT-669 | Done | high | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — AgentDB store split left 188 legacy entries stranded (clawft-knowledge, ruv |
| WEFT-670 | Todo | low | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — memory_import drops the tags column (128 legacy entries migrated without th |
| WEFT-671 | Done | medium | 0.8.x | ws10-voice | E | strong | — | WEFT-214 | ws10: voice — decide the disposition of clawft-plugin/src/voice (blocks 12 audit-era items |
| WEFT-672 | Done | high | 0.8.x | ws16-browser-wasm | D | strong | WEFT-663 | — | ws16: browser target — clawft_llm::hermes::strip_think called ungated from pipeline/transp |
| WEFT-673 | Todo | medium | 0.8.x | ws11-agent-core-v1 | D | strong | WEFT-655 | — | ws11: hermes loop — voice-review-gate residual gaps self-documented by WEFT-655 (forest-co |
| WEFT-674 | Done | high | 0.8.x | ws14-deployment | A | strong | — | — | ws14: CI — pr-gates.yml is PR-triggered only, so hard gates never run on feature branches  |
| WEFT-675 | Done | medium | 0.8.x | ws08-weftos-gui | C | strong | — | — | ws08/ws18: leaf display + ESP32-S3 firmware rewrite — vector-first scene pipeline, 7 new c |
| WEFT-676 | Done | medium | 0.8.x | ws06-memory | C | strong | — | — | ws06/ws11: ADR-058/059 memory + context tier — Qwen3 ONNX embedder, L2 SessionTier, graft/ |
| WEFT-677 | Done | medium | 0.8.x | ws11-agent-core-v1 | D | strong | — | — | ws11: ADR-060 Track A — LocalProvider Hermes serving provider, tool_call round-trip + thin |
| WEFT-678 | Done | medium | 0.8.x | ws10-voice | C | strong | — | — | ws10: ADR-061 Track D voice-front initial build — native AEC, ECAPA embedder, dual-layer T |
| WEFT-679 | Done | medium | 0.8.x | ws14-deployment | A | strong | — | — | ws14: dependency-advisory patch round — quinn-proto/memmap2/rkyv CVEs patched, wasmtime DE |
| WEFT-680 | Done | medium | 0.8.x | ws14-deployment | A | strong | WEFT-600 | — | ws14: Docker/release hardening — v0.6.20 cut, v0.6.21 cut-then-reverted, Alpine + non-root |
| WEFT-681 | Done | medium | 0.8.x | ws14-deployment | A | strong | — | — | ws14: security — wasmtime advisory deferred during the 2026-06-28 patch round and never tr |
| WEFT-682 | Todo | low | 0.8.x | ws15-mcp | J | strong | — | — | ws15: tracker — enforce the two-label rule at item creation (36 items unlabeled, all post- |
| WEFT-683 | Done | medium | 0.8.x | ws02-kernel | B | strong | — | — | ws02: mesh — ADR-031 drift: RVF encoding declared the production default but only JSON was |
| WEFT-684 | Done | high | 0.8.x | ws06-memory | C | strong | — | — | ws06: memory — MCP server runs 'npx ruflo@latest', so a schema-bearing dep is unpinned (ro |

---

## Open tickets by workstream

### ws01-core (8 open)

#### WEFT-11 — ws01: rpc — implement Windows daemon transport (named pipes) for DaemonClient

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Non-Unix DaemonClient::connect() returns None. Comment says "Windows named-pipe transport is planned for v0.2"; we are at 0.6.19 — slipped 4+ minor versions. Windows users cannot run the daemon at all.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-13 — ws01: platform — implement OPFS-backed BrowserFileSystem persistence

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: BrowserFileSystem is currently in-memory HashMap-backed. PWA users lose all state on reload. Comment says "acceptable for the current stub/MVP phase".
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-604 — Unify local-LLM endpoint/model config — one source of truth for daemon + weft agent + voice

- **State**: Backlog · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws01-core, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Problem There is no single source of truth for "which local LLM endpoint + model to use" — three consumers use three different selection mechanisms (full map in docs/handoff-local-llm-config.md): 1. Daemon → [kernel.llm] (service_url/model) → clawft-service-llm::LlmClient (env LL
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-14 — ws01: platform — land OPFS-or-equivalent BrowserEnvironment persistence

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: BrowserEnvironment is in-memory only. Same UX consequence as the FS issue: env variables (e.g. API keys) are lost on reload.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-15 — ws01: kernel-config — wire LogQuantizedStubConfig + SimdDistanceStubConfig runtime

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Two *StubConfig types are serializable stubs in clawft-types; their runtime backends sit in clawft-kernel and require ruvector-core PR #352. Foundation surface ready; runtime missing.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-18 — ws01: foundation — run ADR-010 v0.3 cancel-correctness audit on mesh select! branches

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, tech-debt, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-010 calls for a v0.3 audit of select! branches in mesh networking code (foundation runtime decision). We are at 0.6.19 with no audit completed. Mesh code lives in stream 04 (mesh) but the ADR governs the foundation runtime.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-20 — ws01: types — decide deny_unknown_fields lint mode for Config

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Config accepts both camelCase and snake_case via #[serde(alias)] and silently ignores unknown fields. Forward-compat is good; typo-resistance is bad. No off-by-default lint mode for typo detection.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-26 — ws01: types — clean up panic! macros in test-only canvas/provider/agent_bus arms

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws01-core, audit-finding, audit-0.7.0, tech-debt, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: panic! macros live inside #[cfg(test)] match arms. Harmless (cfg-gated) but stylistically loud. Prefer unreachable! with rationale or proper assert! macros.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws02-kernel (25 open)

#### WEFT-105 — ws02: mesh — implement K6.4 chain replay (LocalChain::tail_from + append_signed)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W1 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, orphan
- **Blocked by**: WEFT-109, WEFT-112, WEFT-115
- **Blocks**: none
- **Gap**: K6.4 chain replay is half-built: build_chain_sync_request and handle_chain_sync_response exist, but no replay into ChainManager. Cross-node chain sync is therefore non-functional.
- **Plan**: Wait for WEFT-109, WEFT-112, WEFT-115

#### WEFT-106 — ws02: mesh — implement K6.4 tree Merkle diff + signed mutations

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W1 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: WEFT-115, WEFT-144
- **Blocks**: none
- **Gap**: Resource tree has no Merkle proof generation, no diff API, and MutationEvent.signature is always None. Cross-node tree sync cannot be authenticated or transmitted incrementally.
- **Plan**: Wait for WEFT-115, WEFT-144

#### WEFT-107 — ws02: mesh — implement S10 key-rotation chain event + verifier

- **State**: Todo · **Priority**: high · **Cycle**: 0.8.x · **Wave**: W1 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: WEFT-113
- **Blocks**: none
- **Gap**: S10 key rotation has no owner in the K6 plan. Without it, a compromised node key requires cluster-wide manual intervention. The security panel defines a 5-step dual-signed-chain-event protocol but nothing implements it.
- **Plan**: Wait for WEFT-113; 0.8.x gate — early wave

#### WEFT-109 — ws02: mesh — decide chain merge strategy (leader vs DAG) and split-brain handling

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: WEFT-105
- **Gap**: Q1 (chain merge: leader vs DAG) and Q5 (split-brain handling) are open design questions blocking K6.4 implementation. Without a decision, chain replay (row #16) cannot land cleanly.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-112 — ws02: mesh — add InMemoryTransport / MockPeer / MockClock / FaultyTransport

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, tests
- **Blocked by**: none
- **Blocks**: WEFT-105
- **Gap**: No multi-node integration test harness exists. K6 mesh code is ~3,500 lines with 133 unit tests but zero two-node integration tests. The recommended fixtures (InMemoryTransport, MockPeer, MockClock, FaultyTransport) belong in crates/clawft-kernel/src/mesh_test_support.rs.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-113 — ws02: mesh — define Clock trait and inject into time-dependent components

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, tests
- **Blocked by**: none
- **Blocks**: WEFT-107
- **Gap**: Mesh time-dependent components (heartbeat, SWIM, retries) hardwire std::time::Instant / tokio::time, blocking deterministic tests. Need a Clock trait so MockClock (row #23) can drive them.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-115 — ws02: mesh — define missing K6 protocol struct types and msg_type enum

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: WEFT-105, WEFT-106
- **Gap**: Multiple protocol-message struct types are referenced but undefined: MeshStream, TransportListener, EncryptedPeer, WeftHandshake, JoinRequest/Response, TreeSyncRequest/Response, ServiceEndpoint, ProcessAdvertisement, ServiceAdvertisement, Frame, plus the full msg_type enumeration. Implementation cannot proceed without 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-118 — ws02: mesh — add QUIC transport (quinn + snow) alongside TCP/WS

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-026 makes QUIC the primary transport, but only TCP (and WS) are implemented. Without QUIC, mesh suffers head-of-line blocking and worse NAT traversal.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-131 — ws02: exo-resource-tree — implement DelegationCert lifecycle (grant/revoke + Ed25519 + expiry)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, stub, governance, security
- **Blocked by**: none
- **Blocks**: WEFT-554
- **Gap**: DelegationCert is a type stub with no lifecycle. K1 deliverables: grant/revoke with Ed25519 signatures, certificate chain validation, time-bounded expiry.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-142 — ws02: kernel — add NodeId composite for cross-node uniqueness + remote inbox bridge

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Pid: u64 has no node component, so PIDs collide across nodes. Inbox mpsc is in-process only; remote delivery has no bridge. Identity / addressing model is incomplete for any multi-node story.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-144 — ws02: mesh — add MutationEvent Ed25519 signing for cross-node tree mutations

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: WEFT-106
- **Gap**: MutationEvent.signature is always None. Cross-node tree mutations cannot be authenticated. Pairs with row #17 (tree Merkle diff).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-146 — ws02: mesh — replace static GovernanceRule Vec with cluster-wide distribution

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Vec<GovernanceRule> is static and per-node. Cross-node governance requires rule distribution + escalation, otherwise nodes drift on policy.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-147 — ws02: mesh — cross-node capability-claim verification (signed advertisement)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: PeerNode.capabilities strings are unvalidated. Any peer can advertise any capability. Cross-node capability advertisement must be signed and verifiable.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-554 — ws02: exo-resource-tree — full K1 ACL engine (Did principals, delegation, exo_consent)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W1 · **Lane**: B · **AC**: ok
- **Labels**: ws02-kernel, gap, security
- **Blocked by**: WEFT-130, WEFT-131
- **Blocks**: WEFT-130
- **Gap**: Follow-up to WEFT-130 scaffold (commit 1fbe0215). The 0.7.x scaffold ships: Principal(String), AclPolicy/Effect, EffectiveAclCache (bulk-clear) CapabilityChecker tree-walk with default-deny root, first-match wins Hash on Action/Role for cache keying Tests: root-deny, principal-gr
- **Plan**: Wait for WEFT-131

#### WEFT-111 — ws02: mesh — decide full libp2p-kad vs lighter DHT

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Q4 is unresolved: stick with full libp2p-kad or build a lighter DHT? This is research / experimentation territory and depends on field telemetry once K6 ships at all.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-128 — ws02: vector — flip LogQuantized + SimdDistance is_available once ruvector-core PR #352 lands

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: LogQuantizedConfig::is_available() and SimdDistanceConfig::is_available() are hardwired false pending ruvector-core PR #352. Once that lands, flip the constants and the +14% QPS branch-free distance path opens.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-135 — ws02: workspace — clean ~150 clippy errors (pre-existing debt)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: scripts/build.sh check is green but scripts/build.sh clippy is red on ~150 pre-existing errors across clawft-types/src/goal.rs, clawft-rpc, eml-core, and older kernel/weave code. Blocks any clippy-as-gate aspiration.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-123 — ws02: services-api — add HTTP facade integration tests once profile/pairing types land

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: HTTP facade integration tests are blocked on ProfilesConfig / PairingConfig types landing in clawft-types. Once those land, the integration coverage should follow.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-125 — ws02: vector — add ecc.vector-config RPC endpoint

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No RPC exposes the active vector backend, so operators have no introspection. A small ecc.vector-config RPC closes that gap.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-129 — ws02: kernel — ship real Wasmtime backend for spectral_embedding (or move to deferred)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, stub, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: spectral_embedding is a TODO. Either ship a real Wasmtime backend or move it to the deferred-feature appendix and remove from the production surface.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-152 — ws02: tests — confirm cognitum-gate-tilezero Permit/Defer/Deny path is exercised

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, governance, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: cognitum-gate-tilezero (TileZeroGate) Permit/Defer/Deny three-way cryptographic-receipt path has no documented tests. May or may not be exercised in production.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-153 — ws02: chain — add EVENT_KIND_* constants for minor non-kernel chain gaps

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Four minor non-kernel sites do not emit chain events: ToolRegistry::register_with_metadata, sandbox::check_tool\|network\|file_read\|file_write (only audit-log), and clawft-graphify::ingest::save_query_result.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-592 — BVH spatial-temporal index — review plan and decompose into phase work items

- **State**: Backlog · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: ok
- **Labels**: ws02-kernel, ws17-research, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: What this ticket is The single re-entry point for the BVH-on-RVF spatial-temporal index work. At the start of cycle 0.8.x, an agent or maintainer claims this ticket, re-reads the ADR + plan below, and decomposes Phase A–E into individual work items (one per phase). This ticket th
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-634 — Governance rules gain action/tool selectors (engine is pure magnitude today)

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The governance engine today decides on pure magnitude — it cannot select on the specific action or tool. Add action/tool selectors so rules can target "the spawn tool" or "this action class" specifically. Prerequisite for approval-UX.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-636 — Per-child / per-user gate principals (attribution; control holds today)

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws02-kernel, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Governance control holds today, but gate verdicts are not attributed to a per-child / per-user principal. Add principals so verdicts carry attribution (who spawned, on whose authority).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws03-pipeline (16 open)

#### WEFT-45 — ws03: routing — wire MicroLoraRouter (v3) once ruvllm-wasm lifts 11-pattern HNSW cap

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: v3 MicroLoraRouter is deferred until ruvllm-wasm lifts the documented 11-pattern HNSW cap. Plumbing in HybridRouter is there; the v3 implementation isn't.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-34 — ws03: routing — resolve CONS-002 (DashMap vs RwLock<HashMap> benchmark)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tech-debt, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: CostTracker and RateLimiter implemented as RwLock<HashMap> (no new dep). Performance trade-off vs DashMap was never benchmarked under contention. Entry remains OPEN pending real production data.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-36 — ws03: routing — resolve CONS-006 (config validation boundary)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Validation lives in routing_validation.rs but the boundary between deserialization-time rejection (serde) and post-load validation was never finalized. Some checks are duplicated; some are only in one place.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-39 — ws03: pipeline — persist RetryModel learned weights across daemon restarts

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: RetryModel trains from observed retry outcomes but the Default impl resets to untrained. Daemon restarts lose all learned retry curves. Persistence path unclear.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-40 — ws03: pipeline — surface routing-decision history via admin endpoint

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The panel surfaces current tier / budget remaining via weft status, but historical routing decision logs (per the security-review §2.3 audit recommendation) are not exposed. Operators have no way to retroactively review per-tier dispatch counts, fallback rates, or budget exhaustion events.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-41 — ws03: research — Iteration 3 EML attention multi-param coordinated perturbation

- **State**: Todo · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Iteration 3 gate: multi-param coordinated perturbation on SafeTree; target ≥80% MSE reduction at (seq_len=4, d_model=8) and final_mse < 5e-2. Not attempted yet. Iterations 4-5+ (full EML-Transformer, hybrid scoring) explicitly aspirational. No tracking issue or plan stub.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-43 — ws03: pipeline — decide consolidation of clawft-service-llm vs clawft-llm

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: clawft-service-llm and clawft-llm overlap. The lib.rs comments argue the split (daemon-narrow vs general provider abstraction), but as the daemon adds streaming / multi-provider features, the split rationale erodes. Two crates maintaining parallel HTTP clients invites drift.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-48 — ws03: rate-limiter — expose rate-limiter metrics via admin endpoint (Element-09)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The pending_count()-equivalent is implemented but not surfaced through any HTTP/RPC admin route.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-49 — ws03: rate-limiter — expose rate-limiter LRU maintenance via admin endpoint (Element-09)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The maintenance method (manual LRU flush) is wired but not exposed.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-51 — ws03: context-router — exhaustively test embedding-router cargo-feature-off path

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, release-gate-blocker, bug, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: When Config.routing.context_router is set to "embedding" but the embedding-router feature is not compiled in, the daemon's behavior is not exhaustively tested. index.rs already handles EmbeddingRouterError::EmptyRegistry, but other branches may panic or silently fall back.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-53 — ws03: pipeline — decide EML score-fusion in scope for 0.7.0 (FitnessScorer weights)

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: FitnessScorer weights remain literal constants (0.4/0.2/0.2/0.2) hand-tuned via FitnessScorerWeights::default(). The synergy scan flagged pipeline/scorer.rs as a candidate for EmlModel::new(2, 4, 1) learning per-task fitness. Work was not assigned to a sprint.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-54 — ws03: pipeline — review FitnessScorer.error_indicators allowlist (localization, jailbreak)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The FitnessScorer.error_indicators allowlist of refusal phrases ("I can't", "as an AI", etc.) is hand-curated. Localization, jailbreak resilience, and false-positive rate are unknown.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-55 — ws03: pipeline — verify experimental-attention CI build/test wiring

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tests, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The experimental-attention feature gate exists in eml-core and the benchmark example exists, but no GH actions step references it. CI may not catch regressions.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-56 — ws03: pipeline — define explicit pipeline-pass step in scripts/build.sh gate

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws03-pipeline, audit-finding, audit-0.7.0, tests, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The 11-check phase gate doesn't have an explicit pipeline-pass step. Pipeline-specific regression coverage relies on the workspace test suite generally.
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

### ws04-plugin-skills (3 open)

#### WEFT-70 — ws04: ux — add macOS-sandbox-downgrade warning to startup banner

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws04-plugin-skills, audit-finding, audit-0.7.0, release-gate-blocker, docs, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: effective_sandbox_type silently downgrades OsSandbox/Combined to Wasm on macOS, emitting tracing::warn!. Downstream operators on macOS may not know they're running in a weaker sandbox — no error, no opt-in confirmation.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-74 — ws04: skills — define pending-skill review timing (interactive prompt vs CLI)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws04-plugin-skills, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Should the .pending marker also trigger an interactive prompt at next agent-loop start, or only on demand via CLI? Today neither happens automatically. Without a defined trigger, pending skills accumulate unseen.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-76 — ws04: skills — add weft skills refresh CLI for headless/CI scenarios

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws04-plugin-skills, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Hot-reload covers FS changes via notify, but in-process discovery state has no manual invalidation surface. Headless / CI scenarios where the watcher is disabled have no fallback.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws05-channels (7 open)

#### WEFT-170 — ws05: PluginHost C7 unification — migrate Telegram/Discord/Slack to ChannelAdapter

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, gap, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The trait shim (ChannelAdapterShim) exists, but Telegram, Discord, Slack still implement the legacy Channel trait directly. Migrating them to ChannelAdapter is the C7 deliverable; the migration changes the cancellation contract (poll-based plugin token vs tokio_util::sync::CancellationToken) and the inbound payload typ
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-171 — ws05: Slash-command surface — decide consumer for ChannelHost::register_command

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, gap, orphan
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ChannelHost::register_command is part of the trait surface and is exercised by mock hosts in tests, but no in-tree channel actually calls it. Discord slash-commands, Telegram BotFather commands, and Slack slash commands are all unimplemented despite the trait surface. No tracker entry covers this.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-173 — ws05: Discord — document intents bitmask default and cover privileged-intent rejection

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, docs, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Factory default intents is 37377 (GUILDS \| GUILD_MESSAGES \| DIRECT_MESSAGES + a few). No doc points at the chosen bits, no test covers intents = 0 or privileged-intent rejection (MESSAGE_CONTENT, GUILD_MEMBERS).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-174 — ws05: Slack — add unknown_envelope counter for API drift detection

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The Slack envelope parser logs non-envelope message on parse failure and silently moves on. There is no metric or counter; a regression where every payload becomes "non-envelope" after a Slack API change would not be observable.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-175 — ws05: iMessage scope — implement AppleScript bridge or formally drop from tracker

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, orphan, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: crates/clawft-channels/src/imessage/ is referenced in 00-orchestrator.md and 04-element-06-tracker.md E4 description as paired with Signal, but the directory does not exist. iteration-1 review flagged it as "Not in orchestrator". Either silently dropped or deferred.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-176 — ws05: WeftOS white-label — add brand() accessor and remove hard-coded clawft strings

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Hard-coded "WeftOS" / "clawft" strings appear in Discord identify (browser: "clawft", device: "clawft"), CLI help, web UI header. White-label is P1 for Valtech, P2 generally. No brand() accessor exists yet.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-177 — ws05: Channel failover chain — decide semantics and either implement or close as out-of-scope

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: H · **AC**: strong
- **Labels**: ws05-channels, audit-finding, audit-0.7.0, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: MEMORY.md mentions "failover chain improvements". The only failover machinery in tree is in clawft-llm/src/failover.rs (provider failover), not channel failover. The PluginHost treats each channel independently. Open product question: per-message? per-session? cross-channel quorum?
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws06-memory (9 open)

#### WEFT-92 — ws06: identity — decide binding-thread-mismatch policy refuse vs annotate (MW-14)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, governance, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: "Hard refusal is a v1.1 follow-up." Binding-thread mismatch currently downgrades to an annotation + warn log, never refuses. Reviewer-flagged as a security posture decision.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-662 — upstream rvf-runtime 0.2: report 3 bugs (macOS __errno_location link failure; open() resets metric to L2; permanent delete bitmap)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: ok
- **Labels**: bug, ws06-memory
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Found during WEFT-616 Phases 0/1 (adc5f9bc, 2026-07-14), all worked around locally: (1) locking.rs hardcodes glibc __errno_location — binaries don't link on macOS; both clawft-core and clawft-cow-memory carry cfg(macos) shims forwarding to libc::__error() — consolidate to one sha
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-85 — ws06: substrate — emit chain_event! for session.append on every appended turn (MW-7)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: chain_event! for session.create fires only when neither cache nor disk has the session. There is no session.update event for appended turns, only on the file-not-existing branch of append_turn. A long-running session looks like one create event followed by a destroy. The hot loop activity is invisible to the chain.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-87 — ws06: sessions — ship weft session gc (or self-cleanup migration path) (MW-9)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: SessionManager migration path (old_filename with _ instead of :) reads the old file and writes the new one, but never removes the old file ("keep old for safety"). After several sessions this leaves orphaned files in sessions/.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-91 — ws06: identity — decide whether FileIdentityProvider needs notify watcher (MW-13)

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, tech-debt, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: "No hot-reload watcher — the cached FileIdentityProvider re-reads on every call (small files; cheap). A notify-driven watcher arrives when measurement says it earns its keep." The deferred-doc is open with no measurement.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-95 — ws06: identity — route IdentityLoader::current through Platform::fs() (MW-17)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: IdentityLoader::current uses std::fs::read_to_string directly instead of going through Platform::fs(). The only sync, platform-bypassing read in the agent identity path. Cannot be exercised in WASM the same way the rest of the agent loop is.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-96 — ws06: identity — define journal substrate read-on-every-turn path (WS-D1)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: "No SOUL.journal write path — F1 seeds the empty journal file and stamps the soul_journal derived-write grant; F2's weaver soul promote reads it, diffs, and applies on confirmation. The journal is not consulted on every-turn loads." Any consumer that needs a per-turn read will need to design that path (likely via the s
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-97 — ws06: identity — substrate-backed Identity::source variant set (WS-D4 / WS-D5)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Identity::source is &'static str placeholder; trait is forward-compatible. Error variants need to be added when substrate path lands (today only signals "files missing").
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-670 — ws06: memory — memory_import drops the tags column (128 legacy entries migrated without their 306 tags)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws06-memory, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The MCP memory_import tool does not carry the tags column. Verified empirically with a two-entry probe before the real migration: namespaces and values are preserved and entries are re-embedded correctly, but tags comes through empty, created_at is reset to import time, access_count resets to 0, and provenance_type is 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws07-multi-agent (12 open)

#### WEFT-193 — ws07: IDE provider — replace IdeToolProvider::stub() with real implementation

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: IdeToolProvider::stub() is the only constructor. Full IDE provider is not implemented. Adjacent to the multi-agent tool surface.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-194 — ws07: Hybrid context-router — wire MicroLoraRouter once agent-core-v1 phase E3+ is ready

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: // TODO(agent-core-v1 phase E3+): wire MicroLoraRouter (v3) once... Micro-lora hybrid context-routing remains unwired.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-195 — ws07: delegate_tool — drop hardcoded claude_available=true, query the delegator for liveness

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, bug, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: If the delegator is later non-functional (auth lapse, network, etc.) the tool will continue to advertise availability and force-route to it.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-196 — ws07: weft delegate — add debug subcommand to surface routing decisions

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not in the SPARC plan but a common ask: a CLI subcommand that exercises the engine to show which target a free-text task would route to.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-197 — ws07: weft doctor — add multi-agent checks (claude on PATH, auto-delegation, ≥1 route)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: weft doctor does not flag missing claude on PATH, auto-delegation enabled status, or whether at least one agent route is configured.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-198 — ws07: claude-flow MCP server — decide whether to add by default to [tools.mcp_servers]

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The skill manifest references claude-flow MCP tool prefixes, but dynamic discovery is M4 and claude-flow is not added to [tools.mcp_servers] by default. End users get prefixed tools that resolve to nothing.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-199 — ws07: SwarmCoordinator topology — implement mesh/hierarchical/adaptive or document as prompt-only

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: CLAUDE.md references mesh-coordinator, hierarchical-coordinator, adaptive-coordinator agents; in tree they exist only as claude-flow swarm prompts. SwarmCoordinator is a flat fan-out/collect with no topology axis.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-200 — ws07: notifications/tools/list_changed — handle inbound and advertise outbound

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not handled either direction today. The MCP bridge does not propagate notifications/tools/list_changed from external servers; weft mcp-server advertises tools.listChanged: false.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-201 — ws07: Auto-delegation classifier — improve regex+keyword accuracy or document fragility (3H MIN-02)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The regex + keyword classifier is fragile. No follow-up bead. False positives/negatives directly affect routing decisions.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-631 — Per-child CostBudget enforcement (budget hint threaded, not enforced)

- **State**: Backlog · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: A per-child cost budget hint is threaded through spawn today but not enforced. Add enforcement so a child agent that exceeds its CostBudget is halted/denied.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-633 — D6 approval-UX — spawn triggers in-conversation approval (Defer + grant); GA end-state

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: GA end-state: when an agent tries to spawn, surface an in-conversation approval — the turn Defers, the user grants, and the spawn proceeds. Requires governance action/tool selectors (separate prerequisite item).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-635 — Spawn-at-user-level permission story

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws07-multi-agent, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Define the permission story for spawning at the user level — how a user authorizes an agent to spawn children on their behalf, and how that authority is scoped/revoked.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws08-weftos-gui (14 open)

#### WEFT-254 — ws08: chat panel — multi-conversation sidebar UI

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Panel currently shows one conversation. No way to switch between concurrent threads or revisit history. Multi-tab terminal also foreshadowed only.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-256 — ws08: chat panel — model / provider switcher in chip strip

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No runtime way to choose model/provider from the panel. Users must edit configs out-of-band.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-258 — ws08: chat panel — real interactive defer (resume on { deferred: true })

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: When agent returns { deferred: true, reason } the panel should prompt the user and resume on response. Today it doesn't differentiate this control case.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-277 — ws08: composer — honest_affordances real GEPA / governance intersection

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, stub, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: honest_affordances is identity passthrough — "ADR-006 rule 2 TODO". GEPA-gated governance intersection deferred to M2 active-radar loop. The hook exists; the policy doesn't.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-577 — ws08: vscode panel wasm bundle — trim back toward 4500/1500 KB ceiling

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: : After the M7+M7b feature wave (chat panel markdown via egui_commonmark, terminal scrollback + glyph styling, canon Field::Date with jiff + Field::Code, Workshop Grid/Tabs layouts, three new viewers — HealthViewer/SensorViewer/sparkline — tree filters, breadcrumb navigation, ObjectType registrations for Mesh/Sensor/No
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-263 — ws08: terminal panel — multi-tab terminal (HashMap<SessionId, Terminal>)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Only one terminal session at a time. Structure foreshadowed (HashMap<SessionId, Terminal>) but unwired.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-264 — ws08: terminal panel — real WASM terminal renderer

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WASM terminal is a stub. VSCode/Cursor panel users get nothing. Real WASM renderer needs design re-think (alacritty doesn't compile to wasm32).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-275 — ws08: explorer — Lineage Object Type + viewer (metadata convention sign-off)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Lineage metadata placement is open: inline field vs sibling <derived-path>/meta/lineage path. Without sign-off, no Lineage Object Type, no lineage viewer.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-281 — ws08: graph viewer — editable Phase 3+ patch UI (egui_node_graph migration)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: GraphViewer (ui://graph) is read-only MVP (rolled-our-own painter). Editable graph is explicit stretch goal NOT in 0.7.0; deferred to "Phase 3+ patch UI". egui_node_graph adapter is the migration seam.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-282 — ws08: vscode panel — capture sidecar (mic/camera) for vscode#303293

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Webviews can't expose allow="microphone". No voice input in panel. Capture sidecar deferred to M2.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-283 — ws08: vscode panel — typed active-radar return schema (variant-id echo)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Webview posts only plain RPC-request / RPC-response messages — no typed active-radar return schema or variant-id echo. Future active-radar work needs typed envelope.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-284 — ws08: vscode panel — ThreadDock primitive for per-agent parallel output

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No ThreadDock primitive — multiple parallel agent outputs collapse together. Multi-agent swarm UX gap.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-285 — ws08: vscode panel — WSP-0.1 verb support (raw RPC only today)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Panel does not yet speak WSP-0.1 verbs; raw kernel.* / agent.chat RPC only. WSP verbs queued for M3.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-630 — ADR-067 G1-G5 GUI phases — umbrella

- **State**: Todo · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws08-weftos-gui, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Umbrella item for the ADR-067 GUI phases G1 through G5 (conversation-graph GUI surface). Decompose into per-phase work items as each is picked up.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws09-clawft-dashboard (17 open)

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

#### WEFT-565 — ws09: api — TopicBroadcaster topics map leaks empty topic Senders

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: crates/clawft-services/src/api/broadcaster.rs::TopicBroadcaster.topics is a HashMap that never evicts entries. When a ?topic= is subscribed, the broadcast::Sender stays in the map for the gateway's lifetime even when all subscribers drop. broadcast::Sender prunes dead receivers, but the topic slot itself leaks. Long-ru
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-567 — ws09: ui — /tools route does not call BackendAdapter.getToolSchema for WASM mode

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-307 shipped clawft-wasm::tool_schema(slug) and WasmAdapter.getToolSchema() correctly, but the /tools route (clawft-ui/src/routes/tools.tsx:70-72) consumes tool.schema straight off the response of api.tools.list — the Axum-only api-client.ts call. There is no fallback to useBackend()/getToolSchema() for WASM mode u
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-568 — ws09: ui — Cmd+K palette index missing agents/sessions/tools/skills/channels + focus trap

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-308 shipped a real fuzzy-finder palette, but the index in clawft-ui/src/components/layout/MainLayout.tsx:60-87 only contains nav routes plus two utility actions (toggle theme, toggle sidebar). The original AC required indexing of "routes, agents, sessions, tools, skills, channels". Agents / sessions / tools / skil
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-571 — ws09: browser-config — validate customBaseUrl is HTTPS in production (mirror WEFT-310)

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: clawft-ui/src/lib/url-validator.ts validates the cors_proxy URL for HTTPS-or-loopback. The "Custom OpenAI-compatible" provider's base URL field (browser-config.tsx:332-344::customBaseUrl) is not subjected to the same validation. A user can save an http://api.example.com/v1 base URL in production, defeating the spirit o
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-574 — ws09: tauri — desktop shell functional features (tray, hotkey, side-car, Spotlight, notifications, build.sh)

- **State**: Backlog · **Priority**: medium · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-313 was closed as "scaffold shipped", but none of the six functional ACs landed. clawft-ui/src-tauri/src/lib.rs:1-16 explicitly lists the gaps: System tray with agent-status colour states (not shipped). Global hotkey Cmd+Shift+W / Ctrl+Shift+W (not shipped). weft-gateway side-car launch on app start, terminate on 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-598 — ws09: Dependabot — triage 142 npm-side vulnerabilities (5 critical/41 high)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Dependabot reports 142 vulnerabilities (5 critical / 41 high) — heavily the npm side (root agentic-flow devdep + clawft-ui React/Vite). The cargo-audit gate (scripts/build.sh audit) only covers Rust deps; the npm surface is unaudited.
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

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: tests, ws09-clawft-dashboard
- **Blocked by**: none
- **Blocks**: WEFT-315, WEFT-575
- **Gap**: Followup from WEFT-315 (jsx-a11y static lint + bundle-size gate shipped). The full runtime a11y audit needs: Set up Playwright suite for clawft-ui (no test infra exists yet) Integrate @axe-core/playwright; visit each of the 14 routes (/, /agents, /canvas, /chat, /sessions, /too
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-566 — ws09: docs — document save_config hot-reload semantics

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-303's AC required: "Decide whether save also triggers a hot-reload signal to the running daemon or only takes effect on restart; document the choice." Today bridge.rs::save_config writes to disk and returns Ok; nothing in docs/ui/api-reference.md (or docs/ui/deployment.md) tells the operator whether weft daemon pi
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-572 — ws09: pwa — replace placeholder vite.svg icon with real 192/512 PNGs and maskable

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: clawft-ui/public/manifest.webmanifest ships with a single icons entry pointing at /vite.svg with sizes: "any". There is no 192x192, no 512x512, and no maskable icon variant. Lighthouse PWA audit penalises this and the AC's "Lighthouse PWA score > 90 in CI" is therefore unreachable today. iOS / Android home-screen insta
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-573 — ws09: pwa — render an offline banner when SW serves the cached shell

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W0 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: clawft-ui/public/sw.js correctly serves the cached /index.html shell when navigations fail offline, but the React app does not detect or display any offline state. The user lands on the cached shell and may not realise they are disconnected. WEFT-311's AC required "Offline reload renders the shell with a clear offline 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-575 — ws09: ui — axe-core runtime a11y scan still missing (WEFT-315 AC unmet, follow-up to WEFT-561)

- **State**: Backlog · **Priority**: low · **Cycle**: 1.0.x · **Wave**: W1 · **Lane**: F · **AC**: strong
- **Labels**: ws09-clawft-dashboard, audit-finding, gap
- **Blocked by**: WEFT-561
- **Blocks**: none
- **Gap**: WEFT-315 shipped the bundle-size budget gate (scripts/bench/check-ui-bundle-size.sh) and an eslint-plugin-jsx-a11y static lint pass. The original AC explicitly required "axe-core integrated into the Playwright suite or run as a standalone script across all 14 routes" — that runtime axe-core scan is not in the tree. The
- **Plan**: Wait for WEFT-561

### ws10-voice (32 open)

#### WEFT-214 — ws10: voice_listen / voice_speak tools — wire to real STT/TTS with cloud fallback

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: WEFT-671
- **Blocks**: none
- **Gap**: voice_listen and voice_speak tools are stubs returning a fixed status string. Without these wired, no agent-callable voice path exists.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-215 — ws10: weft voice setup — real model download with SHA-256 verify and progress UI

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: weft voice setup prints a stub message; no real model fetch happens.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-216 — ws10: WakeWordDetector — wire rustpotter or document an alternative

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WakeWordDetector::process_frame always returns false. No rustpotter dependency, no model file, no CPU enforcement, no "hey weft" model under models/voice/wake/hey-weft.rpw.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-217 — ws10: EchoCanceller and NoiseSuppressor — replace deceptive passthroughs with real DSP

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, bug, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Both modules are passthroughs. EchoCanceller has a circular reference buffer that process() ignores; NoiseSuppressor tracks RMS noise floor via EMA but does not filter. The presence of computed-but-unused state is misleading.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-218 — ws10: WS voice:status — connect a real backend broadcaster

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap, orphan
- **Blocked by**: none
- **Blocks**: none
- **Gap**: VoiceWsEvent and the voice:status topic are defined; the UI subscribes; nothing emits. Half-orphan API.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-219 — ws10: /api/voice/* — replace MSW-only mocks with real handlers in clawft-services

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: /api/voice/{status,settings,test-mic,test-speaker} are MSW-mocked only. UI works only against mocks; the daemon does not serve them.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-221 — ws10: Talk Mode interruption — abort TTS when VAD trips during playback

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: TtsAbortHandle is defined but synthesize() never checks it. TalkModeController::run() is a thin pass-through. There is no interruption detection (mic VAD trips while TTS speaking → abort handle).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-222 — ws10: VoicePersonality — wire per-agent lookup in TTS dispatch

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: VoiceConfig.personalities (HashMap<String, VoicePersonality>) is configured and validated but no TTS path consults it. VoicePersonality.greeting_prefix is never consumed.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-223 — ws10: SC-2 audio buffer zeroization and voice.audio_retention config

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No zeroize of audio buffers after use; no voice.audio_retention config option (none/session/persist). Raw audio can persist in memory or on disk longer than necessary.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-238 — ws10: VoiceConfig.tts.provider="browser" — implement Web Speech dispatch or change default

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, bug, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The default TTS provider is "browser" (Web Speech API) but tts.rs has no browser dispatch. The only TTS providers wired are local-stub, OpenAI, and ElevenLabs. Default points at an unimplemented path.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-613 — Voicelab parity: Chatterbox cloned-voice fast tier (native port)

- **State**: Backlog · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: weak
- **Labels**: ws10-voice, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The david profile's fast tier is Chatterbox with a cloned reference voice (am_onyx, ~0.64s TTFA, timbre-matched to the slow tier). The native stack uses Kokoro (works, but generic voice, timbre shift vs Orpheus dan). Porting Chatterbox to native Rust/ONNX is its own project — def
- **Plan**: Strengthen AC before coding

#### WEFT-644 — SileroVoiceness: neural VAD behind the Voiceness trait (model staging + stateful ONNX + fallback)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: weak
- **Labels**: ws10-voice, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Follow-up to the round-8 spectral voiceness gate (3f0b631c). The Voiceness trait (clawft-channels/src/voice/voiceness.rs) is the seam. Scope: (1) stage silero_vad.onnx — download URL already in clawft-plugin::voice::models::available_vad_models (silero-vad-v5) — to ~/.weftos/mode
- **Plan**: Strengthen AC before coding

#### WEFT-220 — ws10: Windows install-service — automate schtasks or document manual route as final

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: weft voice install-service on Windows prints manual Task Scheduler instructions. Unclear if that is the long-term plan.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-224 — ws10: SC-3 cloud-fallback transparency log line

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, docs, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No transparency log line (Cloud fallback active: sending audio to ...) when the fallback chain falls through to a cloud provider. Users cannot tell when their audio leaves the machine.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-225 — ws10: SC-6 anti-replay nonce and transcription-echo confirmation

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Destructive voice actions need an anti-replay nonce ("Say 'confirm delta' to proceed.") and a transcription-echo confirmation pattern. Today neither exists.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-226 — ws10: SC-8 voice rate limiting (commands/min, wake/min, post-fail cooldown)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No rate limiting on voice commands or wake activations. Spec calls for 10 commands/min, 5 wake activations/min, plus a post-fail cooldown.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-227 — ws10: Speaker diarization via sherpa-rs

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Speaker diarization is not implemented. Multi-party sessions are unsupported.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-228 — ws10: Tauri-side native mic capture — replace browser-only getUserMedia path

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The browser side is the only "real" mic surface (clawft-ui/src/lib/audio.ts calls getUserMedia). Tauri-side native mic access does not exist.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-229 — ws10: Latency + WER + CPU benchmarks for voice pipeline

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, tests, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No latency benchmark suite (speech-end → first-response-byte), no WER benchmark against a standard English corpus, no CPU profiling harness with hard 2 % wake-budget enforcement.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-230 — ws10: Adaptive silence timeout learning

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Silence timeout is fixed at 1.5 s. Spec calls for adaptive learning of user speech patterns.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-231 — ws10: UI partial-transcription streaming and TTS word highlighting

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No partial-transcription stream over WS, no TTS word-highlighting during playback.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-232 — ws10: Discord voice bridge — clawft-channels voice → STT → agent → TTS → VC audio

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Discord voice channel bridge does not exist.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-233 — ws10: audio_transcribe / audio_synthesize tools — real WAV/MP3/OGG/WebM codec support

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: File extensions are validated (.wav/.mp3/.ogg/.webm in; .wav out) but no codec actually decodes/encodes. Tools advertise capability they do not have.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-234 — ws10: Cleanup orphan voice surfaces (events, statuses, voice-chat.ts, model_path types)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, orphan, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Multiple orphan surfaces: WakeWordEvent::Error emitted nowhere; VoiceStatus::Transcribing defined but never reached; voice-chat.ts::sendVoiceMessage not invoked by any UI component; WakeConfig.model_path: Option<String> vs WakeWordConfig.model_path: PathBuf divergence with no bridge; VoicePersonality.greeting_prefix an
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-236 — ws10: clawft-service-whisper — drop legacy dual-publish path post Phase-4 migration

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The whisper service publishes to two paths during a migration window. The TODO says remove after Phase 4.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-239 — ws10: CloudFallbackConfig — config-string to provider router

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: CloudFallbackConfig.stt_provider is documented as "whisper" but no string-to-provider router exists. Config to provider instantiation happens nowhere.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-240 — ws10: WakeConfig.sensitivity vs WakeWordConfig.threshold — unify the knob

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Two names for the same knob with no shared mapping.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-657 — ws10: voice — pocket-tts watch: adopt as fast-tier engine when official ONNX/Candle export ships

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Watch item. pocket-tts (100M-param Mimi-codec streaming TTS; ~200ms TTFA and ~6x real-time on 2 CPU cores claimed; voice cloning; MIT code / gated CC-BY-4.0 weights) is a strong candidate to replace Kokoro in the fast-ack tier — and in-process synthesis would make barge-in cancel instantaneous (drop the generator) vs H
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-615 — ws10: Re-enable barge-in — reframed as ERL-confidence-floor decision (ADR-068 D1)

- **State**: In Progress · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W1 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, tech-debt
- **Blocked by**: WEFT-628
- **Blocks**: WEFT-638
- **Gap**: Barge-in (user interrupting TTS) was disabled by default because self-barge-in fired on the assistant's own audio without echo-return-loss (ERL) verification. ADR-068 supersedes the original "wait for AEC" framing: barge-in is no longer a binary AEC-gated switch but an ERL-confidence-floor decision — the duplex path ad
- **Plan**: Wait for WEFT-628

#### WEFT-617 — ws10: Evaluate midstream for voice/ECC mid-stream gating (50ms CognitiveTick)

- **State**: Todo · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, gap, ruv-integration
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Voice/ECC needs mid-stream gating decisions on a 50ms CognitiveTick cadence. Evaluate whether midstream's primitives fit the tick loop for interrupt/commit gating without a bespoke implementation.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-628 — ADR-068 Phase 1 — desktop thin edge over localhost + ERL-into-compute_urgency

- **State**: Todo · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, ws11-agent-core-v1, gap
- **Blocked by**: WEFT-649, WEFT-650
- **Blocks**: WEFT-615, WEFT-646, WEFT-647, WEFT-648, WEFT-649, WEFT-650
- **Gap**: ADR-068 reframes the standalone voice edge as a thin client: the desktop edge streams into the daemon-hosted loop over localhost rather than owning an in-memory forest. Phase 1 delivers that thin edge and folds ERL confidence into compute_urgency.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-638 — Voice cutover eventually retires TalkForest (ADR-068)

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W2 · **Lane**: E · **AC**: strong
- **Labels**: ws10-voice, tech-debt
- **Blocked by**: WEFT-615
- **Blocks**: none
- **Gap**: Once the desktop thin edge (ADR-068 Phase 1) is proven, the standalone in-memory TalkForest path is redundant and should be retired so voice runs on one engine.
- **Plan**: Wait for WEFT-615

### ws11-agent-core-v1 (17 open)

#### WEFT-331 — ws11: agent-core-v1.1 — interactive Defer UX prompt-and-resume in panel

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: D2 surfaces GateDecision::Defer { reason } as a structured tool-result the LLM can re-plan against. Real interactive defer (panel-side prompt with human-in-the-loop hook) is v1.1 — needs panel UI work.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-673 — ws11: hermes loop — voice-review-gate residual gaps self-documented by WEFT-655 (forest-commit asymmetry + hold-drain seam)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, gap
- **Blocked by**: WEFT-655
- **Blocks**: none
- **Gap**: WEFT-655 shipped and is correctly Done, but its own commit message explicitly records two edges it did not close. Neither has a tracking item — a search of all 659 work items for their exact terms returned nothing, so they would have been lost with the closing ticket. 1. Voice forest-commit asymmetry. Voice-originated 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-326 — ws11: agent-core-v1.1 — stabilize append_turns_are_monotonic flake via injectable clock

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, tech-debt, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: append_turns_are_monotonic occasionally fails when two appends land in the same millisecond. The per-conv counter suffix in turn_id_for was added to address this but the test still races. Needs a reliable injectable clock or a deterministic counter to fully stabilize.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-327 — ws11: agent-core-v1.1 — promote overlay_probe + resolver_live_probe diagnostics into CI

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, tests, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: overlay_probe and resolver_live_probe (crates/clawft-core/tests/resolver_live_probe.rs) are #[ignore]-marked diagnostics that run live config through the production resolver. Useful for verifying the wire end-to-end but not in CI rotation.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-329 — ws11: agent-core-v1.1 — notify-driven hot-reload watcher for identity files

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: FileIdentityProvider re-reads .clawft/SOUL.md + IDENTITY.md on every call. Files are small so it's cheap today, but a notify-driven watcher with cache invalidation "arrives when measurement says it earns its keep."
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-332 — ws11: agent-core-v1.1 — per-user agent_ids for multi-tenant chat

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Chat is single-tenant: one concierge-bot principal registered at boot per D2. Per-user agent_ids (multi-tenant chat) ship in a future phase. Plumb caller identity through AgentService::dispatch so each panel/CLI session gets its own principal in the kernel AgentRegistry.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-333 — ws11: agent-core-v1.1 — register agent.chat SystemService for weft status

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: agent.chat should register a SystemService impl tracking last-completion-time so weft status shows it (kernel C2 surface). Today weft status has no visibility into agent-loop state.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-334 — ws11: agent-core-v1.1 — typed error variants for agent.chat

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: v1 surfaces strings via Response::error("agent.chat: <inner>") mirroring llm.prompt. Typed variants are deferred to v1.1. Replace string-format with a typed enum the panel can branch on.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-336 — ws11: agent-core-v1.1 — weft routing trace + replay commands

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: weft routing trace and weft routing replay commands read from the substrate routing path and expose p99 latency + fallback-rate metrics in weft status. Required for promotion-gate verification.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-338 — ws11: agent-core-v3 — MicroLoraRouter behind ruvllm-wasm 11-pattern HNSW cap lift

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: v3 MicroLoraRouter is explicitly deferred until ruvllm-wasm lifts the documented 11-pattern HNSW cap. The 35+-skill clawft catalog overruns ruvllm-wasm v2.0.1's per-index ceiling. v3 needs MicroLoRA adapter trained on logged decisions + journal preferences with mandatory shadow-mode + WITNESS audit before promotion.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-341 — ws11: agent-core-v1.1 — per-tool Permit token + proof-of-permission API

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Per-tool Permit { token } is currently discarded — the plan calls out "optionally pass the token to tools.execute" as a follow-up that requires a tool-side proof-of-permission API the registry doesn't yet expose. Kernel-side Deny { reason, receipt } also drops receipt because the panel UX has nowhere to render it. Perm
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-343 — ws11: agent-core-v1.1 — Arc<RwLock<LlmClient>> runtime swap on env rotation

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: daemon_llm() captures the client once at boot; runtime env changes (e.g. OPENROUTER_API_KEY rotated mid-session) go stale. Tracked by the resolver-live-probe diagnostic.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-348 — ws11: agent-core — Phase 4 skills auto-promotion from .claude/skills to .clawft/skills

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: After enough successful uses of a .claude/skills/* skill, promote to .clawft/skills/ for faster routing. Detector hooks are in skill_autogen.rs but the autopromote path is manual today.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-349 — ws11: agent-core — cross-agent delegation via existing delegate_tool

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: delegate_tool already exists; chat agent should be able to spawn specialist agents from agents/ profiles. v1 doesn't wire it.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-350 — ws11: agent-core — Phase 2 voice + streaming chat path

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: audio_transcribe / audio_synthesize / voice_listen / voice_speak already exist as tools. Chat path needs TurnContent::Audio populated and agent.chat_stream connection-takeover RPC. TurnContent::Mixed enum is pinned from day 1 (substrate JSONL ready) but never constructed today.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-629 — ADR-067 P1-graph — causal.node.state chain event + fold replay

- **State**: Todo · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Small kernel addition: a causal.node.state chain event plus fold replay so node-state transitions are chain-recorded and replayable.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-637 — Tools-as-nodes enrichment — deterministic spawn-edge rooting (M2 D3 seam)

- **State**: Backlog · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: D · **AC**: strong
- **Labels**: ws11-agent-core-v1, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Enrich tools-as-nodes so spawn edges are deterministically rooted in the conversation graph (the M2 D3 seam). Today spawn edges are not deterministically anchored.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws12-knowledge-graph (25 open)

#### WEFT-352 — ws12: KG-011 — activate LogQuantized for DiskANN once shaal PR #352 merges

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: LogQuantizedConfig::is_available() returns false with TODO(KG-011): Check ruvector-core version once PR #352 merges. Config types ship; activation gated on upstream merge.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-353 — ws12: KG-012 — activate unified SIMD distance kernel once shaal PR #352 merges

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: SimdDistanceConfig::is_available() returns false mirroring KG-011. Configs ship; activation gated on upstream merge.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-354 — ws12: KG-013 — spatio-temporal GNN for sonobuoy (K-STEMIT)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not started. Would create clawft-sensor/ crate or live in sonobuoy firmware. K-STEMIT spatio-temporal dual-branch architecture for sensor systems.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-355 — ws12: KG-015 — EA-Agent entity alignment for multi-repo dedup

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not started. LLM-agent-based multi-repo dedup. Builds on KG-008 single-repo dedup (CodaRAG, shipped).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-356 — ws12: KG-017 — knowledge distillation for edge EML (SevenNet-Nano)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not started. Distill depth-4 EML → depth-2 for WASM/ESP32. Edge-deployable EML.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-358 — ws12: OG-2 — OWL/RDF ingestion (Turtle, JSON-LD)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not started. Needs oxigraph or sophia crate to map RDF triples → graphify entities + relationships.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-359 — ws12: OG-3 — Barnes-Hut force layout + positioned-SVG export

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Basic force layout exists but Barnes-Hut O(n log n) and the SVG positioned-output pipeline still need work. Today the layout is O(n²).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-360 — ws12: OG-4 — VOWL visual encoding rules in SVG export

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Not started. Layer VOWL visual encoding rules (class circles, property arrows, equivalence ellipses, etc.) onto export/html.rs or new SVG export.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-361 — ws12: KG-004 — benchmark RFF vs Lanczos vs EML lambda₂ on 1K/10K/100K graphs

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, tests, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: RFF spectral embedding shipped at causal.rs:3171. Performance comparison vs Lanczos (sparse O(k·m), ADR-009) and EML-approximated lambda₂ on 1K / 10K / 100K node graphs is the open piece. Need a clear rule for which path runs when.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-362 — ws12: layout — implement Sugiyama layered layout (currently falls back to tree)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, bug, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Geometry::Layered falls back silently to layout_as_tree instead of a real Sugiyama / hierarchical-graph layered layout. Behavioural footgun — users requesting Layered get tree layout with no warning.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-364 — ws12: vector — ecc.vector-config RPC to show active backend

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No introspection RPC for which VectorBackend is active. Operators have to read config or trust the build.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-368 — ws12: ingest — replace StubHttpClient with real reqwest-based HTTP client

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, stub
- **Blocked by**: none
- **Blocks**: none
- **Gap**: StubHttpClient is the default for URL ingestion. Production reqwest-based client is not yet wired.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-369 — ws12: graphify — MCP server (Phase 6)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Master plan Phase 6 calls for an MCP server exposing graphify capabilities to MCP clients. Not started.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-370 — ws12: graphify — extraction + graph_ops benchmarks (Phase 6)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, tests, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: benches/extraction.rs and benches/graph_ops.rs named in master plan; not present in tree.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-371 — ws12: graphify — write ADR-049 (graphify port)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-049 (graphify port from Python to Rust) is the canonical decision record for the entire workstream. Status "pending" in master plan; never written.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-372 — ws12: graphify — write ADR-050..053 candidates from phase2 paper survey

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Four ADRs candidate per phase2 survey for already-shipped algorithms: ADR-050 SGKR (KG-006 shipped), ADR-051 CodaRAG entity dedup HNSW pre-filter (KG-008 shipped), ADR-052 TransFIR codebook cold-start (KG-014 shipped), ADR-053 K-STEMIT spatio-temporal (KG-013 not started).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-375 — ws12: graphify — edge embeddings for relationship queries (LightRAG P5)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Embed relationships not just entities, for "how does X interact with Y?" queries. Not implemented.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-376 — ws12: graphify — graph-aware HNSW re-ranking (LightRAG P4)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Re-rank HNSW neighbors by graph topology (degree, community, centrality). Not implemented; would bridge hnsw_service.rs and analyze.rs.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-377 — ws12: graphify — discover_hyperedges() pipeline step

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Hyperedge type exists at model.rs and KnowledgeGraph::hyperedges is populated by analysis but no first-class hyperedge detection algorithm runs in the pipeline yet — pipeline.rs does not call a discover_hyperedges() step.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-378 — ws12: graphify — vault domain hyperedges + SUGGEST→ratify→CRDT pipeline

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: vault/ ships v0.6.11 frontmatter + wikilink analysis but the SUGGEST → ratify → CRDT pipeline from the Ontology Navigator symposium (Sprint 22 plan, Phase 5 codebase schema agent) is not started.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-379 — ws12: graphify — index-based optimization for forensic gap_analysis (O(n·m) cliff)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, tech-debt, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: gap_analysis() is O(n·m) and "should be optimized with indexes for large graphs." Acceptable below 10K entities; known cliff above.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-381 — ws12: graphify — vision_extract end-to-end test fixture

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: vision_extract.rs (246 lines) is feature-gated (vision-extract) and shipped, but no end-to-end test runs through it (the feature-gate is off by default and no fixture in crates/clawft-graphify/schemas/).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-382 — ws12: graphify — schema-based edge validation in validation.rs

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: validation.rs (213 lines) implements JSON-shape validation only; schema-based edge validation is flagged "Minimal" in the symposium pipeline findings.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-385 — ws12: graphify — search-path prediction (HNSW-EML #4, biggest single win)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Region → entry-node lookup yields 2-5× search speedup. Open. Medium effort.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-387 — ws12: graphify — verify+restore standalone export/cypher.rs and export/svg.rs

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws12-knowledge-graph, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: MASTER_PLAN.md named export/cypher.rs and export/svg.rs. Cypher is currently realized inside export/wiki.rs flow only; svg.rs is not in tree.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws13-app-substrate (10 open)

#### WEFT-413 — ws13: clawft-app — wire ADR-015 rule 6 once clawft-adapter exists

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-015 rule 6 (permission ↔ adapter consistency check) is TODO'd until ADR-017/clawft-adapter lands. The let _ = Permission::Camera; placeholder is kept in scope. Governance at install time is the backstop today.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-418 — ws13: clawft-substrate — migrate mic adapter to substrate/<node-id>/sensor/mic/{summary,pcm}

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, stub, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The in-tree MicrophoneAdapter still emits the legacy flat substrate/sensor/mic summary only — no pcm topic, no node-scoped path. ESP32-signed publish path is only declared in planning.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-427 — ws13: clawft-surface — extract canon types and move composer back to clawft-surface

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The composer runtime lives in clawft-gui-egui::surface_host rather than in clawft-surface. The previously-cyclic dep is broken (f5e40c3) by composer relocation — but the proper fix (extract canon types to a shared crate, move composer back) is unscheduled.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-410 — ws13: clawft-app — decide UnknownMode validation variant fate

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ValidationError::UnknownMode is dead code — serde rejects out-of-set supported_modes values at parse time. Decision is open: wire a Rust-constructed-manifest check, or delete the variant.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-411 — ws13: clawft-app — add registry corruption quarantine path

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Registry corruption recovery currently returns a JSON error to the caller. A quarantine path (rename to apps.json.corrupt-<ts>) plus backup/repair would let the daemon self-heal a damaged registry instead of erroring out.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-414 — ws13: clawft-app — cover wasm to_toml_string failure path with negative test

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The web-time switch handled SystemTime but not toml::ser::Error. The wasm32 serialize-failure path is unverified.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-431 — ws13: integration — drive variant_id stamping in CanonResponse from surface binding

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: variant_id stamping is identity-mapped; ADR-006 head-metadata plumbing exists but doesn't drive rendering.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-440 — ws13: weftos-admin — migrate auto-install-from-fixture flow off web-time workaround

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The auto-install-on-boot bundled-fixture flow relies on a web-time workaround instead of a real install pipeline.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-619 — ws13: K6 — vendor exo-core (BLAKE3+HLC) + exo-dag (DagStore/MMR/SMT, no postgres) per ADR-043

- **State**: Todo · **Priority**: none · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, tech-debt, ruv-integration
- **Blocked by**: none
- **Blocks**: none
- **Gap**: K6 substrate wants a content-addressed append-only DAG. exo-core (BLAKE3 + HLC) and exo-dag (DagStore / MMR / SMT, no postgres dependency) are candidate vendored deps per ADR-043. Vendor + wire behind the K6 seam.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-621 — ws13: Clear FSL licensing question for any AgentBBS / late.sh source reuse

- **State**: Backlog · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: B · **AC**: strong
- **Labels**: ws13-app-substrate, governance, ruv-integration
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The AgentBBS ADR work (ADR-063..066) used patterns only. Any source reuse from AgentBBS / late.sh is FSL-licensed and must clear a licensing question before code is copied or linked. This blocks any future "reuse the AgentBBS implementation" path.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws14-deployment (20 open)

#### WEFT-453 — ws14: ci — soft-check docs-site MDX builds locally via scripts/build.sh ui

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, docs, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Docs site deploys via Vercel Git integration on docs/src/. There is no in-repo workflow that runs vercel deploy and the trigger surface (which paths trigger redeploy vs. need manual) is not documented. Contributors don't know whether their change triggers a redeploy.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-454 — ws14: cdn — snapshot every cdn-assets upload by commit SHA for rollback

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: cdn-assets rolling release is updated with --clobber. There is no audit trail for which version of WASM the docs site is serving on any given day, and no rollback path if a bad WASM build clobbers a working one.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-455 — ws14: ci — add browser-WASM size budget to wasm-browser.yml

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: wasm-build.yml (wasip2) has a size gate (300 KB raw / 120 KB gzip). wasm-browser.yml does not, even though it is the user-facing playground bundle. Without a budget the playground bundle can drift up silently.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-456 — ws14: deploy — add health-probe rollback path to scripts/deploy/vps-deploy.sh

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: vps-deploy.sh stops + removes the existing container, then docker runs the new one. If the new container exits, the old one is gone — no rollback. For a self-hosted deploy story that ships in 0.7.0 docs, this is a foot-gun.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-457 — ws14: ci — add macOS + Windows test job to pr-gates.yml

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tests, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: CI runs cargo test --workspace only on ubuntu-latest. macOS and Windows targets only get a cargo build via cargo-dist on tag push — platform-specific test failures are caught after release-cut, not in PR.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-459 — ws14: ci — add SBOM (CycloneDX) generation and attach to releases

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: cargo-dist supports CycloneDX SBOM generation, but [workspace.metadata.dist] does not enable it. No SBOM is attached to any release, which is a supply-chain story gap for downstream consumers.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-460 — ws14: tooling — add scripts/build.sh release-dry-run subcommand

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Every ADR / skill assumes scripts/build.sh covers all release ops, but the release flow today is git tag vX.Y.Z; git push --tags. There is no scripts/build.sh release-dry-run to rehearse the cargo-dist matrix locally for the host triple, so contributors discover release breakage only on tag push.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-461 — ws14: build-kb — move tools/build-kb into the workspace (or document why not)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: tools/build-kb is a build dep for two workflows (release-kb.yml, docs-assets.yml) but is not in the workspace Cargo.toml. It is invisible from the root workspace — no shared lockfile, no shared rustup toolchain, no cargo check coverage.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-462 — ws14: cargo-dist — schedule v0.31 → v1.0+ bump and regenerate release.yml

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: cargo-dist is pinned at v0.31.0; v1.0 has shipped and adds wasip2 support, SBOM, and other features. release.yml is autogenerated and the regenerate cadence is not documented. We will fall further behind every quarter.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-463 — ws14: scripts — bump or delete scripts/09-gate.sh stale floor + paths

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Gate asserts >=1200 kernel tests pass; current count is well above 1200 (handoff cites 1218 in clawft-core alone). The script also references .planning/sparc/weftos/0.1/09b-decision-resolution.md and phase-K0/decisions.md paths that may have moved under phase4/ after the planning reorg — script may be silently no-op'in
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-464 — ws14: scripts — wire scripts/k6-gate.sh into CI or mark developer-rehearsal

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, orphan, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: scripts/k6-gate.sh runs phase-K3..K6 gates but is not invoked from CI. The CI phase-gate is scripts/build.sh gate. Status of k6-gate.sh is ambiguous — production gate, dev rehearsal, or dead.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-465 — ws14: scripts — audit and reorganize dead scripts (wake units, py helpers, weave-init.sh)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, orphan, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Several top-level scripts may be orphaned: systemd + launchd unit files for the wake service aren't included in any release tarball; build_vp_deck.py and dev_server.py aren't part of any workflow; weave-init.sh may be superseded by weaver init (Rust binary).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-468 — ws14: docs — fix Fumadocs link drift for docs/deployment/*.md (move into docs/src or delete)

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, orphan, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-014 declares Fumadocs the single source of truth for docs, but docs/deployment/*.md lives outside docs/src/content/docs/ and is invisible to the public site. Either we move them in or delete them.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-471 — ws14: governance — adopt release-plz/git-cliff or amend ADR-002 to record current flow

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, governance, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-002 says cargo-dist will be 'complemented with release-plz' and git-cliff is also called out, but neither is adopted. Manual version bumps + tag + push is the current flow, with scripts/release/generate-changelog.sh as a home-rolled conventional-commit grouper. ADR-002 and reality have diverged.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-472 — ws14: planning — reconcile Element 10 tracker (ClawHub features tangentially deployment)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, orphan, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Element 10 tracker is marked COMPLETE but references ClawHub features (weft skills publish/install, Ed25519 signing, vector search) that mostly belong to the security/community workstream rather than deployment. Ownership is ambiguous.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-473 — ws14: deps — add quarterly dependency-sweep cadence (post-wasmtime-v33)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, governance, tooling, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The wasmtime v33 upgrade closed 10 Dependabot alerts. There is no documented 'quarterly dependency sweep' cadence — we rely on Dependabot alone, and major-version bumps tend to land reactively when something breaks.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-474 — ws14: deploy — confirm and document assess.weavelogic.ai deploy origin

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-015 names assess.weavelogic.ai as one of three properties. No CI workflow in this repo targets it. It likely lives in a sibling repo, but that's not documented anywhere visible from clawft.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-475 — ws14: homebrew — decide bottle vs source-build formula for weft-cli

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, governance, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The current Homebrew formula rebuilds from source via cargo — slow on macOS, especially Intel. A bottle (compiled binary) would be much faster but requires bottle hosting and matching the cargo-dist macOS targets.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-476 — ws14: ci — add wasm32-wasip2 build to release.yml or cargo-dist when supported

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: release-wasi.yml only fires on tag push and adds a 30-min wait. If/when cargo-dist supports wasm32-wasip2 natively (currently HP-16 deferred per release-wasi.yml header), folding it into the main release.yml would simplify the pipeline.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-477 — ws14: closure-sdk — re-check release-engineering implications when bridge work is proposed

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws14-deployment, audit-finding, audit-0.7.0, governance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Closure-sdk is currently 'defer / conceptual-only' due to AGPL-3.0. If weftos-closure-bridge work is ever proposed it would force a separate AGPL crate behind an IPC boundary — meaning a separate release-artifact lifecycle (separate crate, separate license bundle, possibly separate Docker image).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws15-mcp (9 open)

#### WEFT-559 — ws15: Windows named-pipe transport — implement DaemonClient + daemon listener for x86_64-pc-windows-msvc

- **State**: Backlog · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: J · **AC**: weak
- **Labels**: gap, ws15-mcp
- **Blocked by**: none
- **Blocks**: WEFT-483
- **Gap**: Re-enable Windows in cargo-dist target list once the named-pipe transport ships. See docs/guides/weftos-deferred-requirements.md (Windows transport section). Tracks the followup from WEFT-483 (deferred from 0.7.0). Implementation outline lives in the deferred-requirements doc; ve
- **Plan**: Strengthen AC before coding

#### WEFT-495 — ws15: WASM panel auth — token/capability model for webview proxy

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws15-mcp, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The webview connects to the same UDS the local user owns; there is no token, capability, or per-panel identity on the proxy layer. Multi-user kernels (per ADR-042 modes) would need to add this. For single-user dev workstations this is acceptable; for the multi-user / multi-tenant operating modes contemplated in ADR-042
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-496 — ws15: webview vs daemon allowlist — substrate.publish gating semantics

- **State**: Done · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: strong
- **Labels**: ws15-mcp, audit-finding, audit-0.7.0, governance, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Closed WEFT-496 / ADR-071 — revise "viewer only" to mediated-mutators-yes / raw-`substrate.publish`-no; hard denylist survives WEFT-250 union; agent.chat tools audited (no tool calls `substrate.publish`; sinks grant-gated under `_derived/`).
- **Plan**: Shipped; see `docs/plans/wave-0k-WEFT-496-result.md`

#### WEFT-558 — ws15: VSCode panel E2E — chip-icon DOM assertion (followup to WEFT-486)

- **State**: Done · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: ok
- **Labels**: tests, ws15-mcp
- **Blocked by**: WEFT-486
- **Blocks**: WEFT-486
- **Gap**: Closed — mock chip a11y overlay + `weft._test.chipStripSnapshot`; E2E asserts `data-chip-id="kernel"`. See `docs/plans/wave-0l-WEFT-558-result.md`.
- **Plan**: Shipped on `wave0l/weft-558-chip-e2e`

#### WEFT-497 — ws15: agent-core-chat feature flag — schedule removal post-D3 soak

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: strong
- **Labels**: ws15-mcp, audit-finding, audit-0.7.0, orphan, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: agent-core-chat feature flag survives in daemon.rs:3591-3621 so the D3 cutover can be reverted with one commit + flag flip. Removal is slated 'once D3 has burned in' but no scheduled removal commit exists. Will accumulate cruft if forgotten.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-499 — ws15: weft-gui-egui native bin — promote to scripts/build.sh native --gui + release artifact

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws15-mcp, audit-finding, audit-0.7.0, orphan, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: weft-gui-egui native bin compiles and exists, but is not wired into scripts/build.sh native, not packaged for release. Per docs/handoff.md:258, building it requires cargo build -p clawft-gui-egui --features native --bin weft-gui-egui directly. The handoff explicitly defers promoting it to a first-class artifact ('user 
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-500 — ws15: MCP HTTP transport — verify against real HTTP server (not just mock)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: strong
- **Labels**: ws15-mcp, audit-finding, audit-0.7.0, tech-debt, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: mcp/transport.rs exposes the trait and the audit did not exhaustively verify the HTTP transport implementation. If the HTTP path is tested only against MockTransport, real HTTP MCP servers may surface bugs at first contact. Real-server tests would catch issues like content-length handling, chunked transfer, TLS edge ca
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-682 — ws15: tracker — enforce the two-label rule at item creation (36 items unlabeled, all post-WEFT-556)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: strong
- **Labels**: ws15-mcp, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The project rule requires every work item to carry two labels: a workstream (wsNN-*) and a finding-type (bug/gap/stub/orphan/governance/ tech-debt/docs/tests/tooling). 36 items violate it: 20 missing both, 14 missing the workstream, 2 missing the type. The distribution is the actual finding, not the count. Items WEFT-1
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-639 — plane.sh wrapper fixes: WEFT-N resolution, real assignee lookup, cycle-membership via issue.cycle_id

- **State**: Todo · **Priority**: none · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: J · **AC**: strong
- **Labels**: ws15-mcp, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Three operational defects in the plane.sh wrapper, each hit and worked around during the 2026-07-05 sync (scripts/plane.py was NOT modified in that run): transition / close / comment take an issue UUID, not the WEFT-N sequence id. These subcommands PATCH/POST /issues/{arg}/ directly, so passing WEFT-603 hits /issues/WE
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws16-browser-wasm (17 open)

#### WEFT-390 — ws16: browser — streaming chat via ReadableStream / wasm-streams

- **State**: Todo · **Priority**: medium · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: StreamCallback = Box<dyn FnMut(&str) -> bool + Send> is incompatible with the browser's single-threaded !Send model. PipelineRegistry::complete_stream and LlmTransport::complete_stream default impl are gated #[cfg(not(feature = "browser"))]. A browser streaming entry needs a ?Send callback type or migration to wasm-str
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-393 — ws16: browser — write ADR-027 Browser WASM Support

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, governance, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-027 "Browser WASM Support" was never written. Slot at docs/adr/adr-027-*.md is occupied by adr-027-selective-libp2p.md, an unrelated topic. There is no ADR for the entire W-BROWSER decision tree (hybrid vs full port vs thin client, feature-flag mutex, OPFS deferral, async_trait ?Send tax, CORS-proxy convention).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-394 — ws16: browser — write docs/development/feature-flags.md

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: docs/development/feature-flags.md (rules for adding new deps so they don't break the WASM target) was never written.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-395 — ws16: browser — write docs/browser/cors-provider-setup.md + config-schema.md

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Two planned docs never created: docs/browser/cors-provider-setup.md — per-provider CORS recipes docs/browser/config-schema.md — full annotated config.json schema for browser mode docs/browser/deployment.md has a small CORS section but not the per-provider matrix.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-396 — ws16: browser — update root README.md and CLAUDE.md with browser build instructions

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Neither the root README.md nor CLAUDE.md mentions the browser stack. Discoverability gap.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-397 — ws16: browser — compile_error! when both native and browser features are enabled

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tech-debt, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Mutual-exclusion enforcement is by convention only — no compile_error! if someone enables both native and browser features. Crate stack relies on consumers to set default-features = false and pick one. Easy to break silently.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-398 — ws16: browser — split clawft-wasm host code into dedicated crate

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The two #[cfg(feature = "wasm-plugins")] modules (sandbox, engine, permission_store, audit totalling ~28 KLOC) are wasmtime-host code that runs on native / wasip2 — not the browser. They sit in clawft-wasm because of historical crate naming and are completely orthogonal to W-BROWSER.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-399 — ws16: browser — persistent conversation history via OPFS (CLAUDE.md-per-group)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: OnceLock<BrowserRuntime> puts the Mutex<Vec<ChatMessage>> in module-scope memory; on reload it vanishes. Plan called for CLAUDE.md-per-group in OPFS (mirroring openbrowserclaw); not designed.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-400 — ws16: browser — Web Worker harness variant

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Main-thread WASM blocks UI on long LLM calls. BrowserHttpClient is already worker-ready (uses WorkerGlobalScope::fetch fallback), but the harness never instantiates it in a worker. Decision and a worker.js template owe.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-404 — ws16: browser — data-driven provider-routing fallback order

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, gap, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Hard-coded fallback chain ["openrouter", "openai", "anthropic", "groq", "deepseek", "gemini", "xai"] in resolve_provider. Suitable for a demo but should be data-driven from config.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-405 — ws16: browser — sign + version browser bundle artefact (parity with WASI release)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: A · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tooling, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The release artifact attached to tags via wasm-browser.yml is unsigned. Signing parity with the WASI release flow (release-wasi.yml) is not in scope of any current step doc.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-406 — ws16: browser — threat-model note on api_key in JS-readable WASM memory

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, docs, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: BrowserLlmClient keeps api_key: SecretString in JS-readable memory once injected via init(config_json). Any XSS in the host page lifts the key. UI side at least encrypts in IndexedDB; the WASM side does not. Worth an explicit threat-model note (and ideally mitigations).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-407 — ws16: browser — performance profiling baseline (load, init, first-msg, memory)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tests, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: No metrics captured anywhere for browser load / init / first-msg latency or memory. Baseline missing.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-408 — ws16: browser — final regression suite + ≤10% test-duration regression check (P6.7)

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tests, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The per-crate gate matrix runs every PR but no timing comparison. The promised "final regression suite + docker smoke + ≤10% test-duration regression" is partial.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-563 — ws16: sparc(BW5) — retire scripts/check-features.sh references missed by WEFT-409 sweep

- **State**: Backlog · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, docs
- **Blocked by**: WEFT-409
- **Blocks**: WEFT-409
- **Gap**: WEFT-409 (commit 9630a534, 2026-04-30) retired scripts/check-features.sh across master-plan + BW1 + BW2 + orchestrator + step0 docs but missed .planning/sparc/browser/05-phase-BW5-wasm-entry.md. Two stale references survive: L581: - [ ] scripts/check-features.sh passes (
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-564 — ws16: scripts — actually retire or annotate scripts/check-features.sh (still on disk)

- **State**: Backlog · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, audit-finding, audit-0.7.0, tooling
- **Blocked by**: none
- **Blocks**: none
- **Gap**: WEFT-409 closed with the rationale that scripts/check-features.sh was 'never created'. Verification via git shows the file was actually added on 2026-02-25 by commit 6a1416c6 (feat: three-workstream implementation + unified build script) and is still present on disk: $ ls -l scripts/check-features.sh -rwxr-xr-x 1 aepod
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-599 — ws16: relax transitive wasm-bindgen =0.2.108 exact pin

- **State**: Todo · **Priority**: low · **Cycle**: 0.8.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws16-browser-wasm, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: A transitive dependency hard-pins wasm-bindgen = "=0.2.108", so the browser pkg build (scripts/build.sh browser) requires wasm-bindgen-cli 0.2.108 EXACTLY — the current latest (0.2.126) fails. Friction for any contributor on a newer CLI.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

### ws17-research (31 open)

#### WEFT-519 — ws17: LeWM — codify ADR-058 decoupling-invariant runtime checks

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: none
- **Blocks**: WEFT-520
- **Gap**: ADR-058 states the decoupling invariant — ECC remains authoritative per node, the latent world model is an additive consumer. The five formal rules are documented in prose but there are no compile-time or runtime checks that would catch a violation (e.g. world model short-circuiting a causal edge).
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-520 — ws17: LeWM — create weftos-worldmodel-core crate (no_std traits)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W1 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-519, WEFT-543
- **Blocks**: WEFT-521
- **Gap**: The LeWM diagram and ADR batch describe a weftos-worldmodel-core no_std traits crate. Crate does not exist on master at 2026-04-28.
- **Plan**: Wait for WEFT-519, WEFT-543

#### WEFT-521 — ws17: LeWM — create weftos-worldmodel-impls crate (candle ViT-tiny + AdaLN)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W2 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-520
- **Blocks**: WEFT-522
- **Gap**: Candle-backed ViT-tiny encoder + AdaLN-modulated predictor implementations are described in the diagram and ADRs but have no crate on master.
- **Plan**: Wait for WEFT-520

#### WEFT-522 — ws17: LeWM — create weftos-worldmodel facade crate

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W3 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-521
- **Blocks**: WEFT-523, WEFT-524, WEFT-525, WEFT-527, WEFT-533
- **Gap**: Facade crate that re-exports core + impls, providing the user-facing surface, does not exist.
- **Plan**: Wait for WEFT-521

#### WEFT-523 — ws17: LeWM — create weftos-sensor-pipeline + -wire crates

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W4 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-522
- **Blocks**: WEFT-526
- **Gap**: Sensor pipeline + on-the-wire crates (CBOR + Ed25519, observational-only packets, ExoChain-indexed every frame) do not exist on master.
- **Plan**: Wait for WEFT-522

#### WEFT-524 — ws17: LeWM — create clawft-worldmodel-service binary (3 deployment topologies)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W4 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-522
- **Blocks**: none
- **Gap**: The clawft-worldmodel-service binary is described (single, hot-standby, peer-to-peer topologies) but does not exist.
- **Plan**: Wait for WEFT-522

#### WEFT-525 — ws17: LeWM — create clawft-delegation crate

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W4 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-522
- **Blocks**: none
- **Gap**: In-tree clawft-delegation crate (delegation of cognitive work to remote/peer world-model services) does not exist.
- **Plan**: Wait for WEFT-522

#### WEFT-526 — ws17: LeWM — add mesh.sensor.v1.{encoded,consensus,control} topics on mesh wire

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W5 · **Lane**: B · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-523
- **Blocks**: none
- **Gap**: No mesh.sensor.v1.* topic definitions on the mesh wire today. CBOR + Ed25519 framing, observational-only, ExoChain-indexed every frame is described but not present.
- **Plan**: Wait for WEFT-523

#### WEFT-527 — ws17: LeWM — implement LatticeApi (7 methods) via ServiceApi

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W4 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-522
- **Blocks**: WEFT-528, WEFT-529
- **Gap**: LatticeApi is registered in spec only. Required methods: observe, observe_node, predict, plan, recall, subscribe_surprise, subscribe_drift.
- **Plan**: Wait for WEFT-522

#### WEFT-528 — ws17: LeWM — wire SIGReg sigreg_health Welford monitor + auto-rollback at 0.85/30s

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W5 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-527
- **Blocks**: WEFT-530
- **Gap**: ADR-050 mandates Welford-based sigreg_health measurable in production, version-tagged, ExoChain-attested, with auto-rollback when sigreg_health < 0.85 for 30 s. None of this exists in code.
- **Plan**: Wait for WEFT-527

#### WEFT-529 — ws17: LeWM — implement pred_φ predictor + LatentPlanner (CEM default)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W5 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-527
- **Blocks**: WEFT-530
- **Gap**: No pred_φ (z_t, a_t → ẑ_{t+1}) predictor. No LatentPlanner (CEM default / MPPI-warm / gradient shooting) running on the 10 Hz background thread.
- **Plan**: Wait for WEFT-527

#### WEFT-530 — ws17: LeWM — implement four-condition AND rollback gate

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W6 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: WEFT-528, WEFT-529
- **Blocks**: none
- **Gap**: ADR-055 specifies a four-condition AND gate: cluster SIGReg health, held-out probing accuracy, VoE surprise differentiation, temporal-straightening score. None implemented.
- **Plan**: Wait for WEFT-528, WEFT-529

#### WEFT-533 — ws17: LeWM — ExoChain attestation of (a_t, z_t, z_{t+1}, surprise) tuples

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W4 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, governance
- **Blocked by**: WEFT-522
- **Blocks**: none
- **Gap**: Observational tuples (a_t, z_t, z_{t+1}, surprise) should be ExoChain-attested every frame. Not wired.
- **Plan**: Wait for WEFT-522

#### WEFT-543 — ws17: LeWM — decide 192-dim SIGReg latent dimensionality (ADR-050)

- **State**: Todo · **Priority**: high · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, governance
- **Blocked by**: none
- **Blocks**: WEFT-520
- **Gap**: The diagram fixes 192 dims for the SIGReg manifold. This binds the wire format. We need a documented decision before the wire ships, not after.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-504 — ws17: ECC — verify ecc feature exclusion on wasm32-unknown-unknown

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: G · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, tests
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The ecc feature pulls native deps (blake3, vector-memory) that won't link on wasm32-unknown-unknown. Exclusion is asserted in cfg gates but not verified by CI or local build.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-506 — ws17: governance — make EffectVector explicit on auth/config/a2a/cron gates

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: C · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, tech-debt, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: EffectVector on the auth, config, a2a, and cron gates is currently 'context-only' / heuristic — risk weighting is implicit. Decisions are still being made by the engine, but the inputs aren't auditable.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-508 — ws17: ECC — define new RVF segment types for ECC structures and persistence

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ECC structures (CausalEdge, ImpulseRecord, CalibrationProfile, etc.) are conceptual at the RVF segment level — no formal segment-type definitions, so persistence is ad-hoc.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-510 — ws17: EML — incremental component-count maintenance for O(1) coherence feature extraction

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, tech-debt, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: from_causal_graph is currently O(n+m) because connected_components() is recomputed each call. For true O(1) feature extraction the component count needs incremental maintenance on edge add/remove.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-516 — ws17: KG — SASE clustering replacing label-propagation in cluster.rs

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: SASE (k-order graph convolution + Random Fourier Features) is parameter-free, linear-time, and would replace the current label-propagation in cluster.rs. Sketched, not implemented.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-531 — ws17: LeWM — implement two training surfaces (offline edge + online streaming-merge)

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-055 defines two surfaces: (1) offline per-sensor-class edge intelligence (RVF-delivered, hot-swappable); (2) online streaming-merge world-model training with per-class importance-weighted replay. Neither in code.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-532 — ws17: LeWM — per-sensor-class trainable RVF-hosted small models with hot-swap

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: ADR-057 defines three trainable RVF-hosted small models per sensor class (transmit-gate, aggregate, encode), hot-swap at tick alignment, with auto-rollback. Not implemented.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-544 — ws17: governance — decide rotate-but-not-revoke policy expression for auth_service

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, governance, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: With auth_service gates landing (T4), we need a policy expression for cases like 'agent X may rotate but not revoke'. Today's policy DSL is unclear on this dimension.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-547 — ws17: governance — close out 8-agent / 48-task exochain-fix-plan medium-severity rows

- **State**: Todo · **Priority**: medium · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, governance, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The 8-agent / 48-task exochain-fix-plan.md is partially consumed. Remaining medium-severity rows in the cap matrix carry over.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-509 — ws17: ECC — resolve 5 pre-existing clippy warnings in agent_loop, chain, gate

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, tech-debt
- **Blocked by**: none
- **Blocks**: none
- **Gap**: 5 pre-existing clippy warnings remain at: agent_loop.rs:30,138,219 chain.rs:1239 gate.rs:427 Not introduced by ECC, but unaddressed and surfaced during the K3c work.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-517 — ws17: KG — LightRAG dual-level keyword retrieval

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: LightRAG (Guo et al. 2410.05779) — dual-level keyword retrieval, ~610× fewer tokens than GraphRAG; would slot into suggest_questions(). Not in code.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-535 — ws17: sonobuoy — scaffold clawft-sonobuoy-ranging crate (G1 follow-up)

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap
- **Blocked by**: none
- **Blocks**: none
- **Gap**: G1 (sensor-position uncertainty) closed in RANGING.md (983 lines, ADR-078). Crate scaffold for clawft-sonobuoy-ranging is the next step, scheduled for v2.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

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

#### WEFT-546 — ws17: Democritus — add rate limiting on exposure surface

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, security
- **Blocked by**: none
- **Blocks**: none
- **Gap**: Security audit flagged a TODO acknowledging rate limiting is needed for the Democritus exposure surface; not implemented.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-548 — ws17: EML — numerical-stability scaffolding for nested exp/ln at scale

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, gap, performance
- **Blocked by**: none
- **Blocks**: none
- **Gap**: The CIFAR-10 MLP run blew up on deep EML trees. Full LLM-scale would require heroic numerical-stability engineering. Today there is no framework-level stability layer in eml-core.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

#### WEFT-549 — ws17: orphans — triage OpenFang gap targets (channel breadth, Hands, Tauri, security stack, OFP, marketplace, SDKs, TUI, migration)

- **State**: Todo · **Priority**: low · **Cycle**: 0.9.x · **Wave**: W0 · **Lane**: I · **AC**: strong
- **Labels**: ws17-research, audit-finding, audit-0.7.0, orphan, docs
- **Blocked by**: none
- **Blocks**: none
- **Gap**: OpenFang comparison surfaced ~10 distinct gap targets that no workstream owns: channel breadth (40 vs 13), autonomous Hands agents, Tauri 2.0 desktop, 16-layer security stack, OpenAI-compatible API, P2P OFP wire protocol, agent marketplace, JS+Python SDKs, ratatui TUI, migration tools.
- **Plan**: Ready when claimed; implement to AC; close with commit+tests+build

