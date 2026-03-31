# GEPA & Prompt Evolution: Deep Research + clawft Integration Plan

**Date**: 2026-03-27
**Branch**: feature/weftos-kernel-sprint
**Status**: Research complete

---

## Part 1: GEPA Technical Deep-Dive

### What is GEPA?

GEPA (Genetic-Pareto) is a reflective prompt optimizer that evolves textual
parameters of AI systems through iterative natural-language reflection and
multi-objective evolutionary search. Accepted as an **ICLR 2026 Oral**, the
paper (arxiv:2507.19457) demonstrates that reading full execution traces and
reflecting on them in natural language outperforms both reinforcement learning
(GRPO) and prior prompt optimizers (MIPROv2).

Key insight: instead of collapsing an agent's execution into a single scalar
reward, GEPA reads the *full trace* -- reasoning steps, tool calls, tool
outputs, error messages, profiling data -- and uses an LLM to **diagnose why
a candidate failed** and propose targeted textual fixes.

### Core Algorithm (5-Phase Loop)

```
1. SELECT   -- Pick a candidate from the Pareto frontier
              (probability proportional to coverage of training instances)

2. EXECUTE  -- Run candidate on a minibatch, capturing full execution traces

3. REFLECT  -- LLM reads traces + accumulated ancestry lessons
              → diagnoses root causes of failure
              → produces "Actionable Side Information" (ASI)

4. MUTATE   -- Generate improved candidate informed by ASI
              (targeted refinement, not random perturbation)

5. ACCEPT   -- If improved: validate on full set, update Pareto frontier
              → optionally MERGE complementary candidates from different lineages
```

This loop repeats until the evaluation budget is exhausted (typically 100-500
metric calls, versus 5,000-25,000+ for RL approaches).

### Three Core Principles

1. **Genetic Prompt Evolution**: Candidates form lineages. Each mutation
   accumulates high-level lessons from all ancestors, not just the immediate
   parent. This prevents catastrophic forgetting of earlier insights.

2. **Reflection via Natural Language Feedback**: The reflection LM receives
   the full execution trace (not just pass/fail) and produces structured
   diagnostic feedback. This is the "gradient equivalent" for text --
   informative enough to guide targeted mutations rather than random search.

3. **Pareto-based Selection**: Instead of tracking only the global best,
   GEPA maintains a frontier of non-dominated candidates -- each excelling on
   at least one training instance. This preserves strategy diversity and
   prevents premature convergence.

### Mutation Operators

| Operator | Description |
|----------|-------------|
| **Targeted refinement** | LLM proposes specific edits based on failure diagnosis |
| **System-aware merge** | Combines complementary modules from distinct lineages (up to +5% additional improvement) |
| **Genealogy-informed mutation** | Leverages accumulated lessons from all ancestors |
| **Structured generation** | Respects domain constraints (size limits, format requirements) |

### Fitness Evaluation

GEPA separates two concerns:
- **Scalar metrics** drive selection (which candidates survive)
- **Rich execution traces** drive reflection (how to improve)

The metric function signature in DSPy:
```python
def metric(gold, pred, trace, pred_name, pred_trace) -> float | ScoreWithFeedback
```

When `ScoreWithFeedback` is returned, the feedback string feeds directly into
the reflection step. If only a scalar is returned, GEPA defaults to
"This trajectory got a score of {score}."

### Population Management

- Active population of candidates under evaluation
- Pareto frontier tracking non-dominated solutions
- Complete genealogical history enabling ancestry-aware mutations
- Configurable population size and retention policies
- Stochastic sampling from frontier (probability ~ coverage)

### Experimental Results

