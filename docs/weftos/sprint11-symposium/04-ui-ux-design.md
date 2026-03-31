# Sprint 11 Symposium -- Track 4: UI/UX Design Summit -- The Weaver Interface (Lego OS Shell)

**Chair**: system-architect
**Panelists**: system-architect, kernel-architect, mesh-engineer, mentra-ui, mentra-ux,
doc-weaver, ecc-analyst, weaver, mobile-dev, governance-counsel
**Date**: 2026-03-27
**Branch**: `feature/weftos-kernel-sprint`
**Duration**: 75 minutes (flagship track)

**Key references**:
- `.planning/sparc/weftos/0.1/weftos-gui-design-notes.md` (Lego philosophy + JSON descriptor architecture)
- `docs/weftos/weftos-gui-specifications.md` (original K8 spec)
- `gui/` (Tauri 2.0 + React prototype, 4 views)
- `gui/src-tauri/src/lib.rs` (7 Tauri commands, all stubbed with TODOs)
- Track 1 (01-pattern-extraction.md): 15 registries, `Registry` trait proposal
- Track 3 (03-release-engineering.md): cargo-dist, publish gates, WASM target mismatch
- Track 9 (09-optimization-plan.md): HNSW hotspots, tick loop, `opt-level = "z"` issue

---

## Session 1: The Real WeftOS Console (15 min)

### 1.1 Console-to-Kernel Connection Model

The kernel exposes `ServiceApi` (trait in `service.rs:431`) with three methods: `call(service, method, params)`, `list_services()`, and `health(service)`. The `ShellAdapter` (service.rs:462) wraps `ServiceApi` and parses `"service.method args"` strings into `ServiceApi::call` invocations.

The GUI prototype currently uses a mock WebSocket hook (`useKernelWs.ts`) that generates fake data on a 2-second timer. The Tauri backend has 7 stubbed commands (all return hardcoded values with `// TODO: Wire to...` comments).

**Architecture decision: Dual-channel model.**

```
                           Tauri IPC (invoke)
  React Frontend  <=============================>  Rust Backend
                           WebSocket (push)
                   <-----------------------------
```

| Channel | Direction | Purpose | Latency |
|---------|-----------|---------|---------|
| Tauri `invoke()` | Frontend -> Backend -> Frontend | Command execution (request/response) | <5ms local IPC |
| WebSocket (Tauri event or raw WS) | Backend -> Frontend | Real-time state push (metrics, chain events, process changes) | Continuous, ~100ms batching |

**Why not WebSocket-only?** Tauri's `invoke()` gives type-safe Rust function calls with automatic serde. It is the natural channel for commands. WebSocket is better for push-based streaming of kernel snapshots. Using both avoids mixing request-response semantics with pub-sub semantics on a single channel.

**Why not Tauri events only?** Tauri events (`emit`/`listen`) work well for push but lack the request/response correlation that `invoke()` provides natively. Use Tauri events as the push channel (they are more ergonomic than raw WebSocket within Tauri).

**Implementation**:

1. **Commands** route through `invoke()`:
   - `invoke('execute_command', { command: 'kernel.status' })` calls `ShellAdapter::execute("kernel.status")` on the Rust side.
   - The Rust handler acquires a reference to the kernel's `ShellAdapter` (stored in Tauri's managed state) and awaits the result.

2. **State push** routes through Tauri events:
   - The Rust backend spawns a background task that polls the kernel every 500ms (or subscribes to a kernel event bus when available).
   - It emits `kernel://snapshot` events containing `KernelSnapshot` payloads.
   - The frontend's `useKernelWs` hook is refactored to `useKernelState` and listens to Tauri events instead of a mock timer.

3. **New Tauri commands** (extending the existing 7):

| # | Command | Signature | Backend Wiring |
|---|---------|-----------|----------------|
| 1 | `kernel_status` | `() -> CmdResponse<KernelStatus>` | **EXISTS** -- wire to `ServiceApi::health("kernel")` |
| 2 | `spawn_agent` | `(SpawnAgentArgs) -> CmdResponse<SpawnResult>` | **EXISTS** -- wire to `Supervisor::spawn()` |
| 3 | `stop_agent` | `(pid: u32) -> CmdResponse<bool>` | **EXISTS** -- wire to `Supervisor::stop()` |
| 4 | `set_config` | `(SetConfigArgs) -> CmdResponse<bool>` | **EXISTS** -- wire to ConfigService |
| 5 | `query_chain` | `(from_seq, limit) -> CmdResponse<Vec<ChainEvent>>` | **EXISTS** -- wire to ExoChain |
| 6 | `register_service` | `(RegisterServiceArgs) -> CmdResponse<bool>` | **EXISTS** -- wire to ServiceRegistry |
| 7 | `generate_component` | `(GenerateComponentArgs) -> CmdResponse<GeneratedComponent>` | **EXISTS** -- wire to Weaver |
| 8 | `execute_command` | `(command: String) -> CmdResponse<Value>` | **NEW** -- delegates to `ShellAdapter::execute()` |
| 9 | `list_services` | `() -> CmdResponse<Vec<ServiceInfo>>` | **NEW** -- `ServiceApi::list_services()` |
| 10 | `query_ecc` | `(query: String, top_k: u32) -> CmdResponse<Vec<EccResult>>` | **NEW** -- HNSW search via `HnswService` |
| 11 | `governance_check` | `(GovernanceCheckArgs) -> CmdResponse<GovernanceDecision>` | **NEW** -- `GovernanceEngine::evaluate()` |
| 12 | `get_process_table` | `() -> CmdResponse<Vec<ProcessEntry>>` | **NEW** -- `ProcessTable::list()` |
| 13 | `get_causal_graph` | `(depth: u32) -> CmdResponse<GraphData>` | **NEW** -- `CausalGraph::subgraph()` |

### 1.2 Console Command Catalog

At launch, the console exposes commands organized by service namespace. These map directly to `ServiceApi::call(service, method, params)` via the `ShellAdapter` parser.

```
NAMESPACE       COMMAND                     DESCRIPTION
-----------     -------------------------   ----------------------------------------
kernel          kernel.status               Show kernel version, uptime, health
kernel          kernel.metrics              Show CPU, memory, tick count, peers
kernel          kernel.shutdown             Graceful shutdown

process         process.list                List all processes with PID, state
process         process.spawn <type> [name] Spawn new agent process
process         process.stop <pid>          Stop agent by PID
process         process.info <pid>          Detailed process info + capabilities

service         service.list                List all registered services
service         service.health [name]       Health of one or all services
service         service.register <name>     Register a new service

chain           chain.query [--from N] [--limit N]   Query ExoChain events
chain           chain.height                Current chain height
chain           chain.verify [seq]          Verify chain integrity from sequence

ecc             ecc.search <query>          Semantic search via HNSW
ecc             ecc.graph [--depth N]       Dump causal graph (subgraph)
ecc             ecc.tick-stats              DEMOCRITUS tick loop statistics

governance      governance.check <action>   Run governance gate on hypothetical action
governance      governance.rules            List active governance rules
governance      governance.history          Recent governance decisions

mesh            mesh.peers                  List mesh peers
mesh            mesh.topology               Show mesh topology
mesh            mesh.resolve <service>      Resolve service across mesh

config          config.get <key>            Get configuration value
config          config.set <key> <value>    Set configuration value
config          config.list [namespace]     List config entries

weaver          weaver.generate <desc>      Generate a block descriptor
weaver          weaver.session              Current Weaver session info

wasm            wasm.tools                  List registered WASM tools
wasm            wasm.exec <tool> <input>    Execute a WASM tool
```

