# Wave 0g — WEFT-330 result

**Ticket:** WEFT-330 — agent-side `SOUL.journal.md` write path during chat turns  
**Branch:** `wave0g/weft-330-soul-journal`  
**Status:** implemented

## Acceptance criteria

| AC | Status |
|----|--------|
| Hook in `loop_core` that can append drift observations to `SOUL.journal.md` | Done — `AgentLoop::maybe_append_soul_journal` after successful tool loop (step 12c) |
| Gated by the existing `soul_journal` substrate write grant (from F1) | Done — `SubstrateSoulJournal` publishes via `publish_gated_with_grants` under `substrate/_derived/soul_journal/<ulid>` |
| Drift detection signal documented (heuristic or model-decided) | Done — module docs + table in `soul_journal.rs` |
| Test: synthetic drift turn produces a journal entry | Done — loop_core + soul_journal unit tests |
| Test: `weaver soul promote` picks up the entry | Done — `promote_picks_up_agent_side_journal_entry` + path fix to mesh-canonical prefix |

## What shipped

### Core (`clawft-core`)

- **New** `crates/clawft-core/src/agent/soul_journal.rs`
  - `SoulJournal` trait, `DriftObservation`, `DriftSignal`
  - `detect_drift_signal` (UserCorrection heuristic + Explicit metadata)
  - `InMemorySoulJournal`, `FileSoulJournal`, `CompositeSoulJournal`
  - Constant `SOUL_JOURNAL_SUBSTRATE_PREFIX` = `substrate/_derived/soul_journal`
- **`AgentLoop`**: optional `soul_journal`, `with_soul_journal`, post-turn hook
- **`build_daemon_agent_loop`**: accepts optional journal writer

### Service agent (`clawft-service-agent`)

- **New** `soul_journal.rs` — `SubstrateSoulJournal` (ULID keys, grant-gated publish)
- Tests: publish under `_derived/`, grant denial, kernel grant round-trip

### Weave

- Daemon attaches composite writer (substrate primary + `.clawft/SOUL.journal.md` file mirror)
- **`weaver soul promote` / `status`**: list prefix fixed from node-scoped  
  `substrate/<node>/derived/soul_journal` → mesh-canonical  
  `substrate/_derived/soul_journal` (matches F1 grant + WEFT-330 writer)

## Drift signal (summary)

| Signal | Fires when |
|--------|------------|
| **UserCorrection** | User message matches a correction/preference cue *and* assistant reply is non-empty |
| **Explicit** | Inbound metadata key `soul_journal_observe` is set (forward-compat for model-decided path) |
| **Synthetic** | Tests only |

Not model-decided today. Explicit metadata is the seam for a future `<!-- soul:observe -->` parser.

## How to test

```bash
cargo test -p clawft-core --features native soul_journal
cargo test -p clawft-service-agent soul_journal
cargo test -p clawft-weave soul
```

## Files touched

- `crates/clawft-core/src/agent/soul_journal.rs` (new)
- `crates/clawft-core/src/agent/mod.rs`
- `crates/clawft-core/src/agent/identity.rs` (docs)
- `crates/clawft-core/src/agent/loop_core.rs`
- `crates/clawft-core/src/bootstrap.rs`
- `crates/clawft-service-agent/src/soul_journal.rs` (new)
- `crates/clawft-service-agent/src/lib.rs`
- `crates/clawft-weave/src/daemon.rs`
- `crates/clawft-weave/src/commands/soul_cmd.rs`
- `docs/plans/wave-0g-WEFT-330-result.md` (this file)
