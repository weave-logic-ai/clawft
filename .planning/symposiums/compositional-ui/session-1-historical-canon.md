# Session 1 — Historical Canon (RQ1)

**Scope**: Teardown of nine historical protocol-defined UI systems, read
against the four predicates (ontology-addressable, self-describing,
streaming-native, dual-operator), the tier-A/B split (primitives vs
`ForeignSurface`), the active-radar loop (display=pulse, return=echo,
ECC=receiver, GEPA=learn), and the digital-exhaust=intent framing.
Cites Cooper's posture taxonomy where applicable. No system on this
list satisfies all four predicates — that is the point. Our job is to
harvest the portable bits and name the lock-ins so we do not repeat
them.

Out of scope: modern agentic UIs (RQ2), multiplayer/CRDT systems (RQ3),
ontology→UI synthesis (RQ4), renderer contracts (RQ5), protocol wire
format (RQ6), dev-panel embedding (RQ7), governance binding (RQ8).

---

## Per-system teardown

### 1. X11 core protocol (1984)

A client/server byte protocol with four wire message types —
**Request, Reply, Event, Error** — plus an extension mechanism
(RandR, Composite, XInput2, etc.). The server owns the frame buffer;
clients own geometry trees (Windows) and drawing primitives (GC, font,
pixmap).

- **Portable**: the request/reply/event/error quadrant is still the
  cleanest minimal wire vocabulary ever written for a UI. It is the
  direct ancestor of LSP and MCP. A pixmap is an **opaque
  server-resident surface referenced by an XID** — this is precisely
  our `ForeignSurface` concept 40 years early.
- **Locked-in**: no semantic layer whatsoever. A Window is a rectangle
  with a parent; the server has no idea whether it is a Button, a
  Panel, or a game viewport. Accessibility and automation had to be
  bolted on later (AT-SPI, xdotool scraping) because the protocol has
  no affordance vocabulary.
- **Keep**: the four-message quadrant; the idea that a surface is a
  typed server-side resource with a handle; extension mechanism that
  lets capabilities grow without breaking the core.
- **Reject**: drawing primitives in the core (lines, arcs, fills); no
  streaming model (events are fire-and-forget, no
  `open/chunk/commit/cancel`); no self-description — the protocol does
  not let a client ask a window "what operations do you accept right
  now?". Cooper posture: X11 is **postureless** — it renders whatever
  clients push; this is exactly what we reject when we say "no
  hand-designed screens".

### 2. Wayland (2008–)

A successor protocol where the **compositor is the protocol host**.
Clients render their own pixel buffers (typically via GPU) and hand
shared-memory or DMA-BUF handles to the compositor; the compositor
composites and schedules frames. Extensions are negotiated per-client
via `wl_registry`.

- **Portable**: compositor-as-protocol-host is the right topology for
  a kernel-owned surface layer. Capability negotiation at connection
  time (client asks "what globals do you have?", compositor answers)
  is a cleaner pattern than X11's implicit atoms. Frame callbacks are
  a primitive back-pressure signal.
