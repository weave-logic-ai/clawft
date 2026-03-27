# WeftOS Kernel: Local Inference & Continuous Model Improvement

```
ID:          W-KERNEL-11
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Local Inference & Continuous Model Improvement
Status:      Proposed
Date:        2026-02-28
Depends-On:  10-agent-first-single-user.md, 09-environments-and-learning.md, 07-ruvector-deep-integration.md
```

---

## 1. Core Insight: Every Agent Can Think Locally

The existing WeftOS architecture (docs 07-10) establishes that every service IS an
agent, agents communicate via IPC, and the SONA engine provides self-learning
capabilities. What is missing is the final link: agents carrying their own local
inference models and those models improving continuously through operational feedback.

Today, every agent decision that requires language understanding routes to an
external LLM API. This creates three problems:

1. **Latency**: Cloud roundtrips add 500ms-5s per inference call. For agents making
   dozens of routing, classification, and triage decisions per task, this compounds
   into seconds of pure wait time.

2. **Cost**: Every inference call costs money. Simple decisions (classify this error,
   route this message, format this output) do not need 100B+ parameter models.

3. **Availability**: Air-gapped nodes, edge deployments, and network partitions
   cannot reason at all without cloud connectivity.

The solution is local GGUF model inference via the `ruvllm` crate from the ruvector
ecosystem. Small quantized models (0.5B parameters, Q4_K_M, ~300MB on disk, ~1GB in
RAM) provide sub-5ms inference for simple tasks. The `ruvltra-claude-code` model on
HuggingFace is purpose-built for this: swarm-native, SONA self-learning compatible,
and tuned for agent decision-making.

**Key principles:**

1. **Tiered routing with real GGUF execution.** ADR-026 defines a 3-tier model routing
   system (Agent Booster / Haiku / Sonnet). This document extends it to 4 tiers by
   making Tier 1 actual GGUF inference instead of just WASM pattern matching. The
   `ruvector-tiny-dancer-core` FastGRNN router decides which tier handles each request
   in sub-millisecond time.

2. **Agents carry their own models.** An agent manifest (`.agent.toml`) can declare
   a local GGUF model. The agent uses this model for fast local decisions and falls
   back to cloud models when confidence is low.

3. **Models improve through operations.** SONA records every inference outcome as a
   trajectory. Accumulated trajectories feed into fine-tuning. New GGUF versions are
   generated, evaluated against quality gates, and deployed through the environment
   governance pipeline from doc 09.

4. **Air-gapped independence.** A node with loaded GGUF models can reason locally
   with zero network latency and zero cost. When connectivity returns, accumulated
   learning deltas sync to the cluster.

---

## 2. Inference Service Agent

The inference service is a kernel service agent (doc 10) that manages GGUF model
loading, inference execution, and model lifecycle. It follows the agent-first
architecture: other agents request inference via IPC topics.

```toml
# .agents/inference-service.agent.toml

[agent]
name = "inference-service"
version = "0.1.0"
description = "Local GGUF model inference service"
role = "service"

[capabilities]
tools = [
    "inference_complete",
    "inference_embed",
    "inference_classify",
    "model_list",
    "model_load",
    "model_unload",
]
ipc_scope = "topic"
topics_publish = ["inference.result", "inference.metrics"]
topics_subscribe = ["inference.request"]
can_spawn = true  # spawns worker threads per model
filesystem_access = ["models/", "data/weights/"]

[resources]
max_memory_mb = 2048  # models live in memory
priority = "high"

[interface]
protocol = "ipc"
request_topic = "inference.request"
response_mode = "direct"

[health]
check_interval = "10s"
timeout = "3s"
restart_policy = "always"

[dependencies]
requires = ["memory-service"]
after = ["kernel-init"]
```

The inference service is an **optional** core service. Nodes without local models
skip it entirely. Nodes with models start it after `memory-service` so that
inference results can be cached in persistent memory.

---

## 3. Model-Per-Agent Pattern

### 3.0 Most Agents Are Model-Free

**The `[inference]` section is entirely optional.** Most agents do not need a local
model and should not declare one. The tiered routing system is a capability available
to agents that need it, not a requirement.

Agents that are model-free:

- **Worker agents** -- execute specific tools, transform data, run scripts. They work
  for other agents and need no inference of their own.
- **OpenClaw-style agents** -- tool-calling agents that receive instructions from a
  coordinator. The coordinator (or user) provides the reasoning; the worker executes.
- **Claude/Codex agent definitions** -- agent manifests that wrap external LLM APIs.
  From WeftOS's perspective, these agents *are* inference providers, not consumers.
  They expose their LLM as a service via IPC, and other agents call them.
- **Service agents** -- message-bus, cron, health-monitor, tool-registry. These are
  deterministic services that do not need language understanding.
- **Container/sidecar agents** -- wrappers around external services (databases, APIs).

Agents that benefit from local models:

- **Routing agents** -- need fast classification to dispatch work to the right agent.
- **Triage agents** -- need to categorize incoming requests by urgency/type.
- **Code analysis agents** -- need pattern recognition for common code smells.
- **Security scanning agents** -- need classification of input as safe/suspicious.

The inference-service itself is an optional kernel service. Nodes that have no GGUF
models and no agents with `[inference]` sections simply skip starting it.

**Configuring at the node level:**

```toml
# weft.toml (per-node configuration)

[inference]
enabled = true                           # false = no inference service on this node
default_model = "models/ruvltra-0.5b-q4_k_m.gguf"  # shared model for any agent
max_memory_mb = 2048                     # memory budget for all loaded models
cloud_fallback = true                    # allow cloud tier fallback
cloud_tiers = ["haiku", "sonnet"]        # which cloud tiers are available

# Or disable entirely:
# [inference]
# enabled = false
```

### 3.1 Declaring a Local Model (Optional)

Agents that *do* need inference declare it in their manifest:

```toml
# .agents/code-reviewer.agent.toml

[agent]
name = "code-reviewer"
role = "worker"

# No [inference] section needed if this agent doesn't do local inference.
# It can still request inference from the inference-service via IPC if needed.

[inference]
# OPTIONAL: This agent has its own local model for fast classification
model = "models/ruvltra-0.5b-code-review-q4_k_m.gguf"
fallback = "haiku"          # if local model confidence < threshold
confidence_threshold = 0.7
max_tokens = 256
learning_enabled = true     # record outcomes for future fine-tuning
```

