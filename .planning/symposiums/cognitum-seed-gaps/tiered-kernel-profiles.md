# ADR-049: Tiered Kernel Profiles

| Field | Value |
|-------|-------|
| Status | Proposed |
| Date | 2026-04-04 |
| Deciders | WeftOS kernel team |
| Supersedes | N/A |
| Depends on | ADR-021 (daemon-first RPC), ADR-047 (self-calibrating tick) |

## Context

WeftOS must run on hardware ranging from 512KB-SRAM microcontrollers (ESP32-S3)
to multi-node server clusters. Today the kernel crate compiles as a single
profile with feature flags that the operator must manually compose. There is no
formal mapping between a deployment target and the set of subsystems that should
boot, leading to three problems:

1. **Binary bloat on constrained devices.** Compiling with default features
   pulls in tokio, ECC, ExoChain, and cluster code that cannot run on an edge
   gateway with 512MB of RAM, let alone a 512KB MCU.
2. **Configuration guesswork.** Operators must know which features to enable and
   which `KernelConfig` fields to set. A `profile` field that auto-selects sane
   defaults eliminates this.
3. **No path for bare-metal sensors.** The ESP32 `clawft-edge-bench` crate
   exists as a standalone benchmark but has no protocol contract for joining
   a WeftOS mesh. Sensors need a `no_std` crate with a compact binary protocol.

## Decision

Introduce a **KernelProfile** enum (`Sensor`, `Edge`, `Node`, `Server`,
`Cluster`) that maps each deployment tier to a fixed set of compile-time
features and boot-time subsystem gates. The profile is set in `weave.toml`:

```toml
[kernel]
profile = "node"
```

Individual overrides remain possible:

```toml
[kernel]
profile = "edge"

[kernel.vector]
backend = "hnsw"   # override: enable HNSW even on edge
```

## Tier Definitions

### T0: Sensor (ESP32-C3/S3, 512KB SRAM, 4MB flash)

- **Crate:** `clawft-sensor` (new, `no_std`, `no-alloc` optional)
- **Runtime:** Bare metal or FreeRTOS via esp-idf-hal
- **Subsystems:** None from clawft-kernel. Sensor crate provides:
  - Fixed-size ring buffer (no heap) for sample accumulation
  - On-device FFT/detection (compile-time feature)
  - Compact binary protocol (CBOR or custom TLV, not JSON-RPC)
  - WiFi/BLE/LoRa mesh transport to pack leader (T1/T2)
- **Build:** Separate Cargo workspace member, cross-compiled
  ```bash
  cargo build -p clawft-sensor --target xtensa-esp32s3-espidf --release
  ```
- **Memory budget:** 48-128KB heap (depends on ring buffer size)
- **Binary size:** 200-400KB (flash image including esp-idf)
- **Role:** Hindbrain. Senses, samples, detects, reports. Does not think.

### T1: Edge (Raspberry Pi Zero, 512MB RAM, SD card)

- **Feature set:** `--no-default-features --features edge`
- **Subsystems booted:**
  - Process table (max 8 processes)
  - Service registry
  - IPC (local only)
  - Health system
  - Append-only log (no dual signing, no Ed25519/ML-DSA)
  - Supervisor (basic, no WASM sandbox)
- **Subsystems excluded:**
  - ExoChain (no signing, no governance)
  - ECC (no HNSW, no causal graph, no cognitive tick)
  - Cluster (no Raft, no ruvector)
  - Assessment service
  - A2A router topic subscriptions
  - OS-patterns observability
- **Vector store:** Brute-force linear scan, 1K-5K vectors max
- **Network:** Mesh peer; forwards data upstream to T2/T3 coordinator
- **Build:**
  ```bash
  scripts/build.sh native --features edge --no-default-features
  # or
  cargo build -p clawft-kernel --no-default-features --features edge --release
  ```
- **Memory budget:** 64-128MB (leaves room for OS + application)
- **Binary size:** 4-8MB (no crypto, no vector libs)

### T2: Node (Raspberry Pi 5, 8GB RAM, SSD)

- **Feature set:** default features (`native`, `exochain`, `cluster`,
  `tilezero`, `ecc`)
- **Subsystems booted:** All kernel subsystems
  - Full ExoChain with dual signing (Ed25519 + ML-DSA-65)
  - ECC with HNSW (50K vectors in-memory)
  - Governance gate with genesis rules
  - Resource tree
  - Assessment service
  - Container service
  - Cluster membership (single-node or mesh coordinator for T0/T1)
- **Build:**
  ```bash
  scripts/build.sh native
  # or
  cargo build -p clawft-kernel --release
  ```
