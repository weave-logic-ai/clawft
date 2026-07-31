# ADR-075: Grok Build (and peer CLIs) as WeftOS MCP clients

**Date**: 2026-07-30  
**Status**: Accepted  
**Deciders**: product + integration (owner: Grok CLI must drive a live WeftOS instance as a first-class agent client)  
**Depends-On**: ADR-070 (MCP registry ownership), existing `weft mcp-server` / `McpServerShell`  
**Relates-To**: ADR-054 (claude-flow via MCP), ADR-073 (`WindowIntent` + MCP as equal intent source), ADR-074 (xAI voice; separate from CLI MCP), **ADR-076** (tool surface principles + capability catalog), Claude outbound bridge in `docs/guides/tool-calls.md`

## Context

### Product ask

WeftOS has an MCP-type architecture. Operators want **Grok Build** (`grok` CLI / TUI) to:

1. Connect to a WeftOS instance as an **MCP client**  
2. Discover and call WeftOS tools (files, shell, skills, agent/runtime control)  
3. Eventually drive Agent Workspace verbs (`WindowIntent`) and multi-agent conductor flows without embedding Grok inside egui  

The same pattern already exists for **Claude Code** (`claude mcp add clawft -- weft mcp-server`). Grok Build natively supports MCP (stdio + HTTP/SSE). The gap is productization: docs, stable control surface, remote serve mode, and session identity — not a new protocol.

### What already ships

| Capability | Location | Notes |
|------------|----------|--------|
| **WeftOS as MCP server** | `weft mcp-server` → `McpServerShell` | stdio JSON-RPC; middleware (Security/Permission/Result/Audit) |
| **WeftOS as MCP client** | `tools.mcp_servers` / `weft mcp add` | stdio + HTTP; ADR-070 hybrid config/daemon ownership |
| **Claude outbound bridge** | docs + same binary | Documented; Grok is config-shape parity |
| **Grok host in-repo** | `.grok/rules`, skills, Ruflo MCP | Ruflo orchestration — **not** the WeftOS control plane |

### Levels of “connected to an instance”

| Level | Meaning | Status |
|-------|---------|--------|
| **L1 Tool client** | Grok calls WeftOS tools over MCP | Ready (stdio); needs docs + default config |
| **L2 Control plane** | Stable tools for agents list/spawn, status, `WindowIntent` | Partial; must not dump entire registry blindly |
| **L3 Remote instance** | Grok on laptop → WeftOS on server over HTTP/SSE MCP | Gap: serve path is stdio-only today |

### Non-goal confusion: Grok Voice vs Grok CLI

- **ADR-074** = speech-to-speech / Talk-Mode mouth-and-ears (xAI Realtime).  
- **This ADR** = coding/ops agent harness (Grok Build) attached as MCP client.  
Both may eventually emit `WindowIntent`; they are different frontends on the same bus (ADR-073).

## Decision

### 1. Primary posture: external agent harnesses are MCP clients of WeftOS

**Normative direction of control for “Grok works inside WeftOS”:**

```text
Grok Build (MCP client)
    ── tools/list, tools/call ──►
weft mcp-server (WeftOS tool surface + governance middleware)
    ──► ToolRegistry / skills / optional WindowIntent + agent runtime
```

We do **not** require Grok to be a subprocess of the GUI or a proprietary WeftOS-only plugin protocol. MCP is the integration contract.

### 2. First-class peer: Grok Build is equal to Claude Code on the outbound bridge

Document and maintain **parity** for:

| Client | Register WeftOS |
|--------|-----------------|
| Claude Code | `claude mcp add clawft -- weft mcp-server` |
| Grok Build | `grok mcp add weftos -- weft mcp-server` (or project `.grok/config.toml`) |

Server name `weftos` (preferred) or `clawft` for legacy Claude docs — both acceptable; project template uses **`weftos`**.

### 3. Curated control plane, not only full registry dump

Normative profiles, public names, daemon façade, and the **capability catalog** are defined in **[ADR-076](adr-076-mcp-tool-surface-capability-catalog.md)** and `docs/weftos/mcp-capability-catalog.md`.

Summary for this ADR:

| Profile | Role |
|---------|------|
| **`default`** (Grok attach) | `control` ∪ `workspace` — not full dump |
| **`full`** | Dev / explicit only |
| **`media`** | Session-bound voice/canvas — opt-in |

