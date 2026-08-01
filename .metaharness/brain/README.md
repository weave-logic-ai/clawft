# WeftOS brain

Local, removable corpus for **SEE → WIRE → BUILD → UPSTREAM** and Darwin
**traverse → compare**.

Mirrors the *role* of ruvbrain (`search_ruvnet`): agents can crawl **our** graph
(ADRs, crates, research, MetaHarness, Grok rules, ViewSpecs) without grepping
the whole monorepo.

## Commands

```bash
# Rebuild index
node scripts/metaharness/weftos-brain.mjs index

# Search
node scripts/metaharness/weftos-brain.mjs search "tilezero permit"
node scripts/metaharness/weftos-brain.mjs search "graph views bvh"

# Stats
node scripts/metaharness/weftos-brain.mjs stats

# Crosscut (classify catalog nodes)
node scripts/metaharness/crosscut.mjs

# Darwin loop dry plan (one WIRE node)
node scripts/metaharness/darwin-loop.mjs
node scripts/metaharness/darwin-loop.mjs --confirm   # write proposal under variants/
node scripts/metaharness/darwin-loop.mjs --measure   # plan + flywheel measure
```

npm: `metaharness:brain:*`, `metaharness:crosscut`, `metaharness:darwin`

## Layout

| Path | Role |
|------|------|
| `index.jsonl` | documents |
| `df.json` | document frequency |
| `meta.json` | counts |
| `crosscut-latest.json` | last classify run |
| `docs/research/crosscut-latest.md` | human table |

Index artifacts are gitignored; scripts and this README are tracked.

## Doctrine

`.metaharness/flywheel/STRING.md` · not a `weft` runtime dependency.
