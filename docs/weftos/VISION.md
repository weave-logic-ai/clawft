# WeftOS: The Vision

**How an automation agency became a cognitive operating system,
and why that matters for everyone building with AI.**

---

## The Arc

This project did not begin as WeftOS. It did not begin as a kernel, or a
causal graph, or a mesh network. It began as a business plan for an
automation agency called WeaveLogic AI.

### Phase 1: The Agency (2024)

WeaveLogic AI was conceived as an "AI-native development and automation
agency" -- a consultancy that would build N8N workflow automations and
custom AI solutions for SMEs. The business plan projected $850K-$1.2M in
Year 1 revenue, positioned the company as a "Ceiling-Raiser" (building
bespoke AI systems rather than connecting off-the-shelf tools), and laid
out service packages from $2,500 discovery engagements to $100K+
enterprise automation suites.

The core insight in the business plan was sound: the market was
bifurcated between "Floor-Raisers" (agencies using AI for internal
efficiency) and "Ceiling-Raisers" (firms that build AI into the product
itself). WeaveLogic aimed to be the latter.

But building custom AI solutions for clients revealed a deeper problem
than workflow automation could solve.

### Phase 2: Weave-NN -- The Knowledge Graph (2024-2025)

The first product to emerge was Weave-NN: "an AI-powered knowledge graph
for managing AI-generated documentation." The problem it addressed was
real and personal -- the team was drowning in AI-generated markdown. Every
Claude conversation, every planning session, every architecture review
produced documents that piled up with no structure, no linking, no memory.

Weave-NN started as an Obsidian vault with a Python MCP backend and
evolved through 14 phases over roughly a year:

- **Phases 1-3** (Foundation): Knowledge graph transformation. Flat files
  became interconnected nodes with bidirectional wikilinks and frontmatter
  metadata. 300+ documents structured into a navigable graph.

- **Phases 4-6** (Pre-MVP): MCP integration, vault initialization,
  Claude-Flow agent connectivity. The architecture settled on Obsidian
  desktop + Python FastAPI MCP server + RabbitMQ event bus.

- **Phases 7-9** (Core Systems): Agent rules, git automation, testing.
  The Rule Engine framework processed events with priority-based
  execution. Six core agent rules defined how AI interacted with the
  knowledge graph.

- **Phases 10-11** (Launch): MVP launch and CLI services. The
  PropertyVisualizer could extract metadata analytics. The REST API
  integration had full CRUD, authentication, batch operations, and
  caching.

- **Phase 12** (The Four Pillars): The breakthrough phase. An
  "Autonomous Learning Loop" implemented in 8,593 lines of TypeScript
  across five subsystems -- Perception (context gathering from
  experiences, vault notes, and external sources), Reasoning (multi-path
  plan generation with confidence scoring), Memory (experience storage
  with semantic search), Execution (monitored workflow execution with
  retries), and Reflection (pattern analysis, lesson extraction, meta-
  learning). This was the first system that could learn from its own
  history.

- **Phases 13-14** (Integration): Production hardening and Obsidian deep
  integration.

The key architectural idea that emerged was the **Neural Network
Junction** -- the Weaver service as a synapse where multiple AI "neural
networks" (Claude, local models, specialized agents) connect and share
knowledge through a common graph substrate. The name "Weave-NN" was
literal: a neural network of interconnected knowledge, woven together by
AI and humans.

What worked in Weave-NN:
- The knowledge graph as shared memory across AI sessions
- The learning loop (Perceive -> Reason -> Execute -> Reflect -> Learn)
- The compound learning effect (each task benefited from all previous tasks)
- The vault primitives (patterns, protocols, standards, schemas, guides)
  organized by research from Johnny Decimal, PARA, Zettelkasten, and
  CRUMBTRAIL

What did not work:
- TypeScript + Python + RabbitMQ + N8N + Obsidian was too many moving
  parts for a system that needed to run on constrained devices
- The "neural network junction" metaphor was aspirational -- the system
  had no actual causal reasoning, no cryptographic provenance, no way to
  prove what had happened
- The Obsidian dependency limited deployment to desktop environments
- There was no governance -- any agent could do anything
- The knowledge graph could be tampered with silently

The lessons from Weave-NN pointed toward something more fundamental than
a knowledge management tool.

