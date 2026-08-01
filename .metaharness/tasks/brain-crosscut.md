# Task: weft-brain-crosscut

**String:** `SEE → WIRE → BUILD → UPSTREAM`  
**Loop:** index → crosscut → darwin plan → measure

```bash
npm run metaharness:loop
# or step-wise:
node scripts/metaharness/weftos-brain.mjs index
node scripts/metaharness/weftos-brain.mjs search "<capability>"
node scripts/metaharness/crosscut.mjs
node scripts/metaharness/darwin-loop.mjs          # dry
node scripts/metaharness/darwin-loop.mjs --confirm  # proposal under .metaharness/variants/
node scripts/metaharness/flywheel-measure.mjs measure
```

Prefer **WIRE** nodes from crosscut over BUILD. Never mutate crates/ or ADR-090 in Darwin gens.
Human research crosscut: `docs/research/crosscut-weftos-ruv-2026-08-01.md`
