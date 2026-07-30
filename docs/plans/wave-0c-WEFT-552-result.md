# WEFT-552 result — rustls-webpki via rustls/reqwest/quinn alignment

**Status:** done (acceptance criteria met for the rustls-webpki cluster)  
**Branch:** `wave0c/weft-552-rustls-webpki`  
**Base:** `release/0.8-staging`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb446-80b9-7ea1-b9bd-350548ef3940`  
**Date:** 2026-07-30

## Problem

`Cargo.lock` shipped two `rustls-webpki` versions (`0.101.7` + `0.103.10`), both
hit by RUSTSEC-2026-0098 / 0099 / 0104. The old line came from
`ruvector-core 2.1.0 → reqwest 0.11 → rustls 0.21 → rustls-webpki 0.101.7`.

## What changed

| Crate | Before | After |
|-------|--------|-------|
| `ruvector-core` | 2.1.0 | **2.3.0** (reqwest 0.12 + rustls-tls) |
| `rustls-webpki` | 0.101.7 + 0.103.10 | **0.103.13 only** |
| `rustls` | 0.21.12 + 0.23.36 | **0.23.42 only** |
| `reqwest` | 0.11.27 + 0.12.28 | **0.12.28 only** |
| `quinn` | 0.11.9 (already rustls 0.23) | 0.11.9 unchanged |
| `tokio-rustls` | 0.24.1 + 0.26.4 | **0.26.4 only** |
| `hyper-rustls` | 0.24.2 + 0.27.7 | **0.27.7 only** |
| `crossbeam-epoch` | 0.9.18 | **0.9.20** (free fix on same path; RUSTSEC-2026-0204) |

Gate ignore-list: removed RUSTSEC-2026-0098, 0099, 0104 from
`scripts/build.sh` `CARGO_AUDIT_IGNORES` and `.github/workflows/pr-gates.yml`.

## Files

- `Cargo.lock` — resolution
- `Cargo.toml` — comment: do not re-lock `ruvector-core` to 2.1.x
- `scripts/build.sh` — drop 3 ignores; mark WEFT-552 cleared
- `.github/workflows/pr-gates.yml` — same
- `docs/plans/wave-0c-WEFT-552-result.md` — this file

## Verification

```text
scripts/build.sh check   # pass (workspace)
cargo tree -i rustls-webpki --workspace
  → rustls-webpki v0.103.13 only (via rustls 0.23.42)
cargo audit  # no hits for RUSTSEC-2026-0098 / 0099 / 0104
```

## Acceptance criteria

| Criterion | Result |
|-----------|--------|
| rustls-webpki ≥ 0.103.13 everywhere; no 0.101.x | **yes** |
| rustls 0.23+ consistency (reqwest, hyper-rustls, quinn, tokio-rustls) | **yes** (tonic not in tree) |
| Drop 3 RUSTSEC IDs from gate ignore list | **yes** |
| No regressions in TLS-using crates | **check pass** |
| `scripts/build.sh gate` fully green | **partial** — webpki cluster clear; remaining audit noise is pre-existing / out of scope: quick-xml 0.39.2 (wayland-scanner pin), failure, anyhow, spin (already on staging lock; not introduced by this change) |

## Commands used

```bash
cargo update -p ruvector-core --precise 2.3.0
cargo update -p rustls-webpki --precise 0.103.13
cargo update -p rustls --precise 0.23.42
cargo update -p crossbeam-epoch --precise 0.9.20
scripts/build.sh check
```

## Notes for lead

- Do **not** re-lock `ruvector-core` to 2.1.x — reintroduces webpki 0.101.
- Side effect: `rustls-pemfile` 1.x (WEFT-553 / RUSTSEC-2025-0134) left the
  lock with the reqwest 0.11 stack; ignore can stay until WEFT-553 closes.
- Remaining cargo-audit red items need separate tickets (quick-xml / GUI
  wayland path; failure/spin/anyhow warnings).
