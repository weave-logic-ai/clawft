# ADR-067 G1–G5 GUI phases — phase plan & scaffold status

**Ticket:** WEFT-630 (umbrella)  
**ADR:** [`docs/adr/adr-067-conversation-graph-view.md`](../adr/adr-067-conversation-graph-view.md)  
**Design:** [`.planning/gui/conversation-graph-view/design.md`](../../.planning/gui/conversation-graph-view/design.md)  
**Implementation plan (source):** [`.planning/gui/conversation-graph-view/plan.md`](../../.planning/gui/conversation-graph-view/plan.md)  
**Surface:** `crates/clawft-gui-egui/src/explorer/conversation_graph/`  
**Cycle:** 0.8.x · **Label:** `ws08-weftos-gui`  
**Date:** 2026-07-31

This is the **umbrella phase plan** for the conversation-graph GUI surface
(ADR-067 D1–D7). Kernel prerequisites (P0/P1/P2) are tracked separately; this
doc scopes **GUI phases G1–G5**, records scaffold status, and defines child
items to file when each phase is claimed.

---

## 1. Status summary (scaffold)

| Phase | Name | Status | Child ticket | Module(s) | Depends on |
|-------|------|--------|--------------|-----------|------------|
| **G1** | View scaffold + static fixture render | **Scaffolded** | *file on claim* | `mod`, `model`, `encode`, `layout`, `paint` | none (fixture) |
| **G2** | Live feed (follow-live) | **Scaffolded** | *file on claim* | `feed`, `model` merge | P0 `conversation.graph` (landed `a16a5701`) |
| **G3** | Interaction: zoom/pan, inspect, filters | **Scaffolded** | *file on claim* | `interact` | G1 (+ G2 for live data) |
| **G4** | Time scrubber + history replay | **Scaffolded** | *file on claim* | `interact`, `feed` | P0 (degraded) / P0+P1-graph (full) |
| **G5** | Classification visual polish + clusters | **Scaffolded** | *file on claim* | `paint`, `encode` | P2 + G1–G3 |

**Status legend**

| Status | Meaning |
|--------|---------|
| Not started | No module files / no phase plan |
| **Scaffolded** | Module tree + types + empty-state / fixture hooks compile; behaviour incomplete |
| In progress | Implementation PR open |
| Done | AC met; build + a11y notes + tests per ws08 |

Scaffold code entry: `crates/clawft-gui-egui/src/explorer/conversation_graph/`.  
Phase status is also queryable at runtime via
`conversation_graph::phase_status()` (see `mod.rs`).

---

## 2. Prerequisites (kernel / daemon — not G1–G5 work)

| ID | What | Blocks | State (as of this doc) |
|----|------|--------|------------------------|
| **P0** | `conversation.graph` RPC | G2+ live | **Landed** (`a16a5701`) |
| **P1-graph** | Per-commit graph snapshot + `conversation.replay` | Full G4 scrub | Open (separate WEFT) |
| **P1-brain** | Brain COW branch lanes (agenticow) | Optional G4 branch viz | Deferred |
| **P2** | Turn classifier at `index_turn` | G5 real classification | Partial (keyword path exists; polish open) |
| **M2** | `talk_loop=true` commits on forest | Real (non-fixture) graph | Product path |

G1 ships against a **fixture** and does not need P0 at runtime. G2+ need P0
for live data; until M2 is enabled the view shows the honest empty-state
(ADR-067 D7).

---

## 3. Phase scopes (child items)

When a phase is claimed, create a Plane child (or sibling) under 0.8.x with
labels `ws08-weftos-gui` + `gap`, parent comment linking WEFT-630, and the AC
below. Do **not** bulk-create all five until work starts — umbrella rule:
scope when started.

### G1 — View scaffold + static render from fixture

**Goal:** Explorer-hosted conversation graph surface that paints a hardcoded
fixture so the visual language can be reviewed without a daemon.

**Acceptance criteria**

- [ ] Module tree under `explorer/conversation_graph/` (mod/model/encode/layout/paint)
- [ ] Registered as Explorer peer of Chat/Tree/Terminal (or explicit mount path)
- [ ] Fixture: ≥2 user + ≥2 assistant turns, one `Follows` chain, one `Speaker` crossref
- [ ] Timeline layout (X=time, Y=lane) + encode table (state → style)
- [ ] Empty-state string when ECC / talk_loop off (fixture mode still available)
- [ ] Unit tests: layout ordering; encode state→style exhaustiveness
- [ ] `scripts/build.sh check` green for `clawft-gui-egui` (native + wasm feature matrix notes)
- [ ] A11y: keyboard focus order documented for tab; high-contrast state colors (no color-only meaning)

### G2 — Live feed (follow-live)

**Goal:** Poll `conversation.graph` via aux channel; merge into `GraphModel`.

**Acceptance criteria**

- [ ] `feed.rs` client over aux-poller pattern (`native_live` / Live command)
- [ ] Incremental merge: append new nodes, patch state/class in place (no dupes)
- [ ] `ViewTime::Live` path; frontier outline updates on state patch
- [ ] Degrades to fixture/empty when RPC unavailable
- [ ] Unit: two successive head snapshots → merge invariants
- [ ] Build + a11y: live toggle focusable + announced state

