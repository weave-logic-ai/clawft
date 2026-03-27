---
name: weftos-ecc-weaver
description: Self-evolving cognitive modeler — a kernel-native SystemService that iteratively learns causal models from data, tracks its own evolution in a meta-Loom, and exports deployable configurations for edge devices
version: 0.2.0
category: architecture
tags:
  - ecc
  - cmvg
  - causal-graph
  - hnsw
  - weaving
  - cognitive-tick
  - self-evolving
  - confidence-driven
  - meta-loom
  - edge-deployment
author: WeftOS Kernel Team
---

# ECC Weaver — Self-Evolving Cognitive Modeler

The Weaver is a kernel-native `SystemService` that iteratively discovers, refines, and
maintains causal models from data. It does not produce a fixed model. It runs a continuous
**HYPOTHESIZE -> OBSERVE -> EVALUATE -> ADJUST** loop, tracking its own evolution as an
ECC conversation in its meta-Loom. The Weaver improves at weaving by weaving its own
improvement trajectory.

## Architecture: Kernel-Native Process

The Weaver runs INSIDE WeftOS as a supervised kernel process:

- **PID**: Registered in `ProcessTable` at boot when `ecc` feature is enabled
- **Service path**: `/kernel/services/weaver`
- **Struct**: `WeaverEngine` implements `SystemService`
- **Data access**: Direct `Arc` references to kernel ECC structures (no serialization, no RPC)
- **Communication**: A2ARouter IPC via `KernelMessage`
- **Supervision**: `AgentSupervisor` with restart policies
- **Tick participation**: Consumes kernel `CognitiveTick` events (does NOT run its own loop)
- **Meta-Loom**: Part of the kernel's ECC structures, not a separate store
- **CLI interface**: `weaver ecc` subcommands talk to the Weaver process via daemon Unix socket

The Weaver is NOT a CLI tool. The `weaver ecc` CLI sends messages to the running Weaver
process, which performs operations on the shared kernel structures.

---

## Core Metaphor

- **Loom** — the combined ECC structures for a single domain
- **ModelingSession** — an active hypothesis-observe-evaluate-adjust loop on a Loom
- **CausalModel** — the learned structure: node types, edge types, patterns, tick config
- **ConfidenceReport** — gap analysis with actionable suggestions
- **ExportedModel** — serialized `weave-model.json` for edge deployment
- **Meta-Loom** — the Weaver's own ECC tracking its modeling decisions and outcomes
- **WeaverKnowledgeBase** — cross-domain learning accumulated from past sessions

---

## The Modeling Loop

The Weaver's core operation is a confidence-driven iteration:

```
1. HYPOTHESIZE  — Propose a causal model ("commits Cause test results")
2. OBSERVE      — Ingest data, attempt to fit the model
3. EVALUATE     — Measure confidence: do edges hold? Enough data? Predictive accuracy?
4. ADJUST       — If confidence is low:
                   - Refine edge types ("Causes" -> "Enables")
                   - Request new data sources
                   - Split or merge categories
                   - Adjust HNSW dimensions or embedding strategy
                   - Change the cognitive tick interval
5. REPEAT       — On each cognitive tick, run the loop
```

This is the **Generate -> Analyze -> Act** cycle applied to modeling itself.

Every modeling decision, confidence assessment, data source addition, schema adjustment,
and improvement is a causal event tracked in the meta-Loom. The Weaver's own evolution
IS an ECC conversation.

---

## Capabilities

### 1. Start a New Modeling Session

Create a new modeling session for a domain. The Weaver hypothesizes an initial causal
model from available data and begins iterating.

**When to use**: Onboarding a new codebase, starting analysis of a new domain, beginning
a modeling project.

**CLI**:

```bash
# Start a modeling session for a git repository
weaver ecc session start --domain my-project --git /path/to/repo

# Start with explicit context and goals
weaver ecc session start --domain my-project \
  --context "Rust microservices codebase with CI/CD" \
  --goal "model commit-to-deployment causation with 0.8+ confidence"

# Resume an existing session
weaver ecc session resume --domain my-project
```

**What happens**:

1. WeaverEngine creates a `ModelingSession` associated with a `Loom`
2. Session registers in the resource tree at `/kernel/services/weaver/sessions/{domain}`
3. Initial model is hypothesized from available data:
   - If the WeaverKnowledgeBase has similar domains, it uses learned strategies
   - Otherwise, it starts with default node/edge types and iterates
4. The cognitive tick begins processing the session
5. Chain event records session genesis with domain, context, goals

**Initial hypothesis for a Rust codebase** (from WeaverKnowledgeBase if available):

```json
{
  "node_types": [
    { "name": "commit", "embedding_strategy": "diff_summary", "dimensions": 384 },
    { "name": "file", "embedding_strategy": "ast_signature", "dimensions": 256 },
    { "name": "test", "embedding_strategy": "test_name_context", "dimensions": 128 }
  ],
  "edge_types": [
    { "from": "commit", "to": "file", "type": "Modifies", "confidence": 0.99 },
    { "from": "commit", "to": "test", "type": "EvidenceFor", "confidence": 0.50 }
  ]
}
```

The initial confidence is intentionally low. The Weaver will iterate upward.

### 2. Add Data Sources

Integrate new data streams into a running session. The Weaver re-evaluates its entire
model when new sources arrive — not just the new data.

**When to use**: When the confidence report suggests new sources, when new data becomes
available, when expanding domain coverage.

**CLI**:

```bash
# Add CI pipeline data to improve commit->test confidence
weaver ecc source add --domain my-project --type ci_pipeline \
  --webhook-url https://ci.example.com/hooks/weaver

# Add issue tracker data
weaver ecc source add --domain my-project --type issue_tracker \
  --api-url https://github.com/org/repo/issues

# Add file tree with watch mode for live updates
weaver ecc source add --domain my-project --type file_tree \
  --root src/ --patterns "**/*.rs" --watch

# Add documentation
weaver ecc source add --domain my-project --type documentation \
  --root docs/

# List active sources
weaver ecc source list --domain my-project
```

**What happens**:

1. Source registers with the session
2. Weaver ingests initial data from the source
3. For each event from the source:
   - Creates a causal node of the appropriate type
   - Generates embedding via the session's embedding strategy
   - Inserts into HNSW with metadata linking back to the causal node
   - Infers edges using the current model's edge types
   - CrossRefs link HNSW entry <-> causal node <-> chain event
4. Full model re-evaluation triggers:
   - All existing edges are re-scored given the new data
   - New edge types may be discovered
   - Confidence report updates
5. Meta-loom records: `source_added -> model_reevaluated -> confidence_changed`

**Source types**:

| Source Type | What It Provides | Required? |
|-------------|-----------------|-----------|
| `git_log` | Commit history, branch structure, authorship | Yes (for code domains) |
| `file_tree` | Module structure, imports, test-source mapping | Yes (for code domains) |
| `ci_pipeline` | Test results, build outcomes, deployment events | No — improves commit->test |
| `issue_tracker` | Issues, labels, assignments, closures | No — improves issue->commit |
| `documentation` | Design docs, ADRs, READMEs | No — improves context |
| `sparc_plan` | SPARC planning artifacts | No — improves goal modeling |
| `custom_stream` | Application-specific event streams | No — domain extension |

### 3. Evaluate Confidence

Produce a confidence report with gap analysis and actionable suggestions. The Weaver
does this continuously on the cognitive tick, but you can request an explicit report.

**When to use**: To understand model quality, to identify what data is missing, to
decide whether to export.

**CLI**:

