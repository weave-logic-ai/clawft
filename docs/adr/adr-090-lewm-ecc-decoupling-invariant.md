# ADR-090: LeWM ↔ ECC decoupling invariant (runtime-checkable rules)

**Status:** Accepted (2026-07-31)  
**Plane:** WEFT-519  
**Depends-On:** ECC cognitive substrate (ADR-047/056/062), LeWM symposium design  
**Relates-To:** ADR-058 (per-conversation context tier — *distinct* concern; historical LeWM prose sometimes reused the “ADR-058” label for this invariant — this ADR is the authoritative number for decoupling)  
**Supersedes:** Informal “decoupling invariant” notes in `.planning/symposiums/lewm-worldmodel/diagram.md`

## Context

The latent world model (LeWM) track introduces a learned perceptual / predictive substrate that **consumes** sensor streams and may publish impulses into ECC. Without enforceable boundaries, a world-model path could:

- short-circuit causal edges (skip ECC link evaluation),
- override local reasoning with cluster fusion,
- become a hard dependency that breaks local-only nodes.

Symposium prose already states the invariant; WEFT-519 requires **testable predicates** and **runtime checks** so violations fail loudly in debug / facade paths.

## Decision

### Five formal rules

| # | Rule | Predicate (testable) |
|---|------|----------------------|
| **R1** | ECC is authoritative per node | No WM write may replace ECC reasoning results (`WmWriteKind::ReasoningOverride` → reject) |
| **R2** | World model is optional | Sensor + ECC paths run with WM absent (`local_ecc_sufficient_without_wm() == true`) |
| **R3** | Impulse / observation write path only | WM may submit `Impulse` / `Observation`; must not call causal edge mutators (`CausalEdgeMutate` → reject) |
| **R4** | No causal short-circuit | WM must not bypass edge evaluation (`ShortCircuit` → reject) |
| **R5** | Graceful degradation | Absent cluster/WM, local-ECC-only behaviour is unchanged (same as R2 operationally; cluster fusion is observation input only) |

### Runtime surface

Implemented in `clawft-kernel::lewm_invariant` (feature `ecc`):

- `DecouplingRule` — enum of R1–R5  
- `WmWriteKind` — classification of proposed WM writes  
- `check_wm_write` — pure predicate  
- `WorldModelFacade` — counts accept/reject; `enforce` enables `debug_assert` on violation  

Full LeWM crates (`weftos-worldmodel-*`) will call this facade at every kernel injection seam. Until those crates land, unit tests lock the predicates.

### What is *not* decided

- WM training architecture, latent dim, or SIGReg details (separate ADRs / crates).  
- Cluster fusion wire format (observational packets remain out of scope here).

## Consequences

- Positive: Clear fail-closed rules for any future WM → kernel API.  
- Positive: WEFT-520 (and later LeWM work) can depend on these predicates.  
- Neutral: No behaviour change for current ECC-only builds (facade is opt-in).  
- Follow-up: Wire facade into real WorldModelService when the service process lands.

## References

- `.planning/symposiums/lewm-worldmodel/diagram.md`  
- `crates/clawft-kernel/src/lewm_invariant.rs`  
- WEFT-519 acceptance criteria
