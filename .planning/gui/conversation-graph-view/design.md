# Conversation Graph View — Design

**Status**: Design (companion to ADR-067; no code in this doc)
**Surface**: `crates/clawft-gui-egui` — a new Explorer view
**Decision authority**: ADR-067. Model authority: ADR-062 (node/state/edge), ADR-046
(forest structures), M2 (`.planning/hermes-loop/m2-daemon-ecc-loop-design.md`).
**Grounding (file:line, current-as-of 2026-07-04, treat as landmarks):**
`explorer/viewers/graph.rs` (custom-painter node-link, rejected graph libs, lines
10–21) · `live/native_live.rs:518` (`poll_aux_rpcs`, the aux-RPC feed) ·
`context_graft_state.rs:14` (`NodeState`) · `causal.rs:36` (`CausalEdgeType`) ·
`crossref.rs:125` (`CrossRefType`) · `session_forest.rs:100` (`dual_write_turn`,
`emotion/goal` params) · `daemon.rs:5310` (`ecc.causal`, stats-only).

---

## 1. Screen layout

```
┌─ Explorer ─────────────────────────────────────────────────────────────────┐
│ [Chat] [Tree] [Terminal] [Graph ▸]                          conv: hermes-01 │
├─────────────────────────────────────────────────────────────────────────────┤
│ filters:  topic ◉  intent ◉  emotion ◉  goal ◉   edges: Follows ◉ Contra ◉ … │
├──────────────────────────────────────────────────────────────┬──────────────┤
│  time →                                            ⏵live      │  INSPECT     │
│                                                               │              │
│  lane: spec   ·····(ack)·····✕superseded                      │  node #4     │
│                    ╲                                          │  role: asst  │
│  lane: main   ◯user──▶◼asst──▶◯user──▶◻asst(frontier)         │  state:      │
│   #1   #2      \Follows   \Follows    \Follows                │   Committed  │
│                 └Speaker··○alice                              │  seq: 812    │
│  lane: fork          ⚡Contradicts──▶◼(barge-in)              │  ts: 12:04.3 │
│                                                               │  intent: Q   │
│      backchannel ticks:  ˙  ˙   ˙  (Continuer, no node)       │  topic: puyo │
│                                                               │  emotion:    │
│  ┌─ cluster: "puyo-strategy" ─────────┐                       │   +0.3/0.6   │
│  │  ◼ ◼ ◻                              │  (topic hull)        │  goal: help  │
│  └─────────────────────────────────────┘                     │  edges:      │
│                                                               │   ◀Follows#3 │
│  ├────────────────●────────────────────────────────┤  T      │   ▶Speaker   │
│  scrubber:  10:00        ▲drag             now                │              │
└──────────────────────────────────────────────────────────────┴──────────────┘
```

- **Top bar** — conversation selector + the standard Explorer tab strip; the graph
  is a peer tab of Chat/Tree/Terminal (`explorer/mod.rs` hosts tabs today).
- **Filter row** — classification + edge-type toggle chips (D4). Toggling dims, not
  removes.
- **Canvas** (left, majority) — the timeline graph. X = time, Y = lane (D3). A
  `⏵live` toggle sits top-right of the canvas.
- **Scrubber** (bottom of canvas) — the time slider; the ● handle is `T`, the sweep
  line is drawn vertically through the canvas at `T`.
- **Inspect panel** (right, collapsible) — selected-node detail; reuses the
  Explorer detail-pane idiom.

Narrow-width fallback: inspect panel becomes a modal/sheet
(`canon/sheet.rs`/`modal.rs` exist), canvas takes full width.

---

## 2. Interaction model

| Gesture | Effect |
|---|---|
| Drag scrubber ● | Set virtual `T`; rebuild `GraphModel` at `T` (§4). Leaves follow-live. |
| `⏵live` toggle | Pin `T = now`; resume live polling; frontier animates. |
| ← / → | Step `T` by one checkpoint (needs P1-graph checkpoint lineage; falls back to one commit). |
| Scroll on canvas | Zoom the **time axis** (X scale) about cursor. Node count unchanged. |
| Drag canvas | Pan the time window. |
| Click node | Select → populate Inspect panel; highlight its edges. |
| Hover node | Tooltip: role + truncated text + state. |
| Click filter chip | Toggle dim of that classification value / edge type. |
| Click cluster hull | Collapse/expand a topic cluster (LOD). |

