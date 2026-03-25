# Q&A Roundtable: Design Decisions for K4+

**Format**: Questions collected from all five K3 symposium panels.
**Purpose**: These design decisions shape K4+ implementation direction.
**Context**: K2 Symposium decisions (D1-D22) remain in force. This roundtable
addresses new questions arising from K3 implementation.

---

## Tool Registry Architecture

### Q1: ToolRegistry Scope
**From**: Integration Architect

The daemon currently creates a new ToolRegistry per agent spawn, calling
`builtin_tool_catalog()` (27 allocations) each time. Should the registry
be a kernel-level singleton shared across all agents?

Options:
- (a) Kernel singleton -- one ToolRegistry in Kernel struct, Arc-shared to all agents
- (b) Per-agent registry -- each agent gets its own (current behavior)
- (c) Hierarchical -- kernel has base registry, agents can add tool overrides

### Q2: Tool-Specific Gate Actions
**From**: Research Analyst (Security)

The exec handler currently passes `"tool.exec"` to GovernanceGate for
ALL tool executions. The catalog defines per-tool `gate_action` strings
(e.g., `tool.fs.read`, `tool.fs.delete`). Should K4 use tool-specific
gate actions?

Options:
- (a) Yes, always use the tool's gate_action from its BuiltinToolSpec
- (b) Two-tier: generic "tool.exec" check first, then tool-specific check
- (c) Keep generic "tool.exec" but pass tool name + effect vector as context

### Q3: Remaining Tool Implementations
**From**: Kernel Auditor

25 tools are spec-only. What's the implementation priority for K4?

Options:
- (a) Implement all 25 in K4 (ambitious, complete catalog)
- (b) Implement highest-value 5-8 tools in K4, rest in K5
- (c) Implement only tools needed by other K4 features (containers need fs.write, etc.)
- (d) Defer all -- the 2 reference impls prove the pattern, user tools come through WASM

---

## WASM Runtime

### Q4: Wasmtime Integration Timing
**From**: RUV Expert

Wasmtime is workspace-dep'd but feature-gated. All WASM tools currently
fall through to native execution. When should the actual Wasmtime sandbox
execute real WASM modules?

Options:
- (a) K4 -- implement alongside container runtime
- (b) K5 -- implement when apps need third-party tools
- (c) K4 for validation/compilation, K5 for execution
- (d) Defer until external tool ecosystem demands it

### Q5: Module Caching Strategy
**From**: Research Analyst

Wasmtime module compilation is expensive (~50-200ms for a 1 MiB module).
Should compiled modules be cached?

Options:
- (a) In-memory LRU cache (simple, fast, lost on restart)
- (b) Disk-persisted compiled modules (survives restart, larger)
- (c) Both (memory + disk layers)
- (d) No caching (compile each execution -- simplest)

### Q6: WASI Filesystem Scope
**From**: Research Analyst (Security)

When WASI is enabled for a WASM tool, how much filesystem access should
it get?

Options:
- (a) None -- WASI disabled permanently, tools use host function calls
- (b) Read-only sandbox directory (e.g., /tmp/weft-tools/{tool-name}/)
- (c) Read-write sandbox directory with size limits
- (d) Configurable per-tool via ToolSpec.permissions

---

## Lifecycle & Versioning

### Q7: Version History Storage
**From**: Services Architect

Tool version history is currently in the chain only -- the tree metadata
stores only the current version. Should DeployedTool (with full version
Vec) be persisted?

Options:
- (a) Chain-only (current) -- query chain for history
- (b) Tree metadata -- store version array in node metadata
- (c) Dedicated version registry -- separate data structure
- (d) Both chain + tree metadata (redundant but queryable)

### Q8: Revocation Enforcement
**From**: Services Architect

Revoking a tool version marks metadata but doesn't prevent execution.
Should the ToolRegistry check revocation status before dispatch?

Options:
- (a) Yes -- registry checks tree metadata before every execution
- (b) No -- revocation is informational, governance gate handles enforcement
- (c) Yes, but cached -- registry caches revocation status, refresh on chain event

### Q9: Third-Party Tool Signing
**From**: Research Analyst (Security)

Built-in tools are kernel-signed. Third-party WASM tools will need their
own signing. What's the trust model?

Options:
- (a) Kernel keypair signs everything (central authority)
- (b) Developer keypairs with kernel-signed CA chain
- (c) Web-of-trust model (multiple signers required)
- (d) Content-addressed (hash is the identity, no signing)

---

## Integration & Architecture

### Q10: ServiceApi Trait (K2 Symposium C2)
**From**: Services Architect

C2 from the K2 symposium committed to a ServiceApi internal trait.
K3 has BuiltinTool as a tool dispatch trait. How do these relate?

Options:
- (a) ServiceApi wraps ToolRegistry (tools are services)
- (b) Separate concepts -- ServiceApi for long-running services, BuiltinTool for one-shot tools
- (c) BuiltinTool IS the ServiceApi for tool-type services (no new trait needed)

### Q11: A2ARouter Gate Layer (K2 Symposium C4)
**From**: Integration Architect

C4 committed to dual-layer gate checks. K3 has handler-time gate checks
working. When does the routing-time gate layer land?

Options:
- (a) K4 -- add GateBackend to A2ARouter.route()
- (b) K5 -- add when cross-service routing needs governance
- (c) Not needed -- CapabilityChecker in A2ARouter is sufficient
- (d) Replace CapabilityChecker with GateBackend in A2ARouter

### Q12: FsReadFileTool Path Sandboxing
**From**: Research Analyst (Security)

FsReadFileTool currently reads any path the kernel process can access.
Should it be sandboxed?

Options:
- (a) Yes -- use SandboxEnforcer.check_file_read() from clawft-plugin
- (b) Yes -- add tool-specific allowed_paths config
- (c) No -- governance gate handles authorization, tool just executes
- (d) Yes, but configurable per-environment (dev = permissive, prod = strict)

---

## RUV Ecosystem

### Q13: ruvector-snapshot for WASM State
**From**: RUV Expert

The ruvector-snapshot crate supports state checkpointing. Should WASM
tool execution state be snapshotable?

Options:
- (a) Yes -- checkpoint WASM linear memory for debugging
- (b) Yes -- checkpoint for tool migration (move running tool between nodes)
- (c) No -- tools are stateless, checkpoint is unnecessary overhead
- (d) Defer to K6 (clustering) when tool migration is relevant

### Q14: tiny-dancer for Tool Routing
**From**: RUV Expert

The ruvector-tiny-dancer-core crate provides routing intelligence. Could
it route tool execution to the optimal backend (native vs WASM)?

Options:
- (a) Yes -- use scoring to decide Native vs WASM per invocation
- (b) No -- tool manifest declares backend, no runtime decision
- (c) Yes, but K5+ -- routing decisions need workload data first