### Phase 3: The CMVG Insight (2026-03-19)

While designing a WASM test rig for ClawStage's on-device vector search
at Mentra (the wearable AI glasses project), a test data generator
(`ConversationSimulator`) produced something unexpected: 384-dimensional
embeddings organized as nodes in a Bidirectional Temporal Merkle DAG with
typed causal edges. This was not a deliberate design. It was the simplest
structure that made the test harness work.

The recognition that followed became the CMVG concept note -- "Causal
Merkle Vector Graphs as a General-Purpose Cognitive Primitive." The
argument: three layers that are individually well-understood become
qualitatively different when combined:

```
SEMANTIC (HNSW vectors) -- "What is similar?"
    O(log n) approximate nearest neighbor search
    Cosine similarity, clustering, embedding space navigation

CAUSAL (DAG edges) -- "What caused what?"
    Typed, weighted, directed edges (Causes, Enables, Inhibits,
    Contradicts, Follows, Correlates, TriggeredBy, EvidenceFor)
    Laplacian spectral analysis, algebraic connectivity, community detection

PROVENANCE (Merkle chain) -- "Can I prove this happened?"
    BLAKE3 hash chains, Ed25519 signatures, HLC ordering
    Tamper-evident, offline-verifiable, append-only witness log
```

Vectors alone give similarity matching but cannot explain causation or
prove provenance. Causal graphs alone give reasoning but cannot do fast
approximate search or prove integrity. Merkle chains alone give tamper
evidence but cannot search semantically or express causal relations.

The computational profile mattered: incremental operations (HNSW insert,
causal edge insert, Merkle hash, HLC timestamp) total 3-10ms per tick on
a 1000-node graph with 384-dim vectors. Full spectral recomputation runs
in the background. This was lightweight enough for an ARM Cortex-A53 with
2GB RAM -- a Raspberry Pi, or Mentra glasses.

The CMVG concept note identified 8 application domains where the
primitive shows up: conversational AI, clinical decision support, supply
chain provenance, source code evolution, IoT sensor networks, legal
compliance, scientific research, and financial transaction monitoring.

This was the bridge from Weave-NN to WeftOS. The knowledge graph needed
to become a causal graph. The learning loop needed cryptographic
provenance. The neural network junction needed actual graph-theoretic
analysis.

### Phase 4: clawft + WeftOS (2026-02-28 to present)

The implementation began as a Rust rewrite. Not a port of Weave-NN but a
reimagining: what if the knowledge graph, the learning loop, the
governance model, and the mesh network were all built on the same CMVG
primitive?

The development followed a rigorous Generate-Analyze-Act cycle:

1. **SPARC plans** specified each kernel phase (Specification,
   Pseudocode, Architecture, Review, Completion)
2. **Implementation sprints** produced the Rust code
3. **Symposiums** (structured multi-panel reviews with 5-8 experts)
   evaluated what was built and decided what came next
4. **New SPARC plans** incorporated symposium decisions

This cycle executed at least 5 complete rotations over 27 days, producing
112 commits, 173,923 lines of Rust across 22 crates, 3,953 tests, and 65
symposium decisions.

The kernel grew in phases:
- **K0**: Boot sequence, configuration, daemon
- **K1-K2**: Process management, IPC, capability-based RBAC
- **K3**: WASM tool sandbox, ExoChain hash chain, governance engine
- **K3c**: ECC cognitive substrate (causal DAG, HNSW, cognitive tick)
- **K5**: Application framework, container integration
- **K6**: Mesh networking (Noise Protocol encryption, post-quantum
  ML-KEM-768, mDNS discovery, Kademlia DHT, SWIM heartbeats)

The Weaver -- the cognitive modeler that had been the central service in
Weave-NN -- became a kernel subsystem. At 4,957 lines with 122 tests, it
is the largest single module. It uses all ECC structures (CausalGraph,
HnswService, CognitiveTick, CrossRefStore, ImpulseQueue, ArtifactStore,
EmbeddingProvider) to analyze codebases, identify conversations, compute
confidence, and suggest improvements.

### Phase 5: The Weaver Analyzes Itself (2026-03-26)

On March 26, 2026, the Weaver performed its first real codebase analysis
-- on the clawft repository itself. It ingested:

- 29,029 session log nodes (from 4 sources: current development, clawft
  history, moltworker history, and subagent conversations)
