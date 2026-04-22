# ADR-019: Input Modality Adaptation, `foreign://avatar`, and the Narration Contract

**Date**: 2026-04-20
**Status**: Proposed — symposium round 3
**Deciders**: Compositional UI Symposium (Session 10 — Modes, Inputs, and the App Layer)

**Amendment note (2026-04-20 — amends ADR-006)**: This ADR amends
ADR-006's `affordance` schema with one new field, `invocations`,
declaring which input channels may trigger the affordance. The field
has a defaulted value if absent (`["pointer", "touch"]`) so existing
manifests parse unchanged. The amended CDDL is reproduced verbatim in
§B.1. Renderers must treat an attempt to invoke an affordance over a
channel not in its realised `invocations` list as **malformed at the
kernel boundary** — same rejection class as ADR-006's missing
`privacy-flags`. No wire break; the default covers the common case.

## Context

Session 10 established that **input modality** (pointer / touch / voice
/ hybrid) is orthogonal to **mode** (single-app / desktop / ide) and to
**app identity** (ADR-015). Three things break if input is a
per-primitive concern or a renderer convention:

1. **Hit-target drift.** A surface authored on a desktop at 24 px tap
   targets ships to a kiosk and produces 60 % miss-taps. A per-session
   floor fixes this by construction; per-primitive guidance does not
   scale.
2. **Voice invocation is a privilege**, not a policy. Session 9 negative
   lesson 2 (Cedar-OS) plus ADR-006's self-describing predicate forbid
   acting on a surface via a channel it hasn't advertised. A fuzzy-
   matching voice daemon that escalates pointer-only affordances to
   voice-reachable silently violates predicate 2.
3. **Avatar companions are apps**, not a fourth mode. The `single-app
   × voice` combination recurs (Session 10 §2.3 "avatar" cell) and
   requires: a character render surface (Tier-B), a captions
   `StreamView` (Tier-A), a narration template language, and — per
   ADR-012 — a non-dismissible capture chip whenever mic/camera is
   live. None of this is exotic; it all needs specifying.

All three are specified here because they mutually constrain each
other: the narration contract is the *content* an avatar speaks, the
avatar is the *surface* that speaks it, voice invocation is the
*channel* that gives the user back agency over what was spoken.

## Decision

Three sub-decisions, independently specified, composable at runtime.
Part A defines the input-modality axis as a session-level value. Part
B amends ADR-006 to make voice-invocability a first-class affordance
property. Part C adds `foreign://avatar` and the narration contract.

---

### Part A — Input modality axis

#### A.1 Axis values, lifetime, and default detection

```cddl
session-input = "pointer" | "touch" | "voice" | "hybrid"
```

**Fixed at session launch**, with **one exception**: a session
launched with `"pointer"` auto-upgrades to `"hybrid"` on the first
touch event observed by the compositor. The upgrade is one-way (no
downgrade); every other transition requires relaunch.

Default detection when the host does not set `input` explicitly:

| Host signal | Detected |
|-------------|----------|
| `(pointer: coarse)` (no fine pointer) | `touch` |
| `(pointer: fine)` + `(hover: hover)` | `pointer` |
| Voice daemon present + wake-word active | `voice` |
| Mixed / ambiguous | `hybrid` |

Detection runs once, at session launch; the session value is carried
on every `surface.compose` / `surface.update` frame so composers adapt
without re-deriving it.

#### A.2 Compositor-level adaptation

The compositor consults `session.input` **every frame** and applies
the following rules. **Primitives do not know their own input mode**
(Session 10 rec. 9); they receive adapted `Sense`, hit regions, and
gesture routings from the compositor layer.

**Hit-target floor.** The Canon trait gains one method:

```rust
trait CanonWidget {
    /// Minimum interactive bounding box. Compositor clamps the allocated
    /// rect up to this size when `session.input != pointer`.
    fn min_hit_size(&self) -> egui::Vec2 { egui::vec2(24.0, 24.0) }
}
```