```bash
# Get current confidence report
weaver ecc confidence --domain my-project

# Get confidence for a specific relationship
weaver ecc confidence --domain my-project --edge "commit->test"

# Get confidence with suggestion detail
weaver ecc confidence --domain my-project --verbose

# Watch confidence evolve in real-time
weaver ecc confidence --domain my-project --watch
```

**Output format**:

```json
{
  "confidence_report": {
    "domain": "my-project",
    "model_version": 3,
    "overall_confidence": 0.62,
    "target_confidence": 0.80,
    "per_edge": {
      "commit->file:Modifies": { "confidence": 0.99, "samples": 1247, "status": "strong" },
      "commit->test:EvidenceFor": { "confidence": 0.45, "samples": 89, "status": "weak" },
      "issue->commit:Causes": { "confidence": 0.31, "samples": 23, "status": "insufficient" },
      "file->file:Enables": { "confidence": 0.52, "samples": 340, "status": "developing" }
    },
    "gaps": [
      {
        "relationship": "commit->test:EvidenceFor",
        "confidence": 0.45,
        "reason": "only 30% of commits have corresponding test runs",
        "suggestion": {
          "action": "add_source",
          "source_type": "ci_pipeline",
          "description": "CI test results would establish commit->test causation",
          "expected_confidence_improvement": 0.25
        }
      },
      {
        "relationship": "issue->commit:Causes",
        "confidence": 0.31,
        "reason": "commit messages don't consistently reference issues",
        "suggestion": {
          "action": "add_source",
          "source_type": "issue_tracker",
          "description": "Issue tracker data with commit cross-references",
          "expected_confidence_improvement": 0.30
        }
      }
    ],
    "patterns": {
      "feature_lifecycle": {
        "sequence": ["issue.created", "branch.created", "commit.series", "pr.opened", "review.cycle", "pr.merged"],
        "confidence": 0.71,
        "instances_found": 14
      }
    },
    "meta_loom_summary": {
      "model_versions": 3,
      "adjustments_made": 7,
      "sources_added": 2,
      "last_improvement": "refined file->file edges using import analysis (+0.15 confidence)"
    }
  }
}
```

**Confidence scoring method**:

For each edge type, the Weaver measures:
- **Coverage**: What fraction of source nodes have this edge? (>0.7 = strong)
- **Consistency**: Do the edges agree with each other? (low contradiction = strong)
- **Predictive accuracy**: If this edge exists, does the predicted relationship hold?
- **Sample size**: Enough data points? (>100 samples = reliable)

Overall confidence is the weighted mean of per-edge confidences, where weight reflects
the edge type's importance to the user's stated goals.

### 4. Export Model

Produce a `weave-model.json` that captures the learned modeling approach — not the data,
but the way to model. This is the deployable configuration for edge devices.

**When to use**: When confidence is sufficient, when deploying to edge devices (Raspberry Pi,
embedded systems), when sharing models across teams.

**CLI**:

```bash
# Export the current model
weaver ecc export --domain my-project --output weave-model.json

# Export only if confidence exceeds threshold
weaver ecc export --domain my-project --min-confidence 0.75 --output weave-model.json

# Export with full evolution history
weaver ecc export --domain my-project --include-history --output weave-model.json
```

**Output**: `weave-model.json`