- **Regressed vs X11**: network transparency lost; screen-grab and
  input-synthesis (critical for **dual-operator** and for the active-
  radar loop's observation side) fragmented into vendor-specific
  extensions (`wlr-screencopy`, `xdg-desktop-portal`, Pipewire). This
  is a cautionary tale: **Wayland optimised for the common case
  (single user, direct manipulation) and made agent-driven use
  progressively harder**. We must refuse that trade.
- **Keep**: compositor-owned surface ownership; capability handshake;
  per-client extension negotiation; frame callbacks as back-pressure.
- **Reject**: vendor-split capability surface (ours must be uniform
  across renderers); opt-out accessibility (every primitive must be
  fully agent-addressable by default, not via a special portal).

### 3. NeWS / Display PostScript (1986–1990)

Sun's Network Extensible Window System sent **PostScript code** over
the wire; the server executed it to produce both drawing and UI
behaviour. Display PostScript (NeXTSTEP) was the single-vendor cousin.
This is the most *semantically alive* prior art on the list: the wire
carried a program, not a pixmap.

- **Portable**: the "wire carries a language, not a bitmap" idea is
  philosophically aligned with us. An agent shipping a *composition*
  (Panel > Form > {Field, Button}) rather than a bitmap is exactly
  what we want. NeWS also let the server run event handlers locally —
  a posture-independent reactivity model.
- **Why it lost**: (a) PostScript is Turing-complete — no introspection,
  no affordance discovery, no sandboxing; (b) X was free, NeWS was
  Sun's; (c) debugging a scene you cannot query is miserable. **The
  self-describing predicate is the lesson** — sending code is powerful
  only if the runtime lets you ask the running scene what it is.
- **Keep**: the semantic-wire instinct; server-side event handling
  colocated with surface state.
- **Reject**: a Turing-complete wire without an introspection
  requirement. Our protocol must carry **declarative composition +
  typed affordances**, never opaque code. If an agent wants behaviour,
  it composes primitives whose affordances are enumerable.

### 4. Morphic (Self, 1995; later Squeak, Pharo)

Every visible thing is a **Morph** — a live object with its own
drawing, event handling, submorph tree, and halo of meta-operations
(resize, inspect, duplicate, persist). "Liveness" and "direct
manipulation" are architectural, not stylistic: every Morph is
inspectable and editable in place.

- **Portable**: the Morph-is-a-live-object idea is the correct
  philosophical stance for our primitives. A Morph satisfies
  **three** of our four predicates within the Smalltalk image:
  self-describing (halo), streaming-ish (event-driven), dual-operator
  (both human drag and programmatic `addMorph:`). Only
  ontology-addressable is weak — identity is object identity, not a
  typed URI.
- **Why it never left Smalltalk**: the image was the world. Morphs
  were not serialisable across runtimes in any honest way; there was
  no wire protocol. This is the Smalltalk tragedy in miniature — a
  better model, imprisoned.
- **Cooper posture**: Morphic environments are **sovereign-posture**
  (Self/Squeak is your world for hours; it owns the screen; it
  rewards expertise). This is fine for developers in our dev-panel
  mode; it is wrong for Mission Console, which must be transient.
- **Keep**: halo-style meta-affordances (inspect, persist, duplicate)
  as a standard primitive operation set; liveness-by-default;
  direct-manipulation as a tier-A invariant.
- **Reject**: in-image identity; non-portable object graphs. Our
  primitives' identities live in the ontology, not in any one
  process's heap.

### 5. Naked Objects (Pawson, 2002)

**The closest prior art to our reverse-DDD arrow.** The thesis: the
domain model *is* the UI. Given an entity with properties and
methods, the framework auto-renders forms, lists, actions, and
navigation. The human sees the domain directly; the designer's job is
to get the model right, not to paint screens.

- **Portable**: this is literally our thesis with the agent-
  composition piece missing. Pawson's "behaviourally complete"
  objects (every legal operation is a callable method on the object)
  is *exactly* the self-describing affordance requirement. Entity →
  form; collection → table; action → button: these are ontology-to-
  layout rules we will write.
- **Why it stalled commercially**: (a) uniform auto-rendering
  produced visually flat, identical-looking screens that felt
  bureaucratic — Cooper would say it ignored **posture** and
  **persona**; (b) no streaming, no partial views, no confidence —
  purely synchronous CRUD; (c) enterprise Java ecosystem locked the
  framework in.
- **Cooper posture**: Naked Objects produced **daemonic/parasitic**
  screens even when the task was sovereign, because it had no
  posture vocabulary. Our protocol must carry posture as a hint
  (matching AGENDA's Desktop vs Mission Console split).
- **Keep**: entity-drives-surface; behavioural completeness = agent-
  enumerable affordances; action-is-a-method-on-the-domain.
- **Reject**: one-to-one mapping without a composition layer; lack
  of streaming and confidence; no variant-id/GEPA story (every user
  got the same screen, so the system could not learn which variant
  helped).

### 6. AT-SPI (Assistive Technology Service Provider Interface, 2001–)

A **secondary, semantic UI tree** maintained in parallel with the
pixel tree. Every widget (in GTK, Qt, WebKit, etc.) publishes role
(`button`, `textbox`, `heading`), name, value, states, and relations
over D-Bus. Screen readers, voice control, and UI automation all
consume it.

- **Positive prior art for the dual-operator predicate.** This is the
  industry already conceding that an accessibility tree is a *second
  operator channel*. We take it further: the dual-operator predicate
  says **there is no primary and secondary** — the agent channel is
  equal to the pointer channel.
- **Portable**: the role/state/relations vocabulary (ARIA inherits
  from it) is a decent minimum typed-affordance set. The fact that
  it scales across GTK, Qt, browsers, Electron, and the JVM proves
  a single semantic tree can live above diverse renderers — which
  is *exactly* what we need with egui today and a second renderer
  tomorrow.
- **Locked-in / broken**: (a) it is almost always a **mirror**
  reconstructed from the real widget tree, which drifts, lies, and
  is full of `generic` roles in practice; (b) it is opt-in per
  widget author, so coverage is patchy; (c) no streaming, no
  confidence, no variant-id. Making accessibility the afterthought
  was the original sin — the primary and secondary channels
  diverge because only one is load-bearing.
- **Keep**: role + state + relations as the vocabulary core; the
  fact that it demonstrably scales across renderers.
- **Reject**: mirror-after-the-fact architecture. Our primitives emit
  the semantic tree **as the source of truth**; the pixel tree is
  derived from it, not vice versa. (This inverts AT-SPI; it is the
  reverse-DDD arrow applied to a11y.)

### 7. NSDocument / OLE Automation / COM (1989–1995)

Platform-scoped component models. **OLE/COM** exposed in-process and
out-of-process components with typed interfaces (IDispatch, IUnknown)
and a registry-based discovery system. **NSDocument** on NeXTSTEP/Cocoa
gave a uniform document lifecycle (open/save/revert/undo). Both were
serious attempts at a cross-application component protocol.

- **Portable**: the lesson that **capability discovery must be typed
  and enumerable** — `QueryInterface` is ugly but honest. The idea
  that compound documents (an Excel chart inside a Word doc) need a
  protocol, not bespoke glue per pair, prefigures our `ForeignSurface`
  for nested Tauri/browser/video.
- **Locked-in**: Windows-only; registry-based identity (GUIDs in a
  machine-local store) does not survive migration, duplication, or a
  distributed substrate; IDispatch's late-binding `Invoke(DISPID, …)`
  is string-keyed in practice and re-invented every automation pain.
  OLE's compound-document UX famously failed — users could not tell
  whose toolbars were whose.
- **Keep**: typed interface discovery (QueryInterface-as-pattern);
  lifecycle as a first-class contract (NSDocument's
  open/save/revert/undo is tier-A-primitive-worthy).
- **Reject**: machine-local identity (ours is chain-backed and
  substrate-global); string-keyed invocation; UI-toolbar-merging as
  the composition model (our composition is agent-driven and
  ontology-queried, not "whose frontmost widget wins").

### 8. RDP / VNC / X11-forwarding (1985–1998)

**Remote renderer, central state.** VNC ships framebuffer deltas;
X11-forwarding ships the X protocol over SSH; RDP ships a richer,
semantic stream (window boundaries, clipboard, printer redirection,
audio). All three separate "where state lives" from "where it is
displayed".

- **Directly relevant**: Mission Console is a remote-renderer
  surface. Several humans, one task, shared substrate state, many
  renderers. This topology is the ancestor.
- **RDP vs VNC is the instructive comparison**: VNC ships pixels and
  loses everything semantic — zoom breaks, copy/paste is brittle,
  text is not text. RDP preserves semantics (windows, text, clipboard)
  and scales better on thin links. **Pixel-level remoting is the wrong
  abstraction**; protocol-level remoting is the right one. Our wire
  must ship composition, not bitmaps.
- **Keep**: multi-renderer / single-state topology; clipboard, audio,
  and input routing as **typed side-channels**, not free-for-all
  streams; latency budgets as first-class protocol concerns.
- **Reject**: framebuffer remoting; "a session is a window tree"
  framing (our session is a task with a goal — governance layer, not
  window tree).

### 9. Adaptive Cards (Microsoft, 2017–)

A JSON schema for UI fragments (containers, text blocks, inputs,
action buttons) rendered by host-specific renderers across Teams,
Outlook, Bot Framework, Windows notifications, etc. This is the
**closest modern analog to a surface wire protocol** on the list.

- **Portable**: JSON-declared, renderer-agnostic, versioned schema
  with host capabilities (a host advertises what elements and
  features it supports). Template Language separates data from
  structure. Action model (`Action.Submit`, `Action.OpenUrl`,
  `Action.Execute`) is a small, typed affordance set. **This is
  exactly the shape of wire we want.**
- **Missing**: (a) no streaming — cards are rendered once, not
  progressively; (b) no confidence, no variant-id, no return-signal
  hooks — the pulse/echo loop is impossible; (c) ontology is flat
  (string `id`s), not typed URIs in a KG; (d) no agent-enumerable
  affordance discovery at runtime, only at schema-compile time;
  (e) dual-operator is partial — accessibility is decent, but agent
  programmatic drive of a rendered card is not in the spec.
- **Cooper posture**: Adaptive Cards shine for **transient** and
  **auxiliary** postures (a notification, a bot reply). They break
  down for sovereign surfaces — which is fine; our Mission Console
  is not a card, it is a long-lived composition of many
  primitives.
- **Keep**: host-capability handshake; versioned schema; typed
  action model; template/data separation; the basic shape of the
  wire (JSON/CBOR declarative fragments addressed to renderers
  that advertise capabilities).
- **Reject**: one-shot render model; flat string identity; lack of
  streaming, confidence, variant-id. We are essentially **Adaptive
  Cards with the four predicates enforced and the active-radar loop
  wired in.**

---

## Recommendations for canon

1. **Adopt the X11 four-message quadrant (Request/Reply/Event/Error)
   as the wire skeleton, extended with stream verbs
   (`open/chunk/commit/cancel`).** This satisfies the streaming-native
   predicate at the transport layer and inherits 40 years of proven
   minimalism. Events alone (Wayland-style) are insufficient; we need
   the request/reply pair for affordance enumeration and the stream
   verbs for progressive disclosure. *Predicate: streaming-native.*

2. **Mandate a capability handshake at connection time (Wayland/
   Adaptive Cards pattern) negotiated against the ontology.** The
   renderer advertises which primitive types it can render and at
   what fidelity; the compositor advertises which `ForeignSurface`
   types it hosts. No vendor extensions — any new capability goes
   through ADR and enters the ontology as a typed term.
   *Predicate: ontology-addressable. Tier: A and B.*

3. **Every primitive has a typed URI identity in the KG; no string
   `id`s, no GUIDs.** Reject OLE/AT-SPI mirror models, reject
   Adaptive Cards' flat ids. An identity is `primitive://form/
   customer-edit@v3` resolvable through the ontology, with affordance
   set declared there and verified at render time.
   *Predicate: ontology-addressable.*

4. **Semantic tree is the source of truth; pixel tree is derived.
   Invert AT-SPI.** The dual-operator predicate is not satisfied by
   shipping a parallel accessibility mirror; it is satisfied by the
   renderer consuming the same semantic tree the agent queries.
   Pointer, voice, keyboard, and `query/invoke/subscribe` all drive
   the same tree. *Predicate: dual-operator.*

5. **Declarative composition only on the wire; no Turing-complete
   code (NeWS's fatal choice).** The wire carries ontology-typed
   compositions of primitives with typed affordances. Behaviour is
   expressed by choosing primitives and wiring their subscriptions,
   never by sending a program. This keeps scenes agent-queryable and
   sandbox-trivial. *Predicate: self-describing.*

6. **Adopt Morphic's halo as a universal meta-affordance set on every
   primitive.** Minimum: `inspect` (dump state + affordances +
   confidence), `subscribe` (open a stream), `explain` (why am I
   showing this value, from which source, with what freshness),
   `persist` (snapshot to substrate), `dismiss`. These are the agent's
   reflection API and the human's right-click menu — same operations,
   dual-operator by construction. *Predicate: self-describing, dual-
   operator.*

7. **Adopt Naked Objects' entity-drives-surface rule as the default
   composition strategy, but carry posture as an explicit ontology
   hint on every composition request (Cooper).** Desktop surfaces are
   sovereign by default; Mission Console ephemeral compositions are
   transient; notifications are auxiliary. The renderer picks density,
   focus behaviour, and dismissibility off posture. This fixes the
   "bureaucratic uniform screens" failure mode.
   *Predicate: ontology-addressable. Posture: Cooper.*

8. **Every render carries a `variant-id` and a declared
   `mutation-schema`; every interaction echo carries enough to
   reconstruct topology, doppler, range, bearing (the active-radar
   loop).** No historical system on this list did this. Adaptive Cards
   comes closest but gives up at render time. This is the single
   biggest *new* requirement on the wire and it is where our system
   stops resembling any of its ancestors. *Predicate: streaming-
   native + the active-radar loop; GEPA-legible by design.*

9. **`ForeignSurface` is the only opaque-content primitive (X11
   pixmap + RDP-not-VNC lessons).** Semantic remoting beats pixel
   remoting every time; even for an opaque guest, the shell is tier-A
   and emits the presence/identity/activity/outcome signal tapestry.
   Capture wrappers (`foreign://camera|mic|screen`) are first-class
   and governance-gated on every invocation. Inner interiors are
   opaque to composition but participate as signal. *Tier B as
   canonised in foundations.*

10. **Lifecycle is a first-class contract, not an app concern
    (NSDocument's gift). Every primitive and every `ForeignSurface`
    shell declares `open → ready → active → idle → closing → closed`
    with streamed transitions, plus `undo`/`revert` affordances
    where the underlying substrate supports them.** This makes
    context-switch, interruption policy, and resume-across-sessions
    a wire concern, not per-app. *Predicate: streaming-native + self-
    describing.*

---

## Open questions

- **Should composition itself be a first-class primitive (a
  `Composition` node the agent can hand another agent) or just a
  transient message?** Morphic says yes (the Morph is the unit of
  persistence); Adaptive Cards says no (the card is immutable
  payload). We probably want the Morphic stance so compositions
  can be snapshotted to the substrate — but that collides with
  variant-id/GEPA, which wants compositions to be cheap and
  regenerable. Resolve in a later session, likely RQ6 / ADR.

- **How do we reconcile RDP-style multi-renderer sessions with
  variant-id?** Two humans in the same Mission Console viewing the
  same composition: do they see the same GEPA variant or different
  ones? Same variant keeps the shared-attention invariant (they can
  point at the same thing); different variants give twice the
  learning signal. Likely answer: shared-variant in Mission
  Console, free mutation in solo surfaces — but this is an ADR.

- **Is `posture` an ontology term, a composition hint, or a renderer
  choice?** Cooper's taxonomy is sharp but was written before agents
  composed UIs. An agent should be able to *reason* about posture
  (an ontology term), a compositor should be able to *override* it
  under governance (a composition hint), and a renderer should be
  able to refuse it (density limits on a watch vs a wall). All three
  probably true; exact layering is open.

- **Do we inherit anything from the NeWS server-side event-handler
  idea, or is that fully replaced by ontology-addressed
  subscriptions plus agent-in-the-loop reactivity?** There is a
  real latency argument for colocating some event handling with the
  renderer (e.g. keystroke-level autocompletion). Needs a latency-
  budget ADR.

- **What's the minimal conformance a non-egui renderer must meet to
  claim protocol compliance?** AT-SPI's mistake was making
  conformance optional per-widget. Adaptive Cards' answer is a
  host-capability matrix. We should pick the stricter stance and
  publish a conformance test suite as part of the protocol spec (see
  RQ6). This is the success-criteria bar from AGENDA.
