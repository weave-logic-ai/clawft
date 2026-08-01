# Upstream MetaHarness score ceilings (ADR-041)

Derived from `node_modules/metaharness/dist/repo-scorecard.js` + `analyze-repo.js`
(inventory HIGH_SIGNAL only).

## Formulas (simplified)

| Dimension | Formula | Practical max for WeftOS |
|-----------|---------|---------------------------|
| harnessFit | `recommendPlan.confidence * 100` | ~80–95 (archetype match) |
| compileConfidence | lang + build + test + CI | **100** (already) |
| taskCoverage | `min(agents+skills+commands,10)*7 + min(tokens,20)*1.5` | **~65** on `mcp-server-harness` (surface=5); **~79** if `rust-crate-harness` wins (surface=7); **100** only if recommended plan surface ≥10 |
| toolSafety | policy default-deny stack | **100** possible |
| memoryUsefulness | `min(fileCount,30)*2 + langs*5 + min(tokens,25)` | **~59–61** — inventory sees ≤~12 “files” (HIGH_SIGNAL + dirs) |

## Implications for “100 across the board”

1. **Impossible** for ADR-041 memoryUsefulness and often taskCoverage without
   changing upstream MetaHarness inventory (deep walk) or forcing a richer
   archetype recommendation.
2. **weftosFoundationScore** is the honest “100” target for this repo’s
   harness investment.
3. Flywheel work that only greases ADR-041 without real assets is rejected
   (Goodhart). Anchors require real tasks/views/patterns.

## Improvement levers that still help ADR-041

| Lever | Helps |
|-------|--------|
| Keep CI + cargo + package.json scripts | compileConfidence |
| Stronger MCP default-deny docs / policy files | toolSafety, harnessFit |
| CONTRIBUTING.md, README token richness | tokens (taskCoverage / memory) |
| Reduce “MCP server” signal if we want rust-crate archetype | may raise taskCoverage surface to 7 |

Prefer raising **weftosFoundationScore** components and real product quality.

## Signaling (maturity, not Goodhart)

High-signal files the ADR-041 inventory *does* read should describe the OS
honestly so ranking is not stuck on “tiny MCP scaffold”:

| File | Intent |
|------|--------|
| `package.json` | `name`, `build`/`test`/`gate` → `scripts/build.sh`, MetaHarness npm scripts |
| `CONTRIBUTING.md` | Build path, harness layout, evaluate→promote, Rust-first genus |
| `README.md` | Agent harness section; primary vs secondary scores; MCP as host surface |

Even with perfect signaling: **taskCoverage ≤ ~79** (archetype surface max 7)
and **memoryUsefulness ≤ ~60** (shallow fileCount). That is upstream formula
design, not missing WeftOS assets.

## The missing probe: `metaharness genome` (rUv ADR-041)

RuvNet source (`metaharness` + timesfm harness provenance) treats **genome** as
the 7-section readiness verdict, separate from the 5-dim scorecard:

| Probe | Command | What it answers |
|-------|---------|-----------------|
| Scorecard | `metaharness score` | Fit of a *recommended scaffold* + shallow inventory |
| **Genome** | `metaharness genome` | Repo type, topology, MCP risk, **test/publish readiness** |
| Candidates | `metaharness score --top N` | Beam of archetypes |
| Mint | `metaharness new …` | Writes `.harness/manifest` + witness (Darwin surface) |

WeftOS often looks “mid” on **score** (MCP archetype, surface 5) while **genome**
says **READY** (rust polyglot, test/publish 100%, low risk). ruflo even keeps a
root `Cargo.toml` workspace so analyzers see Rust without deep walks — still not
enough for scorecard *memory* (fileCount), but genome uses build/test/CI signals.

**Do not ship on scorecard alone.** Always capture genome:

```bash
scripts/metaharness/score.sh   # → score-latest.json + genome-latest.json
```

Optional later: mint a side-car `.harness/` (not overwriting product) if Darwin
evolve is desired; product harness remains `.metaharness/`.
