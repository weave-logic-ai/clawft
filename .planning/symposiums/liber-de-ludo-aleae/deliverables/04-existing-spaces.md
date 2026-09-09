# Room Vetus — Cardano already in the walls

**Phase I, Room Vetus only.** This document inventories WeftOS surfaces that already *rhyme* with *Liber de Ludo Aleae*. It does **not** propose the new doctrine (that is Room Nova) and it does **not** merge the two (that is Room Coniunctio).

**Honesty rule:** none of these files cite Cardano. The influence is structural, not bibliographic. We mark each row **rhyme** (same question), **cousin** (same family, different primitive), or **false friend** (shared word, different meaning).

**Live instrument (this session):** MetaHarness score on `weftos` — `harnessFit 75`, `compileConfidence 100`, `taskCoverage 65`, `toolSafety 90`, `memoryUsefulness 53`, `estCostPerRunUsd 0.024`. A five-die throw with a price tag and no named circuit. ADR-096's 2026-07-31 baseline was fit 75 / coverage 65 / memory 51.

**Panel record:** [`panels/P2-vetus.md`](../panels/P2-vetus.md). Workshop: [`workshops/vetus/index.html`](../workshops/vetus/index.html).

**Tabula checklist (inventory only, not a contract):** for each surface — has a circuit? has odds? has edge? has ruin? has calibration? Answered in §6. All live rows are **no**, with two *near-miss* cousins called out.

---

## 1. Inventory

Seed V1–V18 kept. V19–V31 added after reading the files. Class column is binding.

