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

### K4 Install/Publish (Phase 5 addition)

#### Ed25519 Signing Module (`clawft-core/src/security/signing.rs`)
- `security.rs` converted to `security/mod.rs` with `pub mod signing` behind `signing` feature
- `ed25519-dalek = "2"` added to workspace deps
- `signing` feature in clawft-core: `["dep:ed25519-dalek", "dep:sha2", "dep:rand"]`
- `SkillContentHash`: SHA-256 over sorted, length-prefixed file contents
- `SkillSignature`: hex-encoded Ed25519 signature + public key + algorithm
- `generate_keypair()`: creates Ed25519 key pair, hex-encoded, 0o600 permissions
- `compute_content_hash()`: deterministic hash over all non-hidden files
- `sign_content()` / `verify_signature()`: Ed25519 sign and verify
- `load_signing_key()` / `load_public_key()`: load from `keys_dir`
- 8 tests behind `#[cfg(all(test, feature = "signing"))]`

#### ClawHub Client (real HTTP)
- Replaced stubs in `registry.rs` with `reqwest::Client` calls
- `ClawHubError` enum: Http, ServerError, ParseError, ApiError, Unreachable, Io
- `search()`: GET `/skills/search?q=...&limit=...&offset=...`
- `publish()`: POST `/skills/publish` (JSON body)
- `download()`: GET `/skills/{id}/download` (raw bytes)
- `install()`: download + write to disk
- `ClawHubConfig::from_env()`: reads `CLAWHUB_API_URL` and `CLAWHUB_API_TOKEN`
- Graceful error handling: connection refused = descriptive error, not panic
- `clawhub` feature gate added to clawft-services (module always compiled)

#### CLI Subcommands (4 new)
- `weft skills search <query> [--limit N]`: search ClawHub, display results in table
- `weft skills publish <path> [--allow-unsigned]`: parse SKILL.md, hash, sign, POST
- `weft skills remote-install <name> [--allow-unsigned]`: search, download, install
- `weft skills keygen`: generate Ed25519 key pair at `~/.clawft/keys/`
- Standalone helpers: base64 encode, hex decode, FNV hash, YAML frontmatter parser
- All gated behind `#[cfg(feature = "services")]` with fallback error messages

#### Test Summary (K4)
- 8 signing tests (clawft-core, signing feature)
- 12 registry tests (clawft-services)
- 8 CLI parsing tests (clawft-cli main.rs)
- 10 CLI unit tests (skills_cmd.rs -- frontmatter, base64, hex, hash)
- Total: 38 new tests

## K5: Benchmarks

### Comparison Script
- scripts/bench/compare.sh measures 4 metrics:
  1. Binary size (wc -c)
  2. Cold start (hyperfine --warmup 0)
  3. Peak RSS (/usr/bin/time -v)
  4. Throughput (criterion, run separately)
- Outputs bench-comparison.json
- Reads baseline from scripts/bench/baseline.json

## K5: MVP Skills

### Overview
3 MVP skills adapted from OpenClaw references, implemented as SKILL.md files
in the `skills/` workspace directory. All skills parse correctly through the
`skills_v2::parse_skill_md` parser and are discoverable by `SkillRegistry::discover`.

### Skills Delivered

#### 1. `skills/prompt-log/SKILL.md`
- **Adapted from**: openclaw/thesash/prompt-log
- **Purpose**: Extract conversation transcripts from clawft `.jsonl` session logs
- **Tools**: Read, Write, Bash, Glob
- **Variables**: session_file, output_path
- **Features**: Timestamp filtering (--after/--before), chunked reading for large
  files, tool call summarization, Markdown transcript output

#### 2. `skills/skill-vetting/SKILL.md`
- **Adapted from**: openclaw/eddygk/skill-vetting
- **Purpose**: Security vetting of third-party skills before installation
- **Tools**: Read, Bash, Glob, Grep
- **Variables**: skill_path
- **Features**: 6-step vetting workflow, integrates `weft security scan` (57 audit
  checks), manual review checklist, decision matrix (APPROVE/REVIEW/REJECT),
  structured vetting report output

#### 3. `skills/discord/SKILL.md`
- **Adapted from**: openclaw/steipete/discord
- **Purpose**: Discord bot control through clawft's channel adapter
- **Tools**: Bash
- **Variables**: action, channel_id
- **Features**: 8 action types (send, react, thread, poll, pin, search, moderation,
  status), channel ID resolution, rate limit handling, safety rules for
  moderation actions

### Tests Added (4)
All tests in `crates/clawft-core/src/agent/skills_v2.rs`:

1. `parse_prompt_log_skill` -- Verifies prompt-log SKILL.md parses with correct
   name, version, tools, variables, and instruction content
2. `parse_skill_vetting_skill` -- Verifies skill-vetting SKILL.md parses correctly
3. `parse_discord_skill` -- Verifies discord SKILL.md parses correctly
4. `discover_mvp_skills_from_directory` -- Verifies `SkillRegistry::discover`
   finds all 3 skills when pointed at the `skills/` directory

### File Map
| File | Action | Status |
|------|--------|--------|
| skills/prompt-log/SKILL.md | NEW | DONE |
| skills/skill-vetting/SKILL.md | NEW | DONE |
| skills/discord/SKILL.md | NEW | DONE |
| crates/clawft-core/src/agent/skills_v2.rs | EDITED (4 tests) | DONE |
