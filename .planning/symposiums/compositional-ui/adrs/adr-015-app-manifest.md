# ADR-015: App Manifest — TOML Schema, Validation, and Lifecycle

**Date**: 2026-04-20
**Status**: Proposed — symposium round 3
**Deciders**: Compositional UI Symposium (Round 3 — App Layer)

## Context

Session 10 §1 named the fifth structural gap: **WeftOS has no
concept of "an app" as a first-class artefact.** Today
`Desktop::render_blocks_window` is a hand-coded `match` over
`BlockKind::ALL` — there is no declarative path from "I want a
panel that shows the current project's CI/CD status" to a running
surface, no install-time story, no permissions prompt, no
versioning, and nothing the avatar companion (§6.4) can subscribe
to in order to narrate another app's state.

Session 10 §3.1 enumerated nine properties that every app must
declare: identity, supported modes, supported inputs, surfaces,
ontology subscriptions, ontology influences, optional narration
contract, capture permissions, and entry points. Session 10 §3.2
identified three authoring surfaces — Rust crate, declarative
TOML, and agent-authored at runtime via `surface.compose` —
that must all produce an equivalent installable artefact.
Session 10 recommendation 5 settled the wire format (TOML).
Recommendation 2 settled the kiosk contract for `single-app`.
Recommendation 13 set the milestone exit: once this ADR ships
and M1.5 lands, `BlockKind` is deleted and the Blocks window
becomes an app like any other.

This ADR fixes the manifest itself. It does *not* specify how
surfaces are described (see ADR-016), how ontology adapters
subscribe (ADR-017), how the IDE bridge's `ide.*` verbs are
catalogued (ADR-018), or how inputs and narration rules are
interpreted at render time (ADR-019). The manifest is the
**declaration**; the other ADRs are the interpreters.

## Decision

A WeftOS app is a TOML document named `weftos-app.toml`
(canonical filename; alternates rejected at install) with the
schema below. The manifest is parsed once at install time,
schema-validated, capture-gated via governance (ADR-012), and
registered in a local app-registry SQLite table. At launch time
the compositor consults the registry, intersects the caller's
requested `(mode, input)` with the manifest's declared support,
and instantiates surfaces. A manifest that fails any validation
rule in §Validation below is **malformed** and never reaches the
registry — the same structural-rejection posture ADR-006 adopts
for primitive heads.

### Schema

```toml
# weftos-app.toml

# (1) Identity — ontology IRI, display fields, semver.
id       = "app://weftos.admin"        # ontology IRI, globally unique
name     = "WeftOS Admin"
version  = "0.1.0"                     # semver (§Versioning)
icon     = "assets/admin.svg"          # path (relative to manifest) or IRI

# (2) Capability envelope — non-empty subsets.
supported_modes  = ["ide", "desktop"]                 # ⊆ {single-app, desktop, ide}
supported_inputs = ["pointer", "hybrid"]              # ⊆ {pointer, touch, voice, hybrid}

# (3) Entry points — how the host launches this app.
[[entry_points]]
kind      = "cli"                      # `weaver gui --app <id>`
flag      = "admin"

[[entry_points]]
kind      = "vscode-command"
command   = "weft.admin.open"

[[entry_points]]
kind      = "wake-word"                # voice; requires `voice` in supported_inputs
phrase    = "weft, show me admin"

# (4) Surfaces — declared by ID; surface bodies live in ADR-016 files.
surfaces = ["surfaces/admin-main.toml"]   # single-app: len == 1; desktop/ide: >= 1

# (5) Ontology subscriptions — topic paths; adapters declared in ADR-017.
subscriptions = [
  "substrate/kernel/status",
  "substrate/kernel/processes",
  "substrate/kernel/services",
  "substrate/kernel/logs",
]

# (6) Ontology influences — WSP verbs this app invokes.
influences = [
  "wsp.activate",
  "kernel.restart-service",
  "kernel.kill-process",
]

# (7) Permissions — capture channels; ADR-012 governance-gated.
permissions = [
  "fs:/var/log/weftos",                # filesystem read under a path prefix
  "net:api.github.com",                # network egress to a domain
  # "mic", "camera", "screen" would gate capture wrappers
]

# (8) Narration contract — optional; required iff voice ∈ supported_inputs.
[narration]
"substrate/kernel/services" = """
{{count(services, s -> s.status = "healthy")}} services healthy,
{{count(services, s -> s.status = "at_risk")}} at risk.
"""
```

Field semantics (only the non-obvious):