| Comparison | Result |
|-----------|--------|
| GEPA vs GRPO | +6-10% average, up to +20% on specific tasks |
| GEPA vs MIPROv2 | +10-12% (e.g. +12% on AIME-2025) |
| Sample efficiency | 35x fewer rollouts than GRPO |
| Prompt compaction | Up to 9.2x shorter than MIPROv2 |
| Pareto ablation | +6.4-8.2% over greedy selection |
| Merge ablation | Up to +5% when sufficient strategy diversity exists |
| Cost | $2-10 per optimization run (API-only, no GPU) |
| Generalization | Lower gap than few-shot demonstration-based prompts |

Tasks tested: HotpotQA, IFBench, HoVer, PUPA, AIME-2025, plus code
optimization (AMD NPU kernels, NVIDIA CUDA kernels).

### Integration Surfaces

**Standalone library**: `gepa-ai/gepa` on GitHub (MIT licensed)

**DSPy integration**: `dspy.GEPA()` optimizer -- compiles DSPy programs by
evolving their prompts. Key parameters:
- `reflection_lm` -- strong model for diagnosis
- `max_metric_calls` -- evaluation budget
- `candidate_selection_strategy` -- "pareto" or "current_best"
- `use_merge` -- enable crossover optimization
- `log_dir` -- save optimization artifacts for checkpointing

**Hermes Agent integration**: `NousResearch/hermes-agent-self-evolution`
evolves Hermes Agent skills, tool descriptions, and system prompts using GEPA.
Pipeline:
```
Read skill → Generate eval dataset → GEPA optimize → Constraint gates → PR
```

### Runtime vs Training-Time

GEPA is primarily a **development/optimization-time** technique -- you run it
offline to produce better prompts, then deploy the evolved variants. However,
it also works as an **inference-time search strategy** (demonstrated on code
optimization tasks where compiler feedback is available).

---

## Part 2: Prompt Optimization Landscape

### Comparison of Approaches

| Method | Mechanism | Strengths | Weaknesses |
|--------|-----------|-----------|------------|
| **GEPA** | Reflective evolution + Pareto | Sample-efficient, trace-aware, multi-objective | Requires strong reflection LM |
| **MIPROv2** | Bayesian prompt optimization | Good general performance | Longer prompts, higher cost |
| **GRPO** | Group Relative Policy Optimization | Learns from large rollouts | 35x more samples needed |
| **OPRO** | Single mutation prompt | Simple | Limited diversity |
| **PromptBreeder** | Self-referential co-evolution of task prompts AND mutation prompts | Meta-learning capability | Complex, expensive |
| **TextGrad** | Gradient-like text feedback | Gradient analogy intuitive | Single-objective |
| **SCOPE** | Online context evolution from traces | Runtime adaptation (14%→39%) | Focused on context only |
| **EvoAgentX** | Evolutionary agent ecosystem | Full agent optimization | Complex orchestration |
| **GAAPO** | Genetic algorithm for prompts | Standard GA operators | No reflection |
| **SPO** | Self-supervised prompt optimization | 1-6% cost of others | Newer, less proven |

### Most Relevant for an Agent OS

For WeftOS -- an agent OS that learns from its own operation -- the most
relevant approaches are:

1. **GEPA**: Best fit. Trace-aware reflection maps perfectly to WeftOS's
   existing trajectory recording in the pipeline. Pareto selection handles
   the multi-objective reality (correctness, cost, latency, safety).

2. **SCOPE**: Relevant for runtime adaptation. Its online optimization of
   agent context from execution traces could complement GEPA's offline
   evolution.

3. **PromptBreeder**: The self-referential aspect (mutation prompts co-evolve)
   is conceptually aligned with WeftOS's meta-cognitive ambitions, but adds
   significant complexity.

4. **EvoAgentX**: Interesting for evolving entire agent workflows, not just
   prompts. Could inform future WeftOS workflow optimization.

---

## Part 3: Mapping GEPA to clawft/WeftOS

### Current Architecture (What Exists)

