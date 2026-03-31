# Mentra HUD Renderer Constraints

**Version**: 0.2.0
**Date**: 2026-03-27
**Source**: Sprint 11 Symposium Track 6 (Mentra Integration & Network Navigator)
**Status**: Formal specification -- implementable

---

## Overview

The Mentra HUD renderer is a constraint-driven layout engine that takes standard WeftOS block descriptors and renders them for the Mentra smart glasses display. The same JSON descriptor that renders as a full React GUI on desktop renders as a compact, monochrome, voice-controlled view on the HUD.

### Deployment Model

WeftOS does NOT run on the glasses. The BES2700 SoC (ARM Cortex-M33 + Cortex-A7, 8MB PSRAM) runs MentraOS (RTOS). WeftOS runs cloud-side or on a companion device (phone running `Kernel<AndroidPlatform>`). The glasses are a thin terminal that renders pre-computed display frames received over the network.

```
+-------------------+     BLE/WiFi     +-------------------+
| Mentra Glasses    | <--------------> | Companion Device  |
| BES2700 (RTOS)   |                   | (Phone/Laptop)    |
| - Voice capture   |                   | - Audio processing|
| - HUD renderer    |                   | - Descriptor cache|
| - BT/WiFi radio   |                   | - Offline mode    |
+-------------------+                   +--------+----------+
                                                 | HTTPS/WebSocket
                                                 v
                                        +-------------------+
                                        | MentraOS AppServer|
                                        | (Cloudflare Workers)
                                        +--------+----------+
                                                 | WebSocket (persistent)
                                                 v
                                        +-------------------+
                                        | WeftOS Kernel     |
                                        | - ServiceApi      |
                                        | - StateStore      |
                                        | - GovernanceGate  |
                                        +-------------------+
```

---

## Display Constraints

These are hardware limits of the Mentra glasses display. The HUD renderer must respect all of them.

| Constraint | Value | Rationale |
|-----------|-------|-----------|
| Resolution | 400 x 240 pixels | Mentra display hardware |
| Color depth | Monochrome (green on black, or limited palette) | OLED power efficiency, outdoor readability |
| Minimum font size | 16px equivalent | Readable at arm's length through the prism |
| Maximum text lines | 8-10 visible lines | Usable information density at the given font size |
| Maximum characters per line | ~40 characters | 400px / ~10px per character at 16px font |
| Interaction model | Voice only (no touch, no gesture) | Glasses form factor -- hands-free |
| Data refresh rate | 2-5 Hz for data updates | Battery life constraint |
| Animation refresh rate | 30 Hz | Smooth enough for simple transitions |
| Header | 1 line (always visible) | Status/title bar |
| Footer | 1 line (always visible) | Voice command hints |
| Content area | 6-8 lines | Between header and footer |

### Character Budget

```
+----------------------------------------+   <- 40 chars wide
|  HEADER LINE (title + status)          |   <- Line 1 (fixed)
|----------------------------------------|
|  Content line 1                        |   <- Line 2
|  Content line 2                        |   <- Line 3
|  Content line 3                        |   <- Line 4
|  Content line 4                        |   <- Line 5
|  Content line 5                        |   <- Line 6
|  Content line 6                        |   <- Line 7
|  Content line 7                        |   <- Line 8
|  Content line 8                        |   <- Line 9
|----------------------------------------|
|  "cmd1" | "cmd2" | "cmd3"             |   <- Line 10 (fixed)
+----------------------------------------+
```

---

## Block Descriptor Adaptation Rules

The HUD renderer applies these rules when converting a standard block descriptor to a 400x240 monochrome display.

### Layout Block Adaptation

| Block Type | HUD Behavior |
|-----------|-------------|
| `Column` | Renders children sequentially, each getting proportional line allocation |
| `Row` | Renders children side-by-side separated by ` | `. Falls back to Column if total width exceeds 40 chars |
| `Grid` | Falls back to Column layout (grid not supported at 400x240) |
| `Tabs` | Renders only the active tab content. No tab bar. Voice command "tab N" to switch |

### Data Display Block Adaptation

