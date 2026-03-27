# WeftOS Weaver External Analysis: ruvector

Sprint 10 Week 5 -- Track W8: Test the Weaver on an external codebase.

**Target project**: `/claw/root/weavelogic/projects/ruvector/`
**Analysis date**: 2026-03-27
**Analyzer**: WeftOS Weaver scripts (external mode, no decisions graph)

---

## 1. Project Profile

| Metric | Value |
|--------|-------|
| Total crates | 109 |
| Total modules | 470 |
| Total commits | 2,484 |
| Date range | 2025-11-19 to 2026-03-25 |
| Authors | 7 (bus factor: 2) |
| Total lines (all crates) | ~600k+ |
| PR/issue references | 152 |

ruvector is a large Rust workspace with 109 crates covering vector databases,
graph algorithms, neural network inference, quantum computing primitives,
WASM targets, and MCP brain services.

---

## 2. Git History Ingestion

**Result**: SUCCESS -- 2,484 commits parsed into 2,636 nodes and 11,166 edges.

### What worked

- The `ingest-git-history.sh` / `_build_git_graph.py` pipeline ran without
  modification when pointed at ruvector's git repository.
- Commit parsing, file tracking, Follows/Enables/Correlates edge construction,
  and PR reference extraction all functioned correctly.
- The script handled 2,484 commits (20x larger than clawft's 126) without
  performance issues.

### Tag classification (partial match)

The tag rules are written for clawft's domain vocabulary (K0-K8, ECC, ExoChain,
etc.), so many ruvector commits did not match:

| Tag | Commits | Notes |
|-----|---------|-------|
| documentation | 963 | Matches `.md` files -- works generically |
| wasm-sandbox | 341 | Matches `wasm` keyword -- coincidentally relevant |
| testing | 158 | Matches `test`/`spec` -- works generically |
| security | 131 | Matches `auth`/`security`/`capabilit` -- partially relevant |
| agents | 65 | Matches `agent` -- relevant (ruvector has agentic-robotics) |
| *untagged* | 1,206 | 48.6% of commits -- the classification gap |

**Key finding**: 48.6% of commits went untagged because the tag rules are
clawft-specific. The Weaver needs configurable, per-project tag rule sets
to be useful on external codebases.

### Patterns found

- **Author concentration**: rUv (46.8%), Claude (28.1%), github-actions (19.2%).
  This is a human+AI collaborative project with CI automation.
- **PR references**: 152 PR/issue references detected, indicating an active
  GitHub workflow.
- **Temporal density**: 2,484 commits in ~4 months = ~20 commits/day average.

---

## 3. Module Dependency Mapping

**Result**: SUCCESS -- 109 crates, 470 modules, 1,001 edges, avg degree 3.3.

### What worked

- The `_build_module_graph.py` script ran unmodified against ruvector's
  `crates/` directory structure.
- Cargo.toml parsing, internal dependency detection, feature gate extraction,
  `use crate::` analysis, test counting, and line counting all worked correctly.
- Discovered 32 feature gates and 124 inter-crate DependsOn edges.

### Module statistics

| Metric | Value |
|--------|-------|
| Modules with tests | 352 (74.9%) |
| Modules without tests | 118 (25.1%) |
| Orphan modules (no incoming Uses) | 298 (63.4%) |
| Most connected module | `ruqu-core/circuit` (16 incoming Uses) |
| Edge types | Uses: 489, EvidenceFor: 352, DependsOn: 124, Enables: 36 |

### Core crates (3+ dependents)

`ruvector-core`, `ruvector-math`, `ruvector-graph`, `ruvector-attention`,
`ruvector-mincut`, `ruvector-solver`, `ruvector-gnn`, `ruvector-coherence`,
`ruvector-domain-expansion`, `ruvector-delta-core`, `ruvector-router-core`,
`ruqu-core`, `agentic-robotics-core`, `prime-radiant`

### Largest crates by lines

1. ruvllm -- 138,893 lines (170 files)
2. ruvector-postgres -- 65,222 lines (137 files)
3. prime-radiant -- 52,466 lines (112 files)
4. ruvector-mincut -- 45,911 lines (74 files)
5. ruqu-core -- 26,093 lines (32 files)

### Category classification gap

The K-level heuristics are clawft-specific. 460 of 470 modules classified as
`K1` (the default fallback), with only 10 getting more specific categories.
This confirms the tag rules need project-specific configuration.

---

## 4. Gap Analysis

**Result**: SUCCESS -- 16 gaps identified. Ran without the decisions graph
(clawft-specific), using only git-history and module-deps data.

### Top 10 Gaps Found

