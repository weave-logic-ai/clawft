# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.9] - 2026-04-15

### Added

- **EML Attention — Iteration 1** (experimental): end-to-end joint
  coordinate descent across all 5 sub-models (Q/K/V/softmax/out)
  - `ToyEmlAttention::train_end_to_end(EndToEndTrainConfig)` — random-param
    random-perturbation trials with annealed step; accepts when end-to-end
    MSE drops on a 16-sample subset
  - `EmlModel::{params_slice, params_slice_mut, mark_trained}` — public
    access to the flat parameter vector for composed-model training
  - Small-random init in `ToyEmlAttention::new` (~±0.05) replaces the
    all-zeros default that saturated under composition
  - Bounded forward pass: soft-saturation squash `v·1e-3 / (1 + |v·1e-3|)`
    wraps raw `EmlModel::predict` outputs so composition can't produce
    Inf/NaN MSE when the tree saturates at `exp(20) ≈ 4.85e8`
- **Go/no-go gate passes** on the per-position-mean learnable task
  - Measured Iteration-1 result at the reference shape:
    baseline MSE 2.13 → final 0.90 = **57.8% reduction in 3 rounds**
  - 5 of 5 gate criteria PASS: G1 MSE reduction ≥ 5%, G2 p99 ≤ 5 µs
    (observed 2 µs), G3 timings finite, G4 JSON roundtrip, G5 polynomial
    scaling
- **4-phase benchmark** now reports `phase2_baseline_mse` and
  `phase2_mse_reduction` alongside `phase2_final_mse` so future iterations
  can quantify delta against this baseline

### Browser demo

- `/clawft_eml-notebook` gains an "Iteration 1 mode" toggle that runs
  end-to-end coordinate descent (trains all 5 sub-models jointly) instead
  of the Iteration-0 out_model-only self-distillation
- Live training log shows baseline MSE, per-round MSE, % reduction,
  elapsed time

### Known limitations (deferred to Iteration 2)

- **Identity task does not converge.** The Rust `EmlTree` composition
  saturates on identity because the nested `eml(x, y)` can hit `y ≤ 0`,
  triggering `ln(MIN_POSITIVE) ≈ -744` followed by `exp(20) ≈ 4.85e8`
  clamping. Single-param coordinate descent cannot escape. Iteration 2
  will either swap the tree formulation (closer to the TS port's
  `eml(v·c0 + c1, |v| + c2 + 1)` shape) or add a proper trainable output
  projection.
- **Q/K/V participation is modest.** Joint CD over 328 params reduces MSE
  by ~58% but plateaus near the bound-1 region where a majority of heads
  remain saturated. Multi-param coordinated perturbation or a
  saturation-safe tree would unlock further gains.

### Notes

- The 13 unit tests and the 4-phase benchmark harness remain unchanged.
- Default builds are byte-identical to 0.6.8.
- Feature flag `experimental-attention` on `eml-core` still gates the
  entire module.

## [0.6.8] - 2026-04-15

### Added

- **EML Attention — Iteration 0** (experimental): first step toward a
  gradient-free EML-Transformer for WeftOS
  - `ToyEmlAttention` in `eml-core` composed of 5 `EmlModel` instances
    (Q/K/V projections, learned softmax, output projection) with `f64`
    matmul between them — pragmatic hybrid approach, consistent with the
    design note's Iteration 4+ guidance
  - `new(name, d_model, d_k, seq_len, depth)` with toy-scale guards
    (`d_model ≤ 32`, `seq_len ≤ 8`, `depth ∈ 3..=5`)
  - `forward`, `record`, `train`, `to_json`/`from_json`, `AttentionError`
  - Self-distillation training: `train()` runs a forward pass on the buffer,
    derives per-submodel targets, then runs the existing coordinate-descent
    loop on `softmax_model` (row-softmax distillation) and `out_model`
    (`context → target`)
  - Feature flag `experimental-attention` on `eml-core` (off by default);
    default builds are unchanged
  - 13 unit tests covering construction, shape enforcement, numerical-softmax
    stability, serialization roundtrip, training-round counting, and the
    benchmark sanity gates