```json
{
  "version": 3,
  "domain": "rust-codebase-sdlc",
  "created_by": "weaver-engine",
  "confidence": 0.87,
  "model": {
    "node_types": [
      { "name": "commit", "embedding_strategy": "diff_summary", "dimensions": 384 },
      { "name": "file", "embedding_strategy": "ast_signature", "dimensions": 256 },
      { "name": "test", "embedding_strategy": "test_name_context", "dimensions": 128 },
      { "name": "issue", "embedding_strategy": "title_body", "dimensions": 384 },
      { "name": "pr", "embedding_strategy": "description_diff", "dimensions": 384 }
    ],
    "edge_types": [
      { "from": "commit", "to": "file", "type": "Modifies", "confidence": 0.99 },
      { "from": "commit", "to": "test", "type": "EvidenceFor", "confidence": 0.78 },
      { "from": "issue", "to": "commit", "type": "Causes", "confidence": 0.65 },
      { "from": "pr", "to": "commit", "type": "Contains", "confidence": 0.99 },
      { "from": "file", "to": "file", "type": "Enables", "confidence": 0.52, "note": "import dependency" }
    ],
    "patterns": [
      {
        "name": "feature_lifecycle",
        "sequence": ["issue.created", "branch.created", "commit.series", "pr.opened", "review.cycle", "pr.merged", "deploy"],
        "confidence": 0.71
      }
    ],
    "tick_config": {
      "interval_ms": 100,
      "budget_ratio": 0.4,
      "adaptive": true,
      "focus_areas": ["commit_frequency", "test_coverage_delta"]
    },
    "sources_required": [
      { "type": "git_log", "required": true },
      { "type": "file_tree", "required": true },
      { "type": "ci_results", "required": false, "improves": ["commit->test confidence"] },
      { "type": "issue_tracker", "required": false, "improves": ["issue->commit confidence"] }
    ]
  },
  "evolution_history": [
    { "version": 1, "change": "initial model from codebase analysis", "confidence": 0.45 },
    { "version": 2, "change": "added ci_results source, improved commit->test edge", "confidence": 0.72 },
    { "version": 3, "change": "refined file->file edges using import analysis", "confidence": 0.87 }
  ]
}
```

**Edge deployment**: This JSON is pushed to a Raspberry Pi or embedded device that runs
the cognitive tick with this learned model. The device does NOT need LLM access — it uses
the model's node types, edge types, embedding strategies, and tick config to process live
data through the same ECC kernel structures. The model IS the configuration.

### 5. Stitch Models

Combine models from different domains into a cross-domain model. The Weaver identifies
shared concepts via HNSW similarity search and creates cross-forest CrossRefs.

**When to use**: Combining frontend + backend + infrastructure models, merging team-specific
models, creating organization-wide views.

**CLI**:

```bash
# Stitch two domain models
weaver ecc stitch --source frontend --target backend --output product-model

# Stitch with similarity threshold
weaver ecc stitch --source frontend --target backend --threshold 0.85

# Inspect cross-domain connections
weaver ecc stitch inspect --domain product-model
```

**What happens**:

1. For each HNSW entry in the source, search the target for similar vectors
2. Matches above threshold create cross-forest CrossRefs (CrossRefType::Elaborates)
3. Causal graph conflicts are resolved:
   - Same nodes with different edge types: prefer higher-confidence edge
   - Contradictory edges: keep both with provenance metadata
4. The stitched model gets its own confidence report
5. Novel cross-domain connections emit `ImpulseType::NoveltyDetected`
6. Chain events record the stitch with full provenance

### 6. Compare Models (diff)

Compare two exported models to understand what changed between analysis sessions.

**When to use**: Before merging models, reviewing evolution between versions, auditing changes.

**Rust API**: `diff_models(&model_a, &model_b) -> ModelDiff`

The `ModelDiff` struct reports:
- Node types only in A, only in B, or in both
- Edge types only in A, only in B, or in both
- Causal node/edge additions and removals
- Confidence delta and summary text

### 7. Merge Models

Combine two exported models into one, resolving conflicts by preferring higher-confidence edges.

**Rust API**: `merge_models(&model_a, &model_b) -> MergeResult`

The `MergeResult` contains:
- `merged`: the combined `ExportedModel`
- `conflicts`: list of `MergeConflict` records (which fields differed, how resolved)
- `stats`: `MergeStats` with counts of nodes/edges from each source

### 8. Knowledge Base Persistence

The `WeaverKnowledgeBase` supports cross-session learning with full persistence:

