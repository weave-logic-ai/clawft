# ADR-016: Multi-Target Theming System

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (K8.6 deliverable, brought forward)

## Context

WeftOS renders the same JSON block descriptors across 8+ targets (React, Terminal, Mentra HUD, Voice, MCP, PDF, Shell). Each target has different styling primitives: CSS custom properties for web, ANSI escape codes for terminal, brightness levels for HUD, pace/tone hints for voice. Embedding colors and fonts directly in block descriptors or component code would make multi-target rendering impossible.

## Decision

Implement a renderer-agnostic token-based theming system. Themes define semantic tokens (`colors.surface`, `colors.danger`, `typography.fontFamily.mono`). Each renderer maps tokens to target-specific primitives. Theme definitions are JSON files with sections for colors, typography, spacing, borders, effects, animation, and target-specific overrides (console ANSI, HUD brightness, voice pacing).

The default theme (`ocean-dark`) derives its palette from the Fumadocs ocean preset HSL values and maps to the WeftOS indigo brand palette. Themes follow the same discovery chain as skills: workspace > user > built-in.

## Consequences

### Positive
- Block descriptors are completely style-free -- theming is injected at render time
- One theme definition produces consistent styling across all 8+ targets
- User-extensible: themes are shareable JSON files
- Brand consistency enforced via semantic tokens, not hardcoded values

### Negative
- Every new rendering target requires implementing a token-to-primitive mapper
- Theme definitions are larger (must cover all targets) than single-target CSS
- A theme that only defines web colors is considered incomplete

### Neutral
- The Fumadocs ocean.css preset governs the docs site separately; this system governs the Tauri GUI, console, and HUD
