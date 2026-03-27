# Sprint 09d: Integration & Polish

**Document ID**: 09d
**Workstream**: W-KERNEL
**Duration**: 5 days
**Goal**: Feature composition testing, documentation completeness, gate script, and confidence target
**Depends on**: Sprint 09a (tests), Sprint 09b (decisions), Sprint 09c (Weaver)
**Orchestrator**: `09-orchestrator.md`
**Priority**: P1 (High) -- validates all prior sprint outputs and produces the gate

---

## S -- Specification

### Problem Statement

Sprints 09a-09c produce independent deliverables (tests, decisions, Weaver
runtime). Sprint 09d validates that these deliverables compose correctly:
feature gates do not conflict, documentation is current, the gate script
passes, and the Weaver's confidence reaches its 0.92 target. Additionally,
three phases (K3, K4, K5) have incomplete exit criteria that should be
resolved.

**Source data**: `docs/weftos/09-symposium/00-symposium-overview.md` sections 2
and 5, `docs/weftos/09-symposium/01-graph-findings.md` section 2 (feature gate
islands).

### Scope Summary

| Work Area | Items | Source |
|-----------|-------|--------|
| Feature gate CI matrix | 6 gate combinations to test | 01-graph-findings.md section 2 |
| Incomplete phase cleanup | K3 (1 item), K4 (2 items), K5 (1 item) | 00-symposium-overview.md section 2 |
| Documentation audit | 5 unchecked orchestrator items | 00-symposium-overview.md section 4 |
| Gate script | `scripts/09-gate.sh` | New deliverable |
| Weaver confidence | 0.78 -> 0.92 target | 01-graph-findings.md section 6 |

### Feature Gate Composition Risk

Six feature gates create compositional boundaries. No tests verify that
combinations compile and work together:

| Gate | Key Dependency | Risk |
|------|---------------|------|
| `native` | tokio, dirs | Baseline -- always tested |
| `exochain` | rvf-crypto, ed25519-dalek, ciborium | Crypto crate version conflicts |
| `ecc` | blake3, clawft-core/vector-memory | HNSW + causal graph interaction |
| `mesh` | ed25519-dalek, rand | Shared crypto dep with exochain |
| `os-patterns` | exochain (implied) | Transitive dep chain |
| `cluster` | ruvector-cluster, ruvector-raft | Raft consensus + mesh interaction |

**Highest risk composition**: `ecc + mesh + os-patterns`. The `os-patterns`
gate implies `exochain`, and `ecc` implies `blake3`. Both `mesh` and `exochain`
use `ed25519-dalek`. If any version mismatch exists, this combination fails.

### Incomplete Phase Exit Criteria

| Phase | Total | Checked | Remaining |
|-------|:-----:|:-------:|-----------|
| K3 | 20 | 19 | 1 item (likely WASM sandbox config) |
| K4 | 15 | 13 | 2 items (likely container health propagation) |
| K5 | 17 | 16 | 1 item (likely app manifest validation edge case) |

These 4 items represent <2% of total exit criteria but prevent marking phases
as complete.

---

## P -- Pseudocode

### Feature Composition CI Matrix

```
# Define the matrix
compositions = [
    ["native"],
    ["native", "exochain"],
    ["native", "ecc"],
    ["native", "mesh"],
    ["native", "os-patterns"],
    ["native", "ecc", "mesh", "os-patterns"],
]

# For each composition:
for combo in compositions:
    features = combo.join(",")

    # Step 1: Compile check
    run("cargo check -p clawft-kernel --features {features}")

    # Step 2: Test
    run("cargo test -p clawft-kernel --features {features}")

    # Step 3: Clippy
    run("cargo clippy -p clawft-kernel --features {features} -- -D warnings")

    # Record results
    results.push({ combo, compile: ok?, test: ok?, clippy: ok? })

# Fail if any combination has errors
assert results.all(|r| r.compile && r.test && r.clippy)
```

### Documentation Audit Checklist