| Module | File | Status | Role |
|--------|------|--------|------|
| `skill_autogen.rs` | `clawft-core/src/agent/skill_autogen.rs` | Implemented | Detects repeated tool-call patterns, generates SKILL.md |
| `skill_watcher.rs` | `clawft-core/src/agent/skill_watcher.rs` | Implemented | Hot-reload skills on filesystem change |
| `skills_v2.rs` | `clawft-core/src/agent/skills_v2.rs` | Implemented | 3-level priority registry (workspace > user > built-in) |
| `pipeline/learner.rs` | `clawft-core/src/pipeline/learner.rs` | **NoopLearner stub** | Records trajectories (currently discards them) |
| `pipeline/scorer.rs` | `clawft-core/src/pipeline/scorer.rs` | **NoopScorer stub** | Scores responses (currently returns 1.0) |
| `scoring.rs` | `clawft-core/src/scoring.rs` | BasicScorer | Heuristic quality scoring (length, error indicators, tool use) |
| `pipeline/traits.rs` | `clawft-core/src/pipeline/traits.rs` | Defined | `LearningBackend` trait: `record(Trajectory)` + `adapt(LearningSignal)` |
| `causal.rs` | `clawft-kernel/src/causal.rs` | Implemented | Concurrent DAG with typed/weighted edges, provenance tracking |
| `democritus.rs` | `clawft-kernel/src/democritus.rs` | Implemented | SENSE→EMBED→SEARCH→UPDATE→COMMIT cognitive loop |
| `cognitive_tick.rs` | `clawft-kernel/src/cognitive_tick.rs` | Implemented | Heartbeat driving ECC processing with adaptive interval |
| `weaver.rs` | `clawft-kernel/src/weaver.rs` | Implemented | ECC-powered codebase modeling with Meta-Loom for self-improvement |

### Integration Points

#### 1. `skill_autogen.rs` + Genetic Evolution

**Current**: Detects repeated `ToolCallPattern`s, generates template SKILL.md
with sequential tool instructions. No optimization of the generated prompt.

**GEPA enhancement**: After generating a skill candidate, run a mini-evolution
loop to optimize the skill's instructions against evaluation traces.

```
Current flow:
  Pattern detected → Template SKILL.md → .pending → User approval

Enhanced flow:
  Pattern detected → Template SKILL.md → EVOLVE(traces) → Optimized SKILL.md
    → .pending → User approval
```

The `SkillCandidate` struct already has a `skill_md: String` field that
contains the generated instructions. This is exactly the "textual parameter"
GEPA optimizes.

#### 2. `pipeline/learner.rs` as Evolution Engine

**Current**: `NoopLearner` -- discards all trajectories. The `LearningBackend`
trait has two methods:
- `record(trajectory: &Trajectory)` -- stores interaction data
- `adapt(signal: &LearningSignal)` -- processes feedback

**GEPA enhancement**: Replace `NoopLearner` with `EvolutionaryLearner` that:
1. **Records** trajectories in a ring buffer (training data for evolution)
2. **Scores** each trajectory using the pipeline scorer
3. **Periodically triggers** evolution runs when enough data accumulates
4. **Proposes** prompt/skill mutations based on reflection over poor traces

This directly maps to GEPA's "collect traces → reflect → mutate" loop.

#### 3. `pipeline/scorer.rs` as Fitness Function

**Current**: `NoopScorer` returns `QualityScore { overall: 1.0, relevance: 1.0, coherence: 1.0 }`.

**GEPA requires**: A metric function that returns both a scalar score AND
rich textual feedback. The `QualityScore` struct already has three dimensions.
Adding a `feedback: String` field would provide the "Actionable Side
Information" that makes GEPA's reflection effective.

The `BasicScorer` in `scoring.rs` already evaluates length, error indicators,
and tool usage -- this is a starting point for the fitness function.

#### 4. ECC Causal Graph as Prompt Lineage Tracker

**Current**: `CausalGraph` supports typed/weighted directed edges with
provenance tracking (`chain_seq`, `timestamp`). Edge types include `Causes`,
`Enables`, `Follows`, `EvidenceFor`.

