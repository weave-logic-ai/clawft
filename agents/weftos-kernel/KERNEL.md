---
name: weftos-kernel
description: WeftOS kernel development skill — how to add modules, test patterns, feature flags, and navigate the K0-K6 phase structure
version: 0.1.0
category: development
tags:
  - kernel
  - rust
  - weftos
  - modules
  - feature-flags
  - testing
author: WeftOS Kernel Team
---

# WeftOS Kernel Development Skill

This skill teaches how to work on the `clawft-kernel` crate: adding modules,
using feature flags, writing tests, and understanding the K0-K6 phase structure.

## Crate Location

```
crates/clawft-kernel/
  src/
    lib.rs          # Module declarations, feature gates, re-exports
    boot.rs         # Kernel boot sequence
    process.rs      # ProcessTable, PID management
    service.rs      # ServiceRegistry, SystemService trait
    ipc.rs          # KernelIpc, KernelMessage, KernelSignal
    capability.rs   # AgentCapabilities, RBAC
    health.rs       # HealthSystem
    supervisor.rs   # AgentSupervisor, spawn backends
    a2a.rs          # A2ARouter (agent-to-agent IPC)
    governance.rs   # GovernanceEngine
    mesh.rs         # Mesh transport traits (feature = "mesh")
    mesh_*.rs       # Mesh subsystem modules (feature = "mesh")
    causal.rs       # CausalGraph (feature = "ecc")
    hnsw_service.rs # HNSW vector index (feature = "ecc")
    impulse.rs      # ImpulseQueue (feature = "ecc")
    ...
  Cargo.toml
```

## K0-K6 Phase Structure

The kernel is built incrementally across phases. Each phase adds a subsystem:

| Phase | ID | What It Added | Status |
|-------|----|---------------|--------|
| K0 | Foundation | `Kernel`, `ProcessTable`, `ServiceRegistry`, `HealthSystem`, `ClusterMembership`, ExoChain, TreeManager | Complete |
| K1 | Supervisor/RBAC | `AgentSupervisor`, `AgentCapabilities`, spawn-and-run, GateBackend, agency | Complete |
| K2 | A2A IPC | `A2ARouter`, `KernelMessage`, `TopicRouter`, pub/sub | Complete |
| K2b | Hardening | Health monitor, watchdog, graceful shutdown, suspend/resume | Complete |
| K2.1 | Symposium Impl | `SpawnBackend` enum, `ServiceEntry`, post-quantum signing | Complete |
| K3 | WASM Sandbox | Wasmtime tool execution, fuel metering, `ServiceApi` trait | In Progress |
| K3c | ECC Substrate | `CausalGraph`, `HnswService`, `CrossRefStore`, `CognitiveTick`, `ImpulseQueue` | In Progress |
| K4 | Containers | `ContainerManager`, sidecar orchestration | Planned |
| K5 | App Framework | `AppManager`, `AppManifest`, application lifecycle | Planned |
| K6 | Mesh Networking | Transport traits, Noise encryption, framing, discovery, sync | In Progress |

## How to Add a New Kernel Module

### 1. Create the source file

```bash
# Example: adding a new module for resource quotas
touch crates/clawft-kernel/src/quota.rs
```

### 2. Add module declaration to lib.rs

Modules are organized by section in `lib.rs`. Place your module in the appropriate section:

```rust
// If unconditional (always compiled):
pub mod quota;

// If behind a feature flag:
#[cfg(feature = "os-patterns")]
pub mod quota;
```

Feature-gated module sections in `lib.rs`:
- **ECC modules** (`feature = "ecc"`): `causal`, `cognitive_tick`, `crossref`, `hnsw_service`, `impulse`, `calibration`
- **Mesh modules** (`feature = "mesh"`): `mesh`, `mesh_noise`, `mesh_framing`, `mesh_listener`, `mesh_discovery`, etc.
- **ExoChain modules** (`feature = "exochain"`): `chain`, `tree_manager`, `gate`
- **Unconditional**: `boot`, `process`, `service`, `ipc`, `capability`, `health`, `supervisor`, etc.

