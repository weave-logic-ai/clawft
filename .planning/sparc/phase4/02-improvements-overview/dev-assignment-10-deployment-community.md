# Development Assignment: Element 10 -- Deployment & Community

**Element**: 10 (`10-deployment-community`)
**Workstream**: K (in-scope items only: K2-K5, K3a)
**Timeline**: Weeks 8-12
**Sprint Phase**: Phase 5
**Orchestrator**: `.planning/sparc/10-deployment-community/00-orchestrator.md`
**Dependencies**: 04/C2 (WASM host), 04/C3-C4 (skill loader), 08/H2 (vector search)
**Blocks**: None (final element)

---

## 1. Overview

Element 10 is the final sprint element, delivering container deployment, CI/CD automation, per-agent sandboxing, security auditing, the ClawHub skill registry, and OpenClaw benchmark suite. All work begins after the MVP milestone (Week 8) and builds on infrastructure delivered by Elements 03-09.

**Existing assets**:
- `Dockerfile` exists at project root (minimal `FROM scratch` image, copies pre-built static binary)
- `.github/workflows/benchmarks.yml` -- existing benchmark CI with regression detection
- `.github/workflows/wasm-build.yml` -- WASM build + size gate CI

The existing Dockerfile uses `FROM scratch` with a musl static binary. K2 replaces this with a `debian:bookworm-slim` multi-stage build (glibc required for wasmtime) using `cargo-chef` for dependency caching.

---

## 2. Unit Breakdown

### Unit 1: K2 Docker + CI/CD (Week 8-9)

**Goal**: Multi-arch Docker images and a complete CI/CD pipeline with PR gates and release automation.

#### K2: Docker Images

| Deliverable | Description | Crate/File |
|------------|-------------|------------|
| Multi-stage Dockerfile | Replace `FROM scratch` with `debian:bookworm-slim` multi-stage build using `cargo-chef` for dependency caching | `Dockerfile` (replace) |
| Multi-arch build | Support `linux/amd64` and `linux/arm64` via `docker buildx` | `Dockerfile`, CI workflow |
| Size target | <50MB compressed image; strip binaries, remove build deps | `Dockerfile` |
| VPS deployment scripts | One-click VPS deployment scripts for common providers | `scripts/deploy/` (new) |

**Implementation notes**:
- Stage 1: `cargo-chef prepare` -- extract recipe
- Stage 2: `cargo-chef cook --release` -- build deps only (cached layer)
- Stage 3: `cargo build --release` -- build application
- Stage 4: `debian:bookworm-slim` runtime image with only the binary + required shared libs (libssl, libgcc)
- Use `strip` on the release binary to reduce size
- Test both architectures in CI via QEMU emulation (`docker buildx create --use`)

#### K2-CI: CI/CD Pipeline