Agents without `[inference]` can still request inference via IPC to the
inference-service (if it is running) or directly to a Claude/Codex agent.
The difference is ownership: agents with `[inference]` have a dedicated model
loaded for them; agents without it share the node's inference-service or call
cloud tiers explicitly.

### 3.1 Core Types

```rust
/// Configuration for an agent's local inference capability.
pub struct AgentInferenceConfig {
    /// Path to GGUF model file (relative to models/ directory)
    pub model_path: PathBuf,
    /// Fallback to cloud model when confidence is low
    pub fallback: Option<ModelTier>,
    /// Minimum confidence to use local result (0.0-1.0)
    pub confidence_threshold: f64,
    /// Max tokens for local inference
    pub max_tokens: usize,
    /// Whether to fine-tune this model from agent's operations
    pub learning_enabled: bool,
}

/// The four inference tiers available in WeftOS.
pub enum ModelTier {
    /// T0: Agent Booster -- WASM pattern matching rules (<0.05ms, $0)
    /// Handles simple transforms: var->const, regex, formatting
    AgentBooster,

    /// T1: Local GGUF model via ruvllm (<5ms, $0, ~0.5B params)
    /// Handles classification, routing, simple generation
    Local { model_path: PathBuf },

    /// T2: Cloud Haiku (~500ms, low cost, ~20B params)
    /// Handles moderate complexity: code review, summarization
    Haiku,

    /// T3: Cloud Sonnet/Opus (2-5s, medium-high cost, ~100B+ params)
    /// Handles complex reasoning: architecture, security analysis
    Sonnet,

    /// T3+: Cloud Opus (2-5s, highest cost, highest capability)
    /// Reserved for the most complex tasks
    Opus,
}

/// A request for inference from any agent.
pub struct InferenceRequest {
    /// Unique request identifier
    pub id: String,
    /// PID of the requesting agent
    pub from: Pid,
    /// The prompt or input text
    pub prompt: String,
    /// Preferred tier (None = auto-route via FastGRNN)
    pub tier: Option<ModelTier>,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Sampling temperature
    pub temperature: f64,
}

/// Result returned from any inference tier.
pub struct InferenceResult {
    /// Matches the request ID
    pub id: String,
    /// Generated text
    pub text: String,
    /// Which tier actually handled the request
    pub tier_used: ModelTier,
    /// Model confidence in the result (0.0-1.0)
    pub confidence: f64,
    /// Number of tokens consumed
    pub tokens_used: usize,
    /// Wall-clock latency in milliseconds
    pub latency_ms: f64,
    /// Model version string (e.g., "ruvltra-0.5b-code-review-v1.2.0-q4_k_m")
    pub model_version: String,
}
```

### 3.2 Agent Lifecycle with Models

When an agent with an `[inference]` section spawns:

```
1. Root agent reads .agent.toml, finds [inference] section
2. Root agent sends IPC to inference-service:
   topic: "inference.request"
   payload: ModelLoadRequest { path: "models/ruvltra-0.5b-code-review-q4_k_m.gguf", for_agent: pid }
3. Inference service loads model into memory (if not already loaded)
4. Inference service registers model handle for that agent PID
5. Agent spawns and can now call inference_complete/embed/classify tools

When the agent exits:
6. Inference service decrements reference count for that model
7. If no agents reference the model, it is eligible for unloading
8. Unload policy: LRU eviction when total model memory exceeds max_memory_mb
```

---

## 4. Tiered Model Routing

The routing system extends ADR-026 from 3 tiers to 4 tiers with real GGUF execution
at Tier 1.

### 4.1 Tier Overview

| Tier | Handler | Latency | Cost | Params | Use Cases |
|------|---------|---------|------|--------|-----------|
| T0 | Agent Booster (WASM rules) | <0.05ms | $0 | N/A | Simple transforms (var->const, regex, format) |
| T1 | Local GGUF (ruvllm) | <5ms | $0 | 0.5B | Classification, routing, simple generation |
| T2 | Cloud Haiku | ~500ms | $0.0002 | ~20B | Moderate complexity, code review |
| T3 | Cloud Sonnet/Opus | 2-5s | $0.003-0.015 | ~100B+ | Architecture, security analysis, complex reasoning |

### 4.2 Routing Logic

