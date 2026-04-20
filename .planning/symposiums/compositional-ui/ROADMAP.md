# Compositional UI — Roadmap

**Purpose**: a single consolidated list of what's done, what's queued,
and what's deferred. Sources the fragmented lists that already exist
across AGENDA / sessions / ADRs / spec. Update this file as work moves.

Last updated: 2026-04-20.

---

## ✅ Done (Rounds 1–3)

### Round 1 — Foundations
- **`foundations.md`** — four predicates, two-tier model, observation
  tapestry, digital-exhaust = intent, active-radar loop.
- **`AGENDA.md`** — scope, Cooper + Evans + Rasmus + AIOS + Cedar
  anchors, nine research questions.
- **9 session findings** (~31,900 words): historical canon, agentic
  UI SOTA, shared realtime, ontology→UI, renderer+egui, protocol
  design, dev-panel embedding, governance, agentic-OS framing.

### Round 2 — Decisions & protocol
- **14 ADRs** (`adrs/adr-001`…`adr-014`, all *Proposed — Round 2*).
  ADR-013 (`ui://canvas`) and ADR-014 (Modal modality split) added
  2026-04-19 after the demo-lab crosswalk surfaced gaps; ADR-001 and
  ADR-006 amended in place. Canon count reconciled to **21 primitives
  (20 Tier-A + 1 Tier-B)** after the 2026-04-20 amendment re-promoted
  `ui://tabs` as row 21 (distinct from `ui://dock`).
- **`protocol-spec.md`** — WSP-0.1 full spec (11k words, 20 §, full
  CDDL appendix, three wire example flows).

### Milestones shipped
- **M0 — VSCode/Cursor extension scaffold** at
  `extensions/vscode-weft-panel/` — daemon ↔ extension ↔ webview wire
  loop, reload-survival, two sample RPC verbs.
- **M1 — egui-wasm surface in the Cursor webview.**
  `clawft-gui-egui` compiles to `wasm32-unknown-unknown` via wasm-pack;
  target-conditional `Live` (tokio+UDS on native, postMessage bridge on
  wasm); shared monotonic `now_ms()`, unified reply-channel API.
  Extension host loads the bundle from `webview/wasm/`, installs
  `window.__weftPostToHost`, proxies the four poll RPCs to the daemon.
  Build script: `scripts/build-wasm.sh`. Commits `1204f7a`, `894de99`.
- **M1 polish (2026-04-20)** — `web-time` migration (wasm-safe
  Instant/SystemTime), PNG boot logo (PNG-format pin for wasm image
  loaders), VSCode extension wasm hot-reload (`fs.watch` + cache-bust
  token re-render), `ui/` gitignore. Commit `6e2df4d`.
- **M1 canon implementation (2026-04-20)** — full 21-item canon
  landed across four commits:
  - `eae8c17` foundation: `CanonWidget` trait + `CanonResponse`
    (topology/doppler/range/bearing) + `Pressable` reference + ADR-001
    row 21 amendment.
  - `853b9a8` retrofit: 8 existing blocks wrapped (Chip, Stack, Strip,
    StreamView, Table, Tree, Gauge, Plot).
  - `54557f5` new primitives: 11 missing items (Field, Toggle, Select,
    Slider, Grid, Dock, Sheet, Modal, Media, Canvas, Tabs). Adds
    `egui_dock 0.14` dep.
  - `7ddc5ee` merge into `development-0.7.0` + 7 head-metadata smoke
    tests (identity URIs, toggle affordances, modal-freeze on consent
    flows, canvas transform default, field value kind tags, grid has
    no affordances, tabs exposes switch-tab).
- **Canon demo lab in the panel (2026-04-20)** — `canon_demos.rs` +
  `Blocks | Canon` section toggle in the Desktop Blocks window. 20
  per-primitive demos with live `CanonResponse` footer read-out
  (identity / variant / bearing / latency). Commit `7c0c523`.

### Round 3 — App Layer planning (2026-04-20)
- **`session-10-app-layer.md`** (292 lines) — anchor doc. 3 modes
  (`single-app` / `desktop` / `ide`) × 4 inputs (`pointer` / `touch` /
  `voice` / `hybrid`). Avatar as `single-app × voice` archetype with
  `foreign://avatar` + narration contract. Four reference panels
  (WeftOS Admin, Project OS, Deployment Monitor, Avatar Companion)
  cross-walked against modes and inputs.