When `session.input ∈ {touch, voice, hybrid}`, the compositor clamps
each primitive's interactive rect to at least `max(min_hit_size(),
vec2(44., 44.))`. `CanonWidget::show` must honour the clamped size;
this is a structural contract. Voice mode keeps touch-sized hit
regions because voice sessions still need touch fallbacks for explicit
confirmation gestures ("confirm rollback" as a literal tap).

**Hover semantics.**

| Input | Tooltip trigger |
|-------|-----------------|
| `pointer` | hover |
| `touch` | long-press (≥ 500 ms) |
| `voice` | narrated on utterance `"what is this"` — focuses the highlighted primitive's ADR-006 §7 `tooltip` |
| `hybrid` | hover if a pointer cursor is present in the frame; long-press otherwise |

**Gesture mapping.**

| Intent | Pointer | Touch / Hybrid | Voice |
|--------|---------|----------------|-------|
| `click` | click | tap | "activate X" |
| `right-click` | right-click | long-press | "show options for X" |
| `drag` (pannable) | click-drag | one-finger drag | n/a |
| `swipe` (scrollable) | scroll wheel | one-finger swipe | "scroll up / down" |
| `scroll / zoom` (pinchable) | wheel + modifier | two-finger scroll + pinch | "zoom in / out" |

**Chrome adaptation per (mode × input).**

- `desktop × touch` — tray collapses to bottom bar; launcher becomes
  a bottom-anchored drawer; floating windows auto-maximise (user can
  manually unmaximise).
- `single-app × touch` — primary affordances dock to the bottom
  third (thumb-reachable); app body unchanged.
- `ide × touch` — **respects the host layout**. No auto-maximise (it
  would clobber the editor pane). Tray collapses as in `desktop ×
  touch`, but windowing defers to the host frame.
- `* × voice` — launcher becomes voice-addressable (Part C §C.4);
  tray chip retains non-dismissible presence per ADR-012.

**Safe-area insets.** Compositor reads `ctx.input(|i| i.viewport())`
for the safe-area rect (iOS notches, Android nav bars, foldable
hinges), pads the canvas rect inward before any primitive is
allocated. Primitives see the already-padded rect.

**IME awareness.** When a virtual keyboard is up, the viewport rect
reports reduced height. Modal / floating / tool surfaces (ADR-014)
re-flow against the reduced rect; toasts rebase to the top of the
reduced viewport. This is a compositor concern; primitives receive no
explicit "IME up" signal.

---

### Part B — Voice affordance invocation (amends ADR-006)

#### B.1 Amended schema

```cddl
affordance = {
  name        : tstr,
  verb        : tstr,                    ; WSP verb (ADR-005)
  ? actors    : [+ actor-kind],          ; "human" | "agent" | "any" — default "any"
  invocations : [+ invocation-kind],     ; NEW — ADR-019. Default ["pointer", "touch"] if absent.
  ? args-schema : ontology-uri,
  ? reorderable : bool,                  ; ADR-006 2026-04-19 amendment
  ? utterances  : [+ tstr],              ; NEW — ADR-019 §B.4; optional voice aliases
}

invocation-kind = "pointer" | "touch" | "voice" | "gesture"
```

Rules, enforced at the kernel boundary:

1. **`invocations` is non-empty.** An affordance invocable by no
   channel is malformed — that case is expressed by omitting the
   affordance, not by emitting one with an empty list.
2. **Default.** If `invocations` is absent, the kernel substitutes
   `["pointer", "touch"]` before the composer sees the frame. This
   keeps the amendment wire-compatible with pre-ADR-019 manifests:
   every legacy affordance remains pointer- and touch-invocable.
3. **Channel rejection is structural.** Invoking via a channel not in
   the realised list is a malformed action frame — kernel refuses
   with `deny{reason: "malformed:invocation-channel-not-declared",
   affordance, channel}`; renderer never sees the attempt.
4. **`gesture` is distinct from `touch`.** `"touch"` covers tap,
   long-press, one-finger drag. `"gesture"` covers authored
   multi-finger canvas gestures (pinch, two-finger rotate,
   three-finger swipe) per ADR-013. A pannable canvas must list both
   to accept drag *and* pinch-zoom.

#### B.2 Rationale — structural, not policy

Parallel to ADR-012's privacy-flag argument. If voice invocability
were a daemon convention ("refuse utterances that don't match an
affordance name"), two failure modes open:

- **Homophone matching.** The daemon hears "restart service", finds
  `restart-service`, invokes it — but the authoring agent never
  declared voice as an acceptable channel. The invocation succeeds;
  self-description is silently violated.
- **Fuzzy-match drift.** The daemon grammar evolves independently of
  manifests. Previously-out-of-reach affordances become reachable
  because phoneme extraction improved. Predicate 2 fails.

With `invocations` at head level, the kernel rejects a voice attempt
on a pointer-only affordance *before* the fuzzy-matcher runs. The
user hears a polite refusal (narrated by the avatar per §B.3); the
daemon's internals are irrelevant.

#### B.3 Narration of rejection

On rejection in voice mode, the compositor emits a
`ui://modal{modality="toast"}` (ADR-014) with text *"That action
can't be triggered by voice."* The avatar surface (Part C) subscribes
to `substrate/voice/rejections` and narrates the toast. The user
hears *why* their utterance had no effect rather than silence.