### G3 — Interaction: zoom/pan, node inspect, filters

**Goal:** Make the canvas usable for inspection.

**Acceptance criteria**

- [ ] Scroll-zoom time axis; drag-pan; click-select; hover tooltip
- [ ] Filter chips dim by classification / edge type (layout stable)
- [ ] Inspect panel: role, text, state, `chain_seq`, classification, edges
- [ ] Unit: selection, filter toggles, pan clamps
- [ ] A11y: filters as toggle buttons with names; inspect panel keyboard reachable

### G4 — Time scrubber + history replay

**Goal:** Scrub `T` and rebuild graph-at-T.

**Acceptance criteria**

- [ ] Scrubber slider → `ViewTime::Pinned(T)`; sweep line at `T`
- [ ] ←/→ steps one checkpoint
- [ ] Degraded without P1-graph: append+commit replay + honest “history-incomplete” hint
- [ ] Full: `conversation.replay` / snapshot-diff when P1-graph available
- [ ] Unit: pinned rebuild; live→pinned→live restore
- [ ] A11y: scrubber as slider with value text; step keys documented

### G5 — Classification visual polish + clusters

**Goal:** Classification-as-node-property visible (topic hulls, emotion badge, intent glyph).

**Acceptance criteria**

- [ ] Topic-hull cluster overlay; emotion VAD badge; intent/goal glyphs
- [ ] LOD: badges/clusters collapse on zoom-out
- [ ] Unit: cluster grouping; LOD thresholds
- [ ] Requires real P2 classification for non-fixture data (fixture can fake vectors)
- [ ] A11y: classification not color-only (glyphs + tooltips)

**ws08 conventions (every phase):** `scripts/build.sh` for check/test; keep files
under 500 lines; no new graph library deps (ADR-067 D1); WASM panel stays green.

---

## 4. Sequencing

```
Kernel:  P2 ──┐
              ├─▶ P0 (done) ──▶ P1-graph (for full G4)
M2 enabled ───┘

GUI:     G1 (fixture) ──▶ G2 (live) ──▶ G3 (interact) ──▶ G4 (scrub)
                                                          │
                                     G5 ◀── P2 + G1–G3 ───┘
```

**Critical path to usable live view:** M2 enabled → P0 → G1 → G2 → G3.  
**Full scrub:** + P1-graph only (no agenticow).  
**Rich classification:** P2 + G5.

Rough effort (from planning plan): G1–G3 ≈ 3×M, G4 ≈ M, G5 ≈ S (2–3 weeks focused
with P0 already landed).

---

## 5. Module layout (scaffold)

```
crates/clawft-gui-egui/src/explorer/conversation_graph/
  mod.rs        — view shell, ViewTime, phase_status(), empty-state   (G1 shell)
  model.rs      — GraphModel, GNode/GEdge, fixture, merge hooks       (G1/G2)
  encode.rs     — state/role/edge → style table                       (G1/G5)
  layout.rs     — timeline X/Y + lane assignment                      (G1)
  paint.rs      — painter nodes/edges/clusters/sweep                  (G1/G5)
  feed.rs       — conversation.graph client + poll merge              (G2/G4)
  interact.rs   — scrub, zoom/pan, select, filters                    (G3/G4)
```

Registered from `explorer/mod.rs` as `pub mod conversation_graph`.

---

## 6. Test matrix (consolidated)

| Layer | What |
|-------|------|
| Unit (headless) | layout ordering; encode table; model merge; pinned rebuild; interact clamps; cluster/LOD |
| Integration | live daemon + `talk_loop=true`: user Frontier → commit → assistant Follows visible on follow-live |
| Build | `scripts/build.sh check` / package test for `clawft-gui-egui`; wasm feature note in close comment |

---

## 7. Risks (from planning plan)

| Risk | Mitigation |
|------|------------|
| P1-graph slips | G4 ships degraded + honest hint |
| Unbounded global CausalGraph | P0 scopes by `conv_id`; GUI never holds global |
| Classification latency | P2 gated on ECC/`talk_loop` |
| paint.rs growth | Pre-split node vs edge/cluster paint |
| No real graph until M2 | Fixture + empty-state (D7) |

---

## 8. Out of scope (umbrella)

- Kernel P0/P1/P2 implementation (separate tickets)
- Editable graph, multi-conversation forest view, push subscriptions
- M4 tool-node edges (design-ready; wire when M4 lands)
- Filing all five child Plane items in this PR — **scaffold + plan only**; children on claim

---

## 9. Close checklist for WEFT-630 (this umbrella slice)

- [x] Phase plan doc (`docs/plans/adr-067-g1-g5-phase-plan.md`)
- [x] G1–G5 scaffold modules + `phase_status()` table
- [x] Explorer `pub mod conversation_graph`
- [ ] Child Plane items — create when each phase is started
- [ ] Full G1–G5 implementation — subsequent tickets
