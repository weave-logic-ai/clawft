# Symposium: ExoChain Logging — Governance Specialist Findings

**Author:** governance-gate-specialist
**Date:** 2026-04-22
**Scope:** Tags, K-levels, and the governance gate integrated into the proposed
multi-channel log fabric.
**Peer documents:** `01-chain-schema.md`, `02-kernel-lifecycle.md`,
`03-storage-research.md` (siblings — not yet present; this is doc 04).

---

## k-level-audit

### TL;DR

**The string "K-level" does not exist anywhere in the WeftOS codebase as a
runtime concept.** The `K0…K6` references that `rg` turns up are
**milestone/phase markers** in doc comments (K0 = early bootstrap milestone,
K2b-G2 = milestone K2b gate 2, K6 = mesh milestone, etc.). They label *when* a
feature was landed, not *which privilege band a runtime event is in.*

What the codebase *does* have is **three parallel-but-unrelated severity/tier
dimensions**. The user's K-level idea therefore needs to pick one (or be a new
dimension orthogonal to all three). Here is the full audit.

### 1. `LogLevel` — human-facing console severity

**File:** `crates/clawft-kernel/src/console.rs:58-80`

```rust
pub enum LogLevel {
    Debug,  // rank 0
    Info,   // rank 1
    Warn,   // rank 2
    Error,  // rank 3
}
```

- Used by `BootEvent` (`console.rs:71-111`) and the ring-buffered
  `KernelEventLog` (`console.rs:181-310`).
- Total ordering is baked into the private closure at
  `console.rs:274-282` (`filter_level`).
- **No `Fatal` variant today.** If the proposal wants fatal errors as a
  distinct tier, it must be added here.
- No semantic connection to governance — a `LogLevel::Error` is not gated,
  it is just pushed into the ring buffer.

### 2. `RuleSeverity` — governance rule severity

**File:** `crates/clawft-kernel/src/governance.rs:89-98`

```rust
pub enum RuleSeverity { Advisory, Warning, Blocking, Critical }
```

- `PartialOrd`/`Ord` derived, so `Advisory < Warning < Blocking < Critical`.
- Currently only **`Blocking`** and **`Warning`** are emitted from any
  call site; `Critical` and `Advisory` are defined but unused (grep
  `crates/clawft-kernel/src/gate.rs:548..976` — every rule uses
  `Blocking` except the cron-warn rule at line 605 and a stream-warn at
  976 which use `Warning`).
- Drives `GovernanceDecision` inside `GovernanceEngine::evaluate()`
  (`governance.rs` below line 400). `Blocking` + magnitude-over-threshold
  → `Deny`; `Warning` → `PermitWithWarning`.

### 3. `AuditLevel` — governance audit detail

**File:** `crates/clawft-kernel/src/environment.rs:154-164`

```rust
pub enum AuditLevel { SessionSummary, PerAction, PerActionWithEffects }
```

- Per-environment, lives on `GovernanceScope` (`environment.rs:89-91`).
- Drives *how verbose* the audit record is — it is not a filter on
  whether something is logged, it is a descriptor on what columns get
  populated.
- **Also note:** there is a completely unrelated
  `ServiceAuditLevel { Full, GateOnly, .. }` at
  `crates/clawft-kernel/src/service.rs:76-100` which is yet another
  dimension (per-service). This is a latent source of confusion already.

### 4. `EffectVector` dimensions — the 5D scorer, not a tier

**File:** `crates/clawft-kernel/src/governance.rs:111-201`

```rust
pub struct EffectVector { risk, fairness, privacy, novelty, security }
```

- Continuous `f64` per dimension, not a tier.
- `magnitude()` is the L2 norm; `score()` consults the optional
  `GovernanceScorerModel` (EML model at
  `crates/clawft-kernel/src/eml_kernel.rs`, the scorer is referenced
  from `governance.rs:163-178` with a `NOTE(eml-swap)` breadcrumb).