**GEPA mapping**: Prompt lineage (parent → mutation → child → score) maps
directly to the causal graph:

| GEPA Concept | CausalGraph Mapping |
|-------------|---------------------|
| Parent prompt | CausalNode (label="prompt_v1", metadata={content, score}) |
| Mutation | CausalEdge (type=Causes, weight=mutation_temperature) |
| Child prompt | CausalNode (label="prompt_v2", metadata={content, score}) |
| Fitness score | Node metadata + edge weight |
| Reflection insight | CausalNode (label="reflection", type=EvidenceFor) |
| Merge/crossover | Multiple incoming Causes edges |
| Pareto dominance | Edge type=Enables from non-dominated set |

The causal graph's `chain_seq` field provides ExoChain provenance --
every mutation has an immutable audit trail, which is exactly what
governance requires.

#### 5. DEMOCRITUS Loop as Evolution Substrate

The DEMOCRITUS loop (SENSE → EMBED → SEARCH → UPDATE → COMMIT) already
processes impulses through the cognitive substrate. A prompt evolution
impulse could be:

```rust
ImpulseType::PromptEvolution {
    candidate_id: NodeId,
    parent_id: NodeId,
    reflection: String,
    score_delta: f32,
}
```

This would be processed during the UPDATE phase, adding causal edges and
updating the prompt lineage graph.

---

## Part 4: WeftOS Safety Advantages

### Why WeftOS + GEPA is Safer Than Hermes's Approach

Hermes Agent Self-Evolution operates as an external Python pipeline that
submits PRs against the Hermes Agent repo. It has no runtime governance --
safety comes entirely from human PR review.

WeftOS has several architectural advantages:

#### 1. ExoChain Provenance on Every Mutation

Every prompt mutation flows through the causal graph with `chain_seq`
provenance. The entire lineage -- from original prompt through every
intermediate mutation to the current version -- is immutable and auditable.
Hermes has no equivalent; once a PR is merged, the mutation history is
only in git commits.

#### 2. Governance Gate on Deployment

The `skill_autogen.rs` already implements a `.pending` approval gate:
```
generated → .pending marker → user approval → active
```

For evolved prompts, this becomes:
```
evolved → .pending → governance check → score threshold → user approval → active
```

The governance check can enforce:
- Minimum fitness score threshold
- Maximum deviation from parent prompt (semantic distance via HNSW)
- Required improvement over baseline
- Rate limiting on evolution frequency
- Budget caps on reflection LLM calls

#### 3. ECC Causal Reasoning for Anomaly Detection

If an evolved prompt suddenly causes downstream failures, the causal graph
can trace back: "prompt_v7 was deployed at tick T; failure rate increased at
tick T+5; causal edge weight from prompt_v7 to failures exceeds threshold."
This enables **automatic rollback** with causal justification.

#### 4. Resource Tree Permission Model

The `exo-resource-tree` crate enforces hierarchical permissions. An evolution
module would need explicit delegation to:
- Read trajectory history (data access)
- Call the reflection LLM (compute budget)
- Write new skill files (filesystem access)
- Deploy evolved prompts (runtime access)

Each of these is a separate, revocable permission in the resource tree.

#### 5. Cognitive Tick Budget Enforcement

The `CognitiveTick` service enforces compute budgets per tick. Evolution
work that exceeds the tick budget gets deferred, preventing runaway
optimization from starving the main agent loop.

### Comparison Table

| Safety Dimension | Hermes Self-Evolution | WeftOS + GEPA |
|-----------------|----------------------|---------------|
| Mutation provenance | Git commits only | ExoChain causal graph |
| Deployment gate | Human PR review | Automated governance + human approval |
| Anomaly detection | None | Causal graph anomaly tracing |
| Permission model | Unix filesystem | Hierarchical resource tree |
| Budget enforcement | CLI cost estimate | Cognitive tick budget + cost tracker |
| Rollback | Git revert | Automatic causal rollback |
| Audit trail | Git log | Immutable causal chain with timestamps |

