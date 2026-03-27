# WeftOS Kernel: Environment-Scoped Governance & Self-Learning Loop

```
ID:          W-KERNEL-09
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Environment-Scoped Governance & Self-Learning Loop
Status:      Proposed
Date:        2026-02-28
Depends-On:  08-ephemeral-os-architecture.md, 07-ruvector-deep-integration.md
```

---

## 1. Overview

WeftOS environments are not just deployment targets -- they are **governance scopes**
with different risk thresholds, capability sets, and learning policies. The same agent
identity (DID) operates across environments but with capabilities scoped to each
environment's governance rules.

Every action taken in any environment is cryptographically recorded in the exochain DAG,
creating an immutable audit trail that doubles as training data for the SONA self-learning
engine. This creates a closed loop: agents get smarter over time while governance
boundaries remain immutable.

---

## 2. Environment Model

### 2.1 Environment Definition

```rust
pub struct Environment {
    /// Unique environment identifier
    pub id: EnvironmentId,

    /// Human-readable name
    pub name: String,

    /// Environment class determines base governance rules
    pub class: EnvironmentClass,

    /// Nodes that belong to this environment
    pub nodes: Vec<NodeId>,

    /// Governance scope (risk thresholds, approval requirements)
    pub governance: GovernanceScope,

    /// Storage backends available in this environment
    pub storage: Vec<StorageLocation>,

    /// Agent capability overrides for this environment
    pub capability_overrides: HashMap<AgentRole, CapabilityOverride>,

    /// Cross-environment workflow policies
    pub promotion_policy: PromotionPolicy,
}

pub enum EnvironmentClass {
    /// Full autonomy. Agents can experiment freely.
    /// Risk threshold: 0.9 (almost anything allowed)
    Development,

    /// Deployed builds, automated testing. Agents test but don't innovate.
    /// Risk threshold: 0.6 (moderate gating)
    Staging,

    /// Live systems. Strong gating, human approval for high-risk actions.
    /// Risk threshold: 0.3 (strict gating)
    Production,

    /// Custom environment with explicit risk threshold
    Custom { name: String, risk_threshold: f64 },
}
```

### 2.2 Governance Scope per Environment

| Property | Development | Staging | Production |
|----------|-------------|---------|------------|
| **Risk threshold** | 0.9 | 0.6 | 0.3 |
| **Human approval** | Never | On deploy | All high-risk |
| **Rollback authority** | Any agent | DevOps agents | Human + DevOps |
| **Data access** | Synthetic/test only | Anonymized copies | Real data, audited |
| **Network scope** | Dev cluster only | Staging cluster | Production cluster |
| **Learning mode** | Explore (high entropy) | Validate (test hypotheses) | Exploit (proven patterns only) |
| **Audit granularity** | Summary per session | Per-action | Per-action + effect vector |
| **Governance branch** | Exec only (fast) | Exec + Judicial | All three branches |

### 2.3 Governance Scope Type

```rust
pub struct GovernanceScope {
    /// Risk threshold: actions above this are blocked or escalated
    /// Development=0.9, Staging=0.6, Production=0.3
    pub risk_threshold: f64,

    /// Whether human approval is required for actions above threshold
    pub human_approval_required: bool,

    /// Which governance branches are active
    pub active_branches: GovernanceBranches,

    /// How detailed the audit trail needs to be
    pub audit_level: AuditLevel,

    /// SONA learning mode for this environment
    pub learning_mode: LearningMode,

    /// Maximum effect vector magnitude before escalation
    pub max_effect_magnitude: f64,
}

pub struct GovernanceBranches {
    /// Legislative: SOP/rule definition (always on)
    pub legislative: bool,
    /// Executive: agent actions (always on)
    pub executive: bool,
    /// Judicial: CGR validation (off in dev for speed)
    pub judicial: bool,
}

pub enum AuditLevel {
    /// One summary record per agent session
    SessionSummary,
    /// One record per action taken
    PerAction,
    /// Per-action plus full 5D effect vector
    PerActionWithEffects,
}

pub enum LearningMode {
    /// High entropy: try novel approaches, learn from failures
    /// Used in development environments
    Explore,
    /// Medium entropy: test hypotheses from explore phase
    /// Used in staging environments
    Validate,
    /// Low entropy: only use proven patterns
    /// Used in production environments
    Exploit,
}
```