- **This is the closest thing to a K-level today** because it already
  quantifies "how much this action matters." But it is a vector, not a
  scalar tier; mapping it down to K is a **lossy** operation.

### 5. EML models hosting severity-like state

**Files:**
- `crates/clawft-kernel/src/eml_kernel.rs:117-213` — `RestartStrategyModel`
  (inputs: `failure_count`, `failure_type`, `uptime`, `system_load`;
  outputs: `(delay_ms, should_retry)`).
- `crates/clawft-kernel/src/eml_kernel.rs:226-320` — `HealthThresholdModel`
  (outputs: degraded/failed consecutive-failure thresholds).
- `crates/clawft-kernel/src/governance.rs:334-397` —
  `GovernanceEngine::with_scorer` plugging in
  `GovernanceScorerModel`.
- Persistence metadata: `eml_persistence.rs:12-13, 29-70`.

None of these expose a K-level; they *could* consume one as an input
feature. See `gate-interaction` below.

### 6. Tree-calc `Form` — orthogonal structural classifier

**File:** `crates/clawft-treecalc/src/lib.rs:49-95`

```rust
pub enum Form { Atom, Sequence, Branch }
```

This is a **structural** triage (empty / all-equal / mixed) over children
or trajectory deltas. Not a severity at all — mentioned only because the
brief explicitly asked the tree-calc dispatcher be checked. It is
unrelated to the tier question and should stay that way.

### 7. Routing `check_level_ceiling`

**File:** `crates/clawft-core/src/routing_validation.rs:514-572`

This is the A2A router's tier ceiling (ADR-related model-tier routing:
Booster < Haiku < Sonnet < Opus). **Completely unrelated** to
governance severity, but shares the word "level" — flagging to prevent
naming collision if K-level enters the codebase.

### 8. Chain event kinds — categorical, no tier

**File:** `crates/clawft-kernel/src/chain.rs:249-700+`

Roughly 80+ `EVENT_KIND_*` constants. The `kind` is a dotted string
like `cluster.peer.add`, `stream.window_commit`, `wasm.fs.write`,
`governance.deny`. Useful as a dictionary-coded `service.kind` axis
(see `tag-contract` below) but there is **zero tier metadata attached
to them today**. That is the opening the symposium is asking us to
fill.

### Summary table

| Concept              | File                      | Cardinality | Ordered? | Purpose                          |
|----------------------|---------------------------|-------------|----------|----------------------------------|
| `LogLevel`           | `console.rs:58`           | 4           | Yes      | Human console filter             |
| `RuleSeverity`       | `governance.rs:89`        | 4           | Yes      | Governance action outcome        |
| `AuditLevel`         | `environment.rs:156`      | 3           | Partial  | Audit record verbosity           |
| `ServiceAuditLevel`  | `service.rs:76`           | 2+          | No       | Per-service audit scope          |
| `EffectVector`       | `governance.rs:117`       | `f64^5`     | —        | 5D risk scorer                   |
| Routing tier         | `routing_validation.rs`   | 4           | Yes      | Model-tier routing               |
| `Form`               | `treecalc/lib.rs:49`      | 3           | No       | Structural triage                |
| Chain `EVENT_KIND_*` | `chain.rs:260-700`        | ~80         | No       | Category label                   |

**Bottom line:** if K-level lands, it must pick its lane. My recommendation
(see `taxonomy` below) is that it be **orthogonal privilege tier**,
distinct from `LogLevel` (severity), `AuditLevel` (detail), and
`EffectVector` (continuous effect). Adding it as a first-class field on
`ChainEvent` is a one-liner; making it *meaningful* is not.

---

## taxonomy

### Is K-level monotone-with-severity, monotone-with-privilege, or orthogonal?

