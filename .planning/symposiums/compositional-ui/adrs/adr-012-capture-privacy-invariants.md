# ADR-012: Capture Privacy Invariants + Governance-Enforced Tray Chip

**Date**: 2026-04-18
**Status**: Proposed — symposium round 2
**Deciders**: Compositional UI Symposium (Round 1 synthesis)

## Context

Foundations is unambiguous about ambient capture: the observation
stream is the system's most valuable asset **and** the user's most
personal dataset, and these two facts are the same fact. Four
non-negotiable privacy constraints are stated: (1) per-user, never
telemetry; (2) governance-gated by default; (3) signal-lossy by
policy; (4) observable to the user via a persistent, honest surface
primitive — "no dark recording, ever". Session 7 reinforces this:
the dev-panel's `weft-capture` sidecar must refuse to start without
a signed capability grant, and the tray chip must be non-dismissible
while capture is live. Session 8 (goals) binds consent per-goal so
that sealing a goal revokes outstanding consents on the chain.
Session 2's cohort confirms no competitor enforces any of this
structurally. Without a kernel-level invariant, every future commit
could regress — adtech-style silent-capture bugs are the common
failure mode. The rule cannot live in the renderer.

## Decision

Capture privacy is enforced at three layers simultaneously; no
single layer is trusted. The tray chip is composed by the kernel,
not by any agent, and consent is carried per-goal with explicit
expiry tied to goal seal.

**Invariant 1 — `privacy-flags` is structural, not metadata** (ADR-006).
Every primitive head carries `{captured, retained, off-node,
consent-id}`. A `foreign://{camera,mic,screen}` surface with a
non-empty `capture-channels` list and an absent or expired
`consent-id` is a **malformed frame**. The kernel rejects it before
it reaches any renderer. This is a type-system fact: silent
capture cannot be expressed on the wire.

**Invariant 2 — governance gates at grant time, not per-tick**.
The `weft-capture` sidecar (ADR-011) and any other wrapper emitting
to the capture channel refuse to start without a signed
`CapabilityGrant` from `governance-counsel` scoped to a specific
`goal_id` (ADR-008). Grant carries `scope`, `purpose`, `duration`,
and an explicit expiry (`valid_until` or `until_goal_state`).
Revocation on goal seal is automatic and chain-logged as
`governance.goal.consent.revoke`.

**Invariant 3 — kernel composes the tray chip** (not the agent).
Whenever any live `foreign://{camera,mic,screen}` surface exists in
any active session for the user, the kernel implicitly composes a
`ui://chip{kind="capture"}` in the Tray surface. The chip:
- Is **non-dismissible while any capture is live.** The user may
  revoke consent, which terminates the capture; the chip disappears
  only when capture actually stops, not when the user hides it.
- Displays which channels are live, which goal permits them, and
  when consent expires.
- Clicking opens a governance surface with the consent grant's full
  terms, the current `capture-channels` list, and a one-tap
  `consent.revoke` affordance.
- Is composed at the protocol layer (Session 6 §9 + §11 rec. 9),
  not an opt-in renderer feature. A renderer that fails to render
  the tray chip is non-conformant.

**Invariant 4 — signal lossy by default**. What the wrapper emits
on the observation tapestry (`ambient` observation body, ADR-007)
is the *interpretation* — detected text, detected face class,
detected URL, detected application-in-focus, transcript fragment —
never raw capture. Raw capture requires (a) `capture-channel.emits
= "raw"`, (b) a positive `consent-id` with a matching retention
decision, and (c) a sibling `explicit` observation logging the
retention act for the chain audit. "Retained" does not imply
"off-node".

**Invariant 5 — per-user, on-node**. `privacy-flags.off-node = true`
requires an *additional* explicit consent for export. Export is
always a chain event with a ZK proof of the user's authorisation.
No aggregated telemetry. No training off-node without explicit
revocable consent.

**Per-goal consent binding** (ADR-008 `capture_policy`). Capture
permissions are a property of the goal's aggregate, not a standing
user preference. Consent expires on goal seal; the revocation
cascades through all sessions, and the kernel garbage-collects
dependent `foreign://*` surfaces before composing any further
frames. Session 3's tombstone discipline (ADR-009) ensures late
writes into a revoked stream fail cleanly.

**Failure semantics** are honest and user-visible:
- Malformed frame (missing `privacy-flags` or expired consent) →
  kernel rejects, composer receives structured `deny{reason:
  "malformed:capture-without-consent", …}`, renderer never sees
  the frame.