```rust
/// Routes inference requests to the optimal tier.
pub struct TieredRouter {
    /// FastGRNN router from ruvector-tiny-dancer-core (<1ms routing decisions)
    fast_router: ruvector_tiny_dancer_core::Router,

    /// Local GGUF models currently loaded in inference-service
    local_models: Vec<LoadedModel>,

    /// Agent Booster pattern rules (compiled WASM)
    booster_patterns: Vec<BoosterPattern>,

    /// SONA learning engine for improving routing decisions over time
    sona: sona::SonaEngine,

    /// Circuit breaker for cloud tiers (prevents cascading failures)
    circuit_breaker: ruvector_tiny_dancer_core::CircuitBreaker,
}

impl TieredRouter {
    /// Route a request to the best tier. Called by inference-service on every request.
    pub async fn route(&self, request: &InferenceRequest) -> ModelTier {
        // 0. If caller specified a tier, honor it (explicit override)
        if let Some(ref tier) = request.tier {
            return tier.clone();
        }

        // 1. Check if agent booster can handle it (pattern match, <0.05ms)
        if self.is_boostable(&request.prompt) {
            return ModelTier::AgentBooster;
        }

        // 2. FastGRNN complexity estimation (<1ms)
        let complexity = self.fast_router.estimate_complexity(&request.prompt);

        // 3. Route based on complexity score
        let tier = match complexity {
            c if c < 0.3 => {
                // Low complexity: use local GGUF if available
                if let Some(model) = self.best_local_model(&request) {
                    ModelTier::Local { model_path: model.path.clone() }
                } else {
                    // No local model available, fall back to Haiku
                    ModelTier::Haiku
                }
            }
            c if c < 0.6 => {
                // Medium complexity: cloud Haiku
                if self.circuit_breaker.is_open("haiku") {
                    // Haiku circuit breaker open, try local with lower confidence
                    if let Some(model) = self.best_local_model(&request) {
                        ModelTier::Local { model_path: model.path.clone() }
                    } else {
                        ModelTier::Sonnet
                    }
                } else {
                    ModelTier::Haiku
                }
            }
            _ => {
                // High complexity: cloud Sonnet/Opus
                ModelTier::Sonnet
            }
        };

        // 4. Record routing decision for SONA learning
        self.sona.record_trajectory_step(TrajectoryStep {
            input: request.prompt.clone(),
            decision: format!("{:?}", tier),
            complexity,
            timestamp: std::time::Instant::now(),
        });

        tier
    }

    /// Check if the prompt matches an Agent Booster pattern (T0).
    fn is_boostable(&self, prompt: &str) -> bool {
        self.booster_patterns.iter().any(|p| p.matches(prompt))
    }

    /// Find the best local model for this request based on domain and availability.
    fn best_local_model(&self, request: &InferenceRequest) -> Option<&LoadedModel> {
        self.local_models
            .iter()
            .filter(|m| m.is_loaded && m.domain_matches(&request.prompt))
            .max_by(|a, b| a.accuracy_score.partial_cmp(&b.accuracy_score).unwrap())
    }
}

/// A GGUF model currently loaded in memory.
pub struct LoadedModel {
    /// Filesystem path to the GGUF file
    pub path: PathBuf,
    /// Whether the model is currently loaded in memory
    pub is_loaded: bool,
    /// Model domain (e.g., "code-review", "routing", "classification")
    pub domain: String,
    /// Accuracy score from most recent evaluation (0.0-1.0)
    pub accuracy_score: f64,
    /// Memory usage in bytes
    pub memory_bytes: usize,
    /// Model version
    pub version: String,
    /// Number of agents currently referencing this model
    pub ref_count: usize,
}
```

### 4.3 Confidence-Based Fallback

When a local model produces a result with low confidence, the system automatically
escalates to a higher tier:

```rust
impl InferenceService {
    pub async fn handle_request(&self, request: InferenceRequest) -> Result<InferenceResult> {
        let tier = self.router.route(&request).await;

        let result = match &tier {
            ModelTier::AgentBooster => self.run_booster(&request)?,
            ModelTier::Local { model_path } => {
                let local_result = self.run_local(model_path, &request).await?;

                // Check confidence against agent's threshold
                let threshold = self.get_agent_threshold(request.from);
                if local_result.confidence < threshold {
                    // Escalate to fallback tier
                    let fallback = self.get_agent_fallback(request.from);
                    self.run_cloud(&request, fallback).await?
                } else {
                    local_result
                }
            }
            ModelTier::Haiku | ModelTier::Sonnet | ModelTier::Opus => {
                self.run_cloud(&request, tier).await?
            }
        };

        // Record outcome for learning
        self.learning_loop.record_inference(&request, &result);

        Ok(result)
    }
}
```

---

## 5. Continuous Model Improvement Lifecycle

This is the central contribution of this document. Local models are not static
artifacts -- they improve through a continuous cycle driven by operational data.

```
+------------------------------------------------------------------+
|                CONTINUOUS IMPROVEMENT CYCLE                        |
|                                                                    |
|   +---------+    +----------+    +---------+    +------------+    |
|   |  TRAIN  |--->|  DEPLOY  |--->|  LEARN  |--->|  EVALUATE  |    |
|   |         |    |          |    |         |    |            |    |
|   | Fine-   |    | Push new |    | SONA    |    | Compare    |    |
|   | tune    |    | GGUF to  |    | records |    | against    |    |
|   | base    |    | agents   |    | trajec- |    | baseline   |    |
|   | model   |    | in dev   |    | tories  |    | metrics    |    |
|   +----^----+    +----------+    +---------+    +-----+------+    |
|        |                                              |           |
|        |         +----------+    +---------+          |           |
|        +---------| RETRAIN  |<---|  DECIDE |<---------+           |
|                  |          |    |         |                      |
|                  | Generate |    | Meets   |                      |
|                  | new GGUF |    | quality |                      |
|                  | with     |    | gate?   |                      |
|                  | improved |    |         |                      |
|                  | weights  |    |         |                      |
|                  +----------+    +---------+                      |
+------------------------------------------------------------------+
```

### 5.1 Training Phase

Training takes a base GGUF model and fine-tunes it using operational data collected
by SONA. The fine-tuning uses MicroLoRA (rank-1 for fast iteration) or BaseLoRA
(rank-8 for quality) with EWC++ regularization to prevent catastrophic forgetting.

```rust
/// Configuration for a model training run.
pub struct ModelTrainingConfig {
    /// Base model to fine-tune from (path to GGUF)
    pub base_model: PathBuf,
    /// Training data source (SONA trajectories, tool patterns, task completions)
    pub training_data: TrainingDataSource,
    /// LoRA rank for fine-tuning (1 = MicroLoRA fast, 8 = BaseLoRA quality)
    pub lora_rank: usize,
    /// EWC++ regularization strength (higher = more preservation of old knowledge)
    pub ewc_lambda: f64,
    /// Target quantization level for the output GGUF
    pub quantization: QuantizationLevel,
    /// Quality gate thresholds the trained model must pass
    pub quality_gates: QualityGates,
    /// Maximum training duration before timeout
    pub max_duration: Duration,
    /// Output path for the trained GGUF
    pub output_path: PathBuf,
}

/// Where the training data comes from.
pub enum TrainingDataSource {
    /// Learn from agent operation trajectories (SONA TrajectoryBuilder output)
    Trajectories {
        agent_id: String,
        min_score: f64,
    },

    /// Learn from verified tool call patterns (successful tool invocations)
    ToolPatterns {
        tool_names: Vec<String>,
    },

    /// Learn from successful task completions
    TaskCompletions {
        min_success_rate: f64,
    },

    /// Combined sources with weighted mixing
    Combined(Vec<(TrainingDataSource, f64)>),
}

/// GGUF quantization levels for model output.
pub enum QuantizationLevel {
    /// Full precision -- research and evaluation only
    F16,
    /// 8-bit quantization -- high quality, 2x compression
    Q8_0,
    /// 4-bit medium quantization -- good balance of quality and size, 4x compression
    Q4_K_M,
    /// 4-bit small quantization -- maximum compression at 4-bit, 4x compression
    Q4_K_S,
    /// 2-bit quantization -- extreme compression, 8x, quality loss expected
    Q2_K,
}
```