### 1.3 Tab Completion Data Sources

The console supports tab completion by querying live kernel state. Completion sources:

| Prefix / Context | Data Source | Example Completions |
|------------------|-------------|---------------------|
| First token (namespace) | Hard-coded command catalog | `kernel`, `process`, `service`, `chain`, ... |
| Second token (method) | Per-namespace method list | `process.` -> `list`, `spawn`, `stop`, `info` |
| After `process.stop ` | `ProcessTable::list()` PIDs | `1`, `2`, `3`, ... |
| After `process.spawn ` | Agent type registry | `coder`, `reviewer`, `tester`, `planner`, ... |
| After `service.health ` | `ServiceRegistry::list()` names | `registry`, `governance`, `mesh-sync`, ... |
| After `ecc.search ` | No completion (free text) | -- |
| After `governance.check ` | Action prefix catalog | `tool.exec`, `ipc.send`, `service.register`, ... |
| After `config.get ` / `config.set ` | ConfigService key list | `tick_interval_ms`, `max_agents`, ... |
| After `wasm.exec ` | `ToolRegistry::list()` names | `code_search`, `file_read`, `browser_nav`, ... |

**Implementation**: A `CompletionProvider` struct on the Rust side. The console frontend sends partial input; Rust returns a `Vec<String>` of candidates. This is a new Tauri command: `complete_command(partial: String) -> Vec<String>`.

### 1.4 Rich Output: Inline Rendering

When a command produces structured output (not plain text), the console renders it inline using the block descriptor system (Section 2). This is the key integration point between console and Lego engine.

**Protocol**: Every command response includes a `display_hint` field:

```rust
#[derive(Serialize)]
pub struct ShellResult {
    /// Plain text output (always present, always renderable)
    pub text: String,
    /// Optional block descriptor for rich rendering
    pub block: Option<serde_json::Value>,
    /// Display hint
    pub display: DisplayHint,
}

#[derive(Serialize)]
pub enum DisplayHint {
    /// Render as plain monospace text
    Text,
    /// Render as a table (text fallback: ASCII table)
    Table,
    /// Render as an inline block descriptor (rich output)
    Block,
    /// Open a new Lego block pane (detached from console)
    SpawnBlock,
}
```

**Example: `weftos show-ecc-graph`**

1. User types `ecc.graph --depth 3` in the console.
2. ShellAdapter routes to `ecc` service, `graph` method, params `{"depth": 3}`.
3. Kernel returns `ShellResult` with:
   - `text`: ASCII adjacency list (always works)
   - `block`: A JSON block descriptor of type `"CausalGraph"` with the graph data
   - `display`: `DisplayHint::SpawnBlock`
4. The console sees `SpawnBlock` and creates a new Lego block pane adjacent to the console. The block descriptor renders as an interactive force-directed graph (via the `CausalGraph` block component).
5. If the console is in a constrained context (e.g., Mentra HUD or pure terminal), the `text` fallback is used.

### 1.5 Governance Enforcement

Every command passes the governance gate before execution. This is wired at the `ShellAdapter` level, not in each individual command handler.

```
User input
  |
  v
ShellAdapter::execute(command)
  |
  +-- Parse command into (service, method, params)
  |
  +-- GateBackend::check(caller_id, "shell.{service}.{method}", context)
  |     |
  |     +-- CapabilityGate: Does this agent/user have shell access?
  |     +-- TileZeroGate (if enabled): Three-way decision with receipts
  |     |
  |     +-- Deny  -->  Return error with denial reason
  |     +-- Defer -->  Return "awaiting human approval" status
  |     +-- Permit --> Continue
  |
  +-- GovernanceEngine::evaluate(GovernanceRequest)
  |     |
  |     +-- Compute EffectVector for the action
  |     +-- Check risk_threshold
  |     +-- Deny / EscalateToHuman / PermitWithWarning / Permit
  |
  +-- ServiceApi::call(service, method, params)
  |
  v
Return result (with governance metadata in response)
```

The console shows governance status for each command:

```
weftos> process.spawn coder
[PERMIT] governance: risk=0.1, security=0.2 (below threshold)
Spawned PID 42: coder-42

weftos> config.set max_agents 100
[WARN] governance: risk=0.4, security=0.3 (permit-with-warning)
! Increasing max_agents may impact system stability
Config set: max_agents = 100

weftos> mesh.resolve sensitive-data-service
[DENY] governance: privacy=0.9, risk=0.7 (above threshold)
Access denied: privacy threshold exceeded. Escalate with --force-human-review.
```

### 1.6 ECC Witnessing: Event Format

Every console command execution is logged to the ExoChain as a `ChainEvent`. The event format:

```json
{
  "sequence": 1042,
  "chain_id": 0,
  "timestamp": "2026-03-27T14:23:01.003Z",
  "prev_hash": "a1b2c3...",
  "hash": "d4e5f6...",
  "payload_hash": "789abc...",
  "source": "gui.console",
  "kind": "shell.exec",
  "payload": {
    "command": "process.spawn coder",
    "caller": "user:primary",
    "governance_decision": "Permit",
    "effect_vector": { "risk": 0.1, "fairness": 0.0, "privacy": 0.0, "novelty": 0.2, "security": 0.2 },
    "result_summary": "pid=42",
    "duration_ms": 12,
    "session_id": "sess-001"
  }
}
```

This uses the existing `ChainManager::append()` pattern. The `source` is `"gui.console"` to distinguish console-originated events from programmatic calls. The `kind` is `"shell.exec"` for executed commands, `"shell.deny"` for governance-denied commands, and `"shell.error"` for runtime errors.

---

## Session 2: The Lego Block Engine (20 min)

### 2.1 Block Descriptor Schema (Complete)

This finalizes the JSON descriptor format from the design notes (Section 9). A block descriptor is a self-contained JSON document that defines a UI assembly.

