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
