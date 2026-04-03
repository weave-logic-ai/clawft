# Sprint 14 — Completed (v0.4.0, 2026-04-03)

## WS1: Docs Site Rewrite (13/13)
- Title, hero, layer cards, feature highlights, footer, badges
- Prev/next navigation, Edit on GitHub, glossary, troubleshooting
- Getting-started with expected output blocks

## WS2: Interactive Playground (12/12)
- /clawft/ browser WASM sandbox with 1,160-segment RVF KB
- Local mode (no API key) + LLM mode with provider routing
- ExoChain log panel, runtime introspection, New Chat reset
- CDN asset delivery via GitHub Releases + Vercel API proxy
- Blob URL WASM loading, s-maxage edge caching

## WS3: SOPs + Assessment (7/7)
- `weft assess` CLI: run, status, init, link, peers, compare
- AssessmentService kernel module (597 lines, SystemService impl)
- Tree-sitter (Rust/TS symbol extraction + cyclomatic complexity)
- Git mining (commit/ci diff scopes)
- Cross-project coordination (local + HTTP peer linking)
- Validated on clawft (412K LOC) + weavelogic.ai (801K LOC)
- Docs: /docs/weftos/guides/assessment + deployment-sops

## WS5: CLI Kernel Compliance
- clawft-rpc crate extracted (shared DaemonClient)
- 32 weft commands migrated to daemon-first RPC
- ADR-020 to ADR-023 (kernel responsibilities, CLI compliance, ExoChain audit, assessment service)

## ADRs (28 new: ADR-020 through ADR-047)
- Security: Noise, Ed25519, dual signing, three-branch governance
- Wire formats: CBOR, rvf-wire
- Networking: QUIC, libp2p, SWIM, LWW-CRDT
- Architecture: DashMap, effect algebra, ServiceApi, ToolRegistry, ChainAnchor
- Cognitive: three modes, forest of trees, self-calibrating tick
- Platform: MSRV 1.93, Tauri, wasip1, tiered router

## Key Artifacts
- 23 crates (added clawft-rpc)
- 47 total ADRs (001-047)
- v0.4.0 release: 57 assets, 7 platforms
