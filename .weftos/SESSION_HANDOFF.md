# Session Handoff: Sprint 15

**Date**: 2026-04-03
**Version**: v0.4.0 (released, all channels live)
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

- [ ] **Pluggable analyzer registry** — `AnalyzerRegistry` with trait-based analyzers (ADR-023)
- [ ] **DependencyAnalyzer** — parse Cargo.toml, package.json, go.mod; track versions, detect outdated
- [ ] **SecurityAnalyzer** — delegate to clawft-security SecurityScanner via kernel service
- [ ] **TopologyAnalyzer** — discover services from docker-compose.yml, k8s manifests, .env files
- [ ] **DataSourceAnalyzer** — detect database connection strings, cache configs, object store refs
- [ ] **NetworkAnalyzer** — map egress URLs, API endpoints, webhook configs from code/config
- [ ] **Progressive discovery** — each analyzer's findings feed into the next assessment cycle
- [ ] **LLM assessor agent** — spawn via supervisor, analyze findings with LLM for higher-order insights
- [ ] **Assessment diff** — compare current vs. previous assessment, surface regressions/improvements

### WS2: Sandbox Polish (Priority: MEDIUM)

- [ ] **Fix browser WASM CI** — pin wasm-bindgen-cli version to match crate dep
- [ ] **Playground set pieces** — guided tour scenarios (security bug, provider race, governance wall)
- [ ] **Provenance panel** — show WITNESS chain verification on KB segments
- [ ] **Knowledge graph visualization** — D3/React-Three-Fiber view of KB segment relationships
- [ ] **Streaming responses** — SSE from WASM for progressive LLM output
- [ ] **ruvllm-wasm integration** — local inference mode using ruvector's WASM routing engine

### WS3: CI/CD Hardening (Priority: HIGH)

- [ ] **Fix browser WASM workflow** — pin wasm-bindgen-cli, add to release artifacts
- [ ] **PR gates** — add `weft assess run --scope ci --format github-annotations` to pr-gates.yml
- [ ] **Docs-assets workflow** — trigger on Browser WASM success to update cdn-assets
- [ ] **crates.io publish** — automate crate publishing on tag (currently manual)
- [ ] **Dependabot** — address 16 known vulnerabilities (3 high, 10 moderate, 3 low)

### WS4: Client Deployment Readiness (Priority: HIGH)

- [ ] **SOP 3: Cross-project mesh** — implement real mesh coordination (not just artifact comparison)
- [ ] **SOP 4: Continuous assessment triggers** — filesystem watcher, git hooks, cron scheduling
- [ ] **SOP 5: SOP improvement loop** — agents propose SOP amendments from operational data
- [ ] **Multi-company namespace isolation** — separate governance domains per client
- [ ] **Assessment report dashboard** — web view of assessment results (extend /clawft/ or new route)
- [ ] **RabbitMQ/message queue topology discovery** — analyzer for event-driven architectures
- [ ] **Terraform/IaC analyzer** — parse infrastructure-as-code for deployment topology

### WS5: Plugin Ecosystem (Priority: MEDIUM)

- [ ] **clawft-plugin-npm** — Node.js dependency parsing via package.json/lock
- [ ] **clawft-plugin-ci** — GitHub Actions / Vercel config parsing
- [ ] **Plugin marketplace scaffold** — create-weftos-plugin CLI, registry design
- [ ] **Rustdoc JSON-to-MDX** — converter for native Fumadocs API pages

### WS6: weavelogic.ai (Separate Project)

*Tracked in weavelogic.ai project*

- [ ] /about, /contact with Calendly
- [ ] ROI calculator
- [ ] Sitemaps, PostHog analytics
- [ ] Restructure /services as post-assessment flow
- [ ] Consolidate CTAs to 2 variants

---

## Deferred to Sprint 16+

- Block drag-and-drop layout editing (GUI)
- Playground Phase 3-4 (governance panel, agent spawning UI)
- Security audit (1.0 gate)
- Post-quantum key exchange implementation (ADR-028 Phase 2)
- BLAKE3 hash migration from SHAKE-256 (ADR-043)
- ServiceApi trait implementation (ADR-035)
- N-dimensional EffectVector refactor (ADR-034 C9)
- wasip2 migration from wasip1 (ADR-044)
- ChainAnchor blockchain integration (ADR-041)

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