- **Memory budget:** 1-4GB (HNSW at 50K vectors ~200MB, rest for OS + app)
- **Binary size:** 20-35MB
- **Role:** This is what Cognitum Seed runs.

### T3: Server (x86/ARM cloud, 16-64GB RAM, NVMe)

- **Feature set:** `--features full,http-api,profiles,diskann`
- **Additional subsystems:**
  - HTTP API facade (REST + WebSocket)
  - Multi-tenant profile isolation
  - Hybrid vector store (HNSW hot tier + DiskANN cold tier, millions of vectors)
  - Fleet management: coordinates multiple T2 nodes
  - Assessment service for weavelogic.ai clients
- **Build:**
  ```bash
  scripts/build.sh native --features full,http-api,profiles,diskann
  ```
- **Memory budget:** 8-32GB (DiskANN is SSD-backed, HNSW hot tier ~2GB)
- **Binary size:** 40-60MB

### T4: Cluster (Multi-node, 64GB+ aggregate, distributed storage)

- **Feature set:** `--features full,cluster,distributed`
- **Additional subsystems:**
  - Raft consensus via ruvector-raft
  - CRDT process tables (cross-node process migration)
  - Distributed vector store (sharded HNSW across nodes)
  - Cross-node event bus (mesh topology)
- **Build:**
  ```bash
  scripts/build.sh native --features full,cluster,distributed
  ```
- **Memory budget:** 64GB+ aggregate across nodes
- **Binary size:** 45-65MB

## Feature Matrix

| Subsystem | T0 Sensor | T1 Edge | T2 Node | T3 Server | T4 Cluster |
|-----------|:---------:|:-------:|:-------:|:---------:|:----------:|
| Process table | -- | yes (8) | yes (64) | yes (256) | yes (256) |
| Service registry | -- | yes | yes | yes | yes |
| IPC | -- | local | local | local+RPC | mesh |
| Health system | -- | yes | yes | yes | yes |
| A2A router | -- | -- | yes | yes | yes |
| Cron service | -- | -- | yes | yes | yes |
| Assessment service | -- | -- | yes | yes | yes |
| Container service | -- | -- | yes | yes | yes |
| ExoChain (append log) | -- | append | full | full | full |
| Ed25519 + ML-DSA signing | -- | -- | yes | yes | yes |
| Governance gate | -- | -- | yes | yes | yes |
| Resource tree | -- | -- | yes | yes | yes |
| ECC / HNSW | -- | -- | yes (50K) | yes (hybrid) | yes (distributed) |
| Causal graph | -- | -- | yes | yes | yes |
| Cognitive tick | -- | -- | yes | yes | yes |
| DiskANN | -- | -- | -- | yes | yes |
| HTTP API facade | -- | -- | -- | yes | yes |
| Multi-tenant profiles | -- | -- | -- | yes | yes |
| Raft consensus | -- | -- | -- | -- | yes |
| CRDT process table | -- | -- | -- | -- | yes |
| Distributed vector | -- | -- | -- | -- | yes |
| Ring buffer (no_std) | yes | -- | -- | -- | -- |
| Compact binary protocol | yes | -- | -- | -- | -- |
| Sensor sampling/FFT | yes | -- | -- | -- | -- |

## Memory Budget Summary

| Tier | Heap Budget | Binary Size | Vector Capacity |
|------|-------------|-------------|-----------------|
| T0 | 48-128KB | 200-400KB flash | N/A |
| T1 | 64-128MB | 4-8MB | 1K-5K (linear) |
| T2 | 1-4GB | 20-35MB | 50K (HNSW) |
| T3 | 8-32GB | 40-60MB | 1M+ (hybrid) |
| T4 | 64GB+ | 45-65MB | 10M+ (distributed) |

## Cargo Feature Mapping

```
# T0 — separate crate, no kernel dependency
clawft-sensor: no_std, #![no_main], esp-idf-hal

# T1 — kernel with minimal features
edge = ["dep:tokio", "clawft-core/native", "clawft-platform/native", "clawft-types/native"]
# (no exochain, no ecc, no cluster, no tilezero)

# T2 — default
default = ["native", "exochain", "cluster", "tilezero", "ecc"]

# T3 — full + server features
full = ["native", "exochain", "cluster", "tilezero", "ecc", "os-patterns", "mesh", "wasm-sandbox", "treesitter"]
# Plus: http-api, profiles, diskann

# T4 — full + distributed features
# Plus: distributed (new feature, CRDT tables + sharded vectors)
```

## The T0 Problem

