# K8: Self-Building OS GUI Layer – The Weaver Interface

**Date**: 2026-03-26  
**Panel**: GUI & Agent-Extensible User Experience  
**Status**: Planning (K8 target)  
**Vision**: A native-feeling, 3D-capable, self-improving graphical shell for WeftOS that turns the distributed OS into a living, agent-buildable platform. The Weaver (and other agents) can generate, improve, and distribute TypeScript-based applications that interact deeply with the Rust kernel, ECC cognitive network, mesh, governance, and heterogeneous nodes.

## 1. Design Philosophy

WeftOS is not just a kernel for agents and services — it is a **self-aware, self-extending operating system**. The ECC (Causal Merkle Vector Graph) already ingests its own development history (chat logs, git commits, code, etc.) and uses it for self-improvement, bug fixing, and knowledge graph construction. The K8 GUI must close the loop: make the system visually inspectable, interactively manageable, and **dynamically extensible by agents themselves**.

Key principles:
- **Agent-first extensibility**: The Weaver (or any authorized agent) must be able to generate, modify, and hot-reload user-facing applications without restarting the kernel or GUI.
- **Heterogeneous awareness**: The GUI must gracefully degrade or request missing data across the mesh (cloud, edge, browser, WASI, embedded). Partial views are acceptable and encouraged — the UI knows when to trigger `ServiceApi` calls for more data.
- **Native binary feel with modern web power**: Launch like a traditional OS executable (Windows .exe, macOS .app, Linux binary) while leveraging the massive TypeScript/React ecosystem for rapid, beautiful UIs.
- **3D-first cognitive visualization**: The ECC graph, service maps, mesh topology, deployment cycles, permission graphs, resource trees, HNSW clusters, impulses, and governance rules deserve spatial, interactive 3D representation where possible.
- **Governed & secure**: Every GUI action routes through existing `ServiceApi`, dual-layer `GovernanceGate`, and chain witnessing. No bypasses.
- **Self-building**: The OS GUI itself should be introspectable and improvable by the ECC/Weaver loop. Agents can propose UI changes, new apps (e.g., a calculator, asset manager, or full custom shell), and even evolve the shell over time.

This makes WeftOS feel like OpenClass elevated: a true agentic operating system where humans and agents co-evolve the interface and applications.

## 2. Recommended Architecture: Tauri 2.0 + TypeScript/React Frontend

