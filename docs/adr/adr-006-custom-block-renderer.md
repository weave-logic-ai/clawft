# ADR-006: Custom Block Renderer (json-render Pattern, Not Dependency)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (UI/UX Design)

## Context

Two open-source projects validate the JSON-descriptor-to-UI pattern: vercel-labs/json-render (spec-centric, catalog + registry, 15+ render targets) and google/a2ui (protocol-centric, streaming typed messages). WeftOS needs `$state` resolution from the kernel, governance gating on every action, and multi-target rendering (React, Terminal/Ink, Mentra HUD, MCP, PDF, Shell). Taking json-render as a direct dependency would constrain the kernel integration surface.

## Decision

Build a custom block renderer that follows the json-render pattern (flat adjacency list, catalog with Zod validation, component registry) and incorporates A2UI's streaming protocol (surface lifecycle, incremental data updates), but do not take json-render or a2ui as npm dependencies. The WeftOS kernel command set is the action catalog; the Zustand StateStore maps to the kernel's ServiceApi.

## Consequences

### Positive
- Full control over `$state` resolution to kernel state paths
- Governance gating integrated at the action dispatch level
- Multi-target rendering from a single descriptor (React, Terminal, HUD, MCP, PDF)
- Agents generate JSON descriptors validated against Zod schemas, not arbitrary TSX code
- Descriptor format is semver'd independently from renderer implementations

### Negative
- Must implement and maintain a custom renderer instead of using a maintained library
- Risk of diverging from the json-render spec as it evolves
- More upfront engineering effort

### Neutral
- The descriptor schema (v0.2.0) is documented in the block catalog specification
- The pattern is validated by two major projects with 13K+ stars each