- **`id`** is an ontology IRI in the `app://` scheme. It is the
  addressable name used by `surface.compose`, by entry-point
  launch calls, and by the avatar companion when it subscribes
  to another app's narration contract (§6.4).
- **`supported_modes`** is a non-empty subset. Session 10 §2.1
  established that `ide` is a superset of `desktop`; an app
  declaring `ide` but not `desktop` is a legal but unusual
  posture (IDE-only tool). Declaring `desktop` but not `ide`
  means "works in a standalone workstation but not wired to
  the editor" — also legal.
- **`surfaces`** lists surface-description file paths (format
  defined by ADR-016). A `single-app` app declares exactly one;
  a `desktop`/`ide` app may declare one per virtual workspace
  window (Session 10 §3.1 property 4).
- **`subscriptions`** entries are ontology topic paths. Each
  path must resolve to a registered adapter (ADR-017). An
  entry whose adapter is not installed is a dependency error
  at install time — the registry flags the app as "missing
  adapters" and refuses to launch it.
- **`influences`** entries are WSP verbs the app's affordances
  may invoke. `ide.*` verbs require `ide` in `supported_modes`
  (ADR-018). This list is the **write side**; it is what the
  governance gate will intersect against `affordance ∩ permit`
  at render time (ADR-006 rule 2).
- **`permissions`** entries match the capture-channel grammar
  of ADR-012: `camera`, `mic`, `screen`, `fs:<path-prefix>`,
  `net:<domain>`. They are resolved at **install time** by
  governance, and a capture channel not in this list cannot
  be activated at runtime even if an adapter offers it.
- **`[narration]`** is a table keyed by subscribed topic path
  (must appear in `subscriptions`); values are speakable
  templates in the rule language defined by ADR-019.

### Validation

A manifest is **malformed** — rejected at install, never
registered — if any of the following hold:

1. `supported_modes` is empty, or contains a value outside
   `{single-app, desktop, ide}`.
2. `supported_inputs` is empty, or contains a value outside
   `{pointer, touch, voice, hybrid}`.
3. `[narration]` is non-empty but `voice` is not in
   `supported_inputs`. (Narration is a voice-mode concept;
   declaring rules without voice is incoherent.)
4. `supported_modes == ["single-app"]` and `surfaces.len() != 1`.
   Session 10 recommendation 2: single-app is a locked kiosk;
   exactly one surface, no escape.
5. `influences` contains any `ide.*` verb but `ide` is not in
   `supported_modes`. The IDE bridge (ADR-018) is only active
   in `ide` sessions; declaring an IDE write without IDE support
   is unreachable code.
6. `permissions` requests a capture channel (`camera`, `mic`,
   `screen`, `fs:*`, `net:*`) with no corresponding adapter in
   `subscriptions` that consumes that channel. A capture
   permission with no reader is either a privacy-leak waiting
   to happen (channel is live, app never reads) or a copy-paste
   error; either way, rejected.
7. A `wake-word` entry point exists but `voice` is not in
   `supported_inputs`.
8. `version` is not valid semver.
9. `id` is not a well-formed `app://` IRI.
10. Any `subscriptions` topic path references an adapter not
    registered on the host — this is a dependency error, flagged
    but not a structural malformation; the app installs in a
    **disabled** state until the adapter is added.

Rules 1–9 are structural (manifest is wrong). Rule 10 is
environmental (manifest is fine, host is missing pieces).

### Install / Launch / Lifecycle

**Install** (`weaver app install <path>` or VSCode command):

1. Parse `weftos-app.toml`.
2. Validate schema (rules 1–9 above) — reject on failure.
3. For each entry in `permissions`, prompt the user via the
   governance UI (ADR-012); persist the granted capability set
   as an ADR-008 `consent-id` scoped to this app IRI.
4. Resolve each `subscriptions` adapter (ADR-017); record
   dependency status per rule 10.
5. Compile each surface description file (ADR-016) to its
   internal `SurfaceTree` representation; cache in the registry.
6. Write a row to the local `app_registry` SQLite table keyed
   by `id`, with manifest hash, consent-id, compiled surfaces,
   and enabled/disabled state.

**Launch** (any host names an app):

1. Host (`weaver gui`, VSCode extension, wake-word daemon, tray)
   calls the compositor with `(app_id, session_config = {mode,
   input})`.
2. Compositor loads the registry row. If `mode` ∉
   `supported_modes` or `input` ∉ `supported_inputs`, the launch
   is refused with a typed error — no best-effort downgrade.
