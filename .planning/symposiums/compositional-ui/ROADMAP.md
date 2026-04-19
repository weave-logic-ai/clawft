# Compositional UI — Roadmap

**Purpose**: a single consolidated list of what's done, what's queued,
and what's deferred. Sources the fragmented lists that already exist
across AGENDA / sessions / ADRs / spec. Update this file as work moves.

Last updated: 2026-04-19.

---

## ✅ Done (Round 1 + Round 2 foundation)

- **`foundations.md`** — four predicates, two-tier model, observation
  tapestry, digital-exhaust = intent, active-radar loop.
- **`AGENDA.md`** — scope, Cooper + Evans + Rasmus + AIOS + Cedar
  anchors, nine research questions.
- **9 session findings** (~31,900 words): historical canon, agentic
  UI SOTA, shared realtime, ontology→UI, renderer+egui, protocol
  design, dev-panel embedding, governance, agentic-OS framing.
- **12 ADRs** (`adrs/adr-001`…`adr-012`, all *Proposed — Round 2*).
- **`protocol-spec.md`** — WSP-0.1 full spec (11k words, 20 §, full
  CDDL appendix, three wire example flows).
- **M0 — VSCode/Cursor extension scaffold** at
  `extensions/vscode-weft-panel/` — daemon ↔ extension ↔ webview wire
  loop, reload-survival, two sample RPC verbs.
- **`clawft-gui-egui` theming** — WeftOS tokens applied to main shell.
- **`weft-demo-lab` bin** — tabbed local mirror of
  `https://www.egui.rs/#demo` with Fractal / HTTP / 3D / Color Test +
  `theme-on/off` toggle for A/B proof.

---

## 🚧 In flight / next up

### M1 — egui-wasm in Cursor webview *(priority-one)*
The canon's first real render target. Closes the "CLI-trapped chat"
complaint.

- Compile `clawft-gui-egui` to `wasm32-unknown-unknown`.
- Wasm-flavoured `Live` posts through `acquireVsCodeApi()` instead of
  Unix sockets.
- Reload-survival via `WebviewPanelSerializer` + the existing M0
  extension skeleton.
- Boot splash + desktop shell inside Cursor.
- Kill the "X-over-WAN is slow" pain point permanently (see #perf
  below).

### M2 — active-radar loop closed end-to-end
The differentiator none of the market systems have. Builds on M1.