```
# From 08-orchestrator.md, unchecked items:
docs_to_create = [
    "docs/site/content/kernel/08a-self-healing.mdx",   # Architecture stub
    "docs/site/content/kernel/08b-reliable-ipc.mdx",    # Architecture stub
    "docs/site/content/kernel/08c-content-ops.mdx",     # Architecture stub
]

# These are stubs, not full documentation (08 not yet implemented)
for doc in docs_to_create:
    create_stub(doc, {
        title: extract_from_sparc_plan(doc),
        status: "Planned -- not yet implemented",
        architecture: extract_diagram_from_sparc_plan(doc),
        exit_criteria: extract_from_sparc_plan(doc),
    })
```

### Gate Script Design

```
#!/usr/bin/env bash
# scripts/09-gate.sh
#
# Validates all Sprint 09 exit criteria.
# Run before merging the sprint branch.

CHECKS_TOTAL=11
CHECKS_PASSED=0

check() {
    local name="$1"
    shift
    echo -n "[$((CHECKS_PASSED+1))/$CHECKS_TOTAL] $name... "
    if "$@" > /dev/null 2>&1; then
        echo "PASS"
        CHECKS_PASSED=$((CHECKS_PASSED+1))
    else
        echo "FAIL"
        FAILURES+=("$name")
    fi
}

# 1-2: Base build
check "Base compile check" scripts/build.sh check
check "Native debug build" scripts/build.sh native-debug

# 3-6: Feature matrix
check "Feature: ecc" cargo check -p clawft-kernel --features ecc
check "Feature: mesh" cargo check -p clawft-kernel --features mesh
check "Feature: os-patterns" cargo check -p clawft-kernel --features os-patterns
check "Feature: ecc+mesh+os-patterns" cargo check -p clawft-kernel --features ecc,mesh,os-patterns

# 7: All tests
check "Workspace tests" scripts/build.sh test

# 8: Clippy
check "Clippy clean" scripts/build.sh clippy

# 9: Test count
check "Test count >= 1320" bash -c '
    COUNT=$(cargo test --workspace 2>&1 | grep -oP "\d+ passed" | grep -oP "\d+" | head -1)
    [ "$COUNT" -ge 1320 ]'

# 10: Critical path coverage
check "Critical path tests" bash -c '
    for m in boot agent_loop chain wasm_runner a2a; do
        COUNT=$(grep -c "#\[test\]" "crates/clawft-kernel/src/${m}.rs" 2>/dev/null || echo 0)
        [ "$COUNT" -ge 5 ] || exit 1
    done'

# 11: Gate script self-check
check "Gate script self-check" true

# Report
echo ""
echo "=== Sprint 09 Gate: $CHECKS_PASSED/$CHECKS_TOTAL passed ==="
if [ ${#FAILURES[@]} -gt 0 ]; then
    echo "Failed checks:"
    for f in "${FAILURES[@]}"; do echo "  - $f"; done
    exit 1
fi
echo "ALL CHECKS PASSED"
```

### Weaver Confidence Improvement Path

```
Current: 0.78 (structural)

After Sprint 09a (test coverage):
  - Test-to-source mapping improves test_coverage_map confidence: +0.03
  - Orphan wiring improves module_relationships: +0.02
  Subtotal: 0.83

After Sprint 09b (decision resolution):
  - Decision resolutions improve decision_chain confidence: +0.02
  Subtotal: 0.85

After Sprint 09c (Weaver runtime):
  - Real embeddings improve embedding_quality: +0.04
  - CognitiveTick improves temporal_patterns: +0.02
  Subtotal: 0.91

After Sprint 09d (integration):
  - Feature composition testing improves integration_test confidence: +0.01
  - Documentation stubs improve documentation_coverage: +0.01
  Subtotal: 0.93 (exceeds 0.92 target)
```

---

## A -- Architecture

### CI Matrix Integration

The feature composition test matrix can be integrated into `scripts/build.sh gate`:

```bash
# Addition to scripts/build.sh gate command
gate_feature_matrix() {
    local combos=(
        "native"
        "native,exochain"
        "native,ecc"
        "native,mesh"
        "native,os-patterns"
        "native,ecc,mesh,os-patterns"
    )

    for combo in "${combos[@]}"; do
        echo "  Feature matrix: $combo"
        cargo check -p clawft-kernel --features "$combo" 2>&1 | tail -1
    done
}
```

