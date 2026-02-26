# Step 7 - S3.2: Advanced Canvas

## Summary

Implemented advanced canvas element types (Chart, CodeEditor, FormAdvanced) and canvas history (undo/redo) for the Live Canvas system.

## Changes

### 1. TypeScript Types (`ui/src/lib/types.ts`)

- Added `ChartDataPoint`, `ChartType`, `ChartCanvasElement` types for chart data visualization
- Added `CodeCanvasElement` type for code editor elements
- Added `FormFieldType`, `FormFieldSchema`, `FormCanvasElement` types for advanced forms with validation

### 2. Chart Element (`ui/src/components/canvas/chart-element.tsx`)

New component supporting three chart types with no external dependencies:

- **Bar chart**: CSS-based horizontal bars using percentage widths relative to max value
- **Line chart**: SVG-based with axes, grid lines, data points, and x-axis labels
- **Pie chart**: SVG-based with arc slices, color legend, and value display
- All charts use a default color palette and accept custom colors
- Responsive sizing within canvas element bounds

### 3. Code Editor Element (`ui/src/components/canvas/code-element.tsx`)

Enhanced the existing `CodeElement` (preserved for backward compat with `code` type) and added `CodeEditorElement` for the `code_editor` type:

- Line number gutter with dynamic width based on line count
- Optional editable mode using a transparent textarea overlay on top of pre/code
- Tab key inserts 2-space indentation
- Ctrl+Enter submits code via canvas interaction event
- Scroll sync between textarea and pre elements

### 4. Advanced Form Element (`ui/src/components/canvas/form-element.tsx`)

Preserved the existing `FormElement` for backward compat and added `AdvancedFormElement` for `form_advanced` type:

- Field types: text, number, select, checkbox, textarea
- Required field validation with error display
- Number range validation (min/max)
- Error messages clear on field change
- Submit triggers canvas interaction event with all form values

### 5. Canvas Store History (`ui/src/stores/canvas-store.ts`)

Added undo/redo history to the zustand canvas store:

- `past` and `future` stacks as serializable element snapshots (array of entries)
- `pushState()` integrated into `addElement`, `updateElement`, `removeElement`
- `undo()` pops from past, pushes current to future, restores
- `redo()` pops from future, pushes current to past, restores
- Max history depth: 50 entries
- `canUndo` / `canRedo` derived booleans for UI state
- `reset()` clears all history

### 6. Canvas Toolbar (`ui/src/components/canvas/canvas-toolbar.tsx`)

New toolbar component with undo/redo controls:

- Undo button (disabled when past is empty)
- Redo button (disabled when future is empty)
- Keyboard shortcuts: Ctrl+Z (undo), Ctrl+Shift+Z (redo), Ctrl+Y (redo alt)
- Inline SVG icons (no lucide dependency needed)

### 7. Canvas Element Switch (`ui/src/components/canvas/canvas-element.tsx`)

Updated the element type router to handle:

- `chart` -> `ChartElement`
- `code_editor` -> `CodeEditorElement`
- `form_advanced` -> `AdvancedFormElement`
- Preserved all existing element type routing

### 8. Canvas Page (`ui/src/routes/canvas.tsx`)

- Integrated `CanvasToolbar` into the header bar alongside element count and clear button

### 9. Rust Canvas Types (`crates/clawft-types/src/canvas.rs`)

Added three new `CanvasElement` variants:

- `Chart { data, chart_type, title, colors }` with `ChartDataPoint` struct
- `CodeEditor { code, language, editable, line_numbers }` with defaults
- `FormAdvanced { fields, submit_action }` with `AdvancedFormField` struct

Added `CodeSubmit` variant to `CanvasInteraction` enum.

Added comprehensive tests: serialization, deserialization with defaults, roundtrip, render commands with new types. All 216 tests pass.

## Verification

- `cd ui && npm run build` -- passes (tsc + vite build)
- `cargo test -p clawft-types` -- 216 tests pass
- `cargo build --release --bin weft` -- compiles successfully

## Files Modified

| File | Action |
|------|--------|
| `ui/src/lib/types.ts` | Updated (added advanced canvas types) |
| `ui/src/stores/canvas-store.ts` | Updated (added undo/redo history) |
| `ui/src/components/canvas/canvas-element.tsx` | Updated (new type routing) |
| `ui/src/components/canvas/code-element.tsx` | Updated (added CodeEditorElement) |
| `ui/src/components/canvas/form-element.tsx` | Updated (added AdvancedFormElement) |
| `ui/src/routes/canvas.tsx` | Updated (toolbar integration) |
| `ui/src/components/canvas/chart-element.tsx` | Created |
| `ui/src/components/canvas/canvas-toolbar.tsx` | Created |
| `crates/clawft-types/src/canvas.rs` | Updated (new variants + tests) |
