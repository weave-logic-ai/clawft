# WeftOS K3 Symposium: Tool Lifecycle Review

**Date**: 2026-03-04
**Branch**: feature/weftos-kernel-sprint
**Status**: K3 Tool Lifecycle Complete | K4+ Planning
**Predecessor**: [K2 Symposium](../k2-symposium/00-symposium-overview.md)

---

## Purpose

This symposium evaluates the K3 Tool Lifecycle implementation -- Build, Deploy,
Execute, Version, Revoke -- which landed immediately after the K2.1 breaking
changes. The same specialist panels reconvene to verify the implementation,
run live tool executions, stress-test the lifecycle chain, and identify gaps
before K4 (Container Runtime) begins.

## Panel Roster

| Panel | Lead | Scope |
|-------|------|-------|
| Tool Catalog & Registry | Kernel Auditor | 27 tools, ToolRegistry, BuiltinTool trait, reference impls |
| Lifecycle & Chain Integrity | Services Architect | Build/Deploy/Version/Revoke, TreeManager, chain events |
| Sandbox & Security | Research Analyst | WASM sandbox config, fuel metering, memory limits, governance gate |
| Agent Loop & Dispatch | Integration Architect | exec handler, tool_registry wiring, daemon integration |
| Live Testing & Validation | RUV Expert | Manual tool execution, E2E lifecycle, test coverage gaps |

## Key Findings

**K3 Readiness Score: 85%**

- 421 tests passing (with exochain), up from 397 post-K2.1
- 27 built-in tools cataloged with JSON Schema parameters
- 2 reference implementations fully operational (fs.read_file, agent.spawn)
- Full lifecycle chain: Build -> Deploy -> Execute -> Version -> Revoke
- All 50 new/updated tests passing across 4 modules

**What Shipped**

- `BuiltinToolSpec` type with 27-tool catalog across 3 categories
- `ToolVersion` with SHA-256 hashing + Ed25519 signing
- `ToolRegistry` with `BuiltinTool` trait dispatch
- `FsReadFileTool` -- reads files with offset/limit, returns content + metadata
- `AgentSpawnTool` -- spawns agents via ProcessTable, returns PID
- `TreeManager` lifecycle: `build_tool`, `deploy_tool`, `update_tool_version`, `revoke_tool_version`
- Agent loop `exec` handler wired to ToolRegistry dispatch with chain logging
- Boot-time registration of 27 tool nodes in resource tree
- Daemon wires ToolRegistry into every spawned agent

**What's Deferred**

- Wasmtime runtime integration (behind `wasm-sandbox` feature gate)
- ServiceApi trait (C2 from K2 symposium)
- Dual-layer gate in A2ARouter routing (C4)
- N-dimensional EffectVector (C9)
- 25 remaining tool implementations (specs defined, no execute logic)

## Documents

1. [Tool Catalog & Registry Audit](./01-tool-catalog-audit.md) -- 27-tool catalog review
2. [Lifecycle & Chain Integrity](./02-lifecycle-chain-integrity.md) -- Build/Deploy/Version/Revoke verification
3. [Sandbox & Security Review](./03-sandbox-security-review.md) -- WASM config, governance, attack surface
4. [Agent Loop & Dispatch Wiring](./04-agent-loop-dispatch.md) -- exec handler, daemon integration
5. [Live Testing & Validation](./05-live-testing-validation.md) -- Manual test runs, E2E verification
6. [Q&A Roundtable](./06-qa-roundtable.md) -- Design questions for K4+ direction
7. [Symposium Results Report](./07-symposium-results-report.md) -- Decisions, verdicts, approved changes

## Recommended Reading Order

Start with **05-live-testing-validation** for the hands-on verification, then
**01-tool-catalog-audit** for the catalog structure, then **06-qa-roundtable**
for the K4+ design decisions.
