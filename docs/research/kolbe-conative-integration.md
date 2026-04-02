# Kolbe Conative Integration -- Research and Design

**Date**: 2026-04-02
**Status**: Research Complete / Design Proposal
**Scope**: Agent pipeline adaptation + org graph enrichment

---

## 1. Kolbe Framework Summary

### 1.1 What Conation Is

The mind has three parts: cognitive (thinking/IQ), affective (feeling/personality), and conative (doing/instinct). Kolbe measures the third part -- how a person instinctively takes action when free to be themselves. This is distinct from personality assessments (Myers-Briggs, DISC, Big Five) which measure affective traits, and from cognitive assessments which measure learned abilities.

The core insight: conation is stable over a lifetime. A person's instinctive MO (modus operandi) does not change with training, mood, or context. It can only be suppressed, which causes "strain" -- reduced productivity, burnout, and disengagement.

### 1.2 The Four Action Modes

Each person receives a score of 1-10 in each mode. The total across all four modes is always constrained (typically sums to approximately 20), meaning strength in one mode comes at the expense of another.

**Fact Finder (FF)** -- How you gather and share information.

| Zone | Score | Instinctive Strategy |
|------|-------|---------------------|
| Initiating | 7-10 | Researches exhaustively before acting. Needs specifics, sources, historical context. Writes detailed documents. Asks "why" and "what evidence supports this?" |
| Accommodating | 4-6 | Gathers enough information to be competent. Balances depth with pragmatism. |
| Preventing | 1-3 | Filters to essentials. Resists over-analysis. Simplifies. Wants the bottom line, not the backstory. |

**Follow Thru (FT)** -- How you organize and design systems.

| Zone | Score | Instinctive Strategy |
|------|-------|---------------------|
| Initiating | 7-10 | Creates systems, processes, checklists. Needs sequential order. Plans before executing. Maintains structure. |
| Accommodating | 4-6 | Adapts to existing systems. Can follow or deviate as needed. |
| Preventing | 1-3 | Resists rigid structure. Multitasks. Adapts fluidly. Prefers flexibility over process. |

**Quick Start (QS)** -- How you deal with risk and uncertainty.

| Zone | Score | Instinctive Strategy |
|------|-------|---------------------|
| Initiating | 7-10 | Experiments, prototypes, pivots quickly. Thrives on novelty and deadlines. Comfortable with ambiguity. Generates many ideas. |
| Accommodating | 4-6 | Takes calculated risks. Balances innovation with stability. |
| Preventing | 1-3 | Stabilizes. Resists unnecessary change. Needs proof before pivoting. Anchors the team against churn. |

**Implementor (IM)** -- How you handle physical space and tangible solutions.

| Zone | Score | Instinctive Strategy |
|------|-------|---------------------|
| Initiating | 7-10 | Builds physical prototypes. Needs hands-on interaction. Thinks in spatial/mechanical terms. |
| Accommodating | 4-6 | Can work with physical or abstract equally. |
| Preventing | 1-3 | Works abstractly. Models mentally. Prefers diagrams over physical prototypes. Imagines rather than builds. |

### 1.3 Zones of Operation

- **Initiating (7-10)**: The person will proactively generate action in this mode. They cannot help themselves.
- **Accommodating (4-6)**: The person can operate in this mode when needed but does not instinctively lead with it.
- **Counteracting (1-3)**: The person instinctively prevents or resists action in this mode. This is a strength, not a weakness -- they provide a natural check.

### 1.4 Strain

Strain occurs when a person is forced to operate in a zone opposite to their natural MO. A high Quick Start (8) forced into rigid Follow Thru processes (without freedom to deviate) will experience cognitive fatigue, reduced output, and eventually burnout. Kolbe measures this via the Kolbe B Index (which captures job expectations) compared to the Kolbe A (natural MO). When A and B diverge by 4+ points in any mode, strain is present.

For our purposes, strain detection is valuable: if the AI agent notices a user struggling with tasks that contradict their inferred MO, it can suggest delegation, tool changes, or process modifications.

### 1.5 Team Dynamics

- **Synergy**: Occurs when team members have complementary MOs covering all zones across all four modes. The natural population distribution is roughly 20% initiating, 60% accommodating, 20% counteracting in each mode.
- **Conflict**: A difference of 4+ points in any mode between two people creates potential friction (e.g., FF=9 paired with FF=2). This becomes productive conflict when both have freedom to operate in their own zone; destructive when one imposes their style on the other.
- **Depletion**: When >15% of a team has strain in any single mode, organizational problems emerge. At 25% it is serious. At 45% it is catastrophic. This maps directly to the assessment product's risk heatmap.

---

## 2. Signal Taxonomy

### 2.1 Methodology

An LLM cannot administer the actual Kolbe A Index (it is proprietary, 36 questions, and validated psychometrically). What we can do is infer a _behavioral proxy_ from observable interaction data. This proxy is explicitly labeled as "inferred conative tendency" -- never as an official Kolbe score. The distinction matters legally and ethically (see Section 8).

Academic research confirms that personality traits can be inferred from text with reasonable accuracy using NLP. A 2025 study from Auburn University found that AI chatbots can infer personality traits as well as or better than traditional self-report measures. The Big Five model has extensive NLP validation; Kolbe's four modes are less studied but map to observable behavioral signals that are at least as concrete as Big Five dimensions.

### 2.2 Fact Finder Signals