```typescript
/** Root descriptor: defines an entire block assembly */
interface BlockDescriptor {
  /** Schema version for forward compatibility */
  version: "0.2.0";

  /** ID of the root element in the elements map */
  root: string;

  /** Flat map of element ID -> element definition */
  elements: Record<string, BlockElement>;

  /** Optional metadata (creator, timestamp, tags) */
  meta?: BlockMeta;
}

interface BlockMeta {
  /** Who created this descriptor (user, agent, system) */
  creator: string;
  /** ISO 8601 timestamp */
  created_at: string;
  /** Human-readable title */
  title?: string;
  /** Tags for catalog search */
  tags?: string[];
  /** Governance approval chain event sequence number */
  governance_seq?: number;
}

interface BlockElement {
  /** Block type -- must exist in the Block Catalog */
  type: string;

  /** Child element IDs (ordering matters for layout) */
  children?: string[];

  /** Props specific to this block type (validated by Zod schema) */
  props?: Record<string, PropValue>;

  /** Event handlers -- map event names to actions */
  on?: Record<string, BlockAction>;

  /** Data port bindings for inter-block communication */
  ports?: Record<string, PortBinding>;

  /** Layout hints (width, height, position for drag-and-drop) */
  layout?: LayoutHints;
}

/** A prop value can be a literal or a $state reference */
type PropValue =
  | string
  | number
  | boolean
  | null
  | PropValue[]
  | { [key: string]: PropValue }
  | StateRef;

/** Reference to kernel state -- resolved at render time */
interface StateRef {
  $state: string;          // JSON Pointer into the StateStore (e.g. "/kernel/metrics/cpu_percent")
  $default?: PropValue;    // Fallback if path is not yet available
  $transform?: string;     // Optional transform function name (e.g. "percent", "humanBytes")
}

interface BlockAction {
  /** Action type from the Action Catalog */
  action: string;
  /** Static params merged with event data */
  params?: Record<string, PropValue>;
  /** Require governance check before execution */
  governed?: boolean;
}

interface PortBinding {
  /** Direction: "in" (receives data), "out" (emits data) */
  direction: "in" | "out";
  /** Data type hint for validation */
  data_type: string;
  /** For "in" ports: source element ID and port name */
  source?: { element: string; port: string };
  /** For "out" ports: $state path where data is published */
  publish?: string;
}

interface LayoutHints {
  /** Width in grid units (1-12) */
  width?: number;
  /** Height in grid units */
  height?: number;
  /** Absolute position (for canvas mode) */
  x?: number;
  y?: number;
  /** Minimum dimensions */
  minWidth?: number;
  minHeight?: number;
  /** Whether this block is resizable */
  resizable?: boolean;
}
```

### 2.2 Block Catalog

Every built-in block type has a Zod schema that constrains what props it accepts, what ports it exposes, and what actions it supports. The Weaver generates block descriptors validated against these schemas.

