# Session 9 — Agentic OS Canon (Framing Paper)

**Status**: Framing paper for the Compositional UI symposium. Every other
track (R1–R8) reads against this document. Canon defined in
`foundations.md` is binding; this paper positions it against the outside
literature (Rasmus, Mei et al., Cedar-OS) and against the actual
substrate shipped in `crates/`.

---

## Scope

One paragraph: this paper establishes what the symposium is *answering*.
Rasmus gives us the clearest public statement of what an agentic OS is
for and what remains unsolved; Mei et al. (AIOS) give us the clearest
academic kernel layout; Cedar-OS gives us the cleanest negative example
of primitive-level thinking trapped inside a framework. We crosswalk all
three against the WeftOS code actually on disk so downstream tracks can
stop arguing about whether we have primitive X and start arguing about
whether we have conformant primitive X. The four predicates and the
two-tier wrappers in `foundations.md` are the lens; reverse-DDD
(Evans, run backward — substrate+ontology → schema → composition →
renderer) is the arrow; the active-radar loop (display-as-pulse,
interaction-as-return, ECC-as-receiver, GEPA-as-learner) is the learning
engine every primitive must feed.

---

## Rasmus thesis digest

Rasmus proposes a **four-layer reference architecture** for an agentic
operating system, none of which is a "better Windows":

1. **Semantic Foundation.** "Storage becomes fundamentally semantic.
   Every document, email, chat, ticket, log, screenshot, and call
   transcript lands in a vector-native store with a knowledge graph."
   Addressing is by meaning, not by pathname. Natural queries like
   "the scenarios finance and product argued over right after the last
   board meeting" are first-class.

2. **Probabilistic Kernel.** A kernel that "arbitrates intent under
   uncertainty. It doesn't just know that a process requested an SMTP
   connection; it knows that an agent is trying to 'send this to the
   team' and has to decide who 'the team' is." Governance becomes a
   *core* function, balancing confidence, risk, and policy.

3. **Model Control Plane.** Models *are* the control plane. Orchestration
   and agent swarms replace what were "smart features" in individual
   apps. **"Context windows become the real working memory of the
   system."**

4. **Specialized Agents.** Four archetypes replace "features": *janitor*
   (dedup / hygiene), *gatekeeper* (filter / policy at the edge),
   *archivist* (narrative synthesis across history), *strategist*
   (cross-service orchestration).

On top of this stack Rasmus writes the clearest public statement of
**episodic UI**: *"Interfaces become episodic. The system throws up
exactly the UI needed to confirm a decision, tweak parameters, or
explore alternatives. It then gets out of the way."* This is a
Cooper-style posture claim dressed in 2025 vocabulary — it maps almost
exactly onto Cooper's **transient posture** (*About Face*), which is
used briefly, then dismissed, and which fails if it does not respect
the sovereign posture of whatever the user was doing before.

Rasmus then names **six unsolved problems**:

1. **Identity & delegation.** Strategists act with the user's authority
   across organisational boundaries. Delegation rules, audit trails,
   revocation — absent.
2. **Asynchronous execution.** Request/response collapses when agents
   run during meetings or sleep. Needs "a new kind of scheduler, one
   that understands confidence intervals, user preferences about
   interruption, the cost of being wrong, and the opportunity cost of
   waiting."
3. **Multi-agent protocols.** "When my strategist agent and your
   strategist agent try to schedule a meeting, who arbitrates?"
4. **Economics.** "Who gets billed?" when a strategist touches six
   services of mixed ownership.
5. **Training-data lock-in.** The system gets better the more a user
   uses it; the profile it builds is stickier than the data itself —
   no portability standard.
6. **Serendipity vs. efficiency.** Optimisation kills creativity;
   "productive disruption" lacks clean metrics.

The strategic claim: **"Microsoft needs to scrap Windows and start
over."** That's not a viable product strategy for them, and that's the
opening. The player who *ships and open-sources the coordination
protocol* wins.

---