---

## Part 5: Implementation Plan

### Phase 1: Foundation (Immediate)

**Goal**: Replace NoopLearner and NoopScorer with functional implementations
that collect the data GEPA needs.

#### File: `clawft-core/src/pipeline/scorer.rs`
Replace `NoopScorer` with `HeuristicScorer` that uses the existing
`BasicScorer` logic from `scoring.rs`:
- Length analysis (penalize very short/very long)
- Error indicator detection ("I can't", "I'm unable")
- Tool usage presence
- Format compliance (if structured output expected)
- Return `QualityScore` with all three dimensions populated

#### File: `clawft-core/src/pipeline/learner.rs`
Replace `NoopLearner` with `TrajectoryStore`:
- Ring buffer of recent trajectories (configurable size, default 1000)
- Index by quality score for easy retrieval of poor/excellent examples
- Persist to disk on graceful shutdown
- Expose `get_poor_trajectories(n)` for reflection input

#### New file: `clawft-core/src/pipeline/feedback.rs`
Define the extended feedback type:
```rust
/// Rich feedback combining scalar score with textual diagnosis.
pub struct ScoredFeedback {
    pub score: QualityScore,
    /// Natural-language explanation of the score.
    /// This feeds into GEPA-style reflection.
    pub feedback: String,
    /// The full execution trace (tool calls, responses, errors).
    pub trace: Vec<TraceEntry>,
}

pub struct TraceEntry {
    pub step: usize,
    pub action: String,       // "tool_call", "llm_response", "error"
    pub content: String,
    pub timestamp_ms: u64,
}
```

### Phase 2: Evolution Engine (Sprint 12-13)

**Goal**: Implement the core evolution loop as a kernel service.

#### New file: `clawft-core/src/pipeline/evolver.rs`

```rust
//! GEPA-inspired prompt evolution engine.
//!
//! Operates on textual parameters (skill instructions, system prompts,
//! tool descriptions) using reflection + Pareto selection.

use crate::pipeline::traits::{Trajectory, QualityScore, LearningBackend};

/// Configuration for the evolution engine.
pub struct EvolverConfig {
    /// Maximum evaluation budget per evolution run.
    pub max_evaluations: usize,          // default: 100
    /// Minimum trajectories before triggering evolution.
    pub min_trajectories: usize,         // default: 50
    /// Number of candidates in the active population.
    pub population_size: usize,          // default: 8
    /// Score threshold below which a trajectory triggers evolution.
    pub evolution_trigger_threshold: f32, // default: 0.6
    /// Model to use for reflection (should be strong).
    pub reflection_model: String,
    /// Model to use for mutation generation.
    pub mutation_model: String,
    /// Temperature for mutation generation.
    pub mutation_temperature: f64,       // default: 1.0
    /// Whether to enable merge/crossover.
    pub enable_merge: bool,              // default: true
    /// Maximum merge attempts per generation.
    pub max_merge_attempts: usize,       // default: 5
    /// Governance: require approval before deploying evolved prompts.
    pub require_approval: bool,          // default: true
    /// Governance: minimum score improvement to accept a mutation.
    pub min_improvement: f32,            // default: 0.05
}

/// A prompt candidate in the evolution population.
pub struct PromptCandidate {
    /// Unique identifier (also a CausalGraph NodeId).
    pub id: u64,
    /// The textual content being evolved.
    pub content: String,
    /// Parent candidate ID (None for the original).
    pub parent_id: Option<u64>,
    /// Generation number.
    pub generation: u32,
    /// Fitness scores on individual training instances.
    pub instance_scores: Vec<(String, f32)>,
    /// Aggregate fitness score.
    pub aggregate_score: f32,
    /// Accumulated reflection insights from this lineage.
    pub lineage_insights: Vec<String>,
    /// CausalGraph chain_seq for provenance.
    pub chain_seq: u64,
}

/// The Pareto frontier of non-dominated candidates.
pub struct ParetoFrontier {
    candidates: Vec<PromptCandidate>,
}

impl ParetoFrontier {
    /// Select a candidate for mutation, weighted by coverage.
    pub fn select_for_mutation(&self) -> &PromptCandidate { todo!() }

    /// Add a candidate if it is non-dominated.
    pub fn try_add(&mut self, candidate: PromptCandidate) -> bool { todo!() }

    /// Merge two candidates from different lineages.
    pub fn merge(&self, a: &PromptCandidate, b: &PromptCandidate)
        -> PromptCandidate { todo!() }
}

/// Evolution target: what kind of text is being evolved.
pub enum EvolutionTarget {
    /// A skill's SKILL.md instructions.
    SkillInstructions { skill_name: String },
    /// A system prompt section.
    SystemPrompt { section_id: String },
    /// A tool description.
    ToolDescription { tool_name: String },
}

/// A single evolution run result.
pub struct EvolutionResult {
    pub target: EvolutionTarget,
    pub original: PromptCandidate,
    pub best: PromptCandidate,
    pub improvement: f32,
    pub generations_used: u32,
    pub evaluations_used: usize,
    pub pareto_size: usize,
}
```