| Deliverable | Description | File |
|------------|-------------|------|
| PR gates workflow | `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, WASM size assertion (<300KB/<120KB), binary size regression check | `.github/workflows/pr-gates.yml` (new) |
| Release pipeline | Multi-arch Docker build via `docker buildx`, image push to GHCR on tag | `.github/workflows/release.yml` (new) |
| Integration smoke test | Start gateway container, verify `/health` endpoint responds within 10s, shut down | `.github/workflows/pr-gates.yml` or `release.yml` |
| Binary size tracking | Assert release binary < 10MB in CI | `.github/workflows/pr-gates.yml` |

**Implementation notes**:
- PR gates consolidate checks from existing `benchmarks.yml` and `wasm-build.yml` where applicable
- Release pipeline triggers on semver tags (`v*.*.*`)
- GHCR push requires `GITHUB_TOKEN` with `packages: write` permission
- Integration smoke test uses `docker run --rm -d`, polls `/health`, then `docker stop`

**Exit criteria**:
- [x] Docker image builds and runs on both amd64 and arm64
- [x] Compressed image size < 50MB
- [x] PR gates enforce clippy, test, WASM size, binary size checks
- [x] Release pipeline publishes multi-arch images to GHCR on tag
- [x] Integration smoke test passes (gateway start + `/health` check)

---

### Unit 2: K3 + K3a Sandbox & Security Plugin (Week 9-11)

**Goal**: Per-agent sandboxing with WASM + OS-level isolation, plus a security plugin with 50+ audit checks.

#### K3: Per-Agent Sandbox

| Deliverable | Description | Crate/File |
|------------|-------------|------------|
| `SandboxPolicy` struct | Define sandbox policy type with tool restrictions, network rules, filesystem rules | `clawft-plugin/src/sandbox.rs` (new) |
| WASM sandbox layer | Use wasmtime's WASI capabilities to sandbox WASM plugins (filesystem, network, env) | `clawft-wasm/` (extends C2) |
| OS sandbox layer (Linux) | seccomp + landlock for native tool execution on Linux | `clawft-core/src/agent/sandbox.rs` (new) |
| Per-agent config mapping | Agent config (`~/.clawft/agents/<id>/config.toml`) translates tool restrictions to `SandboxPolicy` | `clawft-core/src/agent/` |
| Per-skill permissions | Each skill declares required permissions in manifest; sandbox enforces at runtime | `clawft-plugin/` |
| Audit logging | All sandbox decisions (allow/deny) logged for forensic analysis | `clawft-core/src/agent/sandbox.rs` |
| Secure default | Default sandbox type is NOT `None`: `Wasm` for WASM plugins, `OsSandbox` for native on Linux | Config defaults |
| macOS fallback | WASM-only sandbox on macOS; warning emitted when OS sandbox unavailable | Runtime detection |

**Dependencies**: C2 (WASM host must be functional)

**Implementation notes**:
- `SandboxPolicy` maps from per-agent `config.toml` tool restriction settings
- Agent A can have shell access while Agent B is restricted to read-only file ops
- seccomp filter uses `seccompiler` crate or equivalent; landlock uses `landlock` crate
- Platform detection: `#[cfg(target_os = "linux")]` for OS sandbox, WASM sandbox is cross-platform
- Document macOS limitation: native tools run without OS-level sandbox (WASM layer still applies)

#### K3a: Security Plugin

| Deliverable | Description | Crate/File |
|------------|-------------|------------|
| Security plugin crate | New plugin implementing 50+ audit checks across 10 categories | `clawft-security/` (new crate) or `clawft-plugin/src/security/` |
| Audit check categories | Prompt injection (8+), exfiltration URL (5+), credential detection (5+), permission escalation (5+), unsafe shell (5+), supply chain risk (5+), DoS patterns (5+), indirect prompt injection (5+), information disclosure (3+), cross-agent access (3+) | `clawft-security/src/checks/` |
| Hardening modules | Auto-apply seccomp/landlock profiles, restrict network per-skill, enforce allowlisted domains | `clawft-security/src/hardening/` |
| Background monitors | Watch for anomalous tool usage, excessive API calls, unexpected file access patterns | `clawft-security/src/monitors/` |
| CLI integration | `weft security scan`, `weft security audit`, `weft security harden` commands | `clawft-cli/src/commands/security.rs` (new) |
| Skill install hook | `weft security scan` runs automatically on `weft skill install` | `clawft-core/src/agent/skills_v2.rs` (extend) |

**Implementation notes**:
- Check inventory should be categorized early (Week 7 prep) to manage scope
- P0 categories: prompt injection, exfiltration, credentials
- P1 categories: permission escalation, unsafe shell, supply chain risk
- P2 categories: DoS, indirect injection, information disclosure, cross-agent access
- Each check returns severity (Critical/High/Medium/Low/Info) and a remediation suggestion
- Critical/High severity findings from `weft skill install` scan block activation by default

**Exit criteria**:
- [x] Per-agent sandbox enforces tool restrictions at runtime
- [x] Default sandbox type is NOT `None` (WASM for plugins, OS sandbox on Linux)
- [x] `weft security scan` runs 50+ audit checks across 8+ categories
- [x] Platform-specific sandbox fallback documented for non-Linux (macOS: WASM-only, warning emitted)
- [x] Audit logs record all sandbox allow/deny decisions

---

### Unit 3: K4 ClawHub Registry (Week 10-12)

**Goal**: A skill registry with vector-based semantic search, community features, and supply chain security.