- **4-phase benchmark harness** `run_benchmark` + `AttentionBenchmark`
  mirroring `clawft-weave/src/commands/bench_cmd.rs` (Warmup → Transport
  → Compute → Scalability):
  - Phase 1: single forward-pass timing + JSON roundtrip
  - Phase 2: convergence on a memorizable identity task (96 samples, 3 rounds)
  - Phase 3: inference latency mean + p99 over 256 random inputs
  - Phase 4: `(seq_len, d_model)` scaling sweep with per-point param count
- **Docs**: new `docs/src/content/docs/weftos/eml-attention.mdx` page
- **Live demos**:
  - `/clawft_eml-attention` — pure-JS Iteration 0 forward-pass demonstrator
    (WASM rebuild follows in 0.6.9)
  - `/clawft_eml-notebook` — Python Colab notebook that trains a Python
    mirror and exports JSON directly loadable by Rust `EmlModel::from_json`
- **Planning**:
  - `.planning/development_notes/eml_model_development_assessment.md` —
    Iteration 0 plan, design rationale, 5 explicit go/no-go criteria
- **CHANGELOG + release notes** updated for 0.6.8

### Notes

- EML Attention is **additive**. Default builds are byte-identical to 0.6.7.
- Iteration 0 deliberately keeps Q/K/V at default init — only `softmax_model`
  and `out_model` train in this release. End-to-end coordinate descent across
  all five sub-models is Iteration 1.
- The two inter-projection matmuls (Q·Kᵀ and A·V) run in `f64`, not EML trees.
  Pure-EML matmul is a research problem; the design note treats hybrid as
  acceptable through Iteration 4+.

## [0.6.7] - 2026-04-15

### Added

- **Quantum Cognitive Layer** (experimental): neutral-atom quantum acceleration for ECC
  - `QuantumBackend` trait with object-safe async interface and shared types (`JobHandle`, `QuantumResults`, `JobStatus`, `BackendStatus`, `EvolutionParams`, `QuantumError`)
  - `quantum_register`: deterministic force-directed graph → 2D atom-position layout with device-constraint enforcement
  - **Live `PasqalBackend`** targeting Pasqal Cloud (EMU_FREE / EMU_TN / Fresnel):
    - Auth0 `client_credentials` flow with 24h token caching and 300s refresh skew
    - `POST /api/v1/batches`, `GET /api/v2/jobs/{id}`, `GET /api/v1/batches/{id}/results`, `PATCH /api/v2/jobs/{id}/cancel`
    - Best-effort Pulser abstract-repr JSON builder for `AnalogDevice` + global Rydberg channel
    - `submit_raw_sequence()` escape hatch for Python-generated Pulser JSON
    - Counts → per-atom Rydberg probability parsing
  - `BraketBackend` stub for QuEra Aquila on AWS Braket (interface only; live impl deferred)
  - Feature flags `quantum-pasqal` and `quantum-braket` (both off by default; experimental in 0.6.x)
  - T0 wiremock integration tests (14 tests exercising full auth + REST path)
  - T1/T3/T4 `#[ignore]`-gated live tests against real Pasqal endpoints
- **Docs**: new `docs/src/content/docs/weftos/quantum.mdx` page covering architecture, ECC integration, 4-tier test strategy, and roadmap
- **Planning**: `.planning/development_notes/pasqal-integration.md` extended with dual-backend strategy (§13), tiered dev environments (§14), 0.6.x experimental scope (§15), and 4-phase test runbook (§16)

### Notes

- The quantum layer is **additive**. Default builds are byte-identical to 0.6.6.
- `build_sequence_json` output is best-effort — the exact Pulser `to_abstract_repr()` schema is not fully stable. Runbook includes a Python golden-JSON validation path.

## [0.6.6] - 2026-04-14

### Added

