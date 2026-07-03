# ruv (ruvnet) Repo / Crate Index

**Last Updated**: 2026-07-03. Source: GitHub API `users/ruvnet/repos` (197 repos
total; ~100 active/recent listed). WeftOS relevance flag: H/M/L.

## Established core (already deep-analyzed in dev-notes)

| Repo | Lang | ★ | Rel | Notes |
|------|------|---|-----|-------|
| [RuVector](https://github.com/ruvnet/RuVector) | Rust | 4314 | **H** | 100+ crates: vector DB + self-learning. Our deps: `ruvector-cluster`, `-raft`, `-replication`, `-diskann` (all v2.1.0). See `05-ruvector-crates.md`, `ruv-ecosystem-analysis-20260414.md`. New crates: `ruvector-hyperbolic-hnsw`, `ruvector-solver`, `ruvector-rabitq`. |
| [ruflo](https://github.com/ruvnet/ruflo) | TS | 62810 | **H** | The claude-flow platform (rebranded "leading agent meta-harness"). Provides the MCP server this session uses (swarm, memory, agentic-flow, and the new `agenticow_*` / `federation_bbs_*` / `metaharness_*` tool families). |
| [agentic-flow](https://github.com/ruvnet/agentic-flow) | TS/Rust | 770 | **H** | Agent orchestration, 213 MCP tools, AgentDB, QUIC. |
| [QuDAG](https://github.com/ruvnet/QuDAG) | Rust | 189 | **M** | Quantum-resistant DAG P2P comms, onion routing, QR-Avalanche. |
| [daa](https://github.com/ruvnet/daa) | Rust | 243 | **M** | Decentralized Autonomous Applications: MRAP loop, governance rules, token economy. |
| [ruv-FANN](https://github.com/ruvnet/ruv-FANN) | Rust | 371 | **M** | Fast memory-safe neural net library. |
| [sublinear-time-solver](https://github.com/ruvnet/sublinear-time-solver) | Rust | 82 | **M** | ADD sparse solvers (CG, forward/backward-push, BMSSP). |
| [Synaptic-Mesh](https://github.com/ruvnet/Synaptic-Mesh) | Rust | 75 | L | Self-evolving P2P neural fabric. |

## NEW / significantly updated since 2026-04-14 (this reparse)

| Repo | Lang | ★ | Rel | Deep-dive |
|------|------|---|-----|-----------|
| [agenticow](https://github.com/ruvnet/agenticow) | JS+native | 38 | **H** | `packages/agenticow/overview.md` — COW vector memory branching, MIT. MCP: `agenticow_*`. |
| [AgentBBS](https://github.com/ruvnet/AgentBBS) | Rust | 18 | **H** | `packages/agentbbs/overview.md` — federated signed agent+human BBS, FSL. MCP: `federation_bbs_*`. |
| [midstream](https://github.com/ruvnet/midstream) | Rust | 128 | **H** | `packages/new-libraries-2026-07.md` — real-time token-stream gating/steering, MIT/Apache. |
| [metaharness](https://github.com/ruvnet/metaharness) | TS | 360 | **M** | agent-harness factory, MIT. MCP: `metaharness_*`. |
| [RuLake](https://github.com/ruvnet/RuLake) | Rust | 11 | **M** | cache-coherent receipt-pinned memory fabric, MIT/Apache. |
| [agentdb](https://github.com/ruvnet/agentdb) | TS | 72 | **M** | standalone self-improving vector memory (backs `agentdb_*` MCP tools). |
| [rvm](https://github.com/ruvnet/rvm) | Rust | 114 | **M** | VM for the agentic age. |
| [RuView](https://github.com/ruvnet/RuView) | Rust | — | L | WiFi → spatial intelligence. |
| [rvcsi](https://github.com/ruvnet/rvcsi) | Rust | 13 | L | edge RF/WiFi-CSI runtime (sonobuoy-adjacent). |
| [worldgraph](https://github.com/ruvnet/worldgraph) | Rust | 11 | L | environmental digital twin (RF). |
| [rufield](https://github.com/ruvnet/rufield) | Rust | 17 | L | camera-free multimodal field-sensing spec. |
| [skygraph](https://github.com/ruvnet/skygraph) | Rust | 30 | L | realtime all-sky radar. |
| [rupixel](https://github.com/ruvnet/rupixel) | Rust | 32 | L | pixel-native visual RAG on ruvector ANN. |
| [SonicChamber](https://github.com/ruvnet/SonicChamber) | Rust | 2 | L | acoustic digital-human workbench (USCT). |
| [rvFACE](https://github.com/ruvnet/rvFACE) | Rust | 11 | L | face recognition SDK (Burn, WASM). |
| [helix](https://github.com/ruvnet/helix) | Rust | 26 | L | local-first anti-hallucination health record. |
| [ruv-neural](https://github.com/ruvnet/ruv-neural) | Rust | 20 | L | gamma-entrainment research OS. |
| [ruv-drone](https://github.com/ruvnet/ruv-drone) | Rust | 9 | L | cooperative-UAV fleet coordination. |
| [rvdna](https://github.com/ruvnet/rvdna) | JS | 9 | L | genomic analysis (RVF demo). |
| [ruqu](https://github.com/ruvnet/ruqu) | Rust | 7 | L | quantum computing (RVF demo). |
| [PhotonLayer](https://github.com/ruvnet/PhotonLayer) | Rust | 14 | L | optical-AI front end. |
| [hackerone](https://github.com/ruvnet/hackerone) | TS | 12 | L | defender-triage metaharness vertical. |
| [CVE-bench](https://github.com/ruvnet/CVE-bench) | JS | 16 | L | reproduce-and-fix security benchmark (AgentBBS arena). |
| [retort](https://github.com/ruvnet/retort) | — | 4 | L | platform-evolution/distillation engine. |
| [rudevolution](https://github.com/ruvnet/rudevolution) | JS | 111 | L | semantic decompiler. |
| [open-claude-code](https://github.com/ruvnet/open-claude-code) | JS | 432 | L | Claude Code CLI decompile/rebuild. |
| [Repo-Explainer](https://github.com/ruvnet/Repo-Explainer) | — | 14 | L | repo → visual explainer. |
| [ruvn](https://github.com/ruvnet/ruvn) | JS | 6 | L | research harness (question → cited report). |
| [symbolic-scribe](https://github.com/ruvnet/symbolic-scribe) | TS | 85 | L | — |
| [unsorry](https://github.com/ruvnet/unsorry) | — | 2 | L | Lean-4 theorem-proving agents (SETI@home for math). |
| [experiments](https://github.com/ruvnet/experiments) | — | 3 | L | published predictions / trajectories / logs. |

## Older notable (pre-reparse, context)

`dspy.ts` (262), `SynthLang` (262), `SAFLA` (155), `FACT` (179), `sparc` (470),
`rUv-dev` (426), `flow-nexus` (95), `midstream`'s predecessor experiments,
`obsidian-brain` (RuVector Brain / DiskANN bridge, 19), `code-mesh` (37),
`ARCADIA` (51), `Agent-Name-Service` (78, OWASP ANS protocol), `VIVIAN` (42,
vector index infra), `musica` (17, spectral-graph audio separation).

## MCP tool families → source library map

| MCP tool prefix (claude-flow) | Backing library |
|-------------------------------|-----------------|
| `agenticow_*` | agenticow |
| `federation_bbs_*` | AgentBBS |
| `metaharness_*` | metaharness |
| `agentdb_*`, `embeddings_*`, `ruvllm_*` | agentdb / RuVector |
| `swarm_*`, `hive-mind_*`, `hooks_*`, `daa_*` | ruflo / agentic-flow / daa |
| `aidefence_*` | agentic-security |