- 112 git history nodes with 1,051 edges
- 215 module dependency nodes with 476 edges
- 92 decision nodes with 159 edges

The analysis identified 12 primary conversations happening in the
codebase, 7 meta-conversations (conversations about conversations), and 5
structural patterns including burst development rhythm, symposium-to-
implementation pipelines, and feature gates as conversation boundaries.

The Weaver's self-assessment trajectory: confidence started at 0.62 with
static file analysis, rose to 0.70 after graph ingestion, to 0.75 with
cross-graph queries, and reached 0.78 in the v2 post-sprint analysis.

This was the moment the system demonstrated what it was: software that
understands its own development history, can identify gaps in its own
knowledge, and can suggest how to improve itself.

---

## The Problem We Solve

The real problem is not "AI hallucinations" or "context windows." Those
are symptoms. The root problem:

**Software systems have no memory, no accountability, and no ability to
learn from their own history.**

Every deploy is amnesia. Every incident is investigated from scratch.
Every architecture decision is unlinked from the evidence that motivated
it.

### For Developers

Codebases are dead artifacts. They cannot tell you why a design decision
was made, who made it, what alternatives were considered, or what
evidence supported it. That context lives in Slack threads, lost PRs,
departed colleagues' heads, and meeting recordings nobody watches.

Technical debt is invisible until it explodes. The most critical code
paths often have the least test coverage. The modules that change
together most frequently (burst co-modification) are rarely documented as
coupled. File-level metrics miss structural relationships that only
emerge from graph analysis.

Testing is disconnected from risk. Test suites measure code coverage
(lines touched) rather than causal coverage (decision paths exercised).
A module with 100% line coverage can still have zero tests for the
decision that matters most.

### For Organizations

AI agents operate without governance. There are no constitutional limits
on what an agent can do, no separation of powers, no audit trail, and
no way to prove after the fact what happened. When something goes wrong,
the investigation starts from ephemeral chat logs.

Knowledge leaves when people leave. Tribal knowledge -- the unwritten
understanding of why systems work the way they do -- is not captured in
any machine-readable form. Onboarding new team members means months of
osmotic learning that could be compressed into graph traversal.

Compliance requires manual audit trails. In regulated industries
(healthcare, finance, defense), proving the chain of custody for every
decision is expensive, error-prone, and retroactive. It should be
built into the decision-making process, not bolted on after the fact.

### For the AI Industry

LLMs are stateless. Every conversation starts from zero. RAG is a
band-aid: vector similarity search retrieves what "looks like" the query,
not what "caused" the situation or "is evidence for" the claim.
Similarity matching is not understanding.

Multi-agent systems have no shared consciousness. Agents in frameworks
like LangGraph, CrewAI, and AutoGen cannot learn from each other's
experiences across sessions. There is no persistent cognitive substrate
that accumulates understanding over time.

Deployed AI has no accountability. When an agent makes a decision that
affects production, there is no cryptographic proof of what evidence was
available, what reasoning was applied, or what alternatives were
considered. The audit trail is a text log, not a tamper-evident chain.

---

## What WeftOS Actually Is

Not a chatbot framework. Not a vector database. Not a blockchain.

WeftOS is a **cognitive operating system** -- it gives any software
project:

1. **A brain** -- ECC causal graph + HNSW semantic memory. Not just
   "what is similar" but "what caused what" and "what does this predict."
   The same engine operates in three modes: Act (real-time conversation),
   Analyze (post-hoc corpus understanding), and Generate (goal-directed
   planning). 29,448+ nodes of real development conversation already
   ingested and analyzed.

2. **A conscience** -- Three-branch constitutional governance modeled on
   separation of powers. Legislative (what agents can do), Executive
   (runtime approval), Judicial (post-hoc audit). Every privileged
   operation evaluated against a 5-dimensional effect vector: risk,
   fairness, privacy, novelty, security. 22 constitutional rules
   chain-anchored as genesis events. Environment-scoped: dev is lenient,
   prod is strict.

3. **A memory** -- ExoChain append-only hash chain with SHAKE-256 hash
   linking, Ed25519 + ML-DSA-65 dual signatures, witness receipts, and
   lineage records. Every kernel action that matters becomes a chain
   event. The chain is not a log -- it is a tamper-evident, offline-
   verifiable, cryptographically-signed audit trail.

