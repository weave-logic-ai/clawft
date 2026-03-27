#!/usr/bin/env python3
"""
Structured data extracted from symposium results and SPARC phase plans.

This module contains the hand-extracted decision, commitment, and phase data
from the WeftOS symposium reports and SPARC planning documents. It is consumed
by _build_decisions_graph.py to produce the ECC-ready graph JSON.

Sources:
  - docs/weftos/k2-symposium/08-symposium-results-report.md
  - docs/weftos/k3-symposium/07-symposium-results-report.md
  - docs/weftos/k5-symposium/05-symposium-results.md
  - docs/weftos/ecc-symposium/05-symposium-results-report.md
  - .planning/sparc/weftos/0.1/*.md
"""
import json
import sys


def build_data():
    data = {
        "symposiums": [
            _k2_symposium(),
            _k3_symposium(),
            _ecc_symposium(),
            _k5_symposium(),
        ],
        "phases": _phases(),
        "decision_module_links": _decision_module_links(),
        "symposium_gap_links": _symposium_gap_links(),
    }
    # Apply resolutions from Sprint 09b decision triage.
    _apply_09b_resolutions(data)
    return data


def _apply_09b_resolutions(data):
    """Update decision and commitment statuses based on Sprint 09b triage.

    Source: .planning/sparc/weftos/0.1/09b-decision-triage.md
    Date: 2026-03-26

    The original symposium data captures statuses at symposium time. This
    overlay applies the resolutions discovered during Sprint 09b, fixing the
    stale feedback loop where 12+ decisions showed as "pending" even though
    they had been resolved.
    """
    # Decision status overrides: (symposium_id, decision_id) -> new_status
    decision_overrides = {
        # Tier 1 HIGH - Already implemented or implemented this sprint
        ("k3", "D1"):  "implemented",  # ToolRegistry hierarchical already in code
        ("k3", "D2"):  "implemented",  # GovernanceRequest::with_tool_context() added
        ("k3", "D12"): "implemented",  # SandboxLayer multi-layer added
        ("k2", "D11"): "implemented",  # dual_sign() already in chain.rs
        ("k3", "D8"):  "implemented",  # Revocation constants added

        # Tier 2 MEDIUM - Scheduled for Sprint 10
        ("k3", "D4"):  "scheduled",    # Wasmtime integration Sprint 10
        ("k3", "D6"):  "scheduled",    # WASI sandbox config bundled with D4
        ("k3", "D7"):  "scheduled",    # TreeNodeVersion Sprint 10
        ("k3", "D10"): "implemented",  # ServiceApi/BuiltinTool already separated
        ("k2", "D8"):  "scheduled",    # Chain-anchored API contracts Sprint 10
        ("k2", "D20"): "scheduled",    # N-dimensional EffectVector Sprint 10

        # Tier 3 DEFERRED
        ("k3", "D3"):  "deferred",     # 25 remaining tools -> Sprint 10+
        ("k3", "D5"):  "deferred",     # Disk cache bundled with D4
        ("k3", "D9"):  "deferred",     # CA chain post-1.0
        ("k3", "D11"): "deferred",     # Routing-time gate -> K5
        ("k3", "D13"): "deferred",     # WASM snapshots -> K6
        ("k3", "D14"): "deferred",     # tiny-dancer scoring
        ("k2", "D10"): "deferred",     # WASM-compiled shell
        ("k2", "D12"): "deferred",     # Chain-agnostic blockchain anchoring
        ("k2", "D13"): "deferred",     # ZK proofs
        ("k2", "D17"): "deferred",     # Governance trajectory learning

        # Superseded
        ("k2", "D18"): "superseded",   # SONA superseded by Weaver ECC
    }

    # Commitment status overrides
    commitment_overrides = {
        ("k2", "C2"): "implemented",  # ServiceApi trait exists
        ("k2", "C6"): "implemented",  # dual_sign() in chain.rs
        ("k3", "AC-1"): "implemented",
        ("k3", "AC-2"): "implemented",
        ("k3", "AC-3"): "implemented",
    }

    for symposium in data["symposiums"]:
        sid = symposium["id"]
        for decision in symposium.get("decisions", []):
            key = (sid, decision["id"])
            if key in decision_overrides:
                old = decision["status"]
                decision["status"] = decision_overrides[key]

        for commitment in symposium.get("commitments", []):
            key = (sid, commitment["id"])
            if key in commitment_overrides:
                commitment["status"] = commitment_overrides[key]