### 2.4 Same Agent, Different Capabilities

A single agent (identified by DID) can operate across environments but with
capabilities scoped to each environment's governance:

```rust
pub struct CapabilityOverride {
    /// Tools allowed in this environment
    pub allowed_tools: Option<Vec<String>>,

    /// Tools explicitly denied in this environment
    pub denied_tools: Option<Vec<String>>,

    /// Maximum resource budget in this environment
    pub resource_limits: Option<ResourceLimits>,

    /// IPC scope override for this environment
    pub ipc_scope: Option<IpcScope>,

    /// Whether the agent can spawn sub-agents
    pub can_spawn: bool,

    /// Whether the agent can modify infrastructure
    pub can_modify_infra: bool,

    /// Whether the agent can access external APIs
    pub can_access_external: bool,
}
```

**Example**: A DevOps agent in development can:
- Deploy to any server, restart any service, delete test data
- Spawn unlimited sub-agents for parallel testing
- Access external APIs freely

The same DevOps agent in production can:
- Deploy only after staging validation passes
- Restart services only with human approval
- Cannot delete any data (tombstone only)
- Spawn sub-agents up to a limit
- External API access logged and rate-limited

---

## 3. Cross-Environment Workflows

### 3.1 Promotion Pipeline

Artifacts (code, configs, models) flow through environments via promotion:

```
Development ──build──> Staging ──validate──> Production
    │                     │                     │
    │ risk_threshold=0.9  │ risk_threshold=0.6  │ risk_threshold=0.3
    │ learning=Explore    │ learning=Validate   │ learning=Exploit
    │ audit=Summary       │ audit=PerAction     │ audit=PerAction+Effects
    │                     │                     │
    └─ agents innovate    └─ agents test        └─ agents operate safely
```

### 3.2 Promotion Policy

```rust
pub struct PromotionPolicy {
    /// Source environment
    pub from: EnvironmentId,

    /// Target environment
    pub to: EnvironmentId,

    /// Gates that must pass before promotion
    pub gates: Vec<PromotionGate>,

    /// Who can approve promotion
    pub approvers: ApproverSet,
}

pub enum PromotionGate {
    /// All tests pass in source environment
    TestsPassing,
    /// Staging deployment ran for minimum duration
    MinimumSoakTime(Duration),
    /// Effect vector below threshold across all actions
    EffectBelowThreshold { dimension: String, max: f64 },
    /// Human reviewed and approved
    HumanApproval,
    /// Governance CGR check passed
    CgrValidation,
    /// No regressions detected by monitoring agents
    NoRegressions,
    /// Custom gate (evaluated by rule engine)
    Custom(String),
}

pub enum ApproverSet {
    /// Any agent with the specified role
    Role(AgentRole),
    /// Specific DID(s) must approve
    Specific(Vec<Did>),
    /// Human user must approve
    Human,
    /// Multiple approvers required (quorum)
    Quorum { required: usize, from: Vec<Did> },
}
```

### 3.3 Workflow Example: Feature Deployment

```
1. Engineering agent writes code in Development
   - CGR judicial branch: OFF (fast iteration)
   - Learning mode: Explore (try novel approaches)
   - Risk threshold: 0.9 (almost anything allowed)

2. Engineering agent submits PR, triggers promotion to Staging
   Gate: TestsPassing (all unit/integration tests)
   Gate: CgrValidation (governance rules check)
   Approver: Role(Engineering) -- any engineer agent can approve

3. DevOps agent deploys to Staging
   - CGR judicial branch: ON (validate governance compliance)
   - Learning mode: Validate (test hypotheses)
   - Risk threshold: 0.6 (moderate gating)
   - Monitoring agent watches for regressions

4. After soak time, DevOps agent requests promotion to Production
   Gate: MinimumSoakTime(4 hours)
   Gate: NoRegressions (monitoring agent confirms)
   Gate: EffectBelowThreshold("risk", 0.3)
   Gate: HumanApproval (human reviews deployment)
   Approver: Human (must be a human user)

5. DevOps agent deploys to Production
   - All three governance branches: ON
   - Learning mode: Exploit (proven patterns only)
   - Risk threshold: 0.3 (strict gating)
   - Every action recorded with full effect vector
   - Human notified of all high-risk operations
```