**The catalog is validated against what the kernel actually exposes** (cross-referenced with Track 1's 15 registries, the 7 Tauri commands, and the ServiceApi surface).

#### Layout Blocks (4)

| # | Type | Description | Props | Ports | Children? |
|---|------|-------------|-------|-------|-----------|
| 1 | `Column` | Vertical stack of children | `gap?: number` | -- | Yes |
| 2 | `Row` | Horizontal layout of children | `gap?: number`, `wrap?: boolean` | -- | Yes |
| 3 | `Grid` | CSS grid layout | `columns: number`, `gap?: number` | -- | Yes |
| 4 | `Tabs` | Tabbed container | `labels: string[]`, `activeTab?: number` | -- | Yes (one child per tab) |

#### Data Display Blocks (6)

| # | Type | Description | Props | Ports |
|---|------|-------------|-------|-------|
| 5 | `Metric` | Single value gauge | `label: string`, `value: StateRef`, `unit?: string`, `threshold?: {warn: number, crit: number}` | out: `value` |
| 6 | `DataTable` | Tabular data display | `columns: {key: string, label: string}[]`, `rows: StateRef`, `sortable?: boolean` | out: `selectedRow`, in: `filter` |
| 7 | `ChainViewer` | ExoChain event log | `fromSeq?: number`, `limit?: number`, `filter?: string` | out: `selectedEvent` |
| 8 | `CausalGraph` | Force-directed ECC graph | `depth?: number`, `query?: string`, `layout?: "force"\|"tree"\|"radial"` | out: `selectedNode`, in: `highlight` |
| 9 | `DiffViewer` | Side-by-side diff display | `left: StateRef`, `right: StateRef`, `language?: string` | out: `selectedHunk` |
| 10 | `CodeEditor` | Source code editor | `value: StateRef`, `language: string`, `readOnly?: boolean` | out: `content`, out: `cursor` |

#### Interactive Blocks (5)

| # | Type | Description | Props | Ports |
|---|------|-------------|-------|-------|
| 11 | `Button` | Action trigger | `label: string`, `variant?: "primary"\|"secondary"\|"danger"`, `disabled?: boolean` | -- |
| 12 | `ConsolePan` | WeftOS terminal emulator | `initialCommand?: string`, `contextService?: string` | out: `lastOutput`, in: `inject` |
| 13 | `ApprovalGate` | Governance approval widget | `action: string`, `effect_vector: StateRef` | out: `decision` |
| 14 | `TextInput` | Form input field | `label: string`, `placeholder?: string`, `value?: StateRef` | out: `value` |
| 15 | `Markdown` | Rich text / narrative display | `content: string\|StateRef`, `allowHtml?: boolean` | -- |

#### OS Capability Blocks (3)

| # | Type | Description | Props | Ports |
|---|------|-------------|-------|-------|
| 16 | `WebBrowser` | Embedded WebView | `url: string`, `allowNavigation?: boolean` | out: `currentUrl` |
| 17 | `ResourceTree` | File/asset browser | `rootPath?: string`, `filter?: string` | out: `selectedPath` |
| 18 | `ServiceMap` | 2D/3D mesh topology | `layout?: "force"\|"hierarchical"`, `showScores?: boolean` | out: `selectedService` |

**Total: 18 blocks** (3 more than the design notes' 15 -- added `Tabs`, `TextInput`, `Markdown` which are necessary for journey mode and forms).

#### Zod Schema Example (Metric Block)

```typescript
import { z } from 'zod';

const StateRefSchema = z.object({
  $state: z.string(),
  $default: z.unknown().optional(),
  $transform: z.string().optional(),
});

const PropValueSchema: z.ZodType<PropValue> = z.lazy(() =>
  z.union([
    z.string(),
    z.number(),
    z.boolean(),
    z.null(),
    z.array(PropValueSchema),
    z.record(PropValueSchema),
    StateRefSchema,
  ])
);

export const MetricBlockSchema = z.object({
  type: z.literal('Metric'),
  props: z.object({
    label: z.string(),
    value: z.union([z.number(), StateRefSchema]),
    unit: z.string().optional(),
    threshold: z.object({
      warn: z.number(),
      crit: z.number(),
    }).optional(),
  }),
  layout: z.object({
    width: z.number().min(1).max(12).optional(),
    height: z.number().optional(),
  }).optional(),
});
```

### 2.3 Block Registry

The Block Registry maps block types to React component implementations. One registry entry per block.

```typescript
// gui/src/blocks/registry.ts

import type { ComponentType } from 'react';
import type { BlockElement } from './schema';

/** Props that every block component receives */
export interface BlockRenderProps {
  /** The element definition from the descriptor */
  element: BlockElement;
  /** Resolved prop values (StateRefs already resolved to current values) */
  resolvedProps: Record<string, unknown>;
  /** Children rendered by the engine (for container blocks) */
  children?: React.ReactNode;
  /** Callback to dispatch an action */
  dispatch: (action: string, params: Record<string, unknown>) => void;
  /** Port data interface */
  ports: PortInterface;
}

export interface PortInterface {
  /** Read the current value of a named port */
  read: (portName: string) => unknown;
  /** Write a value to a named output port */
  write: (portName: string, value: unknown) => void;
  /** Subscribe to changes on a named port */
  subscribe: (portName: string, callback: (value: unknown) => void) => () => void;
}

/** Registry entry */
interface BlockRegistration {
  component: ComponentType<BlockRenderProps>;
  schema: z.ZodType;
  /** Render targets this block supports */
  targets: ('react' | 'terminal' | 'mentra' | 'pdf')[];
}

const registry = new Map<string, BlockRegistration>();

export function registerBlock(type: string, reg: BlockRegistration): void {
  registry.set(type, reg);
}

export function getBlock(type: string): BlockRegistration | undefined {
  return registry.get(type);
}

export function listBlocks(): string[] {
  return [...registry.keys()];
}
```

**Registration** (one file per block, tree-shakeable):

```typescript
// gui/src/blocks/metric/index.ts
import { MetricBlock } from './MetricBlock';
import { MetricBlockSchema } from './schema';
import { registerBlock } from '../registry';

registerBlock('Metric', {
  component: MetricBlock,
  schema: MetricBlockSchema,
  targets: ['react', 'terminal', 'mentra'],
});
```

### 2.4 Data Connection Protocol

Blocks communicate through three mechanisms, chosen by context:

#### 1. StateStore (Kernel-backed, read-only from frontend)

The primary data flow for kernel data. `$state` references in block descriptors resolve against a Zustand store that mirrors kernel state.

```typescript
// gui/src/state/kernelStore.ts
import { create } from 'zustand';

interface KernelState {
  metrics: KernelMetrics | null;
  processes: ProcessEntry[];
  events: ChainEvent[];
  services: ServiceEntry[];
  health: HealthStatus;
  // Dynamic paths for $state resolution
  [path: string]: unknown;
}

export const useKernelStore = create<KernelState>((set) => ({
  metrics: null,
  processes: [],
  events: [],
  services: [],
  health: 'healthy',
}));
```

The Tauri event listener pushes updates: `listen('kernel://snapshot', (event) => useKernelStore.setState(flatten(event.payload)))`.

The `$state` reference `/kernel/metrics/cpu_percent` resolves by walking the path in the store.

#### 2. Port Bus (Block-to-block, runtime)

For direct block-to-block communication within an assembly. Implemented as a lightweight event bus scoped to the descriptor instance.

```typescript
// gui/src/blocks/portBus.ts
type Listener = (value: unknown) => void;

export class PortBus {
  private channels = new Map<string, { value: unknown; listeners: Set<Listener> }>();

  /** Write a value to a channel (element:port key) */
  emit(channelKey: string, value: unknown): void {
    let ch = this.channels.get(channelKey);
    if (!ch) {
      ch = { value: undefined, listeners: new Set() };
      this.channels.set(channelKey, ch);
    }
    ch.value = value;
    ch.listeners.forEach((fn) => fn(value));
  }

  /** Read the last value on a channel */
  read(channelKey: string): unknown {
    return this.channels.get(channelKey)?.value;
  }

  /** Subscribe to changes */
  subscribe(channelKey: string, fn: Listener): () => void {
    let ch = this.channels.get(channelKey);
    if (!ch) {
      ch = { value: undefined, listeners: new Set() };
      this.channels.set(channelKey, ch);
    }
    ch.listeners.add(fn);
    return () => ch!.listeners.delete(fn);
  }
}
```

Port wiring is declared in the descriptor's `ports` field. The block engine resolves `source: { element: "cpu-gauge", port: "value" }` to the channel key `"cpu-gauge:value"` and subscribes the target block.

#### 3. Action Dispatch (Block-to-kernel, write)

When a block triggers an action (e.g., a button press), the action is dispatched through the Action Catalog (Section 2.5 below). The result may update the StateStore (via a new kernel snapshot) or return data directly to the block.

**Data flow summary**:

```
Kernel (Rust)  --[Tauri events]--> StateStore (Zustand) --[$state refs]--> Blocks (read)
Blocks (write) --[Action Dispatch]--> Tauri invoke --[ShellAdapter]--> Kernel
Blocks <--[PortBus]--> Blocks  (lateral, within assembly)
```

### 2.5 Action Catalog

Every action a block can trigger maps to a kernel operation. Actions are governed and witnessed.

| Action | Kernel Route | Governed? | Chain-logged? |
|--------|-------------|-----------|---------------|
| `kernel_exec` | `ShellAdapter::execute(command)` | Yes (dual-layer) | Yes (`shell.exec`) |
| `governance_check` | `GovernanceEngine::evaluate(request)` | No (it IS governance) | Yes (`governance.check`) |
| `chain_query` | `ChainManager::query(from, limit)` | Yes (read gate) | No (read-only) |
| `ecc_search` | `HnswService::search(query, top_k)` | Yes (read gate) | No (read-only) |
| `agent_chat` | `A2ARouter::send(pid, message)` | Yes (ipc gate) | Yes (`ipc.send`) |
| `navigate` | Frontend-only: change active view/tab | No | No |
| `open_block` | Frontend-only: spawn a new block pane | No | Yes (`gui.block.open`) |
| `close_block` | Frontend-only: remove a block pane | No | No |

### 2.6 Nesting Model

Blocks can contain other blocks. This is how complex assemblies are built.

**Rules**:
1. Only layout blocks (`Column`, `Row`, `Grid`, `Tabs`) accept `children`.
2. Nesting depth is limited to **6 levels** (prevents pathological recursion in agent-generated descriptors).
3. A child block cannot reference its ancestor's ports (prevents circular data flow).
4. Any block can be the root of its own descriptor (composability -- a saved assembly becomes a reusable block).

**Implementation**: The block engine recursively renders children. Each nesting level creates a new React context scope for port resolution.

```
Dashboard (Column)
  +-- Row
  |     +-- Metric (CPU)
  |     +-- Metric (Memory)
  |     +-- Metric (Chain Height)
  +-- Tabs
  |     +-- Tab 0: DataTable (Processes)
  |     +-- Tab 1: ChainViewer
  |     +-- Tab 2: ConsolePan
  +-- CausalGraph
```

### 2.7 Serialization

Block assemblies are persisted through three tiers:

| Tier | Storage | Use Case | Format |
|------|---------|----------|--------|
| **Runtime** | Zustand store (in-memory) | Current session state | JS objects |
| **Local** | IndexedDB via `idb-keyval` | User workspace persistence across sessions | JSON string |
| **Kernel** | ExoChain event + ExoResourceTree | Shared/governed assemblies, agent-generated blocks | JSON payload in chain event |

**Save flow**: User clicks "Save Workspace" -> serialize current assembly to `BlockDescriptor` JSON -> store in IndexedDB (keyed by workspace name) -> optionally commit to ExoChain via `chain.append("gui.workspace", "workspace.save", descriptor)`.

**Load flow**: On app start -> check IndexedDB for last workspace -> deserialize -> validate against block catalog schemas -> render. If validation fails (e.g., block type removed in upgrade), the engine falls back to a default dashboard.

---

## Session 3: Guided Journey Mode (10 min)

### 3.1 Journey Structure

A journey is a **directed acyclic graph (DAG) of steps**, where each step is a block descriptor. This is more flexible than a linear sequence (allows branching) but simpler than a full state machine (no loops, no complex guards).

```typescript
interface Journey {
  /** Schema version */
  version: "0.2.0";

  /** Unique journey ID */
  id: string;

  /** Journey metadata */
  meta: {
    title: string;
    description: string;
    creator: string;       // "agent:weaver-0" or "user:primary"
    created_at: string;
    source?: string;       // e.g., "git-diff:abc123..def456"
    tags?: string[];
  };

  /** Ordered list of steps */
  steps: JourneyStep[];

  /** Starting step ID */
  start: string;
}

interface JourneyStep {
  /** Unique step ID */
  id: string;

  /** Human-readable title */
  title: string;

  /** Narrative text (Markdown) explaining this step */
  narrative: string;

  /** Block descriptor for the visual content of this step */
  block: BlockDescriptor;

  /** Transitions to next steps */
  next: JourneyTransition[];

  /** Whether the user can break out to free assembly from this step */
  allow_breakout?: boolean;
}

interface JourneyTransition {
  /** Target step ID */
  target: string;
  /** Label for the transition button */
  label: string;
  /** Optional condition: only show this transition if true */
  condition?: {
    /** $state path to check */
    $state: string;
    /** Comparison operator */
    op: "eq" | "neq" | "gt" | "lt" | "exists";
    /** Value to compare against */
    value?: unknown;
  };
}
```

### 3.2 Agent Journey Generation (from Git Diff)

The Weaver generates journeys from structured inputs. The primary use case is a git diff review.

**Input**: A git diff (commit range or PR).
**Process**:

1. Weaver receives the diff via `weaver.generate-journey --source git-diff:abc123..def456`.
2. Weaver parses the diff into semantic chunks: files changed, functions modified, tests added, config changes.
3. For each semantic chunk, Weaver generates a `JourneyStep` with:
   - A narrative explaining the change in context
   - A `DiffViewer` block showing the relevant hunks
   - A `CodeEditor` block for the affected file (read-only)
   - Optionally a `CausalGraph` block showing the ECC nodes related to this change
4. Weaver orders steps by dependency (if change B depends on change A, A comes first).
5. Weaver validates all generated block descriptors against the catalog schemas.
6. The journey is returned as a `Journey` JSON document.

### 3.3 User Interaction

The journey UI renders as a focused mode within the Lego canvas:

- **Step indicator**: A progress bar or breadcrumb showing current position in the DAG.
- **Narrative panel**: Markdown rendered in a `Markdown` block at the top of each step.
- **Content area**: The step's `BlockDescriptor` rendered below the narrative.
- **Navigation**: "Next" / "Previous" buttons, plus any conditional transitions as labeled buttons.
- **Breakout**: If `allow_breakout` is true, a "Free Mode" button detaches the current block assembly from the journey and drops it onto the main canvas. The user can then modify it freely. A "Return to Journey" button is available to go back.

### 3.4 Example: "Sprint 10 Review Tour"

```json
{
  "version": "0.2.0",
  "id": "journey-sprint10-review",
  "meta": {
    "title": "Sprint 10 Review Tour",
    "description": "Walk through the major deliverables of Sprint 10: tool signing and WASM shell execution.",
    "creator": "agent:weaver-0",
    "created_at": "2026-03-27T10:00:00Z",
    "source": "git-diff:5d6c99c..1707da4",
    "tags": ["sprint-review", "sprint-10"]
  },
  "start": "step-overview",
  "steps": [
    {
      "id": "step-overview",
      "title": "Sprint 10 Overview",
      "narrative": "Sprint 10 delivered two major kernel features: **tool signing** (D9) and **WASM shell execution** (D10). This tour walks through each.",
      "block": {
        "version": "0.2.0",
        "root": "overview-layout",
        "elements": {
          "overview-layout": {
            "type": "Column",
            "children": ["metrics-row", "chain-view"]
          },
          "metrics-row": {
            "type": "Row",
            "children": ["test-count", "chain-height"]
          },
          "test-count": {
            "type": "Metric",
            "props": { "label": "Tests Added", "value": 983, "unit": "tests" }
          },
          "chain-height": {
            "type": "Metric",
            "props": { "label": "Chain Height", "value": { "$state": "/kernel/chain/height" }, "unit": "events" }
          },
          "chain-view": {
            "type": "ChainViewer",
            "props": { "limit": 10, "filter": "wasm" }
          }
        }
      },
      "next": [
        { "target": "step-tool-signing", "label": "Tool Signing (D9)" },
        { "target": "step-wasm-shell", "label": "WASM Shell (D10)" }
      ],
      "allow_breakout": true
    },
    {
      "id": "step-tool-signing",
      "title": "D9: Tool Signing",
      "narrative": "Tool signing ensures every WASM tool is cryptographically verified before execution. Ed25519 signatures are checked against a trust root anchored in the ExoChain genesis block.",
      "block": {
        "version": "0.2.0",
        "root": "signing-layout",
        "elements": {
          "signing-layout": {
            "type": "Column",
            "children": ["diff-view", "try-it"]
          },
          "diff-view": {
            "type": "DiffViewer",
            "props": {
              "left": { "$state": "/journey/diff/tool_signing/before" },
              "right": { "$state": "/journey/diff/tool_signing/after" },
              "language": "rust"
            }
          },
          "try-it": {
            "type": "ConsolePan",
            "props": { "initialCommand": "wasm.tools" }
          }
        }
      },
      "next": [
        { "target": "step-wasm-shell", "label": "Next: WASM Shell" }
      ]
    },
    {
      "id": "step-wasm-shell",
      "title": "D10: WASM Shell Execution",
      "narrative": "WASM shell execution allows running sandboxed shell commands inside the WASM runtime. Fuel-limited, memory-bounded, and fully governed.",
      "block": {
        "version": "0.2.0",
        "root": "wasm-layout",
        "elements": {
          "wasm-layout": {
            "type": "Column",
            "children": ["diff-view", "console"]
          },
          "diff-view": {
            "type": "DiffViewer",
            "props": {
              "left": { "$state": "/journey/diff/wasm_shell/before" },
              "right": { "$state": "/journey/diff/wasm_shell/after" },
              "language": "rust"
            }
          },
          "console": {
            "type": "ConsolePan",
            "props": { "initialCommand": "wasm.exec echo '{\"msg\": \"hello from sandbox\"}'" }
          }
        }
      },
      "next": [
        { "target": "step-summary", "label": "Summary" }
      ]
    },
    {
      "id": "step-summary",
      "title": "Sprint 10 Complete",
      "narrative": "Sprint 10 is complete. 14/19 exit criteria met. The kernel now has tool signing and WASM shell execution. Sprint 11 focuses on release engineering and the K8 GUI.",
      "block": {
        "version": "0.2.0",
        "root": "summary",
        "elements": {
          "summary": {
            "type": "Markdown",
            "props": { "content": "## What's Next\n\n- Sprint 11: Release v0.1.0, K8 GUI architecture\n- Sprint 12: Block engine implementation\n- Sprint 13: Journey mode + RL loop" }
          }
        }
      },
      "next": []
    }
  ]
}
```

---

## Session 4: Technology Decisions (15 min)

### TD-1: dockview as Base Layer

**Decision: NO. Do not adopt dockview.**

**Rationale**: dockview provides a panel/docking framework (think VS Code layout). The Lego engine needs a different primitive: blocks that are self-describing JSON descriptors rendered by a registry. dockview's API assumes you already have React components that you arrange into panels. The Lego engine needs to render blocks from JSON descriptors dynamically. Adding dockview would mean:
- Two layout systems (dockview panels + Lego block nesting) that must interoperate
- dockview's serialization format competing with our `BlockDescriptor` format
- An additional 50KB+ dependency for functionality we can build with CSS Grid + Zustand

**Alternative**: Use CSS Grid for top-level layout (3-column: sidebar, main, detail) and let the Lego engine handle all block arrangement within each region. For drag-and-drop, use `@dnd-kit/core` (tree-shakeable, 10KB) once we reach K8.4.

### TD-2: xterm.js for Console

**Decision: YES, adopt xterm.js for the ConsolePan block.**

**Rationale**: xterm.js is the standard terminal emulator for web (used by VS Code, Theia, JupyterLab). It provides:
- GPU-accelerated rendering via WebGL addon
- ANSI escape sequence handling (colors, cursor positioning)
- Selection, copy/paste, search
- Addon ecosystem (fit, search, web-links, image rendering via sixel)

**Rich output overlays**: When a command returns `DisplayHint::Block` or `DisplayHint::SpawnBlock`, the console pauses terminal output, inserts an HTML overlay div within the xterm viewport (xterm.js supports decoration overlays), renders the block descriptor as a React portal into that div, then resumes terminal output below it. This gives inline rich rendering (graphs, tables, metrics) without leaving the terminal context.

For `DisplayHint::SpawnBlock`, the block is detached from the console and placed as a sibling pane on the Lego canvas.

**Size budget**: xterm.js core (~45KB gzip) + WebGL addon (~15KB) = ~60KB. Acceptable for a desktop Tauri app.

### TD-3: Monaco vs CodeMirror for Code Editor Block

**Decision: CodeMirror 6.**

**Rationale**:
- Monaco (~2.5MB) is too heavy for a single block type. It brings the entire VS Code editor engine.
- CodeMirror 6 is modular (~150KB for a full-featured setup with syntax highlighting, search, fold).
- CodeMirror 6 has better mobile support (relevant for Mentra companion app).
- CodeMirror 6 is designed for embedding -- multiple instances on one page are lightweight.
- The code editor block is read-heavy (viewing diffs, browsing source). We do not need Monaco's full IntelliSense for this use case.

### TD-4: json-render Dependency vs Custom Renderer

**Decision: Custom renderer following the json-render pattern. Do NOT take a dependency on json-render.**

**Rationale**:
- json-render (Apache 2.0, 13.5K stars) validates the architectural pattern. Its catalog/registry/StateStore design is exactly right.
- However, our block types are kernel-specific (ConsolePan, CausalGraph, ApprovalGate) and require tight Tauri integration. json-render's built-in blocks (HTML primitives, generic charts) do not overlap with our needs.
- Taking the dependency would mean: wrapping every custom block in json-render's plugin API, adapting to their StateStore interface instead of using Zustand directly, and shipping their unused built-in blocks.
- **Instead**: Adopt the pattern (flat element map, catalog with Zod schemas, registry mapping types to components, `$state` references) but implement it ourselves in ~500 lines of TypeScript. This gives us full control over the rendering pipeline and Tauri integration.
- If json-render matures to the point where we can use it as a renderer target (e.g., their MCP or PDF renderers), we can adopt individual renderers as optional targets without depending on the core library.

### TD-5: State Management

**Decision: Zustand + Tauri events.**

**Rationale**:
- Zustand is already the natural choice (lightweight, React-first, subscriptions work with `$state` resolution).
- Tauri events provide the push channel from the Rust backend.
- No Redux, no MobX, no Jotai. Zustand's `subscribe` API maps cleanly to the `$state` reference resolution mechanism.
- The `PortBus` (Section 2.4) handles block-to-block communication separately from global state.

**Store structure**:

```
useKernelStore         -- kernel-pushed data (metrics, processes, events, services)
useWorkspaceStore      -- current block assembly, layout state, active journey
usePreferencesStore    -- user preferences (theme, font size, default layout)
```

### TD-6: Minimum Viable Implementation for 0.2

**The MVP is a working console + 5 core blocks + descriptor engine.** See Section 7 for the full phased breakdown.

MVP scope (Sprint 11-12, targeting 0.2.0-alpha):

1. **ConsolePan block** with xterm.js, connected to `ShellAdapter` via `execute_command` Tauri command. Tab completion for namespaces and methods. No rich output overlays yet (text only).
2. **Block engine**: Descriptor parser, Zustand StateStore with `$state` resolution, block registry with 5 blocks: `Column`, `Row`, `Metric`, `DataTable`, `ConsolePan`.
3. **Kernel state push**: Replace mock `useKernelWs` with real Tauri event listener backed by kernel polling.
4. **Default dashboard**: A hard-coded `BlockDescriptor` that arranges metrics, process table, and a console pane. Equivalent to the current `Dashboard.tsx` but rendered from JSON.
5. **Wire 4 existing Tauri commands** to real kernel APIs: `kernel_status`, `query_chain`, `spawn_agent`, `stop_agent`.

What is NOT in MVP:
- Drag-and-drop block assembly (K8.4)
- Journey mode (K8.3)
- Rich console output overlays (K8.5)
- WebBrowser block, CausalGraph block (K8.2+)
- Weaver-generated descriptors (K8.5)
- RL learning loop (K8.6)

---

## Session 5: K8 Roadmap (15 min)

### Revised K8 Roadmap

The original design notes define K8.1-K8.6. After cross-referencing with Track 1 (registries need GUI introspection), Track 3 (release blockers must be resolved first), and Track 9 (HNSW and tick loop optimizations affect ECC graph rendering performance), the revised roadmap is:

#### K8.1: Foundation (Sprint 11-12, v0.2.0-alpha) -- BUILD THIS FIRST

**Deliverables**:
1. Block descriptor schema (TypeScript types + Zod validation)
2. Block registry with 5 blocks: `Column`, `Row`, `Metric`, `DataTable`, `ConsolePan`
3. StateStore (Zustand) with `$state` resolution engine
4. `execute_command` Tauri command wired to `ShellAdapter`
5. `complete_command` Tauri command for tab completion
6. Replace `useKernelWs` mock with Tauri event-based `useKernelState`
7. Default dashboard rendered from a `BlockDescriptor`
8. xterm.js `ConsolePan` with basic command execution (text output only)

**Dependencies**: Track 3 blocker B1 (rvf-crypto feature gate) must be resolved so the kernel can build standalone. Track 9 item B7 (release profile) is desirable but not blocking.

**Effort**: ~40 hours.

**What is first**: Item 3 (StateStore + `$state` resolution). Everything else depends on it.

#### K8.2: Core Blocks + Kernel Wiring (Sprint 13, v0.2.0-beta)

**Deliverables**:
1. 6 additional blocks: `ChainViewer`, `CausalGraph`, `DiffViewer`, `CodeEditor`, `ApprovalGate`, `Markdown`
2. Wire all 13 Tauri commands to real kernel APIs
3. Port bus for block-to-block communication
4. `ServiceMap` block (2D only -- depends on Track 1's Registry trait for uniform data)
5. Governance enforcement in ShellAdapter (dual-layer gate on every command)
6. ECC witnessing: chain-log all console commands

**Dependencies**: Track 1 recommendation 1 (Registry trait) enables the ServiceMap block. Track 1 recommendation 2 (ChainLoggable trait) enables consistent chain logging from the GUI.

**Effort**: ~60 hours.

#### K8.3: Journey Mode + Guided Tours (Sprint 14, v0.2.0-rc)

**Deliverables**:
1. Journey schema (TypeScript types + Zod validation)
2. Journey renderer: step navigation, narrative panel, breakout mode
3. Weaver integration: `weaver.generate-journey` command
4. Git diff parsing for journey generation
5. Example journey: Sprint 10 Review Tour
6. `Tabs` block for multi-view steps

**Dependencies**: K8.2 (blocks needed for journey steps). Weaver must be wired to generate valid `BlockDescriptor` JSON.

**Effort**: ~40 hours.

#### K8.4: Assembly + Drag-and-Drop (Sprint 15-16, v0.2.1)

**Deliverables**:
1. Drag-and-drop via `@dnd-kit/core`
2. Block palette sidebar (list all available blocks from registry)
3. Nesting: drag blocks into container blocks
4. Port wiring UI: visual connections between block ports
5. Workspace save/load (IndexedDB)
6. `ResourceTree` block

**Dependencies**: K8.2 (all core blocks must exist before users can assemble them).

**Effort**: ~50 hours.

**This must wait for 0.3 if Sprint 15-16 overflows.**

#### K8.5: Rich Console + Agent Generation (Sprint 17-18, v0.3.0)

**Deliverables**:
1. Rich console output: inline block rendering via xterm.js decoration overlays
2. `DisplayHint` protocol (Text, Table, Block, SpawnBlock)
3. Weaver generates `BlockDescriptor` JSON validated against catalog schemas
4. Tab completion for all data sources (PIDs, service names, ECC labels, governance rules)
5. Console themes and prompt customization
6. `WebBrowser` block (embedded WebView)
7. 3D ECC visualization (React Three Fiber for CausalGraph block)

**Dependencies**: K8.4 (assembly must work before agent-generated blocks can be placed). Track 9 items O1-O3 (HNSW performance) strongly recommended before 3D graph rendering with large datasets.

**Effort**: ~80 hours.

#### K8.6: Polish + RL Loop (Sprint 19-20, v0.3.1)

**Deliverables**:
1. RL learning from assembly patterns and console usage
2. Multi-window support (detach any block assembly to a new Tauri window)
3. Theming system (light/dark, custom colors, font choices)
4. End-to-end example: user builds a custom workspace with console + browser + 3D ECC tour
5. Performance optimization: virtual scrolling for large tables, lazy block rendering
6. Mentra HUD renderer (constraint-driven layout from same descriptors)
7. Terminal renderer (Ink) -- same blocks render in pure CLI mode

**Dependencies**: K8.5. Mentra HUD renderer depends on the Mentra integration work (separate track).

**Effort**: ~60 hours.

### Realistic Version Targets

| Phase | Sprints | Version | Confidence |
|-------|---------|---------|------------|
| K8.1 | 11-12 | 0.2.0-alpha | HIGH -- scope is tight and well-defined |
| K8.2 | 13 | 0.2.0-beta | HIGH -- most blocks are straightforward React components |
| K8.3 | 14 | 0.2.0-rc | MEDIUM -- journey generation from Weaver is the risk |
| K8.4 | 15-16 | 0.2.1 or 0.3 | MEDIUM -- drag-and-drop complexity depends on nesting depth |
| K8.5 | 17-18 | 0.3.0 | MEDIUM-LOW -- 3D and rich console overlays are complex |
| K8.6 | 19-20 | 0.3.1 | LOW -- RL loop and multi-renderer are research-adjacent |

### Dependency Graph

```
Track 3 (B1: rvf-crypto) --.
                            +--> K8.1 (Foundation) --> K8.2 (Core Blocks) --> K8.3 (Journey)
Track 9 (B7: opt-level)  --'                              |
                                                           v
                                          K8.4 (Assembly) --> K8.5 (Rich Console + 3D)
                                                                       |
Track 1 (Registry trait) ----> K8.2 (ServiceMap block)                 v
Track 1 (ChainLoggable) ----> K8.2 (ECC witnessing)      K8.6 (Polish + RL)
Track 9 (O1-O3: HNSW) ------> K8.5 (3D graph perf)
```

---

## 8. Minimum Viable 0.2 Scope (Summary)

The absolute minimum to ship as 0.2.0-alpha and demonstrate the Lego architecture:

1. **Block engine**: Descriptor parser + `$state` resolver + registry (500 lines TypeScript)
2. **5 blocks**: `Column`, `Row`, `Metric`, `DataTable`, `ConsolePan`
3. **ConsolePan**: xterm.js with `execute_command` -> `ShellAdapter` -> `ServiceApi` (text output only)
4. **Tab completion**: namespace + method level (hard-coded catalog, not live kernel query yet)
5. **Kernel state**: Tauri event push replacing the mock timer in `useKernelWs`
6. **Default dashboard**: JSON descriptor rendered instead of hand-coded JSX
7. **4 wired Tauri commands**: `kernel_status`, `query_chain`, `spawn_agent`, `stop_agent`

Everything else (journey mode, drag-and-drop, 3D, rich console overlays, Weaver generation, RL loop) is K8.2+.

---

## 9. ECC Contribution: Causal Nodes and Edges for CMVG

```
NODES (new, extending 00-opening-plenary.md baseline):

  [N12] Block Descriptor Schema Defined
       status: ACHIEVED (this document)
       evidence: Complete BlockDescriptor, BlockElement, StateRef, PortBinding types
       artifact: docs/weftos/sprint11-symposium/04-ui-ux-design.md Section 2.1

  [N13] Block Catalog Enumerated
       status: ACHIEVED (this document)
       evidence: 18 blocks validated against kernel surface
       artifact: Section 2.2 (catalog table + Zod schema examples)

  [N14] Console Architecture Designed
       status: ACHIEVED (this document)
       evidence: Dual-channel model, command catalog, governance + ECC witnessing
       artifact: Section 1

  [N15] Technology Stack Decided
       status: ACHIEVED (this document)
       evidence: 6 technology decisions with rationale
       decisions: No dockview, Yes xterm.js, CodeMirror 6, custom renderer,
                  Zustand, MVP = console + 5 blocks

  [N16] K8 Roadmap Revised
       status: ACHIEVED (this document)
       evidence: 6 phases mapped to sprints with dependencies
       artifact: Section 5

  [N17] Journey Model Specified
       status: DESIGNED
       evidence: Journey schema, generation flow, example journey
       artifact: Section 3

EDGES (new):

  N2  --[Motivates]--> N12    K8 GUI prototype needs structured block format
  N12 --[Enables]-->   N13    Schema enables catalog definition
  N13 --[Enables]-->   N15    Catalog requirements drive technology decisions
  N14 --[Depends]-->   N1     Console requires ShellAdapter (kernel complete)
  N15 --[Enables]-->   N16    Tech decisions constrain the roadmap
  N7  --[Feeds]-->     N13    Registry trait (Track 1) feeds ServiceMap block
  N8  --[Feeds]-->     N14    Audit gaps (Track 1) inform console ECC witnessing
  N16 --[Depends]-->   N8_T3  Release blocker B1 (Track 3) must be resolved for K8.1
  N16 --[Depends]-->   O1_T9  HNSW optimization (Track 9) needed for K8.5 3D graph

CAUSAL CHAIN (K8 critical path):
  N1 (achieved) --> N14 (achieved) --> [K8.1 console wiring]
  N2 (achieved) --> N12 (achieved) --> N13 (achieved) --> [K8.1 block engine]
  N12 + N13 + N14 --> N15 (achieved) --> N16 (achieved) --> [K8.1 implementation start]
  N16 --> [K8.2] --> N17 (designed) --> [K8.3 journey mode]
```

---

## 10. High-Priority Questions

**[HP-12]** The `ShellAdapter::execute()` method parses `"service.method args"` format. However, many natural console commands do not fit this pattern (e.g., `help`, `clear`, `history`, `cd`). Should the console have a set of built-in client-side commands that bypass the ShellAdapter? Recommendation: Yes -- define a `BuiltinCommands` layer in the frontend that intercepts known non-kernel commands before sending to Rust.

**[HP-13]** The existing `useKernelWs` hook connects to `ws://localhost:9800/ws`. When switching to Tauri events, the WS fallback is still needed for browser-mode deployments (when the GUI runs without the Tauri wrapper, as indicated by `window.__TAURI_INTERNALS__` check in `App.tsx` footer). Should we maintain both transport paths? Recommendation: Yes -- the `useKernelState` hook should detect Tauri availability and fall back to WebSocket. This aligns with the heterogeneous support requirement (design notes Section 6).

**[HP-14]** The `ConsolePan` block needs access to the ShellAdapter, which requires the kernel to be running. In development mode (hot-reloading the React frontend without a kernel), what is the console's behavior? Recommendation: Implement a `MockShellAdapter` that returns help text and simulated output, activated when the kernel connection is unavailable. The existing mock data in `useKernelWs.ts` can be repurposed.

**[HP-15]** Block descriptors generated by the Weaver need governance approval before rendering (to prevent malicious or broken UI generation). Should governance approval happen at generation time (before the descriptor is stored) or at render time (before the block engine renders it)? Recommendation: At generation time -- validate against Zod schemas AND governance gate before storing. Once stored and approved, rendering is fast and does not need re-approval. The governance approval chain event sequence is recorded in `BlockMeta.governance_seq`.

---

## Panel Consensus Notes

**system-architect** (chair): The dual-channel model (Tauri invoke for commands, Tauri events for push) is the correct architecture. It maps cleanly to the existing `ServiceApi` and `ShellAdapter` abstractions in the kernel. The decision not to adopt dockview simplifies the implementation considerably.

**kernel-architect**: The ShellAdapter is already designed for exactly this use case. The `execute(command)` method parses `service.method args` and routes through `ServiceApi::call()`. The GUI just needs to expose this as a Tauri command and wire the real kernel implementation (replacing the current TODO stubs). The governance gate integration point is at the ShellAdapter level, which is correct.

**mesh-engineer**: The `ServiceMap` block in K8.2 will need mesh peer data. The `mesh.peers` and `mesh.topology` commands should return structured data (not just text) so the ServiceMap block can render it. This requires the mesh subsystem to expose its peer table via `ServiceApi`.

**mentra-ui**: The 400x240 constraint of the Mentra HUD means most blocks need a radically different layout. The JSON descriptor architecture handles this well -- same descriptor, different renderer -- but the Mentra renderer must be aware of which blocks can be meaningfully rendered at that resolution. `Metric` yes, `CausalGraph` no (too small), `ConsolePan` only with very large font.

**mentra-ux**: Voice-first interaction for the console should be K8.6 scope. The console command catalog is naturally voice-compatible because commands are short, structured strings. Tab completion data sources can be used to constrain speech recognition.

**doc-weaver**: The journey model is powerful but the Weaver's ability to generate good narratives from git diffs depends on having good commit messages and code comments. The Sprint 10 example journey works because the commits are well-structured. For arbitrary repos, the narrative quality will vary. Suggest adding a `narrative_quality_score` to journey metadata so the UI can show a confidence indicator.

**ecc-analyst**: Every console command logged to ExoChain creates a rich audit trail. The `shell.exec` event format captures governance decisions, effect vectors, and timing. This feeds directly into the DEMOCRITUS loop -- the ECC can learn patterns from GUI usage (which commands are run together, which governance decisions are overridden, etc.).

**governance-counsel**: The dual-layer gate on every console command is non-negotiable. The design correctly places governance enforcement at the ShellAdapter level, before the command reaches the ServiceApi. The `shell.deny` chain event for rejected commands is important for audit compliance.

**mobile-dev**: The browser-mode fallback (HP-13) is essential for the SaaS deployment path. The `useKernelState` hook should abstract the transport so components never know whether they are in Tauri or browser mode. The state shape is identical either way.

**weaver**: Generating `BlockDescriptor` JSON is safer and more constrained than generating arbitrary TSX. The Zod catalog schemas provide a validation boundary that prevents the Weaver from generating invalid UI. This is a significant improvement over the current `generate_component` Tauri command which outputs raw TSX strings.

---

*Document generated by Sprint 11 Symposium Track 4 panel.*
*All findings and decisions are based on direct code reading of the `feature/weftos-kernel-sprint` branch as of 2026-03-27.*