3. Compositor instantiates the surfaces declared for this mode
   (single-app: the one; desktop/ide: all, with the first as
   focused).
4. Compositor wires each surface's subscriptions to the relevant
   adapters and registers the app's `influences` as the set of
   WSP verbs that may be dispatched from within this app's
   surfaces (governance still intersects per-frame, ADR-006 §2).
5. Narration rules (if any) are compiled and handed to the
   avatar companion if one is subscribed (ADR-019).

**Lifecycle**:

- **`single-app`** sessions lock to one surface; the compositor
  refuses any `surface.compose` that would introduce a second
  top-level surface within the session. Modals/tooltips are
  composed *inside* the locked surface per ADR-014 modality
  rules — those are not new top-level surfaces.
- **`desktop` / `ide`** sessions allow the app to compose
  additional surfaces up to its declared `surfaces` array plus
  any dynamic surfaces the agent composes at runtime (subject
  to governance). Focus moves between them; minimise / restore
  is free.
- **Terminate** (user closes, host exits, consent revoked):
  compositor tears down all surfaces for this app, unsubscribes
  every topic wired under this session, and revokes the
  affordance-invocation registrations. Any in-flight streaming
  primitive receives the `interrupt` verb (ADR-011 §"Interruption
  is a verb on every streaming primitive").
- **Consent revocation** (ADR-012): if a capture channel's
  consent is revoked mid-session, the adapter stops, the app is
  notified via a `substrate/app/<id>/permission-changed` topic,
  and the relevant surfaces render with the lost channel
  represented honestly (e.g. avatar companion falls back to
  text captions if mic consent is revoked).

### Authoring surfaces

Three paths, all producing the same registry representation
(Session 10 §3.2):

1. **Rust crate.** The app is a Cargo crate containing
   `weftos-app.toml` at its root plus one or more `impl Surface`
   types registered via a `#[weftos_app]` proc-macro. Built via
   `cargo build`; shipped as a binary artefact (a `.wapp` — a
   signed tarball containing the manifest + compiled surface
   code + assets). This is how first-party apps (admin,
   project-OS, deployment monitor) ship. The manifest format is
   identical; the `surfaces` array points to Rust type names
   instead of description files (resolved via the proc-macro's
   generated registry).
2. **Declarative TOML.** The app is a directory containing
   `weftos-app.toml` plus one or more ADR-016 surface-description
   files. No Rust compilation. Shipped as an unsigned tarball
   (signing is orthogonal, see ADR-012 capture channels).
   Community apps take this path.
3. **Agent-authored.** An agent emits a manifest + surface
   descriptions at runtime via `surface.compose` (protocol-spec
   §7). Governance approval is always required; the compositor
   stages the manifest into a scratch namespace and asks for
   the user's consent before registering. Approved agent-authored
   apps are written to the same registry as paths (1) and (2).

The manifest schema in this ADR is identical across all three
paths. The only difference is what `surfaces` entries resolve
to (file paths vs. Rust type IDs vs. inline descriptions).

### Versioning

Manifests declare semver (`version = "MAJOR.MINOR.PATCH"`).
Backwards-compat rules, enforced by the registry's
`app.upgrade` pathway:

- **Patch** (`0.1.0 → 0.1.1`): any change that preserves the
  full surface-description output for equivalent topic inputs.
  Fixes only.
- **Minor** (`0.1.1 → 0.2.0`): additions. New surfaces, new
  narration rules, new voice invocation channels on existing
  affordances, new optional fields. **Narration rules may only
  be added, never rewritten in-place** — rewriting a rule
  changes what the avatar says about identical state, which
  is a semantic break. Replace by adding a new key, then
  dropping the old one in the next major.
- **Major** (`0.2.0 → 1.0.0`): anything else. Specifically:
  dropping an input channel from an affordance's invocation set
  (`voice` can be added without a major, `pointer` cannot be
  dropped without one — foundations-level accessibility
  contract), dropping a supported mode, narrowing
  `subscriptions`, narrowing `influences`, changing the
  manifest schema itself.

On upgrade, the registry diffs the old and new manifests. A
minor upgrade auto-applies. A major upgrade re-prompts the
user for capture consent (ADR-012) and re-compiles all
surfaces.

## Concrete example — WeftOS Admin

Declarative-TOML form:

```toml
# apps/weftos-admin/weftos-app.toml
id       = "app://weftos.admin"
name     = "WeftOS Admin"
version  = "0.1.0"
icon     = "assets/admin.svg"

supported_modes  = ["ide", "desktop", "single-app"]
supported_inputs = ["pointer", "hybrid", "voice"]

[[entry_points]]
kind    = "cli"
flag    = "admin"

[[entry_points]]
kind    = "vscode-command"
command = "weft.admin.open"

[[entry_points]]
kind    = "wake-word"
phrase  = "weft, admin status"

surfaces = [
  "surfaces/admin-main.toml",         # used in ide/desktop
  "surfaces/admin-kiosk.toml",        # used in single-app × touch
]

subscriptions = [
  "substrate/kernel/status",
  "substrate/kernel/processes",
  "substrate/kernel/services",
  "substrate/kernel/logs",
]

influences = [
  "wsp.activate",
  "kernel.restart-service",
  "kernel.kill-process",
]

permissions = [
  "fs:/var/log/weftos",
]

[narration]
"substrate/kernel/services" = """
{{count(services, s -> s.status = "healthy")}} services healthy,
{{count(services, s -> s.status = "at_risk")}} at risk: {{names(services, s -> s.status = "at_risk")}}.
"""
"substrate/kernel/status" = """
Kernel is {{status.state}}. Uptime {{duration(status.started_at)}}.
"""
```

Rust-crate form (same manifest, different `surfaces` resolution):

```toml
# crates/weftos-admin/weftos-app.toml
id       = "app://weftos.admin"
name     = "WeftOS Admin"
version  = "0.1.0"
icon     = "assets/admin.svg"

supported_modes  = ["ide", "desktop", "single-app"]
supported_inputs = ["pointer", "hybrid", "voice"]

[[entry_points]]
kind    = "cli"
flag    = "admin"

[[entry_points]]
kind    = "vscode-command"
command = "weft.admin.open"

[[entry_points]]
kind    = "wake-word"
phrase  = "weft, admin status"

# Rust path: surface IDs resolve to types registered by the
# #[weftos_app] proc-macro inside this crate.
surfaces = [
  "AdminMainSurface",
  "AdminKioskSurface",
]

subscriptions = [
  "substrate/kernel/status",
  "substrate/kernel/processes",
  "substrate/kernel/services",
  "substrate/kernel/logs",
]

influences = [
  "wsp.activate",
  "kernel.restart-service",
  "kernel.kill-process",
]

permissions = [
  "fs:/var/log/weftos",
]

[narration]
"substrate/kernel/services" = """
{{count(services, s -> s.status = "healthy")}} services healthy,
{{count(services, s -> s.status = "at_risk")}} at risk: {{names(services, s -> s.status = "at_risk")}}.
"""
"substrate/kernel/status" = """
Kernel is {{status.state}}. Uptime {{duration(status.started_at)}}.
"""
```

The two manifests are identical except for the `surfaces`
values. Everything else — subscriptions, influences,
permissions, narration, versioning, validation — is
schema-identical across authoring paths.

## Consequences

### Positive
- An app becomes an addressable, installable, versioned,
  permission-gated artefact. The Session 10 §1 gap is closed.
- Validation rules 1–9 are structural, matching ADR-006's
  frame-rejection posture. Malformed manifests cannot reach
  the registry, so runtime code never defends against
  half-initialised apps.
- Single-app kiosk posture (rule 4) is a type-system fact,
  not a runtime policy. Session 10 recommendation 2 is
  enforced at install.
- The three authoring paths (Rust, TOML, agent) share one
  manifest schema and one registry, so first-party, community,
  and agent-authored apps are peers by construction.
- The avatar companion (§6.4) can subscribe to any installed
  app's `[narration]` without knowing anything about that
  app's domain — narration is a property of the declaration,
  not the renderer. This is what makes §6.4 composable.
- Capture permissions live with the app, not with the adapter,
  so ADR-012's governance gate has one place to ask (install
  time), and revocation has one place to enforce (registry
  row flip + adapter stop).