| # | Surface | Path / symbol | Cardano rhyme | Class | What it already does | What it still lacks |
|---|---------|---------------|---------------|-------|----------------------|---------------------|
| V1 | **EffectVector** | ADR-034; `clawft_core::agent::effects::EffectVector`; `clawft_kernel::governance::EffectVector`; `EffectVector::magnitude` | *aequitas* + *periculum* as named dimensions; `fairness` is a first-class axis | rhyme | 5D impact (`risk`, `fairness`, `privacy`, `novelty`, `security`); L2 magnitude; genesis-locked; `effect_for_tool` static table; unknown tool = zero vector (silent Permit); `args` ignored | No circuit. Fairness is a 0–1 vibe ("equitable treatment"), not equality-of-conditions. Unweighted. No uncertainty. Threshold is a house rule (and not even one rule: 0.7 vs 0.8). K2 §5 already flagged this. |
| V2 | **Governance gate** | ADR-033/034; `GovernanceDecision`; two `GateDecision` enums | Refuse an unequal table | rhyme | Four-way constitution: `Permit` / `PermitWithWarning` / `EscalateToHuman` / `Deny`. Magnitude vs `GovernanceEngine.risk_threshold` (tests use `0.7`). Core + kernel `GateDecision` are three-way: `Permit` / `Defer` / `Deny` | Threshold is a house rule without a stated sample space. Seed nicknamed this "Permit / Warn / Escalate / Deny" — that is the governance enum, not the gate enum. Two dice. |
| V3 | **SOUL.md minimax + EV** | `docs/skills/clawft/SOUL.md` § Behavioral Guidelines | *scientia* over *fortuna*; EV as duty | rhyme | Agents are told to maximize expected value and minimize worst-case loss; sample the tree when it is too deep | No arithmetic, no bankroll, no circuit. A commandment, not a scorer |
| V4 | **FitnessScorer / QualityScorer** | `pipeline::scorer::FitnessScorer`; `pipeline::scorer::NoopScorer`; `scoring::QualityScorer`; `scoring::NoopScorer`; `scoring::BasicScorer`; WEFT-54 | Score the cast after it lands | cousin | Pipeline: 4-weight GEPA fitness (`FitnessScorerWeights` 0.4/0.2/0.2/0.2, frozen 0.8.x); −0.2 on first English refusal substring. RVF `BasicScorer`: length + error phrases + `tool_use` bonus. Two `NoopScorer`s: pipeline always **1.0**, RVF always **0.5** | Heuristic = reasoning-on-the-mean's cousin. Length and English substrings are not a circuit. Same type name, two blank-die constants |
| V5 | **NodeScoring 6D** | `exo_resource_tree::scoring::NodeScoring`; `SCORING_DIMS`; `blend`; `to_hash_bytes` | Frequency / reliability over time (*np*) | cousin | trust, performance, difficulty, reward, reliability, velocity; EMA `blend`; Merkle 24-byte leaf; `weighted_score`; Pareto `dominates` | Neutral 0.5 default is a blank die. EMA is not calibration to a declared *p* |
| V6 | **Cost / no-op circuit-breakers** | WEFT-322 `agent::cost_budget::{ConversationBudget, BudgetUsage}`; `PlanningConfig::circuit_breaker_no_op_limit`; `TerminationReason::CircuitBreaker` | Ruin / small-stakes; stop before the purse is gone | cousin / **false friend** on the word *circuit* | Per-conversation token / USD / iteration cap (`circuit_open`, `tripped_dimension`); planning abort after N no-ops (default 3); `max_planning_cost_usd` default $1.0 | *Circuit* here means breaker, not sample space. Stake cap without odds. Two trips, one English word. Routing.md Level-2 "CircuitBreaker" is a **stub** |
| V7 | **MetaHarness score + flywheel** | ADR-096/097; `metaharness_score`; `.metaharness/flywheel/STRING.md` | Count *n* casts; no silent promote; receipts | rhyme | 5-dim readiness; genome; evaluate → receipt → confirm promote; Darwin freezes the model. This session: 75 / 100 / 65 / 90 / 53 @ $0.024 | Score is a snapshot, not an EV. `memoryUsefulness 53` has no interval. Optional; not on the `weft` runtime path |
| V8 | **Router / complexity / savings** | ADR-026; `docs/guides/routing.md`; `pipeline::classifier::KeywordClassifier`; `TaskComplexityAnalyzer` | Pricing a wager; edge vs cheap path | cousin | Live: keyword task-type + `matched_keywords/word_count` ∈ [0.1, 0.9]; `TaskComplexityAnalyzer::analyze` 5 heuristics. Receipts for displaced model exist as MH savings language | **7-factor ruvllm scoring is unimplemented** (`routing.md` §10). No house-edge index on the vendor or the judge |
| V9 | **Assessment service** | ADR-023; `AssessmentService`; `weft assess` | Enumerate the board before you play | cousin | Kernel service walks trees, analyzers (complexity, dependency, security, topology, …), reports, chain-logs | Findings are not odds. No "favorable / total". CLI `coherence_score` is a **false friend** of ECC coherence (V23) |
| V10 | **ECC spectral health** | ADR-062; `agents/weftos/ecc-analyst.md`; `eml_coherence::CoherencePrediction`; `GraphFeatures` | Systematic lean ≠ chance | cousin | λ₂, Fiedler, HNSW, gap analysis; two-tier DEMOCRITUS (`coherence_fast` then Lanczos) | Detects structure, not calibration of a declared *p*. Shares the word *coherence* with three other formulas |
| V11 | **DeFi mesh bond / slash** | `agents/weftos/defi-networker.md`; `TrustLevel`; `SlashCondition`; `PeerEconomics` | Stake, ruin, equal conditions among peers | rhyme | Bond, slash, trust ladder Unknown → Paired → Trusted → Bonded; `uptime_ratio`; promotion criteria | Economic, not epistemic. No circuit on the slashing event |
| V12 | **Trajectory / GEPA** | ADR-017; `pipeline::traits::Trajectory`; `TrajectoryLearner`; `FitnessScorer` | Learn from many casts, not one | cousin | Prompt evolution on fitness; ring buffer; `evolution_ready` | Fitness is not EV; can reward lucky phrasing. Second "trajectory" lives in governance (V29) |
| V13 | **Auto-delegation / remaining work** | `docs/guides/auto-delegation-classifier.md`; `DelegationEngine::complexity_estimate`; `AgentLoop::with_auto_delegation` | Problem of points (continue by what remains) | cousin | Pre-LLM short-circuit; regex rules then 30/20/50 length/qmark/keyword heuristic; `< 0.3` → Local | Remaining-work is a classifier, not a stake split. Documented false positives (`preview` ⊃ `review`) |
| V14 | **ExoChain / NodeScoring hash** | ADR-022; `NodeScoring::to_hash_bytes` | Tamper-evident ledger of the table | cousin | Scores in the Merkle leaf (24 LE bytes; NaN → 0.0) | Integrity ≠ fairness |
| V15 | **K2 industry landscape §5** | `docs/weftos/k2-symposium/04-industry-landscape.md` §5 | The room already named the hole | rhyme | **Explicit gap: no uncertainty quantification** on EffectVector. Lists missing velocity / cascading / adaptive thresholds | Opportunity list (10D, velocity, cascading) still unbuilt. EML `CoherencePrediction.uncertainty` (V25) does not close this gap |
| V16 | **K2 C9 / D20 N-dim EffectVector** | deferred in ADR-034 | Named dimensions = named faces of the die | cousin | 5D frozen pending `governance.root.supersede` | Still deferred. Do not smash a sidecar into genesis |
| V17 | **Governance-counsel agent** | `agents/weftos/governance-counsel.md` | The croupier | rhyme | Designs rules, evaluates vectors, trajectory | Older 5D sketch (`cpu`, `memory`, `network`, `storage`, `trust_delta`) has drifted from ADR-034 — a marked deck inside our own briefing |
| V18 | **Kolbe / conative** | `docs/research/kolbe-conative-integration.md` | Who should play, and when (ch. 2) | cousin | Decision *style* (FF/FT/QS/IM), not probability. Doc forbids calling the proxy an official Kolbe score | Must not be confused with odds |
| V19 | **GovernanceScorerModel** | `clawft_kernel::eml_kernel::GovernanceScorerModel`; `EffectVector::score` | A learned croupier replacing the house L2 | cousin | 5-in / 1-out EML; `predict` falls back to L2 when untrained; `record` takes an expert composite | Learned scalar is not a circuit. Untrained path is bit-identical to `magnitude()`. Still unweighted faces |
| V20 | **GateEffectKind / for_gate** | `governance::GateEffectKind`; `EffectVector::for_gate` (WEFT-506) | Named faces per privileged family | cousin | Auditable constants for auth / config / a2a / cron — not inferred from free-form context | Static table, same 5D, still no sample space. Sibling of V1, not a new algebra |
| V21 | **TaskComplexityAnalyzer** | `clawft_core::complexity::TaskComplexityAnalyzer::analyze` (`rvf`) | How hard is this throw? | cousin | 5 surface heuristics: length 0.25, sentences 0.15, tech keywords 0.25, multi-step 0.20, code fence 0.15 | Keyword density is not difficulty of a circuit. English-only tech list |
| V22 | **KeywordClassifier** | `pipeline::classifier::KeywordClassifier`; `docs/guides/routing.md` Stage 1 | Sort the game before you price it | cousin | Priority substring groups → `CodeGeneration` … `Chat`; complexity = hits/words clamped [0.1, 0.9] | First-match grouping is not a type of game in Cardano's sense. Level-1 7-factor remains a TODO |
| V23 | **Assessment `coherence_score`** | `clawft-cli` `assess_cmd.rs` (~L1026–1029) | — | **false friend** | `doc_files / (rust_files + typescript_files) * 100`, capped 100 | Same English word as V10/V24/V27. It is a documentation ratio, not spectral health, not readability |
| V24 | **Forensic `coherence_score`** | `clawft_graphify::domain::forensic::coherence_score` | Lean of a knowledge graph | cousin / **false friend** of V10 | `density * avg_edge_confidence` on a directed KG; empty=0, single node=1 | Third formula named *coherence*. Optional EML swap (`coherence_score_eml`) is still not a declared *p* |
| V25 | **EML `CoherencePrediction.uncertainty`** | `eml_coherence::CoherencePrediction` | Name the width of the guess | cousin | Third head: "lambda_2 confidence interval width". Untrained fallback `lambda_2 * 0.5` | **Does not close K2 §5.** It is a model-head width, not calibration of a declared circuit. Depth-3 legacy models do not even emit it honestly |
| V26 | **QualityScore vs Fitness weights** | `pipeline::traits::QualityScore` {overall, relevance, coherence} vs `FitnessScorerWeights` 4-dim | Publish the faces you actually score | cousin | Fitness computes four internals then writes three fields (`relevance` ← task_completion; efficiency + tool_accuracy vanish from the struct) | Dimensional mismatch inside one pipeline. A fourth face is thrown and then pocketed |
| V27 | **Fitness `score_coherence`** | `FitnessScorer::score_coherence` | Is the write-up tidy? | **false friend** of V10 | Wall-of-text penalty; markdown/structure bonus; sentence-repetition penalty | Readability heuristic. Not λ₂, not doc/code, not forensic density |
| V28 | **Two `GateDecision` types** | `clawft_core::agent::gate::GateDecision`; `clawft_kernel::gate::GateDecision` | Same call at two tables | **false friend** (internal) | Both: Permit / Defer / Deny. Core carries `token: String`; kernel carries `token: Option<Vec<u8>>` and Deny `receipt` | Same name, different wire. Plus `GovernanceDecision` (V2) is four-way. Three "the gate"s |
| V29 | **Two Trajectory types** | `pipeline::traits::Trajectory`; `governance::TrajectoryRecord` / `TrajectoryRecorder` | Many casts, not one | cousin | Pipeline: request + routing + response + `QualityScore`. Governance: action + outcome FIFO with eviction | Same English word. Neither is EV over a circuit |
| V30 | **Binding-thread EffectVector** | `effect_for_binding_thread`; WEFT-342 | A loaded die for a named cheat | rhyme | Mismatch → `{security: 1.0, risk: 0.9}`; ok → zeros. Dedicated `BindingThread` rule path can fire independent of magnitude | Still the 5D vibe-die. Magnitude bar in tests is `> 0.8`, not ADR-034's `0.7` |
| V31 | **ruvllm 7-factor (planned)** | `docs/guides/routing.md` §6 Level 1 and §10 Adaptive Classifier | A richer count of the throw | cousin (unshipped) | Spec only: word count, vocabulary diversity, syntactic depth, domain specificity, ambiguity, code density, reasoning-chain length | **Not in tree as a scorer.** Inventorying it as live would be a marked card. Do not cite as a WeftOS score until it compiles |