- `weft-capture` sidecar (mic + whisper.cpp) — voice input since VSCode
  webviews can't carry `allow="microphone"` yet
  (microsoft/vscode#303293).
- `ThreadDock` primitive with per-agent columns (R2's `MeshPanel`).
- Workspace adapter publishing `substrate/editor/{buffer,diagnostics,
  focus,tasks}` as first-class topics.
- `variant-id` + `substrate/ux/returns` wired through real widgets —
  topology / doppler / range / bearing capture per WSP §9.

### Primitive canon implementation (ADR-001 → code)
- `CanonWidget` trait + `CanonResponse` (extends `egui::Response` with
  topology/doppler/range/bearing) — R5's mechanism.
- Port the 17 tier-A primitives one by one. 60% of existing
  `src/blocks/*.rs` maps directly; see R5 spike-crosswalk. Priority
  order:
  1. `Chip` · `Button` · `Field` · `Toggle` · `Select` · `Slider`
  2. `Stack` · `Grid` · `Strip` · `Dock` · `Sheet` · `Modal`
  3. `Table` · `Tree` · `Gauge` · `Plot` · `Media`
- Tier-B `ForeignSurface` — stubs for `browser` (wry/viewport),
  `terminal` (egui_term), `video` (gstreamer+egui_wgpu),
  `xwindow` (XComposite), `app` (IPC viewport), `camera|mic|screen`
  (OS capture with the mandatory tray chip).

### Kernel & substrate wiring
- Add `MessagePayload::Wsp { frame: Vec<u8> }` variant to
  `crates/clawft-kernel/src/ipc.rs` (ADR-003).
- Implement WSP 17 verbs (ADR-005) in a `clawft-wsp` crate that
  binds `clawft-rpc` on UDS + the mesh envelope carrier.
- CDDL compile-time validation harness for the 18 primitives.

### Goal / governance binding
- `Goal` aggregate (ADR-008) in
  `crates/clawft-kernel/src/goal.rs` with five invariants.
- `GovernanceGate::check` signature extended to read `goal_id` from
  the context JSON. No new backend — reuse existing.
- `TrajectoryRecorder` per-goal scoping + seal-as-pattern-promotion.
- Tray primitive auto-composition for active captures (ADR-012).

### Perf quick wins *(ship independent of M1)*
- SSH compression + ControlMaster (config snippet in chat).
- Optional `WEFT_DEMO_FPS_CAP` env read in `weft-demo-lab` to clamp
  `ctx.request_repaint_after()` to e.g. 30 ms for X-over-WAN sessions.

---

## 🛠️ Round 3 — open questions carried over

Aggregate of `## Open questions` sections across the Round-1 session
docs + `protocol-spec §18`. Each belongs to a Round-3 session or ADR.

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
chip surface.

### RVF / WSP relationship (protocol-spec §18)
Two wire protocols in the same substrate. Same CBOR shop, different
purposes. Do they share envelopes? A common CDDL root? An explicit
alignment ADR is owed.

### Ontology manifest vs lazy `ontology.describe` (session-4, session-6)
Session-4 proposed a full manifest per session; session-6 kept
`ontology.describe` as a per-IRI lookup. Not both — choose, write ADR.

### Voice grammar — AccessKit vs phrase-map (session-5)
Do we lean on AccessKit's auto-generated grammar, or define our own
phrase map per ontology type? R5 deferred; needs a prototype.

### Ontology-sprawl policy (session-4)
When every primitive has an IRI, who prunes stale ones? Governance
role assignment needed.

### SHACL vs TopologySchema (session-4)
Both shape-constraint candidates for the geometry declarations.
Pilot each against our existing ontology-navigator schemas and
choose.

### CBOR-over-WebSocket perf (protocol-spec §18)
Browser renderers receive WSP as JSON-RPC. Native renderers receive
CBOR over mesh. Cost/benefit of also shipping CBOR over WebSocket
for high-rate observation.

### Cursor-specific integration points (session-7)
Cursor's own chat pane is closed but visible. R7 proposed reading it
as a `foreign://` signal. Concrete implementation path not yet
specified.

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
| Demo lab bin | `crates/clawft-gui-egui/src/bin/demo_lab.rs` |
| Theming tokens | `crates/clawft-gui-egui/src/theming.rs` |
| M0 extension | `extensions/vscode-weft-panel/` |

---

## Commit graph since the pivot

```
1259130 feat(gui): demo lab — add Fractal/HTTP/3D/Color tabs + theme-toggle A/B
5bc1fa2 feat(gui): WeftOS theming + weft-demo-lab bin hosting egui's full demo
c6d98f9 plan(symposium): 12 ADRs — symposium canon (Proposed Round 2)
e158a1a plan(symposium): WSP-0.1 — WeftOS Surface Protocol spec
fad8488 feat(gui): M0 — Cursor/VSCode extension scaffold for WeftOS panel
f826102 plan(symposium): Round 1 research complete — 9 findings docs, 32k words
524b56d plan(symposium): digital exhaust = intent — the protocol reframed
143f734 plan(symposium): two-tier canon — primitives + wrappers-as-signal
e1bb887 plan(symposium): active-radar loop — display=pulse, return=echo, ECC=receiver, GEPA=learn
5bca155 plan(symposium): Tier-B wrappers are a woven signal tapestry
442fa87 plan(symposium): add Rasmus + AIOS + Cedar anchors, codify foundation predicates
ef9a9c1 plan(symposium): compositional UI AGENDA with Cooper + Evans anchors
```
