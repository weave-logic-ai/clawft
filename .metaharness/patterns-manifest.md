# AgentDB pattern keys (MetaHarness foundation)

**Plane:** WEFT-726  
**Namespace:** `patterns`  
**Seed:** `scripts/metaharness/seed-patterns.sh`

| Key | Purpose |
|-----|---------|
| `pattern-metaharness-foundation` | ADR-096/097 doctrine, Grok rules, scripts |
| `pattern-graph-views-fusion` | Graph Views = sensor fusion F1–F10 |
| `pattern-release-gate` | `scripts/build.sh gate` / weft-gate task |
| `pattern-plane-dag` | Plane DAG claim/close discipline |
| `pattern-viewspec-flywheel` | ViewSpec fixtures + evaluate-only promote |
| `pattern-data-surface-governance` | ADR-097 universal surfaces |
| `pattern-ruv-worldgraph` | WorldGraph / OccWorld crosswalk |

## Rules

- No secrets, API keys, or private key material in values.
- Overwrite in place when doctrine updates; keep keys stable for search.
- After seed: `npx ruflo memory search --query "fusion ViewSpec" --namespace patterns`

## Hosts

Grok and Claude both recall via ruflo/claude-flow `memory_search` on namespace `patterns`.
