# Learner Model — Adaptive Knowledge Tracking

## Concept

Invert the knowledge graph onto the user. The tour guide agent doesn't just know about WeftOS — it tracks what the visitor knows, what they've been exposed to, what they understood, and what gaps remain. This same pattern applies to any codebase assessment: tracking what each team member knows about the system.

## The ECC Pattern Applied to Learning

The ECC (Ephemeral Causal Cognition) framework already has the structures needed:

| ECC Structure | Codebase Use | Learner Use |
|--------------|-------------|-------------|
| **CausalGraph** | Module dependencies, data flow | Concept prerequisites, learning paths |
| **HNSW Search** | Find related code/docs | Find related concepts the user should learn next |
| **CrossRef** | Link code ↔ docs ↔ tests | Link concept ↔ exposure event ↔ comprehension signal |
| **ExoChain** | Audit trail of system changes | Audit trail of learning events |
| **DEMOCRITUS** | Continuous system assessment | Continuous comprehension assessment |

## Learner Graph Structure

### Nodes: Concepts

Each concept is a node in the learner's graph:

```json
{
  "id": "governance-constitutional",
  "label": "Constitutional Governance",
  "category": "kernel",
  "prerequisites": ["agent-pipeline", "governance-basics"],
  "depth_levels": ["awareness", "understanding", "application", "mastery"],
  "related_docs": ["/docs/weftos/governance"],
  "related_code": ["crates/clawft-kernel/src/governance.rs"]
}
```

### Edges: Prerequisites and Relationships

```
agent-pipeline ──requires──► llm-providers
governance-constitutional ──requires──► agent-pipeline
governance-constitutional ──requires──► governance-basics
mesh-networking ──enhances──► governance-constitutional
provenance ──proves──► governance-constitutional
```

### Properties: Learner State (per user, per concept)

```json
{
  "concept_id": "governance-constitutional",
  "exposure": "explained",
  "exposure_count": 2,
  "first_seen": "2026-04-02T14:30:00Z",
  "last_seen": "2026-04-02T14:35:00Z",
  "comprehension_signals": {
    "asked_followup": true,
    "asked_clarification": false,
    "applied_in_question": false,
    "confused_indicators": 0
  },
  "depth": "understanding",
  "confidence": 0.7,
  "next_recommended": ["provenance", "mesh-networking"]
}
```

## Comprehension Signals

The agent infers understanding from conversation patterns:

| Signal | Inference | Weight |
|--------|-----------|--------|
| User asks "what is X?" | First exposure, awareness level | +exposure |
| User asks "how does X work?" | Seeking understanding | +depth |
| User asks "can X do Y?" | Applying concept to their use case | +application |
| User references X correctly in a question about Z | Mastery — using X as building block | +mastery |
| User asks "wait, what's X?" after it was explained | Confusion, needs re-explanation | -comprehension |
| User skips/ignores X explanation | Low interest or already knows | neutral (note skip) |
| User asks about Y without having seen prerequisite X | Gap detected | flag prerequisite gap |
| User compares X to competitor feature | Evaluating, high engagement | +interest |

## Agent Behavior

### Gap Detection
```
Agent thinks: "User is asking about mesh networking, but hasn't been
exposed to the agent pipeline concept. Mesh coordination requires
understanding how agents are spawned and managed. I should bridge
this gap first."

Agent says: "Great question about mesh networking! Before we dive in,
it helps to understand how WeftOS manages agents — mesh networking
coordinates multiple agents across machines. Want a quick overview
of the agent pipeline first, or are you already familiar with that?"
```

### Adaptive Depth
```
Agent thinks: "User has asked 3 deep technical questions about
ExoChain and referenced BLAKE3 specifically. They're at 'application'
level. I can skip surface explanations and go straight to
implementation details."

Agent says: "Since you're familiar with the hash chain structure,
here's how the dual-signing works with Ed25519 + ML-DSA-65..."
```

### Progress Awareness
```
Agent thinks: "User has explored 4 of 7 major concept areas.
They haven't seen ECC or mesh networking yet. They seem most
interested in governance (3 questions, all at understanding+ level)."

Agent says: "You've covered a lot of ground — agent runtime,
providers, tools, and governance. Two areas we haven't touched:
the cognitive layer (ECC) which gives agents persistent memory,
and mesh networking for multi-machine coordination. Which
interests you more?"
```

## Implementation Approach

### Phase 1: In-Memory Session Tracking (MVP — playground only)

Track learner state in browser memory during the playground session:

```typescript
interface LearnerState {
  concepts: Map<string, ConceptState>;
  sessionStart: Date;
  messageCount: number;
  topicsExplored: string[];
  currentDepth: 'browsing' | 'learning' | 'evaluating' | 'building';
}

interface ConceptState {
  exposure: 'unseen' | 'mentioned' | 'explained' | 'demonstrated' | 'applied';
  confidence: number;  // 0.0 - 1.0
  interactions: number;
  lastSeen: Date;
}
```

The system prompt includes the learner state summary:

```
## Learner Context

This user has been in the playground for 8 minutes and sent 12 messages.
They appear to be evaluating WeftOS for a project.

Concepts they understand well: agent-pipeline (0.8), providers (0.9)
Concepts they've seen but may need reinforcement: governance (0.5)
Concepts not yet explored: ecc, mesh, provenance, tools
Detected interest: governance, security (asked 4 questions)
Detected gap: asked about mesh without knowing agent lifecycle

Adapt your responses accordingly. Don't re-explain concepts they
already understand. Bridge gaps when you detect missing prerequisites.
When they seem ready, suggest the next most relevant concept.
```

### Phase 2: Persistent Learner Graph (logged-in users)

For users who sign in (via the assessment login), persist their learner graph:

- Store as RVF segments with `namespace: "learner:{user_id}"`
- WITNESS chain tracks learning progression (tamper-evident education record)
- Cross-session continuity: "Last time you were exploring governance — want to pick up where you left off?"
- ExoChain events for each learning interaction

### Phase 3: Team Knowledge Mapping (consulting product)

Apply the same pattern to WeaveLogic client engagements:

- Each team member gets a learner graph against the codebase knowledge graph
- Overlay all learner graphs to find: "These 3 modules are only understood by 1 person"
- That's the tribal knowledge risk quantified
- Dashboard: concept coverage heatmap by team member
- Directly answers: "Your best developer just quit. How much did they take?"

## Concept Prerequisite Graph (WeftOS-specific)

Initial concept map for the WeftOS tour guide:

```
weftos-overview
├── agent-pipeline ← entry point for most visitors
│   ├── llm-providers
│   ├── tools-capability-system
│   ├── channels (Slack, Teams, web, CLI)
│   └── model-routing (tiered complexity)
├── kernel
│   ├── process-management (PIDs, lifecycle)
│   ├── governance-constitutional
│   │   └── governance-effect-vectors
│   ├── provenance-exochain
│   │   └── provenance-witness-chain
│   └── supervision-self-healing
├── ecc-cognitive
│   ├── causal-graph
│   ├── hnsw-semantic-search
│   ├── democritus-loop
│   └── embeddings
├── mesh-networking
│   ├── discovery
│   ├── transport-noise-encryption
│   └── cross-project-coordination
├── security
│   ├── wasm-sandbox
│   ├── capability-system
│   └── ssrf-protection
└── deployment
    ├── install-methods
    ├── docker
    ├── wasm-browser
    └── configuration
```

This graph ships as part of the RVF knowledge base — each concept node is a segment with `tags: ["concept-graph"]` and metadata containing prerequisites and depth levels.

## Generalization to Any Codebase

The learner model is not WeftOS-specific. When WeftOS assesses a client's codebase (SOP 2), it builds a knowledge graph of that system. The same learner tracking applies:

1. **New hire onboarding**: Agent guides them through the codebase, tracks what they've learned, identifies gaps
2. **Cross-training**: "Engineer A knows billing but not auth. Engineer B knows auth but not billing." Quantify and address.
3. **M&A due diligence**: "Of the acquired team, only 2 people understand the payment system. Risk: HIGH."
4. **Continuous learning**: As the codebase changes, the agent identifies what team members need to re-learn

This is the bridge from "open-source documentation tool" to "enterprise knowledge management platform" — the WeaveLogic consulting product, powered by WeftOS's own patterns.

## Connection to SOPs

- **SOP 2 (Building the Knowledge Graph)**: Produces the concept graph that the learner model tracks against
- **SOP 4 (Continuous Assessment)**: Triggers learner model updates when the codebase changes
- **SOP 5 (Iterative Improvement)**: Learner interaction data improves the concept graph and prerequisite map

## MVP for Sprint 14

For the playground, the MVP is lightweight:
1. Define the concept prerequisite graph as JSON (ship with RVF KB)
2. Track exposure/depth in browser sessionStorage
3. Include learner state summary in the system prompt
4. Agent adapts responses based on what it knows about the visitor

No persistence, no login, no dashboard. Just a smarter tour guide that doesn't repeat itself and knows when to go deeper vs. bridge gaps.