4. **A nervous system** -- Encrypted peer-to-peer mesh networking across
   heterogeneous devices. Noise Protocol XX/IK handshake patterns.
   Hybrid ML-KEM-768 + X25519 key exchange for post-quantum protection.
   Static seed peers, mDNS LAN discovery, Kademlia DHT for WAN. SWIM
   heartbeats with Alive/Suspect/Dead state machine. Cross-node IPC
   transparently bridges kernel messages across machines. Chain state
   replicates incrementally.

5. **A growth instinct** -- The Weaver. A cognitive modeler that uses all
   ECC structures to analyze, understand, and improve. It identifies
   conversations happening in a codebase, maps causal relationships
   between decisions and implementations, computes confidence scores,
   detects anomalies, and suggests what to work on next. The more it
   ingests, the more accurate it becomes.

---

## The Weave-NN Legacy

Everything in WeftOS traces back to something that was tried, proven, or
disproven in Weave-NN.

### What We Kept

**The learning loop.** Weave-NN's Phase 12 Autonomous Learning Loop
(Perceive -> Reason -> Execute -> Reflect -> Learn) became the conceptual
foundation for ECC's three operating modes. The five TypeScript
subsystems (PerceptionSystem, ReasoningSystem, MemorySystem,
ExecutionSystem, ReflectionSystem) were reimagined as graph operations
on the CMVG primitive.

**The knowledge graph as shared memory.** Weave-NN proved that AI agents
working on the same knowledge graph produce compound learning -- each
interaction improves the graph, and each subsequent agent benefits. This
validated the "neural network junction" hypothesis. In WeftOS, the ECC
causal graph is that shared substrate, with the addition of provenance
and spectral analysis.

**The vault primitives.** Weave-NN's research-backed organizational
scheme (patterns, protocols, standards, schemas, guides) informed the
WeaverKnowledgeBase structure. The hierarchy was validated by 14 phases
of actual use.

**The compound learning effect.** Weave-NN demonstrated that systems
which retain experience across sessions become measurably more effective.
The memory priming concept (+10% success rate from pre-task context
retrieval) was real. In WeftOS, this effect is structural: the causal
graph grows with every interaction, and HNSW search retrieves relevant
past decisions in sub-millisecond time.

### What We Dropped

**TypeScript + Python + RabbitMQ + N8N + Obsidian.** Five technologies
for a system that needed to run on a Raspberry Pi. Rust replaced all of
them. A single `weft` binary runs the entire stack: agent runtime, kernel,
cognitive substrate, mesh networking.

**The Obsidian dependency.** Tying the system to a specific desktop
application limited deployment to environments where Obsidian could run.
WeftOS targets 7 platforms: Linux, macOS, Windows, browser (WASM), WASI,
ARM64 edge, and cloud VMs.

**The REST API integration pattern.** Weave-NN communicated with the
knowledge graph through HTTP REST calls to the Obsidian Local REST API
plugin. WeftOS uses typed in-process IPC with 7 target types and 6
payload variants. No serialization overhead, no network hop, no API
server.

**The workflow engine dependency.** Weave-NN used N8N and later
workflow.dev for orchestration. WeftOS internalizes orchestration: the
kernel's supervisor, cron service, and application framework handle
lifecycle management natively.

### What We Transformed

**The "neural network junction" metaphor became the ECC cognitive
substrate.** What was a metaphor in Weave-NN (AI systems are like
neurons, the knowledge graph is like long-term potentiation) became
literal in WeftOS. The CausalGraph has typed edges (Causes, Enables,
Inhibits, Contradicts). The spectral analysis computes actual algebraic
connectivity. The HNSW index provides actual approximate nearest
neighbor search. The ImpulseQueue provides actual ephemeral signaling.
The metaphor became the mechanism.

**The four-pillar learning loop became the three-mode engine.** Weave-
NN's Perceive/Reason/Execute/Reflect cycle was sequential: one task at a
time. WeftOS's Act/Analyze/Generate modes run the same engine in
different configurations, and they compose into a continuous loop where
each mode feeds the next. The ExoChain links every transition with
cryptographic provenance.