| Deliverable | Description | Crate/File |
|------------|-------------|------------|
| REST API contract | Implement endpoints per Contract #20 (see API schema below) | `clawft-services/src/clawhub/` (new) |
| Vector search integration | Embed skill descriptions, match user queries semantically via H2 vector store | `clawft-services/src/clawhub/search.rs` |
| Keyword fallback | If vector search (H2) is not ready, fall back to keyword-only search | `clawft-services/src/clawhub/search.rs` |
| Star/comment system | Users can rate and review skills, moderation hooks | `clawft-services/src/clawhub/community.rs` |
| Versioning | Semver versioning for skills; `weft skill update` checks for newer versions | `clawft-cli/src/commands/skill.rs` (extend) |
| Agent auto-search | When agent can't find matching local skill, queries ClawHub automatically | `clawft-core/src/agent/skills_v2.rs` (extend) |
| Skill signing | Mandatory code signing for publication; content hash verification | `clawft-security/src/signing.rs` (new) |
| `weft skill install` | Install skills from ClawHub; triggers security scan before activation | `clawft-cli/src/commands/skill.rs` (extend) |
| `weft skill publish` | Publish skills to ClawHub; requires signed key pair | `clawft-cli/src/commands/skill.rs` (extend) |
| `--allow-unsigned` flag | Local dev only; logs warning when used; rejected by default | `clawft-cli/src/commands/skill.rs` |

**Dependencies**: C3 (skill loader), C4 (dynamic loading), H2 (vector store)

