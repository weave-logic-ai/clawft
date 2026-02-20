# Element 10: Deployment & Community - Sprint Tracker
**Workstream**: K (Deployment & Community)
**Timeline**: Weeks 8-12
**Status**: COMPLETE (K2-K5 delivered, K4 install/publish depends on ClawHub server)

## Phase Tracking

| Phase | Document | Status | Assigned | Notes |
|-------|----------|--------|----------|-------|
| K-Docker (W8-9) | 01-phase-KDocker-cicd.md | DONE | Agent-10 | Multi-arch Docker, CI/CD, PR gates, release pipeline |
| K-Security (W9-11) | 02-phase-KSecurity-sandbox-plugin.md | DONE | Agent-10 | SandboxPolicy, WASM+OS sandbox, 50+ security audit checks |
| K-Community (W10-12) | 03-phase-KCommunity-clawhub-benchmarks.md | DONE | Agent-10 | ClawHub registry, vector search, benchmarks vs OpenClaw |

## Key Deliverables Checklist

### K2: Docker & Deployment
- [x] Multi-arch Docker images (debian:bookworm-slim, cargo-chef, <50MB)
- [x] VPS deployment scripts

### K2-CI: CI/CD Pipeline
- [x] PR gates (clippy, test, WASM size, binary size)
- [x] Release pipeline (docker buildx, GHCR push)
- [x] Integration smoke test

### K3: Sandbox
- [x] Per-agent sandbox (WASM + seccomp/landlock)
- [x] SandboxPolicy struct, per-skill permissions, audit logs
- [x] Secure defaults (not None), macOS fallback

### K3a: Security Plugin
- [x] Security plugin (55 checks, 10 categories)
- [x] CLI commands (weft security scan, weft security checks)

### K4: ClawHub Registry
- [x] ClawHub REST API (Contract #20) -- stubs
- [x] Vector search + keyword fallback
- [x] Skill signing, star/comment, versioning
- [ ] weft skill install/publish from registry (server-side pending)
- [ ] Agent auto-search (depends on C3/C4 landing)

### K5: Benchmarks & MVP Skills
- [x] Benchmark comparison script (4 metrics vs OpenClaw)
- [x] 3 MVP skills: prompt-log, skill-vetting, discord (skills/ directory, 4 parser tests)

## File Map

| File | Unit | Action | Status |
|------|------|--------|--------|
| Dockerfile | K2 | Replaced with multi-stage build | DONE |
| .github/workflows/pr-gates.yml | K2-CI | NEW | DONE |
| .github/workflows/release.yml | K2-CI | NEW | DONE |
| scripts/deploy/vps-deploy.sh | K2 | NEW | DONE |
| scripts/deploy/docker-compose.yml | K2 | NEW | DONE |
| crates/clawft-plugin/src/sandbox.rs | K3 | NEW (SandboxPolicy) | DONE |
| crates/clawft-core/src/agent/sandbox.rs | K3 | NEW (SandboxEnforcer) | DONE |
| crates/clawft-security/ | K3a | NEW crate (55 checks) | DONE |
| crates/clawft-cli/src/commands/security_cmd.rs | K3a | NEW | DONE |
| crates/clawft-services/src/clawhub/ | K4 | NEW module | DONE |
| scripts/bench/compare.sh | K5 | NEW | DONE |

## Test Results

| Crate | Tests | Status |
|-------|-------|--------|
| clawft-security | 19 | PASS |
| clawft-plugin (sandbox) | 62 | PASS |
| clawft-core (sandbox+all) | 18 | PASS |
| clawft-cli | 312 | PASS |
| clawft-services (clawhub+all) | 252 | PASS |

## Cross-Element Dependencies

| Dependency | Description | Status |
|------------|-------------|--------|
| 04/C2 | WASM host for sandbox | Available |
| 04/C3-C4 | Skill loader for ClawHub | Available (MVP skills delivered) |
| 08/H2 | Vector search for ClawHub | Keyword fallback implemented |
| 09/L2 | Per-agent workspace isolation | Available |

## Risks

| # | Risk | Impact | Mitigation | Status |
|---|------|--------|------------|--------|
| 1 | K3a scope (50+ checks) | High | 55 checks delivered across 10 categories | RESOLVED |
| 2 | ClawHub tight timeline | High | REST API stubs + keyword search delivered | MITIGATED |
| 3 | Benchmark reproducibility | Medium | comparison script with baseline.json | MITIGATED |
| 4 | seccomp/landlock Linux-only | Medium | macOS fallback with warning implemented | RESOLVED |
| 5 | CI/CD delays | Medium | All workflows delivered | RESOLVED |
| 6 | Supply chain attacks | High | Signature enforcement in publish API | RESOLVED |
| 7 | Docker image size | Low | cargo-chef multi-stage + strip | RESOLVED |
