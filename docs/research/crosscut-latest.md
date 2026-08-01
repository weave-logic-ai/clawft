# Capability crosscut (auto)

Generated: 2026-08-01T01:51:05.133Z

**String:** `SEE → WIRE → BUILD → UPSTREAM`

## Counts

| Mode | N |
|------|---|
| SEE | 19 |
| WIRE | 0 |
| BUILD | 0 |
| UPSTREAM | 2 |

## Full table

| Node | Domain | Mode | Next | Brain score |
|------|--------|------|------|-------------|
| `ruflo` | orchestrator | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 17.595 |
| `agentdb` | memory | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 8.226 |
| `agenticow` | memory | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 6.155 |
| `metaharness-read` | harness | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 15.291 |
| `metaharness-flywheel` | harness | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 15.457 |
| `metaharness-darwin` | harness | **SEE** | Dry loop present (darwin-loop.mjs); full @metaharness/darwin evolve is optional S3 with --confirm | 11.66 |
| `metaharness-hosts` | host | **SEE** | Grok host reference present — agents can SEE pathfinder; UPSTREAM host-grok package still open (S1 publish) | 16.305 |
| `metaharness-router` | routing | **UPSTREAM** | Optional consume @metaharness/router + savings (S2) | 15.422 |
| `ruvector` | substrate | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 9.616 |
| `cognitum-gate-tilezero` | governance | **SEE** | Dep + agent task present — C3 full cargo CI smoke still optional maturity | 20.624 |
| `cognitum-maas` | cloud | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 18.147 |
| `cognitum-seed` | edge | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 12.762 |
| `worldgraph` | twin | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 15.315 |
| `quic-mesh` | network | **SEE** | Present on both sides; ensure dual-host/MCP path uses it | 6.383 |
| `ruvllm-sona` | learning | **UPSTREAM** | Optional micro-loop (ADR-234); not required for WeftOS product — document only (no BUILD until product need) | 24.304 |
| `weftos-kernel-ecc` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 11.751 |
| `weftos-exochain` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 10.206 |
| `weftos-bvh-spatial` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 15.045 |
| `weftos-lewm` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 12.102 |
| `weftos-voice` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 17.099 |
| `weftos-splat-edge` | weftos | **SEE** | Index/doctor already ok; keep brain index fresh so agents find it | 16.513 |

## Suggested Darwin gen-1

```json
{
  "lever": "prefer_intervention",
  "targetMode": "WIRE",
  "focus": "agentdb",
  "note": "Mutate harness docs/tasks only — one WIRE node per generation"
}
```

Machine JSON: `.metaharness/brain/crosscut-latest.json`