| Data Source | High FF (7-10) Signals | Low FF (1-3) Signals |
|-------------|----------------------|---------------------|
| **Chat/Slack** | Long messages with citations, links, qualifications ("according to...", "the data shows..."). Asks clarifying questions before acting. Corrects inaccuracies in others' messages. | Short, action-oriented messages. "Just do X." Skips preamble. Rarely includes sources. |
| **PR Comments** | Detailed line-by-line reviews. References documentation, standards, prior art. Requests additional context before approving. Review comments are longer than the diff. | Approves quickly with brief comments ("LGTM", "ship it"). Focuses on whether it works, not why it was built that way. |
| **Meeting Transcripts** | Asks for data before decisions. "What's the evidence for that?" Presents research findings. Speaks in longer turns with caveats. | Pushes for decisions without extended discussion. "We can figure it out as we go." Short turns. |
| **Document Creation** | Long, structured documents with appendices, references, version history. Specification-first. | Short documents, bullet points, diagrams over prose. README over spec. |
| **Task Management** | Creates detailed subtasks with acceptance criteria. Estimates include confidence intervals. | Creates few, broad tasks. "Build the thing" with minimal description. |
| **Agent Interaction** | Asks the agent for more detail, sources, alternatives. Pushes back on unsupported claims. | Asks the agent to be concise. Interrupts long responses. Prefers actionable output over explanation. |

**LLM-extractable features**: message length distribution, question frequency, citation/link density, qualifier word frequency ("however", "although", "specifically"), review comment length vs diff size ratio, time-to-first-response (high FF takes longer).

### 2.3 Follow Thru Signals

| Data Source | High FT (7-10) Signals | Low FT (1-3) Signals |
|-------------|----------------------|---------------------|
| **Chat/Slack** | Numbered lists, sequential instructions. Creates threads vs flat replies. References previous decisions. Maintains running docs. | Stream-of-consciousness replies. Topic jumps. Multiple parallel conversations. |
| **PR Comments** | Checks for consistency with existing patterns. "This doesn't match how we do X elsewhere." Requests documentation updates alongside code. | Reviews the change in isolation. Does not cross-reference with existing patterns. |
| **Meeting Transcripts** | Creates agendas beforehand. Takes notes. Follows up with action items. References previous meeting decisions. | Ad hoc discussion flow. Comfortable with tangents. Rarely references prior meetings. |
| **Document Creation** | Highly structured: numbered sections, tables of contents, cross-references between documents. Templates. | Freeform notes, mind maps, whiteboard photos. Creates new documents rather than updating existing ones. |
| **Task Management** | Maintains backlog hygiene. Moves tickets through defined stages. Closes completed items. Links dependencies. | Creates tasks but rarely updates status. Uses tasks as reminders, not as a system. |
| **Agent Interaction** | Asks the agent to follow a defined process. "First do X, then Y, then Z." Expects consistent output format across sessions. | Gives open-ended instructions. Comfortable with varying output formats. |

**LLM-extractable features**: list/numbering frequency, thread vs flat reply ratio, cross-reference density, document structure depth (heading levels), task status update frequency, instruction sequentiality score.

### 2.4 Quick Start Signals

| Data Source | High QS (7-10) Signals | Low QS (1-3) Signals |
|-------------|----------------------|---------------------|
| **Chat/Slack** | Frequent topic changes. "What if we..." and "Let's try..." language. Responds quickly. Shares half-formed ideas. Uses exclamation marks. | Cautious language. "We should think about this." Waits before responding. Questions feasibility of new ideas. |
| **PR Comments** | Opens many PRs (some experimental/draft). Comfortable merging quickly. Reviews focus on "does this unlock something new?" | Fewer, more polished PRs. Questions the necessity of changes. "Why not keep the current approach?" |
| **Meeting Transcripts** | Generates multiple ideas per discussion. Pivots frequently. Gets energized by new proposals. "Let's just ship it and see." | Slows down rapid-fire ideation. "We haven't finished the last thing yet." Advocates for completing current work. |
| **Document Creation** | Creates many short documents. Starts new docs rather than finishing old ones. Prototype-first, document-later. | Finishes documents before starting new ones. Prefers proven approaches documented before implementation. |
| **Task Management** | Creates many tasks, starts many, finishes fewer. High task creation velocity. | Creates tasks only when committed to completing them. Completion rate is high. |
| **Agent Interaction** | Asks the agent to brainstorm, prototype, generate alternatives. "Give me 5 different ways to do this." Changes direction mid-conversation. | Asks the agent to validate before proceeding. "What are the risks of this approach?" Stays on one topic per session. |

**LLM-extractable features**: ideation language frequency ("what if", "let's try", "imagine"), response latency (lower = higher QS), topic entropy per conversation, PR open/merge velocity, draft PR ratio, task creation-to-completion ratio, direction-change frequency within a conversation.

### 2.5 Implementor Signals

| Data Source | High IM (7-10) Signals | Low IM (1-3) Signals |
|-------------|----------------------|---------------------|
| **Chat/Slack** | References physical artifacts, hardware, spatial layouts. "I'll build a prototype." Shares photos of whiteboards, hardware setups. | References abstract models, diagrams, mental models. "Conceptually, this is like..." |
| **PR Comments** | Focuses on concrete implementation details: performance, memory, byte layouts. "This allocates too much on the heap." | Focuses on architecture, abstraction, API design. "The interface should be..." |
| **Meeting Transcripts** | Asks about physical constraints: hardware, network topology, deployment environment. Draws diagrams on whiteboards. | Discusses in abstract terms. Comfortable with "we'll figure out deployment later." |
| **Document Creation** | Includes diagrams, architecture drawings, data flow charts. Specifications include physical constraints. | Pure prose or pseudocode. Abstract models without physical grounding. |
| **Task Management** | Tasks include environment setup, hardware provisioning, tooling configuration. | Tasks are abstract: "design the API", "define the schema." |
| **Agent Interaction** | Asks the agent about concrete implementation: "Show me the code." "What does the memory layout look like?" | Asks the agent about design patterns, trade-offs, conceptual models. |