def _k2_symposium():
    return {
        "id": "k2",
        "date": "2026-03-04",
        "decisions": [
            {"id": "D1", "title": "Services are separate concept from processes", "rationale": "Services may be processes, external systems, or anything an agent is aware of. ServiceRegistry holds ServiceEntry referencing agent PID, external endpoint, or container ID.", "panel": "Architecture", "status": "implemented", "commitments": []},
            {"id": "D2", "title": "Agent backend selection: all approaches, built iteratively", "rationale": "Explicit > manifest > policy > default (Native) fallback chain. Progressive disclosure across K-phases.", "panel": "Architecture", "status": "implemented", "commitments": []},
            {"id": "D3", "title": "Bake SpawnBackend API now, implement incrementally", "rationale": "Crystallize config hooks in K2 so K3+ phases are additive, not breaking.", "panel": "Architecture", "status": "implemented", "commitments": ["C1"]},
            {"id": "D4", "title": "Layered protocol: kernel IPC -> ServiceApi -> protocol adapters", "rationale": "API surface that protocols bind to. Shell, MCP, gRPC, HTTP adapters bind to ServiceApi.", "panel": "Protocol", "status": "implemented", "commitments": ["C2"]},
            {"id": "D5", "title": "Kernel-native first, then A2A + MCP as adapters", "rationale": "Internal API surface IS the foundation for external protocols.", "panel": "Protocol", "status": "implemented", "commitments": []},
            {"id": "D6", "title": "K3 = same-node only; clustering moved to K5", "rationale": "Cross-node service discovery is clustering; K5 gets clustering (from K6), K6 becomes deep networking.", "panel": "Protocol", "status": "implemented", "commitments": []},
            {"id": "D7", "title": "Defense-in-depth: both routing-time and handler-time gate checks", "rationale": "A2ARouter gets routing-time gate check; GovernanceGate remains at handler-time.", "panel": "Governance", "status": "partial", "commitments": ["C4"]},
            {"id": "D8", "title": "ExoChain-stored immutable API contracts", "rationale": "Service API schemas are critical artifacts. Immutable chain-anchored schemas are source of truth.", "panel": "Governance", "status": "pending", "commitments": ["C3"]},
            {"id": "D9", "title": "Universal witness with configurable override", "rationale": "Sub-microsecond SHAKE-256 hashing; full witnessing by default, opt-out via manifest.", "panel": "Governance", "status": "implemented", "commitments": []},
            {"id": "D10", "title": "WASM-compiled shell with container sandbox", "rationale": "Shell scripts compiled to WASM, hash-chained, run inside container sandbox.", "panel": "Governance", "status": "pending", "commitments": ["C5"]},
            {"id": "D11", "title": "Enable post-quantum signing immediately", "rationale": "rvf-crypto already has ML-DSA-65 DualKey; just not called yet.", "panel": "Crypto", "status": "blocked", "commitments": ["C6"]},
            {"id": "D12", "title": "Chain-agnostic blockchain anchoring trait", "rationale": "Define trait surface now, implement simplest binding first.", "panel": "Crypto", "status": "pending", "commitments": ["C7"]},
            {"id": "D13", "title": "Zero-knowledge proofs as foundational capability", "rationale": "ZK critical for rollups, privacy-preserving governance, verifiable computation.", "panel": "Crypto", "status": "pending", "commitments": []},
            {"id": "D14", "title": "TEE backend: build compatible, test later", "rationale": "TEE = another SpawnBackend variant. Define interface, implement when hardware available.", "panel": "Crypto", "status": "implemented", "commitments": ["C8"]},
            {"id": "D15", "title": "Maximize adoption of production-ready code, not just RUV", "rationale": "Don't build what already exists. Each K-phase starts with integration audit.", "panel": "Strategy", "status": "implemented", "commitments": []},
            {"id": "D16", "title": "CRDT consensus + chain-native ordering", "rationale": "Chain provides causal ordering; CRDTs add conflict-free convergence for off-chain state.", "panel": "Strategy", "status": "implemented", "commitments": []},
            {"id": "D17", "title": "Layered routing: tiny-dancer learned + governance enforced", "rationale": "tiny-dancer suggests, capabilities verify, governance gate checks, message delivered.", "panel": "Strategy", "status": "pending", "commitments": []},
            {"id": "D18", "title": "SONA at K5, training data pipeline from K3 forward", "rationale": "K3/K4 build training data into all endpoints. SONA reuptake spike in late K4.", "panel": "Strategy", "status": "pending", "commitments": []},
            {"id": "D19", "title": "Breaking changes required, pre-1.0", "rationale": "Services require IPC API evolution. MessageTarget needs Service(name) routing.", "panel": "IPC", "status": "implemented", "commitments": []},
            {"id": "D20", "title": "Configurable N-dimensional effect algebra", "rationale": "Edge devices use 3D, full nodes use 10D. Configuration-driven dimensionality.", "panel": "IPC", "status": "pending", "commitments": ["C9"]},
            {"id": "D21", "title": "K3-K6 iterative cycle, not waterfall", "rationale": "Later phases may drive changes that loop back. Symposium minimizes K0-K2 rework.", "panel": "Implementation", "status": "implemented", "commitments": []},
            {"id": "D22", "title": "K6 SPARC spec required before implementation", "rationale": "Clear boundary between K5 (apps + clustering) and K6 (networking + replication).", "panel": "Implementation", "status": "implemented", "commitments": ["C10"]},
        ],
        "commitments": [
            {"id": "C1", "description": "Add SpawnBackend enum to SpawnRequest", "status": "implemented", "phase": "K2.1", "source_decision": "D3", "modules": ["clawft-kernel/supervisor"]},
            {"id": "C2", "description": "Define ServiceApi internal surface trait", "status": "pending", "phase": "K3", "source_decision": "D4", "modules": ["clawft-kernel/service"]},
            {"id": "C3", "description": "Chain-anchored service contracts", "status": "pending", "phase": "K3", "source_decision": "D8", "modules": ["clawft-kernel/chain", "clawft-kernel/service"]},
            {"id": "C4", "description": "Dual-layer gate in A2ARouter", "status": "partial", "phase": "K3", "source_decision": "D7", "modules": ["clawft-kernel/a2a"]},
            {"id": "C5", "description": "WASM-compiled shell pipeline", "status": "pending", "phase": "K3/K4", "source_decision": "D10", "modules": ["clawft-kernel/wasm_runner"]},
            {"id": "C6", "description": "Enable post-quantum dual signing (DualKey)", "status": "blocked", "phase": "K2.1", "source_decision": "D11", "modules": ["clawft-kernel/chain"]},
            {"id": "C7", "description": "ChainAnchor trait for blockchain anchoring", "status": "pending", "phase": "K3/K4", "source_decision": "D12", "modules": ["clawft-kernel/chain"]},
            {"id": "C8", "description": "Add SpawnBackend::Tee variant", "status": "implemented", "phase": "K2.1", "source_decision": "D14", "modules": ["clawft-kernel/supervisor"]},
            {"id": "C9", "description": "Configurable N-dimensional EffectVector", "status": "pending", "phase": "K3", "source_decision": "D20", "modules": ["clawft-kernel/governance"]},
            {"id": "C10", "description": "K6 SPARC spec (draft)", "status": "implemented", "phase": "pre-K6", "source_decision": "D22", "modules": []},
        ],
    }


