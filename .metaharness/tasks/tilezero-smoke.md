# Task: tilezero-smoke (WIRE Cognitum gate)

**Mode:** WIRE  
**rUv node:** cognitum-gate-tilezero  

Prove agents and CI can *see* the TileZero path (Permit/Defer/Deny + receipts),
already depended via workspace `cognitum-gate-tilezero` + feature `tilezero`.

```bash
# Feature flag docs
rg -n "tilezero|cognitum-gate" docs/weftos/FEATURE_GATES.md Cargo.toml

# Brain
node scripts/metaharness/weftos-brain.mjs search "tilezero permit defer"

# Prefer: cargo test -p clawft-kernel --features tilezero … (when wiring tests)
# Until C3 full CI: this task documents the surface for agents.
```

Do **not** weaken gate phases. Receipts are cousins of MH flywheel promote.