**Argument for severity (the naïve reading of the user's proposal):**
- The proposal orders K0 (raw frame data) → K5 (sovereign decisions).
- Severity goes Debug → Fatal; privilege goes user → root.
- If interconnect chatter is K1 and errors are K3, then errors outrank
  chatter — that is severity, because chatter can be perfectly healthy
  and errors cannot.

**Argument for privilege (what I think the user actually wants):**
- "Governance mutations" (K4) and "sovereign decisions" (K5) are
  privilege concepts — only authorized actors can emit them, and they
  change the policy surface.
- An error at K3 is not *more privileged* than a heartbeat at K1; it is
  just noisier. Privilege and severity diverge here.
- The word "sovereign" at K5 is a dead giveaway: this is a privilege
  axis.

**The resolution — orthogonal, and the proposed 0..=5 conflates them.**

I argue K-level should be **the privilege / impact axis, not the severity
axis**. Severity already exists as `LogLevel`. A single scalar cannot
simultaneously express "this is a heartbeat" (K1 chatter) *and* "this
heartbeat failed and is now an error" (K3 error) — those are two
dimensions: K=1 (interconnect privilege band) and severity=Error.

Concrete counter-example to monotone-severity: a **warn-level peer-key
rotation** is strictly more dangerous than a **fatal-level rendering
shader crash** — the former changes the trust topology, the latter
hangs a GPU. Privilege says K4 > K0; severity says Warn < Fatal.

### Revised proposal

Split into **two orthogonal fields**, both present on every event:

| Field      | Type   | Cardinality | Axis                           |
|------------|--------|-------------|--------------------------------|
| `k_level`  | `u8`   | 0..=5       | Privilege / governance impact  |
| `severity` | `enum` | 5           | Operational urgency            |

Then the user's original taxonomy becomes:

| K | Name            | Default severity for… | Gate? | Typical kind prefixes                    |
|---|-----------------|-----------------------|-------|------------------------------------------|
| 0 | Raw frames      | Not logged            | No    | (opaque — never enters chain)            |
| 1 | Interconnect    | Debug/Info            | No    | `heartbeat.*`, `peer.envelope`, `stream.window_commit` |
| 2 | Service lifecycle | Info/Warn           | No    | `service.start`, `service.idle`, `health.*` |
| 3 | Errors          | Warn/Error/Fatal      | **Yes** (rate-only) | `*.error`, `*.fatal`, `invariant.*` |
| 4 | Governance mutations | Info/Warn        | **Yes** (effect-scored) | `cluster.peer.add/remove`, `capability.*`, `auth.credential.rotate`, `policy.*` |
| 5 | Sovereign decisions | Warn/Error         | **Yes** (effect-scored + human approval) | `governance.genesis`, `quorum.change`, `rvf.shard.rotate`, `restart.strategy.escalate` |

### Where does streaming go?

User said: *"the interconnect I believe should fall on a lower K level
than the streaming."* I disagree with the phrasing but agree with the
intent once we separate the axes.

- `stream.window_commit` (every N seconds, summarising a media window) is
  **K1** — it is interconnect/infrastructure chatter. It is not
  individually important; the chain has it for replay and integrity.
- `stream.connect` / `stream.manifest` (one-shot, session-establishing)
  is **K2** — service lifecycle, because it *creates* the window-commit
  cadence.
- A K-elevated variant `stream.session.grant` that authorizes a new
  tenant to stream is **K4** — governance mutation.

So the user's instinct was right but he inverted the weights — the
frequent thing is lower K, not higher. (Stream *manifests* are lifecycle,
stream *frames* are interconnect.)

### Where does K-level live on the event?

Proposal: **add `k_level: u8` to `ChainEvent`** at `chain.rs:141-163`,
defaulted to `2` on serde (so existing stored events read back as
lifecycle — the least surprising default for old data).

```rust
pub struct ChainEvent {
    pub sequence: u64,
    // ...
    pub source: String,
    pub kind: String,
    #[serde(default = "default_k_level")]
    pub k_level: u8,
    pub payload: Option<serde_json::Value>,
}
fn default_k_level() -> u8 { 2 }
```

Chain-schema specialist (doc 01) must approve the hash change — adding
`k_level` to `ChainEvent` changes `compute_event_hash` if we decide to
hash-commit it. My recommendation: **hash-commit it** (put it after
`kind` in the hash preimage at `chain.rs:224-244`). Otherwise K-level
is mutable after the fact and cannot be trusted for audit.

---

## gate-interaction

### Should emission ever be gated?

Today only **write-side** actions go through the gate — things like
`add_peer`, `tool.exec`, capability changes. **Read-only "I'm logging
that X happened"** events do not. The question is whether the log
fabric should add a gate hop.

**My position: no for K0/K1/K2, yes-but-rate-only for K3, yes for K4/K5.**

### K0 — never gated (trivially — not logged)

Raw frame bytes are opaque to governance. They do not enter the chain.
Nothing to gate.

### K1 — never gated

- `heartbeat.*` fires at 5s cadence per peer (`config/kernel.rs:39-44`).
  Gating it would add a hot-path synchronous hop through the
  `GovernanceEngine` for zero benefit — a heartbeat cannot be
  meaningfully blocked.
- `peer.envelope` (mesh_runtime.rs:282-308) and `stream.window_commit`
  (`stream_anchor.rs:196-202`) are in the same bucket.
- Gating these violates the "governance is on the **action**, not on
  the **notification of an action**" rule from the three-branch model.

### K2 — never gated

Service lifecycle transitions (`service.start`, `service.idle`,
`health.degraded`) are derived from kernel state changes that were
**already** gated when the *triggering* action went through
`CapabilityGate` / `GovernanceGate`. Double-gating the emission is
redundant and invites deadlocks (governance decision → event → gate
check → …).

### K3 — rate-gated, never blocked

This is the interesting case. The proposal is to route errors through
the gate **not to block them** but to let the scorer see them:

- Rate-limit by sliding window: "≥ 10 fatals/min from one service."
- Emit `governance.anomaly` (a new `K5` event) when the window trips.
- The K3 event itself is still logged — denial would lose information.

Concretely, the `GovernanceGate` at `gate.rs:380-459` would grow a
sibling method `observe(kind, severity, agent_id)` that feeds a counter
into the scorer but **always returns** `GateDecision::Permit`.

**This interacts with `RestartStrategyModel`.** If a service throws 10
fatals/min, two things happen in parallel:

1. `RestartStrategyModel::record()` at `eml_kernel.rs:187-207` gets
   each fatal as a training sample and predicts longer delays.
2. The new gate-observer trips the anomaly threshold and emits
   `governance.anomaly` → the judicial branch can quarantine the
   service before the next restart attempt.

**Race resolution:** governance wins. The gate's quarantine is
synchronous on the next action by that service; the restart tracker is
asynchronous and works on longer timescales. Concretely, when
`governance.anomaly` fires, write a `service.quarantine` desired-state
entry that the reconciler (`chain.rs:504-513`,
`EVENT_KIND_RECONCILER_*`) reads on its next tick. The restart model
still learns from the fatals, but the reconciler refuses to restart a
quarantined service.

This gives us the user's desired "auto-quarantine on fatal burst" without
bolting it into the scorer.

### K4 — fully gated (current behavior)

Already the case today. `add_peer`, `capability.revoke`, `policy.*` all
go through `CapabilityGate` / `GovernanceGate`. The emission of the
corresponding chain event is a *consequence* of the gate decision, not
a separate gate hop. Nothing changes here.

### K5 — fully gated + human approval

`GovernanceScope::human_approval_required`
(`environment.rs:81-83`) already supports this, and
`GovernanceDecision::EscalateToHuman` (`governance.rs:212`) maps to
`GateDecision::Defer` at `gate.rs:405-409`. What is missing is that K5
is not currently detectable from the action string — a
`capability.revoke` (K4) and a `quorum.change` (K5) both look like
strings to the gate. Tagging action requests with `k_level` lets the
gate apply strictness that scales with privilege:

```rust
// Proposed in GovernanceGate::check()
if k_level >= 5 && !self.engine.is_human_approval_required() {
    // K5 with no human in the loop is itself a policy violation
    return GateDecision::Deny { reason: "k5 requires human approval",
                                receipt: None };
}
```

### Should the gate rate-limit itself?

Yes — the gate is a synchronous hot path. Adding `observe()` without a
ring-buffer LRU or token bucket will regress hot-path latency. The
`KernelEventLog::with_capacity` pattern (`console.rs:201-210`) is the
right prior art; copy it for the rate counter.

---

## ontology-hook

### Where the ontology lives today

**File:** `crates/clawft-substrate/src/snapshot.rs:32-260`

```rust
pub struct OntologySnapshot(pub BTreeMap<String, Value>);
```

The substrate is a path-keyed JSON tree. `OntologyAdapter`s publish
into it (`lib.rs:5-91`). The GUI reads from it
(`gui-egui/src/live/native_live.rs:66-157` registers six adapters:
Kernel, Network, Bluetooth, Mesh, Chain, Microphone).

Each adapter declares a `shape: "ontology://…"` string. Grep from the
audit: `ontology://chain-status`, `ontology://mesh-status`,
`ontology://mesh-nodes`, `ontology://audio-level`,
`ontology://bluetooth`. There is already a convention for IRI-shaped
identifiers.

### Proposal: every log event materialises as a resource-tree node

```
/log/<cluster>/<node>/<service>/<kind>          → latest event JSON
/log/<cluster>/<node>/<service>/<kind>/_ring    → ring of last N
/log/<cluster>/<node>/<service>/<kind>/_count   → counter
/log/<cluster>/<node>/<service>/<kind>/_k_level → u8 constant
```

Backed by a `LogOntologyAdapter` in `clawft-substrate/src/` that
subscribes to the new log-fabric channel and publishes into the
substrate tree with TTL eviction (so heartbeat chatter doesn't bloat
the tree). TTL tiered by K-level:

| K | TTL for latest value       | TTL for ring buffer |
|---|----------------------------|---------------------|
| 1 | 30 s (chatter)             | 1 min               |
| 2 | 5 min (lifecycle)          | 15 min              |
| 3 | 1 hour (errors)            | 1 day               |
| 4 | never (governance)         | never               |
| 5 | never (sovereign)          | never               |

K4 and K5 events are **canonically durable** — they belong in the chain
forever, so the substrate mirror should also be durable.

### Why the tree helps the ontological engine

- Tags are inherently flat key-value pairs; a tree is hierarchical.
- The ontology engine reasons about *containment* (is `heartbeat` a
  kind of `interconnect`?) and *adjacency* (two events on the same
  service).
- A pre-categorised tree lets the engine walk `/log/<cluster>/<node>/`
  and enumerate what a node is doing without scanning the flat event
  stream.
- GUI composers (`gui-egui/src/surface_host/compose.rs:93-453`) already
  read `OntologySnapshot` — they get log visibility for free.

### Read-only vs published-into

The substrate's rule is that adapters *publish*; composers *read*. The
log fabric is a *producer* of substrate state. That makes it an
`OntologyAdapter` living beside the existing six, not a cross-cutting
mutation into them.

---

## tag-contract

### Mandatory vs optional fields — cardinality budget

| Field            | Type                       | Mandatory | Cardinality budget | Notes |
|------------------|----------------------------|-----------|-------------------|-------|
| `cluster`        | `NodeId`                   | yes       | 10¹–10³           | Existing: `cluster.rs` |
| `node`           | `NodeId`                   | yes       | 10²–10⁵ per cluster | |
| `service`        | `&'static str`             | yes       | ≤ 256             | **dictionary-coded** |
| `kind`           | `&'static str`             | yes       | ≤ 1024            | maps to `EVENT_KIND_*` |
| `k_level`        | `u8` (0..=5)               | yes       | 6                 | see `taxonomy` |
| `severity`       | enum                       | yes       | 5 (Debug/Info/Warn/Error/Fatal) | extend `LogLevel` with `Fatal` |
| `agent`          | `Option<AgentId>`          | no        | 10³–10⁶           | |
| `correlation_id` | `Option<Uuid>`             | no        | unbounded         | for tracing |
| `labels`         | `BTreeMap<String, String>` | no        | cap 8 entries, 256 B total | escape hatch |

### Conflicts with existing `ChainEvent`

Look at `chain.rs:141-163`:

```rust
pub struct ChainEvent {
    pub sequence: u64,          // provided by chain
    pub chain_id: u32,          // ~ cluster (see note)
    pub timestamp: DateTime<Utc>,
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
    pub payload_hash: [u8; 32],
    pub source: String,         // ~ service
    pub kind: String,           // same
    pub payload: Option<serde_json::Value>,
}
```

**Mapping:**

| Proposed tag     | Existing field        | Gap? |
|------------------|-----------------------|------|
| `cluster`        | `chain_id` (u32)      | **Mismatch.** `chain_id` is a numeric chain selector, not a `NodeId`. Either rename the tag or add a separate `cluster_id` indirection table. |
| `node`           | *(missing)*           | **Gap.** Nothing on `ChainEvent` identifies the emitting node. Needs to be added — probably `node_id: Option<String>` (matches `GovernanceRequest::node_id` at `governance.rs:246-247`). |
| `service`        | `source` (String)     | ok — same semantic. Rename on write? Probably not worth churn. |
| `kind`           | `kind` (String)       | ok |
| `k_level`        | *(missing)*           | see `taxonomy`, add as field |
| `severity`       | *(missing)*           | Gap. Today severity is only on `BootEvent`, not `ChainEvent`. Needs addition. |
| `agent`          | inside `payload`      | Inconsistent. Every event that cares puts `agent_id` in the payload JSON. Promote to a top-level `agent_id: Option<String>` field. |
| `correlation_id` | *(missing)*           | Gap. Not hard to add; governance-gated events already carry an implicit correlation via `evaluated_rules` + timestamp. |
| `labels`         | inside `payload`      | Fine — keep in payload, don't promote. |

### Minimum diff to `ChainEvent`

Add five fields:

```rust
pub struct ChainEvent {
    // existing fields ...
    #[serde(default = "default_k_level")]
    pub k_level: u8,
    #[serde(default)]
    pub severity: LogLevel,          // defaults to Info
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,  // uuid as string, avoid uuid crate churn
}
```

### Dictionary coding

`service` and `kind` are `&'static str` at emission but `String` on the
chain. With 80+ kinds today and ≤ 1024 budget, a u16 dictionary lookup
table held in the `ChainManager` is cheap and makes on-disk events 6 B
instead of ~30 B per event. **Out of scope for this doc** (see doc 03
storage research) but call-site-compatible — the API stays string-typed,
the wire format is coded.

### What stays out of tags

- Media frame payload bytes — those are the K0 layer.
- Cryptographic material (hashes, signatures) — those are in the event
  already, no need to duplicate into tags.
- User-submitted free-form text — goes in `payload`, never in tags (tag
  cardinality is strict).

---

## migration

### Restating the three options

- **A.** Retrofit: rewrite all existing `EVENT_KIND_*` emissions with
  synthetic K-levels + tag defaults in a migration pass. New events
  carry full tags. Old events on disk stay as-is but a migration
  script rewrites them in place.
- **B.** Version the envelope: bump the `ChainEvent` serde version.
  Old events get K-level *inferred* at read time from the `kind`
  string via a lookup table. New events carry it natively.
- **C.** Burn them: new chain, new events only. Old events stay but the
  log fabric ignores them.

### My recommendation: **Option B** (version + infer-at-read-time).

**Why not A:**
- Rewriting on-disk chain data breaks hash linkage (`prev_hash` /
  `hash` at `chain.rs:148-155`). The chain is append-only and
  cryptographically chained; rewriting it invalidates every downstream
  verifier. Full stop.
- Even without the hash issue, a global rewrite requires taking the
  chain offline for the duration, which in a live mesh is a quorum
  event.

**Why not C:**
- Existing events have high provenance value — `governance.genesis`,
  `cluster.peer.add` from last week, etc. Discarding them is a data
  loss event and a compliance problem (audit trails should not have
  gaps).
- The chain is the system of record; "burn and replace" is not an
  allowed operation on a system of record.

**Why B works:**
- The 80+ `EVENT_KIND_*` constants in `chain.rs:249-700` are already a
  static set. A `const fn infer_k_level(kind: &str) -> u8` lookup is
  straightforward — I sketched the full mapping below.
- `serde(default)` for the new fields makes old events deserialise
  cleanly; the reader injects the inferred K-level into the in-memory
  view so upstack code sees a uniform interface.
- Hash preimage change is additive: old events computed hashes without
  `k_level`; new events include it. The `chain_id` disambiguates —
  events with `chain_id` ≥ the genesis of the new scheme hash
  `k_level`; events before that sequence don't. One conditional in
  `compute_event_hash` (`chain.rs:224-244`).

### Inferred K-level table (for Option B)

```rust
fn infer_k_level(kind: &str) -> u8 {
    match kind {
        // K1 interconnect
        "peer.envelope" | "heartbeat" | "stream.window_commit"
            => 1,
        // K2 lifecycle
        "service.start" | "service.stop" | "service.idle" |
        "health.degraded" | "health.ready" | "workspace.create" |
        "session.create" | "session.destroy"
            => 2,
        // K3 errors/invariants  — detected by suffix
        k if k.ends_with(".error") || k.ends_with(".fatal") ||
             k.starts_with("invariant.") || k == "eml.drift"
            => 3,
        // K4 governance mutations
        "cluster.peer.add" | "cluster.peer.remove" |
        "cluster.peer.state" | "mesh.peer.add" | "mesh.peer.remove" |
        "capability.revoked" | "capability.elevate" |
        "auth.credential.register" | "auth.credential.rotate" |
        "auth.token.issue" |
        "tool.deploy" | "tool.version.revoke" | "tool.signed" |
        "env.register" | "env.switch" | "env.remove" |
        "governance.permit" | "governance.warn" |
        "governance.defer" | "governance.deny" |
        "policy.update" | "sandbox.sudo.override"
            => 4,
        // K5 sovereign
        "governance.genesis" | "quorum.change" |
        "rvf.shard.rotate" | "restart.strategy.escalate" |
        "chain.checkpoint" | "chain.fork"
            => 5,
        // Default lifecycle when unclassified
        _ => 2,
    }
}
```

This is the transition bridge. New call sites set `k_level` explicitly;
old chain reads fill it via this table.

### Rollout order

1. Land `k_level` + `severity` + `node_id` + `agent_id` +
   `correlation_id` on `ChainEvent` with `#[serde(default)]`. Inert —
   no call site writes them yet. Run existing tests. Ship.
2. Land `infer_k_level` and wire it into `ChainManager::tail` /
   `tail_from` as an in-memory enrichment on read.
3. Land the `GovernanceGate::observe()` rate-counter path. Still does
   not gate anything; just observes K3.
4. Land the substrate `LogOntologyAdapter` and the
   `/log/<cluster>/<node>/<service>/<kind>` tree.
5. Convert two representative call sites (one K1 heartbeat, one K4
   peer-add) to write K-level explicitly. Verify the infer fallback
   still matches for old events.
6. Mass-convert remaining call sites opportunistically; the infer
   table is the safety net.

No breaking change in steps 1-4; step 5 introduces explicit K in a
targeted way; step 6 is hygiene.

---

## open-questions

1. **Fatal severity.** `LogLevel` today has no `Fatal`. Do we add it, or
   is Error + K≥3 sufficient? I lean toward adding `Fatal` because the
   restart model already distinguishes "should retry" from "catastrophic"
   and a dedicated severity makes that unambiguous.

2. **Who owns the K-level mapping for new `EVENT_KIND_*` constants?**
   Proposal: a clippy-style lint (or compile-time assertion via
   `const_assert!`) that every `EVENT_KIND_*` in `chain.rs` must appear
   in `infer_k_level`. Else new kinds silently fall into the K2
   default.

3. **Gate observation cost.** `GovernanceGate::observe()` is synchronous
   on the hot path. Do we make it async-fire-and-forget, or sync with a
   lock-free counter? I want a lock-free counter (per-service atomic)
   — governance decisions need a stable view of rates, and async
   observation can reorder.

4. **Cluster semantics of `chain_id`.** Today `chain_id: u32` is a
   local chain selector, not a cluster identifier. Do we reuse it as
   cluster, or add a separate `cluster: NodeId`? I lean toward
   separate: clusters are named (node IDs); chains are numbered.

5. **Privacy / PII tagging.** `EffectVector.privacy` already exists. Do
   we let the log fabric route high-privacy events through a
   redaction pipeline before they hit the substrate tree? The
   composer layer (`gui-egui/src/surface_host/compose.rs`) reads from
   the tree directly, and we do not want PII leaking into a rendered
   surface.

6. **Interaction with `witness.bundle`.** The chain already has a
   witness concept at `chain.rs:1779` (`self.append("witness",
   "witness.bundle", …)`). Witnessing is effectively K5 today but not
   marked. Promoting witness events to K5 gives the ontology engine a
   crisp "sovereign-emitted" predicate.

7. **EML-swap interaction.** The `GovernanceScorerModel` at
   `governance.rs:163-178` takes the 5 effect dimensions. Should it
   also take `k_level` as a 6th input feature? Arguments both ways —
   K-level is *derived* from the kind (so information-redundant with
   the context) but *also* a crisp summary feature the model could
   latch onto. Recommend: hold off until the model is trained on
   current features; add K as a feature only if measurable lift.

8. **Sibling-doc coordination:**
   - Doc 01 (schema) must sign off on the `ChainEvent` additions and
     hash-preimage change.
   - Doc 02 (lifecycle) must confirm K2 lifecycle kinds are exhaustive
     in `infer_k_level`.
   - Doc 03 (storage) must confirm the dictionary-coding and TTL
     eviction story at the storage tier.

---

## File-path index (audit citations)

```
crates/clawft-kernel/src/console.rs:58-310          LogLevel, BootEvent, KernelEventLog
crates/clawft-kernel/src/governance.rs:89-397       RuleSeverity, EffectVector, GovernanceEngine
crates/clawft-kernel/src/environment.rs:81-164      GovernanceScope, AuditLevel
crates/clawft-kernel/src/service.rs:76-100          ServiceAuditLevel
crates/clawft-kernel/src/gate.rs:290-459            GovernanceGate::check, extract_effect, chain logging
crates/clawft-kernel/src/gate.rs:540-976            Rule test fixtures (RuleSeverity usage)
crates/clawft-kernel/src/eml_kernel.rs:117-213      RestartStrategyModel
crates/clawft-kernel/src/eml_kernel.rs:226-320      HealthThresholdModel
crates/clawft-kernel/src/eml_persistence.rs:12-70   EML model persistence manifest
crates/clawft-kernel/src/chain.rs:141-163           ChainEvent struct
crates/clawft-kernel/src/chain.rs:224-244           compute_event_hash
crates/clawft-kernel/src/chain.rs:249-700           EVENT_KIND_* constants
crates/clawft-kernel/src/mesh_runtime.rs:282-308    peer.envelope append
crates/clawft-kernel/src/stream_anchor.rs:196-202   stream.window_commit append
crates/clawft-core/src/routing_validation.rs:514-572  check_level_ceiling (routing tier — unrelated)
crates/clawft-treecalc/src/lib.rs:49-95             Form (structural triage — unrelated)
crates/clawft-substrate/src/snapshot.rs:32-260      OntologySnapshot
crates/clawft-substrate/src/lib.rs:5-91             OntologyAdapter contract
crates/clawft-gui-egui/src/live/native_live.rs:66-157  six adapter registrations
crates/clawft-types/src/config/kernel.rs:39-44      heartbeat_interval_secs
```