**Core choice**: [Tauri 2.0](https://v2.tauri.app) as the GUI host.

### Why Tauri?
- Produces tiny, secure, native-feeling binaries (few MB) using the OS's native WebView (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). Launches and feels like a traditional desktop application.
- Full GPU acceleration in the WebView enables smooth 3D rendering.
- Backend remains pure Rust — the GUI is just another client of the existing kernel abstractions (`A2ARouter`, `ServiceApi`, `MeshAdapter`, `CmvgSyncService`, etc.).
- Excellent TypeScript integration with automatic bindings.
- Feature-gated (`gui-tauri` or similar) so edge/WASI/browser builds remain lightweight.
- Supports transparent windows, custom chrome, multi-window, system tray, and deep OS integration.

**Frontend**: React + TypeScript (or any modern web stack). Weaver excels at generating TS/TSX code.

**3D Visualization**: React Three Fiber + Three.js (or `@react-three/drei`). This enables interactive 3D force-directed graphs for the ECC knowledge graph, spatial service maps, mesh topology with node connections, and immersive views of causal relationships, HNSW clusters, and deployment cycles. Performance techniques like InstancedMesh handle large graphs efficiently.

### Rust ↔ TypeScript Bridge (The Critical Adapter)
Use a combination of mature tools for zero-boilerplate type safety:
- **tauri-bindgen** or **rspc** (with Tauri adapter) — defines a shared interface (WIT or Rust traits) and auto-generates TypeScript declarations, invoke functions, and runtime validation.
- **tauri-typegen** or **ts-rs** — scans `#[tauri::command]` functions and generates strongly-typed TS interfaces for all WeftOS types (`ServiceEntry`, `ResolvedService`, `SyncStateDigest`, ECC graph nodes/edges, `PeerNode`, governance rules, EffectVector scores, etc.).
- Result: Weaver writes Rust-side wrappers once (thin `ServiceApi` → Tauri command layer). The frontend gets perfect, always-in-sync TypeScript types and `invoke('service.resolve', { name: 'cache' })` calls with full IntelliSense and type checking.

This turns the kernel into a **TypeScript app server**: agents generate full TS applications that call into the mesh, ECC, chain, governance, and sensors as if they were local services.

## 3. Integration with Existing WeftOS Primitives

The GUI is **not** a separate layer — it is a first-class client of the kernel:

- All interactions route through `A2ARouter` / `ServiceApi` (exactly like the MeshAdapter and RegistryQueryService).
- `RegistryQueryService`, `CmvgSyncService`, chain queries, tree diffs, etc., become directly callable from TypeScript.
- Mesh-aware: The UI can discover and invoke services on remote nodes (as shown in the K6 service resolution diagrams). Partial data triggers "Request from peer" flows.
- Observability: Response scores (RTT, success_rate, deny_rate, headroom, risk_delta) drive dynamic UI elements (node coloring, affinity-based recommendations, circuit-breaker visuals).
- Governance: Every command passes the dual-layer `GovernanceGate`. UI actions are witnessed to the ExoChain.
- Cognitive sync: Real-time updates via multiplexed QUIC streams (IPC + CMVG sync streams) feed live 3D graphs and dashboards.
- Self-awareness: The ECC graph includes the GUI codebase itself. Weaver can analyze, propose improvements, generate new components, and apply them.

**Dynamic app loading**:
- Weaver-generated TS bundles can be loaded at runtime (via Tauri asset handling or dynamic import patterns).
- Distributable "apps" (calculator, asset manager, custom shells, sensor dashboards) become plugins or separate windows.
- Hot-reload during development; production apps can be signed and governed via the existing permissioning system.

## 4. Core GUI Capabilities & Visualizations

The default WeftOS shell should feel like a modern, spatial operating system:

- **3D ECC / Knowledge Graph Explorer**: Spatial navigation of the causal Merkle vector graph. Nodes as 3D objects with force-directed layout, edge types (causal, crossref, impulse), timestamps (HLC), and drill-down to chain events or resource tree subgraphs. Weaver can generate new layouts or filters.
- **Service & Mesh Topology Map**: 2D/3D graph of services, nodes (CloudNative, Edge, Browser, etc.), connections, and real-time status (using NodeEccCapability and scores).
- **Deployment & Task Cycles**: Timeline views + graphs showing governance decisions, chain events, agent actions, and mesh propagations.
- **Asset & Resource Management**: Browsable views of the ResourceTree, with permissioning overlays and governance rule inspection.
- **Agent & Sensor Dashboard**: Live views of running agents, impulses, HNSW queries, and sensor feeds. Partial data handling shows "available on remote node X" with one-click resolution.
- **Governance Console**: Visual rule editor, genesis root verification, supersede flows (with on-chain anchoring support).
- **App Marketplace / Builder**: Interface for Weaver to create, preview, and distribute new TS apps. Users/agents can install governed applications.
- **Classic OS elements**: File-like browser for the ResourceTree, terminal integration (via existing ShellAdapter), system tray for background monitoring.

All views are mesh-aware and partial-data resilient. The UI knows the shape of missing data and can construct forms or service calls to fetch it.

## 5. Self-Building & Agent Extensibility

This is the core differentiator:

- The entire GUI (Rust commands + TS components) lives in the ECC graph.
- Weaver analyzes the codebase, identifies improvements, generates new TSX components or full apps, and proposes them via governance.
- Runtime loading of agent-generated bundles turns WeftOS into a dynamic app platform.
- Example: Weaver creates a "Calculator" app → distributes it as a governed TS bundle → users launch it as a window or embed it.
- Future evolution: Agents can redesign the shell, create specialized 3D workspaces for different roles (developer view, operator view, auditor view), or integrate new sensor visualizations.

The OS literally builds and customizes its own interface over time.

## 6. Heterogeneous & Platform Support

- **Desktop / CloudNative / Edge**: Full Tauri binary with 3D acceleration.
- **Browser / WASI light nodes**: Pure web fallback (same TS codebase, WebSocket to full nodes).
- **Embedded / low-resource**: Optional minimal mode (reduced 3D, 2D-only) or remote GUI via mesh.
- Feature flags ensure the core kernel never depends on GUI crates.

## 7. Implementation Roadmap (K8)

- **K8.1**: Tauri 2.0 scaffolding + basic Rust ↔ TS bindings (ServiceApi exposure, RegistryQueryService, etc.).
- **K8.2**: Core dashboards (service map, mesh topology, chain viewer) with 2D graphs.
- **K8.3**: 3D ECC visualization with React Three Fiber; score-driven dynamic styling.
- **K8.4**: Dynamic app loading + Weaver integration for generating TS apps.
- **K8.5**: Governance integration, partial-data handling, and self-introspection features.
- **K8.6**: Polish (native feel, multi-window, theming, distributable apps) + testing across platforms.

Add under a new `gui/` or `weftos-gui/` crate/directory (monorepo style with shared types).

## 8. Risks & Mitigations

- Binary size: Tauri is already tiny; use feature flags aggressively.
- 3D performance on edge: Fallback to 2D or remote rendering.
- Binding maintenance: Auto-generation tools (tauri-bindgen, rspc, tauri-typegen) keep TS in sync.
- Security: All calls gated; WebView sandbox + Tauri security model.

## 9. Why This Realizes the Vision

This GUI transforms WeftOS from a powerful but invisible distributed kernel into a **visible, interactive, self-evolving operating system**. Agents and humans can explore the cognitive network in 3D, manage assets and permissions visually, and — most importantly — have the Weaver continuously improve both the shell and the applications running on it.

It leverages the strengths of the existing architecture (ServiceApi unification, mesh resolution diagrams, CMVG sync, governance, dual PQ, scores) while opening the door to the massive TS ecosystem for rapid, modern UI development.

The result is a living OS where the boundary between kernel, agents, GUI, and applications blurs — exactly the self-building, agentic platform WeftOS was designed to become.

**Next actions for K8 planning**:
- Prototype Tauri scaffolding in the repo.
- Define the initial set of exposed commands and generate TS bindings.
- Build the first 3D ECC viewer as a proof-of-concept for Weaver self-improvement.

This completes the bridge from the low-level mesh/cognitive layers (K6/K7) to a fully human-and-agent-accessible operating system experience.