# WEFT-553 result — cargo-audit unmaintained crates + unsound rand

**Ticket:** WEFT-553  
**Branch:** `wave0i/weft-553-audit-deps`  
**Date:** 2026-07-30  
**Disposition:** **Partial** — 4 of 6 advisory IDs cleared from the gate ignore-list; 2 residual (upstream-blocked)

## Summary

Minimal, safe dependency upgrades to clear as many WEFT-553 cargo-audit
warnings as possible without breaking the workspace:

| Advisory | Crate | Action | Status |
|----------|-------|--------|--------|
| RUSTSEC-2017-0008 | `serial` 0.4.0 | `portable-pty` 0.8 → **0.9** (uses `serial2`) | **Cleared** |
| RUSTSEC-2024-0384 | `instant` 0.1.13 | `notify` 7 → **8.2** (drops `instant`) | **Cleared** |
| RUSTSEC-2025-0134 | `rustls-pemfile` 1.x | Already gone after WEFT-552 TLS alignment | **Cleared** |
| RUSTSEC-2026-0097 | `rand` 0.8.5 / 0.9.2 | Force **0.8.7** / **0.9.5** (patched lines) | **Cleared** |
| RUSTSEC-2025-0141 | `bincode` 1.3.3 + 2.0.1 | Transitive via `ruvector-*` / `hnsw_rs` | **Residual** |
| RUSTSEC-2024-0436 | `paste` 1.0.15 | Transitive via `tokenizers` / `egui_dock` | **Residual** |

Also applied (hygiene, same lockfile pass):

- `macro_rules_attribute` 0.2.2 → **0.2.3** (uses maintained `pastey` instead of `paste` on that edge only).
- `crossbeam-epoch` 0.9.18 → **0.9.20** (clears RUSTSEC-2026-0204; not part of WEFT-553).

## Why residual?

### bincode (RUSTSEC-2025-0141)

Informational **unmaintained** on package name `bincode` with **no patched versions**
(including 3.x). Pulled exclusively by third-party crates we do not own:

- `hnsw_rs` 0.3.4 → bincode 1.3
- `ruvector-core` / `cluster` / `raft` / `replication` / `diskann` → bincode 2.x

Clearing requires upstream migration to `postcard` / `rkyv` / `wincode` (or
dropping those crates). Not a drop-in workspace bump.

### paste (RUSTSEC-2024-0436)

Unmaintained; successor is `pastey`. Still required by:

- `tokenizers` 0.21.4 (and still on latest 0.23.x)
- `egui_dock` 0.19 / 0.20

`macro_rules_attribute` 0.2.3 already moved to `pastey`. A crates-io → crates-io
`[patch]` of `paste` → `pastey` fails version-req matching (`pastey` is 0.2.x;
dependents ask for `paste ^1`). Vendoring a rename would still leave package
name `paste` in the lockfile, so cargo-audit would keep flagging it.

## Gate ignore-list change

**Removed** (cleared):

- `RUSTSEC-2017-0008`, `RUSTSEC-2024-0384`, `RUSTSEC-2025-0134`, `RUSTSEC-2026-0097`

**Kept** (residual WEFT-553):

- `RUSTSEC-2024-0436` (paste)
- `RUSTSEC-2025-0141` (bincode)

Synced in:

- `scripts/build.sh` → `CARGO_AUDIT_IGNORES`
- `.github/workflows/pr-gates.yml` → `cargo-audit` job

## Files touched

| File | Change |
|------|--------|
| `Cargo.toml` | `rand = "0.8.7"`; `notify = "8"` (+ comments) |
| `Cargo.lock` | rand 0.8.7/0.9.5; notify 8.2; portable-pty 0.9; drop instant/serial; … |
| `crates/clawft-service-terminal/Cargo.toml` | `portable-pty` 0.8 → 0.9 |
| `scripts/build.sh` | ignore-list + help text |
| `.github/workflows/pr-gates.yml` | ignore-list sync |
| `docs/deployment/release.md` | WEFT-553 row → PARTIAL |
| `docs/plans/wave-0i-WEFT-553-result.md` | this result |

No application source changes — public APIs used by our call sites for
`notify` and `portable-pty` remained compatible.

## Verification

```text
scripts/build.sh check                         # pass
cargo check -p clawft-service-terminal         # pass (portable-pty 0.9)
cargo check -p clawft-core -p clawft-services  # pass (notify 8)
cargo audit | grep RUSTSEC-2017-0008           # no match
cargo audit | grep RUSTSEC-2024-0384           # no match
cargo audit | grep RUSTSEC-2025-0134           # no match
cargo audit | grep RUSTSEC-2026-0097           # no match
# residual WEFT-553 only:
cargo audit | grep -E 'RUSTSEC-2024-0436|RUSTSEC-2025-0141'  # present
```

## Out-of-scope residual advisories (not WEFT-553)

Present on cold `cargo audit` after this change; **not** added to the ignore-list
by this ticket (document only):

| ID | Crate | Note |
|----|-------|------|
| RUSTSEC-2026-0194 / 0195 | `quick-xml` 0.39.x | Needs ≥0.41; pinned by `wayland-scanner` ^0.39 via egui/winit |
| RUSTSEC-2020-0036 / 2019-0036 | `failure` 0.1.8 | Via `webrtc-audio-processing-sys` (voice) |
| RUSTSEC-2023-0086 | `lexical-core` 0.7.6 | Via `nom` 5.x |
| (yanked) | `spin` 0.9.8 | Via `lol_alloc` (wasm) |

Follow-up Plane items recommended if the audit gate should go fully green with
`--deny warnings` and no ignores.

## Acceptance criteria

| AC | Status |
|----|--------|
| bincode migrated / both copies removed | **Not met** — residual; upstream ruvector/hnsw_rs |
| instant → web-time / successor path | **Met** — removed via notify 8 |
| paste replaced | **Partial** — one edge → pastey; tokenizers/egui_dock remain |
| rustls-pemfile 1 dropped | **Met** (already absent; ignore removed) |
| serial replaced/removed | **Met** — portable-pty 0.9 / serial2 |
| rand past unsound cutoff | **Met** — 0.8.7 + 0.9.5 |
| All 6 RUSTSEC IDs removed from ignore list | **Partial** — 4 removed, 2 residual |
| scripts/build.sh check | **Met** |

## Commit

Branch: `wave0i/weft-553-audit-deps`  
Tip SHA: see `git rev-parse HEAD` on the branch after commit.
