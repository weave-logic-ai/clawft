# K8: Self-Building OS GUI Layer – The Weaver Interface (Extensible Lego System + Real WeftOS Console)

**Date**: 2026-03-27  
**Panel**: GUI & Agent-Extensible User Experience  
**Status**: Planning (K8 target)  
**Vision**: A native-feeling, 3D-capable, self-improving graphical shell for WeftOS that functions as a **true modular operating system**. 

The GUI is the intelligent face of the **encapsulated project/app/website/tool** that each WeftOS installation serves. Humans primarily enjoy pleasant, agent-guided journeys through changes (interactive PR-style tours), while the entire interface is built from composable **Lego blocks** that users and agents can freely drag, snap, nest, connect, and extend.

**Core mantra**: Build Legos, let the users build the things. Agents generate new Lego pieces on demand. Users assemble custom workspaces, tours, dashboards, approval flows, or full applications. The system learns from every assembly and interaction via an RL loop that continuously improves the ECC cognitive network.

A **crucial and non-negotiable element** is a **real WeftOS Console** — a first-class, fully functional shell deeply integrated into the core. It is not a toy terminal or web-based emulator. It is the native command-line heart of WeftOS, exposed elegantly inside the GUI while remaining powerful enough to feel like a genuine OS shell.

This builds directly on your spike prototype (Tauri 2.0 wrapper + Rust → TS adapter + dynamic Next.js/React component generation) and the extensive console/ShellAdapter infrastructure already present in the kernel.

## 1. Design Philosophy

WeftOS is a **project-centric, self-building, extensible operating system** with a strong dual personality: elegant graphical Lego canvas **and** a rock-solid, real shell.

Key principles (updated):
- **Lego-First Extensibility**: Everything (including console panes) is a composable block.
- **Guided Journey Default**: Pleasant agent-narrated tours remain primary for most human sessions.
- **Real WeftOS Console**: A first-class, high-fidelity shell that feels native, supports full WeftOS commands, pipes, scripting, and deep integration with `ServiceApi`, ECC, governance, and mesh.
- **RL Learning from All Interactions**: Including console usage (commands typed, output patterns, scripting habits).
- **Standard OS Capabilities**: Embedded web browser, resource browser, terminal/console, system tray — making the GUI a complete OS environment.
- **Elegant Text Foundation**: Clean monospace text is the spiritual core, whether in guided tours, code blocks, or the console itself.
- **Governed & Self-Building**: All console commands and GUI assemblies are governed and witnessed.

The console is not an afterthought — it is one of the most important Lego blocks and a core integration point.

## 2. Architecture: Tauri 2.0 + Lego Engine + Deep Console Integration

**Core stack**:
- **Tauri 2.0**: Native binary with multi-window, system tray, and custom titlebar.
- **Frontend**: Next.js/React + TypeScript with dynamic Lego block rendering.
- **Lego Engine**: Drag-and-drop canvas supporting nesting and data connections.
- **Console Core**: Built on the existing `ShellAdapter` and `A2ARouter`. Exposed as both a standalone real shell and as embeddable Lego blocks.

**New/Extended Commands**:
- `open_console` — spawns a new console pane or window with full WeftOS shell semantics.
- `execute_weftos_command` — routes any shell input through the real kernel ShellAdapter (with full governance, ECC witnessing, and mesh forwarding).
- `generate_console_extension` — Weaver generates new shell commands, aliases, or prompt customizations as Lego blocks.
- RL commands for capturing console interactions (`record_console_interaction`).

## 3. The Real WeftOS Console – Design & Integration

The console is implemented in two tightly integrated forms:

### 1. Primary Console Mode (Full-Screen / Dedicated Window)
- A clean, elegant, high-performance monospace terminal that feels like a real OS shell (zsh/bash-like experience but powered by WeftOS primitives).
- Features:
  - Full command history, tab completion (service names, agent PIDs, ECC nodes, governance rules, etc.).
  - Rich output rendering: inline 3D previews, service maps, ECC graph snippets, or generated components when appropriate.
  - Piping and scripting support (WeftOS-specific syntax that routes through `A2ARouter` and `ServiceApi`).
  - Deep integration: `weftos status`, `weftos spawn-agent`, `weftos query-chain`, `weftos generate-component`, `weftos assemble-lego`, etc.
  - Governance enforcement: Every command passes the dual-layer gate before execution.
  - ECC witnessing: All console actions are automatically appended to the ExoChain.
- Appearance: Minimal chrome, beautiful syntax highlighting, subtle glow on important output, support for ANSI colors and rich formatting.

