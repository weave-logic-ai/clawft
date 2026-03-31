# ADR-005: xterm.js for WeftOS Console

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (UI/UX Design)

## Context

The WeftOS GUI includes a first-class console that connects to the kernel's ShellAdapter. This console needs ANSI color support, GPU-accelerated rendering, cursor positioning, and the ability to render inline rich output (decoration overlays for block descriptors). The console is both a standalone full-screen mode and an embeddable Lego block (`ConsolePan`).

## Decision

Use xterm.js as the terminal emulator for the WeftOS console block. Connect it to the kernel ShellAdapter via Tauri's invoke channel for commands and Tauri events for streaming output.

## Consequences

### Positive
- Industry standard terminal emulator (VS Code, Theia, Hyper all use it)
- GPU-accelerated rendering via WebGL addon
- Full ANSI escape sequence support (colors, cursor, decorations)
- Decoration API enables inline rich rendering (block descriptors rendered over terminal output)
- Active maintenance and large community

### Negative
- Adds a non-trivial dependency to the frontend bundle
- Custom features (governance display, inline block rendering) require writing xterm.js addons
- Does not natively support the block descriptor system -- integration layer needed

### Neutral
- Works well within Tauri's WebView environment
- Tab completion and command history must be implemented on top (ShellAdapter provides completion data)
