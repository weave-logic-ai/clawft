# Element 10: Deployment & Community - Sprint Tracker
**Workstream**: K (Deployment & Community)
**Timeline**: Weeks 8-12
**Status**: Planning

## Phase Tracking

| Phase | Document | Status | Assigned | Notes |
|-------|----------|--------|----------|-------|
| K-Docker (W8-9) | 01-phase-KDocker-cicd.md | Planning | TBD | Multi-arch Docker, CI/CD, PR gates, release pipeline |
| K-Security (W9-11) | 02-phase-KSecurity-sandbox-plugin.md | Planning | TBD | SandboxPolicy, WASM+OS sandbox, 50+ security audit checks |
| K-Community (W10-12) | 03-phase-KCommunity-clawhub-benchmarks.md | Planning | TBD | ClawHub registry, vector search, benchmarks vs OpenClaw |

## Key Deliverables Checklist

### K2: Docker & Deployment
- [ ] Multi-arch Docker images (debian:bookworm-slim, cargo-chef, <50MB)
- [ ] VPS deployment scripts

### K2-CI: CI/CD Pipeline
- [ ] PR gates (clippy, test, WASM size, binary size)
- [ ] Release pipeline (docker buildx, GHCR push)
- [ ] Integration smoke test

### K3: Sandbox
- [ ] Per-agent sandbox (WASM + seccomp/landlock)
- [ ] SandboxPolicy struct, per-skill permissions, audit logs
- [ ] Secure defaults (not None), macOS fallback

### K3a: Security Plugin
- [ ] Security plugin (50+ checks, 10 categories)
- [ ] CLI commands (weft security scan/audit/harden)

### K4: ClawHub Registry
- [ ] ClawHub REST API (Contract #20)
- [ ] Vector search + keyword fallback
- [ ] Skill signing, star/comment, versioning
- [ ] weft skill install/publish, agent auto-search

### K5: Benchmarks & MVP Skills
- [ ] Benchmark suite (4 metrics vs OpenClaw)
- [ ] 3 MVP skills (coding-agent, web-search, file-management)

## File Map

| File | Unit | Action |
|------|------|--------|
| Dockerfile | K2 | Replace with multi-stage build |
| .github/workflows/pr-gates.yml | K2-CI | NEW |
| .github/workflows/release.yml | K2-CI | NEW |
| scripts/deploy/ | K2 | NEW directory |
| crates/clawft-plugin/src/sandbox.rs | K3 | NEW |
| crates/clawft-wasm/ | K3 | Extend WASM sandbox |
| crates/clawft-core/src/agent/sandbox.rs | K3 | NEW |
| clawft-security/ (new crate) | K3a | NEW crate |
| clawft-security/src/checks/ | K3a | NEW |
| clawft-security/src/hardening/ | K3a | NEW |
| clawft-security/src/monitors/ | K3a | NEW |
| clawft-cli/src/commands/security.rs | K3a | NEW |
| clawft-services/src/clawhub/ | K4 | NEW module |
| clawft-security/src/signing.rs | K4 | NEW |
| clawft-cli/src/commands/skill.rs | K4 | Extend |
| benches/ | K5 | Extend |
| scripts/bench/compare.sh | K5 | NEW |
| skills/ | K5 | NEW (3 MVP skills) |

## Cross-Element Dependencies

| Dependency | Description |
|------------|-------------|
| 04/C2 | WASM host for sandbox |
| 04/C3-C4 | Skill loader for ClawHub |
| 08/H2 | Vector search for ClawHub |
| 09/L2 | Per-agent workspace isolation |

## Risks

| # | Risk | Impact | Mitigation |
|---|------|--------|------------|
| 1 | K3a scope (50+ checks in 1 week) | High | Prioritize top 20 checks, scaffold remaining with stubs |
| 2 | ClawHub tight timeline | High | Start with minimal REST API, defer advanced features |
| 3 | Benchmark reproducibility | Medium | Pin toolchain versions, use deterministic test data |
| 4 | seccomp/landlock Linux-only | Medium | macOS fallback with reduced sandbox, document limitations |
| 5 | CI/CD delays | Medium | Start CI pipeline early in W8, iterate on gate thresholds |
| 6 | Supply chain attacks | High | Implement skill signing early, require signatures for publish |
| 7 | Docker image size | Low | Use cargo-chef caching, multi-stage builds, strip binaries |