### 2. Console as a Lego Block (Embeddable & Composable)
- The console is itself a first-class **Lego block** (`WeftOSConsolePane`).
- Users can:
  - Drag it onto the main Lego canvas.
  - Nest it inside guided tours or dashboards.
  - Connect its output to other blocks (e.g., pipe console output into a 3D visualizer or approval gate).
  - Have multiple console panes open simultaneously (different contexts, remote nodes via mesh, etc.).
- Weaver can generate console extensions: new commands, prompt themes, or auto-complete rules tailored to the encapsulated project.

**Console + GUI Synergy**:
- From the graphical Lego canvas you can open a console pane pre-loaded with context (e.g., "show me the last 10 chain events related to this service").
- Console commands can spawn new GUI blocks ("weftos show-ecc-graph" opens a 3D Lego block).
- RL learning observes both graphical assembly and console usage to improve future suggestions.

## 4. Standard OS Capabilities (Including Embedded Web Browser)

The GUI includes these built-in Lego blocks out of the box:
- **Embedded Web Browser Pane** (`WebBrowserPane`): Full-featured, multi-tab capable browser using Tauri’s native WebView. Supports live previews of the encapsulated website, external docs, or third-party tools. Can be dragged, resized, and connected to other blocks (e.g., browser output feeds a monitoring dashboard).
- **ResourceTree Browser**: File-system-like explorer for the project assets and ResourceTree.
- **System Tray & Notifications**: Background agent alerts with one-click actions.
- **Multi-Window Support**: Any Lego assembly or console pane can be detached into its own native window.

These standard capabilities make the GUI feel like a complete OS shell rather than just a development tool.

## 5. Guided Journey, RL Learning & Extensibility

- **Guided Journey Mode**: Agent-narrated tours built from Lego blocks, including optional embedded console or browser panes for deeper investigation.
- **Problem Explorer & Approval Gates**: Canvas-based, with console blocks available for quick debugging.
- **RL Loop**: Captures Lego assembly, console commands typed, browser navigation, and explicit feedback. Improves both graphical generation and console behavior (e.g., better auto-complete, preferred output formatting).

**Extensibility in Practice**:
- Weaver generates new Lego blocks (including new console commands or browser extensions).
- Users build custom workspaces by combining console panes, browser panes, 3D viewers, and tour steps.
- Saved assemblies become reusable project tools or new OS features.

## 6. Heterogeneous Support

- Desktop/CloudNative/Edge: Full Lego canvas, real console, embedded browser, 3D, multi-window.
- Browser/WASI: Simplified assembly view with WebSocket fallback to full console/browser capabilities on remote nodes.

## 7. Implementation Roadmap (K8)

- **K8.1**: Extend spike with `WeftOSConsolePane` Lego block and `open_console` / `execute_weftos_command` commands. Ensure full ShellAdapter integration.
- **K8.2**: Core Lego blocks including WebBrowserPane, Text Narrative, Diff Viewer, Approval Gate.
- **K8.3**: Guided Journey as composable Lego assembly + basic RL from console and assembly interactions.
- **K8.4**: Nesting, data connections between blocks (including console output piping), and adaptive block generation.
- **K8.5**: Rich console features (tab completion, rich output rendering, project-specific extensions) + 3D Lego components.
- **K8.6**: Polish, multi-window, theming, and end-to-end examples (user builds a custom workspace with console + browser + 3D ECC tour).

## 8. Why This Delivers the Vision

With a **real WeftOS Console** deeply integrated as both a standalone shell and a composable Lego block, combined with the embedded web browser and full Lego extensibility, the GUI becomes a genuine operating system environment. 

Users get the best of both worlds:
- Elegant guided journeys and visual Lego assembly for high-level collaboration.
- A powerful, real shell for when they want to go deep.
- The ability to build whatever they need by snapping blocks together — including custom combinations of console, browser, 3D views, and project-specific tools.

The RL loop ensures the entire system (graphical and console) continuously learns from the user. Agents generate new bricks, users build the structures, and the OS evolves with the encapsulated project.

This is WeftOS as a living, extensible, self-building operating system — where the console is not an accessory, but a core, first-class citizen that feels truly native to the kernel.

## 9. JSON-Descriptor Architecture: Multi-Target Rendering

**Key Insight (2026-03-28)**: Every Lego block is a **JSON descriptor** that can render to multiple targets from a single definition. This is validated by two major projects:

- **vercel-labs/json-render** (Apache 2.0, 13.5K stars) — Spec-centric: one JSON document = entire UI. Catalog (Zod schema constrains AI generation) + Registry (maps components to real implementations). **15+ render targets**: React, Vue, Svelte, Terminal (Ink), 3D (R3F), PDF, video, email, images, MCP apps.
- **google/a2ui** (Apache 2.0, 13.7K stars) — Protocol-centric: streaming typed messages build UI incrementally. Agent sends `createSurface` → `updateComponents` → `updateDataModel`. Transport-agnostic (A2A, MCP, WebSocket). Separate data model with JSON Pointer binding.