### 5.2 Deployment Phase

Trained models are deployed through the environment governance pipeline (doc 09).
Each deployment carries metadata for traceability and rollback.

```rust
/// A model deployment descriptor.
pub struct ModelDeployment {
    /// Model version (semver, e.g., "1.2.0")
    pub version: String,
    /// GGUF file hash for integrity verification (BLAKE3)
    pub hash: [u8; 32],
    /// Which environment to deploy to (Development, Staging, Production)
    pub target_environment: EnvironmentClass,
    /// Canary percentage: what fraction of agents receive this model (0-100)
    pub canary_percent: u8,
    /// Condition that triggers automatic rollback
    pub rollback_on: RollbackCondition,
    /// DID of the training agent that produced this model
    pub signed_by: Did,
    /// Timestamp of training completion
    pub trained_at: DateTime<Utc>,
    /// Base model this was fine-tuned from
    pub base_model_version: String,
    /// Summary of training data used
    pub training_data_summary: String,
}

/// Conditions that trigger automatic model rollback.
pub enum RollbackCondition {
    /// Accuracy drops below threshold on live inference
    AccuracyBelow(f64),
    /// P99 latency exceeds threshold in milliseconds
    LatencyAbove { p99_ms: f64 },
    /// Error rate exceeds threshold (0.0-1.0)
    ErrorRateAbove(f64),
    /// Multiple conditions (all must trigger)
    All(Vec<RollbackCondition>),
    /// Multiple conditions (any triggers)
    Any(Vec<RollbackCondition>),
    /// Manual rollback only (no automatic triggers)
    Manual,
}
```

### 5.3 Learning Phase (SONA Integration)

During normal operations, every inference request and its outcome are recorded by
SONA as trajectory data. This data accumulates between training cycles and feeds
into the next fine-tuning run.

```rust
/// Per-agent learning loop that records inference outcomes for future training.
pub struct AgentLearningLoop {
    /// SONA engine instance for this agent
    pub sona: sona::SonaEngine,
    /// Current model version being used by this agent
    pub model_version: String,
    /// Active trajectory (one per task/session)
    pub trajectory: sona::TrajectoryBuilder,
    /// Accumulated MicroLoRA weight deltas since last sync
    pub pending_deltas: Vec<LoRADelta>,
    /// Metrics aggregated since last sync
    pub metrics: InferenceMetrics,
}

impl AgentLearningLoop {
    /// Record an inference outcome for future training.
    pub fn record_outcome(
        &mut self,
        request: &InferenceRequest,
        result: &InferenceResult,
        success: bool,
    ) {
        // 1. Record trajectory step (every outcome feeds future training)
        self.trajectory.add_step(TrajectoryStep {
            input: request.prompt.clone(),
            output: result.text.clone(),
            tier_used: result.tier_used.clone(),
            success,
            latency_ms: result.latency_ms,
            confidence: result.confidence,
        });

        // 2. If using local model, compute MicroLoRA delta for online adaptation
        if matches!(result.tier_used, ModelTier::Local { .. }) {
            let delta = self.sona.compute_micro_lora_delta(
                &request.prompt,
                &result.text,
                success,
            );
            self.pending_deltas.push(delta);
        }

        // 3. Update ReasoningBank with pattern for similarity search
        self.sona.reasoning_bank.store_pattern(
            request.prompt.clone(),
            result.text.clone(),
            success,
        );

        // 4. Update running metrics
        self.metrics.record(result);
    }

    /// Sync accumulated deltas to training-service agent via IPC.
    /// Called periodically or when delta buffer exceeds threshold.
    pub async fn sync_deltas(&mut self, ipc: &dyn IpcTransport) -> Result<()> {
        let deltas = std::mem::take(&mut self.pending_deltas);
        if deltas.is_empty() {
            return Ok(());
        }

        // Send deltas to training-service via IPC topic
        let message = KernelMessage::new(
            MessageTarget::Service("training-service"),
            MessagePayload::Custom(serde_json::to_value(DeltaSyncPayload {
                agent_id: self.sona.agent_id().to_string(),
                model_version: self.model_version.clone(),
                deltas,
                metrics: self.metrics.snapshot(),
            })?),
        );

        ipc.send(message).await?;
        self.metrics.reset();

        Ok(())
    }
}

/// Running metrics for inference quality tracking.
pub struct InferenceMetrics {
    pub total_requests: u64,
    pub local_requests: u64,
    pub cloud_requests: u64,
    pub local_success_rate: f64,
    pub avg_local_confidence: f64,
    pub avg_local_latency_ms: f64,
    pub fallback_rate: f64,
    pub since: DateTime<Utc>,
}
```

### 5.4 Evaluation and Quality Gates

Before a trained model can be promoted beyond Development, it must pass quality
gates. The gates are evaluated by the training-service agent using a held-out
evaluation dataset.

```rust
/// Quality gates that a trained model must pass for promotion.
pub struct QualityGates {
    /// Minimum accuracy on the evaluation set
    pub min_accuracy: f64,
    /// Maximum latency regression vs baseline model (percentage)
    pub max_latency_regression_percent: f64,
    /// Minimum number of evaluation samples
    pub min_eval_samples: usize,
    /// Must pass coherence check via prime-radiant SheafLaplacian
    pub coherence_check: bool,
    /// Must not regress on previously learned tasks (EWC++ validation)
    pub catastrophic_forgetting_check: bool,
    /// Maximum memory increase vs baseline model (percentage)
    pub max_memory_regression_percent: f64,
}

/// Results of running a model through quality gates.
pub struct QualityGateResults {
    pub model_version: String,
    pub baseline_version: String,
    pub accuracy: f64,
    pub latency_regression_percent: f64,
    pub eval_samples: usize,
    pub coherence_score: f64,
    pub forgetting_score: f64,
    pub memory_regression_percent: f64,
    pub all_gates_passed: bool,
    pub gate_details: Vec<GateResult>,
}

pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub actual_value: f64,
    pub threshold: f64,
    pub detail: String,
}
```