- Sidecar started without grant → immediate self-exit, logged as
  `governance.goal.consent.refused`, tray chip reads "capture
  blocked: no consent".
- Consent revoked mid-session → capture terminates within one
  kernel tick; a `governance.goal.consent.revoke` event is chained;
  the tray chip fades on confirmed termination, not on request
  arrival.

## Consequences

### Positive
- "No dark recording, ever" (foundations non-negotiable 4) becomes
  a structural property the kernel enforces, not a convention a
  reviewer hopes a developer remembered.
- Adtech-style A/B-test-on-consent-UI (a class of bug that has
  shipped in every major SDK at least once) is impossible:
  `mutation-axes` on capture affordances is empty by fiat, and
  GEPA cannot mutate them (foundations §"active-radar loop").
- Consent binds per-goal, so sealed work releases its capture
  permissions automatically. Users do not accumulate zombie
  standing consents.
- Tray chip being kernel-composed means no agent — benign or
  malicious — can suppress the visible-capture indicator.
- Raw capture exports are chain-auditable ZK events; the user's
  observation data is portable under their own authorisation and
  nowhere else.

### Negative
- Frame validation cost at the kernel boundary is non-zero on every
  capture-eligible primitive; we accept the overhead because the
  alternative is a silent-capture class of bug.
- Per-goal consent means a user who works across many goals in a
  day grants capture many times; the working proposal is
  standing-consent-with-audit-trail per root goal (foundations
  line 195–196), narrowed at subgoal level.
- Tray chip UI real estate is always visible when capture is live;
  transient surfaces cannot suppress it even for aesthetic reasons.
- Sidecar-to-kernel consent handshake adds startup latency on
  voice-first workflows.

### Neutral
- Export + ZK proof path is specified at the policy level; the
  actual proof system is outside this symposium's scope and
  tracked in Rasmus unsolved problem #5 work.

## Alternatives considered

1. **Consent as a user preference, not per-goal** — rejected per
   Session 8: standing preferences accumulate into ambient
   capture with no expiry frame; foundations non-negotiable 1–4
   require revocability and observable lifetime, which the goal
   aggregate provides cleanly.
2. **Renderer composes the tray chip** — rejected per Session 6
   §9: a renderer that forgets or is subverted suppresses the
   indicator. Kernel-level composition is the only trustworthy
   location; renderer non-compliance becomes a conformance failure
   visible at the protocol boundary.
3. **Single consent grant for the user session** — rejected: a
   session-scoped consent is opaque to audit and cannot be
   narrowed per task. Per-goal scoping makes consent legible to
   the user and to the chain.
4. **Interpretation optional, raw by default** — rejected:
   foundations non-negotiable 3 ("signal lossy by policy") is
   exactly inverted here. Interpretation is the default; raw is
   the exception that requires explicit consent.
5. **Rely on renderer sandboxing (browser mic gate / OS perms)** —
   considered and rejected as sole defence: browser gates stop
   `getUserMedia` but do not enforce governance's per-goal
   expiry, retention policy, or off-node rules. OS perms are
   necessary but insufficient.
6. **Make `capture-channels` optional on `foreign-surface`** —
   rejected per ADR-006: absence is the silent-capture hole.
   An explicit empty list means "no capture"; a missing field is
   malformed.

## Related

- Sessions: `session-8-governance.md` (§"Binding to surfaces",
  §"Persistence", capture consent per-goal, `governance.goal.
  consent.*` events, rec. 5), `session-6-protocol-design.md`
  (§9 governance + consent, §11 rec. 5 + rec. 9 tray chip
  protocol obligation), `session-7-dev-panel-embedding.md`
  (§"Voice input path" non-dismissible chip, sidecar refuses
  without grant), `session-2-agentic-ui-sota.md` (gap 8 — no
  industry return-signal model; capture is the highest-stakes
  variant).
- Foundation elements: §"Non-negotiable privacy constraints" 1–4
  (per-user / governance-gated / signal-lossy / observable),
  §"Tier B as woven signal tapestry" (observation is per-user
  substrate), §"Digital exhaust = intent" (proportional
  protection).
- ADRs: ADR-001 (canon includes `ui://chip` + `ui://foreign`),
  ADR-002 (six canonical wrapper types incl.
  camera/mic/screen), ADR-006 (privacy-flags mandatory on head),
  ADR-007 (ambient observation default = interpretation),
  ADR-008 (per-goal `capture_policy`, consent expiry),
  ADR-011 (`weft-capture` sidecar architecture).