---

## 4. Self-Learning Governance Loop

### 4.1 The Core Loop

Every action in any environment feeds back into the learning system:

```
    ┌─────────────────────────────────────────────┐
    │                                             │
    │  ACT         Agent takes action             │
    │   │          (filtered by governance)        │
    │   v                                         │
    │  RECORD      Immutable exochain entry        │
    │   │          (BLAKE3 hash + Ed25519 sig)     │
    │   v                                         │
    │  EVALUATE    Effect algebra scoring           │
    │   │          (risk, fairness, privacy,        │
    │   │           novelty, security)              │
    │   v                                         │
    │  DECIDE      CGR engine validates            │
    │   │          (Permit / Defer / Deny)          │
    │   v                                         │
    │  LEARN       SONA trajectory update           │
    │   │          (MicroLoRA weight delta)         │
    │   v                                         │
    │  IMPROVE     Cross-environment propagation    │
    │   │          (delta-consensus CRDT sync)      │
    │   │                                         │
    │   └─────────────────> ACT (next cycle)       │
    │                                             │
    └─────────────────────────────────────────────┘
```

### 4.2 Constitutional Invariants (Immutable)

These rules can NEVER be changed by any agent, in any environment:

```rust
pub struct ConstitutionalInvariants {
    /// 1. Every action is immutably recorded in the exochain DAG
    ///    No agent can modify or delete audit entries
    pub immutable_audit: bool, // always true, compile-time enforced

    /// 2. Human oversight is required for actions above production risk threshold
    ///    No agent can bypass human approval for high-risk production actions
    pub human_oversight_for_high_risk: bool, // always true

    /// 3. Self-modification requires supermajority consensus
    ///    No single agent can change governance rules
    ///    Changes require 2/3 of governance-capable agents + human approval
    pub supermajority_for_self_modification: bool, // always true

    /// 4. Learning cannot weaken governance boundaries
    ///    SONA optimization must stay within the risk thresholds
    ///    An agent cannot learn its way out of production restrictions
    pub learning_respects_boundaries: bool, // always true

    /// 5. Identity is cryptographically bound to actions
    ///    No agent can act without signing with its DID key
    pub actions_require_signature: bool, // always true
}
```

### 4.3 What Can Be Learned (Within Boundaries)

While constitutional invariants are immutable, agents learn to be **more effective
within those boundaries**:

| Learnable | Example | SONA Component |
|-----------|---------|----------------|
| Better tool selection | "For deploy tasks, prefer canary over big-bang" | MicroLoRA routing weights |
| Faster error diagnosis | "This error pattern means X, fix is Y" | ReasoningBank verdicts |
| Optimal resource usage | "This workload needs 2GB, not 4GB" | EWC++ preserved knowledge |
| Risk prediction | "Changes to auth module are higher risk" | TrajectoryBuilder patterns |
| Cross-agent coordination | "DevOps should notify Monitor before deploy" | Delta-consensus shared state |
| Approval routing | "Alice approves security, Bob approves infra" | MicroLoRA dispatch weights |

### 4.4 Learning Data Flow

```rust
pub struct GovernanceTrajectory {
    /// The action that was taken
    pub action: ActionRecord,

    /// Effect vector at time of action
    pub effect: EffectVector,

    /// Governance decision (Permit/Defer/Deny)
    pub decision: GateDecision,

    /// Outcome (success, failure, partial, timeout)
    pub outcome: ActionOutcome,

    /// Environment where this occurred
    pub environment: EnvironmentId,

    /// Exochain DAG entry hash (immutable reference)
    pub dag_hash: Hash,

    /// HLC timestamp (causal ordering)
    pub timestamp: HybridLogicalClock,
}

pub struct ActionRecord {
    /// Agent DID who performed the action
    pub agent: Did,

    /// What was done (tool call, IPC message, deployment, etc.)
    pub action_type: ActionType,

    /// Input parameters
    pub inputs: serde_json::Value,

    /// Output/result (if completed)
    pub output: Option<serde_json::Value>,

    /// Duration of the action
    pub duration: Duration,
}

pub struct EffectVector {
    /// Risk score (0.0 = safe, 1.0 = dangerous)
    pub risk: f64,
    /// Fairness score (0.0 = biased, 1.0 = fair)
    pub fairness: f64,
    /// Privacy impact (0.0 = no exposure, 1.0 = full exposure)
    pub privacy: f64,
    /// Novelty score (0.0 = routine, 1.0 = unprecedented)
    pub novelty: f64,
    /// Security impact (0.0 = hardening, 1.0 = weakening)
    pub security: f64,
}

pub enum ActionOutcome {
    /// Action completed successfully
    Success { metrics: HashMap<String, f64> },
    /// Action failed with error
    Failure { error: String, recoverable: bool },
    /// Action partially completed
    Partial { completed: f64, reason: String },
    /// Action timed out
    Timeout { elapsed: Duration },
    /// Action was rolled back
    RolledBack { reason: String },
}
```