**The Weaver service became a kernel module.** In Weave-NN, the Weaver
was an external service (a TypeScript process connecting via MCP to an
Obsidian vault). In WeftOS, the Weaver is `weaver.rs` -- 4,957 lines of
Rust running inside the kernel, with direct access to all ECC
structures. No network hop, no serialization, no external dependency.

---

## The Three-Layer Architecture

```
SEMANTIC (HNSW vectors)
  "What is similar to this?"
  Sub-millisecond approximate nearest neighbor search.
  Find the 10 most similar past decisions, code changes, or incidents.
  O(log n) with 384-dim or 768-dim embeddings.

CAUSAL (DAG edges)
  "What caused what?"
  8 typed edge kinds: Causes, Enables, Inhibits, Contradicts,
  Follows, Correlates, TriggeredBy, EvidenceFor.
  Laplacian spectral analysis reveals structural health.
  Lambda_2 (algebraic connectivity) measures conversation coherence.
  Community detection discovers natural conversation boundaries.
  Anomaly detection identifies high-risk structural patterns.

PROVENANCE (Merkle chain)
  "Can I prove this happened?"
  BLAKE3 hash chains with Ed25519 + ML-DSA-65 dual signatures.
  Witness receipts for cross-node verification.
  Lineage records linking chain events to causal edges to tree mutations.
  Offline-verifiable. Tamper-evident. Append-only.
```

No single layer is novel. The combination -- running on a $35 Raspberry
Pi with incremental operations completing in 3-10ms per cognitive tick --
is the contribution.

---

## Where It Applies

### Software Development (proven -- we built it this way)

This is not a hypothetical use case. The clawft repository itself is the
first project analyzed by the Weaver:

- 29,029 session log nodes from 4 sources ingested into the ECC graph
- 12 primary conversations identified in the codebase
- 7 meta-conversations (conversations about conversations) mapped
- 112 commits analyzed for causal structure (609 Correlates edges, 331
  Enables edges, 111 Follows edges)
- 59 architectural decisions tracked (43 implemented, 12 pending, 2
  blocked, 2 deferred)
- Conversation health scores computed (ranging from 1.00 for OS patterns
  to 0.46 for tool lifecycle)
- Critical anomaly detected: `agent_loop` (1,831 lines, 0 tests, 0
  incoming edges) -- the kernel's primary execution loop with no safety
  net

Every architectural decision is traceable to the symposium that produced
it, the evidence that motivated it, and the implementation that realized
it. The development process IS the demonstration.

### Regulated Industries (natural fit)

The three-layer architecture maps directly to regulatory requirements:

**Healthcare**: Clinical decision trails that satisfy 21 CFR Part 11.
Patient encounter nodes with causal edges (symptom -> diagnosis ->
treatment -> outcome). Tamper-evident Merkle chain for audit compliance.
HNSW search finds patients with similar causal trajectories, not just
similar symptoms.

**Finance**: AML decision provenance with cryptographic audit trail.
Transaction pattern embeddings in HNSW. Causal edges from alerts to
investigations to dispositions. The Merkle chain satisfies regulatory
requirements for tamper-evident record-keeping.

**Government/Defense**: Air-gapped mesh clusters with local Ollama
inference. Post-quantum ML-KEM-768 key exchange protects against
store-now-decrypt-later attacks. Three-branch governance enforces data
classification policies. ExoChain provides chain of custody.

**Legal**: Regulatory compliance tracking with causal chain of custody.
Lambda_2 of the regulation -> compliance action graph measures coverage
gaps: low connectivity means regulations without corresponding compliance
actions.

### IoT and Edge Computing (designed for it)

WeftOS was designed for constrained devices from the beginning. The CMVG
primitive was born on an ARM Cortex-A53. The boot-time calibration
benchmarks compute time per tick, measures p50/p95 latency, and auto-
adjusts the cognitive tick interval for the hardware.

- Runs on ARM64 (Raspberry Pi 4/5, Jetson, Mentra glasses)
- mDNS auto-discovery on LAN (zero configuration)
- Mesh networking across heterogeneous devices (a Pi and a cloud GPU
  server join the same cluster)
- WeaverKnowledgeBase shared across nodes
- Minimal build with zero networking dependencies for isolated edge nodes

### Enterprise AI Governance (the most urgent application)

Organizations deploying multi-agent AI systems have no governance
framework. WeftOS provides one:

- **Constitutional governance**: Three branches (Legislative, Executive,
  Judicial) with separation of powers
- **Effect vectors**: Every action scored on risk, fairness, privacy,
  novelty, security
- **Environment scoping**: Development is lenient, production is strict
- **Capability-based RBAC**: Per-agent permissions (IPC scope, tool
  permissions, sandbox policy, resource limits)
- **Chain-witnessed decisions**: Every governance evaluation recorded on
  the ExoChain
- **Cluster-wide consistency**: Governance rules synchronized across all
  mesh nodes

---

## How to Deploy

### For a Developer (30 seconds)

```bash
cargo install clawft-cli
cd my-project
weft agent
weft agent -m "Summarize this project"
```

### For a Team (5 minutes)

```bash
# Start the kernel with mesh networking
weave boot --mesh
weave cluster peers        # Discover other nodes on LAN
weave ecc status          # Check cognitive substrate health
weave chain status        # Verify ExoChain integrity
```

### For an Organization (1 hour)

Docker Compose with seed nodes, governance configuration, embedding
model, chain persistence, and monitoring. The `weftapp.toml` manifest
format packages agent deployments with capability wiring.

### For Air-Gapped / Classified (1 day)

Isolated cluster with no external network access. Local Ollama inference
(prompts never leave the network). Post-quantum encryption on all
internal mesh links. Governance policy derived from organizational
security requirements. ExoChain provides cryptographic audit trail for
compliance. Full spectral analysis support for structural health
monitoring.

---

## The Loop

The fundamental insight: **development IS a conversation. The
conversation IS the intelligence.**

The three modes compose because they share the same ExoChain witness
chain:

```
GENERATE (agents produce plans using the causal graph)
    |
    | ExoChain links generation to analysis
    v
ANALYZE (Weaver evaluates confidence, finds gaps, scores coherence)
    |
    | ExoChain links analysis to execution
    v
ACT (developers and agents implement changes in real-time)
    |
    | ExoChain links execution back to generation
    v
GENERATE (improved plans based on richer graph and new evidence)
    |
    ...continuous loop
```

Each transition is a causal edge in the CMVG. The analysis of a generated
plan is provably linked to the generation. The execution is provably
linked to the analysis. When execution reveals that a plan was wrong, the
causal graph shows exactly which generation-time decision led to the
problem and what evidence was available at the time.

This loop has already executed at least 5 complete rotations in the
development of clawft itself:
1. K0 SPARC plan -> K0-K2 implementation -> K2 symposium (22 decisions)
2. K2 symposium -> K2.1 implementation -> K3 symposium (14 decisions)
3. K3 symposium -> K3c planning -> ECC symposium (14 decisions)
4. ECC symposium -> K3c + K5 implementation -> K5 symposium (15 decisions)
5. K5 symposium -> K6 plan -> K6 implementation -> 09-gaps sprint

This is not a tool you use. It is a process you participate in. The more
you use it, the smarter it gets. The smarter it gets, the more useful it
becomes. The more useful it becomes, the more you use it.

---

## What Is Proven vs. Aspirational

### Proven (we built software this way)

- The Rust kernel compiles and passes 3,953 tests across all feature
  combinations
- The ECC causal graph ingests real data (29,029 session log nodes, 112
  git commits, 215 modules, 92 decisions)
- The Weaver identifies structural patterns, computes health scores,
  and detects anomalies in its own codebase
- The three-mode engine design (Act/Analyze/Generate) is architecturally
  complete and tested
- The mesh networking layer has types, traits, and logic for encrypted
  peer-to-peer communication with 224 tests
- The ExoChain produces tamper-evident hash chains with dual signing
- The governance engine evaluates effect vectors through three branches
- The system runs on ARM64 (Raspberry Pi, the development server for
  this project)
- 7 platform targets compile (Linux, macOS, Windows, browser WASM, WASI,
  ARM64, Docker)

### In Progress

- ONNX embedding backend exists as a stub awaiting a real model file
  (semantic search currently uses mock embeddings)
- Mesh networking Phase 2 (wire I/O: actual TCP/WebSocket connections)
  is pending -- Phase 1 (types + traits + logic) is complete
- Community detection has not yet been run to validate the 12 manually
  identified conversations
- Cross-project Weaver analysis (analyzing mentra and other projects
  alongside clawft) has not been attempted