### Negative
- The manifest is large (30+ lines for a non-trivial app).
  Writing one by hand is tedious; a scaffolding command
  (`weaver app new`) is effectively mandatory. Accepted.
- Rule 6 (capture permission requires a consuming adapter)
  forces app authors to declare `subscriptions` before
  `permissions` mechanically, which is less natural than the
  reverse. Accepted — the constraint is more important than
  the ordering ergonomics.
- Agent-authored apps (path 3) require governance approval
  every install, which introduces a human-in-the-loop step.
  Unavoidable given ADR-012's invariants.
- Versioning rules around narration ("add, never rewrite")
  will frustrate authors who want to tweak phrasing. The
  add-then-drop workaround is two releases where one feels
  natural. Accepted — semantic stability wins over phrasing
  convenience.

### Neutral
- `version` is semver, matching Cargo / npm / most of the
  industry. No surprise here.
- The manifest IRI scheme (`app://`) parallels `ui://`
  (ADR-001 primitive canon) and `substrate/` (topic paths).
  WeftOS uses IRIs for every addressable thing; apps don't
  break the pattern.
- Entry-point `kind` is an open enum — new hosts (AR headsets,
  car dashboards, voice-only speakers) add new kinds without a
  schema break.

## Alternatives considered

1. **JSON or RON instead of TOML.** Rejected per Session 10
   recommendation 5. TOML is Cargo-familiar, comment-friendly,
   and strongly typed. JSON has no comments; RON is Rust-only.
   Authors will often hand-edit manifests; TOML's ergonomics
   win.
