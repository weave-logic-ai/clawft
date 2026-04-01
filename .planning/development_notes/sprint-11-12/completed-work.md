# Completed Work: Sprint 11 + Sprint 12

**Date**: 2026-03-28 through 2026-04-01
**Releases**: v0.1.0 through v0.1.4 (Sprint 11), v0.2.0 (Sprint 12)

---

## Sprint 11: Foundation + Release Engineering

### Symposium (9 Tracks)
11 documents in `docs/weftos/sprint11-symposium/`, 55 work items, 15 tech decisions, 13 risks.

### P0 Pre-Tag Blockers (9/9)
opt-level fix, publish=false on 13 crates, version.workspace on 22 crates, HNSW O(1), unimplemented→Err, non_exhaustive on 118 enums, CHANGELOG rebuilt, README fixed, rvf-crypto inlined.

### P1 Core Items (8/8)
ONNX WordPiece tokenizer, sparse Lanczos spectral analysis, HNSW deferred rebuild, cargo-dist init, Registry trait, ChainLoggable trait, feature gate coarsening, wasm_runner decomposition.

### Team Execution (5/5)
50 new tests, 4 docs, Arc/atomic optimizations, release config, 5 architecture specs.

### Release Engineering
- v0.1.0: First release, 5 platforms, cargo-dist + Homebrew
- v0.1.1: Fixed installer URLs, Homebrew tap org
- v0.1.2: Automated deploy test via skill
- v0.1.3: Expanded to 10 targets (musl + WASI + Win ARM — Win ARM deferred)
- v0.1.4: Fixed musl (vendored-openssl), WASI CI timing, 9 targets final

### Distribution Channels (all live)
- GitHub Releases: 3 binaries × 7 native targets + WASI
- crates.io: 10 crates (weftos, clawft-core/kernel/types/platform/plugin/llm, exo-resource-tree, weftos-rvf-crypto, weftos-rvf-wire)
- npm: @weftos/core (WASM browser module)
- Docker: ghcr.io/weave-logic-ai/weftos (distroless, multi-arch)
- Homebrew: weave-logic-ai/homebrew-tap (3 formulae)
- Docs: weftos.weavelogic.ai (Fumadocs 65+ pages + 1,014 rustdoc API pages)

### Documentation
- Fumadocs unified (65+ pages, 7 sections)
- API reference at /api/ (rustdoc HTML)
- 20 ADRs documented
- Installation page with all channels
- Landing page with install CTA

### HP Decisions
- HP-14: GitHub URL → weave-logic-ai/weftos
- HP-15: Crate names → weftos-* for forks, clawft-* for framework
- HP-16: WASM target → wasm32-wasip2

### Infrastructure Created
- weave-logic-ai/homebrew-tap repo
- Vercel project (weftos.weavelogic.ai)
- scripts/generate-api-docs.sh
- scripts/build-wasi.sh
- .github/workflows/release-wasi.yml
- Distroless Dockerfile
- weftos-build-deploy skill
- weftos-api-docs skill

### Fixes Applied
- Windows cross-compilation (#[cfg(unix)] gates on daemon/signals)
- Ruvector git→crates.io deps (weftos-rvf-crypto 0.3.0, weftos-rvf-wire 0.2.0)
- git2 vendored-openssl for musl builds
- Installer URL correction (repository field in Cargo.toml)
- Homebrew tap org fix (weavelogic → weave-logic-ai)
- WASI CI timing (5 min → 30 min wait for cargo-dist release)

---

## Sprint 12: Block Engine, Theming, Intelligence

### GUI + Theming
- Lego Block Engine: BlockRegistry, BlockRenderer, StateStore (Zustand + Tauri)
- 10 block components: Text, Code, Status, Table, Tree, Terminal, Button, Column, Row, Grid, Tabs
- 4 built-in themes: ocean-dark, midnight, paper-light, high-contrast (WCAG AAA)
- CSS variable bridge (--weftos-* properties), ThemeProvider, ANSI palette mapping
- Tailwind integration

### Agent Intelligence
- Context compression: sliding window, token counting, first-sentence summarization (8 tests)
- GEPA prompt evolution: TrajectoryLearner, FitnessScorer (4-dimension), prompt mutation (46 tests)
- Local LLM provider: Ollama/vLLM/llama.cpp/LM Studio factories (37 tests, 169 total)

### Test Count
- Sprint 12 crates: 1,560 tests passing
- Total workspace: 3,369+ tests

---

## Build & Deploy Runbook

See `.claude/skills/weftos-build-deploy/SKILL.md` for the complete release process.

### Quick Release
```bash
# Edit [workspace.package] version in Cargo.toml
cargo check --workspace
git add Cargo.toml Cargo.lock && git commit -m "chore: bump to X.Y.Z"
git push origin master && git tag vX.Y.Z && git push origin vX.Y.Z
# Monitor: gh run list --repo weave-logic-ai/weftos --limit 3
```

### Gotchas
- Tag must match workspace version exactly
- [profile.dist] must exist in Cargo.toml
- repository URL baked into installer scripts
- crates.io rate limit: ~1 new crate per 10 min
- npm needs granular token with "Bypass 2FA" for @weftos scope
- WASI workflow waits up to 30 min for cargo-dist to create release
- musl builds need vendored-openssl (git2 crate)
- Windows ARM deferred (ring cross-compilation blocked)