#### B.4 Grammar matching — canonical name + `utterances`

Matching uses:

1. **Canonical `name`** split on `-` / `_` / camelCase boundaries.
   `restart-service` matches `"restart service"`, `"restart the
   service"`, `"reset service"` via a small synonym table. Mechanical;
   no ML.
2. **Optional `utterances` array** — caller-declared aliases. If
   present, matching uses `utterances ∪ {canonical-variants}`. If
   absent, only canonical variants.

```toml
[[node.affordances]]
name = "rollback"
verb = "deploy.rollback"
invocations = ["pointer", "touch", "voice"]
utterances = ["roll back", "undo the last deploy", "revert production"]
```

Deliberately small — no Levenshtein, no phonetic matching, no ML.
Authoring agents enumerate realistic utterances; if one is missing,
the user falls back to the canonical name. Grammar evolution is a
manifest concern, not a runtime concern.

---

### Part C — `foreign://avatar` and the narration contract

#### C.1 `foreign://avatar` — new Tier-B ForeignSurface kind

Extends ADR-001 row 20 (`ui://foreign`, Tier-B) with `kind =
"avatar"`. The **shell is Tier-A** (`egui::Frame` hosting an opaque
character render). The character render is the Tier-B payload —
implementers ship Unity, Live2D, Ready Player Me, WebGL shaders, or
pre-rendered animation loops; the shell does not inspect the payload.

**Shell → character signals** (fixed set, emitted over the
foreign-surface bridge):

| Signal | Fires when |
|--------|-----------|
| `ready` | avatar allocated, character render attached |
| `speaking-start` / `speaking-end` | TTS stream begins / ends |
| `listening-start` / `listening-end` | mic capture begins / ends |
| `gesture-detected` | caller affirmed a user gesture (nod, point, wave) |
| `gaze-direction` | where the user is looking, if eye-tracking is provided |

The character render is free to interpret these (blink rate,
lip-sync, idle animation); the shell is responsible for *when* they
fire and for maintaining the tray-chip invariant.

**Capture-chip invariant (ADR-012 binding).** Any avatar surface
whose implementation uses mic or camera **must** render the
persistent capture chip in the shell tray — same non-negotiable
obligation as `foreign://{camera, mic, screen}`. The avatar does
**not** get an exception for being "friendly". A manifest declaring
an avatar surface with mic/camera permissions but no adapter that
publishes the tray chip is rejected at install time.

#### C.2 Narration contract — per-app declaration

Any app whose `supported_inputs` include `voice` may declare a
`[narration]` table in its manifest (ADR-015 §(8)). The table is
keyed by subscribed topic path (must appear in `subscriptions`);
values are **speakable templates**:

```toml
[narration]
"substrate/admin/health" = """
{{count(services, s -> s.status = "healthy")}} services healthy,
{{count(services, s -> s.status = "at_risk")}} at risk.
"""
```

**Template grammar.** Text outside `{{ ... }}` emits verbatim
(whitespace normalised). Inside `{{ ... }}` is a **binding expression
in the ADR-016 §5 grammar** — the same small statically-typed,
side-effect-free expression language used for surface-description
data bindings. This ADR reuses that grammar verbatim; it does not
re-specify it.

Consequence: templates are statically typed against the ontology
schema at **install time**, not at speak time. The avatar never
speaks broken templates because broken templates never get
registered.

#### C.3 Avatar app discovery

An avatar app subscribes to `substrate/apps/installed` (app-registry
topic per ADR-015). For each installed app with a non-empty
`[narration]` table, the avatar:

1. Reads the app's narration rules from the registry's public
   surface.
2. Subscribes to each declared topic path.
3. On each committed topic delta, evaluates the matching template
   against the new state, appends the utterance to its TTS queue.
4. On `surface.update` of the caption stream (§C.6), writes the
   utterance text into the `StreamView`.

**Zero special-casing per app.** The avatar does not know what
"admin" or "deployment" means; it reads the topic path, looks up the
template, expands, speaks. Adding a narrating app is a matter of
installing it — the avatar picks up new rules from the registry
delta without a restart.

#### C.4 Voice-triggered app switching