---

## 2. The word "circuit" is already taken

WeftOS uses **circuit-breaker** (WEFT-322 `BudgetUsage.circuit_open`, planning `circuit_breaker_no_op_limit`, routing.md Level-2 stub) for *stop-loss*. Cardano uses **circuitus** for *sample space*.

Room Vetus verdict: keep both words, never collapse them.

| Word | Meaning in-tree / in-book | Do not |
|------|---------------------------|--------|
| **circuit-breaker** | Abort when spend, iterations, or no-ops exceed a cap | Do not rename to "circuitus" |
| **circuitus / circuit** | Enumerated outcomes of a score | Do not call the cost cap a circuitus |
| **stake** | Tokens, time, treasury, bond | Fine in both rooms |
| **fairness** | EffectVector dim today; equality-of-conditions in the book | Do not pretend they are already the same |

Shipped trips (two, not one):

- `ConversationBudget::check_can_call` → `ClawftError::ConversationBudgetExceeded` until `reset`.
- `PlanningConfig` guard rail → `TerminationReason::CircuitBreaker` when `consecutive_no_ops >= circuit_breaker_no_op_limit` (default 3). Sibling rail `TerminationReason::BudgetExceeded` is the planning USD cap, not the no-op breaker.

Unshipped: routing Level-2 "CircuitBreaker" (provider demotion). Do not inventory as live.