- **18 Knowledge Graph tasks** (KG-001 through KG-018) completing Sprint 17
- **EML Score Fusion** (KG-001): Hybrid query combining keyword, graph, community, and type scoring
- **Community Summaries** (KG-002): GraphRAG-style summary generation for detected communities
- **Causal Chain Tracing** (KG-003): Typed BFS with natural-language explanations
- **RFF Spectral Analysis** (KG-004): O(m) approximate spectral analysis using random Fourier features
- **Info Gain Pruning** (KG-005): Redundant evidence filtering based on information gain
- **Data Flow Tracing** (KG-006): BFS dependency flow tracing through call chains
- **MCTS Graph Exploration** (KG-007): UCB1 + random rollout for knowledge graph traversal
- **Entity Dedup** (KG-008): Levenshtein + structural similarity deduplication
- **Geometric Shadowing** (KG-009): Age-aware decay with per-edge volatility
- **Multi-hop Beam Search** (KG-010): Prioritized traversal with edge priors
- **Sonobuoy Sensor Graph** (KG-013): GraphSAGE aggregation + temporal features
- **VQ Codebook Cold-Start** (KG-014): K-means++ initialization for new entities
- **Entity Alignment** (KG-015): Cross-graph matching via label + structural similarity
- **Conversational Exploration** (KG-016): Stateful multi-turn dialogue over knowledge graphs
- **EML Distillation** (KG-017): Depth-4 to depth-2 model compression
- **Newman Modularity** (KG-018): Global partition quality metric
- **Incremental graph updates** and **multi-key HNSW indexing**
- New modules: `summary.rs`, `alignment.rs`, `conversation.rs`, `sensor_graph.rs`, `vector_quantization.rs`
- 170+ new tests (1,770 total)

## [0.6.5] - 2026-04-04

### Added

- **eml-core standalone crate**: Zero-dep (just serde), configurable depth 2-5, multi-head outputs, coordinate descent training (36 tests)
- **12 EML models across 4 crates**: Coherence, governance scoring, restart strategy, health thresholds, dead letter policy, gossip timing, complexity limits, HNSW (distance/ef/path/rebuild), causal collapse, surprise scoring, cluster thresholds, layout tuning
- **Causal collapse prediction**: `rank_evidence_by_impact()` ranks candidate edges by predicted coherence impact via perturbation theory
- **Conversation cycle detection**: `detect_conversation_cycle()` identifies stuck/oscillating conversations via lambda_2 stagnation
- **HNSW EML training infrastructure**: HnswEmlManager with 4 models (distance, adaptive ef, path prediction, rebuild trigger), 33 tests

### Fixed

- **66 ExoChain compliance gaps closed**: Systematic audit of all mutation paths
- 75+ EVENT_KIND constants, 21 governance gates, EmlEvent types (Trained, Drift, Saved, Loaded) chain-witnessed
- 7 ExoChain/governance certification failures resolved

## [0.6.4] - 2026-04-04

### Added

- **EML depth-4 multi-head coherence**: 50 parameters, 3 output heads (lambda_2, fiedler_norm, uncertainty), 24 tests
- Two-tier DEMOCRITUS tick loop wired with EML coherence

## [0.6.3] - 2026-04-04

### Added

- **O(1) EML coherence approximation**: 34-parameter depth-3 master formula predicting algebraic connectivity from graph statistics
- Based on Odrzywolel 2026, "All elementary functions from a single operator" (arXiv:2603.21852v2)
- ~100ns prediction vs ~500us Lanczos iteration (5000x speedup), enabling 10,000 Hz tick rate for robotics
- Self-training: accumulates data during operation, retrains at 50+ samples
- Convergence verified on 5 standard graph families (K_n, star, cycle, path, Erdos-Renyi)
- 16 new tests

## [0.6.2] - 2026-04-04

### Added

- **Graphify extraction pipeline wired into CLI**: Full detect -> extract -> build -> cluster -> analyze -> export pipeline functional
- `weaver graphify rebuild` produces `graphify-out/graph.json` and `GRAPH_REPORT.md`
- `weaver graphify export` loads graph JSON and exports to 7 formats

## [0.6.1] - 2026-04-04

### Added