In `single-app × voice` (avatar), the launcher is voice-commanded.
Utterances like *"show me deployments"* are routed through the
avatar's launcher matcher:

1. **Enumerate** installed apps via `substrate/apps/installed`.
2. **Match** the utterance against each app's `entry_points` with
   `kind = "wake-word"` or `kind = "voice-phrase"` (ADR-015 §(3))
   using the canonical-name-plus-utterances logic from §B.4.
3. **Branch on capability**:
   - **Has a `[narration]` table**: subscribe to its topics, begin
     narrating current state.
   - **No narration contract** (pointer/touch-only app): avatar
     **politely refuses** — *"the deployment monitor isn't available
     in voice mode"* — and the app does not launch. Surfacing a
     non-voice app to a voice session would violate its own
     `supported_inputs` and is unreachable-by-declaration.
4. On successful switch, the currently-narrated topic set is
   replaced; previous subscriptions are released.

#### C.5 Wake word — out of scope

This ADR **does not specify** the wake-word implementation. Hosts
ship their own wake daemon (`hey-wfi`, Porcupine, successors) and
drop transcript lines onto `substrate/voice/transcript`. The avatar
subscribes. How transcripts get there is the host's problem; what
the avatar does with them is this ADR's problem.

Wake-word privacy — mic-always-on — is governed by ADR-012 verbatim:
the tray chip is non-dismissible while the mic capture channel is
live, regardless of whether the wake daemon or the avatar is the
consumer.

#### C.6 Caption surface — accessibility requirement

Alongside the character, the shell renders a `ui://stream-view`
(ADR-001 row 18) of recent transcripts and avatar responses. The
caption surface is **always visible** — hard accessibility
requirement. Users who cannot hear the TTS (deafness, muted audio,
noisy environment, unplugged headphones) must be able to read what
the avatar would have said.

Canonical `single-app × voice (avatar)` layout (Session 10 §6.4):

- Character top-half (≈ 60 % of viewport height).
- Caption stream bottom-third (`StreamView`, ring buffer of the last
  ~50 utterances).
- Capture chip top-right (ADR-012 tray chip).

The caption surface is a regular Tier-A primitive bound to
`substrate/voice/captions`; "always visible" is enforced by shell
layout, not by a special rendering path.

#### C.7 Test discipline — headless narration

Avatar tests **do not synthesise audio**. The narration engine is
driven with canned topic deltas; assertions compare the expected
expanded-template text against the produced TTS queue entries.

```rust
#[test]
fn admin_health_narration_expands_correctly() {
    let manifest = load_manifest("admin.toml");
    let topic = "substrate/admin/health";
    let state = json!({
        "services": [
            {"name": "weaver",  "status": "healthy"},
            {"name": "mesh",    "status": "at_risk"},
            {"name": "capture", "status": "healthy"},
        ]
    });
    let engine = NarrationEngine::new(&manifest);
    let utterance = engine.expand(topic, &state);
    assert_eq!(utterance, "2 services healthy, 1 at risk.");
}
```

No TTS engine, no mic capture, no character render in the test path.
Because the rule grammar is ADR-016 §5, the test is a pure
expression-evaluation test; audio synthesis is an M1.9 integration
concern.

---

## Consequences

### Positive

- **Input is a session-level value.** Primitives stay naive;
  adaptation lives in one place. Touch panels and voice companions
  reuse the full canon without per-primitive branching.
- **Voice invocability is self-describing.** No fuzzy matcher can
  escalate a pointer-only affordance to voice-reachable. Predicate 2
  holds uniformly across channels.
- **Avatar apps are ordinary apps.** Narration rules live in the
  authored app's manifest, not in a god-mode avatar crate. Domain
  knowledge decentralises to the apps that own it.
- **Template grammar reuse.** Narration templates = surface-
  description binding expressions. One grammar, one type checker,
  one governance review surface.
- **Accessibility by construction.** The caption stream requirement
  makes every avatar session screen-reader-compatible. Deaf users,
  muted-audio users, and noisy-environment users share the same code
  path.
- **Conservative auto-detection.** Ambiguity resolves to `hybrid`,
  preventing "my mouse stopped working" regressions on touch
  laptops.

### Negative

- **One-way hybrid upgrade.** A `pointer` session that catches a
  stray touch event (e.g. a palm-touch) upgrades to `hybrid` for its
  lifetime. Accepted: the alternative is a sticky heuristic worse
  than a deterministic gate.
