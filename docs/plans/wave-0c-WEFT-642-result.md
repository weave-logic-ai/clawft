# WEFT-642 result — ECC brain HNSW chain_seq joinability

**Branch:** `wave0c/weft-642-chain-seq`  
**Date:** 2026-07-30  
**Status:** Shipped (code + democritus unit tests green)  
**Worktree:** this agent worktree (base `release/0.8-staging`)

## Ticket

ECC brain HNSW cannot join back to the atom spine (`chain_seq` hardcoded `0`
in democritus).

Found by the panopticon study
(`.planning/research/panopticon-primary-index.md` §1 row 8):
`democritus.rs` inserted brain HNSW vectors keyed by causal-node id with
`chain_seq` hardcoded `0` on HNSW metadata, causal edges, and cross-refs —
the projection looked populated/searchable but silently could not
reverse-resolve to the witness spine.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Thread real `chain_seq` (or uid) into democritus insert | **Done** |
| Joinability assertion (tests) | **Done** |
| Surgical fix without full AtomRegistry (WEFT-641) | **Done** — democritus-only |

## What shipped

### `crates/clawft-kernel/src/democritus.rs`

1. **`resolve_chain_seq(impulse)`** — reads `payload.chain_seq` as set by
   `session_tier::index_turn` / talk-mode emitters after ExoChain append.
   Returns `0` only when the emitter never supplied a sequence (legacy /
   synthetic impulses). Does **not** invent a fake sequence.

2. **`node_metadata_with_chain_seq`** — ensures causal-node metadata
   always surfaces top-level `chain_seq` when known (object payloads get
   the key asserted; non-object payloads are wrapped).

3. **`update()` threads `chain_seq` into all three projections:**
   - **HNSW insert metadata:** `chain_seq` + `causal_node_id` (key remains
     causal node id so neighbor→`link` parsing still works).
   - **Causal edge `link(..., chain_seq)`** — no more hardcoded `0`.
   - **CrossRef `chain_seq`** — same value.
   - UniversalNodeId material prefers `chain_seq` over bare HLC when
     present (aligns with session_forest turn UIDs).

HNSW **label** is intentionally still the causal node id (neighbor re-link
depends on parsing it as `NodeId`). The join key lives in **metadata**,
matching how SessionView carries `chain_seq` alongside its HNSW store.

### Tests (26/26 democritus pass)

| Test | Asserts |
|------|---------|
| `resolve_chain_seq_reads_payload` | payload present / missing / non-object |
| `chain_seq_joinable_on_hnsw_node_and_crossref` | causal node meta + HNSW meta + CrossRef all equal emitted seq |
| `chain_seq_joinable_on_causal_edges` | neighbor edges inherit impulse `chain_seq` |

## Out of scope (intentionally)

- Full `AtomRegistry` / `atom.locate` / ADR-069 audit (WEFT-641) — this
  fix makes democritus joinable so audit can pass once that lands.
- Minting a new ExoChain sequence inside democritus when payload omits
  `chain_seq` (would require wiring `ChainManager` into the loop; emitters
  that care already supply it).
- Changing substrate_sink's separate `chain_seq=0` path (not democritus).

## Verification

```bash
scripts/build.sh check
cargo nextest run -p clawft-kernel democritus
# → 26 passed (incl. 3 new WEFT-642 tests)
```

## Follow-ups

- WEFT-641 `AtomRegistry::audit()` should regression-guard
  `chain_seq == 0` rows on brain HNSW permanently.
- Optional: wire optional `ChainManager` into `DemocritusLoop` to mint a
  real sequence for impulses that lack payload.chain_seq.