def _k3_symposium():
    return {
        "id": "k3",
        "date": "2026-03-04",
        "decisions": [
            {"id": "D1", "title": "Hierarchical ToolRegistry with kernel base + per-agent overlays", "rationale": "Shared base (Arc) + per-agent overlay registries. Resolves CF-3 waste.", "panel": "Kernel Auditor", "status": "pending"},
            {"id": "D2", "title": "Context-based gate actions: generic tool.exec with rich context", "rationale": "Pass tool name + effect vector as context; governance handles granularity.", "panel": "Services Architect", "status": "pending"},
            {"id": "D3", "title": "Implement all 25 remaining tools in K4", "rationale": "Reference implementations prove the pattern; remaining tools are straightforward.", "panel": "Research Analyst", "status": "pending"},
            {"id": "D4", "title": "Wasmtime integration in K4 alongside container runtime", "rationale": "WASM container provides same isolation as Docker for WASM modules.", "panel": "Integration Architect", "status": "pending"},
            {"id": "D5", "title": "Disk-persisted compiled module cache", "rationale": "Wasmtime compilation is 50-200ms; disk persistence survives restarts.", "panel": "Integration Architect", "status": "pending"},
            {"id": "D6", "title": "Configurable WASI with read-only sandbox default", "rationale": "Per-tool filesystem access via ToolSpec.permissions.", "panel": "Research Analyst", "status": "pending"},
            {"id": "D7", "title": "Tree metadata for version history", "rationale": "Fast O(1) version queries without chain traversal.", "panel": "Services Architect", "status": "pending"},
            {"id": "D8", "title": "Informational revocation; governance gate handles enforcement", "rationale": "Revocation is data; governance rules decide what to do with it.", "panel": "Services Architect", "status": "pending"},
            {"id": "D9", "title": "Central authority + CA chain for tool signing", "rationale": "Kernel signs built-ins; developer keypairs with kernel-signed CA for third-party.", "panel": "Research Analyst", "status": "pending"},
            {"id": "D10", "title": "Separate ServiceApi and BuiltinTool concepts", "rationale": "Tools are one-shot; services are long-running. Different lifecycles.", "panel": "Integration Architect", "status": "pending"},
            {"id": "D11", "title": "Routing-time gate deferred to K5 cluster build", "rationale": "Single-node handler-time gate is sufficient for K4.", "panel": "Integration Architect", "status": "deferred"},
            {"id": "D12", "title": "Multi-layer sandboxing: governance + environment + sudo override", "rationale": "Three-layer defense in depth: governance, environment config, user override.", "panel": "Research Analyst", "status": "pending"},
            {"id": "D13", "title": "WASM snapshots unnecessary now; noted for K6", "rationale": "Tools are stateless. Snapshots relevant for long-running WASM services in K5+.", "panel": "RUV Expert", "status": "deferred"},
            {"id": "D14", "title": "tiny-dancer scoring for native vs WASM routing", "rationale": "Governance finds the right route; tiny-dancer finds the best route.", "panel": "RUV Expert", "status": "pending"},
        ],
        "commitments": [
            {"id": "AC-1", "description": "Hierarchical ToolRegistry with Arc-shared base", "status": "pending", "phase": "K4", "source_decision": "D1", "modules": ["clawft-kernel/wasm_runner", "clawft-kernel/boot"]},
            {"id": "AC-2", "description": "Enriched gate context (tool name + effect vector)", "status": "pending", "phase": "K4", "source_decision": "D2", "modules": ["clawft-kernel/agent_loop"]},
            {"id": "AC-3", "description": "Multi-layer FsReadFileTool sandboxing", "status": "pending", "phase": "K4", "source_decision": "D12", "modules": ["clawft-kernel/wasm_runner", "clawft-kernel/agent_loop"]},
            {"id": "AC-4", "description": "All 25 remaining tool implementations", "status": "pending", "phase": "K4", "source_decision": "D3", "modules": ["clawft-kernel/wasm_runner"]},
            {"id": "AC-5", "description": "Wasmtime activation + disk cache + WASI scope", "status": "pending", "phase": "K4", "source_decision": "D4", "modules": ["clawft-kernel/wasm_runner"]},
            {"id": "AC-6", "description": "Tree version history persistence", "status": "pending", "phase": "K4", "source_decision": "D7", "modules": ["clawft-kernel/tree_manager"]},
            {"id": "AC-7", "description": "CA chain signing for tools", "status": "pending", "phase": "K4", "source_decision": "D9", "modules": ["clawft-kernel/wasm_runner"]},
        ],
    }