## Our substrate crosswalk — Rasmus layers

For each of Rasmus's four layers, what WeftOS ships, a 0–5 maturity
score, and what's missing.

### Layer 1 — Semantic Foundation → **4/5**

Ships:
- `crates/clawft-kernel/src/causal.rs` — `CausalGraph`, `CausalNode`,
  `CausalEdge`, `CausalEdgeType`, spectral analysis (λ₂ + Fiedler).
- `crates/clawft-kernel/src/vector_hnsw.rs`,
  `vector_diskann.rs`, `vector_hybrid.rs`,
  `vector_quantization.rs` — multiple vector backends.
- `crates/clawft-kernel/src/embedding.rs`,
  `embedding_onnx.rs` — embedding pipeline.
- `crates/clawft-kernel/src/hnsw_service.rs`,
  `hnsw_eml.rs` — HNSW service layer.
- `crates/clawft-graphify/` — ingest-to-graph pipeline.
- `crates/exo-resource-tree/src/model.rs` —
  `ResourceId`, `ResourceKind`, `ResourceNode` — the typed tree that
  backs ontology-addressable identity.

Missing:
- Ontology as the *default* addressing mode for UI primitives. The
  `exo-resource-tree` is the right substrate but there is no rendered
  path from `ResourceId` → UI primitive identity yet. That is exactly
  what the surface protocol (this symposium) must add.

### Layer 2 — Probabilistic Kernel → **4/5**

Ships:
- `crates/clawft-kernel/src/gate.rs` — `CapabilityGate`,
  `GovernanceGate`, `GateDecision`.
- `crates/clawft-kernel/src/governance.rs` — `GovernanceEngine`,
  `GovernanceRequest`, `GovernanceResult`, `EffectVector`,
  `GovernanceDecision`, `TrajectoryRecord`, `TrajectoryRecorder`,
  plus RVF bridging (`to_rvf_policy_check`, `to_rvf_policy`).
- `crates/clawft-kernel/src/capability.rs` —
  `CapabilityChecker`, `AgentCapabilities`, `SandboxPolicy`,
  `CapabilityElevationRequest`.
- `crates/clawft-kernel/src/eml_coherence.rs` —
  `EmlCoherenceModel`, `GraphFeatures`, `CoherencePrediction`
  (confidence as a first-class kernel output, not just a log line).
- `crates/clawft-kernel/src/causal_predict.rs`,
  `democritus.rs` — the ECC control loop feeding coherence back to
  policy.

Missing:
- Governance is a request-level decision boundary today; it is not yet
  addressable *on a primitive*. A primitive's `affordances` field has
  to be computed as `raw_affordances ∩ governance.permit(caller, pid,
  resource)`. That handshake exists in pieces and needs to be lifted
  into the surface protocol.

### Layer 3 — Model Control Plane → **3/5**

Ships:
- `crates/clawft-kernel/src/agent_loop.rs`, `agency.rs` (`Agency`,
  `AgentManifest`, `AgentInterface`, `InterfaceProtocol`,
  `ResponseMode`), `weaver.rs` (`WeaverCommand`, `WeaverResponse`,
  `ModelingSession`, `ConfidenceGap`, `ConfidenceReport`,
  `ModelingSuggestion`).
- `crates/clawft-kernel/src/a2a.rs` — `A2ARouter` with topic routing,
  inbox management, mesh-runtime binding, dead-letter queue.
- `crates/clawft-kernel/src/topic.rs` — `TopicRouter`, `Subscription`
  (streaming-native plumbing).
- `crates/clawft-kernel/src/cognitive_tick.rs`,
  `democritus.rs` — DEMOCRITUS SENSE→EMBED→SEARCH→UPDATE→COMMIT tick
  as the closed-loop scheduler.
- `crates/clawft-kernel/src/heartbeat.rs` (`HeartbeatScheduler`,
  `HeartbeatConfig`, `HeartbeatPhase`).

