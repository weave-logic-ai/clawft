# ADR-016: Surface Description Format — Declarative Primitive Tree + Bindings

**Date**: 2026-04-20
**Status**: Proposed — symposium round 3
**Deciders**: Compositional UI Symposium (Session 10, app-layer track)

## Context

Session 10 (§3.2) identifies three authoring surfaces for an app:
first-party Rust crates, declarative documents, and agent-authored
documents submitted at runtime. All three must compile to the **same**
internal representation so the composer runtime treats them uniformly
(§3.4). Without a canonical surface description format, every author
invents their own serialisation and the protocol-spec `surface.compose`
verb (§5.2.1) has no schema to carry.

The canon is frozen (ADR-001, 21 IRIs). The primitive head is frozen
(ADR-006, seven mandatory fields + two optionals). The reverse-DDD
mapping is frozen (ADR-010, aggregate root → Surface). What is missing
is the **tree shape** that binds these together into a renderable
whole — the data the `surface.compose` verb actually carries.

Four forces shape this ADR:

1. **Authoring ergonomics.** Session 10 rec. 6 picks TOML as the
   primary surface syntax (consistent with ADR-015's manifest format).
   A Rust builder must exist for crate-level apps but must emit the
   same `SurfaceTree`.
2. **Ontology binding.** Surfaces don't carry data directly — they
   declare subscription paths (`substrate/kernel/cpu-percent`) and
   the composer resolves them at render time via adapters (ADR-017).
3. **Expressiveness without Turing-completeness.** Surfaces need
   projections (`count(services, s -> s.status == "healthy")`) for
   aggregate displays and narration templates (§5, ADR-019), but a
   full scripting language would make governance unauditable.
4. **Testability.** Session 10 rec. 12 requires every reference panel
   to ship as a test case. The format must be loadable under a
   headless harness with ontology snapshots injected.

## Decision

The surface description is a tree of typed primitive nodes plus a
binding-expression sublanguage, authorable in TOML or via a Rust
builder, resolved at each frame by the composer runtime.

### 1. Tree shape — one schema, two syntaxes

Every node in a surface description carries:

| Field | Required | Purpose |
|-------|----------|---------|
| `type` | yes | One of the 21 canon IRIs (ADR-001) — `ui://pressable`, `ui://stack`, `ui://gauge`, … |
| `id` | yes | JSON-Pointer path within the surface (ADR-006 `path`), e.g. `/root/nav/kernel-tab`. Unique. |
| `bindings` | optional | Map from primitive-specific state slot → ontology binding expression (§5). |
| `affordances` | optional | List of `{name, verb, invocations, args_schema?}` (ADR-006 §2 + ADR-019 extension). |
| `attrs` | optional | Primitive-specific static attributes (`axis = "horizontal"`, `allow_zoom = true`). |
| `when` | optional | Boolean binding expression; node is skipped if it evaluates to false (§6). |
| `children` | conditional | Array of child node descriptions; required for container primitives, forbidden on leaves. |

Containers (`ui://stack`, `ui://strip`, `ui://grid`, `ui://dock`,
`ui://modal`, `ui://tabs`, `ui://sheet`, `ui://tree`) take `children`.
Leaves (`ui://chip`, `ui://pressable`, `ui://field`, `ui://toggle`,
`ui://select`, `ui://slider`, `ui://gauge`, `ui://plot`,
`ui://stream-view`, `ui://media`, `ui://canvas`, `ui://foreign`,
`ui://table`) do not. `ui://table` carries its row schema via `attrs`
and its row stream via `bindings`, not via `children`.

The head fields ADR-006 mandates (`confidence`, `variant`,
`mutation-axes`, `privacy`) are **not** authored in the surface
description — they are filled by the composer runtime at compose time.
Authors only write structural identity (`type`, `id`), behavioural
contract (`bindings`, `affordances`), and presentation (`attrs`).

### 2. TOML syntax

Arrays-of-tables (`[[surfaces.root.children]]`) express the tree.
Binding expressions use a leading `$` sigil for ontology paths.
Affordance `invocations` is an inline array.

```toml
# Top-level app metadata lives in the manifest (ADR-015); this file
# is the surface-descriptions section referenced from it.

[[surfaces]]
id          = "weftos-admin/desktop"
modes       = ["desktop"]                     # §9 mode filter
inputs      = ["pointer", "hybrid"]
title       = "WeftOS Admin"

# Ontology subscriptions — walked at compose time, passed to adapters.
subscriptions = [
  "substrate/kernel/status",
  "substrate/kernel/processes",
  "substrate/kernel/services",
  "substrate/kernel/logs",
]

[surfaces.root]
type = "ui://grid"
id   = "/root"
attrs = { columns = 2, rows = 2, gap = 8 }

[[surfaces.root.children]]              # quadrant 1 — overview chips
type = "ui://stack"
id   = "/root/overview"
attrs = { axis = "horizontal", wrap = true }

[[surfaces.root.children.children]]
type     = "ui://chip"
id       = "/root/overview/kernel-status"
bindings = { label = "'kernel: ' + $substrate/kernel/status.state",
             tone  = "$substrate/kernel/status.state == 'healthy' ? 'ok' : 'warn'" }

[[surfaces.root.children.children]]
type     = "ui://chip"
id       = "/root/overview/uptime"
bindings = { label = "'up ' + fmt_duration($substrate/kernel/status.uptime_ms)" }
```

Authoring rules:

- Paths in `id` match the JSON-Pointer of each node's position in the
  tree; the kernel validates equality at compose time.
- Binding expressions always start with a literal or `$path`. Quoted
  strings allow embedded expressions via `+` concatenation — this ADR
  does not introduce `{{ }}` templating (narration rules do, per
  ADR-019, but those are separate artifacts).
- `affordances` entries are tables-of-tables (see the Admin example
  in §10).

### 3. Rust builder (sketch)

For crate-level apps. Compiles to the same `SurfaceTree` as the TOML
parser. Full type spec lives in `clawft-surface` crate when
implemented; this is a sketch of the surface API, not a contract.

```rust
// crates/clawft-surface/src/builder.rs (sketch)

pub fn weftos_admin_desktop() -> SurfaceTree {
    Surface::new("weftos-admin/desktop")
        .modes(&[Mode::Desktop])
        .inputs(&[Input::Pointer, Input::Hybrid])
        .subscribe("substrate/kernel/status")
        .subscribe("substrate/kernel/processes")
        .root(
            grid("/root").cols(2).rows(2).gap(8)
                .child(stack("/root/overview").horizontal()
                    .child(chip("/root/overview/kernel-status")
                        .bind("label", "'kernel: ' + $substrate/kernel/status.state")
                        .bind("tone",  "$substrate/kernel/status.state == 'healthy' ? 'ok' : 'warn'"))
                    .child(chip("/root/overview/uptime")
                        .bind("label", "'up ' + fmt_duration($substrate/kernel/status.uptime_ms)")))
                .child(table("/root/processes")
                    .rows("$substrate/kernel/processes")
                    .column("pid",  ".pid")
                    .column("name", ".name")
                    .column("cpu",  "fmt_pct(.cpu_percent)")
                    .affordance(Affordance::new("kill", "rpc.kernel.kill")
                        .invocations(&[Invoc::Pointer, Invoc::Voice])
                        .args_schema("ontology://kernel/kill-process")))
                // ... remaining quadrants ...
        )
        .build()
}
```

The TOML parser and the builder both emit the same `SurfaceTree`
internal representation; the composer runtime does not know (or care)
which path authored it.

### 4. Resolution pipeline (composer runtime)

The surface description is the *static* shape. At render time the
composer executes this pipeline once per surface, then once per frame
for binding resolution:

**Compose time (once per `surface.compose`):**

1. Parse TOML (or accept the builder's `SurfaceTree`); validate
   against the 21-IRI canon and the head/affordance shape (ADR-001,
   ADR-006).
2. Walk the tree; for each node, subscribe to every ontology path
   referenced in any binding or `when` clause, via the adapters
   declared in the app manifest (ADR-017 owns the adapter trait).
3. Assign a `variant_id` to each node (ADR-006 field 4), stable
   across a single composed tree.
4. Emit the `surface.compose` wire frame carrying the tree with all
   head fields populated.

**Frame time (once per repaint, per session-10 §3.4):**

1. Read the current ontology snapshot for every subscribed path.
2. For each node in depth-first order:
   - Evaluate `when`; if false, skip the subtree.
   - Evaluate each `bindings` expression; substitute the result
     into the primitive's state slot.
   - Intersect `affordances` with `governance.permit(caller, node,
     resource)` (ADR-006 rule 2); this filters by the session's
     input modality (ADR-019) and any goal-level gates (ADR-008).
   - Stamp the `variant_id` onto the emitted primitive.
3. Hand the concrete primitive tree to canon for rendering; collect
   return-signals (ADR-007) and re-emit any state deltas via
   `surface.update` (protocol-spec §5.2.3).

Performance target: the whole pipeline ≤ 2 ms CPU for a 50-primitive
surface, consistent with session-10 §3.4.

### 5. Binding expression language

Small, statically typed, no side effects. Designed so narration rule
templates (ADR-019) reuse the same grammar.

```
expr   := literal | path | call | access | binop | ternary | parens
literal := number | string | bool
path   := '$' segment ('/' segment)*               ; ontology path read
call   := ident '(' (expr (',' expr)*)? ')'        ; function call
access := expr '.' ident                           ; field access
binop  := expr op expr                             ; op ∈ { ==, !=, <, <=, >, >=, &&, ||, +, -, *, / }
ternary := expr '?' expr ':' expr
lambda := ident '->' expr                          ; sole lambda use: list-combinators
```

Functions, frozen set:

| Function | Shape |
|----------|-------|
| `count(list, pred)` | Count elements where `pred` (a lambda) is true |
| `filter(list, pred)` | Subset of `list` where `pred` holds |
| `sort(list, key)` | Lambda extracts sort key; ascending |
| `first(list)`, `last(list)`, `len(list)` | Obvious |
| `fmt_pct(n)`, `fmt_bytes(n)`, `fmt_duration(ms)` | Presentation helpers |
| `exists(path)` | True iff the ontology path has a committed value |

Examples:

```
$substrate/kernel/cpu-percent
count($substrate/kernel/services, s -> s.status == "healthy")
filter($substrate/kernel/processes, p -> p.cpu_percent > 50)
sort($substrate/gh/prs, p -> p.updated_at).first.title
```

Explicitly out of scope: variable assignment, loops, regex, arbitrary
I/O, module imports. Any expression that cannot be statically typed
against the ontology schema is malformed at compose time.

### 6. Conditional rendering

The `when` field takes a boolean binding expression. If it evaluates
to `false`, the node and its subtree are skipped — not rendered, not
stamped with a variant, not subscribed on frame. Used for
governance-gated sections, empty-state hiding, and mode branches.

```toml
[[surfaces.root.children]]
type = "ui://stack"
id   = "/root/admin-tools"
when = "$session/user.role == 'admin'"
# ... children only rendered for admins ...
```

### 7. Composed primitives (local compositions)

ADR-001 keeps `Form`, `Card`, `Toolbar` out of canon to preserve the
reverse-DDD arrow. Authoring apps may still name local compositions
that expand to canon primitives at compile time. They are **not**
new IRIs; they are author-local macros.

```toml
[compositions.Card]
expands_to = "ui://stack"
attrs      = { axis = "vertical", padding = 12, frame = "rounded" }
# children passed through verbatim

[compositions.Form]
expands_to = "ui://stack"
attrs      = { axis = "vertical", gap = 8 }
# magic: the last child is wrapped in a ui://pressable with verb = "submit"
# if attrs.submit_verb is set
```

Expansion happens at load time, before the tree is handed to the
composer runtime. The wire never sees a `Card` — the kernel only ever
sees canon IRIs, preserving the ADR-001 guarantee.

### 8. Testing harness

Session 10 rec. 12 requires every reference panel to ship as an
integration test. The harness:

1. Loads a manifest (ADR-015) and its referenced surface descriptions.
2. Loads a fixture ontology snapshot from a TOML file
   (`tests/fixtures/kernel-healthy.toml`).
3. Runs the resolution pipeline (§4) against a headless egui Context.
4. Asserts on the rendered primitive tree — primitive count per
   IRI, set of affordance verbs present, rendered text matching
   regex patterns, `variant_id` stamped on every node.

```rust
#[test]
fn weftos_admin_desktop_renders_with_healthy_kernel() {
    let surface = load_surface("apps/weftos-admin/surfaces.toml",
                               "weftos-admin/desktop");
    let ontology = load_fixture("tests/fixtures/kernel-healthy.toml");
    let ctx = headless_egui();
    let tree = compose_and_render(&surface, &ontology, &ctx);

    assert_eq!(tree.count_of("ui://chip"), 3);
    assert_eq!(tree.count_of("ui://gauge"), 4);
    assert!(tree.any_affordance_with_verb("rpc.kernel.kill"));
    assert!(tree.rendered_text_matches(r"kernel: healthy"));
    assert!(tree.all_nodes_stamped_with_variant());
}
```

### 9. Mode-specific variants

A single `surfaces.toml` may declare multiple surfaces with
`modes = [...]` filters. At launch, the composer picks the first
surface whose `modes` and `inputs` arrays both contain the session's
mode and input. A fallback wildcard (`modes = ["*"]`) matches any.

```toml
[[surfaces]]
id    = "weftos-admin/desktop"
modes = ["desktop", "ide"]
inputs = ["pointer", "hybrid"]
# Grid layout as in §10.

[[surfaces]]
id    = "weftos-admin/single-app"
modes = ["single-app"]
inputs = ["pointer", "touch", "hybrid"]
# Tabs layout — bottom strip of Status / Processes / Logs.

[[surfaces]]
id    = "weftos-admin/voice"
modes = ["single-app"]
inputs = ["voice"]
# No visual surface; narration contract only (ADR-019 owns this).
```

Per session-10 §6.1, the WeftOS-Admin app ships at least the desktop
Grid variant and the single-app × touch Tabs variant as live
reference panels for M1.5.

### 10. Worked example — WeftOS Admin desktop

Full TOML for the quadrant Grid (abbreviated only where the pattern
repeats). This is the target shape for M1.5 acceptance.

```toml
[[surfaces]]
id     = "weftos-admin/desktop"
modes  = ["desktop", "ide"]
inputs = ["pointer", "hybrid"]
title  = "WeftOS Admin"

subscriptions = [
  "substrate/kernel/status",
  "substrate/kernel/processes",
  "substrate/kernel/services",
  "substrate/kernel/logs",
]

[surfaces.root]
type  = "ui://grid"
id    = "/root"
attrs = { columns = 2, rows = 2, gap = 8 }

# --- Quadrant 1: overview chips -------------------------------------
[[surfaces.root.children]]
type  = "ui://stack"
id    = "/root/overview"
attrs = { axis = "horizontal", wrap = true }

[[surfaces.root.children.children]]
type     = "ui://chip"
id       = "/root/overview/status"
bindings = { label = "'kernel: ' + $substrate/kernel/status.state",
             tone  = "$substrate/kernel/status.state == 'healthy' ? 'ok' : 'warn'" }

[[surfaces.root.children.children]]
type     = "ui://chip"
id       = "/root/overview/services-healthy"
bindings = { label = "fmt_count(count($substrate/kernel/services, s -> s.status == 'healthy')) + ' healthy'" }

# --- Quadrant 2: process table --------------------------------------
[[surfaces.root.children]]
type  = "ui://table"
id    = "/root/processes"
attrs = { columns = ["pid", "name", "cpu", "mem"], sortable = true }
bindings = { rows = "$substrate/kernel/processes" }

  [[surfaces.root.children.affordances]]
  name        = "kill"
  verb        = "rpc.kernel.kill"
  invocations = ["pointer", "voice"]
  args_schema = "ontology://kernel/kill-process"

# --- Quadrant 3: service gauges -------------------------------------
[[surfaces.root.children]]
type = "ui://stack"
id   = "/root/services"
attrs = { axis = "vertical", gap = 4 }

# Gauges generated per service — authoring tools expand this via
# composition; wire-level tree is flat.
[[surfaces.root.children.children]]
type     = "ui://gauge"
id       = "/root/services/mesh-listener"
bindings = { value = "$substrate/kernel/services.mesh-listener.cpu_percent",
             label = "'mesh-listener'",
             tone  = "$substrate/kernel/services.mesh-listener.status" }

  [[surfaces.root.children.children.affordances]]
  name        = "restart"
  verb        = "rpc.kernel.restart-service"
  invocations = ["pointer", "voice"]
  args_schema = "ontology://kernel/restart-service"

# --- Quadrant 4: log stream ------------------------------------------
[[surfaces.root.children]]
type     = "ui://stream-view"
id       = "/root/logs"
bindings = { stream  = "$substrate/kernel/logs",
             formatter = "log_line" }
attrs    = { max_lines = 500, autoscroll = true }
```

Adapters are referenced by implication (the `substrate/kernel/*`
namespace is served by the `kernel` adapter). The explicit adapter
binding contract is ADR-017's concern; this surface description only
declares subscription paths.

## Consequences

### Positive

- Three authoring surfaces (Rust builder, declarative TOML,
  agent-emitted TOML) compile to a single internal tree; the
  composer runtime and protocol wire see one shape.
- The binding expression language is small enough for governance to
  statically audit (no arbitrary code execution on the surface path)
  and large enough to express aggregate projections without a
  separate query layer.
- `when` + `modes` + `inputs` handle layout variation declaratively,
  so a single app artefact covers session-10's 12-cell
  mode × input grid without per-combination forks.
- Reference-panel tests become the definition of acceptance: load
  manifest, inject fixture, render headless, assert on primitive
  counts. Session 10 rec. 12 is structural, not a pledge.
- Local compositions (`[compositions.Form]`) give authors shadcn-like
  ergonomics without breaking ADR-001's 21-IRI canon.

### Negative

- Binding expressions are an interpreter surface — even small, it
  needs its own parser, type-checker, and governance audit.
  Mitigation: freeze the function list in this ADR; additions
  require an ADR.
- Dual syntax (TOML + Rust builder) doubles the parsing surface.
  Mitigation: the builder is the canonical in-memory form; the TOML
  parser emits directly into builder types.
- Large surfaces with many computed bindings will re-evaluate every
  frame; without memoisation the 2 ms budget tightens. Mitigation:
  the composer caches unchanged sub-expressions by ontology version.
- Author-local compositions can shadow canon IRIs accidentally;
  name collisions must fail load. Mitigation: `[compositions.*]`
  names are validated against the 21-IRI list at load time.

### Neutral

- `variant_id`, `confidence`, `mutation-axes`, `privacy` are filled
  at compose time, not in the description. Authors never write them;
  they never collide with head field rules (ADR-006).
- `ui://table` rows are a binding (`rows = "$path"`), not
  `children`. This keeps the table body streaming-native (ADR-001 P3)
  while allowing affordances on the table itself.

## Alternatives considered

1. **JSON Schema-style top-level document.** Rejected: TOML matches
   ADR-015's manifest format, keeps Cargo-like ergonomics, allows
   comments, and is strictly less expressive than arbitrary JSON
   (which is the point — no dynamic keys).
2. **KDL, HCL, or Dhall.** Considered. KDL lacks ecosystem; HCL
   pulls in Terraform-flavoured type coercions that conflict with
   the binding expression grammar; Dhall's totality checker is
   tempting but the parser maturity and toolchain in Rust lag TOML.
   Reconsider if the binding grammar grows.
3. **Full Rust-only — no declarative format.** Rejected: session-10
   §3.2 authoring-surface 2 (community apps without Rust) and
   authoring-surface 3 (agent-emitted at runtime) both require a
   document format. Losing either closes the agent-authoring path.
4. **JSX-style embedded DSL inside Rust.** Rejected: couples the
   declarative format to Rust macros, which agents cannot emit
   directly, and which community authors must compile.
5. **Bindings as a separate side-file.** Considered. Keeps the tree
   pristine but doubles the files per surface and forces every
   change to update two documents. Rejected for ergonomics; may
   revisit if surfaces become very large.
6. **Turing-complete expression language.** Rejected: governance
   cannot statically audit arbitrary code on the render path, and
   the session-10 §3.4 2 ms budget requires bounded-time evaluation.
   The frozen function list is the escape valve — new functions
   require an ADR, giving governance explicit review points.

## Related

- **Sessions**: `session-10-app-layer.md` §3.2 (authoring surfaces),
  §3.4 (resolution pipeline), §6.1 (Admin reference panel), §8
  recs. 6 and 12 (TOML + reference-panel tests).
- **ADRs**: ADR-001 (21-IRI canon this format references),
  ADR-006 (primitive head the composer fills at compose time),
  ADR-010 (reverse-DDD mapping rule the tree expresses),
  ADR-013 (`ui://canvas`), ADR-014 (`ui://modal` modality),
  ADR-015 (app manifest — this document is the value of the
  manifest's `surfaces` field),
  ADR-017 (ontology adapter trait — owns binding-path resolution),
  ADR-018 (IDE bridge — adds `ide.*` affordances that appear in
  `affordances` arrays),
  ADR-019 (input adaptation, voice `invocations`, narration rule
  templates — reuse the §5 expression grammar).
- **Protocol spec**: §5.2.1 `surface.compose` (the wire carrier
  for this tree), §5.2.3 `surface.update` (diffs), §5.8 surface
  versioning.
- **Code targets (M1.5)**: `crates/clawft-surface/` (parser +
  builder + resolution pipeline — new), `crates/clawft-app/`
  (manifest crate — ADR-015), `apps/weftos-admin/surfaces.toml`
  (first real reference panel), `tests/surfaces/` (integration
  harness).