---

## 3. Three already-honest scores

These are the closest things WeftOS has to a *cast you can recount*:

1. **MetaHarness 5-die** — harnessFit / compileConfidence / taskCoverage / toolSafety / memoryUsefulness. This session: `75 / 100 / 65 / 90 / 53`, `estCostPerRunUsd 0.024`. No *n*, no interval, no "what would count as favorable." ADR-096 baseline (2026-07-31) was 75 / 65 / 51 on the three overlapping faces.
2. **EffectVector L2** — `sqrt(r²+f²+p²+n²+s²)`, max `√5 ≈ 2.236`. ADR-034 default gate `0.7`. `agent_spawn` D6 and binding-thread tests assert `> 0.8`. Fast. Silent about how the five faces were assigned. `GovernanceScorerModel` untrained path is the same formula.
3. **FitnessScorer** — completion minus 0.2 if an English refusal substring hits; weights frozen 0.8.x. Documented as *not* a safety control (WEFT-54). That honesty is already Cardano-shaped: name what the die cannot do.

---

## 4. Drift to flag (Vetus only)

### 4.1 Marked deck: governance-counsel 5D

`agents/weftos/governance-counsel.md` still shows an older EffectVector (`cpu`, `memory`, `network`, `storage`, `trust_delta`) and a CLI example `--vector '{"cpu":0.8,"memory":0.6,"network":0.3}'`. ADR-034 and `effects.rs` / kernel `governance.rs` use `risk`, `fairness`, `privacy`, `novelty`, `security`. Two dice on the same table. Coniunctio should treat this as a **marked-deck** example: unequal conditions inside our own agent briefings.

