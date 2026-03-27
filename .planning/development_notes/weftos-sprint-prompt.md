# WeftOS Sprint Prompt (Persistent Reference)

**Created**: 2026-03-01
**Status**: Active Sprint

## Original Directive

> Get everything checked in, then start a sub-queen coordinator, and run the
> WeftOS project as far as you can through that sub-agent using a swarm of
> agents, expert agents, concurrent and complementary, get consensus when
> needed, ensure you are testing and documenting. All tasks should be broken
> out to expert agents, with notes specific to that task being kept, keep
> detailed info about decisions, challenges, completion, and times. The
> development notes belong in .planning/development_notes. Write and ensure
> tests get run and 100% clean clippy etc, keep git up to date at gate points.
> Work through sub-queen, keeping task running until all of WeftOS that can be
> written is done, noting always what is being skipped and delegating in the
> SPARC planning files. Keep this prompt as a note for this task, all sub
> tasks, and anytime you clear context in the completion of the WeftOS sprint.

## Key Rules

1. **Expert agents** for each task -- not generalists
2. **Development notes** in `.planning/development_notes/weftos/`
3. **Track**: decisions, challenges, completion times
4. **Test after every phase**: `scripts/build.sh test`, `scripts/build.sh clippy`
5. **Git commit at gate points** (never to master)
6. **Note what's skipped** and why in SPARC planning files
7. **Consensus** for architectural decisions
8. **100% clean**: clippy, tests, compilation

## Phase Order (from orchestrator)

| Phase | ID | What | Deps |
|-------|-----|------|------|
| 0 | K0 | Kernel Foundation (crate, boot, process table, service registry, health, console) | None |
| 1 | K1 | Supervisor + RBAC (agent spawn/stop/restart, capabilities) | K0 |
| 2 | K2 | A2A IPC (agent messaging, pub/sub, JSON-RPC) | K1 |
| 3 | K3 | WASM Sandbox (wasmtime, fuel, memory limits) | K0 (parallel with K1) |
| 4 | K4 | Containers (Docker/bollard, sidecar orchestration) | K0 (parallel with K1) |
| 5 | K5 | App Framework (manifests, lifecycle, interop) | K1+K2 |

## What Can Be Written Now

- K0: Full kernel foundation (no external deps beyond existing crates)
- K1: Supervisor + RBAC (builds on K0)
- K2: A2A IPC (builds on K1)
- K3: WASM sandbox stubs (wasmtime is optional feature)
- K4: Container stubs (bollard is optional feature)
- K5: App manifest parsing + lifecycle state machine

## What Must Be Skipped/Stubbed

- Ruvector crate integration (crates not published yet) -- feature-gate stubs only
- ExoChain/exo-resource-tree (crate not published yet) -- feature-gate stubs only
- Docker runtime testing (requires Docker daemon)
- Wasmtime runtime testing (requires wasmtime dep)
- ClawHub marketplace (future work)

## Build Commands

```bash
scripts/build.sh check    # Fast compile check
scripts/build.sh test     # Run all tests
scripts/build.sh clippy   # Lint
scripts/build.sh gate     # Full gate (11 checks)
```

## Branch

`feature/three-workstream-implementation` (current working branch)