### 5.5 Distribution Phase

Models that pass quality gates are distributed to agents. The distribution strategy
determines how aggressively new models roll out.

```rust
/// How trained models reach agents.
pub struct ModelDistribution {
    /// Distribution strategy
    pub strategy: DistributionStrategy,
    /// Model registry for version tracking
    pub registry: ModelRegistry,
}

/// Strategies for rolling out new model versions.
pub enum DistributionStrategy {
    /// Push new model to all agents immediately (Development only)
    Push,
    /// Agents pull new model on next restart (low-risk, simple)
    PullOnRestart,
    /// Canary: deploy to a percentage of agents, compare metrics over duration
    Canary {
        percent: u8,
        duration_hours: u32,
    },
    /// Blue-green: run old and new models in parallel, compare results
    BlueGreen {
        comparison_duration_hours: u32,
    },
}

/// Central registry of all model versions.
pub struct ModelRegistry {
    /// Where GGUF files are stored on the filesystem
    pub storage_path: PathBuf,
    /// Index of all model versions
    pub index: Vec<ModelVersion>,
}

/// A single model version in the registry.
pub struct ModelVersion {
    /// Semver version string
    pub version: String,
    /// BLAKE3 hash of the GGUF file
    pub hash: [u8; 32],
    /// Base model this was fine-tuned from
    pub base_model: String,
    /// Human-readable summary of training data
    pub training_data_summary: String,
    /// Quality gate evaluation results
    pub eval_results: QualityGateResults,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// DID of the training agent that produced it
    pub signed_by: Did,
    /// Current lifecycle status
    pub status: ModelStatus,
    /// File size in bytes
    pub file_size_bytes: u64,
    /// Quantization level
    pub quantization: QuantizationLevel,
}

/// Lifecycle status of a model version.
pub enum ModelStatus {
    /// Currently being trained
    Training,
    /// Training complete, running evaluation
    Evaluating,
    /// Deployed to a subset of agents (canary)
    Canary,
    /// Deployed to all agents in the target environment
    Active,
    /// Superseded by a newer version but still available
    Deprecated,
    /// Rolled back due to regression
    RolledBack,
}
```

---

## 6. Environment-Gated Model Promotion

Model promotion follows the environment governance from doc 09. Each environment
has different policies for model deployment, reflecting the risk profile.

```
Development --train--> Staging --validate--> Production
    |                     |                     |
    | Train freely        | Quality gates       | Blue-green only
    | Experiment          | Canary deploy       | Auto-rollback
    | All agents learn    | Compare vs baseline | Proven models only
    | SONA: Explore       | SONA: Validate      | SONA: Exploit
```

### 6.1 Development Environment

- Any agent can experiment with new GGUF models
- No quality gates required for deployment
- SONA operates in Explore mode (high entropy, try novel approaches)
- Training runs freely with any data source
- Push deployment strategy (instant rollout)
- All inference outcomes recorded for training data

### 6.2 Staging Environment

- New model versions must pass all quality gates
- Canary deployment required (deploy to 10-20% of agents first)
- SONA operates in Validate mode (test hypotheses from Development)
- Metrics compared against baseline model for minimum soak time
- Automatic rollback if accuracy drops or latency regresses
- Human notification on model promotion requests

### 6.3 Production Environment

- Only models that passed Staging validation are eligible
- Blue-green deployment: run old and new models in parallel
- Human approval required for major version bumps
- Automatic rollback on any regression (accuracy, latency, error rate)
- SONA operates in Exploit mode (proven patterns only, no exploration)
- Every inference recorded with full effect vector for audit

```rust
/// Per-environment model deployment policy.
pub struct EnvironmentModelPolicy {
    pub environment: EnvironmentClass,
    pub allowed_strategy: Vec<DistributionStrategy>,
    pub quality_gates_required: bool,
    pub min_canary_duration_hours: Option<u32>,
    pub human_approval_required: bool,
    pub auto_rollback_enabled: bool,
    pub sona_learning_mode: LearningMode,
}

impl EnvironmentModelPolicy {
    pub fn development() -> Self {
        Self {
            environment: EnvironmentClass::Development,
            allowed_strategy: vec![DistributionStrategy::Push],
            quality_gates_required: false,
            min_canary_duration_hours: None,
            human_approval_required: false,
            auto_rollback_enabled: false,
            sona_learning_mode: LearningMode::Explore,
        }
    }

    pub fn staging() -> Self {
        Self {
            environment: EnvironmentClass::Staging,
            allowed_strategy: vec![
                DistributionStrategy::Canary { percent: 10, duration_hours: 4 },
                DistributionStrategy::Canary { percent: 20, duration_hours: 4 },
            ],
            quality_gates_required: true,
            min_canary_duration_hours: Some(4),
            human_approval_required: false,
            auto_rollback_enabled: true,
            sona_learning_mode: LearningMode::Validate,
        }
    }

    pub fn production() -> Self {
        Self {
            environment: EnvironmentClass::Production,
            allowed_strategy: vec![
                DistributionStrategy::BlueGreen { comparison_duration_hours: 24 },
            ],
            quality_gates_required: true,
            min_canary_duration_hours: Some(24),
            human_approval_required: true,
            auto_rollback_enabled: true,
            sona_learning_mode: LearningMode::Exploit,
        }
    }
}
```

---

## 7. Cross-Node Model Sync

For distributed WeftOS deployments (K6+), models and learning data must synchronize
across nodes. This uses the same delta-consensus CRDTs from doc 07.