- **Workspace RPC**: `weft workspace create/list/load/status/delete` connected to daemon RPC
- **Cognitive tick auto-started** during kernel boot sequence
- **ECC tick auto-computed** from calibration bands (0.01ms to 1000ms, was hardcoded at 50ms)
- `weaver graphify ingest` delegates to the full pipeline for local directories

### Fixed

- Workspace RPC dispatch routing
- Cognitive tick startup sequencing

## [0.6.0] - 2026-04-04

### Added

- **Cognitum Seed gap sprint**: 11 gaps identified and closed across the kernel
- **Tiered kernel profiles** (T0-T4): Boot-time calibration selects appropriate resource tier from embedded microcontroller (T0) through GPU server (T4)
- Auto update check and universal install script

## [0.5.5] - 2026-04-04

### Added

- **VectorBackend trait**: Pluggable vector search with insert/search/remove/flush interface
- **HNSW backend** (default): In-memory approximate nearest neighbor via `instant-distance`
- **DiskANN backend**: SSD-backed vector search via `ruvector-diskann` v2.1, Vamana graph with product quantization and mmap persistence
- **Hybrid backend**: Hot HNSW cache + cold DiskANN store with access-counted promotion, LRU eviction, and merged search results
- Configurable via `[kernel.vector]` in `weave.toml`

## [0.5.4] - 2026-04-04

### Added

- **Benchmark v3**: 6-phase comprehensive performance suite (1,342 LOC)
- **ESP32-S3 edge benchmark**: Compatible with `weaver benchmark` v3 for edge devices

### Fixed

- Benchmark method names and parameter formats

## [0.5.3] - 2026-04-04

### Fixed

- **Benchmark scoring recalibrated**: Pi 5 was incorrectly graded A+, now correctly scores B/C

## [0.5.2] - 2026-04-04

### Added

- **`weaver benchmark`**: Standardized kernel performance test
- **Benchmark v2**: Three-tier testing (RPC, compute, stress)

### Changed

- **Kernel enabled by default**: No longer requires explicit `--features kernel` flag
- **ECC RPC dispatch endpoints** wired for all ECC commands
- **Per-project kernel runtime directory**: Prevents state collision across projects

## [0.5.1] - 2026-04-04

### Added

- **clawft-graphify** (new crate): 11,896 lines of Rust across 35 modules with 88 tests
- AST extraction via tree-sitter for Python, JavaScript/TypeScript, Rust, Go
- Community detection via label propagation with oversized splitting
- Analysis: god nodes, surprising connections (5-factor scoring), question generation (5 strategies), graph diff
- 7 export formats: JSON, HTML/vis.js, GraphML, Obsidian vault+canvas, Wiki, Cypher, SVG
- URL ingestion (tweet, arXiv, PDF, webpage) with SSRF protection
- CausalGraph bridge, 9th assessment analyzer, HNSW indexing
- Forensic domain: 14 entity types, 13 edge types, gap analysis, coherence scoring
- CLI: `weaver graphify` with 7 subcommands (ingest, query, export, diff, rebuild, watch, hooks)

## [0.5.0] - 2026-04-04

### Added

- **Sprint 16 architecture sprint**: wasmtime v33 upgrade, security audit
- **ServiceApi**: Unified kernel service registration and lifecycle
- **wasip2**: Full `wasm32-wasip2` target support across all crates
- **Playground phase 3-4**: Browser WASM sandbox improvements
- **Browser WASM features**: Enhanced client-side execution

### Changed

- Renamed `ui/` to `clawft-ui/` for workspace clarity

### Fixed

- Docker tarball directory component stripping

## [0.4.3] - 2026-04-04

### Added

- Sprint 14-15 documentation coverage pass: assessment, GUI, browser, plugins
- Docker Alpine optimization: Build time from ~30min to ~2min, image ~50MB to ~15MB

### Fixed

- `clawft-plugin-treesitter` added to crates.io publish pipeline
- `clawft-services` added to crates.io publish pipeline
- Handle 'already exists' error gracefully in crates.io publish

## [0.4.2] - 2026-04-04

