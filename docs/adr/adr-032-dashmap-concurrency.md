# ADR-032: DashMap as Concurrency Primitive for Kernel State

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 1 (Kernel Architecture), K2b deadlock post-mortem

## Context

The WeftOS kernel manages multiple concurrent registries -- ProcessTable, ServiceRegistry, A2ARouter inboxes, TopicRouter, and ToolRegistry -- that are accessed from many async tasks simultaneously. During K0-K2, several of these registries were protected by `tokio::sync::Mutex`, which serialized all reads behind a single lock and caused contention under load. Sprint 11 Track 1 identified a deadlock in `a2a.rs` caused by nested `DashMap` access (K2b deadlock fix), which prompted a workspace-wide review of concurrency primitives.

The kernel needs a concurrency model that maximizes read throughput for hot-path lookups (process state queries, service discovery, topic subscription checks) while preventing the deadlock patterns discovered during K2b.

## Decision

All kernel registries that serve high-frequency concurrent reads use `DashMap` (from the `dashmap` crate) as the primary concurrency primitive. `tokio::sync::Mutex` is reserved exclusively for subsystems where strict ordering matters, specifically `ChainManager` (append-only chain integrity) and `TreeManager` (tree mutation ordering).

The convention is:

- **`DashMap`**: `ProcessTable` (`DashMap<Pid, ProcessEntry>`), `ServiceRegistry` (`DashMap<String, Arc<dyn SystemService>>` + `DashMap<String, ServiceEntry>`), `A2ARouter` inboxes (`DashMap<Pid, mpsc::Sender<KernelMessage>>`), `TopicRouter` subscriptions (`DashMap<String, Vec<Subscription>>`), `MeshConnectionPool` (`DashMap`-backed), and all future registries.
- **`tokio::sync::Mutex`**: `ChainManager` internals (append ordering), `TreeManager` internals (mutation ordering).
- **`Arc`**: Shared ownership across async task boundaries for all kernel subsystems.
- **`CancellationToken`** (from `tokio_util`): Cooperative shutdown for agent loops and services.

Nested `DashMap` access (holding a ref from one `DashMap` while accessing another) is prohibited. The K2b deadlock in `a2a.rs` was caused by exactly this pattern, where a `DashMap` guard on the inbox map was held while querying the `ProcessTable`.

## Consequences

### Positive
- DashMap's sharded lock model (16 shards by default) enables high read concurrency without contention -- multiple kernel subsystems can query the ProcessTable, ServiceRegistry, and TopicRouter simultaneously
- Eliminates the Mutex bottleneck on hot-path lookups (process state, service resolution, topic matching)
- Single consistent pattern across all kernel registries simplifies code review and onboarding
- Work item W39 (Sprint 11 synthesis) documents this convention for the team

### Negative
- DashMap has known deadlock risk with nested access patterns -- the K2b bug in `a2a.rs` demonstrates this is not theoretical; every new registry access site must be reviewed for nested guard holding
- DashMap guards cannot be held across `.await` points (they are not `Send`), which requires careful structuring of async code to clone/extract values before awaiting
- Slightly higher memory overhead than a bare `HashMap` behind `Mutex` due to per-shard metadata

### Neutral
- The `dashmap` crate is a well-maintained dependency already in the workspace; no new supply chain risk
- Testing concurrent access requires explicit multi-task test harnesses; sequential unit tests do not exercise the concurrency model