- **5 new ADRs** (all *Proposed — Round 3*):
  - **ADR-015** App manifest — TOML schema, `supported_modes`,
    `supported_inputs`, `narration`, `permissions`, `entry_points`,
    three authoring paths, semver backwards-compat rules.
  - **ADR-016** Surface description format — TOML-with-extensions +
    equivalent Rust builder, tree of typed primitive nodes, binding
    expression language (shared with narration), conditional
    rendering, local compositions, mode-specific variants, headless
    test harness.
  - **ADR-017** Ontology adapter contract — `OntologyAdapter` trait,
    `TopicDecl` + `StateDelta`, back-pressure policy, 7 reference
    adapters (`kernel`, `git`, `gh`, `workspace`, `fs`, `lsp`,
    `deployment`), sensitivity + ADR-012 capture binding,
    `substrate/meta/adapter/*/health` for offline awareness.
  - **ADR-018** IDE bridge protocol — 4 linkage components, six
    `substrate/editor/*` topics, seven `ide.*` affordances,
    follow-editor manifest DSL, chrome-match palette bridge, VSCode
    postMessage transport (first class) + JetBrains / Neovim
    deferred, handshake + capability negotiation, security + governance.
  - **ADR-019** Input modality + `foreign://avatar` + narration —
    Part A input axis adaptation rules (hit-target floor, hover
    semantics, gesture mapping, chrome-per-mode matrix, safe-area
    insets). Part B **amends ADR-006** with `invocations: [pointer |
    touch | voice | gesture]` on every `Affordance`. Part C adds
    `foreign://avatar` Tier-B kind + narration contract in app
    manifests + voice-triggered launcher + mandatory caption
    `StreamView` for accessibility.

---

## 🚧 In flight / next up (revised milestone stack)

Round 3 planning reshaped the milestone order. **M1.5 is inserted
ahead of M2** because M2's instrumentation is only meaningful once an
app layer exists to instrument.

### M1.5 — First real panel driven by a surface description
**Acceptance**: the WeftOS Admin reference panel (session-10 §6.1)
runs as a declarative surface description, registered as an app,
launchable in `ide × pointer` (Cursor) and `desktop × pointer`
(`weaver gui`). `Desktop::render_blocks_window`'s hand-coded
`BlockKind` match is replaced by app-manifest dispatch.

- `clawft-app` crate — minimal manifest parser + app registry (SQLite
  or JSON for M1.5). Implements the schema from ADR-015.
- `clawft-surface` crate — minimal surface-description parser +
  `SurfaceTree` IR + composer runtime. Implements a subset of ADR-016
  sufficient for the admin panel (Stack/Strip/Grid/Chip/Gauge/Table/
  StreamView + `$substrate/...` bindings + `count/filter` expressions).
- `kernel` ontology adapter refactor — replaces polling `Live` with
  the trait from ADR-017 §1. Emits `substrate/kernel/{status,
  processes, services, logs}` as deltas.
- WeftOS Admin manifest + surface descriptions (both TOML and Rust
  builder variants per ADR-015 §concrete-example).
- Delete `BlockKind` enum once the admin app covers the existing
  demos. (Session-10 rec. 13.)

**Explicit non-goals for M1.5**: git/gh/deploy adapters, any voice /
avatar work, touch adaptation, follow-editor, full IDE bridge.

### M1.6 — IDE bridge editor-in
Adds `substrate/editor/*` topics (the `workspace` adapter from
ADR-017 §5). Wires the VSCode postMessage transport per ADR-018 §5.
Project-OS reference panel (session-10 §6.2 top row) becomes viable.

### M1.7 — IDE bridge editor-out
`ide.*` affordance catalog per ADR-018 §2. Install-time + per-
invocation governance prompts for destructive verbs. Project-OS panel
can reach back into the editor (open-file, jump-to-line, quick-pick).

### M1.8 — Touch adaptation
Compositor hit-target floor, gesture mapping, safe-area insets per
ADR-019 Part A. Deployment Monitor (session-10 §6.3 row 2) viable on
tablet. Default-detection table wired into host launch paths.

### M1.9 — Voice + avatar + narration
`foreign://avatar` Tier-B shell, `weft-capture` sidecar promoted from
M2, narration template parser, voice-triggered launcher, caption
`StreamView`. Avatar Companion panel (session-10 §6.4) viable.
ADR-006 amendment (Affordance `invocations` field) lands across all
21 canon primitives.

