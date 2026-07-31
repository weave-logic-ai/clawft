# ADR-073: Agent Workspace interaction principles (CNVS-informed)

**Date**: 2026-07-30  
**Status**: Accepted  
**Deciders**: product + desktop (owner directive: match the *effortlessness* of CNVS multi-window / multi-agent canvas UX)  
**Depends-On**: ADR-004 (composer / surface grammar), ADR-038 (desktop shell direction), DESIGN.md desktop archetypes  
**Relates-To**: ADR-061 / ADR-068 (voice loop), ADR-056 (BVH — **not** the window manager), ADR-185-style multi-agent runtime, research on [cnvs.dev](https://cnvs.dev/) + demo [@_MaxBlade/2082679537377144936](https://x.com/_MaxBlade/status/2082679537377144936)

## Context

[CNVS](https://cnvs.dev/) (Max Blade, closed-source **macOS Swift** app, lifetime license) demonstrates a product pattern WeftOS still lacks as a **first-class shell mode**:

1. **Spatial multi-agent stage** — agents are **windows on a canvas**, not hidden workers or a single chat tab.  
2. **Voice (and keys) drive layout** — spawn, focus, rearrange, summarize walls of text; “window management” is an *agent skill*, not only OS chrome.  
3. **Attention routing** — when an agent needs the human, the surface **glows** and voice can announce it.  
4. **Local/remote isomorphism** — remote canvases “look like” local ones (Hermes/VPS demos).  
5. **Conductor UX** — user orchestrates many parallel agents (Claude/Cursor/Codex/etc. in their product; our stack is WeftOS agents + MCP + Ruflo).

WeftOS already has much of the **hard infrastructure** (substrate, composer, multi-agent bus, voice stack design, mesh) but ships a **0.8 sidebar + single active app** desktop (multi-window / snap intentionally deferred). The gap is **orchestration UX grammar**, not a need to fork or reimplement CNVS.

**Evidence (product, not implementation):** cnvs.dev marketing + MaxBlade demos (incl. Grok Voice Think Fast 2.0 driving spawn/prompt, browsers, windows, music, and summarization inside CNVS). CNVS is closed; we adopt **principles only**.

## Decision

### 1. Adopt CNVS interaction principles, not their code or product

WeftOS will **not** port Swift, license CNVS, or clone branding/themes. We adopt the following **normative rules** for a first-class shell mode named **Agent Workspace**.

### 2. Visible-agent rule

Any long-running agent session the user or an orchestrator spawns **must open a UI surface by default** (pane/window). Background-only spawn requires **explicit** user or policy opt-in. Invisible agent fleets are a footgun for trust and attention.

### 3. Agent Workspace mode (optional, not the only desktop)

| Mode | Role |
|------|------|
| **Stock desktop (0.8)** | Calm sidebar + single active app (unchanged DESIGN.md OOB) |
| **Agent Workspace (0.9+)** | Freeform spatial stage for concurrent agent panes + tool panes (terminal, browser view, logs) |

Workspace is launchable from sidebar, palette, or voice. It does **not** replace the OS desktop metaphor for system apps.

### 4. `WindowIntent` API — one bus for voice, keys, MCP

All layout/control verbs share a small typed intent surface (name subject to implementation):

| Intent | Examples |
|--------|----------|
| `Spawn` | open agent pane with role/prompt |
| `Focus` | bring pane to front / select |
| `Arrange` | grid, cascade, stack |
| `Close` | end pane (+ optional agent cancel) |
| `Summarize` | compress agent wall-of-text into spoken/short UI |
| `Attach` | bind terminal/browser/log to an agent pane |

**Sources of intent (all equal):** keyboard, voice (ADR-074 / local), MCP tools (including Grok Build / Claude as clients — ADR-075), GUI chrome. Implementation must not special-case voice into ad-hoc egui hacks.

### 5. Attention bus

Agent “needs human” → substrate (or event bus) event → **visual attention chrome** (ring/glow/chip) and optional TTS announcement. Governed and rate-limited; not spam.

### 6. Composer-first panes; native WM for freeform only

Prefer TOML surfaces + `ui://…` for agent chrome. Native egui owns freeform window manager concerns (hit-test, z-order, drag/resize) until graduated into surface primitives.

### 7. Local/remote isomorphism (schema-level)

Remote agent hosts present the **same workspace scene schema**. File/screenshot drop uses existing transfer paths with security gates (ADR-057 / ADR-072 spirit). Exact remote product is later; the schema must not assume “local only.”

### 8. Memory UX stays auditable

Cross-agent memory is substrate/ECC-backed and **inspectable** (paths, chips, chain). Opaque “shared brain” blobs are non-goals.

### 9. Do not confuse BVH with the window manager

ADR-056 BVH is the **cognitive spatial index** over events/geometry. Agent Workspace layout is a **UI scene graph**. Separate modules and language.

## Non-goals

- Reimplementing CNVS or requiring macOS.  
- Making freeform multi-window the **only** 0.8 desktop model.  
- Auto-tiling i3 parity as a day-one requirement (optional later arrange strategies).  
- Abandoning DESIGN.md empty/offline/loading honesty for demo flash.  
- Using generative “pretty” layouts that hide substrate honesty.

## Implementation phases

| Phase | Cycle | Plane | Work | Exit |
|-------|-------|-------|------|------|
| **A** | 0.8.x | **WEFT-685** | Finish stock desktop; substrate `agents/*` inventory; no hard dependency on freeform WM | OOB desktop solid — see **Phase A exit notes** below |
| **B** | 0.9.0 | **WEFT-686** | **Window manager v1**: multi freeform windows, z-order, drag/resize, edge snap, layout persistence | ≥2 apps + 1 agent pane concurrent |
| **C** | 0.9.x | **WEFT-687** | Agent Workspace: spawn-opens-pane; arrange; attention bus (visual first) | Multi-agent wall without voice |
| **D** | 0.9–1.0 | **WEFT-688** | Wire voice + MCP → `WindowIntent` (spawn/focus/summarize/arrange) | CNVS-like conductor demo on WeftOS |
| **E** | Later | *(not filed)* | Remote workspace isomorphism; drop-to-remote; magnetic layouts | Local/remote same schema |

## Phase A exit notes (WEFT-685)

**Stock desktop OOB honesty (DESIGN.md §5 / §8):** Already met — no UI fix required for Phase A.

| Contract | Where |
|----------|--------|
| empty / loading / offline triad | `crates/clawft-gui-egui/src/apps/state.rs` (`render_if_needed`) |
| desktop chip empty hint | `crates/clawft-gui-egui/src/shell/desktop.rs` (`render_empty_hint`) |
| unit coverage | `state.rs` tests + per-app empty-state tests (processes, services, …) |
| freeform multi-window | **Not** a 0.8 dependency — Agent Workspace mode is 0.9+ (Phase B/C) |

**Agents inventory surface (bindable without freeform WM):**

| Surface | Detail |
|---------|--------|
| Substrate root | `substrate/agents/sessions` (`ontology://agent-session-list`) |
| Per-session | `substrate/agents/sessions/by-id/{session_id}` |
| RPC source | weave `agent.list` (kernel supervisor; excludes PID 0) |
| Pure API | `clawft_substrate::{inventory_from_agent_list, project_agent_session_rows, AgentSessionRow}` |
| Adapter topic | `KernelAdapter` polls `agent.list` → keyed list deltas (WEFT-416 style) |

Phase A exit criterion: OOB desktop solid + inventory listable for later UI binding. **No** freeform WM required.

## Consequences

### Positive

- Clear product target for “Jarvis / multi-agent wall” demos without rewriting the kernel.  
- Reuses voice (ADR-061/074), multi-agent bus, MCP, substrate.  
- Mode-gated scope protects 0.8 ship quality.

### Negative / costs

- Freeform WM is non-trivial in egui (focus, clipping, WASM degradation).  
- Attention noise risk if agents spam “needs human.”  
- Scope creep risk (“CNVS clone”) — mitigated by non-goals + phase gates.

## References

- https://cnvs.dev/  
- https://x.com/_MaxBlade/status/2082679537377144936  
- DESIGN.md desktop / app-window archetypes  
- `docs/plans/desktop-implementation-0.8.0.md` (snap/tiling deferred)  
- ADR-061 conversational voice; ADR-074 interim xAI voice  
