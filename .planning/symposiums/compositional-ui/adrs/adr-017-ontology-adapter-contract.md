# ADR-017: Ontology Adapter Contract

**Date**: 2026-04-20
**Status**: Proposed — symposium round 3
**Deciders**: Compositional UI Symposium (Session 10 synthesis)

## Context

Session 10 §3.3 establishes that apps get data by subscribing to
**ontology topics** — named paths in the substrate state graph —
populated by **ontology adapters**, one per data source. Today
WeftOS has exactly one adapter and it is not called an adapter:
`crates/clawft-gui-egui/src/live/` polls four kernel RPC verbs on
a 1 s ticker and dumps full JSON arrays into a `parking_lot::
RwLock<Snapshot>`. It implements no common trait, emits no deltas,
re-serialises full arrays every tick, declares no permissions, no
health, no test stub. Acceptable for one source; wrong for seven.

Session 10 §7 (M1.5 acceptance #3) requires the kernel adapter
refactored to **emit ontology topic deltas, not RPC polls**, and
§8 rec. 7 names the artefact. Foundations §3 ("Streaming-native")
makes this non-negotiable: adapters are the stream **producers**
and `open/chunk/commit/cancel` (`protocol-spec §5.7`) is the
protocol on the producer side. ADR-012 binds capture-bearing
adapters to per-goal consent. Without a trait, every new adapter
re-invents back-pressure, governance plumbing, lifecycle, and
failure vocabulary — every seam a silent-capture opportunity. The
`Live` refactor is the forcing function.

## Decision

Every ontology adapter implements **one Rust trait** and declares
its topics structurally. Adapters register at host startup; apps
subscribe via manifest declarations mediated by governance; the
composer snapshots the delta stream into `Substrate::state`.

### 1. The trait

```rust
trait OntologyAdapter: Send + Sync {
    fn id(&self) -> &'static str;                         // "kernel", "git", ...
    fn topics(&self) -> &'static [TopicDecl];             // declared topic paths
    fn permissions(&self) -> &'static [PermissionReq];    // capture/fs/net needed
    fn open(&self, topic: &str, args: Value)
        -> Result<Subscription, OpenError>;               // §open verb
    fn close(&self, sub_id: SubId);                       // §cancel verb
}
```

`Subscription` owns an `mpsc::Receiver<StateDelta>`; deltas are
`append { path, value }`, `replace { path, value }`, or
`remove { path }`. `path` is a JSON-Pointer rooted at the topic
(`/0`, `/by-id/pid-1412`). The composer snapshots deltas into
`Substrate::state`; the ADR-004 state-diff tree consumes them,
so adapter and composer-authored deltas share one vocabulary.
Adapters MUST NOT panic on topic close; `close` is a tombstone
(ADR-009) so late deltas into a revoked stream fail cleanly.

### 2. `TopicDecl` — the adapter's declaration

```rust
struct TopicDecl {
    path:           &'static str,   // "substrate/kernel/processes"
    shape:          &'static str,   // "ontology://process-list" (schema TBD ADR-020+)
    refresh_hint:   RefreshHint,    // event-driven | periodic(ms) | request-only
    sensitivity:    Sensitivity,    // public | workspace | private | capture
    buffer_policy:  BufferPolicy,   // drop_oldest | refuse | block_capped
}
```

- `path` is a literal-prefix pattern; one `*` segment at the
  leaf (`substrate/fs/watched/*`). No globs.
- `shape` is the ontology type URI. Placeholders are acceptable
  at M1.5; formal registry is ADR-020+.
- `refresh_hint` — the composer subscribes vs. one-shot-reads
  (`request-only` is always one-shot).
- `sensitivity` drives install-time prompts and the ADR-012
  tray-chip obligation (§6).
- `buffer_policy` is topic-level; logs and status on the same
  adapter want different rings.

### 3. Registration flow

Two paths. M1.5 requires only the first.

1. **Compile-time (required).** Adapters are Rust structs in
   `crates/clawft-adapters/`; `register_builtins(reg)` installs
   them into a `HashMap<id, Arc<dyn OntologyAdapter>>`.
2. **Dynamic-lib (deferred, spec-only).** `libweft_adapter_<id>`
   exposing a `_weft_adapter_v1()` C ABI entry; loaded from
   `~/.weftos/adapters/` behind a config flag. Governance
   treats dynamic-lib adapters as untrusted by default.

An app manifest (ADR-015) declares required adapters by **id**
under `subscriptions`. At install, governance intersects the
manifest's requested permissions with each referenced adapter's
`permissions()`. The user is prompted **once**, per app, with
the union of grants actually needed. Denial → install fails
closed. Partial approval is not supported at M1.5 (becomes a
deny). At session launch the compositor calls `adapter.open` on
each subscription and wires the receiver into the Substrate
state tree at the declared path.

### 4. Back-pressure and lifecycle

Every topic declares a `buffer_policy`:

- **`drop_oldest`** — append-only streams (logs, plots). Drop
  is not surfaced on the hot path; a gap counter surfaces on
  adapter-health (§7).
- **`refuse`** — singletons and form-state (`editor/cursor`):
  the adapter refuses and logs a warning. For consumers that
  MUST see latest full state.
- **`block_capped`** — default for replace-by-id collections
  (`kernel/processes`). Producer blocks ≤ 50 ms; past that the
  frame drops AND the adapter self-degrades (§7).

Channel depth defaults to 128 frames; logs override to 1000,
singletons to 1. Lifecycle mirrors `protocol-spec §5.7` —
`open`/`chunk`/`commit` on clean close, `cancel` on error. The
composer maps these to ADR-004 tapestry events.

### 5. Reference adapters

M1.5 ships `kernel`; the rest are specified so M1.6–M1.9
implement without re-litigating shape. Wire schema deliberately
unfixed — placeholder URIs; binding schemas are ADR-020+.

- **`kernel`** (M1.5, required). Refactored from `live/*`.
  Subscribes to the kernel event log over the existing
  `clawft_rpc::DaemonClient` UDS. Topics: `substrate/kernel/
  {status, processes, services, logs}` — singleton, list-by-pid,
  list-by-name, log ring (N=1000, `drop_oldest`); rest
  `block_capped`, `status` is `refuse`. All `workspace`.
  Permissions: none.
- **`git`** (M1.6+). `git` CLI via `std::process::Command`;
  poll 2 s + `notify` watch on `.git/HEAD` for instant branch-
  change. Topics: `substrate/git/{branch, status, log, diff-
  preview}`. Permissions: `fs:/workspace/.git`. Sensitivity:
  `workspace`.
- **`gh`** (M1.6+). GitHub API via `octocrab` / `reqwest`.
  Topics: `substrate/gh/{issues, prs, workflows, deployments}`.
  Permissions: `net:api.github.com` + read `gh` token at
  `~/.config/gh/hosts.yml`. Sensitivity: `workspace`.
- **`workspace`** (M1.6, hoisted from M2). IDE-side editor
  bridge — consumer of the ADR-018 message schema. Topics:
  `substrate/editor/{buffer, cursor, diagnostics, focus,
  terminal, tasks}`. Permissions: ide-bridge handshake.
  Sensitivity: `workspace`.
- **`fs`** (M1.7+). `notify` watcher. Topics: `substrate/fs/
  {tree, watched}` per declared root. Permissions:
  `fs:/<path>` per root. Sensitivity: per-path (private home
  watch = `private`; workspace watch = `workspace`).
- **`lsp`** (M1.7+). Proxies the active LSP through the ide-
  bridge. Topics: `substrate/lsp/{symbols, diagnostics,
  references}`. Permissions: ide-bridge. Sensitivity:
  `workspace`.
- **`deployment`** (M1.7+). Vercel/Fly/k8s. Topics:
  `substrate/deploy/{environments, health, latest-release}`.
  Permissions: `net:<provider>` + credential. Sensitivity:
  `workspace`.

### 6. Sensitivity and governance (ADR-012 binding)

Sensitivity drives two behaviours:

1. **Install-time prompt copy.** `public` silent; `workspace`
   one-line summary; `private` full disclosure dialog;
   `capture` REQUIRES the ADR-012 per-goal signed
   `CapabilityGrant` flow and CANNOT be granted at install
   alone — the grant is deferred to goal-start.
2. **Tray-chip obligation.** A `capture`-tagged adapter
   participates in the ADR-012 kernel-composed tray chip.
   Startup is **blocked** until the capture `consent_id` is
   valid for the active goal. Stale/expired consent →
   adapter **auto-pauses** emission (state freezes on last
   valid delta; adapter-health → `degraded` with `last_error
   = "consent-expired"`). MUST NOT emit any delta after revoke
   until a new grant is issued — structural precondition for
   foundations non-negotiable 4 ("no dark recording, ever").

Capture adapters also inherit the ADR-006 head rule: any
primitive rendering their state carries `privacy-flags` with
a populated `consent-id` or the frame is malformed at the
kernel boundary.

### 7. Offline / intermittent — `substrate/meta/adapter/<id>/health`

Session-10 §9 lists offline behaviour as open; this ADR closes
it at the adapter level. Every registered adapter publishes a
health topic at `substrate/meta/adapter/<id>/health`, shape
`ontology://adapter-health`, payload `{ state: connected |
degraded | offline, last_success, last_error, gap_count }`.
Surfaces bind against this to show stale-chip affordances,
reconnect buttons, and narration suppression without special-
casing any adapter. A surface wanting "stale badge on all data
when the source is offline" binds `substrate/meta/adapter/*/
health` and applies one uniform treatment. This makes the rest
of the app layer offline-correct by construction.

### 8. Performance budgets

Per-event emission cost MUST be ≤ 500 µs on a modern laptop,
measured wall-clock from the moment source data is available to
the moment the `StateDelta` is on the bounded channel. If the
adapter needs more work — diffing large blobs, parsing
`git log`, hashing trees — it MUST do the work **off the poller
thread** and only publish deltas on completion. The budget is
stated because today's pattern (`live/native_live.rs` pulls
four full kernel arrays each tick, writes wholesale into one
`RwLock`) does NOT scale past one source; each repeat imposes
1 s-multiplied latency on every bound surface. Adapters are
stream producers (foundations §3), not pollers with a shared
scratchpad. A `clawft-adapters/benches/` target measures each
shipped adapter and fails CI on regression.

### 9. Test discipline

Every shipped adapter has two companions:

- **A mock/stub** emitting canned deltas from a fixture file.
  Implements the same trait, so surfaces cannot tell it apart.
  Default for all app-level tests (ADR-016 §testing).
- **An integration test** against a real source: the kernel
  daemon for `kernel`, `git init` in `tempfile::tempdir()` for
  `git`, wiremock for `gh`, a synthetic LSP for `lsp`. Separate
  CI lane (slower, still required).

## Consequences

### Positive
- `Live` refactor produces a reusable pattern; adding `git`,
  `gh`, `workspace`, `fs`, `lsp`, `deployment` is trait-impl
  only, no per-adapter composer glue.
- Streaming-native enforced at the signature — the return type
  is `Receiver<StateDelta>`, not a snapshot; poll-and-replace
  cannot be expressed.
- Governance is install-time, not per-tick — one signed grant
  per adapter per app, cached in the goal aggregate (ADR-008).
- ADR-012 tray-chip obligation preserved: capture adapters
  cannot emit without a live `consent_id`; revoke halts
  emission within one kernel tick.
- Offline is a uniform topic, not per-adapter special-casing.
- Mock adapters make app-level tests hermetic by construction.
- Performance budget stated, measurable, CI-enforceable.

### Negative
- Refactoring `Live` touches the `Desktop` shell, wasm bridge,
  and daemon client — M1.5 cannot ship without it.
- Seven adapters × trait boilerplate is more code than seven
  bespoke pollers; accepted because the trait is the downstream
  integration surface.
- Bounded channels mean deltas can be dropped under load; the
  health-topic gap counter is how we stay honest with the user.
- `refuse` and `block_capped` impose back-pressure on producer
  threads; a slow consumer measurably slows an adapter.
  Differs from today's silent-overwrite pattern, but correct.

### Neutral
- Dynamic-lib registration specified but deferred; first-party
  adapters all ship compile-time in M1.5–M1.9.
- Wire shape per adapter is left to ADR-020+; this ADR fixes
  only topic set, shape-URI placeholders, and delta vocabulary.

## Alternatives considered

1. **Keep `Live` as the de facto adapter; add siblings.** —
   Rejected. No common trait means every new source re-
   implements lifecycle, back-pressure, permissions, health;
   the composer dispatches by adapter type. Status quo, exactly
   what session-10 §3.3 flags as missing.
2. **Synchronous `Snapshot` on demand (pull model).** —
   Rejected. Violates foundations §3; forces the compositor to
   poll. `open/chunk/commit/cancel` is push; adapters must
   match.
3. **Deltas as arbitrary `Value` blobs (no vocabulary).** —
   Rejected. Without structured deltas the composer cannot
   diff into the ADR-004 tapestry; it would deep-compare every
   blob every tick. Three verbs are the minimum.
4. **Governance checked per-delta.** — Rejected per ADR-012
   invariant 2 and §8; grants are per-goal, per-delta checks
   blow the budget by orders of magnitude.
5. **Sensitivity as a runtime label on each delta.** —
   Rejected. Install-time prompts need a static answer.
6. **Skip adapter-health; let apps detect offline by timeout.** —
   Rejected. Every app reinvents the stale-data heuristic;
   some get it wrong. A uniform health topic is the single
   source of truth at one-topic-per-adapter cost.

## Related

- **Sessions**: `session-10-app-layer.md` (§3.3, §7 M1.5
  acceptance, §8 rec. 7, §9 open questions closed here),
  `session-5-renderer-contracts.md` (§streaming pattern),
  `session-6-protocol-design.md` (§9 governance + consent).
- **Foundation**: §3 ("Streaming-native"), §"Non-negotiable
  privacy constraints" 1–4.
- **ADRs**: ADR-004 (state-diff tree, consumer of deltas),
  ADR-005 (verbs the adapter implements), ADR-006
  (`privacy-flags` from adapter sensitivity), ADR-008 (per-goal
  consent), ADR-009 (tombstones), ADR-012 (capture privacy —
  binding in §6), ADR-015 (manifest permission grammar —
  consumed), ADR-016 (surface testing — consumes mocks),
  ADR-018 (IDE bridge — consumed by `workspace`/`lsp`),
  ADR-020+ (shape-URI schemas — deferred).
- **Code**: `crates/clawft-gui-egui/src/live/*` (before —
  refactored in M1.5), `crates/clawft-adapters/` (new crate —
  after), `crates/clawft-core/` (substrate + composer).
