# Review: Deployment & Integration Readiness (Iteration 1)

> **Reviewer**: Deployment & Integration Expert
> **Date**: 2026-02-19
> **Scope**: Element 10 (Deployment & Community), Master Orchestration Plan, Cross-element integration
> **Source docs reviewed**:
> - `.planning/sparc/10-deployment-community/00-orchestrator.md`
> - `.planning/sparc/02-improvements-overview/00-orchestrator.md`
> - `.planning/drafts/01-biz-features-deploy.md`
> - `.planning/drafts/02-tech-pipeline.md`
> - `.planning/drafts/03-dev-phases.md`
> - `.planning/drafts/03-dev-contracts.md`
> - `.planning/drafts/review-1-devguide.md`
> - `.planning/improvements.md`

---

## 1. Element 10 Assessment: Deployment & Community

**Score: 6/10**

### 1.1 Docker: Multi-arch Build Strategy (K2)

**Findings**:
- The orchestrator specifies `linux/amd64` and `linux/arm64` as target architectures. This is correct for the stated deployment targets (self-hosting, VPS).
- **Missing: Base image specification.** No document specifies the base Docker image. For a Rust workspace producing static binaries, the choice between `debian:bookworm-slim`, `alpine`, or `scratch`/`distroless` has significant implications for image size, security surface, and WASM plugin compatibility (glibc vs musl for `wasmtime`). Recommendation: specify `debian:bookworm-slim` as default (glibc compatibility for wasmtime) with a `scratch` variant for minimal deployments where WASM is not needed.
- **Missing: Layer optimization strategy.** No mention of multi-stage builds, cargo-chef for dependency caching, or final image size targets. For a project targeting `<10 MB base binary`, the Docker image should target `<50 MB` compressed. Without explicit layer strategy, CI builds will be slow and images bloated.
- **Missing: Registry target.** Where are images published? Docker Hub? GitHub Container Registry? Self-hosted? The success criterion says "Docker image published" but the target registry is unspecified.
- One-click VPS scripts are mentioned but not specified. What VPS providers? What provisioning tool (cloud-init, docker-compose, shell script)? The biz doc says "zero to running in under 5 minutes" but the tech spec is absent.

### 1.2 CI/CD Pipeline (Gap)

**Critical finding: No CI/CD pipeline is defined anywhere in the planning documents.**

The deployment element assumes Docker images will be built and published, but:
- No GitHub Actions workflow is specified for multi-arch builds (`docker buildx`)
- No CI pipeline for running `cargo test --workspace`, `cargo clippy`, and `cargo build --release` before merge
- No CD pipeline for image publishing on release tags
- The branching strategy in `03-dev-phases.md` defines 11 stream branches but no merge/CI gates
- The Definition of Done says "binary size regression checked" but no CI job enforces this

This is a significant gap for an element that must deliver publishable artifacts. Recommendation: Add a CI/CD specification as a sub-item of K2, covering:
1. PR gates: `cargo clippy`, `cargo test`, WASM size assertion
2. Release pipeline: multi-arch Docker build, image push, release notes
3. Integration test pipeline: smoke test the Docker image (start gateway, verify health endpoint)

### 1.3 Benchmarks: OpenClaw Benchmark Suite (K5)

**Findings**:
- The improvements.md defines K5 as: "Feature parity test suite. Performance comparison (binary size, cold start, memory, throughput)."
- The biz doc success criterion: "Benchmark suite produces comparison table: clawft vs OpenClaw on binary size, cold start, RSS, throughput."
- **Missing: Technical specification for the benchmark harness.** There is no document specifying:
  - How benchmarks are run (Criterion.rs? Custom harness? Shell script with `hyperfine`?)
  - What "feature parity" means concretely (which OpenClaw features are compared?)
  - How "cold start" is measured (process startup to first LLM call? To first tool response?)
  - What "throughput" means (messages per second? Tool calls per second? Tokens per second?)
  - How RSS is measured (peak RSS? Average RSS under load?)
  - What the baseline OpenClaw version is
  - Whether benchmarks run in CI or are manual
- This is a known issue (flagged in the task brief as "Missing tech spec for K5 benchmark harness"). The gap remains unaddressed.
- Recommendation: Create a K5 tech spec defining benchmark methodology before Week 10 when K5 is scheduled to start. Without it, the benchmark results will be non-reproducible and not comparable.

### 1.4 ClawHub: Skill Registry Architecture (K4)