### Added

- **Full boot sequence in ExoChain log**: INIT, CONFIG, SERVICES, NETWORK, READY phases visible
- **Rich markdown rendering**: Headings, bold, italic, code blocks, links, lists in chat bubbles
- **Document preview panel**: Click internal doc links to open in a side panel
- **Plugin marketplace scaffold**: `create-weftos-plugin` CLI for authoring new plugins
- **Rustdoc JSON-to-MDX converter**: Generates native Fumadocs API pages from Rust documentation
- **clawft-plugin-npm**: Node.js dependency parsing via package.json and lockfiles
- **clawft-plugin-ci**: GitHub Actions and Vercel config parsing
- **MeshCoordinator**: Real mesh coordination with AssessmentMessage protocol and gossip discovery
- 25 crates in workspace, 48 ADRs

### Fixed

- crates.io pipeline: 12 crates now publish automatically on tag

## [0.4.1] - 2026-04-03

### Added

- **Pluggable Analyzer Registry**: `AnalyzerRegistry` with `Analyzer` trait for extensible assessment
- **5 Built-in Analyzers**: ComplexityAnalyzer, DependencyAnalyzer (Cargo.toml/package.json), SecurityAnalyzer (secrets, .env, unsafe), TopologyAnalyzer (Docker, K8s), DataSourceAnalyzer (connection strings, S3, APIs)
- **Assessment Diff**: Compare current vs. previous assessment (files added/removed, findings new/resolved)
- **Assessment Hooks**: `weft assess hooks` -- install/uninstall post-commit and pre-push git hooks
- **Assessment Dashboard**: `/assess` route with project stats, findings list, peer comparison
- **Assessment Config**: Load trigger configuration from `.weftos/weave.toml`
- **Multi-project Namespace**: `[project]` section in weave.toml with org isolation
- **PR Assessment Gate**: `weft assess --scope ci --format github-annotations` in pr-gates.yml
- **Cargo Check Gate**: Workspace-wide `cargo check` in PR pipeline
- **Guided Tour Prompts**: 4 categories (getting started, architecture, assessment, security)
- **WITNESS Chain Footer**: Chain integrity display in ExoChain log panel
- **Docs-assets Manual Dispatch**: `workflow_dispatch` with `skip_wasm` input

### Fixed

- **Browser WASM CI**: Pinned wasm-bindgen-cli to v0.2.108 (matches Cargo.lock)
- **Wrong URLs**: Fixed all `github.com/clawft/clawft` to `weave-logic-ai/weftos` and `ghcr.io/clawft/clawft` to `weave-logic-ai/weftos` across 6 doc files
- **Test badge**: Updated from 3,300+ to 3,900+ on homepage
- **Crate count**: Updated from 22 to 23 (added clawft-rpc)
- **Glossary**: Added entries for clawft-rpc, AssessmentService, Analyzer/AnalyzerRegistry

## [0.4.0] - 2026-04-03

### Added

#### Sprint 14: WASM Sandbox, Assessment Framework, CLI Compliance, 28 ADRs

- **WASM Sandbox** (`/clawft/` route): Browser-based WeftOS agent running clawft-wasm with 1,160-segment RVF knowledge base, RAG-powered documentation search, local mode (no API key needed), LLM mode with provider routing, ExoChain log panel with live audit trail, runtime introspection, and "New chat" reset
- **CDN Asset Delivery**: GitHub Releases `cdn-assets` tag with Vercel API route proxy (`/api/cdn/[...path]`), edge-cached via `s-maxage`, blob URL WASM loading for MIME-type bypass
- **clawft-rpc** (new crate): Shared RPC client extracted from clawft-weave. `DaemonClient`, `Request`/`Response` protocol types, `connect_or_bail()` convenience. Both `weft` and `weaver` CLIs now share this crate
- **AssessmentService** (kernel): New `SystemService` with file scanning, tree-sitter symbol extraction (Rust/TypeScript), cyclomatic complexity analysis, scope support (full/commit/ci/dependency), peer coordination (link/compare via local path or HTTP URL), ExoChain audit logging
- **`weft assess` CLI**: `run`, `status`, `init`, `link`, `peers`, `compare` subcommands with daemon-first RPC and local fallback
- **Cross-project coordination**: Peer registry (`.weftos/peers.json`), assessment comparison across projects, validated on clawft (412K LOC) and weavelogic.ai (801K LOC)
- **Docs site**: Previous/next navigation, "Edit on GitHub" links, glossary (25+ terms), troubleshooting section, assessment workflow guide, deployment SOPs
- **28 ADRs** (ADR-020 through ADR-047): Kernel phase responsibilities, CLI daemon compliance, ExoChain mandatory audit, Noise encryption, Ed25519 identity, post-quantum dual signing, CBOR wire format, three-branch governance, effect algebra, forest of trees architecture, three operating modes, and 16 more

