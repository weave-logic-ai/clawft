# Spawn approval UX — Defer + grant (WEFT-633)

**Status**: Foundation scaffold (GA end-state path wired at engine/gate; panel
resume already exists via WEFT-258/331)  
**Prerequisite**: WEFT-634 action/tool selectors  
**Related**: ADR-094, WEFT-636, `DeferInteractor`, `GateDecision::Defer`

## End-state UX

1. Model calls `agent_spawn`.
2. Gate evaluates with `SPAWN-APPROVAL` rule (selector `tool.agent_spawn` /
   tool `agent_spawn`, `force_on_match`).
3. With `human_approval_required`, engine returns `EscalateToHuman`.
4. `GovernanceGate` maps that to `GateDecision::Defer`.
5. Agent loop suspends (or halts with `FINISH_REASON_DEFERRED` without interactor).
6. Panel / `DeferInteractor` prompts the user.
7. **Allow** → tool proceeds; **Deny/Cancel/Timeout** → spawn cancelled cleanly.
8. Decision + `GatePrincipal` witnessed on the chain.

## What shipped (foundation)

| Piece | Location |
|-------|----------|
| Action/tool selectors + `force_on_match` | `governance.rs` (WEFT-634) |
| `GovernanceRule::spawn_requires_approval()` | `governance.rs` |
| Escalate → Defer mapping | `gate.rs` (pre-existing) |
| Tool identity for selectors from `tool.*` action | `GovernanceGate::check` |
| Principal on chain payload | `gate.rs` + `GovernanceDecisionEvent` |
| Unit tests | selector + spawn_approval + gate Defer |

## Operator enablement

```text
GovernanceGate::new(threshold, human_approval=true)
    .add_rule(GovernanceRule::spawn_requires_approval())
```

Without the rule, spawn still trips magnitude (~0.93) against a global blocking
rule when configured. With the rule, only spawn is force-matched even at low
effect magnitude.

Grant exemption (autonomous spawn) remains:

```text
.exempt_action("tool.agent_spawn")  // witnessed as governance.grant
```

## Gaps (honest)

- Default daemon boot does **not** auto-install `SPAWN-APPROVAL` (opt-in).
- Panel grant → chain "user granted" event is the existing Defer path; no new
  UI chrome was added in this ticket.
- Full pre-granted user capability tokens remain ADR-066 / ADR-094 follow-up.

## Acceptance

- [x] Spawn can Defer via selector rule + human_approval (tests)
- [x] Deny path is existing DeferInteractor Deny (no regression)
- [x] Grant recorded on chain via governance.defer / permit / grant kinds
- [ ] Default production policy installs SPAWN-APPROVAL at genesis (follow-up)