**LLM-extractable features**: concrete noun density (hardware terms, physical objects), code vs prose ratio in messages, diagram/image sharing frequency, implementation-detail focus in reviews, spatial/mechanical language frequency.

### 2.6 Composite Scoring Algorithm

```
For each dimension D in {FF, FT, QS, IM}:
  1. Collect all signals S_i for dimension D from all data sources
  2. For each signal S_i, compute a raw score r_i in [-1.0, 1.0]
     where -1.0 = strong counteracting signal, +1.0 = strong initiating signal
  3. Weight each signal by source reliability:
     w_code_review = 0.8  (behavioral, hard to fake)
     w_chat = 0.6         (natural but noisy)
     w_meeting = 0.7      (behavioral but context-dependent)
     w_document = 0.5     (can be templated)
     w_task = 0.4         (often reflects process, not instinct)
     w_agent = 0.9        (most natural interaction, least social pressure)
  4. Compute weighted average: score_D = sum(r_i * w_i) / sum(w_i)
  5. Map to 1-10 scale: kolbe_D = round(score_D * 4.5 + 5.5)
     (maps [-1, 1] to [1, 10])
  6. Apply confidence: conf_D = min(1.0, n_signals / required_minimum)
     where required_minimum = 50 signals for high confidence

Constraint: scores should approximately sum to 20 (Kolbe constraint).
Apply soft normalization: if sum > 22 or sum < 18, redistribute
proportionally toward 20 while preserving relative ordering.
```

---

## 3. ECC Integration Design

### 3.1 New Node Types

The existing `CausalNode` in `clawft-kernel/src/causal.rs` stores a label and metadata. Conative profiles become nodes in the causal graph:

```rust
// New node metadata variants (stored in CausalNode.metadata)

/// A person's inferred conative profile at a point in time.
struct ConativeProfileMeta {
    /// The person this profile belongs to (maps to OrgNode.agent_id).
    person_id: String,
    /// Inferred scores per dimension.
    fact_finder: f32,    // 1.0-10.0
    follow_thru: f32,
    quick_start: f32,
    implementor: f32,
    /// Confidence per dimension (0.0-1.0).
    ff_confidence: f32,
    ft_confidence: f32,
    qs_confidence: f32,
    im_confidence: f32,
    /// Number of signals contributing to this profile.
    signal_count: u64,
    /// Whether the profile is self-reported, inferred, or verified.
    source: ConativeSource,
}

enum ConativeSource {
    /// Inferred from interaction data (behavioral proxy).
    Inferred,
    /// Self-reported by the person.
    SelfReported,
    /// Official Kolbe A Index result (imported with consent).
    Verified,
}
```

### 3.2 New Edge Types

Extend `CausalEdgeType` in `clawft-kernel/src/causal.rs`:

```rust
// Additions to CausalEdgeType enum:

/// Person's conative profile influences their interaction pattern.
ConativeInfluences,
/// Two people have conative synergy (complementary MOs).
ConativeSynergy,
/// Two people have conative friction (4+ point gap in a mode).
ConativeFriction,
/// A behavioral signal provides evidence for a conative score.
ConativeEvidence,
/// A person experiences strain (inferred MO vs role expectations diverge).
ConativeStrain,
```

### 3.3 Graph Topology

```
[PersonNode:jane] --ConativeInfluences--> [ConativeProfile:jane-2026Q1]
    |                                         |
    |--knows_about--> [SystemNode:billing]     |--ConativeEvidence--> [Signal:pr-review-detail-level]
    |                                         |--ConativeEvidence--> [Signal:chat-message-length]
    |                                         |--ConativeEvidence--> [Signal:agent-interaction-style]
    |
    |--ConativeSynergy {weight: 0.85}--> [PersonNode:marcus]
    |--ConativeFriction {weight: 0.72, mode: "fact_finder"}--> [PersonNode:alex]
    |--ConativeStrain {weight: 0.6, mode: "follow_thru"}--> [RoleNode:project-manager]
```

### 3.4 CrossRef Integration

The existing `CrossRefStore` in `clawft-kernel/src/crossref.rs` supports `StructureTag` variants. Add:

```rust
// New StructureTag variant:
ConativeGraph = 0x05,
```

This allows conative profile nodes to be cross-referenced with HNSW vectors (for similarity search of similar conative profiles), ExoChain events (for provenance of when a profile was inferred), and Resource Tree nodes (for connecting profiles to organizational positions).

### 3.5 DEMOCRITUS Loop Integration

The `DemocritusLoop` in `clawft-kernel/src/democritus.rs` runs Sense-Embed-Search-Update-Commit. Conative signals feed into this loop naturally:

- **Sense**: New behavioral signals arrive as `Impulse` events (chat message sent, PR review submitted, meeting transcript processed).
- **Embed**: The signal text is embedded via the `EmbeddingProvider`. The embedding captures communication style features.
- **Search**: HNSW search finds similar past signals. Clustering of signals from the same person reveals patterns.
- **Update**: The `CausalGraph` is updated with `ConativeEvidence` edges linking the signal to the person's conative profile node. Scores are recalculated using EMA (exponential moving average) to weight recent signals more heavily.
- **Commit**: The updated profile is logged to ExoChain, creating an auditable history of how the inference evolved.

The DEMOCRITUS config parameter `correlation_threshold` (default 0.7) controls when two signals are considered evidence for the same conative dimension.

### 3.6 GEPA Fitness Scoring for Adaptation Quality

The existing `FitnessScorer` in `clawft-core/src/pipeline/scorer.rs` evaluates response quality on task completion, efficiency, tool accuracy, and coherence. Extend with a conative adaptation dimension:

```rust
// Addition to FitnessScorerWeights:
/// Weight for conative alignment signal (0.0--1.0).
pub conative_alignment: f32,
```