### Changed

- **CLI Kernel Compliance** (ADR-021): 32 `weft` commands migrated from direct file I/O to daemon-first RPC with local fallback. Commands: cron (6), assess (4), security (1), skills (7), tools (6), agents (3), workspace (8), other (3)
- **clawft-weave**: `DaemonClient` and core protocol types moved to `clawft-rpc`, re-exported for backward compatibility. 5 new daemon dispatch endpoints (`assess.*`)

### Fixed

- WASM JS glue MIME-type: load via blob URL to bypass CDN `application/octet-stream`
- GitHub Releases CORS: server-side proxy instead of Vercel rewrites (redirect chain exposed missing CORS headers)
- Vercel CDN caching: `s-maxage=604800` + `stale-while-revalidate=86400` on proxied assets

## [0.3.0] - 2026-03-31

### Added

#### Sprint 13: GUI Integration, Pipeline Wiring, Paperclip Patterns, HTTP API

- **GUI Integration**: KernelDataProvider for live kernel state in React, ThemeSwitcher component with runtime theme selection, BudgetBlock displaying agent budget consumption
- **Pipeline Wiring**: Config-based scorer and learner instantiation via factory functions, skill mutation pipeline with GEPA-driven prompt evolution
- **Paperclip Patterns**: Company and OrgChart organizational models, HeartbeatScheduler for liveness monitoring, GoalTree for hierarchical objective tracking
- **HTTP API**: `/execute`, `/govern`, and `/health` endpoints for external kernel interaction
- **Full WASI Support**: All 10 publishable crates compile for `wasm32-wasip2` target (10/10)
- **Windows ARM**: Re-enabled `aarch64-pc-windows-msvc` target with native-tls backend
- **Testing**: Property-based tests, fuzz harnesses, and benchmark suites across kernel and core crates
- **Integration Docs**: Paperclip integration guide, OpenClaw connector docs, local inference setup, cloud provider configuration, RuFlo orchestration reference

## [0.2.0] - 2026-03-31

### Added

#### Sprint 12: Block Engine, Theming, GEPA, Local LLM

- **Lego Block Engine** (`gui/src/engine/`, `gui/src/blocks/`): BlockRegistry, BlockRenderer, and Zustand+Tauri StateStore. 10 composable block components (Text, Code, Status, Table, Tree, Terminal, Button, Column, Row, Grid, Tabs) with recursive rendering and JSON descriptor-driven layout.
- **Theming System** (`gui/src/themes/`): 4 built-in themes (ocean-dark, midnight, paper-light, high-contrast with WCAG AAA compliance). CSS variable bridge via `--weftos-*` custom properties, ThemeProvider with runtime switching, ANSI palette mapping, and Tailwind integration.
- **Context Compression** (`crates/clawft-core/src/agent/context.rs`): Sliding-window context management with configurable `max_context_tokens` (default 8192). First-sentence summarization for older messages. Opt-in via `builder.with_compression(config)`.
- **GEPA Prompt Evolution** (`crates/clawft-core/src/pipeline/`): `TrajectoryLearner` replacing `NoopLearner` with trajectory collection, pattern extraction, and 4 prompt mutation strategies (rephrase, add examples, remove ineffective, emphasize). `FitnessScorer` replacing `NoopScorer` with 4-dimension weighted scoring (relevance, coherence, completeness, conciseness).
- **Local LLM Provider** (`crates/clawft-llm/src/local_provider.rs`): OpenAI-compatible provider for Ollama, vLLM, llama.cpp, and LM Studio. Key-optional auth, streaming, model listing. Factory methods: `LocalProvider::ollama()`, `::vllm()`, `::llamacpp()`, `::lmstudio()`.