**Follow-live vs scrub** is one state: `view_time: ViewTime { Live | Pinned(t_ms) }`.
`Live` ⇒ poll head each aux tick and set window to `[now-window, now]`. `Pinned(t)`
⇒ stop polling, rebuild from history at `t`. Switching to `Live` snaps to `now`.

---

## 3. Visual encoding

### 3.1 Node state → fill / opacity (primary signal)

| State | Encoding |
|---|---|
| `Speculative` | dashed outline, ~40% opacity fill (tentative) |
| `Frontier` | bright 2px accent outline, full fill — the live wavefront |
| `Committed` | solid fill, thin neutral outline |
| `Stale` | desaturated (‑60% chroma), 70% opacity |
| `Pruned` | hollow, diagonal strike, ~30% opacity tombstone |

### 3.2 Role → shape

| Role | Shape |
|---|---|
| user | circle `◯` |
| assistant | rounded square `◼` |
| tool-node (M4) | diamond (reserved; wired when M4 lands) |
| speaker-identity (crossref target) | small ring `○` off the main lane |

### 3.3 Classification → hue / badge / cluster

| Axis | Encoding |
|---|---|
| `topic` | node hue (stable hash → fixed semantic palette) + convex-hull cluster group |
| `emotion` | small VAD badge: valence→hue (red↔green), arousal→badge size, dominance→border |
| `intent` | corner glyph: `?` question, `!` request, `·` statement, `↺` correction |
| `goal` | thin colored underline keyed to the active goal thread |

### 3.4 Edge type → line style

| Edge / crossref | Style |
|---|---|
| `Follows` | solid 1.5px neutral, arrowhead (the main lineage) |
| `Contradicts` | red, zigzag, arrowhead (barge-in / repair) |
| `Enables` / `EvidenceFor` (M4 tools) | dashed, arrowhead |
| `Causes` / `TriggeredBy` | solid amber |
| `Speaker` (crossref) | faint dotted to the `○` speaker ring |
| `Continuer` (backchannel) | tick mark on the speaker node, **no edge line, no node** |

Palette generalizes the existing viewer's per-kind color hash (`graph.rs:409`) into a
fixed semantic table so meaning is stable across renders/sessions.

---

## 4. Data structures & update flow

### 4.1 In-memory model (GUI side)

```rust
// module: explorer/conversation_graph/model.rs
struct GraphModel {
    conv_id: String,
    nodes: Vec<GNode>,          // time-ordered
    edges: Vec<GEdge>,
    lanes: Vec<Lane>,           // computed by layout (§5)
    time_span: (f64, f64),      // min/max ts_ms across nodes
    view_time: ViewTime,        // Live | Pinned(t_ms)
    window: (f64, f64),         // visible X range (zoom/pan)
}
struct GNode {
    id: u64, chain_seq: Option<u64>, role: Role,
    text: String, state: NodeState, ts_ms: f64,
    class: Classification,      // intent/topic/emotion/goal
    lane: usize, x: f32, y: f32,
}
struct GEdge { source: u64, target: u64, kind: EdgeKind, weight: f32 }
struct Classification { intent: Option<String>, topic: Option<String>,
                        emotion: Option<Vad>, goal: Option<String> }
enum ViewTime { Live, Pinned(f64) }
```

`NodeState` is re-used from the kernel enum (`context_graft_state.rs`); the GUI does
not redefine it. Edge/crossref kinds map to `CausalEdgeType` + `CrossRefType`.

### 4.2 Feed → model (the two sources, one model)

```
              ┌───────────────── daemon ──────────────────┐
 CausalGraph ─┤  conversation.graph { conv_id, since?,     │  (P0 RPC — live head)
 CrossRefs   ─┤    window? }  →  { nodes[], edges[] }       │  (MUST filter by conv_id)
              │                                             │
 graph        ─┤  conversation.replay { conv_id, at?: T }    │  (P0+P1-graph — history)
 snapshot     │    → snapshot@chain_seq≤T                     │
 series(P1-g)─┘    (conv-scoped CausalGraphSnapshot + xrefs)  │
              └───────────────────┬───────────────────────┘
                                  │  aux-poller channel (native_live.rs pattern)
                                  ▼
   ViewTime::Live  ──▶ poll head each ~1Hz aux tick ──▶ merge into GraphModel
                                                         (append new, patch states)
   ViewTime::Pinned(T) ──▶ conversation.replay{at:T} : fetch snapshot @ chain_seq ≤ T;
                            animate-events = diff(snapshot[T], snapshot[T-1])
                            ──▶ rebuild GraphModel + play the diff
                                  │
                                  ▼
              layout(GraphModel) ──▶ paint(painter, GraphModel, T)
```