def _ecc_symposium():
    return {
        "id": "ecc",
        "date": "2026-03-22",
        "decisions": [
            {"id": "D1", "title": "Nervous system model: WeftOS is a cognitive platform", "rationale": "Every kernel instance is a node in a distributed nervous system, from ESP32 to Blackwell.", "panel": "All Panels", "status": "implemented"},
            {"id": "D2", "title": "Forest of trees, not one graph (polyglot tree ensemble)", "rationale": "Multiple domain-specific structures linked by CrossRefs and Impulses.", "panel": "All Panels", "status": "implemented"},
            {"id": "D3", "title": "Self-calibrating cognitive tick at boot", "rationale": "Tick interval hardware-determined, auto-adjusted, advertised to peers.", "panel": "User Insight", "status": "implemented"},
            {"id": "D4", "title": "CRDTs for convergence, Merkle for verification", "rationale": "Complementary: CRDTs ensure agreement, Merkle proves honest agreement.", "panel": "User Insight", "status": "implemented"},
            {"id": "D5", "title": "DEMOCRITUS as continuous nervous system operation", "rationale": "Not batch; 30-second micro-batches distributed across network.", "panel": "User Insight", "status": "pending"},
            {"id": "D6", "title": "BLAKE3 forward, SHAKE-256 present", "rationale": "New ECC code uses BLAKE3; existing ExoChain keeps SHAKE-256 until K6 migration.", "panel": "Research Panel", "status": "implemented"},
            {"id": "D7", "title": "Per-tree scoring with uniform CrossRef indexing", "rationale": "Each tree defines own N-dimensional scoring; CrossRef indexing is universal via BLAKE3 hashes.", "panel": "User Insight", "status": "implemented"},
            {"id": "D8", "title": "One feature flag (ecc), boot decides what is active", "rationale": "Build provides capability, boot provides configuration.", "panel": "User Insight", "status": "implemented"},
        ],
        "commitments": [],
    }