## [0.1.0] - 2026-02-17

### Added

#### Core
- 9-crate Rust workspace: types, platform, core, llm, tools, channels, services, cli, wasm
- Agent loop with configurable retry and backoff
- 6-stage LLM pipeline: Classifier, Router, Assembler, Transport, Scorer, Learner
- Platform abstraction layer with traits for HTTP, filesystem, environment, and process
- Native platform implementation for Linux/macOS/Windows
- Tool registry with dynamic registration and dispatch
- Event-driven architecture with typed message passing

#### CLI
- Binary `weft` with subcommand-based interface via clap derive
- `agent` subcommand for running agent sessions
- `gateway` subcommand for HTTP/WebSocket gateway server
- `status` subcommand for system health and diagnostics
- `channels` subcommand for managing channel integrations
- `cron` subcommand for scheduled task management
- `sessions` subcommand for session lifecycle management
- `memory` subcommand for agent memory operations
- `config` subcommand for configuration management

#### Tools
- File operations: read, write, edit, list with path validation
- Shell execution with configurable timeout and working directory
- Agent spawn for sub-agent orchestration
- Memory tool for persistent key-value storage
- Web fetch with HTTP client and response parsing
- Web search with provider abstraction
- Message tool for inter-agent communication

#### Channels
- Telegram channel plugin with bot API integration
- Slack channel plugin with Web API and Events API support
- Discord channel plugin with gateway WebSocket connection

#### Services
- Cron scheduling service with cron expression parsing
- Heartbeat service for liveness monitoring

#### WASM
- Platform stubs for WebAssembly target (HTTP, FS, Env, Process)
- Feature flags (`native-exec`, `channels`, `services`) for conditional compilation
- WASM-compatible build profile with size optimizations

### Security
- `CommandPolicy` with allowlist and denylist for shell command execution
- `UrlPolicy` with SSRF protection (private IP blocking, scheme restrictions)
- Path traversal prevention in file operations

### Infrastructure
- GitHub Actions CI workflow with build matrix (stable, nightly, WASM)
- GitHub Actions release workflow with cross-compilation and asset publishing
- GitHub Actions benchmark workflow for performance regression tracking
- GitHub Actions WASM build workflow for browser/worker targets
- Docker multi-stage build with `FROM scratch` minimal image
- Release profile with LTO, strip, single codegen unit, and abort-on-panic
- 1,029 tests across the workspace

[0.6.6]: https://github.com/weave-logic-ai/weftos/compare/v0.6.5...v0.6.6
[0.6.5]: https://github.com/weave-logic-ai/weftos/compare/v0.6.4...v0.6.5
[0.6.4]: https://github.com/weave-logic-ai/weftos/compare/v0.6.3...v0.6.4
[0.6.3]: https://github.com/weave-logic-ai/weftos/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/weave-logic-ai/weftos/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/weave-logic-ai/weftos/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/weave-logic-ai/weftos/compare/v0.5.5...v0.6.0
[0.5.5]: https://github.com/weave-logic-ai/weftos/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/weave-logic-ai/weftos/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/weave-logic-ai/weftos/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/weave-logic-ai/weftos/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/weave-logic-ai/weftos/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/weave-logic-ai/weftos/compare/v0.4.3...v0.5.0
[0.4.3]: https://github.com/weave-logic-ai/weftos/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/weave-logic-ai/weftos/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/weave-logic-ai/weftos/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/weave-logic-ai/weftos/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/weave-logic-ai/weftos/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/weave-logic-ai/weftos/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/weave-logic-ai/weftos/releases/tag/v0.1.0