### 3. Add re-exports

At the bottom of `lib.rs`, add `pub use` for key types:

```rust
#[cfg(feature = "os-patterns")]
pub use quota::{QuotaManager, QuotaPolicy};
```

### 4. Update Cargo.toml features

If your module needs a new feature flag:

```toml
[features]
os-patterns = ["exochain"]
```

### 5. Write tests

Tests go inside the module file using `#[cfg(test)] mod tests { ... }`.

## Build Commands

Always use `scripts/build.sh`, never raw cargo:

```bash
# Fast compile check (no codegen)
scripts/build.sh check

# Run all tests
scripts/build.sh test

# Lint with clippy (warnings = errors)
scripts/build.sh clippy

# Full phase gate (11 checks, run before committing)
scripts/build.sh gate

# Debug build (faster iteration)
scripts/build.sh native-debug

# Release build
scripts/build.sh native

# Build with extra features
scripts/build.sh native --features ecc,mesh
```

## Testing Patterns

### 1. Unit tests inside the module

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_type_defaults() {
        let t = MyType::default();
        assert_eq!(t.field, expected_value);
    }
}
```

### 2. Serde roundtrip tests

Every type that derives `Serialize, Deserialize` should have a roundtrip test:

```rust
#[test]
fn my_type_serde_roundtrip() {
    let original = MyType {
        field_a: "value".into(),
        field_b: 42,
    };
    let json = serde_json::to_string(&original).unwrap();
    let restored: MyType = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.field_a, original.field_a);
    assert_eq!(restored.field_b, original.field_b);
}
```

### 3. Feature-gated tests

Tests that depend on a feature are gated the same way as the module:

```rust
#[cfg(all(test, feature = "ecc"))]
mod ecc_tests {
    use super::*;
    // ...
}
```

### 4. Error display tests

Every error enum variant should have a display test:

```rust
#[test]
fn error_display_variant_name() {
    let err = MyError::SomeVariant("details".into());
    assert_eq!(err.to_string(), "expected message: details");
}
```

### 5. Async tests (with tokio)

```rust
#[tokio::test]
async fn async_operation_works() {
    let result = my_async_fn().await;
    assert!(result.is_ok());
}
```

## File Organization Rules

1. **One module per file**: Each kernel subsystem gets its own file
2. **Keep files under 500 lines**: Split into submodules if needed
3. **Feature gates at the module level**: Gate in `lib.rs`, not inside the file
4. **Re-export key types**: Public API types get `pub use` in `lib.rs`
5. **Tests at the bottom**: `#[cfg(test)] mod tests` at the end of each file
6. **Doc comments on all public items**: `///` for types, fields, methods

## Common Patterns

### SystemService trait

Register services in the kernel at boot:

```rust
#[async_trait]
pub trait SystemService: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn service_type(&self) -> ServiceType;
    async fn start(&self) -> Result<(), KernelError>;
    async fn stop(&self) -> Result<(), KernelError>;
    async fn health(&self) -> HealthStatus;
}
```

### KernelMessage for IPC

Agent-to-agent communication uses typed messages:

```rust
let msg = KernelMessage {
    source: my_pid,
    target: MessageTarget::Pid(target_pid),
    payload: MessagePayload::Json(serde_json::to_value(&my_data)?),
    correlation_id: None,
};
router.send(msg).await?;
```

### Tree registration

Services register in the resource tree for discoverability:

```
/kernel/services/{service_name}
/kernel/processes/{pid}
/agents/{agent_id}/...
```

## Related Files

- **SPARC orchestrator**: `.planning/sparc/weftos/00-orchestrator.md`
- **Gap-filling plan**: `.planning/sparc/weftos/08-os-gap-filling.md`
- **Build script**: `scripts/build.sh`
- **Kernel Cargo.toml**: `crates/clawft-kernel/Cargo.toml`