### 4.5 Cross-Environment Learning Propagation

Learning flows **upward** through environments, not downward:

```
Development ──proven patterns──> Staging ──validated patterns──> Production
    ^                              ^                              │
    │                              │                              │
    │         observations         │       observations           │
    │         from prod            │       from prod              │
    └──────────────────────────────┴──────────────────────────────┘
                    (read-only feedback, not direct learning)
```

**Rules**:
1. Development can learn anything (Explore mode)
2. Staging can only validate patterns that succeeded in Development
3. Production can only apply patterns that passed Staging validation
4. Production observations flow back to Development as new exploration seeds
5. No environment can directly inject learned patterns into Production

```rust
pub struct LearningPropagation {
    /// Source environment where pattern was learned
    pub source: EnvironmentId,

    /// Target environment where pattern should be applied
    pub target: EnvironmentId,

    /// The learned pattern (SONA trajectory)
    pub pattern: TrajectoryPattern,

    /// Confidence score (must exceed target env threshold)
    pub confidence: f64,

    /// Number of successful applications in source
    pub evidence_count: usize,

    /// Exochain DAG hashes of evidence entries
    pub evidence_refs: Vec<Hash>,

    /// Governance decision on propagation
    pub approved: Option<GateDecision>,
}

pub struct TrajectoryPattern {
    /// What situation triggers this pattern
    pub trigger: PatternTrigger,

    /// What action the pattern recommends
    pub recommendation: ActionType,

    /// Expected effect vector
    pub expected_effect: EffectVector,

    /// MicroLoRA weight delta to apply
    pub lora_delta: Vec<f32>,

    /// Confidence interval
    pub confidence_interval: (f64, f64),
}
```

---

## 5. Production Environment Deep Dive

### 5.1 Production Governance Rules

Production environments have the strongest governance, reflecting that live
systems serve real users:

```rust
pub struct ProductionGovernance {
    /// Every action is recorded with full effect vector
    pub audit: AuditLevel, // always PerActionWithEffects

    /// All three governance branches active
    pub branches: GovernanceBranches, // all true

    /// Risk threshold is low (strict)
    pub risk_threshold: f64, // 0.3

    /// Actions above threshold require human approval
    pub escalation: EscalationPolicy,

    /// Continuous monitoring by dedicated agents
    pub monitoring: MonitoringPolicy,

    /// Automatic rollback on anomaly detection
    pub rollback: RollbackPolicy,
}

pub struct EscalationPolicy {
    /// Risk score that triggers escalation
    pub threshold: f64,

    /// Who to escalate to
    pub escalation_chain: Vec<EscalationTarget>,

    /// Maximum time to wait for approval before auto-deny
    pub timeout: Duration,

    /// What happens if no one responds
    pub timeout_action: TimeoutAction,
}

pub enum EscalationTarget {
    /// Another agent (e.g., Security agent reviews DevOps action)
    Agent(Did),
    /// Human user
    Human(String),
    /// Governance committee (quorum required)
    Committee { name: String, quorum: usize },
}

pub enum TimeoutAction {
    /// Deny the action (safe default)
    Deny,
    /// Allow with elevated audit
    AllowWithAudit,
    /// Queue for next available approver
    Queue,
}
```

### 5.2 Production Learning Constraints

