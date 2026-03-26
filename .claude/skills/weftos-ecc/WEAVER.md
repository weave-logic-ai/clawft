---
name: weftos-ecc-weaver
description: ECC Weaver agent skill for constructing, analyzing, and maintaining Causal Merkle Vector Graph forests across domains
version: 0.1.0
category: architecture
tags:
  - ecc
  - cmvg
  - causal-graph
  - hnsw
  - weaving
  - cognitive-tick
  - sdlc-as-conversation
author: WeftOS Kernel Team
---

# ECC Weaver

The Weaver operates on the forest of trees that Ephemeral Causal Cognition represents.
It initializes, populates, stitches, prunes, and analyzes the combined ECC structures --
CausalGraph, HNSW vector index, CrossRefStore, ImpulseQueue, ExoChain, and ResourceTree --
treating them as a single cognitive fabric called a **Loom**.

## Core Metaphor

- **Loom** -- the combined ECC structures for a single domain
- **Threads** -- causal edges woven through the loom
- **Warp threads** -- recurrent patterns (build-test-deploy, plan-implement-review)
- **Weft threads** -- individual events (commits, utterances, test results)
- **Tapestry** -- multiple looms stitched into a cross-domain cognitive structure
- **weave.toml** -- the loom configuration that guides the cognitive tick

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

Meta-conversations are conversations ABOUT the main conversation: "our testing strategy"
is a meta-conversation about the "implementation" conversation. The Weaver identifies
and tracks these explicitly.

---

## Capabilities

### 1. Loom Initialization

Create a new ECC space for a domain and prime it with initial structure.

**When to use**: Starting a new project, onboarding an existing codebase, creating a
domain-specific cognitive workspace.

**Steps**:

1. Parse or generate a `weave.toml` configuration file
2. Create the CausalGraph root nodes representing the domain's top-level namespaces
3. Initialize the HNSW index with configured dimensions and parameters
4. Bootstrap the CrossRefStore with links between initial structure nodes
5. Create genesis chain events in ExoChain for provenance
6. Initialize the ResourceTree with namespace entries
7. Configure the CognitiveTick for this domain's cadence
8. Run calibration to determine hardware-appropriate tick parameters

**Commands**:

```bash
# Analyze a codebase and generate weave.toml
weaver ecc analyze /path/to/codebase --output weave.toml

# Initialize a loom from configuration
weaver ecc init --config weave.toml

# Quick init with defaults for a git repository
weaver ecc init --git /path/to/repo --domain my-project
```

**Produced artifacts**:
- `weave.toml` -- domain configuration
- CausalGraph with root nodes for each top-level namespace
- HNSW index seeded with initial embeddings (if embedding source available)
- CrossRef links between ExoChain genesis and initial CausalGraph nodes
- Calibration results logged to ExoChain

### 2. Pattern Weaving

Build the recurrent causal structure that represents how the domain's conversation flows.

**When to use**: After loom initialization, when ingesting new data sources, when
the domain's conversation patterns change.

**Warp threads** (recurrent patterns):

```
build -> test -> deploy          # CI/CD conversation
plan -> implement -> review      # feature development conversation
report -> triage -> fix -> verify  # bug lifecycle conversation
propose -> discuss -> decide     # architecture decision conversation
write -> review -> merge         # PR conversation
```

**Steps**:

1. Identify the data source type (git log, file tree, documentation, SPARC plan)
2. Parse source data into candidate nodes and edges
3. Create CausalGraph nodes for each significant event
4. Create typed causal edges following the domain's patterns
5. Generate embeddings for each node's content
6. Insert embeddings into HNSW with metadata linking back to causal nodes
7. Create CrossRefs linking HNSW entries to CausalGraph nodes and ExoChain events
8. Emit impulses for any novelty detected during weaving

**Commands**:

```bash
# Ingest git history into the loom
weaver ecc weave git --repo /path/to/repo --branch main --depth 500

# Ingest source tree structure
weaver ecc weave source --root /path/to/src --patterns "**/*.rs,**/*.ts"

# Ingest documentation
weaver ecc weave docs --root /path/to/docs

# Ingest a SPARC plan
weaver ecc weave sparc --plan /path/to/.planning/sparc/

# Weave all configured sources
weaver ecc weave all --config weave.toml
```

**Git history weaving detail**:

Each commit becomes a CausalGraph node. Edges are created as:
- Sequential commits on the same branch: `Follows` edges
- Merge commits: `Enables` edges from both parents
- Commits referencing issues: `TriggeredBy` edges to the issue node
- Commits that break tests: `Contradicts` edges to the test node
- Commits that fix tests: `EvidenceFor` edges to the fix