2. **Embed surface descriptions inline in the manifest.**
   Rejected: manifests for any non-trivial app would become
   thousands of lines, and surface descriptions have their
   own versioning cadence (ADR-016). Keeping them separate
   preserves diff clarity and lets the registry cache
   compiled surfaces independent of manifest changes.
3. **Merge `supported_modes` and `supported_inputs` into
   a single `capabilities` array of `(mode, input)` pairs.**
   Rejected: most apps support every (mode × input) they
   support in either axis — the Cartesian product is almost
   always the intended set. Forcing authors to enumerate 12
   pairs when they mean "mode ∈ {A, B} × input ∈ {X, Y}"
   is bureaucratic. If a genuine exclusion arises (some
   `(mode, input)` combination is nonsensical for a given
   app), the compositor can carry a small `excluded_pairs`
   list as an additive minor change later.
4. **Let `permissions` be implicit — derive from the
   adapters in `subscriptions`.** Rejected: capture is a
   governance concern distinct from adapter presence
   (ADR-012). An app may subscribe to an adapter that
   *could* expose mic but not need mic capture in its use
   case. Requiring explicit declaration forces the author
   to look the user in the eye about each channel.
5. **Skip install-time capture prompts, prompt per-session
   instead.** Rejected: session-level prompting produces
   consent fatigue, the Adtech SDK failure mode (ADR-006
   alternatives §1). Install-time consent with per-goal
   scoping (ADR-008) is the structural answer.
6. **Let narration rules live in the avatar app, not in
   each subscribed app.** Rejected per Session 10
   recommendation 11: decentralising speech content means
   a new installed app instantly has something the avatar
   can say about it; centralising speech content means the
   avatar grows a fork of every domain.

## Related

- **Sessions**: `session-10-app-layer.md` — §1 gap statement,
  §2.1 mode semantics, §3.1 nine properties, §3.2 authoring
  surfaces, §5 narration example, §6.1 admin crosswalk, §7
  milestones, §8 recommendations 2 / 5 / 13.
- **ADRs (consumed)**: ADR-006 (primitive head — structural
  rejection posture mirrored here), ADR-008 (goals and
  consent — consent-id scoping for apps), ADR-012 (capture
  privacy invariants — permission grammar and governance
  gate).
- **ADRs (sibling, this round)**:
  - ADR-016 — surface description format; `surfaces` entries
    point at its files / Rust types.
  - ADR-017 — ontology adapters; `subscriptions` topic paths
    resolve through its registry; dependency check uses its
    capability metadata.
  - ADR-018 — IDE bridge; the `ide.*` namespace in
    `influences` and the `vscode-command` entry-point kind
    are specified there.
  - ADR-019 — input adaptation + avatar + narration rule
    language; `supported_inputs` semantics and the
    `[narration]` template grammar are specified there.
- **Protocol spec**: §7 `surface.compose` — the runtime entry
  point for agent-authored apps (authoring path 3).
- **Code (to land in M1.5)**: `clawft-app` crate (manifest
  parser + validator + registry), `clawft-surface` crate
  (ADR-016 interpreter), `weaver app {install, list, remove,
  launch}` CLI subcommands, SQLite registry schema in
  `crates/clawft-core/src/registry/`. Session 10 §7 sets the
  acceptance criteria — once the WeftOS Admin example above
  loads from manifest and renders in both `ide × pointer` and
  `desktop × pointer`, M1.5 is complete and `BlockKind` is
  deleted per recommendation 13.
