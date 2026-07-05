# ADR-067: Conversation Graph View — timeline-scrubbed causal graph with classification-as-node-property in the egui GUI

**Date**: 2026-07-04
**Status**: Proposed
**Deciders**: GUI graph-view design pipeline (recon + design agents), 2026-07-04
**Depends-On**: **ADR-062** (ECC graph-walk conversation — the node/edge/state model this renders), **ADR-046** (Forest of Trees — the CausalGraph + CrossRef/Impulse structures walked), **ADR-047** (self-calibrating tick — the cadence the daemon-side loop mutates on), **ADR-058** (per-conversation context tier — the `SessionView` frontier this projects), `.planning/hermes-loop/m2-daemon-ecc-loop-design.md` (M2 — the committed-turn lifecycle this view visualizes)
**Relates-To**: ADR-061 (voice loop — the Speculative→Committed supersession this must render), ADR-017 (kernel-as-OntologyAdapter / Substrate — the GUI's data-feed substrate), ADR-057 (substrate read ACL — gates history replay), `.planning/ruv/integration/agenticow-integration-plan.md` (COW branching over `rvf_runtime::RvfStore` — the same checkpoint-diff *pattern* this ADR's replay model uses, but that plan's COW is for the **vector brain**; this view snapshots the **typed graph** over its own serialization instead — related by pattern, **separate effort**, not a dependency), the existing `ui://graph` primitive (`crates/clawft-gui-egui/src/explorer/viewers/graph.rs`)

## Context

WeftOS models a conversation as a walk over a causal graph (ADR-062): user and
assistant turns are `CausalNode`s with a five-state lifecycle
(`Speculative → Frontier → Committed → Stale → Pruned`, `context_graft_state.rs:14`),
linked by typed causal edges (`Causes, Inhibits, Correlates, Enables, Follows,
Contradicts, TriggeredBy, EvidenceFor` — `causal.rs:36`) and cross-structure
`CrossRef`s (`Speaker, EmotionCause, GoalMotivation, TomInference, Elaborates,
Continuer, …` — `crossref.rs:125`). M2 makes text a first-class modality of this
engine: a text turn now commits `Frontier → Committed` on the kernel-global forest
via a daemon-hosted `TalkModeLoop`.

That structure is **invisible today.** The GUI (`clawft-gui-egui`) surfaces the
conversation as a flat chat transcript (`explorer/chat.rs`) and exposes the ECC
substrate only as scalar counters (`ecc.status` → `{nodes, edges, crossref_count}`,
rendered in the Explorer's summary). There is no way to *see* the graph: which node
is Frontier vs Committed, how the reply `Follows` the prompt, when voice's
Speculative ack was superseded by the deep answer, or how classification (topic,
intent, emotion, goal) partitions the conversation.

This ADR records the decision to build a **conversation graph view**: the causal
graph rendered graphically, laid out along a **timeline**, with a **time scrubber**
to replay the conversation and watch the graph evolve (nodes appear, commit, prune,
supersede; edges get drawn), where **every node carries classification** that
manifests visually (typed edges, badges, colors, clusters).

Four facts from recon constrain the design:

1. **A graph-rendering primitive already exists and set the house idiom.**
   `explorer/viewers/graph.rs` is a read-only node-link diagram on egui's 2-D
   painter (`GraphViewer`, priority 14). It **deliberately rejected**
   `egui_node_graph` (editor-grade, egui-version-pinned) and `egui_graphs` (pulls
   `petgraph`, its own egui coupling) in favor of ~250 lines of painter code — zero
   new deps, no WASM bloat, JSON `{nodes, edges}` as the adapter seam (its module
   doc, lines 10–21). The conversation graph view is the same primitive at a higher
   altitude (timeline layout + state animation + classification encoding), not a new
   rendering stack.

2. **The GUI's data feed is the Substrate/OntologyAdapter poll model (ADR-017).**
   `live/native_live.rs` binds daemon RPCs into substrate paths on a 250 ms
   snapshot cadence; an aux poller already calls `ecc.status` at ~1 Hz
   (`poll_aux_rpcs`, `native_live.rs:518`) into `Snapshot::ecc_status`. New live
   graph data rides this exact channel.

3. **No RPC exports the graph.** `ecc.causal` returns only aggregate stats or, given
   a `node_id`, the *reachable node-id set* from `traverse_forward(id, depth)`
   (`daemon.rs:5310`). `ecc.crossrefs` returns a bare `count`. Neither yields the
   node list with labels/states/timestamps, the typed edge list, or the crossref
   list a graph view needs. **This is a prerequisite (P0 below).**

4. **Classification plumbing exists but is dormant, and no extractor fills it.**
   `session_forest::dual_write_turn` already accepts `emotion: Option<&str>` /
   `goal: Option<&str>` and writes `EmotionCause` / `GoalMotivation` crossrefs — but
   every call site passes `None` ("Phase 1 has no VAD/goal extractor",
   `session_forest.rs:154`). Separately, an **archetype/complexity classifier**
   exists for *tier routing only* (`context_router/llm_classifier.rs`,
   `ClassifierOutput { archetype, complexity }`) and never touches the graph. So the
   crossref *types* and the dual-write *parameters* exist; the *extractor* that
   produces classification and writes it as a durable node property does not.
   **This is a prerequisite (P2 below).**

## Decision

**Build the conversation graph view as a new egui Explorer surface that renders the
kernel-global causal graph for one conversation, laid out on a horizontal time axis,
driven by two data sources — a live-head RPC stream and a scrubbed-history replay —
with classification as a first-class node property surfaced through typed edges,
color/shape/badge encoding, and cluster grouping.** Concretely, seven positions:

### D1 — Rendering: extend the existing custom egui painter; do NOT adopt a graph library

The view is drawn with egui's 2-D `Painter`, the same technique as
`explorer/viewers/graph.rs`, promoted into a dedicated `conversation_graph`
module rather than reusing the generic `GraphViewer` (which is a stateless
substrate-value viewer with no timeline, no animation, no interaction state). Note
`explorer/viewers/graph.rs` is already **755 lines — over the 500-line ceiling** — so
the conversation view is a fresh multi-module surface (D-modules below), not an
extension of that file; it reuses the *technique* and the `{nodes, edges}` adapter
seam, not the file.

**Rationale.** The graph is small (a conversation is tens to low-hundreds of nodes,
not thousands), so a force-directed layout engine is unnecessary — the timeline
*is* the layout (D3). The painter approach already proved out in-repo, carries zero
new dependencies, builds to WASM cleanly (the GUI ships a wasm target — the VSCode
panel), and keeps the JSON `{nodes, edges}` adapter boundary as the migration seam
if a heavier lib is ever justified. `egui_plot` was considered for the timeline
axis/scrub scaffolding but rejected as the *primary* surface: it is built for
numeric series, not typed node-link graphs with per-node widgets/badges; we borrow
its axis/pan/zoom *interaction conventions* (D4) but paint the graph ourselves.
`egui_graphs`/`egui_node_graph` rejected for the reasons the existing viewer already
recorded (dep weight, egui-version pin, editor-grade overkill for a read-only view).
Recon confirms the raw materials are in-crate: **`egui_plot 0.35` is already a
dependency** (used by `canon/plot.rs`, `blocks/oscilloscope.rs`) for the time-axis
scaffolding, and **`ChainTailViewer` (`explorer/viewers/chain_tail.rs`)** — which
renders `[{seq, ts, kind, payload}]` newest-first with click-to-expand — is the model
for the scrubber's event track (it is literally a `causal.*` chain-event feed).

### D2 — Data feed: live head via RPC stream + scrubbed history via replay; both required

Two feeds behind one in-memory `GraphModel`:

- **Live head** — a new `conversation.graph` RPC (P0) polled through the existing
  substrate aux-poller channel (`native_live.rs` pattern) returns the *current*
  node/edge/crossref snapshot for a conversation. This drives **follow-live mode**:
  the frontier animates as turns arrive and commit. Poll cadence rides the existing
  ~1 Hz aux tick (graph mutation is turn-paced, ~seconds — sub-second polling is
  waste); a later increment can push deltas over a subscription if latency matters.
- **Scrubbed history** — reconstructing *graph-state-at-T* by **diffing adjacent
  per-commit snapshots of the conversation graph**, keyed by `chain_seq` (D2a). The
  scrubber (D4) sets a virtual time `T` (a `chain_seq`); the model is the snapshot at
  `T`, and the animated mutations are the set-diff `snapshot[T] − snapshot[T-1]`
  (node/edge add·remove, state transition, prune). This realizes the user's directive
  that **COW branching is the replay substrate** — the checkpoint-diff pattern applied
  to the graph's *own* JSON snapshot — and captures prune / stale / supersede, which
  leave no chain-event trace today, *for free*, because a snapshot records **state-at-T**
  (the `metadata.state` each node already carries) rather than mutation events.

The two feeds share one `GraphModel`; follow-live is "scrub pinned to `T = now`."

### D2a — Replay substrate: diff per-commit snapshots of the graph's own JSON serialization, keyed by `chain_seq` (chosen per directive)

**Chosen:** graph-state-at-T is a **per-commit snapshot of the conversation graph**
(`conv_id`-scoped), keyed by `chain_seq`; the mutations the view animates are the
**set-diff of adjacent snapshots**. This is the COW checkpoint-diff *pattern* the user
directed, applied over the graph's **own JSON serialization** — and recon's addendum
(deep evaluation of COW-as-replay-substrate) confirms it is the cleanest fit, revising
recon's own earlier chain-event suggestion. Two facts make it nearly free and *more
faithful than journaling events*:

1. **`CausalGraphSnapshot` already captures node state.** `CausalGraph::save_to_writer`
   → `CausalGraphSnapshot { next_node_id, nodes, forward_edges }` (`causal.rs:1672`)
   serializes each `CausalNode`'s `metadata`, and `set_node_state` (`causal.rs:208`)
   writes the `NodeState` tag into `metadata.state`. So **a snapshot records state-at-T
   — Frontier/Committed/Stale/Pruned — with zero new kernel work.** The scrubber diffs
   two snapshots to recover node-add, edge-add/remove, and *state change* directly.
2. **It captures prunes the chain drops, for free, with no state-path change.**
   `prune_to_recent` (`session_tier.rs:212`) flips state to `Stale` and emits **no**
   chain event; `set_node_state` emits none either. A snapshot doesn't need them —
   it reads the resulting `metadata.state`. This is *why the snapshot form is more
   faithful than a mutation-event journal*: it captures state-at-T robustly, including
   from any un-instrumented mutation path, and needs **no `causal.node.state` event**
   (the earlier draft's interim proposal — recon's addendum retracts it).

**Scope defuses the "unbounded graph" worry.** The global `CausalGraph` is unbounded
(nodes never removed — recon; see P0's mandatory scoping), so the *live query* must
scope by `conv_id`. But the
*snapshot* is likewise `conv_id`-scoped — one conversation is hundreds of small nodes,
so a JSON snapshot is KBs / sub-millisecond (recon). If per-conv snapshots ever grow
or need witnessing, store each as a content-addressed blob via the existing
`ArtifactStore` (`artifact_store.rs`, blake3 — already used by `context_graft` for
large chunks) — *not* RvfStore COW.

**Why NOT `rvf_runtime`/agenticow COW for the graph.** That COW primitive
(`RvfStore::branch()`, ~162 B/O(1)) is a **vector-store** mechanism; the conversation
graph is a **typed graph** (`CausalGraph` = `DashMap` DAG `causal.rs:124`;
`CrossRefStore` = forward/reverse `DashMap`s). RVF slab-COW does not model a typed
graph, its O(1)-branch advantage is moot at conversation scale (a JSON snapshot is
already sub-ms), and the brain is still on the `rvf_stub` (no COW until agenticow
Phase 0). **agenticow COW's real home is the vector brain — a separate effort**
(P1-brain), the checkpoint-diff *pattern* borrowed but not the RVF *substrate*.

**Branches and rollback.** `chain_seq` is the version axis, so linear time-travel is
native. Voice's Speculative→Committed self-branch shows as the state change on the
superseded node between snapshots. True *divergent branch lanes* (a rolled-back hermes
turn rendered as an abandoned parallel lineage) are where the COW branch/rollback form
earns its place — the P1-brain enhancement, not required for faithful state replay.

### D3 — Layout: time on X, causal/thread depth on Y (timeline layout, deterministic)

Nodes are positioned by a **layered timeline layout**, not force iteration:

- **X (horizontal) = time.** A node's X is its timestamp (or `chain_seq` rank for
  committed nodes) mapped to the visible time window. This makes the conversation
  read left-to-right and makes the scrubber a vertical sweep line.
- **Y (vertical) = thread / causal lane.** Turns on the main line share a lane;
  a `Speculative` self-branch (voice's fast ack) sits in a parallel lane above its
  `Committed` supersessor; barge-in `Contradicts` branches fork to their own lane;
  backchannel `Continuer` crossrefs attach as sub-lane ticks, never new lanes
  (ADR-062 D5 — a backchannel is never a turn node).

Layout is deterministic and stateless per frame given `(nodes, edges, time-window)`,
mirroring the existing viewer's "no force sim, no state" property (`graph.rs:337`).
Lane assignment is a single forward pass over nodes ordered by time.

### D4 — Interaction: scrub, zoom/pan, node-inspect, classification/edge filter, follow-live

- **Scrubber** — a horizontal time slider spanning the conversation; dragging sets
  `T` and rebuilds the model at that time (D2). A "⏵ live" toggle pins `T = now`
  (follow-live). Keyboard step (←/→) advances by one mutation event.
- **Zoom/pan** — borrow `egui_plot`'s conventions (scroll = zoom time axis, drag =
  pan) implemented on our painter; zoom changes the visible time window (X scale),
  not node count.
- **Node inspect** — click a node → a detail panel (reuse the Explorer detail-pane
  idiom) showing role, text, state, `chain_seq`, timestamp, full classification
  vector, and its edges/crossrefs.
- **Filter** — toggle chips to show/hide by classification (topic/intent/emotion/
  goal) and by edge type (`Follows`, `Contradicts`, `Enables`, `Speaker`, …).
  Filtering dims rather than removes (keeps layout stable).
- **LOD / culling** — nodes outside the visible time window are culled; when the
  window holds more nodes than a threshold, off-lane detail (labels, badges)
  collapses to dots (level-of-detail), edges to the culled region are elided.

### D5 — Classification is a first-class node property, computed at index time

**Every node carries a classification vector**, produced by a classification pass
that runs when the turn is indexed (at the M2 `SessionTier::index_turn` seam,
alongside the existing `dual_write_turn`), and persisted as **node metadata + typed
crossrefs** so both the RPC feed and history replay carry it.

- **Taxonomy** (v1): `intent` (question / request / statement / correction /
  chit-chat — reuse the existing archetype classifier's label as the seed),
  `topic` (short cluster tag), `emotion` (VAD `{valence, arousal, dominance}` for
  voice; a coarse text-sentiment label for text), `goal` (the active goal/task
  thread). These map onto the **already-defined** crossref types:
  `EmotionCause` (emotion), `GoalMotivation` (goal), and new node-metadata keys for
  `intent`/`topic`. `topic` additionally drives cluster grouping (D6).
- **Where it runs.** A `TurnClassifier` invoked inside `index_turn` before
  `dual_write_turn`, so its `emotion`/`goal` outputs fill the **dormant**
  `dual_write_turn(emotion, goal)` params (`session_forest.rs:109`) instead of the
  current `None, None`, and `intent`/`topic` land in the causal node metadata map.
  For text this is a cheap keyword/LLM-classifier pass (reuse
  `llm_classifier.rs`'s cheap-model round-trip, extended from `{archetype,
  complexity}` to the four-axis vector); for voice the emotion axis is the ECAPA/VAD
  signal ADR-061 already extracts.
- **Why at index time, not in the GUI.** Classification must be a *durable property
  of the witnessed node* (so history replay and any other consumer see the same
  labels, and so classification participates in the ADR-046 forest as real
  crossrefs), not a presentation-layer afterthought the GUI recomputes. The GUI only
  *renders* the classification the kernel recorded.

### D6 — Visual encoding: state → color/opacity, role → shape, classification → badge/hue/cluster, edge-type → line style

A single encoding table (fully specified in `design.md`); the load-bearing choices:

- **Node state** → fill + opacity: `Speculative` dashed/translucent, `Frontier`
  bright outline (the live wavefront), `Committed` solid, `Stale` desaturated,
  `Pruned` tombstone (hollow/struck). State is the primary visual signal — the whole
  point is watching the lifecycle.
- **Role** → shape: user vs assistant vs tool-node (M4) distinguished by node shape.
- **Classification** → the *topic* hue tints the node and groups a cluster hull;
  *emotion* shows as a small VAD badge; *intent*/*goal* as corner glyphs. This is
  how "classification manifests visually" per the feature ask.
- **Edge type** → line style: `Follows` solid thin, `Contradicts` red/zigzag,
  `Enables`/`EvidenceFor` (M4 tool edges) dashed, `Speaker`/`Continuer` crossrefs
  faint dotted. Reuses the existing viewer's per-kind color hashing (`graph.rs:409`)
  generalized to a fixed semantic palette.

### D7 — Coupling to M2: the view visualizes the committed-turn lifecycle, and is gated behind the same flag

The graph view is the **observability surface for M2's ECC-text loop.** It has no
value until turns actually commit on the kernel-global forest, which only happens
when `talk_loop=true` (M2 D7, `anchor_chain=anchor_causal=talk_loop=true`). The view
therefore:

- reads the same forest M2 commits onto (`ecc_causal` / `ecc_crossrefs`), so a
  Frontier→Committed transition the loop performs *is* the node the view animates;
- is **feature/config-gated** consistently with M2 — when the ECC forest is off, the
  view shows an explicit "ECC conversation loop not enabled" empty-state (the honest
  hint idiom the GUI already uses for absent adapters), never a fabricated graph;
- treats the M2 headline flow (user turn Frontier → tick commit → assistant reply
  `Committed`+`Follows`) as its first end-to-end acceptance scenario.

## Prerequisites (must exist before / alongside the view — each a concrete ask)

### P0 — Graph-export RPC (`conversation.graph`) — REQUIRED, blocks the view

`ecc.causal`/`ecc.crossrefs` are too thin (recon fact 3). Add a daemon RPC
`conversation.graph { conv_id, since?: chain_seq, window?: {from_ts, to_ts} }`
returning the existing `{nodes:[…], edges:[…]}` shape the GUI already consumes, where
each node carries `{id, chain_seq, conv_id, role, text, state, ts_ms,
classification:{intent, topic, emotion, goal}}` and each edge carries
`{source, target, kind, weight}`, plus the crossref edges (`Speaker`, `EmotionCause`,
`GoalMotivation`, `Continuer`). **Scoping by `conv_id` is mandatory, not optional**
(recon): the global `CausalGraph` accumulates every turn of every conversation for the
daemon's whole lifetime and is **never pruned** — shipping the whole graph is
untenable, so the handler must filter by `metadata.conv_id` (or walk `ConvForest`'s
per-conv `seq_to_node`). Recon confirms **no forest→`{nodes,edges}` serializer exists
anywhere today** — this RPC (or a published `substrate/_derived/chat/<conv>/graph`
path) is the required new surface. If it emits node `kind` = `NodeState` and edge
`kind` = `CausalEdgeType`/`CrossRefType`, the existing viewer's `kind_color` hashing
(`graph.rs:409`) lights up for free. Read projection over `CausalGraph` +
`CrossRefStore` — no new state, ~one handler + a serializer. Per recon's addendum the
RPC serves **both feeds**: the current `{nodes,edges}` for follow-live, and a
**per-`chain_seq` snapshot series** (`replay`) for the scrubber to diff (P1-graph).
**Kernel/daemon ask.**

### P1 — Snapshot lineage — split into P1-graph (required) and P1-brain (optional)

Per the user directive and D2a, the scrubber replays graph history by **diffing
per-commit `conv_id`-scoped snapshots** of the graph's own JSON serialization. Recon's
addendum confirmed the two halves of the forest have different mechanics and urgency,
so P1 is split:

**P1-graph — persist a per-commit graph snapshot [REQUIRED; standalone; small
kernel/daemon ask; NO agenticow dependency, NO state-path kernel change].** Recon's
addendum landed here after evaluating the alternatives: the change is (1) a small
`CrossRefStore::to_snapshot` (~20 lines; its `CrossRef`/`CrossRefType`/`UniversalNodeId`
already derive `Serialize`, mirroring `CausalGraph`'s existing `to_snapshot`), and (2)
the daemon persisting a `conv_id`-scoped snapshot **at each commit, keyed by
`chain_seq`**. The `CausalGraph` half needs **nothing new** — `CausalGraphSnapshot`
(`causal.rs:1672`) already serializes node `metadata`, and `set_node_state` already
writes `metadata.state`, so **the snapshot already captures Frontier/Committed/Stale/
Pruned state-at-T.** Graph-state-at-T = the snapshot at `chain_seq ≤ T`; the animated
mutations = `diff(snapshot[T], snapshot[T-1])` → node-add/remove, edge-add/remove, and
**state change (incl. prunes)** — the last of which the chain-event path can't see
without a kernel change but the snapshot gets for free. `SessionView` is deliberately
non-serializable (owns an HnswService), but its `NodeState` is already mirrored onto the
causal node via `mirror_state`, so the `CausalGraph` snapshot alone carries the state to
replay. **This unblocks full-fidelity G4 scrub** (all node/edge/state/prune events).
Snapshots are small (`conv_id`-scoped: hundreds of tiny nodes, KBs, sub-ms); if they
ever grow or need witnessing, store each as a content-addressed blob via `ArtifactStore`
(`artifact_store.rs`). *(This also subsumes M2 D8's deferred "reconciliation sweep on
loop restart" — a persisted per-commit graph snapshot is a durable transition record.)*

*Note — chosen over the `causal.node.state` chain event (an interim earlier draft):*
recon's addendum retracts that route. The snapshot needs **no** state-path
instrumentation (state is already in `metadata`), captures prunes robustly regardless of
which path mutated them, and is cheaper to build than journaling every transition. The
chain's existing `causal.node.add`/`edge.add`/`node.remove` events (`chain.rs:541/547/
553`) remain a redundant topology witness, not the replay source.

**P1-brain — brain-half COW branches [OPTIONAL enhancement; rides agenticow
Phase 0/1].** The 162 B / O(1) `RvfStore::branch()` primitive applies to the vector
brain, which the graph view does not render directly. When agenticow Phase 0 (brain
off the `rvf_stub` — no COW exists until then) and Phase 1 (`clawft-cow-memory`:
`checkpoint/rollback/branch/promote/diff/lineage`) land, the view gains the bonus layer:
**true divergent branch lanes** — a rolled-back hermes turn rendered as an abandoned
parallel lineage, speculative turns as visible branches (the one thing the linear
snapshot lineage does not render natively). Sequencing the view on this would block the
scrubber on a brain migration it does not need; instead P1-brain strengthens the
agenticow plan's case and plugs in when ready.

Until P1-graph lands, the scrubber degrades to append+commit replay (follow-live +
committed-turn history, which need only P0), and the "watch a node get pruned/superseded
at time T" animation is best-effort with an honest "history-incomplete" hint.

### P2 — Turn classifier wired at `index_turn` — REQUIRED for classification-as-node-property

The dormant `dual_write_turn(emotion, goal)` params and the `EmotionCause`/
`GoalMotivation` crossref types exist (recon fact 4) but nothing fills them, and no
`intent`/`topic` is recorded. The ask: a `TurnClassifier` run inside `index_turn`
(D5) that produces the four-axis vector and writes it (emotion/goal → the existing
crossref params; intent/topic → node metadata). Reuses the cheap-model classifier
(`llm_classifier.rs`) extended beyond `{archetype, complexity}`. **Service-agent ask;
independent of the GUI and shippable ahead of it.**

## Consequences

**Positive.** The ECC conversation model becomes *visible and debuggable* — the M2
loop's Frontier→Committed lifecycle, voice's speculative supersession, backchannel
handling, and classification clustering all become observable, which is a strong
development and demo asset. The custom-painter choice keeps the GUI dep-light and
WASM-clean. Classification-as-node-property (P2) is independently useful beyond the
view (recall, floor scoring per ADR-062 D4 uses emotion arousal). P0/P1 give the
kernel a real graph-export + audit-event surface other tools can reuse.

**Negative / risks.** P1-graph is modest but real kernel/daemon work, and the view's
"replay any state" promise is only as good as it; P1-brain's branch-lane layer is
gated on the agenticow plan's Phase 0/1 — shipping the view before P1-graph means an honest
degradation (append+commit replay via P0), which must be signposted, not hidden.
Checkpoint granularity bounds scrub resolution: the scrubber snaps to checkpoint
boundaries (turns/commits) and sub-checkpoint flutter (a node added *and* pruned
within one interval) shows only as net diff — acceptable for a conversation view,
where turns/commits are the meaningful stops (see Alternatives for the honest
granularity/diff-cost/memory analysis). Classification at index time (P2) adds
per-turn work on the commit path (a cheap classifier call); gate it so it does not
regress turn latency for deployments that don't want the view. The 500-line file rule
requires the view be split across several modules from the start (`model` / `layout`
/ `paint` / `interaction` / `feed`).

**Deferred (not v1).** Editable graph (the read-only stance matches the existing
viewer); multi-conversation / forest-wide view (v1 is one conversation); push-based
delta subscription (v1 polls); force-directed layout for very large graphs; M4
tool-node edges (`Enables`/`EvidenceFor`) rendering — designed for in the encoding
table, wired when M4 lands.

## Alternatives considered

- **Reuse the generic `GraphViewer` as-is** — rejected: it is a stateless
  substrate-value viewer with no timeline, scrub, animation, or interaction state;
  the conversation view needs all four. We keep its *technique* and adapter seam,
  not its surface.
- **Adopt `egui_graphs` / `egui_node_graph`** — rejected for the reasons the
  existing viewer already recorded: dependency weight, egui-version pin, WASM bloat,
  and editor-grade features irrelevant to a read-only view of a small graph.
- **Client-side classification (GUI classifies on render)** — rejected: makes
  classification a non-durable presentation artifact that history replay and other
  consumers can't see, and duplicates a pass the kernel should own (D5, P2).
- **Live-only, no durable history scrub** — rejected as the *primary* design: the
  feature's core is *watching the graph evolve*, and a session-buffer-only scrub can't
  show history before the panel opened. It is, however, the honest **degradation path**
  until P1-graph's one event lands (see P1). Recon lists it as viable strategy (a); we
  take it only as the interim fallback.

### Replay substrate — the three-way comparison (the load-bearing choice)

The user's directive names **COW branching** as the replay substrate; realized (D2a) as
the checkpoint-diff pattern over the graph's **own JSON snapshot**, `conv_id`-scoped and
keyed by `chain_seq`. Recon's addendum evaluated this against the two alternatives and
landed here — the snapshot form is *more faithful* (captures state-at-T directly) and
needs *no state-path kernel change*:

| Approach | Captures prune/stale/supersede? | New kernel work | Storage | Notes |
|---|---|---|---|---|
| **Per-commit snapshot-diff over `CausalGraphSnapshot` (chosen)** | Yes — snapshot records `metadata.state` at T; prunes for free | `CrossRefStore::to_snapshot` (~20 ln) + daemon per-conv snapshot persist; **no `set_node_state` change** (`CausalGraph` already serializes state) | `conv_id`-scoped snapshots (hundreds of tiny nodes, KBs, sub-ms); `ArtifactStore` blob if large | Realizes the COW checkpoint-diff *pattern* over the graph's own serializer; `chain_seq` version axis; robust to un-instrumented mutation paths |
| Chain-event fold (`causal.node.state`) | Yes — only once a new event is journaled | **Kernel change**: emit `causal.node.state` from every state-transition path | Tiny deltas on-chain | Recon's addendum **retracts** this: needs state-path instrumentation the snapshot avoids, and misses any transition that doesn't route through the instrumented call |
| Live-only, no durable replay | No (only a GUI session buffer) | None | None | The interim degradation path until P1-graph lands, not the design |

- **Chosen: per-commit snapshot-diff over `CausalGraphSnapshot`.** The serializer
  already captures node state (`metadata.state`, written by `set_node_state`
  `causal.rs:208`), so a `conv_id`-scoped snapshot per commit + a small
  `CrossRefStore::to_snapshot` gives graph-state-at-T with **no state-path kernel
  change**; the scrubber diffs adjacent snapshots for add/remove/state-change/**prune**
  events. This is *more faithful than journaling events* (captures state directly,
  robust to any mutation path) and directly realizes the user's COW checkpoint-diff
  directive over the graph's own structures — **not** `rvf_runtime` COW (vector-store
  mechanism, wrong for a typed graph, moot O(1) advantage at conversation scale, brain
  on `rvf_stub`). The "unbounded graph" concern applies to the live query (scope by
  `conv_id`) but not to per-conv snapshots, which are small.
- **Chain-event fold** rejected per recon's addendum: it needs a `causal.node.state`
  kernel change the snapshot form doesn't, and it captures *mutations* rather than
  *state-at-T*, so it misses any transition not routed through the instrumented path.
  The chain's existing add/remove events stay a redundant topology witness.
- **Divergent branch lanes** (a rolled-back hermes turn as an abandoned parallel
  lineage) are the one thing the linear snapshot lineage does not render natively —
  that visualization is the optional **P1-brain** layer, never required for faithful
  state replay.