### WeftOS Synthesis: json-render Spec + A2UI Protocol + Kernel-Backed Actions

The Lego engine should use **json-render's spec model** (flat adjacency list, catalog + registry, `$state` bindings) with **A2UI's streaming protocol** (surface lifecycle, incremental data updates) and **WeftOS kernel commands as the action catalog**.

#### Descriptor Format (WeftOS Lego Block)

```json
{
  "root": "dashboard-1",
  "elements": {
    "dashboard-1": {
      "type": "Column",
      "children": ["cpu-gauge", "agent-list", "spawn-btn"]
    },
    "cpu-gauge": {
      "type": "Metric",
      "props": { "label": "CPU", "value": { "$state": "/kernel/metrics/cpu_percent" }, "unit": "%" }
    },
    "agent-list": {
      "type": "DataTable",
      "props": {
        "columns": ["pid", "agent_id", "state"],
        "rows": { "$state": "/kernel/processes" }
      }
    },
    "spawn-btn": {
      "type": "Button",
      "props": { "label": "Spawn Coder" },
      "on": {
        "press": {
          "action": "kernel_exec",
          "params": { "command": "weftos spawn-agent --type coder" }
        }
      }
    }
  }
}
```

#### Multi-Target Rendering

Same descriptor renders everywhere:

| Target | Renderer | Use Case |
|--------|----------|----------|
| Web (React) | json-render `@json-render/react` or custom | Main Tauri GUI |
| Terminal | json-render `@json-render/ink` + xterm.js | WeftOS Console rich output |
| 3D | json-render `@json-render/react-three-fiber` | ECC Knowledge Graph blocks |
| Voice | Custom renderer → text → TTS | Mentra glasses audio |
| Mentra HUD | Custom renderer (400x240 constraint) | Smart glasses display |
| MCP | json-render `@json-render/mcp` | Tool outputs in Claude/ChatGPT |
| PDF | json-render `@json-render/react-pdf` | Assessment reports |
| Shell | Custom renderer → formatted text | Pure CLI mode |

#### The StateStore IS the Kernel

The json-render `StateStore` maps directly to WeftOS's `ServiceApi`:
- `/kernel/metrics/*` → KernelMetrics from health subsystem
- `/kernel/processes/*` → ProcessTable entries
- `/kernel/chain/*` → ExoChain events
- `/kernel/ecc/*` → Causal graph data
- `/kernel/governance/*` → Governance decisions

State lives in Rust. The frontend is just a projection. `updateDataModel` messages push from kernel → frontend via the existing WebSocket/Tauri command channel.

#### The Action Catalog IS the Kernel Command Set

Every button, every interaction maps to a kernel command:
- `kernel_exec` → ShellAdapter → A2ARouter → ServiceApi
- `governance_check` → GovernanceEngine gate
- `chain_query` → ExoChain query
- `ecc_search` → HNSW similarity search
- `agent_chat` → Agent conversation via A2A

Actions are governed (dual-layer gate) and witnessed (ECC/ExoChain).

#### Agent Generation

The Weaver generates **JSON descriptors, not TSX code**. The catalog constrains what the Weaver can produce (Zod validation). The registry renders it. This is:
- Safer (validated schema, not arbitrary code)
- Multi-target (one generation → all renderers)
- Composable (blocks nest and connect via `$state` references)
- Versionable (descriptor format is semver'd, renderers evolve independently)

### Implications for K8 Roadmap

- **K8.1**: Implement the descriptor format + React registry for core blocks (Console, Dashboard, Browser)
- **K8.2**: Wire `$state` to kernel WebSocket/Tauri commands. Implement `kernel_exec` action.
- **K8.3**: Add Terminal renderer (Ink) — same blocks render in console mode
- **K8.4**: Add Mentra HUD renderer (constraint-driven layout from same descriptors)
- **K8.5**: Weaver generates block descriptors, validated against catalog schema
- **K8.6**: RL loop: learn which block combinations and action sequences are useful

---

**Next actions**:
- Integrate the existing ShellAdapter deeply into the Tauri console pane with full governance and ECC witnessing.
- Implement `WeftOSConsolePane` as a draggable Lego block with rich output rendering.
- Add the embedded `WebBrowserPane` and demonstrate a guided tour that combines console + browser + 3D blocks.
- **NEW**: Define the WeftOS Lego catalog (Zod schemas for all core blocks) and implement a json-render-compatible registry.
- **NEW**: Prototype: same block descriptor renders in both the Tauri web panel AND the xterm.js console.
- **NEW**: Evaluate json-render as a dependency vs building a WeftOS-specific renderer that follows the same pattern.

This completes the foundation for a truly powerful and extensible WeftOS GUI.