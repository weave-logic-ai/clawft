# HNSW × EML opportunities tracking (WEFT-58)

**Date**: 2026-07-31  
**Ticket**: WEFT-58  
**Status**: Decision recorded  
**Sources**:
- [`.planning/development_notes/hnsw-eml-deep-analysis.md`](../../.planning/development_notes/hnsw-eml-deep-analysis.md) (2026-04-14)
- [`.planning/development_notes/hnsw-eml-analysis.md`](../../.planning/development_notes/hnsw-eml-analysis.md)
- Audit: `.planning/reviews/0.7.0-release-gate/03-pipeline-routing.md` (Deferred / From hnsw-eml-deep-analysis.md)
- Code: `crates/clawft-kernel/src/hnsw_eml.rs`, `crates/clawft-core/src/embeddings/path_predict.rs`

## Decision (acceptance criteria)

### Items 1–2 vs 0.8 / 0.9 ship

| # | Opportunity | Enter 0.9.x *product* scope? | Rationale |
|---|-------------|------------------------------|-----------|
| **1** | Adaptive ef (beam width) | **No — remain research** | Scaffolding exists (`HnswEmlManager::predict_ef`, wired in `HnswService::search` when trained). Not a 0.8 ship feature: untrained models fall back to default ef=100; no production training corpus, recall SLA, or release-note commitment. Out of 0.8 publish gate. Revisit for **1.0.x research** or a dedicated spike only after offline recall benches prove ≥1.5× speedup at ≤0.5% recall loss. |
| **2** | Learned distance | **No — remain research** | `distance_model` exists on `HnswEmlManager` for approx / dim-selection style training points. Domain-specific distance is higher risk (silent recall regressions). **Out of 0.8 ship.** Do not promote without differential recall tests against exact cosine on held-out corpora. |

**Separate Plane items for #1–2**: already filed historically as **WEFT-384** (adaptive ef) and graphify/ws12 performance items. This ticket does **not** re-file or re-open them. Status of those closes should be treated as *implementation scaffolding / triage*, not as “0.8 product done” unless their close comments and benches say otherwise. Learned-distance remains covered by this tracking page + the manager’s distance path rather than a new 0.8 ticket.

### Items 3–10

Remain **research** unless a module ticket with benches exists.

| # | Opportunity | Tracking / code | Disposition |
|---|-------------|-----------------|-------------|
| 3 | Cosine similarity decomposition | WEFT-386 (Done on board); deep-analysis #3 | Research / quality-vs-speed; not 0.8 gate |
| 4 | Search-path prediction (region → entry) | **WEFT-385** + commit `815146d0` (`path_predict`, `query_guided`) | **Partial ship**: region table + guided search path landed; still optional / learn-when-ready, not a hard 0.8 marketing claim |
| 5 | Neighbor quality prediction | deep-analysis only | Research; needs custom neighbor heuristic hooks |
| 6 | Rebuild cost prediction | WEFT-380; `predict_rebuild` scaffolding | Research; static threshold remains safe default |
| 7 | Layer probability optimization | deep-analysis | Research; needs pluggable layer assign |
| 8 | Progressive dimensionality | deep-analysis (highest impact claim) | Research; multi-resolution distance wrappers |
| 9 | Cache-aware traversal | deep-analysis | Research |
| 10 | Quantization-aware distance | deep-analysis | Research; DiskANN/PQ path only if that stack lands |

## Honest state of the code (2026-07-31)

`HnswEmlManager` holds four `EmlModel`s (distance, ef, path, rebuild) plus an optional `RegionEntryTable` for #4. Predictions are gated on `config.enabled` and `model.is_trained()`; defaults apply when untrained. That is **infrastructure**, not a finished research result.

What this ticket closes:

- [x] Decide whether items 1–2 enter 0.9.x product scope → **No; research only; out of 0.8 ship**
- [x] If yes: file separate items → N/A (historical WEFT-384 etc. already exist; no new 0.8 filings)
- [x] If no: tracking comment + this page with rationale

What this ticket does **not** claim:

- Offline MSE / recall experiments for adaptive ef or learned distance
- That WEFT-384 / 380 / 386 closes equal production validation
- That HNSW-EML is a 0.8 release feature

## When to promote #1 or #2

Minimum bar before filing a *ship* ticket:

1. Offline bench harness with fixed corpus + exact-NN oracle (see casestudy `ef_search` sweep patterns).
2. Report: speedup vs default, recall@k delta, failure cases (uniform high-d data for #2).
3. Feature flag default **off**; metrics for fallback rate.
4. Docs in `docs/` + CHANGELOG only after green benches.

## Related

- [eml-heuristics-tracking.md](./eml-heuristics-tracking.md) (WEFT-57)
- WEFT-385 search-path prediction implementation
- `docs/plans/eml-attention-iter3-gate.md` (WEFT-41) — separate EML research track