### Phase 3: Causal Integration (Sprint 13-14)

**Goal**: Wire evolution into the ECC causal graph for provenance and
anomaly detection.

#### Modifications to: `clawft-kernel/src/causal.rs`

Add new edge types for prompt evolution:
```rust
pub enum CausalEdgeType {
    // ... existing variants ...

    /// Prompt A was mutated to produce Prompt B.
    MutatedTo,
    /// Prompt A and B were merged to produce Prompt C.
    MergedInto,
    /// Prompt A was deployed and produced observed outcome B.
    ProducedOutcome,
    /// Reflection R informed mutation M.
    InformedBy,
}
```

#### New file: `clawft-kernel/src/prompt_lineage.rs`

Thin wrapper around `CausalGraph` providing prompt-evolution-specific APIs:
- `record_mutation(parent, child, reflection, score)`
- `record_merge(parents, child, score)`
- `record_deployment(candidate, environment)`
- `trace_lineage(candidate_id) -> Vec<CausalNode>` (full ancestry)
- `detect_regression(candidate_id, window) -> Option<RegressionAlert>`

### Phase 4: Governance Integration (Sprint 14)

**Goal**: Connect evolution to WeftOS governance.

#### Modifications to: `clawft-core/src/agent/skill_autogen.rs`

Add evolution step to the skill generation pipeline:
```rust
pub struct SkillCandidate {
    // ... existing fields ...

    /// Evolution metadata (None if skill was not evolved).
    pub evolution: Option<EvolutionMetadata>,
}

pub struct EvolutionMetadata {
    pub generations: u32,
    pub evaluations: usize,
    pub original_score: f32,
    pub evolved_score: f32,
    pub lineage_node_id: u64,
}
```

#### Governance checks before deployment:
1. Score exceeds `min_improvement` threshold
2. Semantic distance from parent below `max_drift` (via HNSW)
3. No flagged patterns in evolved text (injection, prompt leaking)
4. Evolution budget within allocation
5. Human approval if `require_approval` is true

### Implementation Sketch: `pipeline/learner.rs` with Prompt Evolution