- `save_to_file(path)` -- serialize the KB to JSON via `SerializableKB`
- `load_from_file(path)` -- reconstruct from a saved file
- `learn_pattern(pattern)` -- add or update a `StrategyPattern` (merges if same decision_type+context)
- `find_patterns(characteristics)` -- score patterns by how many domain characteristics match, sorted by relevance

### 9. Manage the Meta-Loom

The meta-Loom tracks the Weaver's own evolution. Every modeling decision is a causal
event. This is how the Weaver learns to be a better modeler over time.

**When to use**: Understanding why the model is in its current state, debugging confidence
issues, reviewing the Weaver's decision history.

**CLI**:

```bash
# Show the meta-loom's causal history
weaver ecc meta --domain my-project

# Show the reasoning trajectory for a specific model change
weaver ecc meta --domain my-project --version 3

# Show cross-domain strategy patterns
weaver ecc meta strategies

# Export the WeaverKnowledgeBase
weaver ecc meta export-kb --output weaver-kb.json
```

**Meta-Loom structure**:

The meta-Loom is stored in the kernel's ECC structures (same CausalGraph, HNSW,
CrossRefStore used by all other services). Its nodes are tagged with
`StructureTag::Custom(0x40)` — the Weaver meta namespace.

```
meta-loom/
  causal edges:
    "session_started" --Causes--> "initial_hypothesis"
    "initial_hypothesis" --Enables--> "data_ingestion"
    "data_ingestion" --Causes--> "confidence_evaluation_v1"
    "confidence_evaluation_v1" --Causes--> "adjustment:add_source(ci_pipeline)"
    "adjustment:add_source(ci_pipeline)" --Enables--> "confidence_evaluation_v2"
    "confidence_evaluation_v2" --EvidenceFor--> "model_improvement(+0.27)"

  hnsw entries:
    embeddings of domain characteristics for similarity matching
    embeddings of modeling strategies for cross-domain reuse

  crossrefs:
    links between domain types and successful modeling strategies
    links between confidence gaps and effective remediation actions
```

**WeaverKnowledgeBase**:

The knowledge base accumulates cross-domain learning. A Weaver that has modeled 50 Rust
codebases knows that `cargo test` output is the best source for commit->test edges and
suggests this immediately instead of discovering it from scratch.

```json
{
  "strategy_patterns": [
    {
      "domain_characteristics": ["rust", "cargo", "github-actions"],
      "recommended_sources": ["git_log", "file_tree", "ci_pipeline"],
      "recommended_edge_types": ["Modifies", "EvidenceFor", "Causes", "Enables"],
      "confidence": 0.91,
      "learned_from": ["project-alpha", "project-beta", "project-gamma"]
    }
  ]
}
```

### 7. Stream Processing

The Weaver processes live data within the kernel's cognitive tick. It does not run its
own loop — it is a consumer of CognitiveTick events.

**When to use**: Continuous model maintenance, real-time confidence monitoring, live
data ingestion.

**CLI**:

```bash
# Enable watch mode on a session (sources with watch=true start streaming)
weaver ecc session watch --domain my-project

# View live impulse stream
weaver ecc stream --domain my-project

# View cognitive tick metrics for the Weaver
weaver ecc metrics --domain my-project
```

**What happens on each cognitive tick**:

1. **Poll sources**: Each source with `watch: true` yields new events
2. **Ingest events**: Create causal nodes, embeddings, edges, CrossRefs
3. **Evaluate confidence**: Re-score affected edges
4. **Check thresholds**: If confidence dropped, emit `CoherenceAlert` impulse
5. **Adjust if needed**: Apply pending suggestions (auto-mode) or queue for operator
6. **Record in meta-Loom**: Track what happened this tick
7. **Export check**: If model version bumped and auto-export enabled, write `weave-model.json`

**Impulse types emitted by the Weaver**:

| ImpulseType | When | Payload |
|-------------|------|---------|
| `BeliefUpdate` | New data changes the model | `{ edge, old_confidence, new_confidence }` |
| `CoherenceAlert` | Confidence drops below threshold | `{ domain, confidence, gaps }` |
| `NoveltyDetected` | New pattern or cross-domain connection | `{ pattern, similarity }` |
| `EdgeConfirmed` | Chain validation confirmed an edge | `{ edge, evidence }` |
| `Custom(0x32)` | Model version bumped | `{ version, changes, confidence }` |
| `Custom(0x33)` | Source request (needs operator action) | `{ source_type, reason }` |

---

## Integration with Kernel Services

The Weaver operates on kernel services via direct `Arc` references:

| Service | Role in Weaving |
|---------|----------------|
| `CausalGraph` | Stores typed/weighted causal edges — the model's learned relationships |
| `HnswService` | Vector similarity for semantic matching and cross-domain stitching |
| `CrossRefStore` | Links nodes across structures (causal<->hnsw, chain<->model) |
| `ImpulseQueue` | Emits confidence alerts, novelty detection, source requests |
| `ChainManager` | Provenance: every model version bump, every adjustment recorded |
| `TreeManager` | Session and model organization at `/kernel/services/weaver/` |
| `CognitiveTick` | The Weaver consumes tick events, does not run its own loop |
| `A2ARouter` | IPC with other agents that may provide data or consume models |
| `AgentSupervisor` | Restart policies for the Weaver process |

### CrossRef Types