**API contract** (Contract #20):
```
GET  /api/v1/skills/search?q=&limit=&offset=
POST /api/v1/skills/publish
POST /api/v1/skills/install

Authentication: API key via Authorization: Bearer <token> header
Publish: requires signed key pair

Response schema:
{
  "ok": bool,
  "data": T,
  "error": Option<string>,
  "pagination": { "total", "offset", "limit" }
}
```

**Exit criteria**:
- [x] ClawHub requires skill signatures for publication (`--allow-unsigned` for local dev only, logs warning)
- [ ] `weft skill install` works from ClawHub registry (server-side pending)
- [x] Vector search returns semantically relevant results (or keyword fallback if H2 unavailable)
- [ ] Agent auto-search triggers when no local skill matches (depends on C3/C4 landing)

---

### Unit 4: K5 Benchmarks vs OpenClaw (Week 10-12)

**Goal**: A comprehensive benchmark suite comparing clawft against OpenClaw across 4 metrics, with 3 MVP skills as test subjects.

| Deliverable | Description | Crate/File |
|------------|-------------|------------|
| Micro-benchmarks | `criterion` benchmarks for per-function performance | `benches/` (extend existing) |
| E2E benchmarks | `hyperfine` for end-to-end CLI timing | `scripts/bench/` (extend existing) |
| 4 metric suite | Binary size (bytes, stripped release), cold start to first response (ms, `hyperfine --warmup 0`), peak RSS under load (`/usr/bin/time -v`), messages/sec throughput (sustained 60s, criterion) | `scripts/bench/`, `benches/` |
| OpenClaw baseline | Pin OpenClaw latest release tag at sprint start; run benchmarks on dedicated CI runner | `.github/workflows/benchmarks.yml` (extend) |
| CI integration | Benchmark results run in CI on dedicated runner for reproducibility | `.github/workflows/benchmarks.yml` |
| Comparison report | Generate human-readable comparison report with all 4 metrics | `scripts/bench/compare.sh` (new) |
| 3 MVP skills | `coding-agent`, `web-search`, `file-management` ported as test subjects | `skills/` (depends on C3 landing) |

**Implementation notes**:
- Existing `benchmarks.yml` and `scripts/bench/` provide a foundation; extend rather than replace
- OpenClaw baseline version must be documented in `scripts/bench/baseline.json`
- Dedicated CI runner avoids measurement noise from shared runners
- Peak RSS measurement: `/usr/bin/time -v` on Linux (look for "Maximum resident set size")
- Throughput: sustained 60s run through the agent loop with mock LLM responses
- Skills are ported after C3 lands; scheduling for Week 10-12 assumes C3 is stable

**Exit criteria**:
- [x] Benchmark suite produces comparison report with all 4 metrics against OpenClaw baseline
- [ ] 3 MVP skills (`coding-agent`, `web-search`, `file-management`) pass benchmark suite (depends on C3 landing)
- [x] Benchmark results are reproducible across runs on same hardware
- [x] Results published as CI artifact

---

## 3. Dependency Map

```
Element 04 (C2 WASM host) ──────────────┐
Element 04 (C3-C4 skill loader) ────────┤
Element 08 (H2 vector search) ──────────┤
                                         v
                            ┌─── Unit 1: K2 Docker + CI/CD (Week 8-9)
                            │         (no hard deps on above, can start immediately)
                            │
Element 10 ─────────────────┼─── Unit 2: K3+K3a Sandbox + Security (Week 9-11)
                            │         depends on: C2 (WASM host)
                            │
                            ├─── Unit 3: K4 ClawHub (Week 10-12)
                            │         depends on: C3, C4 (skills), H2 (vector search)
                            │
                            └─── Unit 4: K5 Benchmarks (Week 10-12)
                                      depends on: C3 (skills ported)
```

**Parallel execution**: Unit 1 (K2) has no dependencies on other elements and can begin at Week 8. Units 2-4 depend on plugin/memory infrastructure from earlier elements.

---

## 4. Risk Mitigation

| Risk | Score | Mitigation |
|------|-------|------------|
| K3a 50+ audit checks in 1 week is aggressive | 6 | Create categorized check inventory early (Week 7 prep); prioritize P0 categories first; defer lowest-priority to post-sprint |
| K4 ClawHub tight timeline (2 weeks) | 8 | API contract defined early (Contract #20); H2 must land by Week 9; fallback: keyword-only search if vector not ready |
| K5 benchmark results non-reproducible | 6 | Pin OpenClaw baseline version; dedicated CI runner; document hardware specs |
| seccomp/landlock Linux-only | 4 | WASM sandbox is cross-platform; document macOS as WASM-only; native tool sandbox deferred for non-Linux |
| CI/CD pipeline delays | 4 | K2-CI starts Week 8 (first item); PR gates added incrementally; Docker build is well-understood pattern |
| ClawHub supply chain attack | 8 | Mandatory code signing; content hash verification; auto security scan on install; unsigned rejected by default |
| Docker image exceeds 50MB | 2 | Multi-stage build with cargo-chef; strip binaries; CI assertion |

---

## 5. Cross-Element Integration Tests

From the cross-element integration specification:

| Test | Elements | Week | Priority |
|------|----------|------|----------|
| Agent Routing -> Sandbox | 09, 10 | 10 | P1 |
| Vector Search -> ClawHub Discovery | 08, 10 | 11 | P1 |
| ClawHub Install -> Security Scan | 10, 04 | 11 | P1 |

Tests live in `tests/integration/cross_element/`.

---

## 6. Crate Impact Summary

| Crate | Changes | Units |
|-------|---------|-------|
| `Dockerfile` (root) | Replace with multi-stage build | Unit 1 |
| `.github/workflows/` | New PR gates, release pipeline; extend benchmarks | Units 1, 4 |
| `scripts/deploy/` | New VPS deployment scripts | Unit 1 |
| `clawft-plugin/src/sandbox.rs` | New `SandboxPolicy` type | Unit 2 |
| `clawft-wasm/` | Extend WASM sandbox capabilities | Unit 2 |
| `clawft-core/src/agent/` | New sandbox enforcement, extend skill auto-search | Units 2, 3 |
| `clawft-security/` | New crate: audit checks, hardening, monitors, signing | Units 2, 3 |
| `clawft-cli/src/commands/` | New `security.rs`; extend `skill.rs` | Units 2, 3, 4 |
| `clawft-services/src/clawhub/` | New module: registry API, search, community | Unit 3 |
| `benches/`, `scripts/bench/` | Extend benchmark suite | Unit 4 |

---

## 7. Definition of Done

All exit criteria from the orchestrator must pass:

- [x] Docker image builds and runs on amd64 and arm64 (<50MB compressed)
- [x] CI/CD pipeline enforces PR gates (clippy, test, WASM size, binary size)
- [x] Release pipeline publishes multi-arch images to GHCR on tag
- [x] Integration smoke test passes in CI (gateway start + health check)
- [x] Per-agent sandbox enforces tool restrictions
- [x] Default sandbox type is NOT `None` (WASM for plugins, OsSandbox on Linux)
- [x] `weft security scan` runs 50+ audit checks across 8+ categories
- [x] ClawHub requires skill signatures for publication
- [x] Platform-specific sandbox fallback documented for non-Linux
- [ ] `weft skill install` works from ClawHub registry (server-side pending)
- [x] Benchmark suite produces comparison report with all 4 metrics against OpenClaw baseline
- [ ] 3 MVP skills pass benchmark suite (depends on C3 landing)
- [x] All existing tests pass