- The Fumadocs documentation site (32 pages) has not been publicly
  deployed

### Aspirational

- External project adoption (no project outside weavelogic has used
  WeftOS yet)
- crates.io publication
- Real-world regulated industry deployment (healthcare, finance, defense)
- Federated mesh clusters across organizations
- Multi-language support via MCP bridge (TypeScript, Python, Go)
- The Weaver as the primary interface to a codebase (replacing IDE-based
  navigation with graph-based exploration)

---

## Differentiation

| Capability | WeftOS | RAG + Vector DB | Traditional DevTools | Agent Frameworks |
|---|---|---|---|---|
| Semantic search | HNSW ANN | HNSW/IVF | None | Sometimes |
| Causal reasoning | Typed DAG + spectral analysis | None | None | None |
| Cryptographic audit | ExoChain + ML-DSA-65 | None | Git (commits only) | None |
| Constitutional governance | 3-branch + effect vectors | None | Manual process | None |
| Self-evolving model | Weaver + 3-mode loop | Static embeddings | None | None |
| Platform targets | 7 (native + WASM + edge) | Cloud | Desktop | Cloud + CLI |
| Post-quantum encryption | ML-KEM-768 hybrid | TLS | SSH | None |
| Edge deployment | ARM64, 3-10ms/tick | Cloud-dependent | Local | Cloud-dependent |
| Mesh networking | P2P with auto-discovery | Client-server | None | None |
| Provenance | Tamper-evident chain | None | Git log | Text logs |

### Target Personas

1. **AI-first engineering teams** who deploy multi-agent systems and need
   governance, accountability, and shared memory across agents and
   sessions

2. **Regulated industry developers** who need cryptographic provenance,
   tamper-evident audit trails, and causal chain of custody for every
   decision an AI system makes

3. **Edge computing builders** who need intelligence on constrained
   devices (Raspberry Pi, Jetson, wearables) with offline capability,
   mesh networking, and automatic peer discovery

4. **Open source maintainers** who want their project to understand
   itself -- to track its own architectural decisions, identify structural
   health issues, and suggest where to focus effort next

---

## What Comes Next

### Near-term (Q2 2026)

- Real ONNX embedding model integration (semantic search goes live with
  actual vectors instead of mock embeddings)
- Community detection algorithm validates the 12-conversation model
  discovered by manual analysis
- Mesh networking Phase 2: wire I/O with actual TCP and WebSocket
  connections
- Cross-project Weaver: analyze mentra, clawstage, and weave-nn
  alongside clawft to test the primitive across domains
- Bulk-update 12 stale decision statuses identified by the Weaver

### Mid-term (Q3-Q4 2026)

- crates.io publication of `weftos` and `clawft-cli`
- Fumadocs site deployed publicly
- First external project adopts WeftOS
- Spectral analysis identifies structural health metrics that correlate
  with defect rates
- Weave-NN lessons formalized into WeaverKnowledgeBase entries (the
  learning loop research, the vault primitives scheme, the compound
  learning measurements)

### Long-term (2027)

- WeftOS as a standard cognitive layer for Rust projects
- Multi-language support (TypeScript, Python, Go via MCP bridge)
- Federated mesh clusters across organizations
- The Weaver becomes the primary interface to a codebase -- you ask it
  questions, it traverses the causal graph, it retrieves evidence, it
  computes confidence
- Integration with the original WeaveLogic vision: AI governance as a
  service for enterprises deploying multi-agent systems

---

## From Agency to OS

The WeaveLogic business plan asked: "How do we help businesses automate
their processes with AI?"

The Weave-NN knowledge graph asked: "How do we manage the knowledge that
AI produces?"

The CMVG insight asked: "What if the knowledge structure itself could
reason about causation and prove its own integrity?"

WeftOS answered: "Build the operating system."

The arc was not planned. An automation agency needed a knowledge graph.
The knowledge graph needed causal reasoning. Causal reasoning needed
cryptographic provenance. Provenance needed a kernel. The kernel needed
governance. Governance needed a mesh. The mesh needed a cognitive
substrate. The substrate needed a Weaver. The Weaver analyzed the system
that built it and found 12 conversations, 65 decisions, and a confidence
score of 0.78.

The next conversation will raise that score. And the one after that. And
the one after that.