The conative alignment score measures: "Did the response match the user's inferred conative style?" Indicators:
- Response length vs user's Fact Finder score (high FF should get longer responses)
- Structure level vs user's Follow Thru score (high FT should get structured output)
- Optionality vs user's Quick Start score (high QS should get multiple alternatives)
- Concreteness vs user's Implementor score (high IM should get concrete examples/code)

This creates a feedback loop: trajectories with high conative alignment scores teach the learner what adaptation strategies work.

### 3.7 Governance Transparency

The `GovernanceGate` evaluates a 5D `EffectVector`. Conative adaptation decisions should be gated and transparent:

```
// Example governance log entry:
{
  "action": "conative_adaptation",
  "effect_vector": { "risk": 0.1, "performance": 0.7, "difficulty": 0.2, "reward": 0.8, "reliability": 0.6 },
  "adaptation": {
    "user": "jane",
    "inferred_mo": { "ff": 8.2, "ft": 3.1, "qs": 6.5, "im": 2.2 },
    "confidence": 0.74,
    "adaptation_applied": "extended_detail_mode",
    "explanation": "Adapting response depth because Fact Finder tendency is high (8.2, confidence 0.74). Source: 127 chat signals, 34 PR review signals."
  },
  "transparency_note": "This adaptation can be viewed and overridden via /preferences conative"
}
```

---

## 4. Agent Pipeline Integration Design

### 4.1 The 6-Stage Pipeline

The current pipeline in `clawft-core/src/pipeline/` has six stages:

1. **TaskClassifier** -- Classify request type and complexity
2. **ModelRouter** -- Select provider/model
3. **ContextAssembler** -- Assemble system prompt, memory, history
4. **LlmTransport** -- Execute LLM call
5. **QualityScorer** -- Score response quality
6. **LearningBackend** -- Record trajectory for learning

Conative adaptation touches stages 2, 3, 5, and 6. It does NOT add a new stage; it enriches existing stages.

### 4.2 Stage 2: ModelRouter Enhancement

The `TieredRouter` already routes based on task complexity. Conative profile adds a secondary signal:

- High Fact Finder users asking research questions should route to higher-tier models (Sonnet/Opus) because they expect depth that Haiku cannot provide.
- Low Fact Finder users asking simple questions can stay on Haiku even for slightly complex tasks because they want brevity.
- High Quick Start users get faster responses (prefer lower-latency models) over more thorough ones.

Implementation: the `TaskProfile.complexity` score gets a conative adjustment factor before routing.

### 4.3 Stage 3: ContextAssembler Enhancement (Primary Integration Point)

This is where the main adaptation happens. The `TokenBudgetAssembler` currently preserves the system prompt and recent messages. The conative-aware assembler injects a conative adaptation directive into the system prompt.

```rust
/// Conative adaptation directive injected into the system prompt.
fn build_conative_directive(profile: &ConativeProfile) -> String {
    let mut directives = Vec::new();

    // Fact Finder adaptation
    match profile.ff_zone() {
        Zone::Initiating => directives.push(
            "The user values thorough, well-sourced responses. Include evidence, \
             references, and caveats. Explain your reasoning. Longer responses \
             with depth are preferred over brevity."
        ),
        Zone::Counteracting => directives.push(
            "The user values concise, action-oriented responses. Lead with the \
             answer. Skip background unless asked. Bullet points over paragraphs."
        ),
        Zone::Accommodating => {} // No strong directive needed
    }

    // Follow Thru adaptation
    match profile.ft_zone() {
        Zone::Initiating => directives.push(
            "The user values structured, sequential output. Use numbered lists, \
             clear headings, and consistent formatting. Reference prior context. \
             Maintain continuity with previous responses."
        ),
        Zone::Counteracting => directives.push(
            "The user values flexible, adaptive responses. Vary your format. \
             Don't over-structure. Respond to the energy of the conversation \
             rather than imposing rigid formats."
        ),
        Zone::Accommodating => {}
    }

    // Quick Start adaptation
    match profile.qs_zone() {
        Zone::Initiating => directives.push(
            "The user values speed, options, and experimentation. Offer multiple \
             approaches. Prototype before perfecting. Say 'let's try' more than \
             'we should consider'. Keep momentum high."
        ),
        Zone::Counteracting => directives.push(
            "The user values stability and proven approaches. Don't suggest \
             changes for the sake of novelty. Validate before recommending. \
             One well-vetted recommendation over five speculative ones."
        ),
        Zone::Accommodating => {}
    }

    // Implementor adaptation
    match profile.im_zone() {
        Zone::Initiating => directives.push(
            "The user values concrete, tangible output. Show code, not just \
             descriptions. Include examples with real values. Discuss \
             implementation details, memory layouts, performance characteristics."
        ),
        Zone::Counteracting => directives.push(
            "The user values conceptual models and abstractions. Lead with \
             design patterns and architecture. Use diagrams over code dumps. \
             Discuss trade-offs at the interface level."
        ),
        Zone::Accommodating => {}
    }

    directives.join("\n\n")
}
```

### 4.4 Complementary vs Adversarial Mode

Two adaptation strategies:

**Complementary (default)**: Support the user's natural MO. High Quick Start gets rapid-fire prototyping. High Fact Finder gets deep analysis. This is comfortable and productive for routine work.

**Adversarial (opt-in)**: Deliberately counter the user's MO to compensate for blind spots. High Quick Start gets "wait, have you considered the risks?" High Fact Finder gets "you have enough information to decide -- what's your call?" This is uncomfortable but valuable for growth, learning, and catching errors.

The mode is controlled by user preference with a per-conversation override:

```
/conative mode complementary   (default -- support my style)
/conative mode adversarial     (challenge my blind spots)
/conative mode neutral         (no adaptation)
```