| # | Severity | Area | Detail |
|---|----------|------|--------|
| 1 | HIGH | Untested: mcp-brain-server/routes | 5,095 lines, 0 tests |
| 2 | HIGH | Untested: mcp-brain-server/types | 1,395 lines, 0 tests |
| 3 | HIGH | Untested: mcp-brain-server/store | 1,252 lines, 0 tests |
| 4 | HIGH | Untested: ruvllm-wasm/bindings | 1,201 lines, 0 tests |
| 5 | HIGH | Untested: mcp-brain-server/trainer | 1,015 lines, 0 tests |
| 6 | HIGH | Untested: ruvector-attention-node/training | 851 lines, 0 tests |
| 7 | HIGH | Untested: ruvector-attention-unified-wasm/dag | 806 lines, 0 tests |
| 8 | HIGH | Untested: mcp-brain/tools | 784 lines, 0 tests |
| 9 | HIGH | Untested: mcp-brain-server/graph | 750 lines, 0 tests |
| 10 | HIGH | Untested: ruvector-sona/wasm | 718 lines, 0 tests |

### High Fan-Out / Single Points of Failure

| Module | Incoming | Outgoing | Tests |
|--------|----------|----------|-------|
| ruqu-core/types | 17 | 0 | 0 |
| ruqu-core/circuit | 16 | 2 | 0 |
| ruqu-core/gate | 16 | 1 | 0 |
| ruvector-solver/types | 15 | 0 | 0 |
| ruvector-solver/error | 11 | 1 | 0 |

These are foundation modules with 11-17 dependents and zero tests -- real
single points of failure that the analysis correctly identified.

---

## 5. What Worked vs. What Needs Adaptation

### Worked out of the box

1. **Git history ingestion** -- format-agnostic, works on any git repo
2. **Module dependency mapping** -- works on any Rust workspace with `crates/`
3. **Cargo.toml parsing** -- internal deps, features, crate names
4. **`use crate::` analysis** -- intra-crate module dependencies
5. **Test counting** -- `#[test]` annotation counting
6. **Edge construction** -- Follows, Enables, Correlates, DependsOn, Uses
7. **Orphan/untested detection** -- correctly identifies real gaps
8. **Fan-out analysis** -- found genuine single points of failure
9. **Scaling** -- handled 109 crates / 2,484 commits without issues

### Needs adaptation for non-clawft projects

1. **Tag rules** -- `SUBJECT_TAG_RULES` and `FILE_TAG_RULES` in `_build_git_graph.py`
   are clawft-specific (K0-K8, ECC, ExoChain, mesh). 48.6% of ruvector commits
   went untagged. Need configurable per-project rule sets.

2. **K-level categories** -- `K_CATEGORIES` and `classify_module()` in
   `_build_module_graph.py` are clawft-specific. 98% of ruvector modules
   fell through to the default `K1`. Need pluggable classification.

3. **Decisions graph** -- `analyze-gaps.py` requires `decisions.json` which
   comes from clawft's symposium system. Sections 4 (Decision Bottlenecks),
   7 (Conversation Health), 8 (Phase Status), and 9 (Cross-Graph Balance)
   are all clawft-specific. The external analyzer skips these gracefully.

4. **Hardcoded domain** -- `"domain": "clawft-weftos"` in output JSON.
   Should be parameterized.

5. **Hardcoded paths** -- `ingest-git-history.sh` uses `git -C` relative to
   its own location. Needs a `--repo` flag for external use.

6. **Non-Rust projects** -- Module mapping assumes `crates/`, `Cargo.toml`,
   `*.rs`, `use crate::`. Would need separate scanners for JS/TS/Python/Go.

### Observations

- The orphan module count (298/470 = 63.4%) is high but expected: many
  ruvector modules are leaf implementations (WASM bindings, CLI entry points,
  FFI wrappers) that export functionality rather than being imported internally.

- The "most connected" finding (`ruqu-core/circuit` with 16 dependents) is a
  genuine architectural insight -- this is the quantum circuit primitive that
  everything else in ruqu builds on.

- The test coverage finding (74.9%) and specific untested modules (especially
  mcp-brain-server with 5 large untested modules) represent real, actionable
  gaps a developer could act on immediately.

---

## 6. Recommendations for External Portability

1. **Extract tag rules to config files** -- Move `SUBJECT_TAG_RULES`,
   `FILE_TAG_RULES`, `K_CATEGORIES` to a `.weftos/config/tags.json` that can
   be customized per project.

2. **Add `--repo` flag to shell scripts** -- Allow
   `ingest-git-history.sh --repo /path/to/project` instead of requiring the
   script to live inside the target repo.

3. **Make decisions graph optional** -- The gap analyzer should degrade
   gracefully when `decisions.json` is absent (skip those analysis sections
   rather than failing).

4. **Add language-specific scanners** -- The module graph builder should detect
   the project type (Cargo.toml -> Rust, package.json -> JS/TS, etc.) and
   dispatch to the appropriate scanner.

5. **Parameterize domain name** -- Accept `--domain` flag or infer from
   project directory name.

---

## 7. Files Produced

| File | Description |
|------|-------------|
| `/tmp/weftos-external/graph/git-history.json` | 2,636 nodes, 11,166 edges |
| `/tmp/weftos-external/graph/module-deps.json` | 609 nodes, 1,001 edges |
| `/tmp/weftos-external/analysis/gap-report.json` | 16 gaps, full analysis |
