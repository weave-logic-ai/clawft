# ADR-036: Hierarchical ToolRegistry with Kernel Base and Per-Agent Overlays

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K3 Symposium D1 (Hierarchical ToolRegistry), Sprint 09b Decision Resolutions (CF-3)

## Context

WeftOS ships 27 built-in tools (filesystem, shell, agent management, chain operations, etc.) that are registered during kernel boot. Before the hierarchical model, every agent received its own full copy of the tool registry, which Sprint 09b finding CF-3 identified as wasteful allocation -- 27 tool specs duplicated per agent. Additionally, governance needs to restrict tool access per-agent (e.g., an untrusted agent should not have access to `shell_exec`), and different environments may need different tool availability at boot time.

K3 Symposium D1 evaluated three options: (a) singleton registry, (b) per-agent copies, and (c) hierarchical with shared base and per-agent overlays. Option (c) was selected because it resolves the CF-3 wasteful allocation while enabling fine-grained governance control.

## Decision

The `ToolRegistry` struct in `crates/clawft-kernel/src/wasm_runner/registry.rs` uses a hierarchical parent-child model:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn BuiltinTool>>,
    parent: Option<Arc<ToolRegistry>>,
    require_signatures: bool,
    trusted_keys: Vec<[u8; 32]>,
    signatures: HashMap<String, ToolSignature>,
}
```

**Kernel boot** (`boot.rs`) creates one shared base registry with all 27 built-in tools. This base registry is wrapped in `Arc<ToolRegistry>` and shared across all agents.

**Per-agent overlays** are created via `ToolRegistry::with_parent(parent: Arc<ToolRegistry>)`. Each agent gets its own overlay that can:
- Add custom tools (tools local to the overlay, not visible to other agents)
- Override base tools (shadow a base tool with a modified version)
- Work with governance deny-lists (governance can inject deny entries at the agent layer)

**Lookup walks the chain**: When a tool is requested, the registry checks the local `tools` HashMap first. If not found and a `parent` exists, it delegates to the parent. This continues up the chain.

**Tool signing**: The registry supports `require_signatures` mode where only tools with valid Ed25519 signatures (via `ToolSignature`) can be registered. Trusted public keys are stored as `Vec<[u8; 32]>`. The `register_signed()` method verifies signatures against `trusted_keys` before adding the tool.

**Built-in tool contract**: Each tool implements the `BuiltinTool` trait:
```rust
pub trait BuiltinTool: Send + Sync {
    fn name(&self) -> &str;
    fn spec(&self) -> &BuiltinToolSpec;
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}
```

Each `BuiltinToolSpec` includes the tool name, description, `ToolCategory`, `gate_action` string, `EffectVector` (per ADR-034), and a `native` flag.

## Consequences

### Positive
- Resolves CF-3: the 27-tool base registry is `Arc`-shared, eliminating per-agent duplication of tool specs and implementations
- Governance can restrict tools per-agent by injecting deny-lists or omitting tools from agent overlays without modifying the shared base
- Custom agent tools are scoped to the overlay -- they do not pollute the base registry or leak to other agents
- Tool signing with Ed25519 verification enables supply-chain security for third-party tools
- Four tests verify hierarchical behavior: parent lookup, child override, child-only tools, parent accessor

### Negative
- The parent chain lookup adds one indirection per tool call when the tool is not in the overlay -- for the common case (base tools), every call traverses overlay -> base
- Governance deny-lists at the agent layer require coordination between the governance engine and the overlay registry; there is no built-in deny-list data structure -- governance must prevent tool execution at the gate level (per ADR-033/034)
- The `HashMap`-based overlay (not `DashMap`) means per-agent tool registration is not concurrent-safe; tool registration is expected during agent initialization, not at runtime

### Neutral
- The GUI tool browser displays tools by walking the hierarchy, showing which tools are base vs. overlay
- All future tool registration (WASM tools, plugin tools, K4 ClawHub tools) must follow this pattern: register in the overlay, not the base