### 4.5 Stage 5: QualityScorer Enhancement

As described in Section 3.6, the `FitnessScorer` gains a conative alignment dimension. After each response, the scorer evaluates whether the response style matched the user's profile. This feeds into the trajectory.

### 4.6 Stage 6: LearningBackend Enhancement

The `TrajectoryLearner` already records `Trajectory` objects with request, routing, response, and quality data. The conative-aware learner adds:

- The conative profile that was active during the interaction
- Which adaptation directives were injected
- The conative alignment score from the quality scorer
- Whether the user explicitly liked/disliked the adaptation (thumbs up/down, correction, or override)

Over time, this builds a corpus of "what adaptation strategies work for what conative profiles" that transcends individual users.

### 4.7 Interaction with Tiered Model Routing

The existing 3-tier model routing (Agent Booster / Haiku / Sonnet-Opus) is preserved. Conative adaptation does not change which tier handles the request for Tier 1 (WASM transforms) since those are deterministic. For Tier 2 (Haiku) and Tier 3 (Sonnet/Opus), the conative directive is injected into the system prompt by the ContextAssembler regardless of which model is selected.

---

## 5. Org Chart Integration

### 5.1 Extending OrgNode

The existing `OrgNode` in `clawft-types/src/company.rs` has `agent_id`, `role`, `reports_to`, `budget_cents`, and `goals`. It represents agent positions in the org chart. For human org charts (the assessment product), we extend with conative data:

```rust
/// Conative profile attached to an org chart node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConativeProfile {
    /// Fact Finder score (1.0-10.0).
    pub fact_finder: f32,
    /// Follow Thru score (1.0-10.0).
    pub follow_thru: f32,
    /// Quick Start score (1.0-10.0).
    pub quick_start: f32,
    /// Implementor score (1.0-10.0).
    pub implementor: f32,
    /// Confidence level (0.0-1.0). Below 0.5 = insufficient data.
    pub confidence: f32,
    /// Source of the profile data.
    pub source: ConativeSource,
    /// When the profile was last updated.
    pub last_updated: DateTime<Utc>,
}
```

This extends the three-graph model from the assessment knowledge model (09-assessment-knowledge-model.md):
- Graph 1 (System Graph): unchanged
- Graph 2 (Org Graph): each person node gains a conative profile
- Graph 3 (Knowledge Graph): the `knows_about` edges gain context -- not just "does Jane know billing?" but "how does Jane naturally approach billing problems?"

### 5.2 Enriched Assessment Queries

With conative data on person nodes, the assessment product can answer new questions:

| Query | What It Reveals |
|-------|----------------|
| "Show me the conative distribution of the platform team" | Whether the team has balance across all four modes, or is skewed (e.g., all high Quick Start, no Follow Thru) |
| "Find knowledge transfer paths for the billing module" | Not just who knows billing, but who would learn it most naturally given their MO (high Fact Finder + high Follow Thru = systematic learner) |
| "Identify strain points in the engineering org" | People whose inferred MO diverges from their role's demands (e.g., high Quick Start person stuck in a compliance-heavy Follow Thru role) |
| "Optimize team composition for Project X" | Given a project's requirements (novel/experimental = needs Quick Start; compliance/audit = needs Follow Thru + Fact Finder), recommend team members whose MOs fit |
| "Predict communication friction between Jane and Marcus" | If Jane is FF=9/QS=2 and Marcus is FF=2/QS=9, flag that their communication styles will clash and suggest mitigation |

### 5.3 Heatmap Extension

The existing assessment heatmap (from 09-assessment-knowledge-model.md) shows knowledge depth per person per system. Add a conative overlay:

```
                    Auth  Billing  API  Deploy  Frontend  ML
Jane   [FF8/FT3]   ████   ████   ███   ██      █        ░
Marcus [QS8/FT7]   ███    █      ████  ███     ░        ░
Sarah  [FT9/FF6]   █      ░      ██   ████    ████     ░
Alex   [QS7/IM3]   ░      ░      █    ██      ████     ███
Priya  [FF7/IM8]   ██     ░      ░    █       ░        ████

Conative Balance:   Mixed  RISK   Good  Good   QS-heavy  Needs FT
```

### 5.4 Knowledge Transfer Path Selection

When the assessment identifies a knowledge gap (bus factor = 1 for the billing module), the conative profile influences the recommended transfer approach:

| Learner MO | Recommended Transfer Method |
|------------|---------------------------|
| High FF | Pair programming with documentation creation. Give them the spec first, then the code. Let them research before hands-on. |
| High FT | Structured onboarding checklist. Sequential module walkthrough. Regular checkpoints. |
| High QS | "Break it and fix it" approach. Give them a failing test suite and let them debug their way to understanding. |
| High IM | Hands-on lab environment. Clone the service, deploy locally, modify and observe. |

### 5.5 Communication Adaptation Recommendations

The assessment report includes per-pair communication guidance:

```
Jane (FF8, FT3, QS6, IM3) <-> Marcus (FF2, FT7, QS8, IM3)

Friction: Fact Finder (gap = 6 points)
  Jane will want to discuss evidence and history.
  Marcus will want to skip to action.

  Recommendation for Jane -> Marcus communication:
    Lead with the conclusion. Put evidence in an appendix.
    Don't expect Marcus to read a 3-page analysis before a meeting.

  Recommendation for Marcus -> Jane communication:
    Include your reasoning, not just your recommendation.
    Give Jane time to research before expecting a decision.
    Don't interpret her questions as resistance -- she's gathering data.

Synergy: Quick Start (both accommodating-to-initiating)
  Both are comfortable with experimentation.
  Use this as common ground during Fact Finder friction.
```

---

## 6. Team Dynamics Patterns