### 4.2 Two bars for the same L2

| Source | Bar |
|--------|-----|
| ADR-034; `GovernanceEngine::new(0.7, …)` tests | `0.7` |
| `effect_for_tool("agent_spawn")` D6 comment + test | `> 0.8` |
| `effect_for_binding_thread(false)` test | `> 0.8` |
| kernel binding-thread path (`governance.rs` ~L808) | magnitude `> 0.8` |

### 4.3 Same name, two (or four) formulas

| Name | Formula |
|------|---------|
| pipeline `NoopScorer::score` | always 1.0 |
| RVF `NoopScorer::score` | always 0.5 |
| Fitness `score_coherence` | readability heuristic |
| ECC `CoherencePrediction.lambda_2` | algebraic connectivity (exact or EML) |
| assess CLI `coherence_score` | doc/code × 100 |
| forensic `coherence_score` | density × avg confidence |

### 4.4 Seed corrections (Vetus, after reading)

- V2 is `GovernanceDecision` (4-way) plus two `GateDecision`s (3-way). Not a single "Permit / Warn / Escalate / Deny" type.
- V6 is two shipped breakers plus one stub.
- V8 "ruvllm 7-factor" is **not live** (now V31).
- V10 does not own *coherence*.

---

## 5. What Vetus will hand Coniunctio

- This table (V1–V31), classed.
- The circuit / circuit-breaker homonym (including the unshipped Level-2 stub).
- The four-way *coherence* homonym and the two-Noop / two-gate / two-trajectory twins.
- The three live scores, with this session's MH cast.
- The governance-counsel drift and the 0.7 / 0.8 threshold split as worked examples of unequal conditions.
- A request: **do not smash a new 5D into genesis**. Sidecar the Cardano contract first. C9 stays deferred until a sidecar proves lift.
- Explicit non-delivery: no combining contract, no LDA-ADR text, no Cardano arithmetic.

WeftOS agents present: `governance-counsel` (vectors + gates), `ecc-analyst` (coherence as lean-detector and as a homonym), `defi-networker` (stake/slash), scoring architect (this inventory). Seed recorder: `doc-weaver`.

**Room Nova is not allowed to edit this file.**

---

## 6. Tabula checklist (facts only)

Has this surface already got Cardano's load-bearing pieces? **No row invents them.** Near-miss cousins are marked, not promoted.

