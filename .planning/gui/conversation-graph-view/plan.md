# Conversation Graph View — Implementation Plan

Companion to `design.md` and ADR-067. File paths absolute-from-repo-root. Line
anchors are current-as-of-2026-07-04; treat as landmarks. Estimates are net-added
lines + rough effort (S ≤ half-day, M ≈ 1–2 days, L ≈ 3–5 days).

---

## 1. Dependency graph (what blocks what)

```
        P2 (turn classifier, kernel-side)  ─┐
                                             ├─▶  P0 (graph-export RPC) ─┐
        M2 landed (turns commit on forest) ──┘                          │
                                                                        ▼
   P1-graph  per-commit graph snapshot (standalone) ········▶  GUI phases G1..G5
             (CausalGraphSnapshot already has state;          (optional for G1–G3;
              + CrossRefStore::to_snapshot; diff = replay;      P1-graph required for
              REQUIRED for full G4 scrub; no agenticow dep)     full G4 scrub)
   P1-brain  brain COW branches (optional enhancement)
     rides agenticow Ph0 (brain off stub) + Ph1
     (clawft-cow-memory); adds divergent branch/rollback lanes
```

- **P0 (RPC)** is the hard blocker for any GUI phase — nothing renders without it.
- **P2 (classifier)** should land with/just before P0 so P0 can surface real
  classification; P0 can ship first returning empty classification (view renders
  uncolored) if P2 slips.
- **P1-graph (per-commit graph snapshot)** blocks only the *faithful history scrub* of
  state transitions/prunes (G4); G1–G3 and live-follow (G4-live) do not need it. The
  `CausalGraph` half needs no state-path change (`CausalGraphSnapshot` already serializes
  `metadata.state`); add only `CrossRefStore::to_snapshot` + daemon per-commit persist
  keyed by `chain_seq` — **standalone, NO agenticow dependency** — so full G4 fidelity
  is NOT gated on the brain migration. **P1-brain** (agenticow Phase 0/1) is an optional
  later layer adding *divergent* brain branch/rollback lanes; it is a **shared
  dependency** with the agenticow plan, not a competitor.
- **M2** must be landed and enabled (`talk_loop=true`) for any real graph to exist
  (ADR-067 D7). Until then the view uses fixture data + shows the empty-state.

---

## 2. Kernel / daemon prerequisites (not GUI)

### P0 — `conversation.graph` RPC  [M, ~120 lines]
- **`crates/clawft-weave/src/daemon.rs`** — new RPC arm near the ECC block
  (`~5262`). Signature `conversation.graph { conv_id, since?: u64, window?: {from_ts,
  to_ts} }`. Reads `k.ecc_causal()` + `k.ecc_crossrefs()`, filters to `conv_id`,
  serializes `{nodes:[{id,chain_seq,conv_id,role,text,state,ts_ms,classification}],
  edges:[{source,target,kind,weight}]}` (the shape `graph.rs` already parses). Emit
  node `kind`=`NodeState` and edge `kind`=`CausalEdgeType`/`CrossRefType` so
  `graph.rs`'s `kind_color` (`graph.rs:409`) lights up for free.
- **`conv_id` filtering is MANDATORY, not a nicety (recon)** — the global `CausalGraph`
  is unbounded (nodes never removed — §6 Risks); shipping the whole graph is
  untenable. No forest→`{nodes,edges}` serializer exists today; this handler is that
  new surface.
- **`crates/clawft-kernel/src/causal.rs`** — add a `nodes_for_conv(conv_id)` +
  edge-listing helper if not present (`traverse_forward` returns ids only; need the
  full node payloads incl. `state` metadata). ~40 lines.
- **`crates/clawft-kernel/src/crossref.rs`** — add a `by_conv`/listing accessor
  (currently only `count()` is exposed via RPC). ~25 lines.
- **Test**: daemon unit — build a small forest with 2 committed turns + a Follows
  edge + a Speaker crossref, call the handler, assert node/edge/crossref JSON shape
  and `conv_id` scoping.