```rust
pub struct ProductionLearningPolicy {
    /// Only exploit proven patterns (no exploration)
    pub mode: LearningMode, // always Exploit

    /// Minimum confidence for applying a pattern
    pub min_confidence: f64, // 0.95

    /// Minimum evidence count from staging
    pub min_evidence: usize, // 100

    /// Patterns must have been validated in staging for this long
    pub min_staging_soak: Duration, // 7 days

    /// Maximum effect vector magnitude for automated application
    pub max_auto_effect: f64, // 0.2

    /// Above this effect magnitude, human must approve pattern application
    pub human_approval_threshold: f64, // 0.5
}
```

### 5.3 Every Production Decision as Training Data

Production is where the system generates the highest-quality learning signal:

```
Production Action
    │
    ├─ GovernanceTrajectory record (exochain DAG)
    │   ├─ Action type, inputs, outputs
    │   ├─ Effect vector (5D scoring)
    │   ├─ Gate decision (Permit/Defer/Deny)
    │   └─ Outcome (success/failure/timeout/rollback)
    │
    ├─ Immutable storage (BLAKE3 hash, Ed25519 signature)
    │   └─ Cannot be modified or deleted by any agent
    │
    ├─ Real-time metrics feed to Monitoring agents
    │   ├─ Latency, error rates, resource usage
    │   └─ Anomaly detection via prime-radiant coherence engine
    │
    └─ Periodic learning extraction
        ├─ SONA trajectory analysis (batch, not real-time)
        ├─ Pattern extraction (what worked, what failed)
        ├─ Feedback to Development as exploration seeds
        └─ Governance rule refinement proposals (require supermajority)
```

---

## 6. Implementation Strategy

### 6.1 Additions to Existing Phases

| Phase | Addition | Description |
|-------|----------|-------------|
| K1 | `EnvironmentId` in `AgentCapabilities` | Capabilities scoped by environment |
| K1 | `GovernanceScope` in `Kernel` config | Per-environment risk thresholds |
| K2 | `EnvironmentId` in `KernelMessage` | Messages tagged with source environment |
| K5 | `environments` in `weftapp.toml` | App declares supported environments |
| K6.4 | `GovernanceScope` enforcement | Cross-node governance per environment |

### 6.2 New Module: `environment.rs`

```rust
// crates/clawft-kernel/src/environment.rs

/// Environment registry that maps environment IDs to governance scopes
pub struct EnvironmentRegistry {
    environments: DashMap<EnvironmentId, Environment>,
    node_assignments: DashMap<NodeId, EnvironmentId>,
}

impl EnvironmentRegistry {
    /// Get the governance scope for an agent in a specific environment
    pub fn resolve_capabilities(
        &self,
        agent: &AgentUser,
        env: &EnvironmentId,
    ) -> Result<AgentCapabilities> {
        let env = self.environments.get(env)
            .ok_or(EnvironmentError::NotFound)?;

        let mut caps = agent.capabilities.clone();

        // Apply environment-specific overrides
        if let Some(overrides) = env.capability_overrides.get(&agent.role) {
            caps.apply_overrides(overrides);
        }

        // Enforce governance scope
        caps.risk_threshold = env.governance.risk_threshold;
        caps.learning_mode = env.governance.learning_mode.clone();
        caps.audit_level = env.governance.audit_level.clone();

        Ok(caps)
    }

    /// Check if an action is allowed in this environment
    pub fn check_action(
        &self,
        agent: &Did,
        env: &EnvironmentId,
        action: &ActionRecord,
        effect: &EffectVector,
    ) -> GateDecision {
        let scope = self.environments.get(env)
            .map(|e| &e.governance);

        match scope {
            Some(scope) => {
                let magnitude = effect.magnitude();
                if magnitude > scope.risk_threshold {
                    if scope.human_approval_required {
                        GateDecision::Defer // escalate to human
                    } else {
                        GateDecision::Deny
                    }
                } else {
                    GateDecision::Permit
                }
            }
            None => GateDecision::Deny, // unknown environment = deny
        }
    }
}
```

### 6.3 New Module: `learning_loop.rs`