| Area | Examples |
|------|----------|
| Control | `status`, `agent_list` / `agent_spawn`, `window_*`, `skill_get` |
| Workspace | sandboxed FS, `exec_shell`, file `memory_*` |
| Not default | Admin weave, N skill tools, MCP re-export, media |

Empty `tools.allowed_tools` must not mean “expose everything” as the product default for `mcp-server` (see ADR-076).

### 4. One intent bus; many frontends

MCP tool calls that change layout or agent visibility **must** map to the same `WindowIntent` (ADR-073) path as keyboard, GUI, and voice (ADR-074). No Grok-only layout hacks.

### 5. Transports

| Transport | Role |
|-----------|------|
| **stdio** | Default local attach (Grok/Claude spawn `weft mcp-server`) |
| **HTTP / SSE / streamable HTTP** | Remote attach (L3); **must ship** as `weft mcp-server` listen mode or equivalent gateway |

Remote mode requires auth (bearer / capability token). No open-internet anonymous serve as default.

### 6. Session identity and capability (L2+)

MCP sessions are not anonymous forever:

- Prefer binding a **session capability** (scope: which tools, workspace roots, agent-spawn rights).  
- Align with ADR-071 spirit (capability tokens) without requiring WASM panel machinery for CLI clients.  
- Audit log must record client label (`grok` / `claude` / `unknown`) when known.

### 7. Bidirectional bridge optional; recursion guarded

WeftOS may still **consume** external MCP servers (inbound). Running Grok tools inside WeftOS *and* WeftOS tools inside Grok simultaneously is allowed only with **delegation loop protection** (existing Claude bridge rules apply). Default product story remains **Grok → WeftOS** (outbound from WeftOS’s perspective of “serving”).

### 8. Config templates in-repo

Ship project-local Grok MCP stanza (`.grok/config.toml` or documented snippet) so `grok mcp doctor weftos` works after build + PATH. Do not hardcode machine-specific absolute paths in committed config when `weft` is on PATH.

## Non-goals

- Embedding Grok Build UI inside the egui shell.  
- Replacing Ruflo / claude-flow MCP (orchestration) with WeftOS tools — they are complementary.  
- Making xAI cloud the only way to attach (local `weft` binary remains primary).  
- Vendoring the Grok binary into WeftOS releases.  
- Replacing ADR-070 ownership model.

## Implementation phases

| Phase | Cycle | Plane | Work | Exit |
|-------|-------|-------|------|------|
| **G0** | 0.8.x | **WEFT-692** | Docs + guide + project Grok config stanza; `grok mcp doctor weftos` path | Manual attach works on dev machine |
| **G1** | 0.8.x | **WEFT-693** | Control-plane profile / curated tool list (implements ADR-076 profiles) | Grok session not flooded; documented tools |
| **G2** | 0.8–0.9 | **WEFT-694** | Control tools: status, agents list/spawn (policy), hooks for WindowIntent | CLI conductor basics without freeform WM |
| **G3** | 0.9.x | **WEFT-695** | Wire MCP → `WindowIntent` (with ADR-073 Phase D / WEFT-688) | Grok moves panes / spawns visible agents |
| **G4** | 0.9.x | **WEFT-696** | HTTP/SSE (or streamable HTTP) serve + auth | Remote Grok → WeftOS instance |
| **G5** | 0.9–1.0 | **WEFT-697** | Session capability tokens + audit client label | Multi-tenant / enterprise attach |

## Consequences

### Positive

- Reuses existing MCP server shell and Claude bridge investment.  
- Grok, Claude, and future harnesses share one contract.  
- Aligns Agent Workspace and voice on the same tool/intent plane.  
- Clear L1→L3 ladder for shipping.

### Negative / costs

- Tool surface curation requires ongoing product judgment.  
- Remote serve expands attack surface (auth mandatory).  
- Dual attach (Grok + WeftOS agent both running) can confuse users if not documented.

## References

- https://docs.x.ai/build/overview — Grok Build  
- Grok MCP config: `~/.grok/docs/user-guide/07-mcp-servers.md` (host docs)  
- `weft mcp-server` — `crates/clawft-cli/src/commands/mcp_server.rs`  
- `McpServerShell` — `crates/clawft-services/src/mcp/server.rs`  
- `docs/guides/tool-calls.md` — Claude outbound bridge  
- `docs/guides/grok-weftos-mcp.md` — operator guide (this decision)  
- ADR-070, ADR-073, ADR-074, **ADR-076** (capability catalog / profiles)  
