# Plane Dependency DAG & Wave Plan

> Generated: 2026-07-31 20:36 UTC
> Full inventory: [`plane-board-inventory.md`](./plane-board-inventory.md)
> DAG data: [`plane-dag.json`](./plane-dag.json)
> Skill: `.grok/skills/plane-dag/SKILL.md`
> Helper: `scripts/plane-dag.sh`

## Graph model

```
Node  = Plane work item (WEFT-N)
Edge  = blocked_by (A → B means B waits for A Done)
Wave  = open nodes whose open blockers are empty
Lane  = parallel track inside a wave (A–J)
```

### Inferred edges

| From | To | Reason |
|------|----|--------|

## Wave overview

| Wave | Tickets | High/Urgent | In 0.8.x |
|-----:|--------:|------------:|---------:|
| 0 | 14 | 0 | 1 |
| 1 | 1 | 0 | 0 |

## Critical paths

- **len 2**: WEFT-561 → WEFT-575

## Lanes

| Lane | Focus |
|------|-------|
| A | Release/CI (ws14) |
| B | Kernel/Mesh (ws02, ws13) |
| C | Memory/Vector (ws06, ws12) |
| D | Agent/Hermes (ws11, ws07) |
| E | Voice (ws10) |
| F | UI/Surface (ws08, ws09, ws18) |
| G | WASM/Browser (ws16) |
| H | Channels (ws05) |
| I | Research/LeWM (ws17) |
| J | Tooling/Plane (ws15, tests) |

## Wave 0 — ready now (0.8.x high/medium first)

- **WEFT-657** [LE/low/0.8.x] — ws10: voice — pocket-tts watch: adopt as fast-tier engine when official ONNX/Candle export ships
- **WEFT-301** [LF/medium/1.0.x] — ws09: api-bridge — wire skill install/uninstall to real loader
- **WEFT-304** [LF/medium/1.0.x] — ws09: api — replace mock delegation handlers with FlowDelegator wiring
- **WEFT-305** [LF/medium/1.0.x] — ws09: api — replace mock monitoring handlers with metrics collector
- **WEFT-316** [LF/medium/1.0.x] — ws09: auth — Tailscale provider and per-user session isolation
- **WEFT-574** [LF/medium/1.0.x] — ws09: tauri — desktop shell functional features (tray, hotkey, side-car, Spotlight, notifications, b
- _…plus 8 more ready items (see inventory)_

## Commands

```bash
scripts/plane-dag.sh refresh
scripts/plane-dag.sh ready --cycle 0.8.x --priority high
scripts/plane-dag.sh show WEFT-593
scripts/plane-dag.sh claim WEFT-593
scripts/plane-dag.sh done WEFT-593 --shipped '...' --commits abc123 --tests 'scripts/build.sh test' --build 'scripts/build.sh check'
```

## Lifecycle

| Event | Action |
|-------|--------|
| Claim | `plane-dag.sh claim WEFT-N` |
| Note | `plane-dag.sh note WEFT-N "..."` |
| Done | `plane-dag.sh done WEFT-N --shipped ... --commits ...` then `refresh` |
| Defer | `plane.sh defer <uuid> 0.9.x --reason "..."` |