def _k5_symposium():
    return {
        "id": "k5",
        "date": "2026-03-25",
        "decisions": [
            {"id": "D1", "title": "Selective composition: snow + quinn + selective libp2p", "rationale": "Full libp2p too heavy (~80 deps); iroh QUIC-only. Selective composition with tokio control.", "panel": "Mesh Architecture", "status": "implemented"},
            {"id": "D2", "title": "Ed25519 public key as node identity", "rationale": "No CA needed; node authenticates by signing challenge. Consistent with chain.rs signing.", "panel": "Security", "status": "implemented"},
            {"id": "D3", "title": "Noise Protocol for all inter-node encryption (XX/IK)", "rationale": "Transport-agnostic, forward secrecy, mutual auth, no X.509.", "panel": "Security", "status": "implemented"},
            {"id": "D4", "title": "governance.genesis as cluster trust root", "rationale": "Genesis event already exists; its SHAKE-256 hash = cluster ID.", "panel": "Readiness Audit", "status": "implemented"},
            {"id": "D5", "title": "Feature-gated mesh networking (mesh/mesh-discovery/mesh-full)", "rationale": "Zero networking code in default build. Follows existing ecc/exochain/cluster pattern.", "panel": "All Panels", "status": "implemented"},
            {"id": "D6", "title": "QUIC primary transport, WebSocket browser fallback", "rationale": "QUIC for multiplexing/migration; browsers cannot use QUIC directly.", "panel": "Mesh Architecture", "status": "implemented"},
            {"id": "D7", "title": "Ruvector algorithms as pure computation (no I/O)", "rationale": "Mesh layer provides I/O; ruvector crates produce/consume messages only.", "panel": "Ruvector Inventory", "status": "implemented"},
            {"id": "D8", "title": "rvf-wire as mesh wire format", "rationale": "Already in workspace, zero-copy deserialization, avoids new serialization dep.", "panel": "Mesh Architecture", "status": "implemented"},
            {"id": "D9", "title": "Dual signing (Ed25519 + ML-DSA-65) for cross-node chain events", "rationale": "Quantum resistance today + backward compat. Already implemented in chain.rs.", "panel": "Security", "status": "implemented"},
            {"id": "D10", "title": "6-phase implementation (K6.0-K6.5)", "rationale": "Each sub-phase independently testable. Prep phase (K6.0) has zero new deps.", "panel": "All Panels", "status": "implemented"},
            {"id": "D11", "title": "Hybrid Noise + ML-KEM-768 post-quantum key exchange", "rationale": "Protects against store-now-decrypt-later. ~2.4KB extra per handshake, ~1ms latency.", "panel": "Security", "status": "implemented"},
            {"id": "D12", "title": "Layered service resolution with genesis-hash DHT keys", "rationale": "9-step flow: resolution cache, negative cache, connection pool, circuit breaker.", "panel": "Mesh Architecture", "status": "implemented"},
            {"id": "D13", "title": "Mesh RPCs reuse ServiceApi adapter pattern", "rationale": "MeshAdapter feeds through A2ARouter + ServiceApi. All services remotely callable.", "panel": "Mesh Architecture", "status": "implemented"},
            {"id": "D14", "title": "CMVG cognitive sync via multiplexed QUIC streams", "rationale": "Chain replication carries ~80% CMVG sync. Dedicated cognitive streams in K7.", "panel": "CMVG Architecture", "status": "implemented"},
            {"id": "D15", "title": "Sync framing, stream priority, delta computation, observability", "rationale": "RVF wire segments, QUIC priority Chain>Tree>IPC>Cognitive>Impulse, PeerMetrics.", "panel": "K6 SPARC", "status": "implemented"},
        ],
        "commitments": [
            {"id": "C1", "description": "MessageTarget::RemoteNode variant", "status": "implemented", "phase": "K6.0", "source_decision": "D10", "modules": ["clawft-kernel/ipc"]},
            {"id": "C2", "description": "GlobalPid composite identifier", "status": "implemented", "phase": "K6.0", "source_decision": "D10", "modules": ["clawft-kernel/ipc"]},
            {"id": "C3", "description": "MeshTransport trait (transport-agnostic)", "status": "implemented", "phase": "K6.1", "source_decision": "D1", "modules": ["clawft-kernel/mesh"]},
            {"id": "C4", "description": "mesh feature gate structure", "status": "implemented", "phase": "K6.0", "source_decision": "D5", "modules": ["clawft-kernel/Cargo.toml"]},
            {"id": "C5", "description": "Cluster-join authentication protocol", "status": "implemented", "phase": "K6.0", "source_decision": "D4", "modules": ["clawft-kernel/cluster"]},
        ],
    }