| # | circuit? | odds? | edge? | ruin? | calibration? |
|---|----------|-------|-------|-------|--------------|
| V1 EffectVector | no | no (`risk` is a vibe-p) | no | no | no |
| V2 Governance gate | no | no | no | no | no |
| V3 SOUL minimax | no | commandment only | no | "worst-case" named, not computed | no |
| V4 Fitness / Quality | no | no | no | no | history exists (`BasicScorer.history`) without a declared *p* |
| V5 NodeScoring | no | no | no | no | EMA `blend` is a cousin of frequency, not calibration |
| V6 Breakers | no (the word is taken) | no | no | **cousin** — stop-loss without P(bust) | no |
| V7 MetaHarness | no | no | cost tag ≠ edge | no | receipts ≈ *n* casts, no interval |
| V8 Router live | no | no | savings language, no house-edge index | no | no |
| V9 Assessment | enumerates files, not outcomes | no | no | no | no |
| V10 ECC spectral | no | no | no | no | lean ≠ calibration |
| V11 DeFi bond/slash | no | no | no | **cousin** — economic ruin, no P(bust) | no |
| V12 GEPA / Trajectory | no | no | no | no | many casts, fitness ≠ EV |
| V13 Auto-delegation | no | no | no | no | documented FP corpus, not a *p* |
| V14 ExoChain hash | no | no | no | no | integrity ≠ calibration |
| V15 K2 §5 | names the hole | — | — | — | — |
| V16 C9 N-dim | deferred | — | — | — | — |
| V17 Counsel briefing | drifted | — | — | — | — |
| V18 Kolbe | no | no | no | strain ≠ ruin | no |
| V19 GovernanceScorerModel | no | no | no | no | training *n*, not circuit *n* |
| V20 GateEffectKind | no | no | no | no | no |
| V21 TaskComplexityAnalyzer | no | no | no | no | no |
| V22 KeywordClassifier | no | no | no | no | no |
| V23 Assess coherence % | no | no | no | no | no |
| V24 Forensic coherence | no | no | no | no | no |
| V25 EML uncertainty | **cousin** — a width, not a circuit | no | no | no | **cousin** — interval-shaped, uncalibrated |
| V26 QualityScore vs weights | no | no | no | no | no |
| V27 Fitness coherence | no | no | no | no | no |
| V28 Two GateDecisions | no | no | no | no | no |
| V29 Two Trajectories | no | no | no | no | FIFO ≠ calibration |
| V30 Binding-thread EV | no | no | no | no | no |
| V31 7-factor (stub) | unshipped | — | — | — | — |

---

## 7. Code citations (load-bearing)

```20:40:crates/clawft-core/src/agent/effects.rs
pub struct EffectVector {
    /// Probability of negative outcome (0.0 → 1.0).
    #[serde(default)]
    pub risk: f64,
    /// Impact on equitable treatment (0.0 → 1.0).
    #[serde(default)]
    pub fairness: f64,
    /// Impact on data privacy (0.0 → 1.0).
    #[serde(default)]
    pub privacy: f64,
    /// How unprecedented the action is (0.0 → 1.0).
    #[serde(default)]
    pub novelty: f64,
    /// Impact on system security (0.0 → 1.0).
    #[serde(default)]
    pub security: f64,
}
```

```46:53:crates/clawft-core/src/agent/effects.rs
    pub fn magnitude(&self) -> f64 {
        (self.risk * self.risk
            + self.fairness * self.fairness
            + self.privacy * self.privacy
            + self.novelty * self.novelty
            + self.security * self.security)
            .sqrt()
    }
```

```589:598:crates/clawft-kernel/src/governance.rs
pub enum GovernanceDecision {
    Permit,
    PermitWithWarning(String),
    EscalateToHuman(String),
    Deny(String),
}
```

```141:163:crates/clawft-core/src/agent/gate.rs
pub enum GateDecision {
    Permit { token: String },
    Defer { reason: String },
    Deny { reason: String },
}
```

```19:32:crates/exo-resource-tree/src/scoring.rs
pub struct NodeScoring {
    pub trust: f32,
    pub performance: f32,
    pub difficulty: f32,
    pub reward: f32,
    pub reliability: f32,
    pub velocity: f32,
}
```

```56:78:crates/clawft-core/src/agent/cost_budget.rs
pub struct BudgetUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub usd: f64,
    pub iterations: u32,
    pub circuit_open: bool,
    pub tripped_dimension: Option<String>,
}
```

```246:253:crates/clawft-core/src/planning.rs
        if consecutive_no_ops >= self.config.circuit_breaker_no_op_limit {
            // ...
            return Some(TerminationReason::CircuitBreaker);
        }
```

```40:47:crates/clawft-kernel/src/eml_coherence.rs
pub struct CoherencePrediction {
    pub lambda_2: f64,
    pub fiedler_norm: f64,
    pub uncertainty: f64,
}
```

```63:69:agents/weftos/governance-counsel.md
pub struct EffectVector {
    pub cpu: f32,
    pub memory: f32,
    pub network: f32,
    pub storage: f32,
    pub trust_delta: f32,
}
```
