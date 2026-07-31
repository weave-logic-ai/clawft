# ECC RVF segment types (WEFT-508)

**Status:** Defined (2026-07-31)  
**Code:** `clawft_kernel::ecc_segment` (feature `ecc`)

## Why

Upstream `rvf-types::SegmentType` covers vector/index/federation codes through `0x36`. ECC structures (`CausalEdge`, impulses, calibration profiles, spectral checkpoints) were only conceptual at the segment layer, so persistence stayed ad-hoc JSON / ExoChain events.

## Domain block

WeftOS allocates **`0x50`–`0x5F`** for ECC payloads (avoids rvf-types and research CTX reservations `0x40`–`0x42`).

| Byte | Variant | Payload |
|------|---------|---------|
| `0x50` | `CausalEdge` | Causal edge record |
| `0x51` | `CausalNode` | Causal node record |
| `0x52` | `Impulse` | Impulse / ephemeral causal event |
| `0x53` | `CalibrationProfile` | Calibration / Democritus profile |
| `0x54` | `CrossRef` | Cross-tree reference |
| `0x55` | `SpectralCheckpoint` | λ₂ / eigenvalue snapshot |
| `0x56` | `CausalGraphSnap` | Whole-graph snapshot envelope |

Wire framing (WeftOS helper, not full RVF header):

```text
[type:u8][codec:u8=0x01 JSON][payload...]
```

Encode/decode: `encode_ecc_segment` / `decode_ecc_segment` / `segment_to_wire` / `segment_from_wire`.

## Migration plan

1. **Current (shipped):** ExoChain / JSON remain authoritative for live kernels.  
2. **Dual-write (next):** Optional writers emit ECC segments alongside chain events.  
3. **Prefer-read:** Loaders accept ECC segments when present; fall back to legacy.  
4. **Deprecate:** After one release of dual-write, new events need not write legacy-only forms.

No on-disk format is deleted in this change — definitions + round-trip tests only.

## Tests

See unit tests in `crates/clawft-kernel/src/ecc_segment.rs`.