| CrossRefType | Usage |
|-------------|-------|
| `MemoryEncoded` | HNSW embedding encodes a causal node's content |
| `EvidenceFor` | Test result provides evidence for code correctness |
| `Elaborates` | Cross-forest stitch link |
| `TriggeredBy` | Chain event triggered model adjustment |
| `Custom(0x40)` | Meta-loom node (Weaver's own reasoning) |
| `Custom(0x41)` | Strategy pattern link (knowledge base) |

---

## Workflow Examples

### Example 1: Model the WeftOS Codebase

```
User: weaver ecc session start --domain weftos --git . \
        --context "Rust kernel codebase, ECC architecture" \
        --goal "model module dependencies and test coverage with 0.8+ confidence"

Weaver: Session started for domain "weftos".
  Initial hypothesis: 3 node types (commit, file, test), 2 edge types
  Ingesting git log... 847 commits processed
  Ingesting file tree... 234 files mapped
  Initial confidence: 0.48

  Gaps identified:
  - commit->test (0.32): only 18% of commits have test runs in history
    SUGGESTION: add ci_pipeline source
  - file->file (0.41): import analysis not yet performed
    SUGGESTION: will refine on next tick using Cargo.toml + use statements

User: weaver ecc source add --domain weftos --type ci_pipeline \
        --webhook-url https://ci.example.com/hooks/weaver

Weaver: Source added. Re-evaluating model...
  commit->test confidence: 0.32 -> 0.67 (+0.35)
  Overall confidence: 0.48 -> 0.71

  Model version bumped: v1 -> v2
  Remaining gap: issue->commit (0.29) — suggest adding issue tracker

User: weaver ecc confidence --domain weftos --verbose

Weaver: Confidence report for "weftos" (model v2, confidence: 0.71)
  [strong]      commit->file:Modifies        0.99  (847 samples)
  [developing]  commit->test:EvidenceFor      0.67  (312 samples)
  [developing]  file->file:Enables            0.58  (189 samples)
  [insufficient] issue->commit:Causes          0.29  (12 samples)

  Pattern: "feature_lifecycle" detected 8 instances (confidence: 0.64)

  To reach 0.80 target:
  1. Add issue_tracker source (expected +0.15 overall)
  2. Wait for more CI data (expected +0.08 for commit->test)

User: weaver ecc export --domain weftos --output weave-model.json

Weaver: Exported model v2 (confidence: 0.71) to weave-model.json
  Warning: below target confidence (0.80). Export anyway? [y/N]

User: y

Weaver: Written. Evolution history: v1(0.48) -> v2(0.71)
```

### Example 2: Cross-Domain Stitching

```
User: weaver ecc stitch --source frontend --target backend --output product

Weaver: Stitching "frontend" (confidence: 0.83) + "backend" (confidence: 0.79)
  Searching for cross-domain semantic matches...
  Found 47 connections above threshold (0.80):
  - frontend:api-client <-> backend:api-handler (0.94)
  - frontend:auth-store <-> backend:auth-service (0.91)
  - frontend:user-model <-> backend:user-schema (0.89)
  ...

  Conflicts resolved: 3 (all PreferHigherConfidence)
  Novel connections: 12 (patterns not seen in either domain alone)

  Product model confidence: 0.76
  Impulse emitted: NoveltyDetected (12 cross-domain patterns)
```

### Example 3: Meta-Loom Inspection

```
User: weaver ecc meta --domain weftos

Weaver: Meta-Loom for "weftos" — Weaver reasoning trajectory:

  v1 (confidence: 0.48)
  |- "initial_hypothesis" based on WeaverKnowledgeBase match (rust, cargo)
  |- "ingested git_log" -> 847 commits, 1204 edges
  |- "ingested file_tree" -> 234 nodes, 189 import edges
  |- "evaluated confidence" -> 0.48 (below target 0.80)
  |- "identified gap: commit->test at 0.32"
  |- "recommended: add ci_pipeline"

  v2 (confidence: 0.71)
  |- "source_added: ci_pipeline" -> 312 test results linked
  |- "model_reevaluated" -> commit->test improved 0.32 -> 0.67
  |- "refined file->file edges using use-statement analysis"
  |- "identified gap: issue->commit at 0.29"
  |- "recommended: add issue_tracker"

  Strategy learned: "for Rust/Cargo codebases, CI pipeline data is high-value"
  This strategy will apply to future Rust domain sessions.
```

---

## Fundamental Principle: SDLC as Conversation

The Weaver treats all software development lifecycle activity as conversation:

| SDLC Artifact | Conversation Analog | CausalEdgeType |
|---------------|---------------------|----------------|
| Commit | Utterance | Follows |
| PR | Topic shift | Enables |
| Issue | Goal statement | Causes |
| Test result | Evidence | EvidenceFor |
| Code review | Evaluation | Correlates |
| Deployment | Resolution | TriggeredBy |
| Incident | Interruption | Contradicts |
| Refactor | Clarification | Enables |

The Weaver's modeling loop discovers WHICH of these mappings hold for the specific domain
and quantifies their confidence. The table above is the initial hypothesis; the Weaver
validates, refines, or replaces each mapping based on evidence.

---

## Operating Modes

The Weaver inherits the three-mode paradigm from the kernel:

### Analyze Mode
Read existing artifacts, build understanding, produce confidence reports.
The modeling loop runs in evaluation-only: it hypothesizes and evaluates but does not
auto-adjust. Suggestions are queued for operator review.

### Generate Mode
Set a goal and produce new model structure toward it. Expert agents may be spawned
to provide domain-specific hypotheses. The Weaver evaluates their proposals.

### Act Mode
Process live events in real-time. Auto-adjustment is enabled: when confidence drops,
the Weaver applies its suggestions automatically and records the reasoning in the
meta-Loom. This is the steady-state operating mode for production.

---

## Related Skills

- **sparc-methodology** — SPARC plans are a primary data source for weaving
- **stream-chain** — Chain pipelines can drive generate-mode conversations
- **swarm-orchestration** — Swarm agents can provide domain-specific hypotheses

## Backing Crate

The Weaver skill is backed by the `ecc-weaver` Rust crate. See the SPARC plan at
`.planning/sparc/weftos/09-ecc-weaver-crate.md` for the full specification,
pseudocode, architecture, and implementation plan.
