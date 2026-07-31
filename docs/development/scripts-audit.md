# Scripts audit — top-level orphans (WEFT-465)

**Status:** closed for the Task #20 set  
**Source audit:** `.planning/reviews/0.7.0-release-gate/14-deployment-release.md`  
**Scope:** `scripts/clawft-wake.service`, `scripts/com.clawft.wake.plist`,
`scripts/build_vp_deck.py`, `scripts/dev_server.py`, `scripts/weave-init.sh`  
**Rule:** record a disposition (wire / move / keep+document / delete);
only delete or move **safe, obvious** dead code.

## Summary table

| Path | Disposition | Rationale |
|------|-------------|-----------|
| `scripts/clawft-wake.service` | **Keep** (live) | Embedded at compile time into `clawft-cli` via `include_str!` and installed by `weft voice install-service` (systemd). |
| `scripts/com.clawft.wake.plist` | **Keep** (live) | Same path for launchd; `include_str!` in `crates/clawft-cli/src/commands/voice.rs`. |
| `scripts/install-clawft-wake-schtasks.ps1` | **Keep** (live companion) | Optional Windows schtasks installer; documented in `docs/guides/voice.md` and printed by the CLI (WEFT-220). |
| `scripts/dev_server.py` | **Keep** (live) | Invoked by `scripts/build.sh serve` for the browser WASM harness CORS proxy. |
| `scripts/weave-init.sh` | **Keep** (distinct tool) | Copies `agents/` → `.claude/agents/` + `.claude/skills/` for Claude Code. **Not** the same as `weaver init`. |
| `scripts/build_vp_deck.py` | **Moved** → `scripts/dev/build_vp_deck.py` | One-shot QSR VP deck generator; no CI/workflow callers. |

Related deferred items **not** closed here (separate Plane rows / tasks):

- `scripts/09-gate.sh` — stale test floor (audit Task #21)
- `scripts/k6-gate.sh` — not on CI path (audit Task #22; likely WEFT-464 family)

## Detail

### Wake unit files — keep; “not in tarball” is intentional

Audit claim: units are missing from `[workspace.metadata.dist] include` /
release tarballs, so “probably dead.”

**Finding:** they are the **source of truth for install-from-tree and for
the binary**, not loose release assets.

- Linux: `install_systemd_service` writes
  `include_str!("../../../../scripts/clawft-wake.service")` into the
  user unit directory, then `systemctl --user enable`.
- macOS: same pattern for `scripts/com.clawft.wake.plist` → LaunchAgents.
- Windows: automated `schtasks` path in the CLI; optional
  `scripts/install-clawft-wake-schtasks.ps1` for checkout-based install.

Because content is baked into `weft` at compile time, **cargo-dist does
not need to ship the unit files as separate artifacts**. Operators who
install the binary get the units via `weft voice install-service`.
Repo checkouts keep the files at `scripts/` so `include_str!` paths stay
stable and docs (`docs/guides/voice.md`,
`docs/development/testing-three-workstreams.md`) remain accurate.

**Do not move** these files without updating the `include_str!` paths
and all docs.

### `dev_server.py` — keep

Audit claim: not part of any workflow.

**Finding:** `scripts/build.sh` `serve` launches:

```bash
python3 "$SCRIPT_DIR/dev_server.py" "$port" "$www_dir"
```

That is the CORS-proxy static server for the `clawft-wasm` www harness.
Leave at `scripts/dev_server.py` next to `build.sh` (path is hard-wired).

### `weave-init.sh` vs `weaver init` — keep both; different jobs

| | `scripts/weave-init.sh` | `weaver init` (Rust) |
|--|-------------------------|----------------------|
| Purpose | Install Claude Code agents/skills from `agents/` into `.claude/` | Bootstrap workspace: `weave.toml`, `.weftos/runtime/`, `.clawft/` identity |
| Audience | Contributors using Claude Code agent teams | Runtime / agent identity bootstrap |
| Documented in | `agents/README.md` | `docs/guides/agents.md`, weave crate |

CHANGELOG 0.6.14’s “weaver init rewritten” does **not** supersede the
shell script. Deleting `weave-init.sh` would break the documented agent
install path without a replacement.

### `build_vp_deck.py` — moved to `scripts/dev/`

One-off generator for `.planning/clients/qsr/qsr-vp-briefing.pptx`.
No CI, no `build.sh` subcommand, no docs outside this audit.

- New path: `scripts/dev/build_vp_deck.py`
- `ROOT` adjusted to `parents[2]` after the move
- Run: `python3 scripts/dev/build_vp_deck.py` (needs `python-pptx`)

Not deleted: still useful if the QSR deck is regenerated; not product
runtime.

## Packaging notes (kept scripts)

| Script | Release artifact | Docs |
|--------|------------------|------|
| Wake `.service` / `.plist` | Content embedded in `weft` binary; install via CLI | `docs/guides/voice.md` |
| `install-clawft-wake-schtasks.ps1` | Checkout / source tree only | voice guide + CLI help |
| `dev_server.py` | Dev only (`build.sh serve`) | browser / build help |
| `weave-init.sh` | Contributor tooling | `agents/README.md` |
| `scripts/dev/build_vp_deck.py` | None (client one-shot) | this file |

## Acceptance (WEFT-465)

- [x] Disposition recorded for each named script
- [x] Kept scripts documented (and wake units explained vs tarball)
- [x] Dead one-shot moved under `scripts/dev/` (no unsafe deletes)
- [x] Audit Task #20 rows marked closed with WEFT-465
- [x] CHANGELOG notes the move under Unreleased
