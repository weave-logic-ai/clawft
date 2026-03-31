# ADR-004: No Dockview -- CSS Grid + Custom Lego Engine

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (UI/UX Design)

## Context

The WeftOS GUI is built around a "Lego block" metaphor where every UI element is a JSON-described block that can be composed, nested, and connected. Dockview provides VS Code-style panel docking (split panes, tabs, drag-to-rearrange). Using dockview would mean two competing layout systems: dockview's panel arrangement and the Lego engine's block composition.

## Decision

Do not adopt dockview. Use CSS Grid for layout and build a custom Lego engine that renders block descriptors directly. The layout is driven by the JSON block descriptor's `Column`, `Row`, and nesting structure, not by a panel docking library.

## Consequences

### Positive
- Single layout system -- the JSON descriptor drives everything
- Full control over `$state` resolution, governance gating, and multi-target rendering
- Simpler dependency graph; no conflict between dockview's model and block descriptors
- Blocks render identically across targets (web, terminal, Mentra HUD) because layout is in the descriptor

### Negative
- Must implement drag-and-drop panel rearrangement from scratch
- No free split pane / tab group behavior from a battle-tested library
- More initial development effort for the layout engine

### Neutral
- CSS Grid is well-supported and performant in all modern browsers and Tauri's WebView