PR nodes aggregate their commits. Issue nodes link to their resolution commits.
The result is a conversation where the git history tells a causal story.

**Source tree weaving detail**:

Files become leaf nodes. Directories become namespace nodes. Edges:
- File belongs to module: `Enables` (module enables the file's function)
- Module imports module: `Causes` (dependency relationship)
- Test file tests source file: `EvidenceFor`
- Config file configures module: `Enables`

### 3. Forest Stitching

Merge multiple Looms into a unified cognitive structure (Tapestry).

**When to use**: Combining related projects, creating cross-domain analysis,
linking a codebase loom to its documentation loom.

**Steps**:

1. Load both Looms and their configurations
2. Identify shared concepts via HNSW similarity search across both indices
3. Create cross-forest CrossRefs for matched concepts (CrossRefType::Elaborates)
4. Resolve causal graph conflicts:
   - If both graphs have edges between matched nodes with different types, prefer
     the edge with higher weight
   - If edges contradict (one Causes, other Inhibits), create both with provenance
     metadata noting the conflict
5. Merge HNSW indices (union of vector spaces, dedup by similarity threshold)
6. Create ExoChain bridge events linking the forests' chains
7. Emit ImpulseType::NoveltyDetected for any cross-domain connections discovered

**Commands**:

```bash
# Stitch two looms
weaver ecc stitch --source loom-a --target loom-b --output tapestry

# Stitch with conflict resolution strategy
weaver ecc stitch --source loom-a --target loom-b --strategy prefer-recent

# List cross-forest connections
weaver ecc stitch inspect --tapestry tapestry
```

### 4. Pruning and Trimming

Remove stale structure to keep the loom healthy and performant.

**When to use**: Periodic maintenance, after major refactors, when the loom
exceeds size thresholds, during cold storage archival.

**Decay function**: Every causal edge has a weight. On each cognitive tick,
edges that have not been refreshed (referenced by a new event) decay:

```
new_weight = weight * (1.0 - decay_rate)
```

When weight drops below a configurable threshold (default 0.05), the edge
is eligible for pruning.

**Steps**:

1. Apply decay to all edges not refreshed since the last prune cycle
2. Remove edges below the weight threshold
3. Remove orphaned CausalGraph nodes (no remaining edges)
4. Remove HNSW entries whose corresponding causal nodes were pruned
5. Garbage collect unreferenced CrossRefs
6. Archive pruned subtrees to cold storage (serialized snapshot)
7. Record pruning statistics in ExoChain

**Commands**:

```bash
# Run pruning cycle
weaver ecc prune --config weave.toml

# Dry run -- show what would be pruned
weaver ecc prune --dry-run --config weave.toml

# Archive a subtree to cold storage
weaver ecc archive --subtree namespace::old_module --output archive.bin

# Restore from archive
weaver ecc restore --input archive.bin --target loom
```

### 5. Analysis and Planning

Analyze existing data and produce an ECC initialization or restructuring plan.

**When to use**: Before initializing a loom for an existing codebase, when
evaluating loom health, when planning domain expansion.

**Three modes apply** (from 06-three-modes.md):
- **Analyze**: Read existing code/docs/git and map the conversation structure
- **Generate**: Produce weave.toml and initialization plans
- **Act**: Execute the plan by creating the loom and weaving data

**Analysis outputs**:

1. **AnalysisPlan** -- structured report containing:
   - Discovered nodes (files, modules, features, tests)
   - Recommended causal edges with types and estimated weights
   - Embedding plan (which chunks to embed, estimated vector count)
   - CrossRef plan (which structures to link)
   - Recommended HNSW dimensions based on content complexity
   - Identified meta-conversations (testing strategy, deployment pipeline, etc.)
   - Estimated resource requirements (memory, storage, tick budget)

2. **Suggested patterns** -- recurrent sequences identified in the data:
   - From git history: commit frequency patterns, PR lifecycle patterns
   - From code structure: dependency chains, test coverage patterns
   - From documentation: topic flow, reference patterns

3. **Meta-conversation identification**:
   - "Testing strategy" -- conversations about how testing is done
   - "Deployment pipeline" -- conversations about how code reaches production
   - "Architecture decisions" -- conversations about system structure
   - "Technical debt" -- conversations about what needs improvement

**Commands**:

```bash
# Full analysis of a codebase
weaver ecc analyze /path/to/codebase --output analysis.json

# Analyze only git history
weaver ecc analyze /path/to/repo --source git --output git-analysis.json

# Analyze and generate weave.toml
weaver ecc analyze /path/to/codebase --generate-config --output weave.toml

# Health check on existing loom
weaver ecc analyze --loom my-project --health
```

### 6. Cognitive Tick Configuration

The `weave.toml` file controls how the cognitive tick processes this domain's loom.

**Structure**:

```toml
[domain]
name = "my-project"
description = "Main application codebase"
mode = "analyze"  # act | analyze | generate

[tick]
interval_ms = 50
budget_ratio = 0.3
calibration_ticks = 100
adaptive = true
adaptive_window_s = 30

[causal]
edge_types = [
    "Causes", "Inhibits", "Correlates", "Enables",
    "Follows", "Contradicts", "TriggeredBy", "EvidenceFor"
]
decay_rate = 0.01
decay_threshold = 0.05
max_edges = 100000
max_nodes = 50000

[hnsw]
dimensions = 384
ef_search = 100
ef_construction = 200
max_entries = 100000

[impulse]
ttl_ticks = 100
max_queue_depth = 1000

[[patterns]]
name = "ci-cd"
description = "Build, test, deploy lifecycle"
edge_sequence = ["Causes", "EvidenceFor", "TriggeredBy"]
recurrence = "on_commit"

[[patterns]]
name = "feature-dev"
description = "Plan, implement, review cycle"
edge_sequence = ["Causes", "Follows", "Enables", "EvidenceFor"]
recurrence = "on_event:pr_opened"

[[patterns]]
name = "bug-lifecycle"
description = "Report, triage, fix, verify"
edge_sequence = ["Causes", "Enables", "Follows", "EvidenceFor"]
recurrence = "on_event:issue_opened"

[[patterns]]
name = "architecture-decision"
description = "Propose, discuss, decide, document"
edge_sequence = ["Causes", "Correlates", "Enables", "EvidenceFor"]
recurrence = "periodic:weekly"

[[sources]]
type = "git_log"
path = "."
branch = "main"

[[sources]]
type = "file_tree"
root = "src/"
patterns = ["**/*.rs", "**/*.ts"]

[[sources]]
type = "documentation"
root = "docs/"

[[sources]]
type = "sparc_plan"
path = ".planning/sparc/"

[meta]
enabled = true
conversations = [
    { name = "testing-strategy", relates_to = ["implementation"], pattern = "plan-implement-review-reflect" },
    { name = "deployment-pipeline", relates_to = ["ci-cd"], pattern = "configure-test-deploy-monitor" },
    { name = "architecture", relates_to = ["feature-dev"], pattern = "propose-discuss-decide-document" },
]
```

**Configuration sections explained**:

- `[domain]` -- Identity and operating mode. The mode determines whether the
  cognitive tick processes live events (act), reads existing data (analyze),
  or produces new structure toward a goal (generate).

- `[tick]` -- CognitiveTick parameters. These map directly to `CognitiveTickConfig`.
  The adaptive option auto-adjusts interval based on measured compute latency.

- `[causal]` -- CausalGraph parameters. `decay_rate` controls how fast unused
  edges lose weight per tick. `max_edges`/`max_nodes` trigger pruning when exceeded.

- `[hnsw]` -- HnswService parameters. `dimensions` should match the embedding
  model output size. 384 is standard for MiniLM/all-MiniLM-L6-v2.

- `[impulse]` -- ImpulseQueue parameters. `ttl_ticks` is how many ticks an
  unacknowledged impulse survives before being garbage collected.

- `[[patterns]]` -- Recurrent structural patterns (warp threads). Each pattern
  defines a sequence of CausalEdgeTypes that recurs under certain conditions.
  `recurrence` can be `on_commit`, `on_tick`, `on_event:<event_name>`, or
  `periodic:<interval>`.

- `[[sources]]` -- Data sources to ingest. Each source type has its own
  ingestion strategy.

- `[meta]` -- Meta-conversation configuration. Each meta-conversation tracks
  a higher-order discussion about some aspect of the main domain conversation.

---

## Operating Modes

### Analyze Mode

Read existing artifacts and build an understanding without modifying the source.

```bash
# Enter analyze mode
weaver ecc mode analyze --config weave.toml

# The cognitive tick in analyze mode:
# 1. Reads source data (git log, file tree, docs)
# 2. Creates causal nodes and edges (read-only model)
# 3. Generates embeddings and inserts into HNSW
# 4. Scores coherence, identifies gaps
# 5. Produces analysis report
```

Outputs: structural analysis, goal completion status, coherence scores, causal
decision graph, gap identification.

### Generate Mode

Set a goal and produce new structure toward it.

```bash
# Enter generate mode with a goal
weaver ecc mode generate --config weave.toml \
  --goal "Produce architecture plan for real-time bidding system"

# The cognitive tick in generate mode:
# 1. Spawns expert agent processes
# 2. Agents converse toward the goal (their utterances become causal nodes)
# 3. Speculative branches are explored and pruned
# 4. Winning approach commits to the MainLine
# 5. Output is the committed conversation with full causal provenance
```

Outputs: goal artifact, causal history of production, quality scores.

### Act Mode

Process live events in real-time.

```bash
# Enter act mode
weaver ecc mode act --config weave.toml

# The cognitive tick in act mode:
# 1. Watches for new events (commits, PRs, test results)
# 2. Creates causal nodes and edges in real-time
# 3. Emits impulses for cross-structure communication
# 4. Updates HNSW embeddings incrementally
# 5. Maintains live coherence scoring
```

Outputs: live cognitive model, real-time impulse stream, coherence metrics.

---

## Integration with Kernel Services

The Weaver operates on these kernel services (all from `clawft-kernel`):

| Service | Role in Weaving |
|---------|----------------|
| `CausalGraph` | Stores typed/weighted causal edges between nodes |
| `HnswService` | Vector similarity search for semantic matching |
| `CrossRefStore` | Links nodes across structures (causal-to-hnsw, chain-to-tree) |
| `ImpulseQueue` | Inter-structure event communication |
| `ChainManager` | Provenance recording (ExoChain events) |
| `TreeManager` | Namespace and resource organization |
| `CognitiveTick` | Heartbeat driving the processing cycle |
| `EccCalibration` | Hardware capability measurement at boot |

### CrossRef Types Used by Weaver

| CrossRefType | Weaver Usage |
|-------------|-------------|
| `TriggeredBy` | Commit node triggered by issue node |
| `EvidenceFor` | Test result provides evidence for code correctness |
| `Elaborates` | Documentation elaborates on code structure |
| `GoalMotivation` | Issue motivates feature implementation |
| `MemoryEncoded` | HNSW embedding encodes a causal node's content |
| `Custom(0x20)` | Cross-forest stitch link |
| `Custom(0x21)` | Meta-conversation link |

### Impulse Types Used by Weaver

| ImpulseType | Weaver Usage |
|------------|-------------|
| `BeliefUpdate` | New commit changes the causal model |
| `CoherenceAlert` | Graph structure has become incoherent |
| `NoveltyDetected` | New pattern or cross-domain connection found |
| `EdgeConfirmed` | Chain validation confirmed an edge |
| `Custom(0x30)` | Pruning completed |
| `Custom(0x31)` | Forest stitch completed |

---

## Workflow Examples

### Bootstrap a Rust Project

```bash
# 1. Analyze the codebase
weaver ecc analyze /path/to/rust-project --generate-config --output weave.toml

# 2. Review and edit weave.toml as needed
# (adjust dimensions, patterns, sources)

# 3. Initialize the loom
weaver ecc init --config weave.toml

# 4. Weave all sources
weaver ecc weave all --config weave.toml

# 5. Check loom health
weaver ecc analyze --loom rust-project --health
```

### Cross-Project Analysis

```bash
# 1. Initialize looms for each project
weaver ecc init --git /path/to/frontend --domain frontend
weaver ecc init --git /path/to/backend --domain backend
weaver ecc init --git /path/to/infra --domain infrastructure

# 2. Weave each
weaver ecc weave all --loom frontend
weaver ecc weave all --loom backend
weaver ecc weave all --loom infrastructure

# 3. Stitch into tapestry
weaver ecc stitch --source frontend --target backend --output product-tapestry
weaver ecc stitch --source product-tapestry --target infrastructure --output full-tapestry

# 4. Analyze cross-domain connections
weaver ecc analyze --loom full-tapestry --cross-domain
```

### Continuous Weaving (Act Mode)

```bash
# Start the weaver in act mode, watching for git events
weaver ecc mode act --config weave.toml --watch

# The weaver will:
# - Watch for new commits and create causal nodes
# - Watch for PR events and create topic-shift edges
# - Watch for CI results and create evidence edges
# - Emit impulses for any coherence alerts
# - Auto-prune on the configured schedule
```

---

## Related Skills

- **sparc-methodology** -- SPARC plans are a primary data source for weaving
- **stream-chain** -- Chain pipelines can drive generate-mode conversations
- **swarm-orchestration** -- Swarm agents can be expert participants in generate mode

---

## Backing Crate

The Weaver skill is backed by the `ecc-weaver` Rust crate. See the SPARC plan at
`.planning/sparc/weftos/09-ecc-weaver-crate.md` for the full specification,
pseudocode, architecture, and implementation plan.