| Block Type | HUD Behavior |
|-----------|-------------|
| `Metric` | Single line: `Label: [========  ] Value Unit` (with bar if threshold defined) or `Label: Value Unit` |
| `DataTable` | Fixed-width columns, max 4 columns visible, truncated to fit content area. Column headers on first line |
| `ChainViewer` | One line per event: `#Seq Kind Summary`. Max 6 events visible |
| `CausalGraph` | Linearized causal chain (numbered list). Max 6 nodes visible. No graph layout. |
| `DiffViewer` | NOT rendered. Falls back to summary: `N files, +A/-D lines` |
| `CodeEditor` | NOT rendered. Falls back to: `File: name.rs (N lines)` |

### Interactive Block Adaptation

| Block Type | HUD Behavior |
|-----------|-------------|
| `Button` | Rendered as a voice command hint in the footer bar |
| `ConsolePan` | NOT directly rendered. Voice commands route through the same ShellAdapter |
| `ApprovalGate` | Summary line: `Action: risk=N`. Voice commands: "approve", "deny" |
| `TextInput` | NOT interactive on HUD. Renders current value as read-only text |
| `Markdown` | Plain text. Headings rendered as UPPERCASE. Bold as-is. Code blocks omitted. Max 6 lines |

### OS Capability Block Adaptation

| Block Type | HUD Behavior |
|-----------|-------------|
| `WebBrowser` | NOT rendered. Shows URL text only |
| `ResourceTree` | Flat list of top-level entries, max 8 visible |
| `ServiceMap` | Radial topology (same rendering as RadialTopology). Max 8 nodes |

### General Rules

1. **Truncation**: Any text longer than 40 characters is truncated with `...` at position 37.
2. **Nesting depth**: Only the first 2 nesting levels are rendered. Deeper content is collapsed to a summary line.
3. **Color**: Only three visual states: normal (green), warning (yellow/brighter), error (red/inverted). Mapped from threshold values and status fields.
4. **Scrolling**: Content that exceeds the visible area supports voice-driven pagination: "scroll down", "scroll up", "page 2".

---

## Voice Command Mapping

Voice commands are the sole interaction method on the Mentra HUD. They map to kernel commands through the ShellAdapter.

### Global Voice Commands

These are available on any HUD view.

| Voice Command | Kernel Command | Description |
|--------------|----------------|-------------|
| "status" | `kernel.status --format hud` | Show system status card |
| "agents" | `process.list --format hud` | Show agent list |
| "network" | `mesh.peers --format hud` | Show network topology |
| "back" | (navigation) | Return to previous view |
| "refresh" | (re-fetch current view data) | Force refresh of current view |
| "scroll down" | (pagination) | Next page of content |
| "scroll up" | (pagination) | Previous page of content |
| "help" | (show command list) | Display available voice commands |

### Contextual Voice Commands

Available only when specific views or blocks are active.

| Context | Voice Command | Kernel Command | Description |
|---------|--------------|----------------|-------------|
| Agent list | "spawn [type]" | `process.spawn <type>` | Spawn a new agent |
| Agent list | "kill [pid]" | `process.stop <pid>` | Stop an agent |
| Agent list | "detail [pid]" | `process.info <pid>` | Show agent detail |
| Network view | "detail [peer]" | `mesh.peer-detail <peer>` | Show peer detail |
| Network view | "services on [peer]" | `mesh.services <peer>` | List services on a peer |
| Chain view | "expand [seq]" | `chain.detail <seq>` | Expand a chain event |
| Causal graph | "expand [N]" | `ecc.node-detail <id>` | Expand a causal node |
| Causal graph | "show diff" | `ecc.node-diff <id>` | Show associated diff |
| Approval gate | "approve" | `governance.approve <id>` | Approve the pending action |
| Approval gate | "deny" | `governance.deny <id>` | Deny the pending action |
| Assessment | "details" | (paginate findings) | Show next page of findings |
| Assessment | "fix top" | `weaver.fix <top_issue_id>` | Trigger Weaver fix for top issue |
| Assessment | "full report" | `weaver.report --format pdf` | Generate PDF report |
| Journey | "next" | (navigation) | Go to next journey step |
| Journey | "back" | (navigation) | Go to previous journey step |

### Wake Word

The wake word is "Hey WeftOS". The glasses perform on-device wake word detection (low-power always-on audio). After detection, the next utterance is captured and sent to the companion device for parsing.

