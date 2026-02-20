# Element 10 Implementation Notes

## K2: Docker & CI/CD

### Dockerfile
- Replaced `FROM scratch` with multi-stage `debian:bookworm-slim` build
- 4 stages: chef (tooling) -> planner (recipe) -> builder (compile) -> runtime (minimal)
- Uses `cargo-chef` for dependency caching (rebuild only when Cargo.toml changes)
- Runtime image includes only: binary + ca-certificates + libssl3 + libgcc-s1
- Non-root user (`weft`) for security
- HEALTHCHECK directive for container orchestration
- Multi-arch support via `docker buildx` (linux/amd64, linux/arm64)

### CI/CD Workflows
- `pr-gates.yml`: 5 jobs (clippy, test, wasm-size, binary-size, smoke-test)
  - Concurrency group cancels in-progress runs for same PR
  - Binary size assertion: <10MB for release binary
  - WASM size gate: <300KB raw, <120KB gzipped
  - Integration smoke test: builds Docker image, starts container, verifies it runs
- `release.yml`: Triggered on semver tags (v*.*.*)
  - Multi-arch build via QEMU + buildx
  - Pushes to GHCR with semantic version tags (major, major.minor, version, latest)
  - Post-publish smoke test pulls and runs the image

### Deployment Scripts
- `scripts/deploy/vps-deploy.sh`: One-click deployment with configurable options
- `scripts/deploy/docker-compose.yml`: Compose file for quick setup

## K3: Per-Agent Sandbox

### SandboxPolicy (clawft-plugin/src/sandbox.rs)
- Comprehensive policy struct with network, filesystem, process, env sub-policies
- Domain matching supports exact, wildcard (*.example.com), and star (*)
- Blocked lists take precedence over allowed lists
- Default sandbox type is NOT None: OsSandbox on Linux, Wasm on other platforms
- `effective_sandbox_type()` handles platform-specific fallback with logging

### SandboxEnforcer (clawft-core/src/agent/sandbox.rs)
- Wraps SandboxPolicy with enforcement methods (check_tool, check_network, etc.)
- In-memory audit log with ring buffer (max 10,000 entries, halves when full)
- All decisions logged: allowed at DEBUG, denied at WARN
- `apply_os_sandbox()` dispatches to platform-specific implementation
- Linux: seccomp + landlock framework (kernel filter installation via crates)
- Non-Linux: WASM-only fallback with warning

### Audit Logging
- SandboxAuditEntry records: timestamp, agent_id, action, target, allowed, reason
- Enforcer maintains thread-safe audit log via Arc<Mutex<Vec>>

## K3a: Security Plugin

### clawft-security crate
- 55 audit checks across 10 categories (exceeds 50+ requirement)
- Categories: PromptInjection (8), ExfiltrationUrl (6), CredentialLiteral (6),
  PermissionEscalation (6), UnsafeShell (6), SupplyChainRisk (6),
  DenialOfService (6), IndirectPromptInjection (5), InformationDisclosure (4),
  CrossAgentAccess (4)
- All patterns use Rust `regex` crate (no backreferences or lookahead)
- SecurityScanner with scan_content() and scan_report() methods
- AuditReport tracks findings by severity, determines pass/fail

### CLI Integration
- `weft security scan <path>` -- scans files/directories
- `weft security checks` -- lists all available checks
- Supports --format json/text and --min-severity filtering
- Skips binary files, hidden dirs, target/, node_modules/
- Max file size: 1MB per file

## K4: ClawHub Registry

### REST API Contract (Contract #20)
- ApiResponse<T> wrapper: { ok, data, error, pagination }
- SkillEntry: id, name, description, version, author, stars, content_hash, signed
- PublishRequest: requires signature by default (--allow-unsigned for dev only)
- ClawHubClient with search/publish/install methods (server stubs)

### Search
- keyword_search() with scoring: name (2x), description (1x), tags (1.5x)
- Normalized scores (0.0-1.0)
- Vector search integration point ready (when H2 lands)

### Community
- CommunityStore with star/comment/version management
- Rating clamped to 1-5
- Moderation flag on comments

## K5: Benchmarks

### Comparison Script
- scripts/bench/compare.sh measures 4 metrics:
  1. Binary size (wc -c)
  2. Cold start (hyperfine --warmup 0)
  3. Peak RSS (/usr/bin/time -v)
  4. Throughput (criterion, run separately)
- Outputs bench-comparison.json
- Reads baseline from scripts/bench/baseline.json