```rust
//! Evolutionary learning backend (Level 1+ implementation).
//!
//! Replaces NoopLearner. Records trajectories, detects quality patterns,
//! and triggers GEPA-style prompt evolution when performance degrades.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::traits::{LearningBackend, LearningSignal, Trajectory};

/// Configuration for the evolutionary learner.
pub struct EvolutionaryLearnerConfig {
    /// Maximum trajectories to retain in the ring buffer.
    pub max_trajectories: usize,
    /// Score threshold below which a trajectory is "poor".
    pub poor_threshold: f32,
    /// Minimum poor trajectories before triggering evolution.
    pub evolution_trigger_count: usize,
    /// How often to check for evolution triggers (in trajectory count).
    pub check_interval: usize,
}

impl Default for EvolutionaryLearnerConfig {
    fn default() -> Self {
        Self {
            max_trajectories: 1000,
            poor_threshold: 0.6,
            evolution_trigger_count: 10,
            check_interval: 50,
        }
    }
}

struct LearnerState {
    trajectories: VecDeque<ScoredTrajectory>,
    total_recorded: u64,
    poor_count: usize,
    evolution_pending: bool,
}

struct ScoredTrajectory {
    trajectory: Trajectory,
    feedback: String,
    recorded_at: u64,
}

/// Evolutionary learning backend.
///
/// Records trajectories with quality scores and triggers evolution
/// when performance degrades below threshold.
pub struct EvolutionaryLearner {
    config: EvolutionaryLearnerConfig,
    state: Mutex<LearnerState>,
    // evolver: Arc<PromptEvolver>,  // injected dependency
    // causal: Arc<CausalGraph>,     // for lineage tracking
}

impl EvolutionaryLearner {
    pub fn new(config: EvolutionaryLearnerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(LearnerState {
                trajectories: VecDeque::new(),
                total_recorded: 0,
                poor_count: 0,
                evolution_pending: false,
            }),
        }
    }

    /// Get the N poorest trajectories for reflection input.
    pub fn get_poor_trajectories(&self, n: usize) -> Vec<ScoredTrajectory> {
        let state = self.state.lock().unwrap();
        let mut poor: Vec<_> = state.trajectories.iter()
            .filter(|t| t.trajectory.quality.overall < self.config.poor_threshold)
            .cloned()
            .collect();
        poor.sort_by(|a, b| a.trajectory.quality.overall
            .partial_cmp(&b.trajectory.quality.overall)
            .unwrap_or(std::cmp::Ordering::Equal));
        poor.truncate(n);
        poor
    }

    /// Check if evolution should be triggered.
    fn should_evolve(&self, state: &LearnerState) -> bool {
        state.poor_count >= self.config.evolution_trigger_count
            && !state.evolution_pending
            && state.total_recorded % self.config.check_interval as u64 == 0
    }
}

impl LearningBackend for EvolutionaryLearner {
    fn record(&self, trajectory: &Trajectory) {
        let mut state = self.state.lock().unwrap();

        // Generate feedback text from quality dimensions
        let feedback = format!(
            "Overall: {:.2}, Relevance: {:.2}, Coherence: {:.2}. {}",
            trajectory.quality.overall,
            trajectory.quality.relevance,
            trajectory.quality.coherence,
            if trajectory.quality.overall < self.config.poor_threshold {
                "Below quality threshold -- candidate for reflection."
            } else {
                "Acceptable quality."
            }
        );

        let scored = ScoredTrajectory {
            trajectory: trajectory.clone(),
            feedback,
            recorded_at: state.total_recorded,
        };

        // Track poor trajectories
        if trajectory.quality.overall < self.config.poor_threshold {
            state.poor_count += 1;
        }

        // Ring buffer
        state.trajectories.push_back(scored);
        if state.trajectories.len() > self.config.max_trajectories {
            let removed = state.trajectories.pop_front().unwrap();
            if removed.trajectory.quality.overall < self.config.poor_threshold {
                state.poor_count = state.poor_count.saturating_sub(1);
            }
        }

        state.total_recorded += 1;

        // Check evolution trigger
        if self.should_evolve(&state) {
            state.evolution_pending = true;
            // In production: emit ImpulseType::PromptEvolution to the
            // DEMOCRITUS loop, which will schedule the evolution run
            // on the next cognitive tick with available budget.
            tracing::info!(
                poor_count = state.poor_count,
                total = state.total_recorded,
                "evolution trigger: scheduling prompt evolution run"
            );
        }
    }

    fn adapt(&self, signal: &LearningSignal) {
        // User feedback amplifies or dampens the evolution trigger.
        // Negative feedback (thumbs_down) lowers the threshold temporarily.
        // Positive feedback (thumbs_up) raises it.
        let mut state = self.state.lock().unwrap();
        if signal.value < 0.0 {
            state.poor_count += 2; // accelerate evolution trigger
        }
        // Positive signals are recorded but don't suppress evolution --
        // the Pareto frontier handles balancing exploration vs exploitation.
    }
}
```