def _phases():
    return [
        {
            "id": "K0",
            "title": "Kernel Foundation",
            "goal": "Create clawft-kernel crate with boot, process table, service registry, health, cluster membership",
            "status": "complete",
            "exit_criteria_total": 20,
            "exit_criteria_checked": 20,
            "depends_on": [],
            "deliverables": ["Kernel<P> struct", "ProcessTable", "ServiceRegistry", "HealthSystem", "KernelConsole", "ExoChain local chain", "TreeManager", "ClusterMembership"],
        },
        {
            "id": "K1",
            "title": "Supervisor + RBAC + ExoChain Integration",
            "goal": "Wire agent supervisor to execute agents, enforce RBAC, integrate with resource tree and chain",
            "status": "complete",
            "exit_criteria_total": 20,
            "exit_criteria_checked": 20,
            "depends_on": ["K0"],
            "deliverables": ["spawn_and_run()", "GateBackend trait", "Chain persistence", "Agent tree nodes", "IPC RBAC", "weaver agent CLI"],
        },
        {
            "id": "K2",
            "title": "Agent-to-Agent IPC",
            "goal": "Implement A2A messaging with typed envelopes, pub/sub topic routing, IPC scope enforcement",
            "status": "complete",
            "exit_criteria_total": 15,
            "exit_criteria_checked": 15,
            "depends_on": ["K1"],
            "deliverables": ["A2AProtocol", "TopicRouter", "JSON-RPC wire format", "RVF deep integration"],
        },
        {
            "id": "K2b",
            "title": "Kernel Work-Loop Hardening",
            "goal": "Close 6 gaps: health monitor, watchdog, graceful shutdown, resource tracking, suspend/resume, gate enforcement",
            "status": "complete",
            "exit_criteria_total": 9,
            "exit_criteria_checked": 9,
            "depends_on": ["K2"],
            "deliverables": ["Health monitor loop", "Agent watchdog sweep", "Graceful shutdown", "Resource usage tracking", "Suspend/resume", "Gate-checked commands"],
        },
        {
            "id": "K2.1",
            "title": "Symposium Implementation",
            "goal": "Implement K2 Symposium breaking changes: SpawnBackend, dual signing, IPC restructure, ServiceEntry",
            "status": "complete",
            "exit_criteria_total": 28,
            "exit_criteria_checked": 28,
            "depends_on": ["K2b"],
            "deliverables": ["SpawnBackend enum (C1+C8)", "MessageTarget restructure (D19)", "ServiceEntry (D1)", "GovernanceGate verification"],
        },
        {
            "id": "K3",
            "title": "WASM Tool Sandboxing / Tool Lifecycle",
            "goal": "27-tool catalog, 2 reference impls, Build->Deploy->Execute->Version->Revoke lifecycle with ExoChain audit",
            "status": "complete",
            "exit_criteria_total": 20,
            "exit_criteria_checked": 19,
            "depends_on": ["K2.1"],
            "deliverables": ["27-tool catalog", "FsReadFileTool", "AgentSpawnTool", "ToolRegistry", "WASM validation", "Tool lifecycle with chain events"],
        },
        {
            "id": "K3c",
            "title": "ECC Integration -- Cognitive Substrate",
            "goal": "Integrate Ephemeral Causal Cognition into kernel: CausalGraph, HnswService, CognitiveTick, CrossRefs, Impulses",
            "status": "complete",
            "exit_criteria_total": 15,
            "exit_criteria_checked": 15,
            "depends_on": ["K3"],
            "deliverables": ["CausalGraph DAG", "HnswService", "CognitiveTick", "Calibration", "CrossRef store", "Impulse queue"],
        },
        {
            "id": "K4",
            "title": "Container Integration",
            "goal": "Container-based sidecar service orchestration with Alpine image, Docker integration, Wasmtime activation",
            "status": "in-progress",
            "exit_criteria_total": 15,
            "exit_criteria_checked": 13,
            "depends_on": ["K3"],
            "deliverables": ["ContainerManager", "Docker/Podman integration", "Hierarchical ToolRegistry", "25 tool implementations", "Wasmtime integration"],
        },
        {
            "id": "K5",
            "title": "Application Framework",
            "goal": "Application manifests, lifecycle management, external framework interop, clustering",
            "status": "in-progress",
            "exit_criteria_total": 17,
            "exit_criteria_checked": 16,
            "depends_on": ["K3", "K4"],
            "deliverables": ["AppManager", "AppManifest", "ServiceApi trait", "Crypto-signed app bundles", "Cross-node service discovery"],
        },
        {
            "id": "K6",
            "title": "Transport-Agnostic Encrypted Mesh Network",
            "goal": "Multi-node cluster with encrypted P2P networking, distributed IPC, chain replication, tree sync, service discovery",
            "status": "complete",
            "exit_criteria_total": 48,
            "exit_criteria_checked": 48,
            "depends_on": ["K0", "K1", "K2", "K5"],
            "deliverables": ["MeshTransport trait", "QUIC transport", "Noise encryption", "Kademlia DHT", "mDNS discovery", "Chain replication", "Tree sync", "CRDT process table", "SWIM heartbeats", "Hybrid KEM upgrade"],
        },
        {
            "id": "K8",
            "title": "OS Gap Filling (Self-Healing + IPC + Content Ops)",
            "goal": "Self-healing supervisor, dead letter queue, named pipes, metrics, logging, artifact store, Weaver engine, config/secret/auth services",
            "status": "planned",
            "exit_criteria_total": 117,
            "exit_criteria_checked": 0,
            "depends_on": ["K6"],
            "deliverables": ["Self-healing supervisor", "Dead letter queue", "Named pipes", "Metrics registry", "Log service", "Artifact store", "WeaverEngine", "Config/Secret/Auth services"],
        },
    ]


