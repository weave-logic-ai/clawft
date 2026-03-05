# A2A & Services Architecture Review

**Presenter**: Services Architect
**Scope**: Service runtime readiness for K3+ agents-as-services

---

## Executive Summary

The current A2A protocol, service registry, and app framework provide a
**solid foundation** but require **targeted extensions** to support the
full vision of agents running as cryptographically-contracted services
with APIs, endpoints, workflows, and shell scripts.

## Current Architecture

### A2A Router (a2a.rs, 797 lines)

**What works well:**
- Per-agent inboxes with bounded channels (1024 capacity)
- Five routing modes: Direct PID, Topic pub/sub, Broadcast, Service, Kernel
- Capability-checked routing via CapabilityChecker
- Chain-logged send_checked (exochain feature)

**What's missing:**
- Service routing is a stub (logs "not yet implemented" and returns Ok)
- No multi-protocol support (kernel IPC only, no HTTP/gRPC/Shell)
- No request-response tracking (correlation_id exists but no timeout logic)
- No cross-node routing (cluster.rs exists but A2A is local-only)

### Service Registry (service.rs, 352 lines)

**What works well:**
- SystemService trait (start, stop, health_check)
- DashMap-based registry with lifecycle management
- ServiceType enum (Core, Plugin, Cron, Api, Custom)
- Tree registration (exochain feature)

**What's missing:**
- No API/endpoint registration (services can't declare routes)
- No protocol metadata (ServiceType::Api is just a label)
- No service dependencies or ordering (start_all is concurrent)
- No service discovery (local registry only)
- No contract enforcement (no schema validation)

### App Framework (app.rs, 980 lines)

**What works well:**
- AppManifest with agents, tools, services, capabilities, hooks
- State machine lifecycle (Installed -> Starting -> Running -> Stopped)
- Manifest validation (name uniqueness, format checks)
- Namespaced IDs (app-name/agent-id, app-name/tool-name)

**What's missing:**
- No agent-to-service binding (agents can't declare "I provide service X")
- No API schemas in service declarations
- Hooks are file paths, not structured API contracts
- No actual deployment (state tracking only, no agent spawning)
- No per-app resource isolation

### Agency Model (agency.rs, 350 lines)

**What works well:**
- AgentRole hierarchy (Root, Supervisor, Service, Worker, User, Custom)
- Agency with spawn permissions and capability ceilings
- AgentManifest with interface protocol declarations
- AgentInterface with IPC/REST/gRPC/MCP protocol options

**What's missing:**
- AgentInterface is declarative only (no runtime enforcement)
- No service capability advertisement
- No runtime contract validation

## Gap Analysis

### Service Routing

```
Current:
  Agent A --> MessageTarget::Service("redis") --> "not yet implemented"

Needed:
  Agent A --> MessageTarget::Service("redis")
          --> ServiceRegistry.resolve("redis") --> PID 5
          --> A2ARouter.deliver_to_inbox(5, msg)
          --> Agent B (redis service) handles request
```

### Multi-Protocol Support

```
Current:
  All agents communicate via kernel IPC (mpsc channels)

Needed:
  IPC Agent     <--> kernel inbox (current)
  HTTP Agent    <--> HTTP listener + router
  gRPC Agent    <--> gRPC server + stubs
  Shell Agent   <--> subprocess stdin/stdout
  MCP Agent     <--> MCP server + tools
```

### Request-Response Pattern

```
Current:
  send(msg) --> fire-and-forget, agent manually correlates replies

Needed:
  let reply = rpc.call("my-service", "get_status", args).await?;
  // Timeout-based, auto-correlated, typed response
```

## Proposed Architecture

### Phase 1: Basic Service Routing (1-2 weeks)

1. Implement ServiceTarget resolution in A2ARouter
2. Agent-as-service binding (manifest.provides_services -> ServiceRegistry)
3. Basic RPC abstraction (call/reply with timeout)

### Phase 2: Multi-Protocol Support (2-3 weeks)

4. HTTP endpoint binding via ProtocolAdapter trait
5. Shell service adapter (subprocess pipes)
6. Multi-protocol agent loop (tokio::select across sources)

### Phase 3: Contract Enforcement (2-3 weeks)

7. Service API schemas in manifests (OpenAPI, gRPC proto, MCP)
8. Service-level governance rules
9. Cryptographic service calls (call_with_proof via exochain)

### Phase 4: Advanced Features (3-4 weeks)

10. Cross-node service discovery
11. Workflow orchestration engine
12. SLA monitoring with governance feedback

## Agent Loop Evolution

The current agent_loop handles inbox messages only. For K3+, it needs
to become a multi-protocol event loop:

```
loop {
    tokio::select! {
        _ = cancel.cancelled() => break,

        // IPC inbox (current)
        msg = inbox.recv() => handle_ipc(msg),

        // HTTP requests (K3+)
        req = http_listener.accept() => handle_http(req),

        // Shell output (K3+)
        line = shell_stdout.read_line() => handle_shell(line),
    }
}
```

## Governance for Services

The existing GovernanceGate needs new action types for K3+:

| Action | Risk | Security | Use Case |
|--------|------|----------|----------|
| tool.wasm.load | 0.3 | 0.2 | Loading WASM module |
| tool.wasm.exec | 0.4 | 0.3 | Executing WASM tool |
| container.start | 0.5 | 0.4 | Starting Docker container |
| container.exec | 0.7 | 0.6 | Running command in container |
| app.install | 0.4 | 0.3 | Installing application |
| app.start | 0.5 | 0.4 | Starting application |
| service.call | 0.3 | 0.2 | Cross-service RPC |
| http.endpoint | 0.4 | 0.5 | Exposing HTTP endpoint |

## Cryptographic Service Contracts

With exochain enabled, every service call can produce a verifiable proof:

```
1. Gate check -> GovernanceDecision logged to chain
2. Service call -> Intent logged to chain (caller, service, method, args)
3. Execution -> Result logged to chain
4. Witness bundle -> Cryptographic receipt linking all three events
```

This creates a **cryptographically-contracted service model** where:
- Every permission grant is auditable
- Every service call is traceable
- Every result is attributable
- The full interaction can be independently verified

## Key Questions for Project Lead

See [Q&A Roundtable](./07-qa-roundtable.md) for the full list of design
decisions needed before K3+ implementation begins.
