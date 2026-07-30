# Wave 0b integration summary — 2026-07-30

Base: **`release/0.8-staging`**. Nine worktree agents (`general-purpose`; `ruflo-coder` was unavailable until `.grok/agents` restored). All merged cleanly.

## Results

| WEFT | Outcome | Key SHA | Summary |
|------|---------|---------|---------|
| **594** | Shipped | `c25663a1` | Self-contained multi-arch Docker CI (native amd64+arm64, no QEMU); OrbStack/Apple container docs |
| **429** | Shipped | `144516dd` | ADR-012 `CapturePrivacyGate` on `Substrate::subscribe_adapter` |
| **661** | Shipped | `b4471f06` | Hybrid hot/cold merge via **RRF (k=60)** |
| **651** | Shipped | `ea6ff4aa` | Identical tool-failure breaker **N=3** + schema-echo |
| **672** | Shipped | `df62d89b` | `clawft_llm::hermes` always available (browser/WASM) |
| **597** | Shipped | `b613d02a` | `ChainEventLayer` → ChainManager for 12 ExoChain kinds |
| **681** | Shipped (doc) | `18c7fc64` | wasmtime advisories risk-accepted; vehicle **WEFT-551** (need ≥46) |
| **645** | Shipped | `b669b603` | Hermetic RPC tests via `connect_path` + tempdir |
| **667** | Shipped | `92c6048c` | `~` pin esp-hal / esp-radio on edge-pad + lgfx-bus |

## Wave 0a (closed on Plane)

593, 596, 660, 605, 671, 595, 663, 684 → Done with SHAs.

## Suggested Wave 0c candidates

- **WEFT-551** wasmtime 33→46+ (from 681)
- **WEFT-668** set-wise esp version bump (after 667)
- **WEFT-430** affordance ∩ permit (after 429/596)
- Medium 0.8 backlog from `scripts/plane-dag.sh ready --cycle 0.8.x`