**Findings**:
- ClawHub is described across multiple documents with reasonable consistency:
  - HTTP index + git clone backend
  - Vector search for discovery (depends on H2)
  - Star/comment system, versioning (semver)
  - Agent auto-search when no local skill matches
  - `weft skill install`, `weft skill publish`, `weft skill search` CLI commands
- **Architecture is directionally clear but lacks protocol specification.** The review-1-devguide.md correctly identifies this: "The API format (REST endpoints, response schema, search semantics) is not specified as a contract." This means:
  - No HTTP API spec (REST endpoints, request/response schemas)
  - No authentication model for publishing (how are publishers verified?)
  - No storage backend specified (is this a static git repo? A proper registry service?)
  - No moderation workflow specified beyond "moderation hooks for quality control"
- The dependency chain (C3 + C4 + H2 -> K4) is correct. K4 cannot start until the skill loader and vector store are functional. This puts K4 at Week 10-11, leaving only 1-2 weeks for implementation and testing. This is tight.
- **Missing: Self-hosting specification.** The non-goals say "self-hostable HTTP index + git backend" but no deployment instructions for the registry itself are planned.

### 1.5 Security Plugin (K3, K3a)

**Findings**:
- The security plugin architecture is well-specified in the contracts draft (Contract #19: AuditCheck trait with severity levels and finding structure).
- K3 (sandbox) depends on C2 (WASM host), correctly capturing that WASM isolation must exist before per-agent sandboxing can use it.
- K3a depends on C1 + K3, forming a correct chain: traits -> WASM sandbox -> security checks.
- The "50+ audit checks" number is ambitious. No check inventory exists. Recommendation: Create a categorized check list (prompt injection patterns, exfiltration URLs, credential literals, permission escalation, etc.) to validate that 50 is achievable and identify gaps.
- seccomp/landlock integration is Linux-specific. No mention of macOS (sandbox-exec) or Windows equivalent. If Docker images are the deployment target, this is fine (Linux containers). But for local development/testing on macOS (common for Rust developers), the sandbox tests need a fallback or skip strategy.

### 1.6 Element 10 Summary

| Sub-item | Spec Quality | Implementability | Risk |
|----------|-------------|-----------------|------|
| K2 Docker | Weak -- missing base image, layers, CI/CD | Medium -- straightforward Rust Docker builds | Medium |
| K3 Sandbox | Good -- contract defined, deps clear | Medium -- seccomp/landlock requires Linux expertise | Medium |
| K3a Security | Good -- trait and severity model defined | High risk -- 50+ checks in 1 week is aggressive | High |
| K4 ClawHub | Moderate -- features clear, protocol missing | Low -- no API spec means implementation will drift | High |
| K5 Benchmarks | Weak -- no methodology specified | Low -- without spec, results non-comparable | Medium |

---

## 2. Orchestration Plan Assessment

**Score: 7.5/10**

### 2.1 Strengths

1. **Dependency graph is well-structured.** The critical path identification (A -> B3 -> C1 -> C2-C4 -> feature work -> K) is correct and documented in both the phases draft and contracts draft.

2. **Parallel group design is sound.** The four parallel groups (Group 1: fixes, Group 2: plugin foundation, Group 3: feature buildout, Group 4: deployment) allow meaningful parallelism while respecting dependency gates.

3. **C1 correctly identified as the central dependency hub.** Both drafts recognize that `clawft-plugin` trait crate gates virtually all feature work. This awareness is critical for prioritization.

4. **Interface contracts are well-defined.** Contracts #13-#19 provide concrete type signatures, wire formats, and behavior specifications. This reduces integration risk.

5. **Risk mitigation is comprehensive.** Between the two draft documents, 17+ risks are identified with detection criteria and response strategies.

### 2.2 Weaknesses

1. **No integration testing strategy across elements.** Each element defines its own validation criteria, but there is no specification for end-to-end integration tests that verify elements work together. For example:
   - Does a plugin loaded via C4 work correctly through the agent routing table (L1)?
   - Does ClawHub install (K4) of a skill picked up by hot-reload (C4) and used through MCP (C6)?
   - Does the FlowDelegator (M1) work with per-agent isolation (L2)?

2. **Merge coordination underspecified.** 11 concurrent stream branches with 4+ conflict zones (identified in contracts draft) but no merge cadence, no integration branch CI, and no specification of merge order within the dependency graph.

3. **Resource allocation absent.** The plan assumes unlimited parallelism but does not address team size, skill distribution, or workload balancing. If the team is 1-3 people, the 4-group parallel design collapses into largely sequential work.

4. **Forward-compatibility hooks are specified but not testable.** The voice and UI hooks (VoiceHandler placeholder, binary payload support, stable MCP schemas) are requirements but no test verifies they actually work. A future voice implementation could find the hooks insufficient. Recommendation: Add at minimum a compile-time test that instantiates VoiceHandler and a runtime test that sends a binary payload through ChannelAdapter.

---

## 3. Cross-Element Integration Gaps

### 3.1 Known Issues (from task brief)

| Issue | Status | Severity | Detail |
|-------|--------|----------|--------|
| F9 missing from MVP milestone | Confirmed | MEDIUM | `improvements.md` MVP includes "MCP client for external servers" but `03-dev-phases.md` places F9 at Week 10, past MVP at Week 8. The dev guide MVP checklist omits F9 entirely. |
| Phase numbering inconsistency | Confirmed | HIGH | Biz docs use Phase 1.5/2/3G+/4/5/6. Dev guide uses Phase 5 for all sprint work. `review-1-devguide.md` confirms this and rates it HIGH severity. All documents must standardize. |
| Missing tech spec for K5 benchmark harness | Confirmed | MEDIUM | No benchmark methodology exists. See Section 1.3 above. |
| Cross-cutting concerns not captured | Partially confirmed | MEDIUM | "Offline capability" and "no core forks post-C" are stated in improvements.md but not enforced via tests or CI. No offline-mode integration test exists. |
| Forward-compat for Voice and UI needs tech specs | Confirmed | LOW | Hooks are specified declaratively but no verification tests are planned. Risk is low because hooks are simple (trait placeholders, feature flags) but should still be tested. |

### 3.2 Additional Integration Gaps Identified

**Gap 1: K3 sandbox <-> L2 per-agent isolation interaction**

K3 defines WASM + seccomp/landlock sandboxing. L2 defines per-agent workspace isolation with independent tool restrictions. These two systems must cooperate: the sandbox enforces the restrictions that L2 configures. But no document specifies how the per-agent config (`~/.clawft/agents/<id>/config.toml`) translates into sandbox policy. Does each agent get its own WASM instance? Its own seccomp profile? This integration point is undefined.

**Gap 2: K4 ClawHub <-> K3a security plugin interaction**

The biz doc says "Mandatory security scan on install" for ClawHub skills. But no document specifies the interaction between `weft skill install` (K4) and `weft security scan` (K3a). Is scanning automatic? Can it be bypassed? What severity level blocks installation? The contracts draft defines the AuditCheck trait but not the integration with the install workflow.

**Gap 3: M4 dynamic MCP <-> F9 MCP client dependency resolution**

`improvements.md` shows M4 depends on F9. `03-dev-phases.md` places M4 at Week 5 and F9 at Week 10. `review-1-devguide.md` flags this as MEDIUM severity. The suggested resolution (M4 CLI can be built independently, F9 transport integrates later) is reasonable but not documented as a decision. This needs to be formally captured as a phased implementation plan.

**Gap 4: H2 vector store <-> K4 ClawHub vector search integration**

K4 uses H2's vector store for semantic skill discovery. H2.1 (HNSW) is scheduled for Week 5 but H2.2 (production embedder) is also Week 5. K4 starts Week 10-11. The gap is: what embedding model does ClawHub use for skill descriptions? Is it the same as the agent's memory embedding? If the user has not configured a production embedder (H2.2), does ClawHub search fall back to keyword search? This fallback path is unspecified.

**Gap 5: F6 OAuth2 <-> E5a Google Chat dependency ordering**

Confirmed from `review-1-devguide.md`: F6 is at Week 8, E5a is at Week 7. E5a depends on F6. The OAuth2 helper must be available before Google Chat can authenticate. This ordering violation needs correction.

---

## 4. MVP Milestone Feasibility Assessment (Week 8)

### 4.1 MVP Definition (from improvements.md)

The MVP milestone requires:
1. Plugin system with skill precedence + hot-reload (C1-C4)
2. Email channel (E2)
3. Multi-agent routing (L1)
4. 3 ported OpenClaw skills
5. MCP client for external servers (F9)
6. Claude Flow integration: FlowDelegator + dynamic MCP + delegate feature enabled (M1-M4)
7. All critical/high fixes resolved (A1-A9)

### 4.2 Feasibility Analysis

| MVP Item | Week Required By | Week Available | Float | Risk |
|----------|-----------------|----------------|-------|------|
| A1-A9 fixes | 8 | 2 (scheduled) | +6 weeks | Low |
| B3 file splits | 4 (gates C1) | 3-4 (scheduled) | 0 weeks | Medium -- 9 files to split |
| C1 plugin traits | 4 (gates all features) | 3-4 (scheduled) | 0 weeks | HIGH -- single point of failure |
| C2 WASM host | 6 (gates C4) | 4-6 (scheduled) | 0 weeks | Medium |
| C3 skill loader | 5 (gates C4) | 4-5 (scheduled) | 0 weeks | Low |
| C4 hot-reload | 7 (gates MVP skill test) | 6-7 (scheduled) | 0 weeks | Medium |
| E2 email channel | 8 | 5 (scheduled) | +3 weeks | Low |
| L1 routing table | 8 | 5 (scheduled) | +3 weeks | Low |
| M1 FlowDelegator | 8 | 3 (scheduled) | +5 weeks | Low -- separate chain |
| M3 delegate default | 8 | 4 (scheduled) | +4 weeks | Trivial |
| M4 dynamic MCP | 8 | 5-6 (scheduled) | +2 weeks | Medium -- M4/F9 dep unclear |
| F9 MCP client | 8 (per improvements.md) | 10 (scheduled) | **-2 weeks** | **HIGH -- infeasible at MVP** |
| 3 OpenClaw skills | 8 | Unscheduled | Unknown | **MEDIUM -- no schedule** |

### 4.3 Key Findings

1. **F9 cannot make MVP at Week 8.** It is scheduled for Week 10 and depends on D9 (Week 4). Even with acceleration, implementing a full MCP client with auto-discovery, connection pooling, tool schema caching, and health checks in parallel with everything else is unrealistic by Week 8. **Recommendation**: Either remove F9 from the MVP definition or define a minimal F9 (connect to a single configured MCP server, list tools, invoke) that can land by Week 8, with the full feature at Week 10.

2. **C1 is the zero-float critical gate.** If C1 slips by even 1 week, C2/C3 slip, which slips C4, which puts the entire plugin system at risk for MVP. See Section 5 for slip analysis.

3. **"3 ported OpenClaw skills" has no schedule or specification.** Which skills? When do they start? Who ports them? These should be named explicitly (e.g., "coding-agent skill, web-search skill, file-management skill") and assigned to a specific week after C3 lands (Week 5-6).

4. **The MVP is ambitious but largely feasible** if: (a) F9 is scoped down or removed, (b) C1 lands on time, and (c) the 3 OpenClaw skills are named and scheduled. The critical path has zero float from A through C4, meaning any delay on the plugin infrastructure directly threatens the milestone.

---

## 5. Critical Path Risk Analysis

### 5.1 Critical Path

```
A (W1-2) -> B3 (W2-3) -> C1 (W3-4) -> C2+C3 (W4-6) -> C4 (W6-7) -> K (W8-12)
```

Total critical path: 12 weeks. Zero float on the full path.

### 5.2 What Happens If C1 Slips by 1 Week?

If C1 completes at end of Week 5 instead of Week 4:

| Item | Original | Slipped | Impact |
|------|----------|---------|--------|
| C1 | W3-4 | W3-5 | +1 week |
| C2 (WASM host) | W4-6 | W5-7 | +1 week, now overlaps with C4's slot |
| C3 (skill loader) | W4-5 | W5-6 | +1 week |
| C4 (dynamic load) | W6-7 | W7-8 | +1 week, now at MVP boundary |
| E2 (email) | W5 | W6 | +1 week (needs C1), still within float |
| L1 (routing) | W5 | W6 | +1 week (needs C1 + B5), still within float |
| Hot-reload verification | W7 | W8 | At MVP boundary -- no margin |
| K (deployment) | W8-12 | W9-12 | -1 week for all K items |
| K4 (ClawHub) | W10-11 | W11-12 | Extremely tight, risk of incomplete |
| K5 (benchmarks) | W12 | W12-13 | **Slips past sprint end** |

**Cascade effect**: A 1-week slip on C1 does not break the MVP (barely), but it compresses K from 4 weeks to 3 weeks and likely pushes K5 benchmarks out of the sprint entirely. K4 ClawHub goes from tight (2 weeks) to extremely tight (1 week). K3a security plugin (Week 10) shifts to Week 11, leaving 1 week for 50+ audit checks.

**Mitigation**: Pre-draft C1 trait signatures during Week 2 (alongside B3 file splits). Allow downstream elements (E2, F1, L1) to code against draft traits, finalizing when C1 merges. This is already suggested in the contracts draft ("E and F can prototype against C1 trait drafts before C1 merges").

### 5.3 Are Parallel Workstreams Truly Independent?

| Workstream Pair | Independent? | Conflict Zone |
|----------------|-------------|---------------|
| A (fixes) vs D (pipeline) | Yes | Minor: both touch `loop_core.rs` but B3 splits it first |
| D (pipeline) vs M (Claude Flow) | Partially | D9 (MCP transport) gates M1 (FlowDelegator). Not independent. |
| E (channels) vs F (dev tools) | Yes | Both depend on C1 but no mutual dependency |
| L (routing) vs H (memory) | Partially | L2 depends on H1. Not fully independent. |
| I (type safety) vs everything | Yes | Truly independent -- small targeted patches |
| J (docs) vs everything | Yes | Follows feature completion, no code conflict |

The parallel groups are mostly sound, but two dependency chains (D9->M1 and H1->L2) create hidden serialization within nominally parallel groups. These are correctly captured in the contracts draft but the phases draft's "parallel group" framing slightly oversells the parallelism.

### 5.4 Schedule Buffer Analysis

| Element | Scheduled Duration | Estimated Effort | Float |
|---------|-------------------|-----------------|-------|
| 03 (fixes/cleanup) | 5 weeks (W1-5) | 3 weeks | +2 weeks |
| 04 (plugin system) | 6 weeks (W3-8) | 6 weeks | 0 weeks |
| 05 (pipeline) | 4 weeks (W2-5) | 3 weeks | +1 week |
| 06 (channels) | 5 weeks (W4-8) | 4 weeks | +1 week |
| 07 (dev tools) | 6 weeks (W5-10) | 5 weeks | +1 week |
| 08 (memory) | 5 weeks (W4-8) | 5 weeks | 0 weeks |
| 09 (multi-agent) | 5 weeks (W5-9) | 4 weeks | +1 week |
| 10 (deployment) | 5 weeks (W8-12) | 5 weeks | 0 weeks |

Elements with zero float: 04 (plugin system), 08 (memory/workspace), and 10 (deployment). These three are the highest schedule risk. The plugin system (04) is particularly concerning because it gates all other feature elements.

---

## 6. Specific Recommendations

### 6.1 Critical (Must Address Before Execution)

1. **Add CI/CD pipeline specification to K2.** Define GitHub Actions workflows for: PR gates (clippy, test, WASM size), release pipeline (multi-arch Docker build, image push), integration smoke test. Without CI/CD, the deployment element cannot deliver its primary artifact.

2. **Resolve F9 MVP inclusion.** Either: (a) Define a "minimal F9" that connects to a single configured MCP server by Week 8 with full auto-discovery at Week 10, or (b) remove F9 from the MVP definition and update `improvements.md` accordingly. The current state is contradictory.

3. **Standardize phase numbering across all documents.** All biz requirement drafts must use "Phase 5" for the unified sprint. This is rated HIGH by the devguide review and affects every document.

4. **Fix F6/E5a dependency ordering.** Move F6 (OAuth2 helper) to Week 6-7 or move E5a (Google Chat) to Week 8+.

5. **Name and schedule the 3 OpenClaw skills for MVP.** These are MVP criteria but have no schedule, no names, and no owner.

### 6.2 Important (Should Address)

6. **Specify Docker base image and layer strategy.** Recommend `debian:bookworm-slim` with multi-stage build using `cargo-chef` for dependency caching. Document image size target (`<50 MB` compressed).

7. **Create K5 benchmark methodology spec.** Define: benchmark harness (recommend `criterion` + `hyperfine`), specific metrics with measurement methodology, baseline OpenClaw version, CI integration plan.

8. **Add integration test plan for cross-element interactions.** At minimum define end-to-end tests for: plugin install -> hot-reload -> MCP exposure, agent routing -> sandbox enforcement, FlowDelegator -> per-agent isolation.

9. **Specify K4 ClawHub HTTP API contract.** Before implementation starts (Week 10), the API endpoints, authentication model, and response schemas should be defined as Contract #20.

10. **Document K3/L2 sandbox-routing integration.** How per-agent config translates to sandbox policy needs a specification.

11. **Add forward-compatibility verification tests.** Even simple compile-time tests for VoiceHandler instantiation and binary payload ChannelAdapter would catch breakage early.

### 6.3 Minor (Nice to Have)

12. **Add sandbox fallback for non-Linux platforms.** seccomp/landlock are Linux-only. Document that sandbox tests skip on macOS/Windows, or provide a WASM-only fallback sandbox for non-Linux.

13. **Define ClawHub self-hosting deployment guide.** If ClawHub is "self-hostable HTTP index + git backend," a minimal deployment guide should be in scope.

14. **Add M4/F9 phased implementation decision record.** Document that M4 CLI builds independently while F9 transport integrates later, so the apparent dependency conflict is resolved.

---

## 7. Development Readiness Assessment

### 7.1 Readiness by Element

| Element | Readiness | Blockers |
|---------|-----------|----------|
| 03 (fixes/cleanup) | READY | None -- well-specified, independent |
| 04 (plugin system) | READY | None -- contracts defined, zero-float schedule is the concern |
| 05 (pipeline) | READY | None -- tech specs in 02-tech-pipeline.md are implementation-ready |
| 06 (channels) | CONDITIONAL | F6/E5a ordering must be fixed first |
| 07 (dev tools) | CONDITIONAL | F9 MVP scope must be clarified |
| 08 (memory) | READY | None -- H2 sub-items well-defined |
| 09 (multi-agent) | READY | None -- contracts and config format defined |
| 10 (deployment) | NOT READY | Missing: CI/CD spec, Docker details, K5 methodology, K4 API spec |

### 7.2 Overall Verdict

**CONDITIONAL GO**

The sprint plan is well-structured with strong architectural thinking, comprehensive risk identification, and sound dependency management. The majority of elements (03, 04, 05, 08, 09) are development-ready with clear contracts and specifications.

However, Element 10 (Deployment & Community) is the weakest element in the plan and requires additional specification work before execution can begin on K2, K4, and K5. The CI/CD gap affects the entire sprint, not just Element 10 -- without CI gates, the 11-branch development strategy has no automated quality enforcement.

**Conditions for GO**:

1. Add CI/CD pipeline specification (can be done in Week 1, before K2 starts at Week 8)
2. Resolve F9 MVP scope (immediate decision needed)
3. Fix F6/E5a ordering (immediate fix in dev phases doc)
4. Standardize phase numbering (can be done in parallel with execution)
5. Name the 3 MVP OpenClaw skills and add to schedule

All five conditions can be resolved within the first 1-2 weeks of sprint execution without delaying the critical path (which starts with A fixes and B3 splits). Element 10 specification work (Docker details, K5 methodology, K4 API spec) can proceed in parallel during Weeks 3-7 before K work begins at Week 8.

**The sprint should proceed** with the understanding that Element 10 specifications will be completed before their execution window opens.

---

## Appendix: Issue Tracker

| # | Severity | Issue | Source | Recommendation |
|---|----------|-------|--------|----------------|
| 1 | HIGH | Phase numbering inconsistency across all docs | review-1-devguide.md | Standardize on "Phase 5" |
| 2 | HIGH | No CI/CD pipeline defined | This review | Add CI/CD spec to K2 |
| 3 | MEDIUM | F9 missing from MVP milestone in dev guide | review-1-devguide.md | Scope down F9 for MVP or remove |
| 4 | MEDIUM | F6/E5a dependency ordering violation | review-1-devguide.md | Move F6 to Week 6-7 |
| 5 | MEDIUM | K5 benchmark harness has no tech spec | Task brief + this review | Create methodology spec |
| 6 | MEDIUM | K4 ClawHub has no API contract | review-1-devguide.md + this review | Add Contract #20 |
| 7 | MEDIUM | "3 OpenClaw skills" in MVP unscheduled/unnamed | This review | Name skills, assign to Week 5-6 |
| 8 | MEDIUM | B3->C1 dependency missing from phases diagram | review-1-devguide.md | Add to dependency diagram |
| 9 | MEDIUM | K3/L2 sandbox-routing integration unspecified | This review | Add integration spec |
| 10 | LOW | Docker base image and layer strategy missing | This review | Specify in K2 tech spec |
| 11 | LOW | M4/F9 dependency needs formal resolution | review-1-devguide.md | Document phased approach |
| 12 | LOW | Forward-compat hooks lack verification tests | This review | Add compile-time tests |
| 13 | LOW | K3a 50+ audit checks -- no inventory list | This review | Create categorized check list |
| 14 | LOW | Phase 4/5 ownership transition not addressed | review-1-devguide.md | Add transition gate |
