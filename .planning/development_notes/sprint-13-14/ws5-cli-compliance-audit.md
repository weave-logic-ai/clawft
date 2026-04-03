# WS5: CLI Kernel Compliance Audit

## Rule (ADR-021)
All CLI commands performing operations MUST route through kernel daemon via RPC.

## Completed
- clawft-rpc crate extracted from clawft-weave
- 32 commands migrated to daemon-first with local fallback
- All commands print deprecation warning when running without daemon

## Command Migration Status

| Category | Commands | Status |
|----------|----------|--------|
| Cron | list, add, remove, enable, disable, run | DONE |
| Assess | run, link, compare | DONE (init exempt: bootstrap) |
| Security | scan | DONE (checks exempt: display) |
| Skills | list, show, install, remove, search, publish, remote-install | DONE (keygen exempt: pure crypto) |
| Tools | list, show, mcp, search, deny, allow | DONE |
| Agents | list, show, use | DONE |
| Workspace | create, list, load, status, delete, config set/get/reset | DONE |

## Remaining (Sprint 15+)
- onboard: bootstrap exception, accepted
- mcp-server: self-hosting service, accepted
- agent/gateway: bootstrap own AppContext, accepted
- ui: should be kernel-managed (deferred)
- voice: should be kernel-managed (deferred)