Missing:
- **Binding working memory to surfaces, not just agents.** Today a
  context window is bound to an agent session. Rasmus's framing
  requires the *surface* to be the memory carrier — a Mission Console
  outlives the agent instance by design. We need a surface-scoped
  working-memory primitive that agents attach to.
- **Variant-id provenance on renders** — required for the active-radar
  loop to attribute return signals to the pulse that produced them.

### Layer 4 — Specialized Agents → **3/5**

Ships:
- 60+ agent types in our own classification (see
  `~/CLAUDE.md` agent catalogue).
- `crates/clawft-kernel/src/weaver.rs` — weaver daemon, the archivist
  analogue.
- `crates/clawft-kernel/src/agency.rs` — `AgentRole` (Root,
  Supervisor, Service, …), the supervisor/strategist spine.
- `governance-counsel` agent + `GovernanceGate` — the gatekeeper.
- Process table + `supervisor.rs` + `reconciler.rs` — the janitor
  (dedup, reconcile).

Missing:
- Rasmus's archetypes are roles-for-composed-surfaces. We have them as
  roles-for-agents. Mapping each archetype to a *set of surface
  affordances* (archivist = timeline + crossref chip + narrative
  panel; strategist = plan tree + delegation chip; …) is on us.

---

## Our substrate crosswalk — Rasmus's six unsolved problems

### 1. Identity & delegation → **4/5**

Ships:
- Dual-signed chain (`crates/clawft-kernel/src/chain.rs`): Ed25519 +
  ML-DSA-65 (`has_dual_signing`), checkpoints, witness verification,
  attestations.
- ExoChain consensus (`chain.rs`, `mesh_chain.rs`).
- `crates/clawft-types/src/delegation.rs` — `DelegationConfig`,
  `DelegationRule`, `DelegationTarget`.
- Governance gates + `GovernanceRule` severity classes; `revocation.rs`
  in kernel.
- Claims / RVF auth layer via `auth_service.rs`.

Missing: cross-substrate delegation. Today delegation is one chain
deep. Rasmus's pain point is *organisation to organisation* — that's
the cross-chain identity handshake, scoped out but not yet specified.

### 2. Asynchronous execution → **4/5**

Ships: `mesh_runtime.rs`, `mesh_heartbeat.rs`, `MeshClockSync`
(`sync_clock_from_peer`, `mesh_time_us`, `clock_uncertainty_us`),
`HeartbeatScheduler`, `ReliableQueue`, `ImpulseQueue`, `DeadLetterQueue`,
ECC coherence as a priority signal.

Missing: *interruption policy per surface*. We have interruption
heuristics on agents; we do not yet have a **per-surface interruption
posture** (Cooper's sovereign vs transient distinction expressed as a
protocol field). This is a primitive-level decision — it must be
read off the surface, not inferred by the kernel.

### 3. Multi-agent protocols → **3/5**

Ships: MCP, `A2ARouter`, `TopicRouter`, agentic-payments claims, mesh
framing (`mesh_framing.rs`), MessagePayload::Binary (v0.6.16).

Missing: **a surface-composition wire format** — the deliverable of
this symposium. We have routing; we do not yet have a schema for
"surface", "primitive", "subscription-bound-to-a-resource",
"verb-with-variant-id".

### 4. Economics → **2/5**

Ships: DeFi bond staking + trust progression + slashing (externally
documented, in `weftos-leaf-types` / related). No direct UI tie-in.

Missing: **orchestration-as-value pricing** — how a strategist
surface composed across four services bills back. Out of scope for
this symposium but the protocol must leave *room* (cost annotations
on verbs; consent flows that a payments subsystem can read).

### 5. Training-data lock-in → **5/5** (we are ahead)

Ships: ECC stores **causal models**, not user profiles. See
`eml-causal-collapse-research.md` and `eml_coherence.rs`. The
perturbation formula δλ₂ = w·(φ[u]−φ[v])² is an elementary function,
i.e. the model itself is expressible and portable.
`ReasoningBank` + trajectory portability makes the shape exportable;
the chain makes the provenance portable.