### 6.1 Classic Conative Archetypes in Engineering Teams

| Archetype | Typical MO | Natural Role | Risk |
|-----------|-----------|--------------|------|
| The Researcher | FF9/FT4/QS2/IM5 | Tech lead, architecture decisions | Analysis paralysis; never ships |
| The Builder | FF3/FT6/QS3/IM8 | Infrastructure, DevOps, hardware | Builds before validating requirements |
| The Innovator | FF4/FT2/QS9/IM5 | Product prototyping, hackathons | Starts everything, finishes nothing |
| The Systemizer | FF5/FT9/QS3/IM3 | Project management, QA, compliance | Resists change even when needed |
| The Generalist | FF5/FT5/QS5/IM5 | Cross-functional roles | No strong instinctive drive; can feel directionless |

### 6.2 Productive Conflict Pairs

| Pair | Conflict Mode | Productive Outcome |
|------|--------------|-------------------|
| High FF + Low FF | Information depth | One does deep research; other synthesizes to essentials. Together they produce thorough-yet-actionable output. |
| High FT + Low FT | Structure vs flexibility | One creates the system; other stress-tests it with edge cases and exceptions. |
| High QS + Low QS | Innovation vs stability | One generates ideas; other filters to viable ones. Together they innovate sustainably. |
| High IM + Low IM | Concrete vs abstract | One builds the prototype; other designs the architecture. |

### 6.3 Destructive Conflict Patterns

Conflict becomes destructive when one person imposes their mode on the other:
- High FF manager demands exhaustive documentation from a Low FF engineer (creates strain)
- High QS product manager constantly pivots, burning out a High FT engineer who needs to finish
- Low QS stakeholder blocks all experiments, frustrating a High QS team

The assessment product flags these patterns and provides mitigation recommendations.

### 6.4 Team Composition Optimization Algorithm

```
Input: project requirements (innovation_need, structure_need, research_need, build_need)
Input: candidate pool with conative profiles

Score each candidate:
  fit_score = (
    qs_score * innovation_need +
    ft_score * structure_need +
    ff_score * research_need +
    im_score * build_need
  ) / sum(needs)

Score team compositions for balance:
  balance_score = 1.0 - variance_across_modes(team_aggregate_scores)

Final team score = 0.6 * sum(fit_scores) + 0.4 * balance_score
```

---

## 7. Implementation Roadmap

### Phase 1: Foundation (Sprint 14-15)

**Agent-side only. No org graph integration yet.**

1. Define `ConativeProfile` struct in `clawft-types`
2. Implement the signal extraction layer:
   - Parse chat messages for FF/FT/QS/IM signal features
   - Start with agent interaction signals only (highest reliability, easiest to collect)
3. Implement the scoring algorithm (Section 2.6)
4. Build the `ConativeAwareAssembler` that wraps `TokenBudgetAssembler` and injects conative directives into the system prompt
5. Add `/conative` user command for viewing inferred profile and setting preferences
6. Store profiles in the existing `LearningBackend` trajectory data

**Ship criteria**: A user who interacts with the agent for 20+ messages gets a visible conative tendency (with confidence score), and responses adapt accordingly. The user can override or disable.

### Phase 2: Multi-Source Signals (Sprint 16-17)

**Expand signal sources beyond agent interaction.**

1. Git plugin integration: extract FF/FT/QS/IM signals from PR review behavior, commit patterns, code style
2. Meeting transcript processing: parse transcripts for speaking pattern signals
3. Chat/Slack integration: process message history for communication style signals
4. Implement EMA-based score updates in the DEMOCRITUS loop
5. Add conative evidence edges to the CausalGraph
6. Add `ConativeGraph` as a `StructureTag` in the CrossRefStore

**Ship criteria**: Conative profiles incorporate signals from at least 3 data sources. Confidence scores reflect multi-source corroboration.

### Phase 3: Org Graph Integration (Sprint 18-19)

**Assessment product integration.**

1. Add `ConativeProfile` to the Org Graph person nodes
2. Implement team-level conative analysis (balance, friction, synergy)
3. Build strain detection (inferred MO vs role demands)
4. Extend the assessment heatmap with conative overlay
5. Implement knowledge transfer path selection that considers conative profiles
6. Add per-pair communication recommendations to assessment reports

**Ship criteria**: The assessment product shows conative distribution for teams, flags strain points, and recommends transfer methods tailored to learner MO.

### Phase 4: Feedback Loop and Validation (Sprint 20+)

1. Implement the conative alignment score in `FitnessScorer`
2. Build A/B testing infrastructure: adapted responses vs neutral responses, measure user satisfaction
3. Create a voluntary self-assessment survey (not Kolbe -- our own simplified version) to validate inferred profiles
4. For customers with official Kolbe results: compare inferred vs official scores to calibrate the model
5. Implement adversarial mode
6. Build the team composition optimizer

**Ship criteria**: Measured correlation between inferred profiles and self-reported/official Kolbe scores. Demonstrated improvement in user satisfaction with conative adaptation vs baseline.

---

## 8. Privacy and Ethics Framework

### 8.1 When to Infer vs When to Ask