```
User: "Hey WeftOS, status"
  |
  +-- Wake word detected on-device
  +-- "status" captured and sent to companion
  +-- Companion parses to kernel_exec("kernel.status --format hud")
  +-- WeftOS returns StatusCard descriptor
  +-- HUD renders it
```

---

## MVP HUD Wireframes

### MVP View 1: System Status Card

This is the default view shown when the user says "status".

```
+----------------------------------------+
|  WEFTOS STATUS                         |
|----------------------------------------|
|  Kernel:   Running     v0.2.0          |
|  Services: 14 active                   |
|  Agents:   3 running, 1 idle           |
|  Health:   [==========] 100%           |
|  Chain:    block #4,271                |
|  Uptime:   14h 22m                     |
|                                        |
|----------------------------------------|
|  "agents" | "network" | "refresh"      |
+----------------------------------------+
```

**Block descriptor for this view**:

```json
{
  "version": "0.2.0",
  "root": "status-card",
  "meta": { "target_hint": "mentra-hud", "refresh_hz": 2 },
  "elements": {
    "status-card": {
      "type": "Column",
      "children": ["header", "content", "hints"]
    },
    "header": {
      "type": "StatusBar",
      "props": {
        "left": "WEFTOS STATUS",
        "right": ""
      }
    },
    "content": {
      "type": "Column",
      "children": ["kernel-line", "services-line", "agents-line", "health-line", "chain-line", "uptime-line"]
    },
    "kernel-line": {
      "type": "Metric",
      "props": {
        "label": "Kernel",
        "value": { "$state": "/kernel/state", "$default": "Unknown" }
      }
    },
    "services-line": {
      "type": "Metric",
      "props": {
        "label": "Services",
        "value": { "$state": "/kernel/services/count" },
        "unit": "active"
      }
    },
    "agents-line": {
      "type": "Metric",
      "props": {
        "label": "Agents",
        "value": { "$state": "/kernel/processes/summary" }
      }
    },
    "health-line": {
      "type": "Metric",
      "props": {
        "label": "Health",
        "value": { "$state": "/kernel/health/overall" },
        "unit": "%",
        "threshold": { "warn": 80, "crit": 50 }
      }
    },
    "chain-line": {
      "type": "Metric",
      "props": {
        "label": "Chain",
        "value": { "$state": "/kernel/chain/height", "$transform": "blockNumber" }
      }
    },
    "uptime-line": {
      "type": "Metric",
      "props": {
        "label": "Uptime",
        "value": { "$state": "/kernel/uptime", "$transform": "humanDuration" }
      }
    },
    "hints": {
      "type": "HintBar",
      "props": {
        "hints": ["agents", "network", "refresh"]
      }
    }
  }
}
```

**HUD renderer output** (what actually appears on the 400x240 display):

```
Line 1:  WEFTOS STATUS
Line 2:  ----------------------------------------
Line 3:  Kernel:   Running     v0.2.0
Line 4:  Services: 14 active
Line 5:  Agents:   3 running, 1 idle
Line 6:  Health:   [==========] 100%
Line 7:  Chain:    block #4,271
Line 8:  Uptime:   14h 22m
Line 9:  ----------------------------------------
Line 10: "agents" | "network" | "refresh"
```

---

### MVP View 2: Agent List

Shown when the user says "agents".

```
+----------------------------------------+
|  AGENTS: 4 total                       |
|----------------------------------------|
|  PID  Agent        State    CPU        |
|  001  weaver       running  12%        |
|  002  coder-1      running   8%        |
|  003  reviewer     idle      0%        |
|  004  tester-1     running   5%        |
|                                        |
|                                        |
|----------------------------------------|
|  "spawn [type]" | "kill [pid]" | "back"|
+----------------------------------------+
```

**Block descriptor for this view**:

```json
{
  "version": "0.2.0",
  "root": "agent-list",
  "meta": { "target_hint": "mentra-hud", "refresh_hz": 2 },
  "elements": {
    "agent-list": {
      "type": "Column",
      "children": ["header", "table", "hints"]
    },
    "header": {
      "type": "StatusBar",
      "props": {
        "left": { "$state": "/kernel/processes/active_count", "format": "AGENTS: {v} total" },
        "right": ""
      }
    },
    "table": {
      "type": "DataTable",
      "props": {
        "columns": [
          { "key": "pid", "label": "PID" },
          { "key": "agent_id", "label": "Agent" },
          { "key": "state", "label": "State" },
          { "key": "cpu_percent", "label": "CPU" }
        ],
        "rows": { "$state": "/kernel/processes" }
      },
      "on": {
        "voice_select": {
          "action": "kernel_exec",
          "params": { "command": "process.info {selected.pid}" }
        }
      }
    },
    "hints": {
      "type": "HintBar",
      "props": {
        "hints": ["spawn [type]", "kill [pid]", "back"]
      }
    }
  }
}
```

