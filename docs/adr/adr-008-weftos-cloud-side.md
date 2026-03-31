# ADR-008: WeftOS Cloud-Side for Mentra (Not On-Device)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 6 (Mentra Integration)

## Context

Mentra smart glasses use a BES2700 SoC with 8 MB PSRAM. WeftOS kernel requires 50-200 MB minimum for its runtime (HNSW store, ECC causal graph, process table, ExoChain). Running WeftOS directly on the glasses hardware is physically impossible with current silicon.

## Decision

WeftOS runs cloud-side or on a companion device, never on the Mentra glasses themselves. The glasses communicate with WeftOS via WebSocket using the A2UI streaming protocol. The JSON block descriptor architecture is the integration contract -- the same descriptor renders on the Tauri desktop GUI and on the Mentra HUD with a constraint-driven layout (400x240, monochrome, 8-10 lines, voice-only input).

## Consequences

### Positive
- No hardware constraints on WeftOS kernel complexity
- Clean separation: glasses are a thin rendering client, WeftOS is the compute backend
- Latency budget (<500ms simple commands, <800ms search) is achievable over WebSocket
- Same descriptor format means no separate API for the glasses

### Negative
- Requires network connectivity for glasses to function with WeftOS
- Latency is bounded by network round-trip, not local compute
- Companion device or cloud infrastructure adds deployment complexity

### Neutral
- MVP scope: 2 HUD views (System Status, Agent List) + 1 voice command ("Status")
- Implementation starts Sprint 12; Sprint 11 delivers specification only