| Scenario | Approach | Rationale |
|----------|----------|-----------|
| Agent interaction (user's own sessions) | Implicit inference, always disclosed | The user is directly interacting with the agent. Adaptation is a feature, like autocomplete. Disclosed via `/conative` command. |
| Org assessment (analyzing others' data) | Explicit opt-in required | Profiling another person's conative style from their chat/PR/meeting data requires their informed consent. |
| Team analytics (aggregate patterns) | Anonymized by default; identified with consent | Team-level conative distribution can be shown without identifying individuals. Per-person views require that person's consent. |
| Hiring/promotion decisions | Prohibited | Inferred conative profiles must never be used as a factor in hiring, firing, promotion, or compensation decisions. This is an advisory tool, not a decision-making tool. |

### 8.2 Disclosure Requirements

Every conative adaptation must be traceable and explainable:

1. **Visibility**: The user can always see their inferred profile via `/conative show`
2. **Explanation**: When adaptation is active, the agent can explain why (on request): "I'm giving you a detailed response because your communication pattern suggests you value thorough analysis."
3. **Override**: The user can override any dimension: `/conative set fact_finder 3` forces the agent to treat them as low FF regardless of inference.
4. **Disable**: `/conative disable` turns off all adaptation.
5. **Data access**: The user can export all signals used to build their profile: `/conative export`
6. **Deletion**: The user can delete their profile: `/conative reset`

### 8.3 Legal Considerations

**GDPR (EU)**:
- Conative profiling constitutes "profiling" under Article 22. Automated decisions based solely on profiling that produce legal or similarly significant effects are restricted.
- Our mitigation: conative adaptation is a convenience feature, not a decision-making system. It does not produce legal effects. Users have full control (override, disable, delete).
- A Data Protection Impact Assessment (DPIA) is required before deployment in the EU.

**EU AI Act**:
- The AI Act prohibits emotion recognition systems in workplace settings except for medical/safety purposes. Conative inference is NOT emotion recognition (it measures stable action patterns, not transient emotional states), but the distinction must be clearly documented and defensible.
- Classification: likely "limited risk" (transparency obligations) rather than "high risk" (conformity assessment), provided it is not used for employment decisions.

**US (State-level)**:
- Illinois BIPA does not apply (no biometric data collected).
- California CCPA/CPRA: conative profiles are "inferences drawn from personal information" and are covered as personal information. Users must be able to access, correct, and delete.
- No federal US regulation currently prohibits behavioral profiling from text, but the FTC has taken enforcement actions against deceptive AI practices.

### 8.4 Ethical Guardrails

1. **Never label**: The system should present tendencies as "your communication pattern suggests..." not "you are a..." Labels are reductive and can be used to stereotype.
2. **Never pathologize**: All conative profiles are equally valid. Low Follow Thru is not a disorder. The system must never frame any profile as deficient.
3. **Never weaponize**: Conative profiles must never be used to justify micromanagement ("your Quick Start is too high, you need more Follow Thru"). They inform how to collaborate, not how to control.
4. **Confidence transparency**: Always show the confidence score. A profile inferred from 10 chat messages (confidence 0.2) should be treated very differently from one built on 500 multi-source signals (confidence 0.95).
5. **Cultural sensitivity**: Conative expression varies across cultures. Communication patterns that read as "low Fact Finder" in a direct-communication culture (US, Germany) may reflect cultural norms in an indirect-communication culture (Japan, Korea) rather than conative tendency. The system must be calibrated per cultural context or clearly disclaim this limitation.

### 8.5 Data Minimization

- Store conative profile scores, not raw interaction data. The profile is a statistical summary, not a surveillance log.
- Signals are processed and scored, then the scoring contributes to the EMA average. Individual signals are not retained beyond the DEMOCRITUS tick that processed them.
- ExoChain records profile updates (score changed from X to Y) but not the raw signals that caused the change.

---

## 9. Validation Approach

### 9.1 Internal Validation

**Temporal stability test**: A valid conative proxy should be stable over time. If a user's inferred profile changes dramatically week-to-week, the signals are noise, not signal. Measure autocorrelation of weekly profile snapshots. Target: r > 0.8 over 4-week windows.

**Cross-source consistency**: Signals from different data sources (chat, PR, meetings) should converge on similar scores for the same person. Measure inter-source agreement using Cronbach's alpha. Target: alpha > 0.7.

**Behavioral prediction**: If the inferred profile is accurate, it should predict future behavior. Test: given a person's profile, predict which of two response formats they prefer (detailed vs concise, structured vs freeform). Measure prediction accuracy against actual user preference expressed via feedback. Target: > 70% accuracy.

### 9.2 External Validation

**Self-assessment correlation**: Ask users to complete a brief self-assessment survey (4 questions, one per dimension: "When starting a new project, I instinctively..."). Compare self-reported tendencies to inferred scores. Target: Pearson r > 0.6 for each dimension.

**Official Kolbe correlation**: For customers who have taken the official Kolbe A Index and consent to share results, compare inferred scores to official scores. This is the gold standard validation. Target: Pearson r > 0.5 per dimension (lower target than self-assessment because the inference is from behavioral proxy, not direct measurement).

Note: we cannot claim our inference IS a Kolbe score. We can claim it correlates with Kolbe scores at a measured level. The distinction is legally important -- Kolbe Corp holds trademarks and may enforce against tools that claim to replicate their assessment.

### 9.3 Adaptation Effectiveness

**A/B testing**: For users with sufficient confidence in their profile, randomly assign some conversations to adapted mode and some to neutral mode. Measure:
- User satisfaction (explicit: thumbs up/down; implicit: conversation length, task completion rate)
- Task completion speed
- Follow-up question frequency (fewer follow-ups = better initial response fit)

Target: adapted mode shows statistically significant improvement (p < 0.05) in at least one metric without regression in others.

---

## 10. Prior Art and Competitive Landscape

### 10.1 Existing Tools

**Crystal Knows**: Uses DISC personality framework. Infers profiles from public data (LinkedIn, social media). Provides communication recommendations. Integrates with CRM tools. Limitations: DISC is affective (personality), not conative (action style). Does not use interaction data -- relies on public profile scraping.

**Humantelligence**: Team culture and collaboration platform using multiple assessment frameworks. Focuses on team dynamics and hiring. More assessment-oriented than interaction-adaptive.

**Receptiviti**: NLP-based personality and psycholinguistic analysis. Academic grounding in LIWC (Linguistic Inquiry and Word Count). Strong on text analysis, weak on multi-source behavioral signals.

### 10.2 Academic Research

- Mairesse et al. (2007): Established that Big Five personality traits can be predicted from text features. Foundational work.
- Schwartz et al. (2013): Large-scale personality prediction from Facebook language. Demonstrated that word choice correlates with personality at population scale.
- Park et al. (2015): Showed that automatic personality assessment from language is as accurate as self-other agreement on personality questionnaires.
- Auburn University (2025): Found that AI chatbots can infer personality traits as well as or better than traditional self-report measures.

### 10.3 Our Differentiation

No existing tool combines:
1. **Conative** (not just personality/affective) framework
2. **Multi-source behavioral signals** (not just text or public profiles)
3. **Real-time agent adaptation** (not just static reports)
4. **Organizational knowledge graph integration** (not just individual profiles)
5. **Cryptographic provenance** (ExoChain audit trail for every inference)
6. **ECC cognitive substrate** (DEMOCRITUS loop for continuous refinement)

The closest competitor is Crystal Knows, which operates in the personality/DISC space. Our approach is fundamentally different: we measure action patterns, not personality traits, and we adapt in real-time rather than providing static recommendations.

---

## Relevant Files

Architecture and types:
- `/claw/root/weavelogic/projects/clawft/crates/clawft-types/src/company.rs` -- OrgNode, OrgChart, Company types
- `/claw/root/weavelogic/projects/clawft/crates/clawft-types/src/goal.rs` -- Goal types for org alignment

Pipeline (where conative adaptation hooks in):
- `/claw/root/weavelogic/projects/clawft/crates/clawft-core/src/pipeline/traits.rs` -- 6-stage pipeline trait definitions
- `/claw/root/weavelogic/projects/clawft/crates/clawft-core/src/pipeline/assembler.rs` -- TokenBudgetAssembler (Stage 3)
- `/claw/root/weavelogic/projects/clawft/crates/clawft-core/src/pipeline/scorer.rs` -- FitnessScorer (Stage 5)
- `/claw/root/weavelogic/projects/clawft/crates/clawft-core/src/pipeline/learner.rs` -- TrajectoryLearner (Stage 6)
- `/claw/root/weavelogic/projects/clawft/crates/clawft-core/src/pipeline/tiered_router.rs` -- Tiered routing (Stage 2)

ECC / kernel (where conative graph lives):
- `/claw/root/weavelogic/projects/clawft/crates/clawft-kernel/src/causal.rs` -- CausalGraph, CausalEdge, CausalEdgeType
- `/claw/root/weavelogic/projects/clawft/crates/clawft-kernel/src/democritus.rs` -- DEMOCRITUS loop
- `/claw/root/weavelogic/projects/clawft/crates/clawft-kernel/src/cognitive_tick.rs` -- Cognitive tick service
- `/claw/root/weavelogic/projects/clawft/crates/clawft-kernel/src/crossref.rs` -- CrossRefStore, StructureTag
- `/claw/root/weavelogic/projects/clawft/crates/clawft-kernel/src/hnsw_service.rs` -- HNSW vector search

Assessment design:
- `/claw/root/weavelogic/projects/clawft/.planning/weftos.weavelogic.ai/09-assessment-knowledge-model.md` -- Three-graph assessment model
- `/claw/root/weavelogic/projects/clawft/docs/weftos/ecc-symposium/01-research-synthesis.md` -- ECC integration thesis

---

Sources:
- [Kolbe A Index](https://www.kolbe.com/kolbe-a-index/)
- [Interpreting Kolbe's 4 Action Modes](https://e3discovery.com/a-kolbe-consultant-explains-how-to-interpret-the-four-action-modes/)
- [Kolbe Action Modes Explained](https://melissafroehlich.com/kolbe-action-modes/)
- [Kolbe's Conative Index: Measuring Your Striving Instincts](https://psychology.town/fundamentals-of-mental-health/kolbes-conative-index-striving-instincts/)
- [Kolbe Wisdom: Three Parts of the Mind](https://www.kolbe.com/kolbe-wisdom/)
- [Kolbe Test - The Behavioral Scientist](https://www.thebehavioralscientist.com/glossary/kolbe-test)
- [Kolbe Statistical Handbook](https://assets.kolbe.com/wp-content/uploads/20190916202031/Kolbe-Statistical-Handbook.pdf)
- [Kolbe Capabilities and Research Report](https://e.kolbe.com/_shared/elements/research-validity/Kolbe-Capabilities-and-Research-Report.pdf)
- [Understanding Kolbe - Insight Strategic Concepts](https://www.insightsc.com/s/InsightSC-UnderstandingKolbe.pdf)
- [A Survey of Automatic Personality Detection from Texts (ACL 2020)](https://aclanthology.org/2020.coling-main.553.pdf)
- [Machine and Deep Learning for Personality Traits Detection (Springer 2025)](https://link.springer.com/article/10.1007/s10462-025-11245-3)
- [Personality in Just a Few Words: NLP Assessment (ScienceDirect 2025)](https://www.sciencedirect.com/science/article/pii/S0191886925000406)
- [Crystal Knows - Personality Platform](https://www.crystalknows.com/)
- [Emotion AI in the Workplace: US Legal Considerations (2026)](https://www.tandfonline.com/doi/full/10.1080/10580530.2026.2616521)
- [AI Employee Monitoring: How Does It Work and Is It Legal](https://clario.co/blog/ai-employee-monitoring/)
- [Biometrics in the EU: Navigating GDPR and AI Act (IAPP)](https://iapp.org/news/a/biometrics-in-the-eu-navigating-the-gdpr-ai-act)
