# ADR-007: Active-Radar Return Schema — CanonResponse on `ux/returns`

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Foundations frames the system as an active-sensing one, not a passive
paint: display is the emitted pulse, user interaction is the return
echo, ECC is the receiver, GEPA is the learning step. Four
reconstructable quantities must flow back on every interaction —
**topology** (which primitive, in what order, how long), **doppler**
(velocity / direction of action), **range** (latency-to-act), and
**bearing** (which affordance was chosen). Session 1 confirmed no
historical protocol carried any of this; Session 2 confirmed no
current competitor does either (gap 8). Session 5 showed the
renderer-side implementation path (wrap `egui::Response` with the
four fields). Session 6 §7 specified the wire envelope. Session 9
rec. 10 made it non-negotiable. Without a structural return schema,
foundations' single biggest claim — that we collect intent as
first-class data — is a slogan.

## Decision

Every WSP interaction produces a `CanonResponse` that extends
`egui::Response` (on egui renderers) or its equivalent on any other
renderer. The four quantities are explicit fields; they are emitted
on the substrate topic **`ux/returns`** as event-sourced
observations (ADR-004), and every observation carries the
`variant-id` of the pulse that produced it (ADR-006).

**Renderer-side trait contract** (egui reference; other renderers
MUST provide equivalent):

```rust
pub struct CanonResponse {
    pub inner: egui::Response,          // pointer / keyboard state
    pub topology: Topology,              // (1)
    pub doppler:  Option<Doppler>,       // (2)
    pub range:    Option<Range>,         // (3)
    pub bearing:  Option<Bearing>,       // (4)
    pub variant:  VariantId,             // from primitive-head
}
```

**Four canonical signal bodies** (Session 6 §7, normalised):

1. **Topology** — `{ surface_id, path, dwell_ms, order_index }`.
   Reconstructed from `Response.rect` + the renderer's layer
   hierarchy (egui: `ctx.memory(|m| m.layer_id_at(pos))`).
   Describes which primitive was engaged, in what sequence, for
   how long.
2. **Doppler** — `{ surface_id, path, velocity: f64, direction:
   "forward" | "backward" }`. Inter-press velocity + drag delta +
   pointer delta. Signed scalar per primitive: fast confident
   action shifts toward *coherence rising*; hesitation, re-reads,
   backtracking shift toward *coherence falling*.
3. **Range** — `{ surface_id, path, latency_ms }`. Delta between
   the frame the primitive first appeared and the frame the user
   acted. Long latencies on the same primitive are a design smell
   the system can see without a bug report.
4. **Bearing** — `{ surface_id, path, affordance, actor }`.
   Encoded as the affordance IRI, not an index, so ECC attribution
   is stable across variants that reorder the affordance list.

Additional observation kinds ride the same stream:
**explicit** (click / utterance / submit), **implicit** (dwell /
hover / scroll without click, sampled ≥ 1 Hz), **ambient** (wrapper
digests — interpretation only, raw only under active consent). The
observation-kind set is open (Session 6 §10): new kinds are added
by IRI, not by breaking the wire.

**Substrate topic**: `ux/returns`. One topic per session; the
observation stream is event-sourced (ADR-004) with monotonic
`seq: u64`. The renderer emits signals; ECC and GEPA consume
directly. The protocol does **not** pre-aggregate — intent is
history, not a scalar (foundations §"Digital exhaust = intent").

**Variant-id discipline**: every return observation MUST echo the
`variant-id` of the rendered primitive that produced it. Renderers
read variant from `primitive-head.variant` (ADR-006) and stamp it on
`CanonResponse`. ECC attributes echoes to pulses with no shared
state between composer and renderer.

**Privacy rules**:
- Ambient observations default to interpretation-only digests
  (OCR text, transcript fragment, detected face class — not raw
  pixels or audio). Raw requires a positive `consent-id` and a
  sibling `explicit` observation recording the retention decision.
- Return-signal emission from a surface composed under a goal
  whose `capture_policy` forbids exhaust emission is suppressed at
  renderer, logged at governance (ADR-008 / ADR-012).

## Consequences

### Positive
- Foundations' most load-bearing new requirement (the active-radar
  loop) becomes a structural fact of the wire, not a convention.
  Every competitor is years behind on this specifically.
- ECC receives pulse + echo pairs at resolution, keyed by
  variant-id, with no reconstruction required. GEPA's mutation
  attribution is trivial: emit, observe, update.
- Return signals are uniform across renderers; a second renderer
  (success-criterion proof) inherits the schema for free if it
  implements the `CanonResponse` trait.
- Implicit and ambient signals ride the same envelope as explicit
  ones; ECC can weight them, merge them, or ignore them without a
  second pipeline.

### Negative
- Per-interaction observation volume is not small; raw stream rate
  on a hot surface can exceed tens of Hz. Back-pressure (cancel,
  coalesce, drop-oldest) is a real concern we rely on LSP-style
  progress tokens to manage (Session 6 §6).
- Every renderer must implement the four-field capture; naïve
  renderers that cannot reconstruct topology (no layer hierarchy)
  degrade to bearing-only and forfeit doppler/range. This limits
  the bar a minimal second renderer must clear.
- Ambient capture observations require careful privacy discipline;
  one bug leaks intent. ADR-012 locks the invariants.

### Neutral
- The topic choice (`ux/returns`) slots into the existing
  `substrate/*` namespace convention and reuses the same subscribe
  / unsubscribe verbs (ADR-005 §7/8).

## Alternatives considered

1. **Pixel coordinates for topology (Figma-cursor shape)** —
   rejected per Session 3 rec. 2: pixel coords break under
   recomposition and mean nothing to ECC when the layout morphs.
   IRI + path is the only invariant encoding.
2. **Coalesce signals server-side before ECC** — rejected: intent
   is history (foundations); pre-aggregation loses the weave that
   foundations §"Tier B as woven signal tapestry" commits to
   preserving.
3. **Single scalar return (AI Elements onClick)** — rejected:
   everything interesting is in the other three quantities. A click
   tells you bearing; without topology / doppler / range the
   causal signal is unrecoverable.
4. **Separate stream per signal kind** — rejected: Session 6 §7
   explicitly unifies kinds on one observation stream so the
   ordering between (e.g.) explicit click and its preceding doppler
   is preserved without cross-stream alignment.
5. **Flush-on-commit rather than continuous** — rejected: implicit
   signals (dwell, scroll, re-read latency) die on commit-only
   semantics; they are exactly the weak-signal channels that carry
   coherence trajectory information.

## Related

- Sessions: `session-6-protocol-design.md` (§7 return channel, §11
  rec. 11), `session-5-renderer-contracts.md` (§"Return-signal
  schema", `CanonResponse` trait), `session-2-agentic-ui-sota.md`
  (gap 8 — no return-signal model anywhere),
  `session-1-historical-canon.md` (Adaptive Cards comes closest
  and still stops at render), `session-9-agentic-os-canon.md`
  (rec. 10 return signals first-class).
- Foundation elements: §"Active-radar loop" (topology / doppler /
  range / bearing / strength), §"Digital exhaust = intent",
  predicate 3 (streaming-native), Non-negotiable 3 (signal lossy
  by policy).
- ADRs: ADR-004 (schema shape — event-sourced tapestry), ADR-005
  (`observe` verb), ADR-006 (primitive head variant-id), ADR-008
  (goals as frame of reference), ADR-012 (capture privacy).