```rust
// crates/clawft-kernel/src/learning_loop.rs

/// Orchestrates the self-learning governance loop
pub struct LearningLoop {
    /// SONA engine for pattern learning
    sona: Arc<SonaEngine>,

    /// Exochain DAG for immutable recording
    dag: Arc<ExochainDag>,

    /// Environment registry for scope resolution
    environments: Arc<EnvironmentRegistry>,

    /// Governance trajectory buffer (batched for efficiency)
    trajectory_buffer: Arc<Mutex<Vec<GovernanceTrajectory>>>,

    /// Flush interval for trajectory buffer
    flush_interval: Duration,
}

impl LearningLoop {
    /// Record an action and its outcome
    pub async fn record(
        &self,
        action: ActionRecord,
        effect: EffectVector,
        decision: GateDecision,
        outcome: ActionOutcome,
        environment: EnvironmentId,
    ) -> Result<Hash> {
        // 1. Create immutable DAG entry
        let trajectory = GovernanceTrajectory {
            action,
            effect,
            decision,
            outcome,
            environment,
            dag_hash: Hash::default(), // filled by DAG append
            timestamp: HybridLogicalClock::now(),
        };

        let dag_hash = self.dag.append(&trajectory).await?;

        // 2. Buffer for batch learning
        let mut buffer = self.trajectory_buffer.lock().await;
        buffer.push(GovernanceTrajectory { dag_hash, ..trajectory });

        // 3. Flush if buffer is full
        if buffer.len() >= 100 {
            self.flush_batch(&mut buffer).await?;
        }

        Ok(dag_hash)
    }

    /// Extract patterns from buffered trajectories
    async fn flush_batch(
        &self,
        buffer: &mut Vec<GovernanceTrajectory>,
    ) -> Result<()> {
        let trajectories = std::mem::take(buffer);

        // Group by environment
        let by_env: HashMap<EnvironmentId, Vec<_>> = trajectories
            .into_iter()
            .fold(HashMap::new(), |mut acc, t| {
                acc.entry(t.environment.clone()).or_default().push(t);
                acc
            });

        for (env_id, env_trajectories) in by_env {
            let env = self.environments.get(&env_id)?;

            match env.governance.learning_mode {
                LearningMode::Explore => {
                    // Full trajectory learning, update all weights
                    self.sona.learn_trajectories(&env_trajectories).await?;
                }
                LearningMode::Validate => {
                    // Only validate existing patterns, don't create new ones
                    self.sona.validate_patterns(&env_trajectories).await?;
                }
                LearningMode::Exploit => {
                    // Record outcomes for monitoring, don't update weights
                    self.sona.record_outcomes(&env_trajectories).await?;
                }
            }
        }

        Ok(())
    }

    /// Propose pattern propagation from source to target environment
    pub async fn propose_propagation(
        &self,
        pattern: TrajectoryPattern,
        source: EnvironmentId,
        target: EnvironmentId,
    ) -> Result<LearningPropagation> {
        let target_env = self.environments.get(&target)?;
        let target_policy = target_env.governance.learning_policy();

        // Check if pattern meets target environment's requirements
        let approved = if pattern.confidence >= target_policy.min_confidence
            && pattern.evidence_count >= target_policy.min_evidence
        {
            Some(GateDecision::Permit)
        } else {
            Some(GateDecision::Deny)
        };

        Ok(LearningPropagation {
            source,
            target,
            pattern,
            confidence: pattern.confidence,
            evidence_count: pattern.evidence_count,
            evidence_refs: vec![], // filled from DAG query
            approved,
        })
    }
}
```

### 6.4 CLI Commands

```
# Environment management
weave env list                              -- list all environments
weave env show <env-id>                     -- show environment details + governance
weave env create <name> --class <class>     -- create new environment
weave env assign-node <env-id> <node-id>    -- assign node to environment

# Cross-environment workflows
weft promote <artifact> --from <env> --to <env>    -- promote artifact
weft promote status <promotion-id>                  -- check promotion status
weft promote approve <promotion-id>                 -- approve promotion (human)

# Learning loop
weft learn status                          -- SONA learning statistics
weft learn patterns --env <env-id>         -- list learned patterns
weft learn propagate <pattern-id> --to <env-id>  -- propose pattern propagation
weft learn audit --env <env-id> --since <date>   -- audit trail query
```

### 6.5 Configuration

```toml
# In weftapp.toml or kernel config

[environments.development]
class = "development"
risk_threshold = 0.9
learning_mode = "explore"
audit_level = "session_summary"
human_approval = false

[environments.staging]
class = "staging"
risk_threshold = 0.6
learning_mode = "validate"
audit_level = "per_action"
human_approval = "on_deploy"

[environments.production]
class = "production"
risk_threshold = 0.3
learning_mode = "exploit"
audit_level = "per_action_with_effects"
human_approval = true
min_confidence = 0.95
min_evidence = 100
min_staging_soak = "7d"
```