```rust
/// Cross-node model synchronization using ruvector-delta-consensus.
pub struct CrossNodeModelSync {
    /// Delta-consensus CRDT for weight synchronization
    pub delta_sync: ruvector_delta_consensus::DeltaConsensus,
    /// How often to sync weight deltas between nodes
    pub sync_interval: Duration,
    /// Maximum delta size before triggering a full model push
    pub max_delta_size_bytes: usize,
    /// Model registry shared across nodes
    pub shared_registry: Arc<ModelRegistry>,
}

impl CrossNodeModelSync {
    /// Sync accumulated LoRA deltas to peer nodes.
    pub async fn sync_deltas(&self, local_deltas: Vec<LoRADelta>) -> Result<()> {
        // 1. Package deltas as a CRDT delta
        let crdt_delta = self.delta_sync.create_delta(
            serde_json::to_vec(&local_deltas)?,
        );

        // 2. Gossip delta to peers
        self.delta_sync.gossip(crdt_delta).await?;

        Ok(())
    }

    /// Receive deltas from a peer node.
    pub async fn receive_deltas(&self, peer_delta: CausalDelta) -> Result<()> {
        // 1. Merge into local state (CRDT guarantees convergence)
        self.delta_sync.merge(peer_delta)?;

        // 2. Check if accumulated deltas exceed threshold
        let total_size = self.delta_sync.pending_size();
        if total_size > self.max_delta_size_bytes {
            // Trigger full model regeneration with merged deltas
            self.trigger_retrain().await?;
        }

        Ok(())
    }

    /// Trigger a full retrain to incorporate all accumulated deltas.
    async fn trigger_retrain(&self) -> Result<()> {
        // Send retrain request to local training-service agent
        // The training service merges all deltas into a new GGUF
        // New GGUF is then distributed through the normal deployment pipeline
        Ok(())
    }
}
```

**Sync behavior by scenario:**

- **Connected cluster**: Deltas sync at `sync_interval` (default: 60s). Models
  converge across nodes within minutes.
- **Edge nodes**: Deltas accumulate locally. When the edge node reconnects, it
  sends all accumulated deltas in one batch.
- **Air-gapped nodes**: Deltas accumulate indefinitely. When the node is physically
  connected for maintenance, deltas sync. The node continues to use its local model
  for inference in the meantime.
- **Partition recovery**: CRDT merge guarantees convergence regardless of message
  ordering or duplication. No coordination protocol needed.

---

## 8. Training Service Agent

The training service is a dedicated agent for model fine-tuning, evaluation, and
lifecycle management. It aggregates learning deltas from all agents and orchestrates
the continuous improvement cycle.

```toml
# .agents/training-service.agent.toml

[agent]
name = "training-service"
version = "0.1.0"
description = "Model training, evaluation, and lifecycle management"
role = "service"

[capabilities]
tools = [
    "model_train",
    "model_evaluate",
    "model_export",
    "model_promote",
    "model_rollback",
    "training_status",
]
ipc_scope = "topic"
topics_publish = ["training.complete", "training.progress", "model.new_version"]
topics_subscribe = ["training.request", "learning.deltas"]
can_spawn = true  # spawns training workers for parallel evaluation
filesystem_access = ["models/", "data/training/", "data/trajectories/"]

[resources]
max_memory_mb = 4096  # training requires loading base model + computing gradients
priority = "normal"   # training is background work, not latency-critical

[interface]
protocol = "ipc"
request_topic = "training.request"
response_mode = "direct"

[health]
check_interval = "30s"
timeout = "10s"
restart_policy = "on-failure"

[dependencies]
requires = ["inference-service", "memory-service"]
after = ["inference-service"]
```

### 8.1 Training Service Responsibilities

1. **Delta aggregation**: Collects MicroLoRA deltas from all agents via
   `learning.deltas` topic. Aggregates into a combined training dataset.

2. **Training orchestration**: Schedules fine-tuning runs when sufficient deltas
   accumulate. Uses configurable thresholds (minimum delta count, minimum time
   since last training, minimum quality improvement potential).

3. **Evaluation**: Runs trained models through quality gates using held-out
   evaluation datasets. Reports results via `training.complete` topic.

4. **Promotion**: Manages model promotion through environments (Development ->
   Staging -> Production). Enforces per-environment deployment policies.

5. **Rollback**: Monitors deployed models for regression. Triggers automatic
   rollback when rollback conditions are met.

6. **Registry maintenance**: Manages the model registry. Cleans up deprecated
   and rolled-back model versions according to retention policy.

---

## 9. ruvllm Integration

The `ruvllm` crate from the ruvector ecosystem provides Rust-native GGUF loading
and inference. `clawft-kernel` wraps it behind an `InferenceBackend` trait to
maintain the adapter pattern from doc 07 (depend, do not fork; wrap, do not expose).

```rust
/// Trait abstracting over inference backends.
/// No ruvector type appears in the public API.
pub trait InferenceBackend: Send + Sync {
    /// Load a GGUF model from the filesystem.
    async fn load_model(
        &self,
        path: &Path,
        config: &ModelConfig,
    ) -> Result<ModelHandle>;

    /// Run text completion on a loaded model.
    async fn complete(
        &self,
        handle: &ModelHandle,
        prompt: &str,
        max_tokens: usize,
        temperature: f64,
    ) -> Result<CompletionResult>;

    /// Generate embeddings for text.
    async fn embed(
        &self,
        handle: &ModelHandle,
        text: &str,
    ) -> Result<Vec<f32>>;

    /// Classify text into one of the provided categories.
    async fn classify(
        &self,
        handle: &ModelHandle,
        text: &str,
        categories: &[String],
    ) -> Result<ClassificationResult>;

    /// Unload a model from memory.
    async fn unload(&self, handle: &ModelHandle) -> Result<()>;

    /// Get information about a loaded model.
    fn model_info(&self, handle: &ModelHandle) -> ModelInfo;

    /// Get total memory usage across all loaded models.
    fn total_memory_usage(&self) -> usize;
}

/// Configuration for loading a model.
pub struct ModelConfig {
    /// Number of GPU layers to offload (0 = CPU only)
    pub n_gpu_layers: u32,
    /// Context window size in tokens
    pub context_size: usize,
    /// Number of threads for inference
    pub n_threads: usize,
    /// Whether to use memory-mapped I/O for model loading
    pub use_mmap: bool,
}

/// Opaque handle to a loaded model.
pub struct ModelHandle {
    id: String,
    path: PathBuf,
    loaded_at: DateTime<Utc>,
}

/// Result of a completion request.
pub struct CompletionResult {
    pub text: String,
    pub tokens_used: usize,
    pub confidence: f64,
    pub latency_ms: f64,
}

/// Result of a classification request.
pub struct ClassificationResult {
    pub category: String,
    pub confidence: f64,
    pub all_scores: Vec<(String, f64)>,
}

/// Information about a loaded model.
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub parameters: u64,
    pub quantization: QuantizationLevel,
    pub memory_bytes: usize,
    pub context_size: usize,
}

// --------------------------------------------------------------------------
// Backend implementations
// --------------------------------------------------------------------------

/// ruvllm backend for native builds. Provides real GGUF inference.
#[cfg(feature = "native")]
pub struct RuvllmBackend {
    // Wraps ruvllm GGUF loading and inference
    // All ruvllm types are private to this struct
}

#[cfg(feature = "native")]
impl InferenceBackend for RuvllmBackend {
    // Full implementation using ruvllm crate
    // Model loading: ruvllm::load_model(path, config)
    // Inference: ruvllm::complete(handle, prompt, params)
    // Embeddings: ruvllm::embed(handle, text)
    // ...
}

/// Stub backend for browser/WASM builds. All inference routes to cloud.
#[cfg(feature = "browser")]
pub struct CloudOnlyBackend {
    // No local inference capability
    // All requests return Err(InferenceError::LocalNotAvailable)
    // Caller must use cloud tiers (Haiku/Sonnet/Opus)
}

#[cfg(feature = "browser")]
impl InferenceBackend for CloudOnlyBackend {
    async fn load_model(&self, _path: &Path, _config: &ModelConfig) -> Result<ModelHandle> {
        Err(InferenceError::LocalNotAvailable(
            "GGUF inference is not available in browser builds".to_string()
        ))
    }

    // All other methods similarly return LocalNotAvailable
    // This is expected: browser agents always route to cloud tiers
}
```

