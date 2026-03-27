# Phase K4: Container Integration -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. Types without bollard dependency
**Problem**: Adding `bollard` (Docker API crate) to the workspace would increase compile times and require Docker headers. The `containers` feature gate should control this.

**Decision**: All types (ContainerConfig, ContainerState, PortMapping, VolumeMount, RestartPolicy, ManagedContainer, ContainerError) are compiled unconditionally. The actual Docker engine integration is behind `#[cfg(feature = "containers")]`. Without the feature, `start_container()` returns `ContainerError::DockerNotAvailable`.

**Rationale**: Same pattern as K3 (WASM). Types are useful for configuration, serialization, and error handling even without Docker. The feature gate only controls the heavyweight bollard dependency.

### 2. DashMap for managed containers
**Decision**: ContainerManager stores managed containers in `DashMap<String, ManagedContainer>`, keyed by container name.

**Rationale**: Consistent with ProcessTable and ServiceRegistry patterns elsewhere in the kernel. DashMap provides lock-free concurrent access without `Mutex<HashMap<>>`.

### 3. Health integration via HealthStatus
**Decision**: `health_check()` maps ContainerState to the existing `HealthStatus` enum: Running -> Healthy, Stopped -> Unhealthy, Failed -> Unhealthy, other -> Degraded.

**Rationale**: Reuses the kernel's health system rather than inventing a separate container health model. Makes container health visible through the standard `HealthSystem` aggregator.

### 4. Lazy Docker connection
**Decision**: `ContainerManager::new()` does NOT attempt to connect to Docker. Connection is deferred to the first operation that needs it (start_container).

**Rationale**: The manager should be constructable in unit tests and in environments without Docker. Fail-at-use rather than fail-at-construction.

### 5. Clippy: needless return in cfg block
**Problem**: Clippy flagged `return Err(...)` as needless in the `#[cfg(not(feature = "containers"))]` block because it was the last expression.

**Decision**: Changed to expression-form `Err(...)` without `return`.

**Rationale**: Clippy compliance. The `#[cfg]` blocks each form their own expression context, so the last expression in each block is the function return value.

## What Was Skipped

1. **Actual bollard integration** -- Docker API calls (pull, create, start, stop, inspect). Requires adding bollard dependency.
2. **Container network management** -- Creating and managing the "weftos" Docker network.
3. **ServiceRegistry wiring** -- Wrapping containers in ServiceType and registering with the registry.
4. **Health check endpoint probing** -- HTTP health check polling for containers with health_endpoint.
5. **Container logs streaming** -- Tailing container stdout/stderr.
6. **Port conflict detection** -- Checking host port availability before binding.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/container.rs` | ~647 | ContainerManager, config, types, errors, 15 tests |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/lib.rs` | Added container module, doc comment, re-exports |

## Test Summary

- 15 new tests in container.rs (config, serde, states, register/list, stop, health, stop_all, error display, restart policy)
- All 167 kernel tests pass
- All workspace tests pass
