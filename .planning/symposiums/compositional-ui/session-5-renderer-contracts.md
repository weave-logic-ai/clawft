# Session 5 — Renderer Contracts (egui-canonical)

**RQ5**: What is the smallest primitive set that satisfies the four
predicates and maps cleanly to egui's immediate-mode model? egui is
canonical; this document is about fit, not alternatives.

---

## Scope

We define **the canon** — an opinionated 18-primitive set, each with
a four-predicate checklist and a concrete egui implementation path.
We then resolve four cross-cutting concerns that the primitive list
alone cannot carry: `ForeignSurface`, streaming, variant-id / mutation,
and return-signal capture. The existing spike at
`crates/clawft-gui-egui/src/{blocks,shell,live}` is treated as a
pre-canon prototype — about 60% of what it already does maps to the
canon one-for-one; the rest is reorganised under the new primitives
or demoted to Tier B.

The deliverable is scoped to the protocol seam: *what the substrate
says to the renderer, and what the renderer returns to the substrate*.
Widget look-and-feel (typography, spacing, density) is a theme layer
on top of the canon, not part of it.

---

## Canon — 18 primitives

Each primitive is named, purposed, predicate-checked (Ont / Self /
Stream / Dual), and mapped to concrete egui APIs. All 18 are Tier A
— they all satisfy the four predicates — except `ForeignSurface`
which is the sole Tier B, and `ForeignSurface`'s shell still satisfies
the four predicates as required by foundations.md.

### Interactive surface primitives

**1. `Chip`** — a labelled, coloured, interactive status token.
*Predicates*: Ont ✓ (chip type URI + state id), Self ✓ (state / tint /
confidence / tooltip / affordances), Stream ✓ (state updates push),
Dual ✓ (clickable + voice-invokeable by label).
*egui*: `egui::Frame` → `allocate_exact_size` + `Painter::circle_filled`
+ `selectable_label`, wrapped in a `Widget` impl that returns
`Response`. Persistent selection via `ctx.memory_mut().data.insert_temp(id, ...)`.
The existing `shell/tray.rs::chip()` is already 90% the target shape;
needs an `id_salt` and an affordance list attached.

**2. `Button`** — an invoke-one-verb affordance.
*Predicates*: Ont ✓ (verb URI), Self ✓ (enabled / loading / destructive
flag / last-outcome), Stream ✓ (outcome streams back), Dual ✓
(`Response::clicked` + voice phrase attached via label or
`aria_label`).
*egui*: `ui.add(egui::Button::new(label))` → `Response`. Variants
(primary / secondary / ghost / destructive) map to `Button::fill`,
`::frame(false)`, and a small custom painter for destructive tint.
`blocks/button.rs` is already this, minus affordance metadata.

**3. `Field`** — a typed input binding (text / number / date / enum /
boolean / multiline).
*Predicates*: Ont ✓ (binding URI + schema type), Self ✓ (value /
dirty / validation / confidence / inference-origin), Stream ✓ (both
directions — substrate push and user-edit push), Dual ✓ (keyboard +
voice via STT into the same binding).
*egui*: `TextEdit::singleline` / `multiline`, `DragValue`,
`egui_extras::DatePickerButton`, `ComboBox`, `Checkbox`. Dirty /
validation tint on `ui.style_mut().visuals.widgets.inactive.bg_stroke`.
Voice is a sibling input source that writes the same underlying
`String` the `TextEdit` borrows.

