# Capability crosscut (auto)

Generated: 2026-08-01T01:42:07.768Z

**String:** `SEE → WIRE → BUILD → UPSTREAM`

## Counts

| Mode | N |
|------|---|
| SEE | 16 |
| WIRE | 3 |
| BUILD | 1 |
| UPSTREAM | 1 |

## Full table

| Node | Domain | Mode | Next | Brain score |
|------|--------|------|------|-------------|
| `ruflo` | orchestrator | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 16.763 |
| `agentdb` | memory | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 8.442 |
| `agenticow` | memory | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 6.184 |
| `metaharness-read` | harness | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 16.511 |
| `metaharness-flywheel` | harness | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 15.836 |
| `metaharness-darwin` | harness | **BUILD** | Darwin not enabled — dry-run wrapper then --confirm (S3) | 11.982 |
| `metaharness-hosts` | host | **WIRE** | Grok overlay exists (pathfinder); package host-grok reference for upstream (S1) | 16.279 |
| `metaharness-router` | routing | **UPSTREAM** | Optional consume @metaharness/router + savings (S2) | 15.51 |
| `ruvector` | substrate | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 9.597 |
| `cognitum-gate-tilezero` | governance | **WIRE** | Dep present — CI smoke Permit/Defer/Deny (C3) | 11.47 |
| `cognitum-maas` | cloud | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 18.286 |
| `cognitum-seed` | edge | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 12.873 |
| `worldgraph` | twin | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 15.407 |
| `quic-mesh` | network | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 6.336 |
| `ruvllm-sona` | learning | **WIRE** | Brain found related docs; add explicit package pin or task | 10.199 |
| `weftos-kernel-ecc` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 11.581 |
| `weftos-exochain` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 10.136 |
| `weftos-bvh-spatial` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 14.946 |
| `weftos-lewm` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 12.119 |
| `weftos-voice` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 17.011 |
| `weftos-splat-edge` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 16.48 |

## Suggested Darwin gen-1

```json
{
  "lever": "prefer_intervention",
  "targetMode": "WIRE",
  "focus": "metaharness-hosts",
  "note": "Mutate harness docs/tasks only — one WIRE node per generation"
}
```

Machine JSON: `.metaharness/brain/crosscut-latest.json`