The ESP32 cannot use `clawft-kernel` at all. The kernel crate depends on `std`,
`tokio`, `serde_json`, `dashmap`, and many other allocating crates. Even with
`--no-default-features`, the kernel's core types (ProcessTable, ServiceRegistry)
assume a heap allocator and threading.

**Resolution: `clawft-sensor` crate.**

The existing `clawft-edge-bench` crate demonstrates the pattern: it is a
standalone ESP32 binary that uses `esp-idf-hal` and communicates with the
kernel over TCP. We formalize this into a proper `clawft-sensor` crate:

```
clawft-sensor/
  Cargo.toml          # no_std, esp-idf-hal, optional alloc
  src/
    lib.rs            # Core sensor logic (ring buffer, detection, protocol)
    protocol.rs       # Compact binary wire format (CBOR or TLV)
    transport/
      wifi.rs         # WiFi mesh transport
      ble.rs          # BLE transport (optional)
      lora.rs         # LoRa transport (optional)
    sampling.rs       # Sensor sampling + local FFT
    ring_buffer.rs    # Fixed-size, no-alloc ring buffer
```

The `clawft-edge-bench` crate continues to exist as the performance benchmark
for T0 devices. `clawft-sensor` is the production runtime.

**Protocol contract between T0 and T1/T2:**

```
Header (4 bytes):
  [0]    magic: 0xWF
  [1]    version: 0x01
  [2]    msg_type: Report=0x01, Heartbeat=0x02, Alert=0x03, Config=0x80
  [3]    payload_len_hi

Payload (variable, max 1024 bytes):
  CBOR-encoded map with sensor-specific fields

Footer (4 bytes):
  CRC32 of header + payload
```

## Pack/Swarm Model

The tiered architecture forms a natural pack hierarchy:

```
                    T3/T4 Server (Fleet Coordinator)
                    /           |           \
              T2 Node A    T2 Node B    T2 Node C
              /    \          |          /    \
         T1 GW   T1 GW    T1 GW    T1 GW   T1 GW
         / | \    / \       |       / \      |
       T0  T0 T0 T0 T0   T0 T0   T0  T0   T0
```

**T0 -> T1/T2 (Sensor to Pack Leader):**
- T0 sensors report to their nearest T1 gateway or T2 node
- Reports use compact binary protocol over WiFi/BLE/LoRa
- T0 has no concept of the wider mesh; it knows one upstream address
- T1/T2 translates compact binary into JSON-RPC for kernel consumption

**T1 -> T2 (Gateway to Coordinator):**
- T1 forwards aggregated sensor data upstream via JSON-RPC over TCP/WebSocket
- T1 can perform local filtering (drop noise, deduplicate) before forwarding
- T2 manages the T1 device's health via periodic heartbeat checks

**T2 -> T3 (Node to Fleet Coordinator):**
- T2 nodes participate in mesh via WebSocket (mesh feature)
- T3 fleet coordinator tracks all T2 nodes, aggregates assessments
- T3 can push configuration updates down to T2 nodes

**T3 -> T4 (Server to Cluster):**
- Raft consensus for leader election
- CRDT process tables for cross-node process migration
- Sharded vector store with automatic rebalancing

## Configuration Examples

### T0 (weave.toml not used -- compiled into firmware)

```rust
// Compile-time config in clawft-sensor
const UPSTREAM_HOST: &str = "192.168.1.100";
const UPSTREAM_PORT: u16 = 8081;
const SAMPLE_INTERVAL_MS: u32 = 100;
const RING_BUFFER_SIZE: usize = 256;
```

### T1 (weave.toml)

```toml
[kernel]
profile = "edge"
enabled = true
max_processes = 8
health_check_interval_secs = 60

# No chain, no vector, no cluster sections needed
# Profile defaults handle exclusion
```

### T2 (weave.toml)

```toml
[kernel]
profile = "node"
enabled = true
max_processes = 64
health_check_interval_secs = 30

[kernel.chain]
enabled = true
checkpoint_interval = 100
checkpoint_path = "/home/genesis/.clawft/chain.json"

[kernel.resource_tree]
enabled = true

[kernel.vector]
backend = "hnsw"

[kernel.vector.hnsw]
max_elements = 50000
ef_construction = 200
m = 16
```

### T3 (weave.toml)

