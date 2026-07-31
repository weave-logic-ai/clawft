# Agent identity security

**Related tickets**: WEFT-92 (binding-thread policy), WEFT-91 (hot-reload),
WEFT-342 (deny default), WEFT-329 (notify watcher)  
**Code**: `crates/clawft-core/src/agent/identity.rs`,
`identity_watcher.rs`, governance rule `soul.binding_thread_intact`

## Binding-thread integrity (WEFT-92 / WEFT-342)

Every Concierge turn loads `SOUL.md` + `IDENTITY.md` and checks that
`SOUL.md` still contains the compile-time **binding-thread excerpt**
(`BINDING_THREAD_EXCERPT` in `identity.rs`). This is a tamper / drift
signal: a stripped or replaced soul must not silently continue as the
same agent.

### Policy

| Mode | Config key | Effect |
|------|------------|--------|
| **Deny (default)** | `agents.binding_thread_mode = "deny"` | Turn hard-refuses. Gate action `soul.binding_thread_intact` returns `GateDecision::Deny { reason: "binding-thread mismatch" }`. Maps to `IdentityError::BindingThreadMismatch`. |
| **Warn only (legacy)** | `agents.binding_thread_mode = "warn_only"` | Prompt annotated `binding-thread-status: mismatch`; `warn!` log; agent continues degraded. |

**Decision (WEFT-92)**: config-toggle with **refuse as default**. Operators
who need soft degrade for migration set `warn_only` explicitly.

### Operator config (weave.toml / config JSON)

```toml
[agents]
binding_thread_mode = "deny"   # or "warn_only"
```

### Tests

- `clawft-types`: `binding_thread_mode_defaults_to_deny`
- `clawft-core`: `handle_turn_hard_refuses_on_binding_thread_mismatch_deny_mode`,
  `handle_turn_continues_on_binding_thread_mismatch_warn_only`
- `clawft-kernel`: `binding_thread_rule_denies_mismatch`,
  `binding_thread_rule_permits_ok`

## Identity hot-reload (WEFT-91 / WEFT-329)

`FileIdentityProvider` caches the last successful load.

- **Default / portable**: re-read on every `current()` (small files).
- **Native daemon**: `identity_watcher` (notify crate) watches
  `.clawft/SOUL.md` and `IDENTITY.md`, debounces, calls `invalidate()`,
  next turn re-reads.

**Decision (WEFT-91)**: ship the watcher; do not wait for further profiling.
WASM remains re-read-every-call.

## Related

- Decision batch: `docs/plans/decisions/0.8-decision-batch-open-questions.md`
- Soul journal (WEFT-96 / WEFT-330): pending observations do not replace
  the binding-thread check on the base `SOUL.md`.