### File Summary: What to Modify / Create

| Action | File | Description |
|--------|------|-------------|
| **Modify** | `clawft-core/src/pipeline/scorer.rs` | Replace NoopScorer with HeuristicScorer |
| **Modify** | `clawft-core/src/pipeline/learner.rs` | Replace NoopLearner with EvolutionaryLearner |
| **Create** | `clawft-core/src/pipeline/feedback.rs` | ScoredFeedback + TraceEntry types |
| **Create** | `clawft-core/src/pipeline/evolver.rs` | GEPA-style evolution engine |
| **Modify** | `clawft-core/src/pipeline/mod.rs` | Add `pub mod evolver; pub mod feedback;` |
| **Modify** | `clawft-kernel/src/causal.rs` | Add MutatedTo, MergedInto, ProducedOutcome, InformedBy edge types |
| **Create** | `clawft-kernel/src/prompt_lineage.rs` | Prompt lineage tracking via CausalGraph |
| **Modify** | `clawft-kernel/src/lib.rs` | Add `pub mod prompt_lineage;` |
| **Modify** | `clawft-core/src/agent/skill_autogen.rs` | Add EvolutionMetadata to SkillCandidate |
| **Modify** | `clawft-core/src/pipeline/traits.rs` | Add `feedback: Option<String>` to QualityScore |

### Dependency Additions

The evolution engine needs LLM access for reflection and mutation. It should
use the existing `LlmTransport` trait from the pipeline, routed through the
same cost tracker and rate limiter. No new external dependencies are required
-- the engine is pure Rust orchestration over existing pipeline infrastructure.

For the Pareto frontier implementation, a small utility crate or module
handles dominance checking -- this is straightforward comparison logic,
not requiring external optimization libraries.

---

## Sources

- [GEPA Paper (arxiv:2507.19457)](https://arxiv.org/abs/2507.19457)
- [GEPA GitHub Repository](https://github.com/gepa-ai/gepa)
- [dspy.GEPA Optimizer Documentation](https://dspy.ai/api/optimizers/GEPA/overview/)
- [GEPA DSPy Tutorial](https://dspy.ai/tutorials/gepa_ai_program/)
- [NousResearch/hermes-agent-self-evolution](https://github.com/NousResearch/hermes-agent-self-evolution)
- [Hermes Agent GitHub](https://github.com/NousResearch/hermes-agent)
- [GEPA on Emergent Mind](https://www.emergentmind.com/papers/2507.19457)
- [GEPA on OpenReview (ICLR 2026)](https://openreview.net/forum?id=RQm2KQTM5r)
- [PromptBreeder Paper](https://arxiv.org/abs/2309.16797)
- [SCOPE: Prompt Evolution for Agents](https://arxiv.org/abs/2512.15374)
- [EvoAgentX Framework](https://github.com/EvoAgentX/EvoAgentX)
- [NVIDIA OpenShell for Safe Self-Evolving Agents](https://developer.nvidia.com/blog/run-autonomous-self-evolving-agents-more-safely-with-nvidia-openshell/)
- [GAAPO: Genetic Algorithm Applied to Prompt Optimization](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1613007/full)
- [Survey of Automatic Prompt Optimization](https://arxiv.org/pdf/2502.18746)
