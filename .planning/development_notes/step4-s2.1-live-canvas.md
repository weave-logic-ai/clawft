# S2.1: Live Canvas -- Development Notes

## Summary

Implemented the Live Canvas system: protocol types in Rust, a `render_ui` tool for agents, and a full Canvas React component with per-element rendering and interaction handling.

## Changes

### 1. Canvas Protocol Types (Rust)

**File:** `crates/clawft-types/src/canvas.rs`

- Defined `CanvasElement` enum with 7 variants: `Text`, `Button`, `Input`, `Image`, `Code`, `Table`, `Form`
- Defined `FormField` struct for form element fields
- Defined `CanvasCommand` enum with 5 variants: `Render`, `Update`, `Remove`, `Reset`, `Batch`
- Defined `CanvasInteraction` enum with 3 variants: `Click`, `InputSubmit`, `FormSubmit`
- All types use `#[serde(tag = ...)]` for clean JSON serialization
- All optional fields use `#[serde(default)]` for backward compatibility
- 27 unit tests covering serialization, deserialization, defaults, and roundtrips

**File:** `crates/clawft-types/src/lib.rs`

- Registered `pub mod canvas;`

### 2. render_ui Tool (Rust)

**File:** `crates/clawft-tools/src/render_ui.rs`

- Implements the `Tool` trait from `clawft-core`
- Parses input JSON as `CanvasCommand` with validation
- Returns element ID on success
- Logs commands via `tracing`
- Functional stub -- real message bus integration comes later
- 17 unit tests covering all element types, commands, and error paths

**File:** `crates/clawft-tools/src/lib.rs`

- Added `#[cfg(feature = "canvas")] pub mod render_ui;`
- Added canvas registration in `register_all()` behind feature flag

**File:** `crates/clawft-tools/Cargo.toml`

- Added `canvas = []` feature flag

### 3. Canvas React Route + Components

**Route:** `ui/src/routes/canvas.tsx`

- Full canvas page with WebSocket subscription to `canvas` topic
- Handles `canvas_command` messages to update canvas state
- Sends `canvas_interaction` messages back to server
- Reset button to clear canvas

**Store:** `ui/src/stores/canvas-store.ts`

- Zustand store with `Map<string, CanvasElementData>` for element storage
- `addElement`, `updateElement`, `removeElement`, `reset` actions

**Components** (`ui/src/components/canvas/`):

- `canvas-renderer.tsx` -- Renders list of elements from store with empty state
- `canvas-element.tsx` -- Switch component dispatching to element-specific renderers
- `text-element.tsx` -- Renders plain or markdown text
- `button-element.tsx` -- Clickable button with action callback
- `input-element.tsx` -- Text input with submit on Enter or button click
- `code-element.tsx` -- Code block with language badge
- `table-element.tsx` -- HTML table with headers and rows
- `form-element.tsx` -- Dynamic form with field types and validation

### 4. Router + Navigation

**File:** `ui/src/App.tsx`

- Added `/canvas` route with `CanvasPage` component

**File:** `ui/src/components/layout/MainLayout.tsx`

- Added "Canvas" nav item with "V" icon

### 5. TypeScript Types

**File:** `ui/src/lib/types.ts`

- Added `CanvasElementData`, `CanvasCommandData`, `CanvasInteractionData`

## Verification

- `cargo test -p clawft-types -- canvas`: 27 tests pass
- `cargo test -p clawft-tools --features canvas -- render_ui`: 17 tests pass
- `cargo test --workspace`: all tests pass, zero regressions
- `cd ui && npm run build`: TypeScript compiles and Vite builds successfully

## Design Decisions

1. **Feature flag**: `render_ui` tool is behind `canvas` feature flag so it does not affect existing builds unless explicitly opted in.
2. **Stub approach**: The tool validates and logs commands but does not yet publish to the message bus. This allows frontend development to proceed independently.
3. **Map-based store**: Using `Map<string, CanvasElementData>` instead of array for O(1) lookups by element ID.
4. **WebSocket topic**: Canvas subscribes to `canvas` topic and listens for `canvas_command` events, sending `canvas_interaction` events back.