- **Live merge**: new nodes appended; existing nodes' `state`/`class` patched in
  place (cheap — a conversation is small). Frontier→Committed shows as an animated
  outline fade over ~2–3 frames.
- **Pinned rebuild (snapshot-diff, ADR-067 D2a/P1-graph)**: the scrubber's `T` is a
  `chain_seq`. State-at-T = the `conv_id`-scoped graph snapshot at `chain_seq ≤ T` — a
  `CausalGraphSnapshot` (`causal.rs:1672`, which already serializes `metadata.state`) +
  `CrossRefStore::to_snapshot`, persisted per commit. The animate-events =
  `diff(snapshot[T], snapshot[T-1])` → node-add/remove, edge-add/remove, and **state
  change (incl. prunes)**, captured for free because the snapshot records state-at-T
  (not mutation events). `chain_seq` is the version axis. NO `causal.node.state` chain
  event and NO RVF COW (recon addendum). This is the COW checkpoint-diff pattern over
  the graph's own serializer (ADR-067 D2a).
- **Degradation without P1-graph**: the scrubber shows the live `{nodes,edges}` head +
  the chain's already-durable add/remove events for topology; historical *state*
  transitions before the panel opened are not reconstructable and are omitted with a
  small "history-incomplete past this point" canvas hint (honest, signposted).

### 4.3 Classification source (kernel side, ADR-067 D5/P2)

Recorded at `SessionTier::index_turn` by a `TurnClassifier`: `emotion`/`goal` fill
the existing `dual_write_turn(emotion, goal)` params (`session_forest.rs:109`),
`intent`/`topic` land in the causal node metadata map. The RPC (P0) reads them back.
The GUI never classifies — it renders what the node carries.

---

## 5. Layout algorithm (timeline, deterministic)

