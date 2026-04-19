# Foundations — Compositional UI for an Agentic OS

This is the short version. The AGENDA is long; this is the canon.

---

## The four predicates (every primitive must satisfy)

A primitive is a base UI element (Button, Panel, Form, Chip, Surface,
Field, Binding, Verb, Subscription). Before a primitive enters the
canon, it must be:

### 1. Ontology-addressable
Each primitive has a typed identity in the project's knowledge graph.
An agent queries the ontology to find the right primitive and its
configuration, it does not pattern-match on strings or guess. The
ontology is the ubiquitous language (Evans); the primitives are its
executable vocabulary.

### 2. Self-describing
Three things are first-class readable properties on every primitive,
not inferred from context:

- **State** — current value(s), dirty flags, validation status.
- **Affordances** — what operations the primitive supports right now,
  given its state and the caller's identity (read / write / subscribe
  / invoke / dismiss). Agents enumerate affordances; they do not
  attempt operations and hope.
- **Confidence** — where the primitive's displayed value came from
  (deterministic computation / model inference / last-known-cache /
  user input), its confidence interval if probabilistic, and its
  freshness.

### 3. Streaming-native
Partial values, progressive disclosure, out-of-order updates,
cancellation, and back-pressure are primitive-level concerns. A
primitive that only supports a single synchronous load-then-render
cycle is rejected. The protocol carries stream verbs (`open`, `chunk`,
`commit`, `cancel`) that every primitive understands.

### 4. Dual-operator
Every primitive is equally usable by:

- A human via pointer, keyboard, touch, or **voice** (voice is
  first-class, not an accessibility afterthought).
- An agent via `query` / `invoke` / `subscribe` against the primitive's
  ontology identity.

Neither path is a wrapper over the other. If a primitive only exposes
a human surface or only exposes an agent API, it's not a primitive
under this canon.

---

## Two tiers — primitives vs wrappers

Not every useful surface can satisfy all four predicates. A web
browser renders arbitrary HTML/JS that wasn't authored against our
ontology. A video decoder produces pixels the agent cannot
meaningfully interrogate. A legacy native egui/Tauri app runs in its
own process. We still want to host them.

The canon admits **two tiers**, and no more:

### Tier A — primitives
Satisfy **all four predicates**. These are the Lego blocks. Agents
reason about them, compose them, interrogate them. Buttons, Panels,
Forms, Fields, Chips, Gauges, Sparklines, Tables, Trees, Sheets,
Modals, Docks, Trays. This is where the real value is, and this is
what the protocol primarily describes.

### Tier B — wrappers (`ForeignSurface`)
A single primitive — `ForeignSurface` — is the canonical host for
opaque content. The **shell** around a foreign surface is a tier-A
primitive and **must** satisfy all four predicates:

- **Ontology-addressable** — the wrapper has a typed identity
  (`foreign://browser`, `foreign://video`, `foreign://xwindow`,
  `foreign://tauri-app/weftos-gui`) and a declared capability set
  (what the outer shell can do: position, focus, visibility,
  lifecycle, audio routing, clipboard access, URL navigation,
  screenshot/scrape, DOM accessibility tree if available).
- **Self-describing** — shell state (opened-at, last-URL, focus,
  size, exit-code, health) is queryable; inner state is declared
  opaque unless the wrapper type provides an introspection adapter
  (browser → DOM+a11y tree; terminal → ANSI stream; pdf → page+text).
- **Streaming-native** — lifecycle events (load-start, ready,
  navigation, crash, closed) stream on the shell's subscription.
- **Dual-operator** — the shell is driveable by both pointer/voice
  and agent (`navigate`, `focus`, `screenshot`, `terminate`,
  `inject-text`, `read-accessibility-tree`).

**Inner content is not fully exempt — it participates as signal.**
The agent cannot compose primitives *inside* a ForeignSurface, but
the surface's *existence, identity, state, and activity* are
first-class signals that feed the ontology and context layer. A
wrapper is reduced-fidelity participation, not withdrawal. Examples
of signals every wrapper emits:

- **Presence** — "a foreign surface of type X is active and in focus"
  is a context fact. Agents use it to decide interruption priority
  ("user is watching a video → defer audio notification", "user is
  in a terminal → they're probably hands-on-keyboard").
- **Identity hint** — the URI (`foreign://browser` + current URL,
  `foreign://video` + current media id, `foreign://app` + bundle id)
  is a coarse but meaningful ontology signal. "Browser on
  weftos-chain.com" is a stronger prior than "browser".
- **Activity** — input rate, scroll position, audio level, resource
  usage. The wrapper measures; the ontology ingests.
- **Outcomes** — navigation events, lifecycle transitions, exit
  codes, screenshots at checkpoints. These are ingestable into the
  knowledge graph the same way any other substrate event is.

So the rule is: **the interior is opaque to primitive composition,
but the surface still emits typed signals into the substrate.** The
four predicates apply *in full* to the wrapper shell and *as signal
channels* to the interior. A wrapper that refuses to emit presence /
identity / activity signals is not a conformant wrapper.

**Canonical wrapper types** to support from day one:

- `foreign://browser` — Chromium/WebView pane; affordances:
  `navigate`, `reload`, `screenshot`, `read-a11y-tree`,
  `execute-script` (governance-gated), `export-text`.
- `foreign://terminal` — pty host; affordances: `write`,
  `screenshot-buffer`, `tail-output`, `resize`, `kill`.
- `foreign://video` — media player pane; affordances: `play`,
  `pause`, `seek`, `set-track`, `subtitles`.
- `foreign://xwindow` — embed an X11/Wayland window (existing
  legacy app); affordances: `focus`, `resize`, `screenshot`,
  `terminate`.
- `foreign://app` — a nested WeftOS app (another egui process or
  Tauri binary); affordances: `launch`, `focus`, `terminate`,
  `send-message` (via the app's own protocol handshake).

**Design rule**: when a wrapper type earns enough usage, we may
promote specific inner behaviours to a *native* primitive (e.g. a
native Browser becomes a real tier-A `WebPane` that *does* speak
ontology). Promotion is a deliberate, ADR-worthy event. Wrapping is
the default; native is the reward for proving the pattern is worth
owning.

### Tier B as a woven signal tapestry — the user's causal collapse cascade

Wrappers *can* be read as mere conformance. That's fine, but it
massively understates them. What they really emit is a **thick,
woven tapestry of signals** — many threads running simultaneously
(presence, identity, attention, activity, outcome, co-occurrence
with other surfaces, timing, audio level, focus, scroll, query,
waveform, frame, caption, clipboard, transcript). The weft is
literally in the name of the system; it's the same metaphor. The
value of a wrapper is that it weaves *many* threads across the
substrate's warp at once, and no single thread tells the story —
the interference pattern does.

A user browsing, watching, speaking, and typing is performing the
behaviour their own intent is collapsing through in real time — the
**causal collapse cascade** (see `.planning/development_notes/
eml-causal-collapse-research.md` for the ECC treatment). Every URL
visited, every search typed mid-thought, every scroll stopped at,
every voice fragment, every frame the camera saw — these are the
raw threads of *how* a user's intent graph resolved into this
particular action right now. The ECC reads the weave, not any one
thread. That weave is what lets the system:

- **Expand the conversation graph** with content the user cared
  enough to look at.
- **Close loops** by reminding the user of something they were
  working on three browser tabs and two conversations ago.
- **Pull ideas and content quickly** — "that page you were reading
  yesterday about bonded-mesh peers" is a query the system can
  answer only if it observed.
- **Time interruptions well** — an agent that sees the user's
  attention state chooses when to speak.
- **Feed ECC** with the causal data it needs to model the user at
  all, given that people are far harder to understand than code.

For this reason, the ambient-capture wrappers are first-class:

- `foreign://camera` — camera frame stream; affordances:
  `enable`, `disable`, `snapshot`, `subscribe-tagged-frames`,
  `level` (audio level if paired mic).
- `foreign://mic` — mic stream; affordances: `enable`, `disable`,
  `subscribe-transcript`, `subscribe-waveform`.
- `foreign://screen` — screen-capture of the current surface;
  affordances: `snapshot`, `subscribe-diff-frames`,
  `subscribe-ocr-text`.

**Non-negotiable privacy constraints**:

1. **Per-user, never telemetry.** Observation data is the user's
   property and lives in their own chain-backed substrate. It is
   not uploaded, aggregated, or used for training off-node without
   explicit, revocable consent tracked on-chain.
2. **Governance-gated by default.** Every capture wrapper is gated
   by the constitutional governance layer; a user can require
   per-invocation consent, per-session consent, or standing
   consent with an audit trail.
3. **Signal lossy by policy.** What's emitted to the ontology is
   the *interpretation* (detected text, detected face, detected
   URL, detected application-in-focus), not the raw capture,
   unless the user explicitly requests a retained recording.
4. **Observable to the user.** A persistent, honest surface
   primitive (probably a tray chip) shows which captures are live
   right now. No dark recording, ever.

The observation stream is the system's most valuable asset **and**
the user's most personal dataset. Those two facts are the same fact;
the protocol must never let them diverge.

### Digital exhaust = intent

The OS interface is **the most data-rich surface for understanding a
human** that any system will ever get near. Every other channel is
downstream of it — transcripts are downstream of what was said while
doing something; repositories are downstream of what was typed at a
workspace; emails are downstream of what a user opened, read, and
decided to respond to. All of that lives *on the interface*. The
interface is the delta.

What flows off it is **digital exhaust**, and the critical recognition
is that *the exhaust is not metadata. The exhaust is intent.*
Traditional systems treat digital exhaust as telemetry — a by-product
to be sampled for aggregate analytics. For us it is the primary
product, because the exhaust of a single person working toward a goal
*is* the highest-resolution trace of that goal we will ever obtain.

This reframes the UI protocol. It is not primarily a display
protocol. It is **an intent-collection protocol that happens to
render pixels**. That inversion is what makes this system different
from every OS that preceded it, and it is the lens every primitive,
wrapper, signal, and ADR must be read through.

Everything in the Non-negotiable privacy constraints above follows
directly: if digital exhaust is intent, then surrendering it without
consent is surrendering *the user's intent itself*, which is a
harm of a different order than surrendering a click-stream. The
protocol must protect it in proportion.

**Anti-rule**: we do not allow Tier-C. No "halfway primitives" that
satisfy three of the four predicates. Either full citizen (Tier A),
or opaque guest behind a compliant shell (Tier B). This keeps the
surface protocol small and legible to agents.

---

## The reverse-DDD arrow

Classical DDD runs `domain → model → code → UI`.

We run `substrate state + ontology → schema → composition → renderer`.

The flip happens at the UI step: the UI is not the output of
modelling, it is a *view* into a modelled substrate. The primitives
above are the legal views; the protocol describes compositions of
those views; the renderer (currently egui) is disposable.

---

## Rasmus four-layer crosswalk (today → tomorrow)

| Rasmus layer | WeftOS today | What the symposium must add |
|-|-|-|
| Semantic foundation (vector + KG) | AgentDB, HNSW indexing, graphify, ontology-navigator symposium | Make it the *default addressing mode* for every primitive. |
| Probabilistic kernel (intent under uncertainty) | Gate backend, governance counsel, ECC scoring | Protocol-level expression of confidence + delegation + interruption priority. |
| Model control plane (context-window as working memory) | Agentic-flow, ReasoningBank, EWC++ | Bind working memory to *surfaces*, not just agents. |
| Specialized agents (janitor / gatekeeper / archivist / strategist) | 60+ agent types, weaver daemon | Express these as surface-composable roles with typed affordances. |

## Rasmus six unsolved problems crosswalk (our answers)

| Problem | Our current answer | Where it's weak |
|-|-|-|
| Identity + delegation | ExoChain + dual signing + RVF crypto + governance gates + claims authorization | Delegation across org boundaries (cross-substrate identity) is unspecified. |
| Asynchronous execution | Mesh runtime + HeartbeatTracker + MeshClockSync; ECC for priority | Interruption policy per-surface is not yet a primitive. |
| Multi-agent protocols | MCP + A2A router + agentic-payments + claims | No surface-composition wire format exists yet — **this symposium's deliverable.** |
| Economics | DeFi layer + bond staking + trust progression + slashing | No payment allocation model for orchestration-as-value. |
| Training data lock-in | ECC (causal models, not user profiles) + trajectory portability | Export / portability spec not yet written. |
| Serendipity vs. efficiency | ECC hypothesis generation + weaver HYPOTHESIZE loop | Surface-level "introduce productive disruption" affordance not yet a primitive. |

**We are further ahead than Rasmus realises.** The symposium's job is
to make this visible by fitting a surface protocol on top that the
industry can read.

---

## Non-negotiable rejections

These are rejected at the ADR stage, no discussion:

- Framework lock-in (React, Tailwind, Next.js, Tauri as a hard
  requirement).
- Color as primary identity at the base layer — later, yes; base, no.
- Hand-designed screens (bespoke layout code per view is an
  architectural failure).
- CLI-shaped chat as the dev-panel primary surface.
- **Tier-C "halfway primitives"** that satisfy three of the four
  predicates. All four (Tier A), or opaque behind a compliant shell
  (Tier B). Nothing in between.

---

## Mandatory reading for every participant

1. Rasmus, *The Agentic Operating System* — the thesis we are
   executing, and whose unsolved problems we already partly answer.
2. Cooper, *About Face* — posture taxonomy, goal-directed design,
   behavioural/interface split.
3. Evans, *Domain-Driven Design* — ubiquitous language, bounded
   contexts, aggregates, context maps — **read with the arrow
   reversed**.
4. Mei et al., *AIOS: LLM Agent Operating System* (arXiv 2403.16971)
   and `agiresearch/AIOS` — academic prior art, crosswalk our kernel.
5. `CedarCopilot/cedar-OS` — abandoned but instructive primitives
   (state-registration, semantic mentions, spells, shadcn ownership
   model).