For nightly CI (per Q5 recommendation in symposium overview), add a GitHub
Actions workflow that runs the full matrix on schedule.

### Documentation Stub Structure

```mdx
---
title: "08a Self-Healing & Process Management"
description: "Planned: Supervisor restart strategies, process links, resource enforcement"
status: planned
---

# 08a Self-Healing & Process Management

> **Status**: Planned -- not yet implemented. See SPARC plan at
> `.planning/sparc/weftos/0.1/08a-self-healing.md` for full specification.

## Architecture Overview

[Diagram from 08a-self-healing.md section A]

## Planned Components

| Component | Description | Status |
|-----------|-------------|--------|
| Restart strategies | OneForOne, OneForAll, RestForOne | Planned |
| Process links | Bidirectional crash notification | Planned |
| Resource enforcement | Continuous limit checking | Planned |
| Reconciliation | Desired vs actual state controller | Planned |
| Probes | Liveness + readiness health probes | Planned |

## Exit Criteria

[Checklist from 08a-self-healing.md section C]
```

---

## R -- Refinement

### Incomplete Phase Analysis

To resolve the K3/K4/K5 remaining exit criteria, we need to identify the
specific items. Based on the phase completion data:

**K3 (1 remaining)**: The K3 WASM Tool Sandboxing phase has 19/20 exit criteria
checked. The remaining item is likely related to WASM sandbox configuration
persistence or sandbox audit logging, based on the K3 SPARC plan.

**K4 (2 remaining)**: The K4 Container Integration phase has 13/15 checked.
The remaining items likely relate to container health propagation to kernel
health system and container restart via supervisor integration.

**K5 (1 remaining)**: The K5 Application Framework phase has 16/17 checked.
The remaining item likely relates to app manifest versioning or rolling upgrade
verification.

**Action**: Identify exact remaining items from the SPARC plans, implement or
document why they should be deferred.

### Composition Testing Strategy (mesh-engineer review)

**mesh-engineer's concern**: "The riskiest composition is `mesh + os-patterns`.
Both features bring in `ed25519-dalek`. The `mesh` feature uses it for peer
identity signatures; `exochain` (implied by `os-patterns`) uses it for chain
entry signatures. If they require different features of `ed25519-dalek` (e.g.,
`serde` vs `batch`), the compilation will fail."

**Resolution**: Test `mesh + os-patterns` first in the matrix. If it fails,
align the `ed25519-dalek` features in `Cargo.toml` before proceeding with
other tests.

### Documentation Priority (doc-weaver review)

**doc-weaver's concern**: "The most out-of-date documentation is the kernel
subsystem overview. The Fumadocs site has content through K5, but K6 mesh
networking has no documentation page. The 08a/08b/08c stubs are important but
lower priority than documenting what already exists."

**Resolution**: Documentation order:
1. K6 mesh networking page (documents existing implementation)
2. 08a/08b/08c stubs (documents planned work)
3. Sprint 09 decision resolution summary (documents decisions made)

### Performance Considerations

- Feature matrix compilation: each combination takes ~30-60 seconds. Total
  matrix: ~3-6 minutes. Acceptable for gate script.
- Running tests for all combinations: each takes ~20-30 seconds. Total: ~2-3
  minutes. Also acceptable.
- Clippy for all combinations: each takes ~30-45 seconds. Total: ~3-4.5 minutes.
- Full 09-gate.sh: estimated ~12-15 minutes total.

---

## C -- Completion

### Work Packages

#### WP-1: Feature Gate CI Matrix (Day 1-2)

**Owner**: test-sentinel
**Reviewer**: mesh-engineer

- Test all 6 feature gate combinations: compile, test, clippy
- Start with highest-risk: `native + ecc + mesh + os-patterns`
- Fix any compilation issues in `Cargo.toml` feature definitions
- Create `crates/clawft-kernel/tests/feature_composition.rs` integration tests
  (expanded from 09a initial tests)
- Add per-composition integration test that boots TestKernel and verifies
  all expected services are registered
- Estimated: ~300 lines of test code + Cargo.toml fixes

#### WP-2: Incomplete Phase Cleanup (Day 2-3)