### P2 — Turn classifier at `index_turn`  [M, ~90 lines]
- **`crates/clawft-service-agent/src/session_tier.rs`** — in `index_turn`, before
  `dual_write_turn`, run a `TurnClassifier::classify(text) -> Classification`; pass
  its `emotion`/`goal` into the existing `dual_write_turn(…, emotion, goal)` params
  (`session_forest.rs:109`, currently `None,None`); stash `intent`/`topic` in the
  causal node metadata map.
- **`crates/clawft-core/src/agent/context_router/llm_classifier.rs`** — extend
  `ClassifierOutput { archetype, complexity }` → add `intent, topic, emotion, goal`
  (or add a sibling `classify_turn`); reuse the cheap-model round-trip. Keep the
  routing path untouched (it reads `archetype/complexity` only). ~40 lines.
- **Config gate**: only run when the ECC forest / `talk_loop` path is on, so
  deployments not using the view pay nothing (ADR-067 consequence).
- **Test**: `index_turn` with a mock classifier writes `EmotionCause`/
  `GoalMotivation` crossrefs and `intent/topic` node metadata; routing path
  unaffected (existing `llm_classifier` tests stay green).

### P1-graph — Per-commit graph snapshot  [S–M, standalone]  *(REQUIRED for full G4; no agenticow dep)*
Per ADR-067 D2a/P1 and recon's addendum, replay is the DIFF of per-commit `conv_id`-scoped
graph snapshots — the COW checkpoint-diff pattern over the graph's own JSON serializer.
No `causal.node.state` chain event, no RVF COW:
- **`CausalGraph` half needs nothing new**: `CausalGraphSnapshot` (`causal.rs:1672`,
  `save_to_writer`/`to_snapshot`) already serializes each `CausalNode`'s `metadata`, and
  `set_node_state` (`causal.rs:208`) writes the `NodeState` tag into `metadata.state`. So
  a snapshot already captures Frontier/Committed/Stale/Pruned state-at-T.
- **`CrossRefStore::to_snapshot`** (~20 lines): forward/reverse `DashMap`s of
  `CrossRef`s (which already derive `Serialize`), mirroring `CausalGraph::to_snapshot`.
- **Daemon persist**: at each commit, persist the `conv_id`-scoped
  `{CausalGraphSnapshot, cross-refs}` keyed by `chain_seq`. Snapshots are KBs (conv-scoped:
  hundreds of tiny nodes); if large, store as `ArtifactStore` (`artifact_store.rs`,
  blake3) blobs. `SessionView` is non-serializable but its `NodeState` is already mirrored
  onto the causal node (`mirror_state`), so the graph snapshot alone carries the state.
- **Replay**: `graph-state-at-T` = the snapshot at `chain_seq ≤ T`; animate-events =
  `diff(snapshot[T], snapshot[T-1])` → node/edge add·remove + state change (incl. prunes,
  captured for free as state-at-T).
- **P0 extension**: `conversation.replay { conv_id, at: ts }` returns the snapshot ≤ `ts`
  (and optionally the prior snapshot for the client to diff).
- **Test**: drive a turn sequence (add/commit/prune/supersede), snapshot per commit, diff
  at several `T`, assert reconstructed state + diff match known state — including the
  prune (Stale) that emits no chain event.