---

## 7. New Files

| File | Phase | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/environment.rs` | K1 (extended) | Environment registry + capability scoping |
| `crates/clawft-kernel/src/learning_loop.rs` | K5 (extended) | Self-learning governance loop |
| `crates/clawft-kernel/src/promotion.rs` | K5 (extended) | Cross-environment promotion pipeline |
| `crates/clawft-kernel/src/effect.rs` | K1 (extended) | Effect algebra (5D scoring) |
| `crates/clawft-kernel/src/trajectory.rs` | K5 (extended) | Governance trajectory types |

---

## 8. Testing Strategy

### Unit Tests

| Module | Test | Description |
|--------|------|-------------|
| `environment.rs` | `resolve_capabilities_dev` | Dev environment gives full capabilities |
| `environment.rs` | `resolve_capabilities_prod` | Prod environment restricts capabilities |
| `environment.rs` | `check_action_below_threshold` | Action below risk threshold → Permit |
| `environment.rs` | `check_action_above_threshold` | Action above risk threshold → Defer/Deny |
| `environment.rs` | `unknown_environment_denied` | Unknown environment → Deny |
| `learning_loop.rs` | `record_creates_dag_entry` | Every record creates immutable DAG entry |
| `learning_loop.rs` | `explore_mode_learns` | Explore mode updates SONA weights |
| `learning_loop.rs` | `exploit_mode_records_only` | Exploit mode does not update weights |
| `learning_loop.rs` | `propagation_checks_confidence` | Low confidence patterns rejected |
| `promotion.rs` | `promotion_requires_gates` | All gates must pass for promotion |
| `promotion.rs` | `prod_promotion_requires_human` | Production promotion needs human approval |
| `effect.rs` | `effect_magnitude` | Effect vector magnitude calculation correct |

### Integration Tests

| Test | Description |
|------|-------------|
| `env_scoped_workflow` | Agent performs action in dev (Permit), same action in prod (Defer) |
| `learning_propagation` | Pattern learned in dev, validated in staging, applied in prod |
| `promotion_pipeline` | Artifact promoted dev → staging → prod with gates |
| `audit_trail_immutable` | Verify audit entries cannot be modified after creation |
| `constitutional_invariants` | Verify no agent can bypass immutable rules |

---

## 9. Risks and Mitigations

| ID | Risk | Severity | Mitigation |
|----|------|----------|------------|
| R1 | Learning loop latency impacts production performance | High | Batch learning (100+ trajectories), async flush, exploit-mode is read-only |
| R2 | Pattern propagation introduces regressions in production | High | Min 100 evidence, 7-day staging soak, 0.95 confidence threshold |
| R3 | Environment configuration drift across nodes | Medium | Environment configs stored in exochain DAG, CRDT-synced |
| R4 | Human approval bottleneck in production | Medium | Timeout policy (deny/queue), escalation chains, delegation |
| R5 | SONA weight updates conflict across environments | Medium | Learning is per-environment; propagation is explicit, not automatic |
| R6 | Audit storage grows unbounded | Low | MMR accumulator provides O(log n) proofs; cold storage for old entries |

---

## 10. Cross-References

| Document | Relationship |
|----------|-------------|
| `07-ruvector-deep-integration.md` | SONA engine, delta-consensus CRDTs, cognitum-gate |
| `08-ephemeral-os-architecture.md` | Multi-tenant fabric, agent-as-user, cryptographic filesystem |
| `01-phase-K0-kernel-foundation.md` | Process table, service registry (base for environments) |
| `02-phase-K1-supervisor-rbac.md` | Capability system (extended with environment scoping) |
| `06-phase-K5-app-framework.md` | Application manifests (extended with environment declarations) |
| `.planning/ruv/packages/ai-sdlc/overview.md` | CGR engine, effect algebra, constitutional invariants |
| `.planning/ruv/packages/ruvector/overview.md` | SONA, delta-consensus, cognitum-gate patterns |
| `.planning/ruv/packages/exochain/overview.md` | DAG, DID, HLC, Gatekeeper, bailment model |
