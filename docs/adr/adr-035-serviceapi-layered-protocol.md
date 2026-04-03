# ADR-035: Layered Protocol Architecture (ServiceApi Trait)

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K2 Symposium D4 (Layered Protocol Architecture), K3 Symposium (C2 status tracking)

## Context

The WeftOS kernel needs to expose its capabilities to external consumers through multiple protocols: MCP (Model Context Protocol) for AI tool ecosystems, gRPC for inter-service communication, Shell for interactive use, and HTTP/REST for web integrations. If each protocol adapter directly accesses kernel internals, the result is a tangled dependency graph where protocol-specific logic bleeds into kernel modules and every new protocol requires touching kernel code.

K2 Symposium D4 established that the question is not "which protocol" but "what shape is the API that protocols bind to." The decision called for an internal API layer that runs local to the kernel as a service, with protocol adapters binding to this surface. K2 D5 further decided that kernel-native performance is the foundation, with A2A and MCP as adapters over it.

## Decision

The kernel exposes an internal `ServiceApi` trait (defined in `crates/clawft-kernel/src/service.rs`) that all protocol adapters bind to. The trait defines three operations:

```rust
pub trait ServiceApi: Send + Sync {
    async fn call(
        &self,
        service: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;

    async fn list_services(&self) -> Vec<ServiceInfo>;

    async fn health(
        &self,
        service: &str,
    ) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>>;
}
```

Protocol adapters implement the translation between their wire format and `ServiceApi::call()`:

- **`ShellAdapter`** (`service.rs`): Parses `"service.method args"` command strings, extracts the service name and method, deserializes arguments from JSON or wraps as `{"args": ...}`, and dispatches through `ServiceApi::call()`.
- **`McpAdapter`** (`service.rs`): Maps MCP tool names to `service.method` via underscore or dot separator (e.g., `"kernel_status"` -> `("kernel", "status")`), passes MCP `arguments` JSON directly to `ServiceApi::call()`.
- **gRPC adapter** (planned K4/K5): Will bind gRPC service definitions to `ServiceApi`.
- **HTTP/REST adapter** (planned K5): Will map HTTP routes to `ServiceApi::call()` when networking lands.

The layered architecture is:

```
Kernel IPC (K0-K2)
  -> ServiceApi (K3 -- internal, local)
       -> MCP adapter (K3/K4)
       -> gRPC adapter (K4/K5)
       -> Shell adapter (K3)
       -> HTTP/REST adapter (K5)
```

The agent loop dispatches through the API layer, not directly to protocols. The Tauri GUI also routes through `ServiceApi` via `invoke()` commands (per ADR-007).

## Consequences

### Positive
- Single internal API surface means protocol adapters are thin translation layers -- adding a new protocol does not require touching kernel internals
- All external integrations (MCP tools, HTTP endpoints, gRPC services, shell commands) go through the same governance enforcement path via `ServiceApi`
- `ServiceApi::list_services()` provides uniform service discovery across all protocols
- The `ServiceInfo` struct gives every protocol adapter the same view of available services and their health status
- Clean separation enables testing protocol adapters independently from kernel logic

### Negative
- The `ServiceApi` trait uses `serde_json::Value` as the parameter and return type, which erases type safety at the adapter boundary -- callers must know the expected JSON schema for each service method
- All protocol adapters pay the cost of JSON serialization/deserialization even for in-process calls where typed dispatch would be zero-cost
- gRPC and HTTP adapters are planned but not yet implemented (K3 C2 status: NOT STARTED as of K3 results); the adapter pattern is proven only by Shell and MCP so far

### Neutral
- The `ServiceRegistry` (DashMap-backed, per ADR-032) provides the runtime mapping from service names to implementations; `ServiceApi::call()` resolves against this registry
- Protocol adapters hold `Arc<dyn ServiceApi>`, enabling runtime swapping of the API implementation for testing or configuration changes