**HUD renderer output**:

```
Line 1:  AGENTS: 4 total
Line 2:  ----------------------------------------
Line 3:  PID  Agent        State    CPU
Line 4:  001  weaver       running  12%
Line 5:  002  coder-1      running   8%
Line 6:  003  reviewer     idle      0%
Line 7:  004  tester-1     running   5%
Line 8:
Line 9:  ----------------------------------------
Line 10: "spawn [type]" | "kill [pid]" | "back"
```

---

## Latency Budget

End-to-end latency for voice commands from the glasses to the HUD response.

```
Phase                          Target    Max
---------------------------------------------
Voice capture + VAD            50ms      100ms
Audio to glasses BLE buffer    20ms      50ms
BLE/WiFi to companion          30ms      80ms
Companion to AppServer         40ms      100ms
AppServer to WeftOS            20ms      50ms
WeftOS command execution       100ms     500ms
Response to AppServer          20ms      50ms
AppServer to companion         40ms      100ms
Companion to glasses           30ms      80ms
HUD render                     10ms      30ms
---------------------------------------------
TOTAL (simple command)         360ms     640ms
TOTAL (search command)         400ms     800ms
TOTAL (analysis command)       1-5s      (async with progress)
```

**Target**: Simple commands (status, agents, network) under 500ms perceived latency. Analysis commands show immediate "working..." indicator with streaming progress via A2UI `updateDataModel` messages.

---

## Offline Behavior

The companion device maintains a descriptor cache with TTL. When the network connection is lost:

| Feature | Offline Support | Mechanism |
|---------|----------------|-----------|
| Cached status display | Yes | Last-known state from companion cache |
| Cached topology view | Yes | Last snapshot of peer list |
| Voice command parsing | Partial | On-device wake word + basic local commands |
| HNSW search | No | Requires kernel |
| Weaver analysis | No | Requires kernel + LLM |
| Alert history | Yes | Cached on companion |
| Time/date/battery | Yes | Local device |

Cached views render with a stale-data indicator:

```
+----------------------------------------+
|  NETWORK: 5 peers  |  [CACHED 2m ago]  |
|----------------------------------------|
```

The `[CACHED Nm ago]` badge appears in the header when data age exceeds the refresh interval.

---

## Ambient Layer

Outside of full views, the HUD displays a persistent ambient status bar:

```
+------------------------------------------+
|  [5 peers] [12ms] [3 agents]  11:42 AM   |
+------------------------------------------+
```

This is a `StatusBar` block with `$state` bindings:
- `/kernel/cluster/peer_count`
- `/kernel/cluster/avg_latency_ms`
- `/kernel/processes/active_count`
- Local time

The ambient layer is always visible when no full view is active. It consumes 1 line of the display. Saying "status", "agents", or "network" replaces the ambient layer with the full view. Saying "back" from any full view returns to the ambient layer.

---

## Testing Strategy

Until Mentra hardware is available (Sprint 14+), testing uses:

1. **Text-based HUD simulator**: A terminal program that renders descriptors within a 40x10 character box, simulating the exact HUD constraints. Validates that every descriptor produces output within bounds.
2. **Mock WebSocket client**: Simulates glasses traffic patterns (voice commands, descriptor requests, latency profiles).
3. **Constraint validation unit tests**: For each block type, verify that the HUD renderer output fits within 40 characters per line and the content area line count.

Test matrix entry for the HUD renderer:
```
INPUT:   Any valid BlockDescriptor (from block-descriptor-schema.json)
OUTPUT:  Vec<String> where each string.len() <= 40 and vec.len() <= 10
ASSERT:  No output line exceeds 40 characters
ASSERT:  Total output lines do not exceed 10
ASSERT:  Header line is always present
ASSERT:  Footer (HintBar) line is always present
```
