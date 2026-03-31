# ADR-003: CodeMirror 6 Over Monaco for Code Editor Block

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (UI/UX Design)

## Context

The WeftOS GUI needs a code editor block for displaying and editing code within the Lego block engine. The two dominant web-based editor libraries are Monaco (VS Code's engine, ~2.5 MB) and CodeMirror 6 (~150 KB, modular architecture). The GUI will frequently embed multiple editor instances within a single view, and the Tauri desktop app bundles the frontend into a native binary where bundle size matters.

## Decision

Use CodeMirror 6 for all code editing blocks in the WeftOS GUI. Do not use Monaco.

## Consequences

### Positive
- 150 KB vs 2.5 MB -- roughly 17x smaller, significant when embedding multiple instances
- Modular architecture allows loading only needed language modes
- Better suited for embedding within Lego block panels (not designed to be a full IDE)
- More responsive in constrained environments (Mentra HUD text fallback, terminal rendering)

### Negative
- Fewer built-in IDE features compared to Monaco (no integrated IntelliSense)
- Smaller ecosystem of third-party extensions
- Some developers may expect Monaco-grade editing in a desktop app

### Neutral
- Both libraries support syntax highlighting, completions, and customization
- CodeMirror 6 has strong TypeScript support
