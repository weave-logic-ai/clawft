# ADR-047: Self-Calibrating Cognitive Tick

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: ECC Symposium (D3, Q4 Resolution), K3c Implementation

## Context

The cognitive tick is the heartbeat of the ECC cognitive substrate -- the interval at which the kernel processes causal reasoning, HNSW queries, impulse dispatch, and cross-reference updates. A fixed tick interval would force a choice: fast enough for a server (10ms) but wasteful on constrained hardware, or conservative enough for an ESP32 (100ms) but sluggish on a server.

The ECC Symposium (Q4) resolved this with a user insight: the tick interval should be hardware-determined, not hardcoded. The Mentra project demonstrated that ECC is viable on a $30 ARM SoC (Cortex-A53) at 3.6ms per cognitive tick. A heterogeneous cluster where a glasses node (50ms tick) and a server node (10ms tick) participate in the same nervous system is an explicit design goal (D1: the nervous system model).

K3c implemented `CognitiveTick` (`cognitive_tick.rs`) and `run_calibration()` (`calibration.rs`) as kernel services behind the `ecc` feature gate.

## Decision

The cognitive tick interval is self-calibrated at boot, auto-adjusted at runtime, and advertised to peers as a cluster membership property.

**Boot-time calibration** (`calibration.rs`):
- `run_calibration()` exercises the HNSW index and CausalGraph with synthetic data (deterministic pseudo-random vectors via LCG, avoiding the `rand` dependency)
- Measures per-tick latency across `calibration_ticks` iterations (default: 30)
- Produces `EccCalibration` with: `compute_p50_us`, `compute_p95_us`, `tick_interval_ms` (auto-adjusted), `headroom_ratio` (p95/tick_interval), `hnsw_vector_count`, `causal_edge_count`, `spectral_capable` flag, and `calibrated_at` timestamp
- Configuration via `EccCalibrationConfig`: `calibration_ticks` (default 30), `tick_interval_ms` (default 50ms), `tick_budget_ratio` (default 0.3 = 30% of tick for compute), `vector_dimensions` (default 384)
- If p95 compute time exceeds the budget ratio, the tick interval is automatically raised

**Runtime adaptive adjustment** (`cognitive_tick.rs`):
- `CognitiveTick` service implements `SystemService` with `CognitiveTickConfig`: `tick_interval_ms`, `tick_budget_ratio` (0.3), `calibration_ticks` (100), `adaptive_tick` (true), `adaptive_window_s` (30s)
- `CognitiveTickStats` tracks: `tick_count`, `current_interval_ms`, `avg_compute_us`, `max_compute_us`, `drift_count` (ticks exceeding budget), `running` flag
- Internal `CognitiveTickState` maintains `recent_timings_us` for windowed averaging
- When `adaptive_tick` is enabled, the interval adjusts based on recent compute timings within the `adaptive_window_s` window

**Cluster integration** (via `boot.rs`):
- Calibration runs during ECC boot: `run_calibration(&hnsw, &causal, &cal_config)`
- Results are logged: "ECC calibration complete (p50={us}, p95={us}, tick={ms}, spectral={})"
- `CognitiveTickConfig.tick_interval_ms` is set from `calibration.tick_interval_ms`
- Calibration results are logged to ExoChain as an `ecc.boot.calibration` event
- The `tick_interval_ms` is advertised to peers as a cluster membership property, enabling heterogeneous tick rates within the same cluster

## Consequences

### Positive
- Hardware-appropriate performance: a server gets 10ms ticks, a glasses node gets 50ms ticks, without configuration changes
- The 30% budget ratio ensures 70% headroom for other kernel work (IPC, governance, mesh heartbeat)
- Adaptive adjustment handles load spikes without operator intervention
- Calibration results as chain events provide hardware capability audit trail
- Heterogeneous tick rates enable the "distributed nervous system" vision where every device class participates

### Negative
- Calibration adds startup latency: 30 synthetic ticks at 50ms = ~1.5s minimum boot delay for ECC
- Synthetic benchmarks may not accurately predict real workload performance (HNSW with random vectors vs. real embeddings)
- Heterogeneous tick rates complicate delta synchronization: a 10ms server generates 5x more deltas per second than a 50ms node, creating asymmetric sync pressure
- The adaptive window (30s default) introduces lag in responding to sustained load changes
- Clock drift between the adaptive adjustment and the calibrated baseline is not bounded -- a heavily loaded system may drift far from its boot-time calibration

### Neutral
- The `spectral_capable` flag gates whether spectral analysis (Lanczos eigensolver) runs on this node, providing a clean degradation path for constrained hardware
- The deterministic pseudo-random vector generator avoids a `rand` dependency in calibration, keeping the `ecc` feature gate lightweight
- The `drift_count` metric in `CognitiveTickStats` provides observability into tick budget violations without requiring external monitoring
