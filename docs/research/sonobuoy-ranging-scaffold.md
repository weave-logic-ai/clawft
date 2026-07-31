# clawft-sonobuoy-ranging scaffold (WEFT-535)

**Status**: Scaffold landed  
**Cycle**: 0.8.x  
**Related**: ADR-078, `.planning/sonobuoy/RANGING.md` (G1 closed)

## What shipped

Crate `crates/clawft-sonobuoy-ranging/`:

| Item | Notes |
|------|--------|
| `BuoyId` | Opaque field member id |
| `RangeObservation` | OWTT travel-time + sound-speed → range |
| `owtt_range_m` | Pure `r = c · τ` helper |
| `DistanceMatrix` | Dense `D(t) ∈ R^{N×N}` with fill ratio |
| `synthetic_ring_field` | Unit-test fixture (equal spacing on a circle) |

No kernel / mesh / sensor I/O deps. Pure geometry for v2 consumers
(Tzirakis dynamic-adjacency GCN, Grinstein relation-net, Grassmann DoA).

## Acceptance (WEFT-535)

- [x] Crate scaffolded under `crates/clawft-sonobuoy-ranging/`
- [x] Initial ranging primitives per ADR-078 / RANGING.md (OWTT, D matrix)
- [x] Tests on synthetic data
- [x] Doc page entry (this file)

## Deferred

- Doppler radial velocity matrix `V(t)`
- Per-pair covariance `Σ(t)` fusion with GPS
- TSHL / D-Sync clock discipline
- Multipath SSP inversion (EOF basis)
- Mesh wire framing for ranging slots
- JANUS waveform generation

See RANGING.md §0 executive summary for the full v2–v4 roadmap.