Missing: an **export spec** (the file format + signing + consent
ratchet) and a **revocation spec** on what happens when a user
revokes consent mid-trajectory. Not a primitive-level concern but
the protocol must never prevent export.

### 6. Serendipity vs. efficiency → **3/5**

Ships: `WeaverCommand::Hypothesise` flows, `ModelingSuggestion`,
`ConfidenceGap` drives affirmative probes. GEPA-style mutation is
already named in `foundations.md` as the learning step on the active-
radar loop. DEMOCRITUS's HYPOTHESIZE phase is the substrate-level
serendipity engine.

Missing: a **"productive disruption" affordance** on surfaces — a
typed way for a surface to suggest the user *consider* something
off-path. Today that would be an ordinary notification chip; it
should be a named primitive with its own consent and rate-limit
semantics, because "introduce entropy" and "interrupt with news" are
two different governance classes.

---

## AIOS (Mei et al.) crosswalk

Mei et al. (*AIOS: LLM Agent Operating System*, arXiv:2403.16971) and
`agiresearch/AIOS` frame the kernel as a six-to-seven module stack:
**LLM Core, Scheduler, Context Manager, Memory Manager, Tool Manager,
Storage Manager, Access Control**. Agents enter via *syscalls*; the
SDK (`agiresearch/Cerebrum`) is the authorship surface. UI is *Web
UI* plus a *Terminal UI* (an LLM-based semantic filesystem shell).

| AIOS component | Our analogue | Where we're ahead | Where they're ahead |
|-|-|-|-|
| LLM Core | `clawft-llm` + provider plane | Multi-provider routing + tiering (ADR-026) | Cleaner multi-tenant inference pooling |
| Scheduler | `HeartbeatScheduler`, `TopicRouter`, DEMOCRITUS tick | ECC-weighted priority, coherence-driven; `mesh_runtime` has distributed scheduling | FIFO/Round-Robin pluggability is more explicit |
| Context Manager | `agent_loop.rs` + `profile_store.rs` + ReasoningBank | Causal provenance on every context item | Context-switch primitives are first-class in their API |
| Memory Manager | AgentDB + HNSW + `eml_persistence.rs` | 150× semantic search, quantised storage, HNSW-EML hybrid | Simpler to reason about as a Python object store |
| Tool Manager | `clawft-tools` + 12-plugin set under `clawft-plugin-*` + MCP bridge | Governance-gated tool invocation, capability elevation flow, sandboxing | VM controller + MCP server for computer-use are already wired |
| Storage Manager | `artifact_store.rs`, `persistence.rs`, chain | Content-addressed + chain-anchored | Nothing they do materially better |
| Access Control | `gate.rs`, `governance.rs`, `capability.rs` | RVF crypto + chain-anchored audit, `EffectVector` on every request | Theirs is simpler, ours is correct |

**Where AIOS is ahead**: nothing on the substrate. They are ahead on
*legibility to academics* — a clean Python stack with named modules
everyone can read. We should imitate that legibility, not their
architecture.

**Where we are ahead**: everything that touches the real world —
governance, chain, mesh, ECC, confidence-as-output.

**Primitives we lack that they have**: a published syscall shape for
agents. Our RPC is typed (`crates/clawft-rpc/src/protocol.rs` —
`Request`, `Response`, `runtime_dir`, `socket_path`) but we do not
have an "agent syscall" nomenclature that a reader of the AIOS paper
could map onto one page. Fixable by naming, not by rewriting.

