# Session Handoff: Sprint 15

**Date**: 2026-04-03
**Version**: v0.4.1 (released, all channels live)
**Branch**: master

---

## Current State

Sprint 14 complete. v0.4.0 released with WASM sandbox, assessment framework, CLI kernel compliance, and 28 ADRs. 23 crates, 48 ADRs, 3,900+ tests.

**Previous sprints archived**: `.planning/development_notes/sprint-13-14/`

## What's Working

| Component | Status |
|-----------|--------|
| 23-crate Rust workspace | Compiles, 3,900+ tests (added clawft-rpc in Sprint 14) |
| 7 native build targets | Linux (glibc x2, musl x2), macOS x2, Windows x86 |
| 2 WASM targets | wasip1 (CI), browser (local build, cdn-assets) |
| WASM sandbox | /clawft/ route with RVF KB (1,160 segments), local + LLM mode |
| Assessment | `weft assess` CLI + AssessmentService kernel module + tree-sitter |
| Cross-project | Peer linking (local + HTTP), comparison, tested on 2 projects |
| CLI compliance | 32 commands use daemon-first RPC (clawft-rpc crate) |
| 48 ADRs | docs/adr/adr-001 through adr-047, with causal graph in .weftos/memory/ |
| Docs site | weftos.weavelogic.ai — 77 pages, prev/next nav, Edit on GitHub, glossary |
| All distribution channels | GitHub Releases, crates.io, npm, Docker, Homebrew, WASI |

---

## Sprint 15 — Depth, Polish, Client Readiness

### Theme

Sprint 14 built breadth (sandbox, assessment, CLI compliance, ADRs). Sprint 15 goes deep: make the assessment actually useful for client engagements, polish the sandbox into a product demo, fix CI gaps, and prepare for the first external deployment.

---

### WS1: Assessment Depth (Priority: HIGH)

The AssessmentService currently does file counting + complexity + TODOs. Make it actually discover and map systems.

- [x] **Pluggable analyzer registry** — `AnalyzerRegistry` with trait-based analyzers (ADR-023)
- [x] **DependencyAnalyzer** — parses Cargo.toml and package.json, flags missing/wildcard versions
- [x] **SecurityAnalyzer** — hardcoded secrets, .env files, `unsafe` blocks
- [x] **TopologyAnalyzer** — Docker, K8s manifests, .env files, port mappings
- [x] **DataSourceAnalyzer** — postgres://, redis://, S3, API endpoint detection
- [x] **NetworkAnalyzer** — map egress URLs, API endpoints, webhook configs from code/config
- [x] **Progressive discovery** — each analyzer's findings feed into the next assessment cycle
- [x] **LLM assessor agent (heuristic)** — spawn via supervisor, analyze findings with LLM for higher-order insights
- [x] **Assessment diff** — compare current vs. previous assessment, surface regressions/improvements

### WS2: Sandbox Polish (Priority: MEDIUM)

- [x] **Fix browser WASM CI** — pinned wasm-bindgen-cli to v0.2.108
- [x] **Guided tour set pieces** — 4 categories (getting started, architecture, assessment, security)
- [x] **WITNESS chain footer** — chain verification display in ExoChain log panel
- [x] **Knowledge graph visualization** — D3/React-Three-Fiber view of KB segment relationships
- [x] **Streaming responses** — SSE from WASM for progressive LLM output
- [x] **ruvllm-wasm stub (local-ai mode)** — local inference mode using ruvector's WASM routing engine

### WS3: CI/CD Hardening (Priority: HIGH)

- [x] **Fix browser WASM workflow** — pinned wasm-bindgen-cli, builds pass
- [x] **PR gates** — assessment + cargo check gates added to pr-gates.yml
- [x] **Docs-assets workflow** — manual dispatch with skip_wasm input, CDN fallback
- [x] **crates.io publish** — automate crate publishing on tag (currently manual)
- [x] **Dependabot** — fixed: quinn-proto, rustls-webpki, path-to-regexp, picomatch, flatted, brace-expansion. Accepted: wasmtime v29 (pinned) (3 high, 10 moderate, 3 low)

### WS4: Client Deployment Readiness (Priority: HIGH)

