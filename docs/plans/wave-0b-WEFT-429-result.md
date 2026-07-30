# WEFT-429 result — ADR-012 governance::Gate through Substrate::subscribe_adapter

**Ticket:** WEFT-429  
**Branch:** `wave0b/weft-429-governance-gate`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-429 (wave-0b)

## Problem

`NoopGate` / `StrictGate` were launch-only placeholders. `OntologyAdapter::permissions()`
was advisory, `Substrate::subscribe_adapter` never consulted governance, and
`AdapterError::PermissionDenied` was never emitted. Capture topics (ADR-012)
could open without a `CapabilityGrant`.

WEFT-596 path ACLs cover **read** egress for remote callers; this ticket wires
**open-time** governance on the ontology subscribe path (orthogonal).

## What wired

### `clawft-app` — real ADR-012 gate

| Item | Detail |
|------|--------|
| `Gate::authorize_adapter_open` | New method: `(adapter_id, topic, required, granted) → AdapterOpenResult` |
| `AdapterOpenResult` | `Granted` / `Denied { missing, reason }` |
| `CapturePrivacyGate` | ADR-012 capture-privacy implementation |
| `StrictGate` | Type alias of `CapturePrivacyGate` (M1.5 name preserved) |
| `NoopGate` | Always grants (tests / open-dev) |
| Helpers | `is_capture`, `permission_covered` (fs prefix), `infer_capture_permission` |

**Launch path:** still denies manifests that declare camera/mic/screen at launch
(standing ambient capture forbidden; open via granted adapter subscribe).

**Adapter open:** every required permission must be covered by session grants;
capture channels need exact match; empty required → allow (public adapters).

### `clawft-substrate` — plumb through `subscribe_adapter`

| API | Role |
|-----|------|
| `Substrate::set_gate` / `with_gate` / `clear_gate` / `has_gate` | Install ADR-012 gate |
| `Substrate::set_grants` / `grant` / `with_grants` / `grants` | Session capability grants |
| `subscribe_adapter` | Calls gate **before** `OntologyAdapter::open`; deny → `PermissionDenied` + health `error` |
| `effective_required_permissions` | Capture sensitivity + empty `permissions()` → infer Mic/Camera/Screen (omit-to-bypass closed) |
| Re-exports | `Gate`, `CapturePrivacyGate`, `NoopGate`, `StrictGate`, `AdapterOpenResult`, … |

### Mic adapter

`PERMISSIONS` is now `&[PermissionReq::Mic]` so the declaration matches
`Sensitivity::Capture` on `substrate/sensor/mic`.

### Backward compatibility

No gate installed (default) → legacy open (desktop boot / tests unchanged).
Production hosts must `set_gate(Arc::new(CapturePrivacyGate))` + grant capture
channels before subscribing capture adapters.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Replace placeholder gates with ADR-012-backed implementation | **Done** — `CapturePrivacyGate` |
| Plumb gate through `Substrate::subscribe_adapter` and open path | **Done** — authorize before `open` |
| Adapters emit `AdapterError::PermissionDenied` when denied | **Done** — substrate returns it; health event carries reason |
| Integration tests cover allow/deny paths | **Done** — unit + `tests/mock_adapter.rs` |

## Tests

```bash
scripts/build.sh check
scripts/build.sh test clawft-app clawft-substrate
```

- **check:** pass  
- **tests:** 185 passed (clawft-app + clawft-substrate nextest + doctests)

Coverage highlights:

- `lifecycle`: deny without Mic grant; allow with grant; public empty-required; fs prefix
- `snapshot`: gate deny/allow; undeclared capture still denied; public under gate; ungated legacy
- `mock_adapter`: integration allow/deny through `subscribe_adapter`

## Residual / follow-ups

1. **Desktop boot** (`clawft-gui-egui::live::native_live`) still uses ungated
   substrate — install `CapturePrivacyGate` + grant Mic when product wants
   deny-closed capture in the tray.
2. **Per-goal consent expiry** (ADR-008) and tray-chip composition (ADR-012
   invariant 3) remain compositor/kernel work.
3. **`OntologyAdapter::open` direct calls** bypass the gate by design; only
   `subscribe_adapter` is the enforced entry. Adapters may still self-check.
4. **Install-time** app-manifest ↔ adapter permission intersection (ADR-015
   rule 6 / WEFT-413) is separate.

## Files

- `crates/clawft-app/src/lifecycle.rs`
- `crates/clawft-app/src/lib.rs`
- `crates/clawft-substrate/src/snapshot.rs`
- `crates/clawft-substrate/src/lib.rs`
- `crates/clawft-substrate/src/adapter.rs`
- `crates/clawft-substrate/src/mic.rs`
- `crates/clawft-substrate/tests/mock_adapter.rs`
- `docs/plans/wave-0b-WEFT-429-result.md` (this file)
