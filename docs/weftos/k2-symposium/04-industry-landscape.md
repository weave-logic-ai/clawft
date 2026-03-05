# Industry Landscape: Agent Protocols, OS Patterns, and Cryptographic Contracting

**Presenter**: Research Analyst
**Sources**: 25+ industry papers, specifications, and blog posts (cited inline)

---

## 1. Google A2A Protocol (Agent-to-Agent)

### Overview

Google launched A2A v0.3 (July 2025) as an open standard for agent
interoperability, backed by 150+ organizations. Now governed by the
Linux Foundation.

**Core Components:**
- Agent Cards: JSON discovery at `/.well-known/agent.json`
- Message Format: JSON-RPC 2.0 over HTTP + SSE
- Authentication: OpenAPI-compatible (OAuth, API Keys)
- Four Capabilities: Discovery, task management, collaboration, UX negotiation

### WeftOS Comparison

| Feature | Google A2A | WeftOS A2A |
|---------|-----------|------------|
| Transport | HTTP/SSE | Kernel IPC (mpsc) |
| Discovery | Agent Cards (JSON) | Capability metadata |
| Auth | OAuth/API Keys | Capability checker |
| Audit | None built-in | ExoChain (every message) |
| Governance | None built-in | 5D effect algebra |
| Latency | Network (ms-s) | In-process (us) |
| Scope | Cross-network | Same kernel |

**Opportunity**: Build an A2A-to-WeftOS gateway that translates HTTP/JSON-RPC
messages into kernel IPC messages. Auto-generate Agent Cards from WeftOS
capability metadata.

**Differentiation**: WeftOS's kernel-level integration offers sub-millisecond
latency with built-in cryptographic provenance -- something A2A doesn't provide.

## 2. Anthropic MCP (Model Context Protocol)

### Overview

MCP (Nov 2024, donated to Linux Foundation Dec 2025) provides three
primitives for tool/resource access:
- **Tools**: Executable functions with typed input schemas
- **Resources**: Data sources providing contextual information
- **Prompts**: Reusable templates for LLM interactions

### WeftOS Comparison

| Feature | Anthropic MCP | WeftOS Tools |
|---------|--------------|--------------|
| Protocol | JSON-RPC | Kernel IPC |
| Tool Schema | Type-validated | ToolSpec in manifest |
| Security | Server boundaries | GovernanceGate |
| Audit | None built-in | Chain-logged tool calls |
| Hosting | Separate process | In-kernel or WASM sandbox |

**Opportunity**: Implement WeftOS as both MCP client (consuming external
tools) and MCP server (exposing kernel tools). The ruv ecosystem includes
`mcp-gate` which could expose WeftOS's coherence gate as MCP tools.

## 3. Agent OS Patterns

### Industry Approaches (2026)

- **VAST AgentEngine**: Containerized runtimes, lifecycle management, MCP integration
- **PubMatic AgenticOS**: System-level orchestration for agent commerce
- **AgentOps Framework**: Observability, governance, traceability

### Common Patterns

1. **Lifecycle Management**: Boot, spawn, suspend, resume, stop
2. **Sandboxing**: Containers, microVMs, WASM for isolation
3. **Resource Scheduling**: CPU/memory allocation per agent
4. **Observability**: Metrics, logs, traces across distributed agents
5. **Governance**: Policy enforcement at the OS layer

### WeftOS Advantage

WeftOS implements all five patterns with deeper integration than any
commercial offering:
- Lifecycle via ProcessTable + AgentSupervisor
- Sandboxing via capabilities + WASM (K3) + containers (K4)
- Resource tracking via ResourceUsage (scheduling is future work)
- Observability via ExoChain + BootLog + HealthSystem
- Governance via GovernanceGate with 5D effect algebra

## 4. Cryptographic Contracting

### State of the Art

**Provably Honest Agents**: Policy adherence verified via R1CS/SNARK
constraint satisfaction proofs. Each agent execution produces a
deterministic receipt.