**4. `Toggle`** — a boolean binding distinguished from `Field` because
agents reason about it specifically (it's a verb-capable state).
*Predicates*: Ont ✓, Self ✓, Stream ✓, Dual ✓ (tap / space / say
"turn on X").
*egui*: `ui.toggle_value(&mut bool, label)` and `Checkbox`.

**5. `Select`** — a closed-choice picker.
*Predicates*: Ont ✓ (choice set URI), Self ✓ (choices + selected +
disabled set), Stream ✓ (choice set itself can stream — agents
amend the list live), Dual ✓.
*egui*: `egui::ComboBox` for ≤N; `RadioButton` for small N;
`egui_extras::TableBuilder` + row-selectable for large or structured
sets.

**6. `Slider`** — a continuous range binding.
*Predicates*: Ont ✓, Self ✓ (range, step, units, confidence if
model-suggested), Stream ✓, Dual ✓ ("set X to 40%").
*egui*: `egui::Slider`.

### Layout primitives

**7. `Stack`** — one-axis flow (row / column). Every other primitive
nests inside one.
*Predicates*: Ont ✓ (container URI), Self ✓ (axis / gap / align /
child list), Stream ✓ (children can be streamed in / removed), Dual ✓
(agent reorders via schema; user reorders via drag-handle when
declared).
*egui*: `ui.horizontal(|ui| ...)` / `ui.vertical(|ui| ...)` /
`ui.horizontal_wrapped`. `blocks/layout.rs` already proves this.

**8. `Grid`** — two-axis regular layout.
*Predicates*: all four.
*egui*: `egui::Grid::new(id).num_columns(n)` + `ui.end_row()`.

**9. `Strip`** — fixed-ratio, fixed-size division (dashboards, split
panes).
*Predicates*: all four.
*egui*: `egui_extras::StripBuilder`, `Size::exact` / `::remainder` /
`::relative`. This is what the Mission Console war-room layout will
be built from.

**10. `Dock`** — resizable tabbed / split panels with persisted
geometry; the Mission Console chrome.
*Predicates*: Ont ✓ (layout URI + tab IDs), Self ✓ (tab tree + focus
+ docking state), Stream ✓ (tabs appear and close), Dual ✓
(drag-to-dock + agent `dock(panel, region)`).
*egui*: `egui_dock::DockArea` + `DockState`. Geometry is persistable
through egui memory + serde — the right home for our "session
resumable" requirement.

**11. `Sheet`** — a scrollable region with sticky headers; the spine
of long content (logs, chats, docs).
*Predicates*: all four.
*egui*: `egui::ScrollArea::vertical().stick_to_bottom(true)` (the
`blocks/terminal.rs` pattern). `id_salt` for multiple scrollers.

**12. `Modal`** — a foreground surface that captures focus for a
bounded decision.
*Predicates*: all four (affordances are `confirm / cancel / dismiss`,
voice-operable).
*egui*: `egui::Area::new(id).order(egui::Order::Foreground).movable(false)`
+ a scrim painted on `LayerId::background` sublayer, or `Window`
with `collapsible(false).resizable(false)`. Agent closes via a typed
affordance, not by mutating DOM-style.

### Data-view primitives

**13. `Table`** — sortable / filterable tabular data with selection.
*Predicates*: Ont ✓ (schema URI per column), Self ✓ (sort state /
selection / visible-columns / row count), Stream ✓ (rows append,
sort reorders a live list), Dual ✓ (click sort, "sort by PID
descending").
*egui*: `egui_extras::TableBuilder` + `Column::auto/remainder/exact`
+ `.header(h, |h| ...)` + `.body(|b| b.row(...))`. `blocks/table.rs`
is the right shape; it needs a `column_schema` attached for agent
introspection.

**14. `Tree`** — hierarchical disclosure.
*Predicates*: all four.
*egui*: `egui::CollapsingHeader::new(label).id_salt(path).show(ui, |ui| ...)`
(the `blocks/tree.rs` pattern). Persist open-set via
`ctx.memory_mut().data.get_persisted_mut_or_default::<HashSet<Id>>()`.

**15. `Gauge`** — a single scalar with bounds, thresholds, and a
confidence halo; for status cards, budget bars, health readouts.
*Predicates*: Ont ✓, Self ✓ (value / min / max / thresholds /
confidence / source), Stream ✓, Dual ✓ (speak value, agent queries
current).
*egui*: `egui::ProgressBar::new(pct).fill(c)` for linear;
`ui.painter().circle` + `Painter::arc` for radial. `blocks/budget.rs`
+ `blocks/status.rs::card` already cover the two shapes.

**16. `Plot`** — continuous-time series (oscilloscope, sparkline, net
graph, doppler plot).
*Predicates*: Ont ✓ (series schema URI), Self ✓ (window / samples /
axes / confidence band), Stream ✓ (samples push), Dual ✓ (zoom /
query a sample, agent subscribes).
*egui*: `egui_plot::Plot::new(id)` + `Line` / `Points` / `Polygon`.
`blocks/oscilloscope.rs` is the reference. Live subscription support
is what `live.rs` already does for RPC polling — generalise to named
streams.

**17. `Media`** — an image / icon / glyph primitive, distinct from
`ForeignSurface` because we render the decoded pixels ourselves and
the content is ontology-knowable (icon URI, asset URI).
*Predicates*: Ont ✓, Self ✓ (source / dims / alt / confidence),
Stream ✓ (progressive loader), Dual ✓ (alt text is the agent's read
handle).
*egui*: `egui_extras::install_image_loaders` +
`egui::Image::new("bytes://foo" | "file://" | "https://...")`. We
already use this in `app.rs` for the boot logo.

### Wrapper primitive (Tier B)

**18. `ForeignSurface`** — the single Tier-B primitive; the host for
opaque content. Its **shell** is Tier A (must satisfy all four
predicates per foundations.md); its **interior** is opaque to
primitive composition but still emits signals.
*Predicates on the shell*: Ont ✓ (`foreign://<kind>/<id>`), Self ✓
(kind / source / focus / lifecycle / health / last-event), Stream ✓
(lifecycle + activity events), Dual ✓ (navigate / focus / screenshot
/ terminate / inject).
*egui mapping*: see the per-kind table below.

---

## Primitives deliberately excluded

- **`Form`** — composed from `Stack(Field…) + Button(submit)`; not its
  own primitive. Agents describe forms as compositions.
- **`Card`** — a `Stack` with a styled `Frame` and a `Gauge` / `Chip`
  cluster. Theme concern, not a primitive.
- **`Toolbar` / `Menu` / `Tabs`** — `Stack(Button…)` and
  `Stack(Selectable…)` respectively. `Tabs` in the spike
  (`blocks/tabs.rs`) is re-expressed as `Stack` + `Select(single)` +
  `Sheet`.
- **`Notification` / `Toast`** — `Modal` at `Order::TOP`, auto-dismiss.
- **`Launcher` / `Tray`** — specific compositions of `Dock` + `Chip`,
  authored at the shell level, not a canon entry.

This keeps the canon ≤25 and pushes bespoke UIs into *composition*,
which is the whole point of the reverse-DDD arrow.

---

## egui mapping — condensed

| Primitive | Core egui API | State home | Response surface |
|-|-|-|-|
| Chip | `Frame`+`allocate_exact_size`+`Painter::circle_filled`+`selectable_label` | `IdTypeMap` via `id_salt` | `Response.clicked`, `.hovered` |
| Button | `Button::new` | stateless (caller holds) | `Response.clicked`, `.lost_focus` |
| Field | `TextEdit::*`, `DragValue`, `DatePickerButton` | caller-owned binding | `Response.changed`, `.lost_focus` |
| Toggle | `toggle_value`, `Checkbox` | caller-owned binding | `Response.changed` |
| Select | `ComboBox`, `RadioButton`, `TableBuilder` | caller-owned selection | `Response.changed` |
| Slider | `Slider` | caller-owned value | `Response.changed`, `.dragged` |
| Stack | `ui.horizontal`, `.vertical`, `.horizontal_wrapped` | layout-only | inner responses |
| Grid | `egui::Grid::new(id)` | grid id in memory | inner responses |
| Strip | `egui_extras::StripBuilder` | layout-only | inner responses |
| Dock | `egui_dock::DockArea` + `DockState` | `DockState` (persist-serde) | tab/close events |
| Sheet | `ScrollArea::vertical().stick_to_bottom` | scroll state in memory | `Response.dragged`, scroll vel |
| Modal | `Area.order(Order::Foreground)` + scrim | focus stack in memory | verdict enum |
| Table | `egui_extras::TableBuilder` | sort/selection in caller + memory | row clicks |
| Tree | `CollapsingHeader` | persistent open-set via `memory.data` | node clicks |
| Gauge | `ProgressBar` or `Painter::arc` | stateless | hover only |
| Plot | `egui_plot::Plot` | plot bounds in memory | `PlotUi` inspector |
| Media | `Image::new(uri)` + `egui_extras` loaders | loader cache | `Response.hovered` |
| ForeignSurface | viewport / texture / process host (see next) | shell state | typed lifecycle stream |

All primitives implement a single trait:

```rust
trait CanonWidget {
    fn id(&self) -> egui::Id;                 // ontology-addressable
    fn identity_uri(&self) -> &str;            // substrate URI
    fn state(&self) -> CanonState;             // self-describing
    fn affordances(&self) -> &[Affordance];    // self-describing
    fn confidence(&self) -> Confidence;        // self-describing
    fn variant_id(&self) -> u64;               // GEPA hook
    fn mutation_axes(&self) -> &[MutationAxis];// GEPA hook
    fn show(self, ui: &mut egui::Ui) -> CanonResponse;  // maps to egui::Response
}
```

`CanonResponse` wraps `egui::Response` and adds the return-signal
fields (topology / doppler / range / bearing); see the dedicated
section below.

---

## ForeignSurface — per-kind implementation

The shell is always a tier-A `egui::Frame` that owns an `Id`, a
lifecycle state machine, a signal sink, and a body callback the
renderer invokes. Six bodies, six implementations.

### `foreign://browser`

**Host**: spawn a native `WebviewWindow` via `wry` in a child
`ViewportBuilder` (`ctx.show_viewport_deferred`) on native desktop;
reduce to an `Image` of a server-rendered screenshot on web. The egui
body draws chrome (URL bar chip, reload button, progress gauge); the
webview is a sibling native window anchored to the egui viewport's
rect. Anchoring is handled by reading `ctx.input(|i| i.viewport())`
and issuing `ViewportCommand::OuterPosition` each frame — this is
the same trick Tauri's `tauri-egui` uses.
**Signals**: load-start, title, url, a11y-tree-digest, screenshot
(periodic), clipboard-read, script-exec-result. URL → ontology hint
(`foreign://browser` + URL). Governance-gated affordances:
`execute-script`, `read-a11y-tree`.
**Fallback**: when running inside a Cursor/VSCode webview sidecar,
the egui surface *is* a webview already — the browser child is a
nested `<iframe>` the egui shell controls via `postMessage`.

### `foreign://terminal`

**Host**: `alacritty_terminal::tty::Pty` + `egui_term::TerminalView`
(upstream already solves the cell grid → egui texture problem). The
shell adds the affordance layer; the inner terminal stays opaque.
**Signals**: resize, exit-code, output-rate, active-process (parsed
from zsh/bash `$ ` prompts and fallback regex), clipboard-read,
transcript window. `blocks/terminal.rs` is the wrong shape for this
— it's an RPC REPL, not a PTY host; we keep it as a separate
`RpcConsole` composition (`Stack(Sheet, Field, Button)`) and let
`ForeignSurface(terminal)` be the real PTY.

### `foreign://video`

**Host**: GStreamer `appsink` → YUV / RGBA → upload as a
`wgpu::Texture` via `egui_wgpu::Renderer::register_native_texture`
(there's a reference impl at `ShabbirHasan1/egui-gstreamer-video-player`).
The egui body is a `Media`-like primitive over that texture with
overlay controls (`Button(play/pause)`, `Slider(seek)`).
**Signals**: play-state, position, track, audio-level, caption-text
(via `textsink`), frame-hash (every N seconds for scene-change
heuristics).

### `foreign://xwindow`

**Host**: Linux-only path. Reparent an existing X11 / Wayland window
into an offscreen buffer via XComposite (X11) or wayland-scanner +
`wp_viewporter` (Wayland), grab the pixmap, upload as an egui
texture. Input is proxied back via XTestFakeKeyEvent / wl-shim. This
is the highest-effort wrapper; realistic plan is to ship Wayland via
`smithay` on our compositor builds and defer X11 to a Q3 deliverable.
macOS / Windows: not supported in v1, fall back to `foreign://app`.
**Signals**: window-title, PID, focus, resize, last-input-at, crash.

### `foreign://app`

**Host**: spawn another WeftOS renderer process; communicate via our
own kernel daemon IPC (the mesh already carries `a2a` — this is the
same thing). The egui body either (a) hosts the remote app in a new
native viewport (`show_viewport_deferred`) or (b) reads a shared
snapshot texture if the peer renders headless.
**Signals**: bundle-id, PID, handshake version, last-message,
health, exit-code.

### `foreign://camera`, `foreign://mic`, `foreign://screen`

**Host**: OS capture APIs → a ring buffer on the poller thread
(`live.rs` model generalised), consumed by the egui body as
`egui_plot::Line` (mic waveform), `Image` (camera preview),
`Line+dots` (screen OCR deltas). Governance gate is non-negotiable:
these three primitives render a persistent `Chip` in the tray as a
permanent observability affordance — the "Observable to the user"
constraint from foundations.md.
**Signals**: level / frames / transcript / OCR-text; never raw
capture to ontology.

**ForeignSurface state machine** (egui-side, same for all kinds):

```
Created → Loading → Ready → (Focused ↔ Blurred) → (Suspended) → Closed
                     │
                     └── Error ──→ Closed
```

Every transition is a streamed event on the shell's subscription;
the substrate records them into the user's chain-backed observation
store.

---

## Streaming + variant-id + return-signal

### Streaming pattern

egui's immediate-mode model redraws every frame. Streaming is solved
by a **Store + Poller + repaint** triad that is already operational
in `live.rs`. Generalised:

```
Substrate ──stream──▶ PollerThread ──write──▶ Store<RwLock>
                                              │
egui frame N ◀──snapshot()──────────────────── │
                                              ▼
                                         ctx.request_repaint_after(Δ)
```

The `Store` holds per-subscription typed buffers (append-only rings
for `Plot`/`Sheet`/`Terminal`, last-value cells for `Gauge`/`Chip`,
CRDT state for `Field` with collaborative writers). Each primitive
reads its slice of the store on `show()`; the poller calls
`ctx.request_repaint()` when a write lands. Stream verbs (`open`,
`chunk`, `commit`, `cancel`) from foundations.md map directly to
store ops: `open` creates a subscription, `chunk` appends, `commit`
flips dirty→clean, `cancel` closes and drains.

Back-pressure: the poller's `mpsc` command queue already bounds
writes. When the ring is full we either drop the oldest (plot,
log) or refuse the write (form submission), declared per
subscription at `open` time.

### Variant-id + mutation schema

Every `CanonWidget::show()` call is tagged with `variant_id: u64`
produced by the GEPA layer. The id is stashed in
`ctx.memory_mut().data.insert_temp(id.with("variant"), variant_id)`
and stamped onto every outgoing return-signal event so the ECC can
attribute the echo to the exact pulse that emitted it. The schema
declares per-primitive mutation axes (e.g. `Button`: {copy, icon,
tint, size, placement}; `Gauge`: {radial-vs-linear, threshold-labels
yes/no}; `Plot`: {y-autoscale, sample-density}). Non-mutable axes
are fixed by the schema — `Modal.affordances = [confirm, cancel]`
is *never* legal to vary, nor are consent flows, nor brand marks.

### Return-signal schema

`CanonResponse` extends `egui::Response` with four reconstructable
quantities:

- **Topology** — the `egui::Id` of the touched primitive plus its
  path through its containing `Stack/Grid/Strip/Dock`. Reconstructed
  from `Response.rect` + the egui layer hierarchy
  (`ctx.memory(|m| m.layer_id_at(pos))`).
- **Doppler** — `Response.drag_delta()` magnitude and direction, plus
  inter-press velocity (`time_since_last_click` from `memory.data`),
  plus `InputState::pointer.delta`. Emitted as a signed scalar per
  primitive.
- **Range / latency** — the delta between the frame the primitive
  first appeared (stored in `memory.data.get_persisted_mut_or`) and
  the frame the user acted. Measured in ms via
  `ctx.input(|i| i.time)`.
- **Bearing** — which affordance out of the declared set was chosen.
  Encoded as the affordance URI, not an index, so ECC attribution is
  stable across variants that reorder the affordance list.

`CanonResponse` is serialised and pushed on the observation stream
every time an action is taken. Implicit signals (dwell, hover,
scroll without click) are sampled at a lower rate (1 Hz) by walking
`ctx.memory` for tracked primitive ids. All of this fits behind one
function that the UI calls once at end-of-frame:

```rust
fn emit_return_signals(&mut self, ctx: &egui::Context) {
    for (id, tracker) in &self.return_trackers {
        tracker.sample(ctx).map(|sig| self.observation_tx.send(sig));
    }
}
```

---

## Recommendations for canon

These are numbered for ADR crosswalk. Each cites the predicate(s) or
tier it enforces.

1. **Adopt the 18-primitive set as the canon.** Tier-A: 17. Tier-B:
   1 (`ForeignSurface`). No Tier-C. (foundations.md §"Anti-rule")
2. **Every primitive implements one trait (`CanonWidget`) that
   returns `CanonResponse`.** This is the surface protocol's
   renderer-side boundary. (Predicates 1, 2, 4)
3. **State lives in egui `Memory` keyed by `id_salt` for ephemeral
   UI state; substrate-backed state lives in the `Store`.** Never
   mix. The poller's `live.rs` pattern is the reference.
   (Predicate 3)
4. **Streaming uses append-only rings in the `Store` plus
   `request_repaint`; there is no frame-skipping fast path.** egui's
   immediate-mode redraw is cheap enough at 60fps for our use; if
   it isn't for Plot/Terminal, drop to `ctx.request_repaint_after`
   with variable Δ. (Predicate 3)
5. **`ForeignSurface` uses native viewports (desktop) or nested
   webviews (editor sidecar), and textures for
   camera/mic/screen/video; it never spawns a raw X window inside
   the main egui layer on our primary targets.** Predictable cross-
   platform behaviour is more valuable than pixel-parity. (Tier B
   shell, all four predicates)
6. **The three capture wrappers (`camera`, `mic`, `screen`) render a
   permanent tray `Chip` whenever any is live.** This is a
   foundations.md non-negotiable; enforce it at the shell level, not
   per-app. (Privacy constraint 4)
7. **Every primitive carries a `variant_id` and declares its
   mutation axes.** GEPA may mutate declared axes; everything else
   is frozen by schema. Consent flows, Modal affordances, brand
   marks are never mutable. (foundations.md §"Active-radar loop")
8. **`CanonResponse` always carries topology / doppler / range /
   bearing.** ECC requires all four to reconstruct the echo;
   omission at the primitive layer is the hardest form of data loss
   to recover.
9. **Use `egui_dock` for the Mission Console shell and
   `egui_extras::StripBuilder` for bespoke dashboards; do not roll
   a dock system.** The dock is the single hardest retained-mode
   concern in an immediate-mode renderer; community solved it.
10. **Use `egui_plot` for `Plot`, `egui_term` for the terminal
    wrapper, `egui_extras::install_image_loaders` for `Media`.**
    Community-maintained, already well-shaped; no in-house fork
    unless we hit a proven ceiling.
11. **The `Chip` + `Gauge` + `Sheet` + `Stack` quartet is
    sufficient to re-express the whole boot / desktop / tray stack
    we already have in the spike** — demonstrating the canon is not
    strictly poorer than the hand-authored shell. The
    `shell/tray.rs` and `shell/desktop.rs` passes survive as *shell
    compositions over canon primitives*, not bespoke widgets.
12. **Demote `Tabs` and `Form` from primitives to compositions.**
    `blocks/tabs.rs` becomes `Stack + Select(single) + Sheet`; the
    ported forms become `Stack + Field… + Button`. This keeps the
    canon under 25.
13. **`egui::Id` is the canonical renderer-side handle; the
    ontology URI is the canonical substrate handle; the two are
    bidirectionally mappable via a single hashmap owned by the
    renderer-side adapter.** Agents address by URI; egui reasons
    by Id; the adapter hides the translation. (Predicate 1)
14. **AccessKit stays on by default (eframe enables it).** It is
    how voice-control and screen-readers attach without bespoke
    per-primitive code; Predicate 4 is otherwise far more expensive
    to satisfy.
15. **The `live.rs` poller is the reference architecture for the
    `Store`; keep it single-threaded tokio, keep RwLock snapshots,
    keep `try_send` back-pressure.** Extend it from "four RPCs"
    to "subscription manager" in the protocol pass, not in the
    primitive set pass.

---

## Open questions

1. **Persist format for `DockState`**. `egui_dock` serialises but
   our protocol wants the dock as a first-class substrate object —
   do we re-encode into a CBOR `Dock(tabs, splits)` and lose
   `egui_dock`'s internal fidelity, or ship their blob opaquely and
   re-layout on restore? Recommend the former; accept minor geometry
   drift.
2. **Modal affordance set — fixed or open?** Foundations.md forbids
   varying `Modal.affordances`, but some modals (share-sheet,
   attach) genuinely need extensible verbs. Proposal: the *safety*
   affordances (confirm / cancel / dismiss) are frozen; *additive*
   verbs can be appended by schema. ADR candidate.
3. **Voice-invoke grammar**. Predicate 4 says voice is first-class;
   but we haven't decided whether voice routes through AccessKit's
   labels or through an explicit phrase-map per primitive. AccessKit
   is the lower-effort path and probably correct; we should
   formalise before too many primitives ship.
4. **Variant-id allocation authority**. Is the variant-id minted by
   the substrate (GEPA layer) or by the renderer? If by the
   renderer, two users on a shared Mission Console can emit the
   same id for two different renders. Suggest substrate-minted
   with a per-frame handshake.
5. **Confidence interval encoding per primitive**. `Gauge` and
   `Plot` show a halo / band; what about `Field`, `Chip`, `Select`?
   The spike currently has no visual for confidence. Proposal: a
   single desaturated-stroke treatment with hover-for-interval text,
   uniform across all primitives that display an inferred value.
   ADR candidate.
6. **ForeignSurface interior signals**. How much CPU do we spend
   parsing ANSI / DOM / OCR frames? Foundations.md says "lossy by
   policy," which is the right default, but we haven't decided the
   sampling rates. Start at 1 Hz across the board; let ECC raise
   the rate on surfaces it finds informative.
7. **Does `Plot` really need to be a primitive, or is it a
   composition of `Sheet + Media(texture)` that happens to be
   GPU-accelerated?** Suggest yes, primitive — plots have a
   first-class semantic (time series with axes and units) that
   ontology agents interrogate directly, and reducing them to
   opaque textures loses that.
8. **Editor-sidecar embedding (RQ7 dependency).** The dev-panel
   surface is priority one; the egui-inside-webview path has
   unresolved performance and focus-stealing questions. This
   session proposes the `ForeignSurface(browser)` shell *inside
   egui*, but the inverse case — egui inside the editor's webview
   — is the one that actually ships first and needs its own ADR.
9. **Where does `Goal / Governance` surface live?** Foundations.md
   mandates project-scoped goals but the canon doesn't name a
   primitive for them. Proposal: goals are `Modal` (for
   authorisation) + `Chip` (for ambient presence) + `Sheet` (for
   the roster); no new primitive needed. Confirm before ADR.

---

## Sources

- [egui docs](https://docs.rs/egui/latest/egui/)
- [egui README + immediate-mode model](https://github.com/emilk/egui/blob/master/README.md)
- [egui::Memory — IdTypeMap and persistence](https://docs.rs/egui/latest/egui/struct.Memory.html)
- [egui::viewport — multi-window support](https://docs.rs/egui/latest/egui/viewport/index.html)
- [egui_extras — TableBuilder, StripBuilder, DatePicker, loaders](https://crates.io/crates/egui_extras)
- [egui_plot — 2D plotting](https://crates.io/crates/egui_plot)
- [egui_dock — docking](https://crates.io/crates/egui_dock)
- [egui_tracing — log/event integration](https://crates.io/crates/egui_tracing)
- [3rd-party egui crates wiki](https://github.com/emilk/egui/wiki/3rd-party-egui-crates)
- [egui_term — alacritty-backed terminal widget](https://github.com/Harzu/egui_term)
- [egui + GStreamer video player reference](https://github.com/ShabbirHasan1/egui-gstreamer-video-player)
- [wry — cross-platform webview for Rust](https://github.com/tauri-apps/wry)
- [tauri-egui — egui inside Tauri webview](https://v2.tauri.app/blog/tauri-egui-0-1/)
- [AccessKit — cross-platform a11y infrastructure](https://accesskit.dev/)
- [Unity UI Toolkit — UXML / USS retained-mode precedent](https://docs.unity3d.com/6000.2/Documentation/Manual/ui-systems/introduction-ui-toolkit.html)
- [Adaptive Cards — JSON UI schema + schema explorer](https://adaptivecards.io/explorer/)
- [Lynx by ByteDance — React-Native alternative architecture](https://lynxjs.org/) ([coverage](https://thenewstack.io/cross-platform-ui-framework-lynx-competes-with-react-native/))
- [Epic Slate — retained-mode declarative widgets via SNew](https://dev.epicgames.com/documentation/en-us/unreal-engine/slate-overview-for-unreal-engine)
- [Flutter architectural overview — widget / element / RenderObject trees](https://docs.flutter.dev/resources/architectural-overview)
- [React Native core components](https://reactnative.dev/docs/components-and-apis)
- [Xilem — data-oriented retained UI for Rust](https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html)