**Owner**: kernel-architect
**Reviewers**: sandbox-warden (K3), app-deployer (K4, K5)

- Identify exact remaining exit criteria for K3 (1), K4 (2), K5 (1)
- For each:
  - If implementable in <50 lines: implement
  - If out of scope: document why and mark as deferred with rationale
- Update phase completion tracking in symposium documentation
- Target: K3 20/20, K4 15/15, K5 17/17 (or documented deferrals)
- Estimated: ~100-200 lines of code + documentation

#### WP-3: Gate Script (Day 3)

**Owner**: test-sentinel
**Reviewer**: kernel-architect

- Create `scripts/09-gate.sh` with 11 checks (see Pseudocode section)
- Checks: base build, native build, feature matrix (4 combos), workspace tests,
  clippy, test count verification, critical path coverage, self-check
- Make executable, test locally
- Estimated: ~80 lines of bash

#### WP-4: Documentation Audit (Day 3-4)

**Owner**: doc-weaver
**Reviewer**: kernel-architect

- Create K6 mesh networking Fumadocs page (from existing implementation)
- Create 08a/08b/08c documentation stubs (architecture + planned components)
- Write Sprint 09 decision resolution summary
- Verify `scripts/build.sh gate` passes with `os-patterns` feature
- Define `scripts/08-gate.sh` structure (shell with placeholder checks)
- Files:
  - `docs/site/content/kernel/mesh-networking.mdx` (new)
  - `docs/site/content/kernel/08a-self-healing.mdx` (new stub)
  - `docs/site/content/kernel/08b-reliable-ipc.mdx` (new stub)
  - `docs/site/content/kernel/08c-content-ops.mdx` (new stub)
  - `scripts/08-gate.sh` (new placeholder)
- Estimated: ~500 lines of MDX + ~30 lines of bash

#### WP-5: Weaver Confidence Push (Day 4-5)

**Owner**: weaver + ecc-analyst
**Reviewer**: kernel-architect

- Run Weaver with new test coverage data (from 09a)
- Run Weaver with decision resolution data (from 09b)
- Run Weaver with real embeddings (from 09c)
- Measure confidence and identify remaining gaps
- Add data sources to close gaps:
  - CI pipeline configuration as data source
  - Feature gate composition results as test coverage evidence
  - Decision resolution log as decision chain data
- Target: confidence >= 0.92
- If target not reached: document remaining gaps and what data would close them
- Estimated: ~100 lines of configuration + data source additions

#### WP-6: Final Integration Validation (Day 5)

**Owner**: test-sentinel
**Reviewer**: kernel-architect

- Run `scripts/09-gate.sh` end-to-end
- Run `scripts/build.sh gate` with `os-patterns` feature
- Verify total test count meets 1,320+ kernel / 3,950+ workspace targets
- Verify no regressions: all pre-existing tests pass
- Run clippy on entire workspace
- Document any outstanding issues for Sprint 10
- Estimated: validation only, no new code

### Exit Criteria

- [ ] Feature gate CI matrix: `native` builds and all tests pass
- [ ] Feature gate CI matrix: `native + exochain` builds and all tests pass
- [ ] Feature gate CI matrix: `native + ecc` builds and all tests pass
- [ ] Feature gate CI matrix: `native + mesh` builds and all tests pass
- [ ] Feature gate CI matrix: `native + os-patterns` builds and all tests pass
- [ ] Feature gate CI matrix: `native + ecc + mesh + os-patterns` builds and all tests pass
- [ ] `scripts/09-gate.sh` exists and passes all 11 checks
- [ ] K3 remaining exit criterion completed or documented deferral
- [ ] K4 remaining exit criteria completed or documented deferral
- [ ] K5 remaining exit criterion completed or documented deferral
- [ ] Fumadocs documentation: K6 mesh networking page created
- [ ] Fumadocs documentation: 08a stub created
- [ ] Fumadocs documentation: 08b stub created
- [ ] Fumadocs documentation: 08c stub created
- [ ] `scripts/build.sh gate` passes with `os-patterns` feature
- [ ] `scripts/08-gate.sh` structure defined (placeholder)
- [ ] Weaver confidence reaches 0.92 target (or gap documented)
- [ ] Total workspace test count reaches 3,950+ (185+ new across all sprints)
- [ ] No regressions: all pre-existing tests pass
- [ ] All clippy warnings resolved