### M2 (revised) — Active-radar loop closed end-to-end
Unchanged in spirit but now instruments a real app layer:
- `ThreadDock` primitive with per-agent columns (R2's `MeshPanel`) —
  ships as an app-level composition over Canon primitives (Stack +
  Tree + StreamView), not a new primitive.
- `variant-id` + `substrate/ux/returns` wired through real widgets —
  topology / doppler / range / bearing capture per WSP §9.
- `Goal` aggregate (ADR-008) in `crates/clawft-kernel/src/goal.rs`
  with five invariants.
- `GovernanceGate::check` signature extended to read `goal_id` from
  the context JSON. No new backend — reuse existing.
- `TrajectoryRecorder` per-goal scoping + seal-as-pattern-promotion.

### Kernel & substrate wiring (unscheduled — runs in parallel)
- Add `MessagePayload::Wsp { frame: Vec<u8> }` variant to
  `crates/clawft-kernel/src/ipc.rs` (ADR-003).
- Implement WSP 17 verbs (ADR-005) in a `clawft-wsp` crate that
  binds `clawft-rpc` on UDS + the mesh envelope carrier.
- CDDL compile-time validation harness for the 21 primitives.

### Tier-B `ForeignSurface` stubs (M2+)
- `browser` (wry/viewport), `terminal` (egui_term), `video`
  (gstreamer+egui_wgpu), `xwindow` (XComposite), `app` (IPC viewport),
  `camera|mic|screen` (OS capture with the mandatory tray chip).
- `avatar` — lands in M1.9 per ADR-019 Part C.

### Perf quick wins *(ship anytime)*
- SSH compression + ControlMaster (config snippet in chat).
- Optional `WEFT_DEMO_FPS_CAP` env read in `weft-demo-lab` to clamp
  `ctx.request_repaint_after()` to e.g. 30 ms for X-over-WAN sessions.
- wasm bundle size optimise (release-lto + wasm-opt-from-source →
  ~1.3 MB gzipped). Current debug bundle: 5.0 MB.

---

## 🛠️ Round 3 — open questions carried over

Aggregate of `## Open questions` sections across the Round-1 session
docs + `protocol-spec §18` + Round-3 ADR deferrals. Each belongs to a
Round-3+ session or ADR.

### App-layer questions opened by Session 10
- **App distribution.** Crates.io? Private registry? IPFS/content-
  addressed? Not urgent for M1.5 — local filesystem install only.
- **App sandboxing.** Rust apps have full process access; declarative
  apps are sandboxed by construction (read declared topics only). Do
  we need a middle tier? Deferred until first community app.
- **Localisation.** Narration rules in English only for M1.9. i18n is
  a Round 4 concern.
- **Multiple avatars.** Can a session host two avatars (technical +
  executive assistant)? Not designed in ADR-019; out of scope M1.9.
- **Ontology shape URIs.** ADR-017 uses `ontology://<x>` placeholders.
  Full ontology-shape spec is TBD in ADR-020+ (paired with the SHACL
  vs TopologySchema decision below).

### Cross-substrate identity (session-7, session-8, protocol-spec §18)
Rasmus unsolved #1 at the "multiple WeftOS substrates collaborating"
layer. ExoChain identity maps inside one substrate; cross-substrate
delegation is unspecified. Proposal: follow-up symposium track plus an
ADR.

### Observation back-pressure (protocol-spec §18, session-6)
What happens when a greedy consumer falls behind the `ux/returns`
stream? Drop-oldest vs coalesce vs apply per-channel quality
gradients. Needs a field experiment.

### Mission Console projection negotiation (session-3, session-9)
Rooms with heterogeneous renderer capabilities — browser-based
participant + native participant + TTY observer — need a per-
participant projection of the same surface. Spec stubs in WSP but
negotiation protocol is undefined.

### OT vs event-sourced for the composition spine (session-3)
R3 picked event-sourced spine + CRDT leaves; worth a follow-up
empirical validation on large Mission Console rooms.

### Variant reconciliation across participants (session-3, session-6)
When two participants render different GEPA variants of the same
primitive, what's the audit representation? How is "which variant
won" recovered?

### Offline / partition behaviour over our mesh (session-3)
A peer drops, rejoins with a divergent local history — we have the
chain primitives, but the replay story for Mission Console rooms is
sketched only.

### Permissions-as-affordances vs permissions-as-filters (session-3, session-8)
If the primitive already lists the affordances it offers, is
permission a gate on *which* affordances appear, or a later filter
when the user/agent tries to invoke one? The answer shapes the tray
chip surface. (Partially addressed by ADR-015 install-time consent +
ADR-018 per-invocation governance; full resolution deferred.)

### RVF / WSP relationship (protocol-spec §18)
Two wire protocols in the same substrate. Same CBOR shop, different
purposes. Do they share envelopes? A common CDDL root? An explicit
alignment ADR is owed.

### Ontology manifest vs lazy `ontology.describe` (session-4, session-6)
Session-4 proposed a full manifest per session; session-6 kept
`ontology.describe` as a per-IRI lookup. Not both — choose, write ADR.

### Voice grammar — AccessKit vs phrase-map (session-5)
Do we lean on AccessKit's auto-generated grammar, or define our own
phrase map per ontology type? ADR-019 §Part B picked canonical-name +
optional per-app `utterances` — AccessKit integration remains open.

### Ontology-sprawl policy (session-4)
When every primitive has an IRI, who prunes stale ones? Governance
role assignment needed.

### SHACL vs TopologySchema (session-4)
Both shape-constraint candidates for the geometry declarations.
Pilot each against our existing ontology-navigator schemas and
choose. ADR-017's shape-URI placeholders await this.

### CBOR-over-WebSocket perf (protocol-spec §18)
Browser renderers receive WSP as JSON-RPC. Native renderers receive
CBOR over mesh. Cost/benefit of also shipping CBOR over WebSocket
for high-rate observation.

### Cursor-specific integration points (session-7)
Cursor's own chat pane is closed but visible. R7 proposed reading it
as a `foreign://` signal. Concrete implementation path not yet
specified. ADR-018 specifies the *bridge* but not this specific
read-only capture.

### Confidence-visual uniformity (session-5, session-7)
Every primitive exposes confidence; every renderer visualises it
differently if unspecified. Decide one visual language (icon, halo,
opacity gradient, badge) and apply everywhere.

### Goal vs Task vs Milestone naming (session-8)
R8 landed on "Goal = Evans aggregate"; related concepts are currently
unnamed. Likely not an ADR until M2 forces the question.

---

## 📚 Repo landmarks for this workstream

| Thing | Path |
|-|-|
| Canon | `.planning/symposiums/compositional-ui/foundations.md` |
| Agenda | `.planning/symposiums/compositional-ui/AGENDA.md` |
| Research | `.planning/symposiums/compositional-ui/session-*.md` |
| Decisions | `.planning/symposiums/compositional-ui/adrs/adr-*.md` |
| Spec | `.planning/symposiums/compositional-ui/protocol-spec.md` |
| This file | `.planning/symposiums/compositional-ui/ROADMAP.md` |
| Main shell | `crates/clawft-gui-egui/` |
| Canon primitives | `crates/clawft-gui-egui/src/canon/` |
| Canon demos | `crates/clawft-gui-egui/src/canon_demos.rs` |
| Demo lab bin | `crates/clawft-gui-egui/src/bin/demo_lab.rs` |
| Theming tokens | `crates/clawft-gui-egui/src/theming.rs` |
| M0 extension | `extensions/vscode-weft-panel/` |
| M1.5 new crates | `crates/clawft-app/` (TBD), `crates/clawft-surface/` (TBD) |

---

## Commit graph since the pivot

```
7c0c523 feat(gui): canon demo lab — 20 primitive demos in the WeftOS panel
7ddc5ee merge(canon): foundation + retrofit + new primitives — full 21-item canon landed
6e2df4d feat(gui+ext): M1 wasm-compat polish — web-time, PNG logo, extension hotload
ab25a49 merge(canon): new primitives — 11 missing canon items (Field/…)
c937620 merge(canon): retrofit — 8 existing blocks wrapped in CanonWidget trait
54557f5 feat(canon): new primitives — Field/Toggle/Select/Slider/Grid/Dock/Sheet/Modal/Media/Canvas/Tabs
853b9a8 feat(canon): retrofit — wrap 7 existing blocks in CanonWidget trait
eae8c17 feat(canon): foundation — CanonWidget trait + CanonResponse + Pressable reference
894de99 feat(gui): M1 (part 2) — wasm bundle + VSCode extension hosts egui in Cursor
1204f7a feat(gui): M1 (part 1) — clawft-gui-egui compiles to wasm32-unknown-unknown
c0470d3 plan(symposium): canon reconciled — 19 Tier-A + 1 Tier-B (StreamView + Canvas + Modal modality)
7452ea3 plan(symposium): consolidated ROADMAP — done/in-flight/Round 3 open
1259130 feat(gui): demo lab — add Fractal/HTTP/3D/Color tabs + theme-toggle A/B
5bc1fa2 feat(gui): WeftOS theming + weft-demo-lab bin hosting egui's full demo
c6d98f9 plan(symposium): 12 ADRs — symposium canon (Proposed Round 2)
e158a1a plan(symposium): WSP-0.1 — WeftOS Surface Protocol spec
fad8488 feat(gui): M0 — Cursor/VSCode extension scaffold for WeftOS panel
f826102 plan(symposium): Round 1 research complete — 9 findings docs, 32k words
524b56d plan(symposium): digital exhaust = intent — the protocol reframed
```