- **Note**: subsumes M2 D8's deferred "reconciliation sweep on restart" — a persisted
  per-commit snapshot is a durable transition record. Chosen over a `causal.node.state`
  chain event (interim draft, retracted by recon's addendum): the snapshot needs no
  state-path instrumentation and captures prunes robustly. `exo-resource-tree::to_checkpoint`
  (`boot.rs:69/81`) is the in-repo checkpoint precedent.

### P1-brain — Brain COW branch lanes  [L, optional enhancement; rides agenticow Ph0/Ph1]
The 162 B / O(1) `RvfStore::branch()` primitive applies to the vector brain, which
the view does not render directly. When agenticow Phase 0 (brain off `rvf_stub` →
real `RvfStore`, `Cargo.toml:208`) and Phase 1 (`clawft-cow-memory`:
`checkpoint/rollback/branch/promote/diff/lineage`) land, extend the P1-graph
checkpoint into the full **forest envelope** (graph snapshot + brain COW branch +
chain marker, coupled to `ChainManager::checkpoint`/`record_lineage` per agenticow §7
DualStateBridge; rollback discards the COW child + appends a witnessed `TurnReverted`,
never truncates the chain). The view then gains brain-state branch lanes: a
rolled-back hermes turn renders as an abandoned COW child, speculative turns as
visible branches. **Shared dependency** with the agenticow plan — strengthens its
Phase 0/1 case; must NOT gate G4 scrub fidelity, which P1-graph alone delivers.

---

## 3. GUI phases

### G1 — View scaffold + static render from fixture  [M]
- **NEW** `crates/clawft-gui-egui/src/explorer/conversation_graph/{mod,model,encode,
  paint,layout}.rs` (§7 of design). Register as an Explorer tab
  (`explorer/mod.rs`, alongside Chat/Tree/Terminal).
- Render a hardcoded fixture `GraphModel` (2 user + 2 assistant turns, one Follows
  chain, one Speaker crossref) with timeline layout (`layout.rs`) and the encoding
  table (`encode.rs`, §3). No feed, no scrub yet.
- **Depends on**: nothing (fixture). Ships the visual language for review early.
- **Test**: `layout` unit (nodes land in-window, main-lane ordering by ts);
  `encode` unit (state→style mapping table); reuse `graph.rs` test patterns.

### G2 — Live feed (follow-live)  [M]
- **`feed.rs`** — `conversation.graph` client over the aux-poller channel
  (`native_live.rs` `poll_aux_rpcs` pattern, `native_live.rs:518`); add a
  `Snapshot::conversation_graph` field (mirrors `ecc_status`).
- **`model.rs`** — incremental merge (append new, patch state/class in place).
- `ViewTime::Live` path; frontier outline animation on state patch.
- **Depends on**: P0. (P2 optional — renders uncolored if classification empty.)
- **Test**: merge unit — apply two successive head snapshots, assert append + in-place
  state patch (no duplicate nodes, Frontier→Committed reflected).

### G3 — Interaction: zoom/pan, node inspect, filters  [M]
- **`interact.rs`** — scroll-zoom time axis, drag-pan, click-select, hover-tooltip;
  filter chips (dim by classification/edge type).
- **Inspect panel** — reuse Explorer detail-pane; show node fields + edges.
- **Depends on**: G1 (+ G2 for live data). Independent of P1-graph.
- **Test**: interaction-state unit (selection, filter toggles, window pan clamps).

### G4 — Time scrubber + history replay  [M with P1-graph / S degraded without]
- **`interact.rs`/`feed.rs`** — scrubber slider → `ViewTime::Pinned(T)`; on pin,
  request `conversation.graph{window:{to_ts:T}}` (append+commit replay) or, when P1-graph
  exists, `conversation.replay{at:T}` (snapshot ≤ T; diff adjacent). Sweep line
  at `T`. ←/→ steps one checkpoint.
- **Degradation without P1-graph**: render append+commit faithfully; show the
  "history-incomplete past this point" canvas hint for prune/supersede (ADR-067 P1-graph).
- **Depends on**: P0 (degraded) / P0+P1-graph (full).
- **Test**: pinned-rebuild unit — open fixture checkpoint ≤ `T` + apply diff → model
  matches expected node set/states; live→pinned→live transitions restore correctly.

### G5 — Classification visual polish + clusters  [S]
- **`paint.rs`/`encode.rs`** — topic-hull cluster overlay, emotion VAD badge, intent
  glyph, goal underline (§3.3). LOD collapse of badges/clusters on zoom-out.
- **Depends on**: P2 (real classification) + G1–G3.
- **Test**: cluster-hull unit (nodes sharing topic grouped; hull recomputed on
  visible-set change); LOD threshold unit.

---

## 4. Suggested sequencing & ownership

Two independent tracks converge at G2:

- **Kernel track** (`kernel-specialist` + `coder`): P2 → P0 → P1-graph (P1-brain deferred, rides agenticow).
- **GUI track** (`coder` with GUI idioms): G1 (fixture, needs nothing) in parallel
  with the kernel track; G2 once P0 lands; G3; G4 (degraded) once P0, upgraded when
  P1-graph lands; G5 once P2 + G3.
- **Reviewer** gates each phase; **tester** owns the per-phase unit tests + a
  GUI-level smoke (fixture render, live-merge, pinned-rebuild).

Critical path to a *usable live view*: **M2 enabled → P0 → G1 → G2 → G3.**
Critical path to *full-fidelity history scrub* (all node/edge/state/prune/supersede
events): **+ P1-graph only** — no agenticow / brain-migration dependency. Rich
classification (G5) needs P2. P1-brain (brain rollback/branch lanes) is a later,
optional layer that rides agenticow and never gates G4.

Rough total: P0 (M) + P2 (M) + G1–G3 (3×M) + G4 (M) + G5 (S) ≈ 2–3 weeks of focused
work with P1-graph (M) sequenced after P0 and P1-brain (L) deferred onto agenticow.

---

## 5. Test plan (consolidated)

**Kernel/daemon**
- P0 handler: node/edge/crossref JSON shape + `conv_id` scoping + `since`/`window`
  filtering.
- P2: classifier writes emotion/goal crossrefs + intent/topic metadata; routing path
  unchanged.
- P1-graph: replay-at-T reconstructs known state for add/commit/prune/supersede.

**GUI (unit, headless where possible per `graph.rs` test style)**
- `layout`: timeline ordering, lane assignment (main / spec / fork / speaker),
  in-window containment.
- `encode`: exhaustive state/role/edge → style table.
- `model` merge: append + in-place state patch, no duplicates.
- `model` pinned rebuild: open fixture checkpoint ≤ `T` + diff → matches known state at multiple `T`.
- `interact`: selection, filter dim, pan/zoom clamps.
- `cluster`/LOD: topic grouping + collapse thresholds.

**Integration / regression**
- End-to-end against a live daemon with `talk_loop=true` (the M2 dev config): drive
  the M2 headline flow (user Frontier → tick commit → assistant `Committed`+`Follows`)
  and assert the view shows the two nodes, the Follows edge, and the state transition
  on follow-live.
- `scripts/build.sh test` (workspace) + `scripts/build.sh gate` before any commit.
- Feature matrix: build the GUI wasm target (VSCode panel) — painter-only, no new
  deps, must stay green; `native` + `voice` feature builds.

---

## 6. Risks & mitigations

| Risk | Mitigation |
|---|---|
| P1-graph (per-commit snapshot) slips | G1–G3 + live-follow don't need it; G4 ships degraded (live head + chain add/remove for topology; historical state not replayable) with an honest hint (ADR-067 P1-graph). It's `CrossRefStore::to_snapshot` + a persist loop (no agenticow dep), so this risk is small. |
| Global `CausalGraph` unbounded (nodes never removed) | P0 filters by `conv_id` at the source (mandatory); GUI never holds the global graph. Long single conversation ≈ hundreds of nodes (tractable). |
| Scrub re-RPCs per frame | GUI holds a local `causal.*` event buffer; re-RPC only when `T` leaves the buffered range (poll cadence is 400 ms–1 s, no streaming). |
| Classification adds turn latency | Gate P2 behind the ECC/`talk_loop` path; cheap keyword classifier default, LLM classifier opt-in. |
| `paint.rs` exceeds 500 lines | Pre-split node-paint vs edge/cluster-paint (design §7 flags it). |
| Layout reflow jarring on live merge | Deterministic layout (design §5) + memoized `x,y` — same input ⇒ same layout. |
| No real graph until M2 enabled | G1 fixture path + explicit empty-state when ECC forest off (ADR-067 D7). |
| RPC payload large for long convs | `since`/`window` params bound the payload; LOD/culling GUI-side. |