### Agent Assignment

| Agent | Role | Work Packages |
|-------|------|---------------|
| **test-sentinel** | Primary | WP-1 (CI matrix), WP-3 (gate script), WP-6 (validation) |
| **mesh-engineer** | Reviewer + risk expert | WP-1 (composition risk assessment) |
| **kernel-architect** | Reviewer + implementer | WP-2 (phase cleanup), all WP reviews |
| **sandbox-warden** | Reviewer | WP-2 (K3 remaining item) |
| **app-deployer** | Reviewer | WP-2 (K4/K5 remaining items) |
| **doc-weaver** | Primary | WP-4 (documentation audit and creation) |
| **weaver** | Primary | WP-5 (confidence push) |
| **ecc-analyst** | Co-implementer | WP-5 (data source analysis) |

### Expert Review Notes

**mesh-engineer**: "Test `mesh + os-patterns` FIRST. If `ed25519-dalek` has
a version conflict between the mesh and exochain features, it blocks the entire
matrix. The fix is usually to align the version in the workspace `Cargo.toml`
`[workspace.dependencies]` section with a unified feature set."

**doc-weaver**: "Priority order for documentation: (1) K6 mesh page -- documents
existing implementation that has no docs, (2) 08a/08b/08c stubs -- provides
architecture context for planned work, (3) Sprint 09 decision summary --
records rationale for future reference. The K6 page is the highest priority
because it fills a documentation gap for working code."

**kernel-architect**: "The incomplete phase items (K3, K4, K5) are likely minor.
Before implementing, read the exact remaining exit criteria from the SPARC
plans (04-phase-K3-wasm-sandbox.md, 05-phase-K4-containers.md,
06-phase-K5-app-framework.md). Some may be obsoleted by subsequent work."

**test-sentinel**: "The 09-gate.sh script should be deterministic and
reproducible. Avoid depending on network calls (e.g., no `cargo fetch` during
gate). All dependencies should be cached. The gate should complete in under
15 minutes on the current ARM host."

**weaver**: "The 0.92 confidence target is ambitious. The trajectory shows
0.78 -> 0.85 from Sprints 09a+09b, then 0.85 -> 0.91 from Sprint 09c. The
last 0.01 to reach 0.92 depends on integration testing data feeding back into
the model. If we reach 0.91, that is acceptable -- document the gap and what
data would close it."

**ecc-analyst**: "The confidence improvement path assumes each data source
addition provides diminishing returns. The jump from 0.78 to 0.85 (7 points)
comes from two major sources (tests + decisions). The jump from 0.85 to 0.91
(6 points) comes from one major source (real embeddings). The final point
(0.91 to 0.92) requires marginal data. This trajectory is realistic but tight."

### Testing Verification Commands

```bash
# Run the full gate
scripts/09-gate.sh

# Run feature matrix manually
for combo in native "native,exochain" "native,ecc" "native,mesh" "native,os-patterns" "native,ecc,mesh,os-patterns"; do
    echo "=== $combo ==="
    cargo check -p clawft-kernel --features "$combo"
done

# Run build.sh gate
scripts/build.sh gate

# Check Weaver confidence
weave ecc confidence --domain clawft

# Verify test count
cargo test --workspace 2>&1 | grep "test result"

# Clippy entire workspace
scripts/build.sh clippy
```

### Implementation Order

```
Day 1:
  WP-1: Feature gate CI matrix (start with high-risk compositions)

Day 2:
  WP-1: Complete CI matrix, fix any Cargo.toml issues
  WP-2: Incomplete phase cleanup (K3, K4, K5)

Day 3:
  WP-3: Gate script creation + local testing
  WP-4: Documentation audit (start)

Day 4:
  WP-4: Documentation (complete K6 page + 08a/08b/08c stubs)
  WP-5: Weaver confidence push (run with Sprint 09a/09b/09c data)

Day 5:
  WP-5: Confidence push (complete, document gaps if needed)
  WP-6: Final integration validation (full gate run)
  Sprint 09 retrospective notes
```