**AIOS's UI story**: a Web UI and an LLM-in-a-terminal. This is exactly
the posture the canon **rejects** ("No CLI-shaped chat as the
dev-panel primary surface"). The academic prior art concedes the UI
layer; that concession is the gap this symposium exists to close.

---

## Cedar-OS lessons

Positive primitives to adopt:

1. **State registration (`useCedarState`)** — state that an agent can
   read and mutate through a typed API, not by scraping the DOM. This
   *is* predicate 2 (self-describing) applied to state. Adopt, but
   bind to `ResourceId` from `exo-resource-tree`, not to a React key.
2. **Semantic mentions (`@user`, `@file:…`, `@state:…`)** — ontology-
   addressable references inside free-text input. This is predicate 1
   (ontology-addressable) in the input modality. Adopt — every
   Input-type primitive must carry a mention-resolution channel whose
   namespaces are the ontology's node kinds.
3. **Spells & Quick Actions radial** — goal-directed discoverability
   (Cooper). Adopt as a tier-A `SpellTray` primitive that queries the
   ontology for "verbs legal on the current selection" and renders
   the result as a radial. Note this is **not** a right-click menu;
   it is the agent-facing affordance enumeration visualised.
4. **Shadcn-style "you own the code"** — the renderer is disposable,
   the user's copy is theirs. Matches foundations: "renderer
   (currently egui) is disposable." Adopt as a philosophical
   precedent.

Negative lessons (do **not** inherit):

1. **React-only lock-in.** Cedar is a React framework. We rejected
   framework lock-in in foundations ("No Tailwind, no React, no Next,
   no Tauri"). Every Cedar primitive must be re-expressed renderer-
   agnostically — the protocol carries the primitive, the renderer
   instantiates it (egui today; anything tomorrow).
2. **No confidence.** Cedar has no notion of "this value came from a
   model inference with 0.6 confidence". Predicate 2 demands it.
3. **No partial views / streaming-native affordances.** Cedar is a
   single-shot state framework. Predicate 3 is a first-class
   requirement for us.
4. **No agent-emission signals.** Cedar has no structured return
   channel for "the agent finished" or "the agent needs intervention";
   it relies on streamed text. We need structured *return-signal*
   events (topology / doppler / range / bearing) on every primitive
   render — see foundations, active-radar section.

**One-line summary**: Cedar got three primitives right, took them out
of the framework wrapper, and we have what we need. Leaving them in
their React wrapper is the negative example.

---

## Recommendations for canon

Ten numbered recommendations. Each cites which predicate / tier /
framing element it serves. Any downstream track that wants to deviate
from one of these must write an ADR that explicitly names the
recommendation and argues it down.

1. **Every primitive resolves through `ResourceId` from
   `exo-resource-tree`.** Ontology identity is a typed path
   (`ResourceId("board/finance/april-forecast")`), not a string key.
   *Serves predicate 1 (ontology-addressable); Evans — ubiquitous
   language made executable; rejects Cedar's React-key addressing.*

2. **Every primitive render carries `{state, affordances, confidence,
   variant_id, provenance}` as first-class fields.** `variant_id` is
   mandatory for the active-radar loop; `confidence` must include
   provenance (deterministic | model | cache | user).
   *Serves predicate 2; mandatory for GEPA learning step; closes
   Cedar's missing-confidence gap.*

3. **The canvas protocol in `crates/clawft-types/src/canvas.rs`
   (`CanvasElement`, `CanvasCommand`, `CanvasInteraction`) is the
   nucleus of the surface wire format, but its current element set is
   not yet predicate-compliant.** It must be extended with
   `ontology_id`, `confidence`, `affordances`, `variant_id`, and a
   `ForeignSurface` variant with explicit capability set. Do not
   replace the crate; *extend* it — it already solves serialisation
   and round-trip, is referenced by `clawft-gui-egui/src/live.rs`, and
   is the only substrate-blessed UI type boundary today.
   *Serves all four predicates; preserves reverse-DDD arrow (schema
   first, composition second, renderer third).*

4. **Tier B wrappers are a single primitive, `ForeignSurface`, whose
   inner content emits typed signals (presence / identity / activity /
   outcome).** Ambient capture wrappers (`foreign://camera`,
   `foreign://mic`, `foreign://screen`) are *first-class*,
   governance-gated by default, signal-lossy by policy, and visibly
   observable to the user. This is non-negotiable.
   *Serves tier-A/B split; enforces digital-exhaust-as-intent privacy
   constraints; prevents Tier-C creep.*

5. **Every primitive exposes a `subscribe` verb returning a stream of
   `{open, chunk, commit, cancel}` envelopes.** Back-pressure is a
   protocol-level concern, not an application-level workaround.
   Binding to `TopicRouter` + `A2ARouter` is the in-process path;
   `mesh_runtime` is the distributed path; the envelope shape is the
   same.
   *Serves predicate 3 (streaming-native); leverages existing
   `crates/clawft-kernel/src/topic.rs`, `a2a.rs`.*

6. **Every primitive is dual-operator: a human via pointer / keyboard /
   touch / voice, and an agent via `query` / `invoke` / `subscribe` on
   the same ontology identity.** Voice is a first-class channel (see
   `clawft-types/src/config/voice.rs` — `VoiceConfig`, `SttConfig`,
   `TtsConfig`, `VadConfig`, `WakeConfig` — we already have the
   config surface); agent and human paths are peers, not wrappers.
   *Serves predicate 4; closes the Cooper "behavioural vs interface"
   seam — the agent composes behaviour, the renderer paints pixels or
   speaks audio; same predicate, two transducers.*

7. **Interruption posture is a declared property on every primitive,
   not inferred.** `InterruptionPosture` has at least
   `{sovereign, transient, daemon, ambient, safety}` — lifted
   directly from Cooper's posture taxonomy (*About Face*, posture
   chapter: sovereign posture owns the user's attention, transient
   comes and goes, daemon runs headless, parasitic/auxiliary
   decorate). The kernel's async scheduler (Rasmus problem #2) reads
   this field when deciding whether to deliver an impulse.
   *Serves predicate 2 (self-describing); closes Rasmus unsolved
   problem #2's "preferences about interruption".*

8. **Mutation is permitted only along axes the primitive declares
   legal, and only outside fixed axes (brand marks, safety
   affordances, consent flows).** GEPA operates only within the
   `mutation_schema` declared by the primitive type. Any agent that
   mutates a fixed axis is a protocol violation and gets gated, not
   rate-limited.
   *Serves active-radar loop; enforces "non-negotiable privacy
   constraints" of foundations; prevents the class of "A/B testing
   on consent UI" bugs that every adtech SDK has shipped.*

9. **Goals are resources, not agent state.** A `Goal` (see
   `crates/clawft-types/src/goal.rs` — `Goal`, `GoalTree`, `GoalStatus`)
   lives in the substrate and is subscribable like any other resource.
   Agent sessions come and go; the goal tree persists. The surface
   reads goals; governance reads goals; the active-radar ECC reads
   goals. This is the reverse-DDD (Evans) arrow at scale — the domain
   object drives the surface, never the other way round.
   *Serves RQ8 (constitutional governance); predicate 1; reverse-DDD
   framing; binds Layer 3 (Model Control Plane) working memory to
   surfaces instead of agent instances.*

10. **Return signals (topology, doppler, range, bearing, strength) are
    emitted by every primitive on every interaction, with variant-id
    attribution, and are fed to ECC before they are permitted to
    train anything.** ECC is the receiver; GEPA is the learning step;
    the chain (`ChainManager.append`) is the provenance audit so
    `TrainingDataLockIn` (Rasmus problem #5) is answered by portable,
    signed, revocable causal models, not by off-node aggregates.
    *Serves active-radar loop; closes Rasmus problem #5; makes the
    four predicates forward-learning rather than merely descriptive.*

---

## Open questions (for R1–R8 to pick up)

1. **Wire format**: CBOR vs FlatBuffers vs JSON-on-the-wire with a
   CBOR fast path? `canvas.rs` is JSON today. R6 owns this — the
   recommendation here is "extend `canvas.rs`, don't replace it", but
   the encoding choice for the streaming envelope is open.

2. **`ForeignSurface` capability discovery**: how does the shell
   announce its own capability set to the agent (`foreign://browser`
   with or without a11y-tree introspection, with or without
   execute-script)? Needs a capability-declaration schema. R5 / R7.

3. **Cross-renderer portability proof**: foundations say "a second
   renderer can be written against the protocol without touching the
   composition logic." Which second renderer do we ship as the proof?
   R5 / R7 converge here — a minimal web renderer or a TUI renderer
   are both adequate proofs.

4. **Cross-substrate delegation (Rasmus #1)**: the cross-org handshake
   is scoped out of this symposium but the surface protocol must not
   foreclose it. R8 owns the constitutional-governance face; R6 owns
   the wire face; whichever track has the cleaner insight should
   draft a non-binding placeholder.

5. **Mutation-schema for the foundation primitives**: which axes are
   GEPA-legal on a `Button`? On a `Form`? On a `SpellTray`? Needs a
   per-primitive declaration table. R5 owns primitive catalogue, R3
   owns the "how do shared surfaces mutate differently from private
   ones" question.

6. **Agent-archetype → surface-affordance mapping**: Rasmus's
   archivist / janitor / gatekeeper / strategist archetypes need
   composed-surface templates. R2 (agentic UI state of the art) is
   best positioned to draft these against concrete competitors
   (Cursor / Claude Code / Devin / v0).

7. **Episodic surface lifecycle**: an episodic UI surfaces, completes,
   dismisses. Where does its state go? ECC keeps causal traces; what
   about the transient form values? R4 (ontology → UI) owns the
   "what is retained and how is it re-addressed later" question.

8. **Cedar radial spells in voice**: radial menus assume a pointer.
   A voice-first analogue (utterance grammar generated from the same
   affordance enumeration) is not obvious. R2 / R7.

---

## What to read next

**R1 (historical canon)**: treat the "Rasmus thesis digest" and
"AIOS crosswalk" sections as load-bearing — your job is to test
Rasmus against every protocol that actually shipped (X11, Wayland,
NeWS, Naked Objects, Morphic) and find which of his four layers they
quietly already had.

**R2 (agentic UI SOTA)**: "Cedar-OS lessons" and recommendations 2,
5, 10 are load-bearing — your comparator matrix must score Cursor /
Claude Code / Devin / v0 / Continue on each of the four predicates
and call out which have `variant_id` at all.

**R3 (shared surfaces)**: recommendation 7 (interruption posture) and
recommendation 4 (Tier B signals) are load-bearing — shared surfaces
change posture per-user, which is a harder case than the single-user
one foundations.md implies.

**R4 (ontology → UI)**: recommendation 1 (ResourceId addressing) and
recommendation 9 (Goals as resources) are load-bearing — Naked
Objects is the nearest prior art for reverse-DDD rendering and your
comparison must be explicit.

**R5 (renderer contracts)**: recommendation 3 (extend canvas.rs),
recommendation 5 (streaming envelope), and recommendation 8
(mutation schema) are load-bearing — your primitive catalogue
assembles directly on top of those three recommendations.

**R6 (protocol design)**: recommendation 3, recommendation 5, and
every substrate-crosswalk file-path citation (`a2a.rs`, `topic.rs`,
`chain.rs`, `canvas.rs`, `mesh_runtime.rs`) are load-bearing —
the protocol exists in pieces; your job is to unify them.

**R7 (dev-panel embedding)**: recommendation 4 (ForeignSurface),
recommendation 6 (voice as peer), and the Tier B wrapper list from
`foundations.md` are load-bearing — embedding in Cursor/VSCode *is*
a `foreign://vscode-panel` conversation, shipped in reverse.

**R8 (goals as governance)**: recommendation 9 (Goals as resources)
and the "substrate crosswalk — Probabilistic Kernel / Governance"
section are load-bearing — your track is the *only* one that touches
`governance.rs` and `gate.rs` directly, and you must respect the
`EffectVector` / `GovernanceDecision` shape already shipped.
