# ADR-007: Zustand + Tauri Events for State Management

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 4 (UI/UX Design)

## Context

The WeftOS GUI needs to synchronize frontend state with the Rust kernel backend. The kernel pushes state snapshots (process table, metrics, chain events, ECC data) to the frontend, and the frontend resolves `$state` path references in block descriptors against this state tree. The dual-channel architecture uses Tauri `invoke()` for request/response commands and Tauri events for real-time state push.

## Decision

Use Zustand as the frontend state management library with `$state` path resolution. The Zustand StateStore subscribes to Tauri events (`kernel://snapshot`) for real-time state updates pushed from the Rust backend. Commands use Tauri `invoke()` for type-safe request/response. This replaces the mock `useKernelWs` hook.

## Consequences

### Positive
- Zustand is lightweight (~1 KB) and supports path-based state access naturally
- Integrates cleanly with Tauri's event listener API
- Separates command semantics (invoke) from subscription semantics (events)
- Type-safe: Tauri invoke provides automatic serde; events carry typed payloads
- `$state` references in block descriptors resolve against the Zustand store directly

### Negative
- Zustand's flat store model may require denormalization of deeply nested kernel state
- No built-in time-travel debugging (Redux DevTools not available)

### Neutral
- State lives in Rust; the frontend is a projection -- Zustand is the projection surface
- Backend pushes snapshots every ~500ms or on kernel event bus subscription