- [x] **SOP 3: Cross-project mesh** — MeshCoordinator, AssessmentMessage protocol, gossip, daemon RPC (not just artifact comparison)
- [x] **SOP 4: Git hooks** — `weft assess hooks` installs post-commit/pre-push hooks
- [x] **SOP 4: Config loading** — weave.toml trigger configuration with AssessmentConfig
- [x] **SOP 5: SOP review** — `weft assess review` with trend analysis — agents propose SOP amendments from operational data
- [x] **Multi-project namespace** — `[project]` section in weave.toml with org isolation
- [x] **Assessment report dashboard** — `/assess` route with stats, findings, peer comparison
- [x] **Cross-project demo** — clawft ↔ weavelogic.ai linked, assessed, compared bidirectionally
- [x] **RabbitMQ analyzer** — analyzer for event-driven architectures
- [x] **Terraform analyzer** — parse infrastructure-as-code for deployment topology

### WS5: Plugin Ecosystem (Priority: MEDIUM)

- [x] **clawft-plugin-npm** — Node.js dependency parsing via package.json/lock
- [x] **clawft-plugin-ci** — GitHub Actions / Vercel config parsing
- [x] **Plugin marketplace scaffold** — create-weftos-plugin CLI, registry design
- [x] **Rustdoc JSON-to-MDX** — converter for native Fumadocs API pages

### WS6: weavelogic.ai (Separate Project)

*Tracked in weavelogic.ai project. Cross-project coordination active.*

- [x] WeftOS initialized on weavelogic.ai (`.weftos/` created, assessed: 2,549 files, 801K LOC)
- [x] Linked as peer from clawft (bidirectional: clawft ↔ weavelogic-ai)
- [x] Git post-commit hook installed (auto-assessment on commit)
- [ ] /about, /contact with Calendly
- [ ] ROI calculator
- [ ] Sitemaps, PostHog analytics
- [ ] Restructure /services as post-assessment flow
- [ ] Consolidate CTAs to 2 variants

---

## Sprint 16 Plan

### Carried from Sprint 15
- [ ] **SOP 3: Cross-project mesh** — real-time mesh coordination via K6 transport (WS4)
- [x] **clawft-plugin-npm** — Node.js dependency parsing via package.json/lock (WS5)
- [x] **clawft-plugin-ci** — GitHub Actions / Vercel config parsing (WS5)
- [x] **Plugin marketplace scaffold** — create-weftos-plugin CLI, registry design (WS5)
- [x] **Rustdoc JSON-to-MDX** — converter for native Fumadocs API pages (WS5)
- [ ] **Evaluate gui/ vs ui/** — consolidate or archive redundant web dashboard (ui/) vs Tauri desktop (gui/)

### weavelogic.ai site (WS6)
- [ ] ROI calculator
- [ ] /about, /contact with Calendly
- [ ] Sitemaps, PostHog analytics
- [ ] Restructure /services as post-assessment flow
- [ ] Consolidate CTAs to 2 variants

### Architecture & Platform
- [ ] Block drag-and-drop layout editing (GUI)
- [ ] Playground Phase 3-4 (governance panel, agent spawning UI)
- [ ] Security audit (1.0 gate)
- [ ] Post-quantum key exchange implementation (ADR-028 Phase 2)
- [ ] BLAKE3 hash migration from SHAKE-256 (ADR-043)
- [ ] ServiceApi trait implementation (ADR-035)
- [ ] N-dimensional EffectVector refactor (ADR-034 C9)
- [ ] wasip2 migration from wasip1 (ADR-044)
- [ ] ChainAnchor blockchain integration (ADR-041)
- [ ] wasmtime upgrade to v33+ (closes remaining 10 Dependabot alerts)

---

## Known Issues

| Issue | Severity | Notes |
|-------|----------|-------|
| Browser WASM CI fails | Medium | wasm-bindgen version mismatch; local build works, cdn-assets has working pkg |
| 16 Dependabot alerts | Medium | 3 high, 10 moderate, 3 low — pre-existing |
| WS5 remaining exceptions | Low | ui, voice commands still bypass daemon (accepted exceptions) |
| Assessment service stubs | Low | Daemon RPC returns acknowledgments, not full results yet |

---

## Key References

| Resource | Location |
|----------|----------|
| ADR catalog (47) | `docs/adr/adr-001 through adr-047` + `docs/adr/PROPOSED.md` |
| ADR causal graph | `.weftos/memory/adr-graph.json` |
| Deployment SOPs | `docs/guides/weftos-deployment-sops.md` |
| Sprint 13-14 archive | `.planning/development_notes/sprint-13-14/` |
| WASM sandbox | `docs/src/app/clawft/WasmSandbox.tsx` |
| Assessment service | `crates/clawft-kernel/src/assessment.rs` |
| RPC crate | `crates/clawft-rpc/` |
| Release notes | `CHANGELOG.md`, `docs/src/content/docs/weftos/vision/releases.mdx` |
| Build script | `scripts/build.sh` |
| Pull assets (dev) | `scripts/pull-assets.sh` |
