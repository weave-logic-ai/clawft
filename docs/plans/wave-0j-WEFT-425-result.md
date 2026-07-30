# WEFT-425 result — parse `[compositions.*]` and expand in composer

**Ticket:** WEFT-425  
**Branch:** `wave0j/weft-425-compositions`  
**Base:** `release/0.8-staging`  
**Commit:** branch tip `wave0j/weft-425-compositions`  
**Date:** 2026-07-30  
**Agent:** coder-425 (wave-0j)

## Problem

User-defined compositions (`[compositions.*]`) were not parsed (M1.5
scope reduction documented in `clawft-surface` lib.rs). Reusable surface
fragments could not be authored in TOML; authors had to inline every
`ui://stack` / layout attrs by hand.

Source: `.planning/reviews/0.7.0-release-gate/13-app-substrate-surface.md`
(task 16); ADR-016 §7.

## What shipped

Load-time expansion of document-level composition macros into canon
IRIs, per ADR-016 §7 (“Expansion happens at load time… The wire never
sees a `Card`”).

| Piece | Detail |
|-------|--------|
| `CompositionDef` | New IR type: `name`, `expands_to: IdentityIri`, default `attrs` |
| `SurfaceTree.compositions` | Retains document defs after expansion (introspection / tests) |
| TOML `[compositions.Name]` | Requires `expands_to` (canon `ui://…`); optional `attrs` |
| Node `type = "Name"` | Resolves via composition table when not a canon IRI |
| Attr merge | Composition defaults first; instance attrs win |
| Form sugar | If merged attrs contain `submit_verb`, last child wraps in `ui://pressable` with that verb; attr consumed |
| Shadow guard | Composition name colliding with short name or full `ui://…` fails load |
| Target guard | `expands_to` must be a known `IdentityIri` |

Composer runtime in `clawft-gui-egui::surface_host` is unchanged: it
already only understands canon IRIs. Expansion is complete before the
tree is handed to `compose()`.

### Example

```toml
[compositions.Card]
expands_to = "ui://stack"
attrs      = { axis = "vertical", padding = 12, frame = "rounded" }

[compositions.Form]
expands_to = "ui://stack"
attrs      = { axis = "vertical", gap = 8 }

[[surfaces.root.children]]
type  = "Card"
id    = "/root/card"
attrs = { padding = 8 }   # overrides composition default

[[surfaces.root.children]]
type  = "Form"
id    = "/root/form"
attrs = { submit_verb = "rpc.form.submit" }
# children… last child becomes pressable(submit)
```

## Files

| Path | Change |
|------|--------|
| `crates/clawft-surface/src/tree.rs` | `CompositionDef`; `SurfaceTree.compositions` |
| `crates/clawft-surface/src/parse/toml.rs` | Parse + expand; unit tests |
| `crates/clawft-surface/src/parse/mod.rs` | Re-export `parse_compositions` |
| `crates/clawft-surface/src/parse/expr.rs` | Doc: compositions not expr-language |
| `crates/clawft-surface/src/lib.rs` | Docs; re-export `CompositionDef` |
| `crates/clawft-surface/fixtures/multi-composition.toml` | Multi-comp fixture |
| `crates/clawft-surface/tests/roundtrip.rs` | Integration round-trip |
| `docs/plans/wave-0j-WEFT-425-result.md` | This result |

## Acceptance

| Criterion | Status |
|-----------|--------|
| TOML parser accepts `[compositions.*]` blocks | **Done** |
| Composer expands referenced compositions during render | **Done** — expand at load; composer receives canon-only tree |
| Round-trip test covers a multi-composition surface | **Done** — unit + `tests/roundtrip.rs` + fixture |

## Tests

```bash
scripts/build.sh check
cargo test -p clawft-surface
cargo test -p clawft-gui-egui --test compose_extra_iris --test surface_headless_render
```

- **check:** pass  
- **clawft-surface:** 32 lib + 5 eval + 5 roundtrip + 2 builder + 1 doctest — all pass  
  - new: `parses_and_expands_multi_composition_surface`, shadow/unknown/bad-target rejects, `multi_composition_surface_expands`  
- **gui-egui surface suite:** pass (no API break)

## Out of scope / follow-ups

- Rust builder helpers for compositions (TOML path is AC focus).
- Nested composition templates (children inside the def body) — ADR
  only specifies `expands_to` + attrs + instance children passthrough.
- Moving composer from `gui-egui` into `clawft-surface` remains WEFT-427.
