# WeftOS Feature Gate Reference

All feature flags are defined in `crates/clawft-kernel/Cargo.toml`. Only
`native` is enabled by default.

## Feature Table

| Feature | What it enables | Key dependencies | Use when |
|---------|----------------|------------------|----------|
| `native` | Tokio async runtime, full kernel boot | tokio, tokio-util, futures | Always -- this is the default |
| `ecc` | Cognitive substrate: causal graph, HNSW vector memory, tick loop | blake3, clawft-core/vector-memory | AI/cognitive features, semantic search |
| `exochain` | Cryptographic audit trail, resource tree, governance events | rvf-crypto, rvf-wire, rvf-types, rvf-runtime, exo-resource-tree, ed25519-dalek, ciborium | Audit logging, compliance, governance |
| `mesh` | P2P WebSocket networking between nodes | ed25519-dalek, tokio-tungstenite, futures-util | Multi-node clusters |
| `os-patterns` | Self-healing, dead-letter queue, metrics, structured logging | (implies `exochain`) | Production deployments |
| `onnx-embeddings` | Real ONNX model inference for embeddings | ort, ndarray | Quality semantic search (replaces mock provider) |
| `wasm-sandbox` | Wasmtime-based tool execution sandbox | wasmtime, wasmtime-wasi | Running untrusted WASM tools safely |
| `cluster` | Distributed coordination: Raft consensus, sharding, replication | ruvector-cluster, ruvector-raft, ruvector-replication, parking_lot | Multi-node consensus and data replication |
| `tilezero` | TileZero governance gate | cognitum-gate-tilezero (implies `exochain`) | Advanced governance policies |
| `containers` | Container runtime integration markers | (none extra) | Docker/Podman orchestration hooks |

## Dependency Graph

```
tilezero ──> exochain
os-patterns ──> exochain
cluster (independent)
mesh (independent)
ecc (independent)
onnx-embeddings (independent)
wasm-sandbox (independent)
containers (independent)
```

Features that imply `exochain` automatically pull in its dependencies. You do
not need to list `exochain` explicitly when enabling `tilezero` or
`os-patterns`.

## Recommended Combinations

### Development (fast builds)

```bash
scripts/build.sh native
# Only the default `native` feature. Fastest compile times.
```

### Single-node Production

```bash
scripts/build.sh native --features ecc,exochain,os-patterns
```

Gives you the cognitive substrate, cryptographic audit trail, self-healing,
and structured metrics -- everything needed for a single production node.

### Multi-node Cluster

```bash
scripts/build.sh native --features ecc,exochain,os-patterns,mesh,cluster
```

Adds P2P mesh networking and Raft-based distributed consensus on top of the
single-node stack.

### Full Features

```bash
scripts/build.sh native --features ecc,exochain,os-patterns,mesh,cluster,onnx-embeddings,wasm-sandbox
```

Everything enabled. Requires the ONNX model to be downloaded
(`scripts/download-model.sh`) and a Wasmtime-compatible host.

## Checking Active Features

At runtime, `weave status` reports which feature gates were compiled in.
During development, `scripts/build.sh check` validates that the selected
feature combination compiles cleanly.

## Adding a New Feature Gate

1. Add the feature and its optional deps to `crates/clawft-kernel/Cargo.toml`.
2. Guard code with `#[cfg(feature = "your-feature")]`.
3. Add a row to the table above.
4. Run `scripts/build.sh gate` to verify all combinations still compile.

## Platform constraints (host OS, not Cargo features)

Some host-local substrate adapters are gated by **target OS**, not by
Cargo features — Cargo features cannot express "only on Linux."

### Linux-only sysfs adapters (WEFT-420)

| Adapter | Crate path | Probe source |
|---------|------------|--------------|
| `network` | `clawft-substrate::network` | `/sys/class/net`, `/sys/class/power_supply` |
| `bluetooth` | `clawft-substrate::bluetooth` | `/sys/class/bluetooth`, `/sys/class/rfkill` |
| `rfkill` | `clawft-substrate::rfkill` | `/sys/class/rfkill` (same ABI; related) |

**Compile gate:** modules are native-only (`cfg(not(target_arch = "wasm32"))`).
**Runtime gate:** `clawft_substrate::sysfs::linux_sysfs_native()`
(`cfg!(target_os = "linux")`).

| Host | Behaviour |
|------|-----------|
| Linux | Real sysfs sampling. Missing hardware → plain `absent` / `present: false` (no platform `cause`). |
| macOS / Windows / other | Adapter still **opens** so tray/Explorer keep a uniform surface. Emits placeholders with `platform: "unsupported"` and `cause: "sysfs-unavailable"`, and writes `substrate/meta/adapter/<id>/health` with `event: "error"` and that cause in `reason`. |
| wasm32 | Modules not compiled; GUI uses Snapshot fallback. |

macOS (CoreWLAN / IOBluetooth) and Windows (WinRT) ports are
**unscheduled**. Do not treat non-Linux `absent` as "no radio" without
checking `cause == "sysfs-unavailable"` or the adapter-health reason.

Shared helpers live in `crates/clawft-substrate/src/sysfs.rs`.