Single forward pass, no force iteration (matches `graph.rs:337` "no force sim, no
state"):

1. **Sort** nodes by `ts_ms` (ties broken by `chain_seq`, then `id`).
2. **X position**: map `ts_ms` (or `chain_seq` rank for committed nodes) through the
   current `window` → canvas X. Scrubber sweep line is the X of `T`.
3. **Lane (Y) assignment**, walking sorted nodes:
   - main-line turns (linked by `Follows` on the trunk) → lane 0.
   - a `Speculative` node whose supersessor is a later `Committed` → lane +1 (above),
     drawn until its `Superseded`/prune time, then faded.
   - a **rollback / abandoned lineage** (a hermes-loop rollback, ADR-067 D2a) → its own
     branch lane, faded / struck to read as "abandoned but witnessed." The linear
     per-commit snapshot lineage shows the abandonment inline (a node that goes
     Stale/Pruned); a true *divergent* parallel lane needs the COW branch form
     (P1-brain), so v1 renders it inline and the full parallel-lane view arrives with
     P1-brain.
   - a `Contradicts` fork (barge-in) → next free fork lane.
   - `Speaker` crossref targets → a thin identity sub-lane below main.
   - `Continuer` (backchannel) → tick on the referenced node, **no lane**.
4. **Cluster hulls**: group nodes sharing a `topic` into a convex-hull overlay
   (drawn behind nodes); hull is advisory, does not change node positions.

Lane count is bounded by conversation branchiness (typically ≤4). Deterministic:
same `(nodes, edges, window)` ⇒ same layout, so live merges don't reflow jarringly.

---

## 6. Performance strategy

Conversation graphs are small (tens–low-hundreds of nodes), so the budget is
comfortable; the strategy is about keeping the **live path** cheap and the **paint**
bounded:

- **Conv-scoping is mandatory, at the source (recon).** The global `CausalGraph`
  accumulates every turn of every conversation for the daemon's whole lifetime and is
  **never pruned** — nodes are not removed on conversation end (only the ephemeral
  `SessionView` is dropped). The P0 RPC MUST filter by `conv_id` (via `metadata.conv_id`
  or `ConvForest`'s per-conv `seq_to_node`); the GUI never receives or renders the whole
  global graph. A long single conversation is hundreds of nodes (tractable); the global
  graph is unbounded (not).
- **Scrub uses a local buffer, not per-frame RPC (recon).** Poll cadence is 400 ms
  (selected path) – 1 s (slow tick) and there is no streaming/subscribe. Dragging the
  scrubber at speed must replay from a GUI-held buffer of the conversation's per-commit
  snapshots, re-RPCing only when `T` leaves the buffered range — never once per frame.
- **Poll cadence** rides the existing ~1 Hz aux tick for the live head — graph mutation
  is turn-paced; sub-second polling would be waste (per `native_live.rs` aux-throttle
  rationale).
- **Incremental merge** on live: patch node states in place, append new nodes; never
  rebuild the model on a live tick unless `conv_id` changes.
- **Culling**: nodes outside `window` are not painted; edges with both endpoints
  culled are skipped (the existing viewer already skips missing-endpoint edges,
  `graph.rs:132`).
- **LOD**: when visible node count exceeds a threshold (propose 60), collapse badges/
  labels to dots and elide crossref (dotted) edges; expand on zoom-in. Topic clusters
  collapse to a single hull glyph when zoomed out.
- **Layout memoization**: recompute layout only when `(nodes.len, window, filters)`
  change; cache `x,y` on `GNode` between frames.
- **Paint budget**: all drawing is immediate-mode painter calls (no retained scene);
  a few-hundred-node worst case is well within one egui frame. If a forest-wide view
  is ever added (deferred), that is where a real layout lib / spatial index earns its
  place — not here.
- **WASM**: no new deps, painter-only — the wasm target (VSCode panel) builds
  unchanged; classification/history data arrive as JSON over the same RPC the wasm
  live path already uses.

---

## 7. Module layout (respecting the 500-line rule)

```
explorer/conversation_graph/
  mod.rs        — view struct, tab wiring, ViewTime state          (~120)
  model.rs      — GraphModel, GNode/GEdge, feed-merge + rebuild     (~180)
  feed.rs       — conversation.graph RPC client + poll integration  (~120)
  layout.rs     — timeline lane assignment + X/Y mapping            (~150)
  paint.rs      — painter: nodes, edges, clusters, sweep line       (~220)
  encode.rs     — state/role/class/edge → color/shape/style table   (~120)
  interact.rs   — scrub, zoom/pan, select, filter state             (~150)
```

Each file stays under 500 lines. `paint.rs` is the largest risk; if it grows, split
node-paint from edge/cluster-paint. The `encode.rs` table is the single source of the
§3 visual mapping.

---

## 8. Open design questions (resolve during implementation)

- **`chain_seq` vs `ts_ms` for X.** Committed nodes have both; Frontier/Speculative
  have only `ts_ms`. Propose: X by `ts_ms` always (uniform), with `chain_seq` as the
  tie-break and the commit-order sanity check. Confirm the RPC surfaces `ts_ms` for
  all states.
- **`chain_seq` vs HLC for the snapshot key.** Snapshots are keyed by `chain_seq`; the
  axis should render in whatever order M2 stamps so scrub order == causal order. Align
  with M2's HLC decision.
- **Cluster stability across scrub.** Topic hulls should not jump as `T` moves;
  compute hulls from currently-visible nodes only, and accept that a hull grows as its
  cluster's nodes appear.
- **Supersession animation without P1-graph.** Until per-commit snapshots are persisted,
  the Speculative→Committed fade can only be shown live (when we witness the state
  patch), not replayed. Documented degradation (ADR-067 D2a/P1-graph).
- **Replay mechanism (recon addendum).** State-at-T is the per-commit `conv_id`-scoped
  graph SNAPSHOT at `chain_seq ≤ T`, NOT RVF/agenticow COW and NOT a `causal.node.state`
  chain event (the interim draft — retracted). `CausalGraphSnapshot` (`causal.rs:1672`)
  already serializes `metadata.state`, so no state-path kernel change; add only
  `CrossRefStore::to_snapshot` + daemon per-commit persist. The scrubber diffs adjacent
  snapshots; prunes captured for free (state-at-T, not events). RVF COW is the vector
  brain's separate concern (P1-brain).
- **Snapshot cadence & storage.** Persist per commit keyed by `chain_seq`; a conv-scoped
  snapshot is KBs. If long conversations make this large, store snapshots as
  content-addressed `ArtifactStore` blobs (`artifact_store.rs`) and/or thin older ones —
  confirm when P1-graph is built.