**Computable Contracts**: Outsourced computation where workers return
results with cryptographic proofs. Two approaches:
- Trusted Hardware (SGX, TrustZone)
- Zero-Knowledge Proofs (SNARKs, STARKs)

**Blockchain Integration**: Smart contract wallets for agent delegation.
Ed25519 signing as bearer authorization.

### WeftOS Comparison

| Feature | Industry | WeftOS ExoChain |
|---------|----------|-----------------|
| Hash Algorithm | SHA-256, Blake3 | SHAKE-256 (stronger) |
| Signatures | Ed25519, ECDSA | Ed25519 |
| Witness Chains | Custom per vendor | RVF format (standardized) |
| Zero-Knowledge | SNARKs/STARKs | Not yet |
| Hardware Enclaves | SGX, TrustZone | Not yet |
| Blockchain Anchor | Ethereum, Solana | Local only |
| Merkle Proofs | O(log n) | O(n) chain scan |

**Opportunity**: Add Merkle tree indexing for O(log n) proofs. Consider
SNARK integration for privacy-preserving governance. Add optional
blockchain anchoring for external verifiability.

## 5. Effect Algebra / Risk Assessment

### Industry Frameworks

- **NIST AI RMF**: Probability x magnitude across security, privacy,
  fairness, accountability
- **Multidimensional Risk**: 15+ dimensions including velocity, scope,
  reversibility, cascading effects
- **Systemic Risk**: Compounding, cascading, difficult-to-reverse impacts

### WeftOS EffectVector (5D)

```
risk:     probability of negative outcome [0.0, 1.0]
fairness: impact on equitable treatment
privacy:  data privacy impact
novelty:  how unprecedented the action is
security: system security impact
```

**Strengths**: Quantitative, compositional, fast (sub-ms), extensible.

**Gaps vs Industry**:
- No temporal dimensions (velocity, recurrence, time-to-impact)
- No cascading analysis (effects across system boundaries)
- Static thresholds (no adaptive learning)
- No uncertainty quantification (no confidence intervals)

**Opportunity**: Expand to 10D (add velocity, scope, reversibility,
uncertainty, cascading_risk). Use contextual thresholds per environment.
Track outcomes to tune scoring over time.

## 6. Append-Only Audit Logs

### Industry Approaches

- **Certificate Transparency** (RFC 6962): Merkle tree, public audit
- **Verifiable Audit Logs**: Agent-signed entries with SPIFFE IDs
- **Post-Quantum Considerations**: SPHINCS+, Dilithium for future-proofing

### WeftOS ExoChain Comparison

| Feature | Industry Best | WeftOS |
|---------|--------------|--------|
| Tamper Detection | Merkle tree | Hash chain |
| Proof Efficiency | O(log n) | O(n) |
| Public Verifiability | Public blockchain | Local only |
| Post-Quantum | SPHINCS+ available | Ed25519 only |
| Witness Bundles | Custom | RVF format |
| Lineage Tracking | N/A | DNA-style provenance |

**Unique WeftOS Features**: RVF witness bundles and lineage tracking
are not found in any commercial offering. The kernel-level integration
ensures 100% capture of agent actions.

## Summary: WeftOS Positioning

### Where WeftOS Leads

1. Kernel-level governance integration (unique in industry)
2. Constitutional three-branch model
3. RVF witness bundles with lineage tracking
4. Tight coupling of audit, governance, and IPC

### Where WeftOS Should Catch Up

1. Network protocols (A2A/MCP HTTP bridge)
2. Merkle tree indexing for efficient proofs
3. Post-quantum signature readiness
4. Public anchoring for external verifiability

### Where WeftOS Should Differentiate

1. Real-time kernel-level risk scoring (sub-ms)
2. Cryptographic provenance for every agent action
3. Multi-sandbox architecture (capabilities + WASM + containers)
4. Constitutional governance as type-level constraints