```toml
[kernel]
profile = "server"
enabled = true
max_processes = 256
health_check_interval_secs = 15

[kernel.chain]
enabled = true
checkpoint_interval = 1000
checkpoint_path = "/var/lib/weftos/chain.json"

[kernel.resource_tree]
enabled = true

[kernel.vector]
backend = "hybrid"

[kernel.vector.hnsw]
max_elements = 100000
ef_construction = 400

[kernel.vector.diskann]
max_points = 10000000
data_path = "/var/lib/weftos/diskann"

[kernel.vector.hybrid]
hot_capacity = 100000
promotion_threshold = 5

[kernel.cluster]
seed_nodes = []
node_name = "assess-us-east-1"
```

### T4 (weave.toml)

```toml
[kernel]
profile = "cluster"
enabled = true
max_processes = 256
health_check_interval_secs = 10

[kernel.chain]
enabled = true
checkpoint_interval = 5000

[kernel.vector]
backend = "hybrid"

[kernel.vector.diskann]
max_points = 50000000
data_path = "/data/weftos/diskann"

[kernel.cluster]
replication_factor = 3
shard_count = 128
heartbeat_interval_secs = 3
node_timeout_secs = 15
enable_consensus = true
min_quorum_size = 3
seed_nodes = ["10.0.1.10:7000", "10.0.1.11:7000", "10.0.1.12:7000"]
node_name = "cluster-node-01"
```

## Build Commands

```bash
# T0: Sensor (cross-compile for ESP32-S3)
cargo build -p clawft-sensor --target xtensa-esp32s3-espidf --release

# T0: Flash + monitor
cargo espflash flash -p clawft-sensor --release --monitor

# T1: Edge
scripts/build.sh native --no-default-features --features edge

# T2: Node (default)
scripts/build.sh native

# T3: Server
scripts/build.sh native --features full,http-api,profiles,diskann

# T4: Cluster
scripts/build.sh native --features full,cluster,distributed

# Verify any tier compiles
scripts/build.sh check
scripts/build.sh check --no-default-features --features edge
```

## Migration Path

Upgrading a device from one tier to the next requires two changes:

1. **Recompile** with the new feature set (or deploy the pre-built binary for
   the target tier).
2. **Update `weave.toml`** to set `profile = "<new-tier>"`.

The kernel reads the profile at boot and adjusts:
- `max_processes` default
- Which subsystems to initialize (even if the feature is compiled in, the
  profile can gate boot-time initialization)
- Vector store backend selection
- Health check interval

This means a T2 binary (compiled with default features) can be configured to
run in "edge" mode by setting `profile = "edge"` -- it will skip ExoChain,
ECC, and cluster initialization even though the code is linked.

**Downgrade path:** Set profile to a lower tier. The kernel skips subsystems
that belong to higher tiers. Data from higher-tier subsystems (chain checkpoints,
HNSW indexes) is preserved on disk but not loaded.

## Boot-Time Profile Gating

In `boot.rs`, after loading `KernelConfig`, the profile determines which
subsystems initialize:

```rust
let profile = kernel_config.profile();

// ExoChain: only T2+ (Node, Server, Cluster)
#[cfg(feature = "exochain")]
let chain_manager = if profile.has_exochain() {
    // ... existing chain init ...
} else {
    boot_log.push(BootEvent::info(
        BootPhase::Services,
        format!("ExoChain skipped (profile={})", profile),
    ));
    None
};

// ECC: only T2+ (Node, Server, Cluster)
#[cfg(feature = "ecc")]
let (ecc_hnsw, ...) = if profile.has_ecc() {
    // ... existing ECC init ...
} else {
    boot_log.push(BootEvent::info(
        BootPhase::Ecc,
        format!("ECC skipped (profile={})", profile),
    ));
    (None, None, None, None, None, None, None)
};
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| T0 protocol diverges from kernel RPC | Define protocol in shared `clawft-wire` crate used by both sensor and kernel |
| Edge profile too minimal for some use cases | Allow per-subsystem overrides in config (e.g., enable HNSW on edge) |
| Feature flag combinatorial explosion | Profiles are the primary interface; raw features are expert-only |
| Binary size regression on T1 | CI job that builds `--no-default-features --features edge` and checks size |
| T2 operators confused by profile system | Default profile is "node" (matches current behavior), zero migration needed |

## Consequences

**Positive:**
- Clear deployment story for each hardware tier
- Smaller binaries on constrained devices (T1: 4-8MB vs current 20-35MB)
- Self-documenting configuration via profile names
- ESP32 sensors get a proper runtime crate instead of ad-hoc benchmarks

**Negative:**
- New `clawft-sensor` crate to maintain (different build toolchain)
- `edge` feature must be kept in sync with kernel evolution
- Profile gating adds conditional logic to boot.rs

**Neutral:**
- Existing T2 deployments are unaffected (default profile = "node")
- ADR does not change the wire protocol between T2+ nodes