---

## 10. New Files

| File | Phase | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/inference.rs` | K0 | `InferenceBackend` trait, `InferenceService`, `ModelTier`, `TieredRouter` |
| `crates/clawft-kernel/src/inference_ruvllm.rs` | K0 | `RuvllmBackend` implementation (feature-gated `native`) |
| `crates/clawft-kernel/src/model_registry.rs` | K5 | `ModelRegistry`, `ModelVersion`, `ModelStatus`, `ModelDistribution` |
| `crates/clawft-kernel/src/training.rs` | K5 | `TrainingService`, `ModelTrainingConfig`, `QualityGates` |
| `crates/clawft-kernel/src/model_sync.rs` | K6 | `CrossNodeModelSync`, delta weight distribution |
| `crates/clawft-kernel/src/learning_loop.rs` | K0 | `AgentLearningLoop`, `InferenceMetrics` (extends doc 09) |
| `.agents/inference-service.agent.toml` | K0 | Inference service agent manifest |
| `.agents/training-service.agent.toml` | K5 | Training service agent manifest |

---

## 11. Impact on Existing Phases

| Phase | Impact |
|-------|--------|
| K0 | Add `inference-service` as optional core service agent. Inference backend loaded during kernel boot if models are present. |
| K1 | Model access as a capability: `AgentCapabilities` gains `inference_scope` field. Agents declare which models they can use in `.agent.toml`. |
| K2 | Inference requests routed via IPC topics (`inference.request` / `inference.result`). Fits existing A2A protocol. |
| K3 | WASM tools can call inference-service via IPC. Sandboxed tools do not load models directly -- they request inference through the service agent. |
| K5 | Training-service agent added. Model registry, promotion pipeline, and quality gates. SONA integration for continuous improvement. |
| K6 | Cross-node model sync via delta-consensus CRDTs. LoRA deltas gossip between nodes. Model registry shared across cluster. |
| ADR-026 | Tier 1 becomes real GGUF execution via ruvllm, not just WASM pattern rules. 3-tier model becomes 4-tier (T0 Booster / T1 Local / T2 Haiku / T3 Sonnet+Opus). |

---

## 12. CLI Commands

### Model Management

```
weft model list                           -- list available GGUF models in registry
weft model load <path>                    -- load a GGUF model into inference service
weft model unload <name>                  -- unload model from memory
weft model status                         -- show loaded models, memory usage, request stats
weft model versions <name>                -- show version history for a model
weft model rollback <name> <version>      -- rollback to a previous model version
weft model diff <v1> <v2>                 -- compare two model versions (eval metrics)
weft model gc                             -- garbage-collect deprecated/rolled-back models
```

### Inference

```
weft inference <prompt>                   -- run inference (auto-routes to best tier)
weft inference --tier local <prompt>      -- force local GGUF model
weft inference --tier haiku <prompt>      -- force cloud Haiku
weft inference --tier sonnet <prompt>     -- force cloud Sonnet
weft inference --agent <name> <prompt>    -- use a specific agent's model
weft inference stats                      -- show inference stats (tier distribution, latency, cost)
```

### Training

```
weft training start --base <model> --data trajectories  -- start a training run
weft training status                      -- show current training progress
weft training evaluate <model>            -- run evaluation suite on a model
weft training promote <model> staging     -- promote model to staging environment
weft training promote <model> prod        -- promote model to production (requires quality gates)
weft training history                     -- show training run history
weft training deltas                      -- show accumulated learning deltas per agent
```

---

## 13. Testing Strategy

| Test | Description |
|------|-------------|
| `model_load_unload` | Load a GGUF model into inference service, verify memory allocation, unload, verify memory freed |
| `tiered_routing_complexity` | Low complexity prompt routes to local, high complexity routes to cloud |
| `tiered_routing_no_local` | When no local model is loaded, low complexity falls back to Haiku |
| `inference_request_ipc` | Agent sends inference request via IPC topic, receives result on response topic |
| `confidence_fallback` | Local model result below confidence threshold triggers fallback to cloud tier |
| `sona_trajectory_recording` | Inference outcomes are recorded as SONA trajectory steps |
| `micro_lora_delta_generation` | Local inference outcomes produce MicroLoRA weight deltas |
| `model_quality_gates` | New model must pass accuracy, latency, coherence, and forgetting gates |
| `quality_gate_failure` | Model that fails any gate is rejected with detailed report |
| `canary_deployment` | Deploy to 10% of agents, verify only canary agents use new model |
| `rollback_on_regression` | Detect accuracy drop below threshold, auto-rollback to previous version |
| `delta_sync_merge` | Two nodes accumulate deltas independently, merge produces consistent state |
| `delta_sync_partition` | Node accumulates deltas during partition, syncs cleanly on reconnect |
| `ewc_no_forgetting` | New training does not regress accuracy on previously learned task set |
| `air_gapped_inference` | Node with no network connectivity runs local inference successfully |
| `model_registry_lifecycle` | Model version transitions: Training -> Evaluating -> Canary -> Active -> Deprecated |
| `agent_model_ref_counting` | Model loaded when first agent needs it, unloaded when last agent exits |
| `browser_backend_fallback` | Browser build returns LocalNotAvailable, caller routes to cloud |
| `concurrent_inference` | Multiple agents issue concurrent inference requests without deadlock |
| `training_service_delta_aggregation` | Training service correctly aggregates deltas from multiple agents |

---

## 14. Risks and Mitigations

| ID | Risk | Severity | Mitigation |
|----|------|----------|------------|
| R1 | 0.5B model quality insufficient for many tasks | Medium | Tiered routing ensures cloud fallback for anything above simple classification. Local model handles only T1 tasks. Confidence threshold triggers escalation. |
| R2 | Memory pressure from multiple loaded models | High | Inference service enforces `max_memory_mb` limit. LRU eviction unloads least-recently-used models. Reference counting prevents unloading models in active use. |
| R3 | Training data poisoning from adversarial agents | Medium | Trajectory validation via prime-radiant coherence check. Only learn from verified outcomes (success=true). Training service sanitizes input data. |
| R4 | Model version sprawl consuming disk space | Low | Model registry with lifecycle (Active/Deprecated/RolledBack). `weft model gc` auto-cleans old versions. Configurable retention policy. |
| R5 | Catastrophic forgetting during fine-tuning | Medium | EWC++ regularization preserves important weights. Quality gate includes forgetting check against held-out eval set from previous training runs. |
| R6 | GGUF format incompatibility across ruvllm versions | Low | Pin ruvllm version in Cargo.toml. Test format compatibility in CI. Keep format version in ModelVersion metadata. |
| R7 | LoRA delta divergence across nodes in large clusters | Medium | Delta-consensus CRDTs ensure eventual consistency. Periodic full model sync when delta size exceeds threshold. Convergence verified by comparing model hashes after sync. |
| R8 | Training runs consume excessive CPU/memory on production nodes | Medium | Training-service has `priority = "normal"` (background). Can be configured to run only on designated training nodes. Resource limits enforced by kernel budget system. |
| R9 | FastGRNN router makes poor routing decisions early | Low | Router starts with conservative defaults (route to cloud when uncertain). SONA learning improves routing over time. Manual tier override always available via `--tier` flag. |
| R10 | Model loading latency on cold start delays agent spawn | Medium | Pre-warm frequently-used models during kernel boot. Lazy loading with first-request penalty as alternative. Memory-mapped I/O for faster loading. |

---

## 15. Cross-References

| Document | Relationship |
|----------|-------------|
| `07-ruvector-deep-integration.md` | ruvllm crate dependency. SONA engine integration. FastGRNN router from ruvector-tiny-dancer-core. Feature gate pattern follows doc 07 conventions. |
| `08-ephemeral-os-architecture.md` | Cross-node model sync aligns with ephemeral node lifecycle. Air-gapped inference supports disconnected nodes. Model registry is content-addressed like the cryptographic filesystem. |
| `09-environments-and-learning.md` | Model promotion follows environment governance gates (Development -> Staging -> Production). SONA learning modes (Explore/Validate/Exploit) apply to model training, not just agent actions. Quality gates extend the PromotionGate enum. |
| `10-agent-first-single-user.md` | Inference-service and training-service are agent services defined by `.agent.toml` manifests. They follow the service agent pattern (PID allocation, IPC communication, health monitoring). Model-per-agent pattern extends the agent manifest format. |
| `00-orchestrator.md` | Adds inference-service to K0 optional services. Adds training-service and model registry to K5 scope. Adds cross-node model sync to K6 scope. |
| ADR-026 (3-Tier Model Routing) | Extends to 4 tiers with actual GGUF execution at T1. Agent Booster (T0) remains WASM pattern matching. Cloud tiers (T2, T3) unchanged. Routing logic now uses FastGRNN instead of static rules. |

---

## 16. Definition of Done

### For this architecture document

- [x] Core insight documented: agents carry local models, models improve over time
- [x] Inference service agent defined with manifest and capabilities
- [x] Model-per-agent pattern with TOML configuration
- [x] 4-tier routing system specified with FastGRNN integration
- [x] Continuous improvement lifecycle (Train -> Deploy -> Learn -> Evaluate -> Retrain)
- [x] SONA integration for trajectory recording and MicroLoRA deltas
- [x] Quality gates for model promotion
- [x] Environment-gated deployment policies (Development/Staging/Production)
- [x] Cross-node model sync via delta-consensus CRDTs
- [x] Training service agent defined with manifest
- [x] ruvllm integration behind InferenceBackend trait (wrap, do not expose)
- [x] CLI commands for model management, inference, and training
- [x] Testing strategy with unit and integration test cases
- [x] Risk assessment with mitigations
- [x] Cross-references to all dependent documents

### For implementation (future phases)

- [ ] `ruvllm` crate compiles and loads a GGUF model on target Rust version
- [ ] Inference service agent boots and handles requests via IPC
- [ ] Tiered routing correctly classifies prompts by complexity
- [ ] Confidence-based fallback triggers cloud escalation
- [ ] SONA records inference trajectories and produces MicroLoRA deltas
- [ ] Training service aggregates deltas and produces new GGUF
- [ ] Quality gates pass/fail correctly on test models
- [ ] Model promotion pipeline enforces environment policies
- [ ] Cross-node delta sync converges in test cluster
- [ ] All tests in Section 13 pass
- [ ] No ruvllm type appears in clawft-kernel public API
- [ ] Browser/WASM build unaffected (CloudOnlyBackend compiles cleanly)