- **`invocations` default is a compatibility hack.** Legacy manifests
  will implicitly be `["pointer", "touch"]`; voice-mode users will
  hit polite rejections on almost every legacy affordance until
  manifests are updated. Correct failure mode, but an M1.9 rollout
  should coincide with a manifest review pass.
- **Install-time template validation.** A template referencing a
  path whose adapter is uninstalled is rejected at install — avatar
  apps can have install-time coupling to narrating apps' adapter
  sets. The registry surfaces this as a dependency diagnostic.
- **Caption `StreamView` is hard-wired.** Avatar authors cannot hide
  captions for aesthetic reasons — same posture as ADR-012's tray
  chip. Accessibility floor > author aesthetic freedom.
- **No wake-word spec here.** Hosts must solve wake-word
  independently; the protocol boundary is
  `substrate/voice/transcript`. Deliberate (plural implementations)
  but avatar apps cannot assume a particular wake daemon.

### Neutral

- Safe-area inset handling is a one-liner against
  `egui::Context::input`; no new dependencies.
- `utterances` is optional; apps that don't declare it fall back to
  canonical-name matching, usually enough.
- `gesture` as a distinct invocation channel only matters for canvas
  primitives today; specified now so future canvas gesture handlers
  don't retrofit the schema.

## Alternatives considered

1. **Per-primitive input declaration.** Rejected: every author would
   re-derive the 44 px floor, tooltip semantics, and gesture mapping;
   drift across primitives guaranteed.
2. **Voice-as-a-policy-layer (daemon decides).** Rejected per §B.2:
   the Cedar-OS failure mode where the surface claims one thing and
   the daemon does another. Self-description must cover channels.
3. **Avatar as a fourth mode.** Rejected per Session 10 rec. 4.
   Avatar is `single-app × voice` with `foreign://avatar` as the
   surface; making it a mode duplicates the compositor code path
   with no gain.
4. **Narration rules in the avatar app.** Rejected per Session 10
   rec. 11. The avatar would carry domain knowledge of every
   narratable app; decentralising to each app's manifest makes
   narration a property of the app that owns the topic.
5. **Turing-complete narration language.** Rejected for the same
   reason ADR-016 §5 rejected it for binding expressions:
   governance cannot statically review side-effectful templates, and
   voice TTS runaway cost is a denial-of-service vector.
6. **Separate ADRs for each sub-decision.** Rejected: the three
   parts mutually reference each other. Splitting would force every
   reader to cross-reference three files for any single concern.
7. **Two-way hybrid (downgrade on idle pointer).** Rejected. Once a
   touch event proves the hardware exists, there is no legitimate
   reason to retract touch affordances. Idle-downgrade introduces
   non-determinism.

## Related

- **Sessions**: `session-10-app-layer.md` (§2.2 input axis, §5 input
  adaptation, §6.4 avatar crosswalk, §8 rec. 4/9/10/11).
- **Foundation elements**: four predicates (predicate 2 extended in
  Part B to cover channel), §"Non-negotiable privacy constraints" 4
  (observable capture — honoured in §C.1), accessibility parity for
  screen-reader and deaf users.
- **ADRs amended**: `adr-006-primitive-head.md` — `affordance` gains
  `invocations` and optional `utterances` (Part B).
- **ADRs extended**: `adr-001-primitive-canon.md` row 20
  (`ui://foreign`) — new Tier-B kind `avatar` (Part C).
- **ADRs referenced**: `adr-012-capture-privacy-invariants.md`
  (tray-chip invariant binds to avatar mic/camera use),
  `adr-014-modal-modality-split.md` (toast narration of voice
  rejections; reduced-viewport re-flow under IME), `adr-015-app-
  manifest.md` (`supported_inputs`, `entry_points[kind=wake-word]`,
  `[narration]` — not re-specified here), `adr-016-surface-
  description.md` §5 (narration templates reuse this grammar — not
  re-specified here), `adr-013-canvas-primitive.md` (`gesture`
  invocation consumers), `adr-007-active-radar-return-schema.md`
  (voice rejection → return-signal chain), `adr-008-goal-aggregate-
  and-delegation.md` (per-goal consent for mic/camera in avatar
  sessions), `adr-018-ide-bridge.md` (`ide × touch` chrome
  deference; §A.2 references it).
- **Milestones**: M1.8 (compositor touch adaptation — Part A); M1.9
  (voice + avatar — Parts B and C); M2 (active-radar loop carries
  voice invocation through return-signal schema).
- **Open**: wake-word daemon plurality (Session 10 §9); multi-avatar
  sessions (Session 10 §9); narration i18n (Round 4).