def _decision_module_links():
    """Edges from decisions to the modules they caused changes in."""
    return [
        # K2 symposium decisions -> modules
        {"from": "decision:k2:D1", "to": "clawft-kernel/service", "weight": 1.0},
        {"from": "decision:k2:D3", "to": "clawft-kernel/supervisor", "weight": 1.0},
        {"from": "decision:k2:D4", "to": "clawft-kernel/service", "weight": 1.0},
        {"from": "decision:k2:D7", "to": "clawft-kernel/a2a", "weight": 0.9},
        {"from": "decision:k2:D7", "to": "clawft-kernel/gate", "weight": 0.9},
        {"from": "decision:k2:D8", "to": "clawft-kernel/chain", "weight": 0.9},
        {"from": "decision:k2:D10", "to": "clawft-kernel/wasm_runner", "weight": 0.8},
        {"from": "decision:k2:D14", "to": "clawft-kernel/supervisor", "weight": 0.7},
        {"from": "decision:k2:D16", "to": "clawft-kernel/ipc", "weight": 0.8},
        {"from": "decision:k2:D19", "to": "clawft-kernel/ipc", "weight": 1.0},
        {"from": "decision:k2:D20", "to": "clawft-kernel/governance", "weight": 0.8},
        # K3 symposium decisions -> modules
        {"from": "decision:k3:D1", "to": "clawft-kernel/wasm_runner", "weight": 1.0},
        {"from": "decision:k3:D2", "to": "clawft-kernel/agent_loop", "weight": 1.0},
        {"from": "decision:k3:D4", "to": "clawft-kernel/wasm_runner", "weight": 0.9},
        {"from": "decision:k3:D10", "to": "clawft-kernel/service", "weight": 1.0},
        {"from": "decision:k3:D12", "to": "clawft-kernel/wasm_runner", "weight": 0.9},
        # ECC symposium decisions -> modules
        {"from": "decision:ecc:D1", "to": "clawft-kernel/cognitive_tick", "weight": 1.0},
        {"from": "decision:ecc:D2", "to": "clawft-kernel/causal", "weight": 1.0},
        {"from": "decision:ecc:D2", "to": "clawft-kernel/crossref", "weight": 1.0},
        {"from": "decision:ecc:D2", "to": "clawft-kernel/impulse", "weight": 1.0},
        {"from": "decision:ecc:D3", "to": "clawft-kernel/cognitive_tick", "weight": 1.0},
        {"from": "decision:ecc:D3", "to": "clawft-kernel/calibration", "weight": 1.0},
        {"from": "decision:ecc:D4", "to": "clawft-kernel/crossref", "weight": 0.8},
        {"from": "decision:ecc:D6", "to": "clawft-kernel/causal", "weight": 0.7},
        {"from": "decision:ecc:D7", "to": "clawft-kernel/crossref", "weight": 0.9},
        {"from": "decision:ecc:D8", "to": "clawft-kernel/boot", "weight": 0.8},
        # K5 symposium decisions -> modules
        {"from": "decision:k5:D1", "to": "clawft-kernel/mesh", "weight": 1.0},
        {"from": "decision:k5:D2", "to": "clawft-kernel/cluster", "weight": 1.0},
        {"from": "decision:k5:D3", "to": "clawft-kernel/mesh_noise", "weight": 1.0},
        {"from": "decision:k5:D4", "to": "clawft-kernel/cluster", "weight": 0.9},
        {"from": "decision:k5:D5", "to": "clawft-kernel/Cargo.toml", "weight": 0.8},
        {"from": "decision:k5:D6", "to": "clawft-kernel/mesh_quic", "weight": 1.0},
        {"from": "decision:k5:D6", "to": "clawft-kernel/mesh_ws", "weight": 0.9},
        {"from": "decision:k5:D8", "to": "clawft-kernel/mesh_framing", "weight": 0.9},
        {"from": "decision:k5:D10", "to": "clawft-kernel/mesh", "weight": 0.8},
        {"from": "decision:k5:D11", "to": "clawft-kernel/mesh_noise", "weight": 0.9},
        {"from": "decision:k5:D12", "to": "clawft-kernel/mesh_service", "weight": 1.0},
        {"from": "decision:k5:D13", "to": "clawft-kernel/mesh_adapter", "weight": 1.0},
        {"from": "decision:k5:D14", "to": "clawft-kernel/mesh_chain", "weight": 0.8},
        {"from": "decision:k5:D14", "to": "clawft-kernel/mesh_tree", "weight": 0.8},
        {"from": "decision:k5:D15", "to": "clawft-kernel/mesh_framing", "weight": 0.9},
    ]


def _symposium_gap_links():
    """Edges from symposium phases to gap analyses."""
    return [
        {"from": "phase:K2.1", "to": "gap-analysis:k2-symposium", "weight": 0.8},
        {"from": "phase:K3c", "to": "gap-analysis:ecc-symposium", "weight": 0.8},
        {"from": "phase:K6", "to": "gap-analysis:k5-symposium", "weight": 0.8},
    ]


if __name__ == "__main__":
    json.dump(build_data(), sys.stdout, indent=2)
    print()
