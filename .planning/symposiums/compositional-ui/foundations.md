# Foundations — Compositional UI for an Agentic OS

This is the short version. The AGENDA is long; this is the canon.

---

## The four predicates (every primitive must satisfy)

A primitive is a base UI element (Button, Panel, Form, Chip, Surface,
Field, Binding, Verb, Subscription). Before a primitive enters the
canon, it must be:

### 1. Ontology-addressable
Each primitive has a typed identity in the project's knowledge graph.
An agent queries the ontology to find the right primitive and its
configuration, it does not pattern-match on strings or guess. The
ontology is the ubiquitous language (Evans); the primitives are its
executable vocabulary.

### 2. Self-describing
Three things are first-class readable properties on every primitive,
not inferred from context:

- **State** — current value(s), dirty flags, validation status.
- **Affordances** — what operations the primitive supports right now,
  given its state and the caller's identity (read / write / subscribe
  / invoke / dismiss). Agents enumerate affordances; they do not
  attempt operations and hope.
- **Confidence** — where the primitive's displayed value came from
  (deterministic computation / model inference / last-known-cache /
  user input), its confidence interval if probabilistic, and its
  freshness.

### 3. Streaming-native
Partial values, progressive disclosure, out-of-order updates,
cancellation, and back-pressure are primitive-level concerns. A
primitive that only supports a single synchronous load-then-render
cycle is rejected. The protocol carries stream verbs (`open`, `chunk`,
`commit`, `cancel`) that every primitive understands.

### 4. Dual-operator
Every primitive is equally usable by:

- A human via pointer, keyboard, touch, or **voice** (voice is
  first-class, not an accessibility afterthought).
- An agent via `query` / `invoke` / `subscribe` against the primitive's
  ontology identity.

Neither path is a wrapper over the other. If a primitive only exposes
a human surface or only exposes an agent API, it's not a primitive
under this canon.

---

## The reverse-DDD arrow

Classical DDD runs `domain → model → code → UI`.

We run `substrate state + ontology → schema → composition → renderer`.

The flip happens at the UI step: the UI is not the output of
modelling, it is a *view* into a modelled substrate. The primitives
above are the legal views; the protocol describes compositions of
those views; the renderer (currently egui) is disposable.

---

## Rasmus four-layer crosswalk (today → tomorrow)

| Rasmus layer | WeftOS today | What the symposium must add |
|-|-|-|
| Semantic foundation (vector + KG) | AgentDB, HNSW indexing, graphify, ontology-navigator symposium | Make it the *default addressing mode* for every primitive. |
| Probabilistic kernel (intent under uncertainty) | Gate backend, governance counsel, ECC scoring | Protocol-level expression of confidence + delegation + interruption priority. |
| Model control plane (context-window as working memory) | Agentic-flow, ReasoningBank, EWC++ | Bind working memory to *surfaces*, not just agents. |
| Specialized agents (janitor / gatekeeper / archivist / strategist) | 60+ agent types, weaver daemon | Express these as surface-composable roles with typed affordances. |

## Rasmus six unsolved problems crosswalk (our answers)

| Problem | Our current answer | Where it's weak |
|-|-|-|
| Identity + delegation | ExoChain + dual signing + RVF crypto + governance gates + claims authorization | Delegation across org boundaries (cross-substrate identity) is unspecified. |
| Asynchronous execution | Mesh runtime + HeartbeatTracker + MeshClockSync; ECC for priority | Interruption policy per-surface is not yet a primitive. |
| Multi-agent protocols | MCP + A2A router + agentic-payments + claims | No surface-composition wire format exists yet — **this symposium's deliverable.** |
| Economics | DeFi layer + bond staking + trust progression + slashing | No payment allocation model for orchestration-as-value. |
| Training data lock-in | ECC (causal models, not user profiles) + trajectory portability | Export / portability spec not yet written. |
| Serendipity vs. efficiency | ECC hypothesis generation + weaver HYPOTHESIZE loop | Surface-level "introduce productive disruption" affordance not yet a primitive. |

**We are further ahead than Rasmus realises.** The symposium's job is
to make this visible by fitting a surface protocol on top that the
industry can read.

---

## Non-negotiable rejections

These are rejected at the ADR stage, no discussion:

- Framework lock-in (React, Tailwind, Next.js, Tauri as a hard
  requirement).
- Color as primary identity at the base layer — later, yes; base, no.
- Hand-designed screens (bespoke layout code per view is an
  architectural failure).
- CLI-shaped chat as the dev-panel primary surface.
- Primitives that satisfy three of the four predicates. All four, or
  it's not a primitive.

---

## Mandatory reading for every participant

1. Rasmus, *The Agentic Operating System* — the thesis we are
   executing, and whose unsolved problems we already partly answer.
2. Cooper, *About Face* — posture taxonomy, goal-directed design,
   behavioural/interface split.
3. Evans, *Domain-Driven Design* — ubiquitous language, bounded
   contexts, aggregates, context maps — **read with the arrow
   reversed**.
4. Mei et al., *AIOS: LLM Agent Operating System* (arXiv 2403.16971)
   and `agiresearch/AIOS` — academic prior art, crosswalk our kernel.
5. `CedarCopilot/cedar-OS` — abandoned but instructive primitives
   (state-registration, semantic mentions, spells, shadcn ownership
   model).
