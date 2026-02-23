# SPARC Feature Element 10: Deployment & Community

**Workstream**: K (Deployment & Community) -- in-scope items only (K2-K5, K3a)
**Timeline**: Weeks 8-12
**Status**: Planning
**Dependencies**: 04/C2 (WASM for sandbox), 04/C3-C4 (skills for ClawHub), 08/H2 (vector search for ClawHub)
**Blocks**: None

---

## 1. Summary

Docker images, CI/CD pipelines, per-agent sandboxing, security plugin, ClawHub skill registry with vector search, and OpenClaw benchmark suite. This is the final sprint element.

---

## 2. Phases

### Phase K-Docker: Container Deployment & CI/CD (Week 8-9)

| Item | Description |
|------|-------------|
| K2 | Multi-arch Docker images (linux/amd64, linux/arm64) |
| K2 | Base image: `debian:bookworm-slim` (glibc required for wasmtime). Multi-stage build with `cargo-chef` for dependency caching. Target: <50MB compressed image |
| K2 | One-click VPS deployment scripts |
| K2-CI | CI/CD pipeline (cross-cutting, benefits entire sprint) |
| K2-CI | PR gates: `cargo clippy`, `cargo test --workspace`, WASM size assertion (<300KB/<120KB), binary size regression check |
| K2-CI | Release pipeline: multi-arch Docker build via `docker buildx`, image push to GHCR (GitHub Container Registry) |
| K2-CI | Integration smoke test: start gateway container, verify `/health` endpoint responds, shut down |

### Phase K-Security: Sandbox & Security Plugin (Week 9-11)

| Item | Description |
|------|-------------|
| K3 | Per-agent sandbox (WASM + seccomp/landlock) |
| K3 | Per-skill permission system and audit logs |
| K3 | Integration note (K3/L2): per-agent config (`~/.clawft/agents/<id>/config.toml`) translates to sandbox policy via `SandboxPolicy` struct. Each agent's tool restrictions map to a `SandboxPolicy` that the K3 sandbox enforces at runtime |
| K3a | Security plugin (50+ audit checks, hardening, monitors) |
| K3a | Audit check inventory target -- categorized list of check types: prompt injection patterns (8+), exfiltration URL detection (5+), credential literal detection (5+), permission escalation patterns (5+), unsafe shell command patterns (5+), supply chain risk indicators (5+), DoS patterns (5+), indirect prompt injection (5+), information disclosure (3+), cross-agent access violations (3+) |

### Phase K-Community: ClawHub & Benchmarks (Week 10-12)

| Item | Description |
|------|-------------|
| K4 | ClawHub skill registry with vector search |
| K4 | Star/comment system, versioning, agent auto-search |
| K4 | API contract stub (forward reference to Contract #20): REST endpoints -- `GET /api/v1/skills/search?q=&limit=&offset=`, `POST /api/v1/skills/publish`, `POST /api/v1/skills/install`. Authentication: API key via `Authorization: Bearer <token>` header; publish requires signed key pair. Response schema: `{ "ok": bool, "data": T, "error": Option<string>, "pagination": { "total", "offset", "limit" } }` |
| K5 | Benchmark suite vs OpenClaw (feature parity, performance) |
| K5 | Methodology: `criterion` for micro-benchmarks (per-function), `hyperfine` for end-to-end CLI timing. Metrics: (1) binary size (bytes, stripped release), (2) cold start to first response (ms, `hyperfine --warmup 0`), (3) peak RSS under load (`/usr/bin/time -v` or `/proc/self/status`), (4) messages/sec throughput (sustained 60s run, criterion). Baseline: OpenClaw latest release tag at sprint start. Results run in CI on dedicated runner for reproducibility |
| K5 | 3 MVP OpenClaw skills: `coding-agent`, `web-search`, `file-management` -- ported after C3 lands, scheduled for Week 6-7 |

---

## 3. Exit Criteria

- [ ] Docker image builds and runs on amd64 and arm64 (<50MB compressed)
- [ ] CI/CD pipeline enforces PR gates (clippy, test, WASM size, binary size)
- [ ] Release pipeline publishes multi-arch images to GHCR on tag
- [ ] Integration smoke test passes in CI (gateway start + health check)
- [ ] Per-agent sandbox enforces tool restrictions
- [ ] Default sandbox type is NOT `None` (secure by default: `Wasm` for WASM plugins, `OsSandbox` for native on Linux)
- [ ] `weft security scan` runs 50+ audit checks across 8+ categories (including SupplyChainRisk, DenialOfService, IndirectPromptInjection)
- [ ] ClawHub requires skill signatures for publication (`--allow-unsigned` flag available for local dev only, logs warning)
- [ ] Platform-specific sandbox fallback documented for non-Linux systems (macOS: WASM-only fallback; warning emitted when OS sandbox unavailable)
- [ ] `weft skill install` works from ClawHub registry
- [ ] Benchmark suite produces comparison report with all 4 metrics against OpenClaw baseline
- [ ] 3 MVP skills (`coding-agent`, `web-search`, `file-management`) pass benchmark suite
- [ ] All existing tests pass

---

## 4. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| K3a 50+ audit checks in 1 week is aggressive | Medium | Medium | 6 | Create categorized check inventory early (Week 7); prioritize P0 categories (prompt injection, exfiltration, credentials); defer lowest-priority categories to post-sprint |
| K4 ClawHub tight timeline (2 weeks for full registry) | Medium | High | 8 | API contract stub defined early (Contract #20); H2 vector search must land by Week 9; fallback: keyword-only search if vector not ready |
| K5 benchmark results non-reproducible | Medium | Medium | 6 | Pin OpenClaw baseline version; run benchmarks on dedicated CI runner; document hardware specs in results |
| seccomp/landlock Linux-only leaves macOS unsandboxed | Low | High | 4 | WASM sandbox layer is cross-platform; document macOS as WASM-only sandbox; native tool sandbox deferred to post-sprint for non-Linux |
| CI/CD pipeline delays block image publishing | Low | High | 4 | CI/CD work starts Week 8 (first K item); PR gates can be added incrementally; image build is well-understood Rust+Docker pattern |
| ClawHub supply chain attack via malicious skill | Medium | Critical | 8 | Mandatory code signing; content hash verification; `weft security scan` runs automatically on `weft skill install`; unsigned skills rejected by default |
| Docker image size exceeds 50MB target | Low | Low | 2 | Multi-stage build with cargo-chef; strip binaries; monitor size in CI with assertion |
