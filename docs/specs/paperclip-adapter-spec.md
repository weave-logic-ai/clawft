# Paperclip Adapter Specification

**Status**: Draft
**Sprint**: 13 (D2, D3)
**Package**: `@paperclipai/adapter-weftos`

## Overview

This document specifies the TypeScript adapter that integrates Paperclip's
agent orchestration platform with the WeftOS kernel. The adapter implements
Paperclip's heartbeat protocol and delegates task execution and governance
decisions to WeftOS via its HTTP API.

```
Paperclip CEO/Manager/Worker agents
    |
    v
@paperclipai/adapter-weftos  (this adapter)
    |
    v  HTTP JSON
WeftOS kernel  /api/v1/*
    |
    v
clawft 7-stage pipeline + governance engine
```

---

## 1. Adapter Interface

### TypeScript Definition

```typescript
import type { AgentAdapter, HeartbeatContext, TaskResult } from "@paperclipai/plugin-sdk";

export interface WeftOSAdapterConfig {
  /** WeftOS kernel HTTP base URL (default: http://127.0.0.1:9820) */
  baseUrl: string;

  /** Authentication token — API key or WeftOS capability token */
  token: string;

  /** Request timeout in milliseconds (default: 30000) */
  timeoutMs?: number;

  /** Whether to call /api/v1/govern before every task execution */
  governanceEnabled?: boolean;

  /** Retry count on transient failures (default: 2) */
  retries?: number;
}

export class WeftOSAdapter implements AgentAdapter {
  constructor(config: WeftOSAdapterConfig);

  /** Called by Paperclip on each heartbeat cycle. */
  async heartbeat(ctx: HeartbeatContext): Promise<void>;

  /** Execute a task via WeftOS pipeline. */
  async execute(agentId: string, task: string, context: Record<string, unknown>): Promise<TaskResult>;

  /** Governance check — can this agent perform this action? */
  async govern(agentId: string, action: string, context: Record<string, unknown>): Promise<GovernResult>;

  /** Health check — is WeftOS reachable? */
  async ping(): Promise<boolean>;
}

export interface GovernResult {
  decision: "permit" | "deny" | "escalate";
  attestation?: string;
  chainHash?: string;
  evaluatedRules: string[];
  effectMagnitude?: number;
}
```

---

## 2. Heartbeat Protocol

Paperclip agents operate on a heartbeat loop:

1. **Wake** -- Paperclip wakes the agent on its configured interval.
2. **Check queue** -- Paperclip provides pending tasks via `HeartbeatContext`.
3. **Governance gate** (optional) -- Adapter calls `POST /api/v1/govern` for
   each task. If the decision is `deny`, the task is marked as rejected. If
   `escalate`, it is returned to Paperclip for human approval.
4. **Execute** -- Adapter calls `POST /api/v1/execute` with the task payload.
5. **Report** -- Adapter returns the `TaskResult` to Paperclip.
6. **Sleep** -- Agent sleeps until the next heartbeat.

### Heartbeat Sequence Diagram

```
Paperclip           Adapter                WeftOS Kernel
   |                   |                       |
   |-- heartbeat() --> |                       |
   |                   |-- POST /govern -----> |
   |                   |<-- { decision } ----- |
   |                   |                       |
   |                   |-- POST /execute ----> |
   |                   |<-- { result } ------- |
   |                   |                       |
   |<-- TaskResult --- |                       |
```

---

## 3. Request / Response Formats

### 3.1 Execute Task

**Endpoint**: `POST /api/v1/execute`

**Request**:
```json
{
  "agent_id": "worker-analyst-01",
  "task": "Summarise Q1 revenue trends from the attached data.",
  "context": {
    "company_id": "acme-corp",
    "budget_remaining_usd": 12.50,
    "heartbeat_seq": 42
  }
}
```

**Response (success)**:
```json
{
  "result": "Q1 revenue grew 18% YoY driven by...",
  "score": 0.85,
  "tokens_used": 1234,
  "execution_id": "exec-a1b2c3d4"
}
```

**Response (error)**:
```json
{
  "code": "INVALID_AGENT_ID",
  "message": "agent_id must not be empty"
}
```

### 3.2 Governance Check

**Endpoint**: `POST /api/v1/govern`

**Request**:
```json
{
  "action": "write_file",
  "agent_id": "worker-analyst-01",
  "context": {
    "path": "/data/reports/q1.md",
    "risk": 0.3,
    "security": 0.1
  }
}
```

**Response**:
```json
{
  "decision": "permit",
  "attestation": "exo:sha256:abc123...",
  "chain_hash": "0xdeadbeef...",
  "evaluated_rules": ["rule-fs-write", "rule-data-access"],
  "effect_magnitude": 0.316
}
```

### 3.3 Health Check

**Endpoint**: `GET /api/v1/health`

**Response**:
```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 3600
}
```

---

## 4. Authentication

The adapter supports two authentication modes:

### 4.1 API Key

A shared secret passed as a Bearer token in the `Authorization` header.
Suitable for single-machine deployments where Paperclip and WeftOS run
on the same host.

```
Authorization: Bearer weftos_sk_live_abc123...
```

### 4.2 Kernel Capability Token

A WeftOS capability token issued by the kernel's `AuthService`. This token
encodes the agent's permissions and is validated against the kernel's
capability model. Preferred for multi-tenant or networked deployments.

```
Authorization: WeftOS-Cap eyJhZ2VudF9pZCI6...
```

The kernel validates the token and enforces capability restrictions before
processing the request. If the token lacks the required capability, the
request is rejected with a `403` response.

---

## 5. Error Handling

### HTTP Status Codes

| Status | Meaning |
|--------|---------|
| 200 | Success |
| 400 | Invalid request (parse error, missing fields) |
| 401 | Authentication failed |
| 403 | Insufficient capabilities |
| 404 | Unknown route |
| 429 | Rate limited |
| 500 | Internal kernel error |
| 503 | Kernel not ready (boot in progress) |

### Retry Strategy

The adapter retries on `429` and `503` with exponential backoff:

- Attempt 1: immediate
- Attempt 2: 500ms delay
- Attempt 3: 2000ms delay
- After 3 failures: propagate error to Paperclip

Transient network errors (connection refused, timeout) also trigger retries.

---

## 6. Paperclip Configuration

### company.json

```json
{
  "name": "Acme Corp",
  "agents": [
    {
      "role": "analyst",
      "runtime": "weftos",
      "config": {
        "baseUrl": "http://127.0.0.1:9820",
        "token": "${WEFTOS_API_KEY}",
        "governanceEnabled": true,
        "timeoutMs": 30000
      }
    }
  ],
  "runtimes": {
    "weftos": {
      "adapter": "@paperclipai/adapter-weftos",
      "version": "^0.1.0"
    }
  }
}
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `WEFTOS_API_KEY` | Yes | API key or capability token |
| `WEFTOS_BASE_URL` | No | Override kernel URL (default: `http://127.0.0.1:9820`) |
| `WEFTOS_GOVERNANCE` | No | Enable governance gate (`true`/`false`, default: `true`) |

---

## 7. Governance Bridge (D3)

### How Paperclip Approval Gates Map to WeftOS Policy Kernel

Paperclip has a built-in approval gate system where managers can require
human sign-off before a worker agent executes a task. WeftOS extends this
with cryptographic governance:

| Paperclip Concept | WeftOS Mapping |
|-------------------|----------------|
| Approval gate (manager) | `GovernanceEngine::evaluate()` |
| Budget check | Effect vector `risk` dimension |
| Role-based permission | Kernel capability model |
| Audit log | ExoChain provenance record |
| Human escalation | `GovernanceDecision::EscalateToHuman` |

### Governance Flow

1. Paperclip triggers a task for a worker agent.
2. The adapter calls `POST /api/v1/govern` with the action and context.
3. WeftOS evaluates the request against:
   - Legislative rules (SOPs, manifests)
   - Executive policies (agent capabilities)
   - Judicial validation (CGR engine, effect algebra)
4. The 5-dimensional effect vector (risk, fairness, privacy, novelty,
   security) is scored. If the magnitude exceeds the environment's
   threshold, the action is denied or escalated.
5. The decision is returned with:
   - `decision`: `"permit"`, `"deny"`, or `"escalate"`
   - `attestation`: cryptographic hash of the decision record
   - `chain_hash`: ExoChain block hash linking this decision to the
     immutable audit trail

### ExoChain Logging of Paperclip Decisions

Every governance decision from a Paperclip-originated request is logged
to the ExoChain as a `GovernanceDecisionEvent`:

```rust
ChainEvent::GovernanceDecision {
    agent_id: String,           // Paperclip agent identifier
    action: String,             // Proposed action
    decision: GovernanceDecision,
    effect_magnitude: f64,
    source: "paperclip",        // Distinguishes from internal requests
    paperclip_company: Option<String>,
    paperclip_heartbeat_seq: Option<u64>,
    timestamp: DateTime<Utc>,
}
```

This ensures that:
- All Paperclip governance decisions have cryptographic provenance.
- Decisions can be audited by querying the ExoChain with
  `source = "paperclip"`.
- The chain hash in the response allows Paperclip's dashboard to link
  back to the WeftOS audit trail.

### Governance Request Format

**Endpoint**: `POST /api/v1/govern`

**Request**:
```json
{
  "action": "deploy_to_production",
  "agent_id": "worker-deployer-01",
  "context": {
    "risk": 0.8,
    "security": 0.6,
    "novelty": 0.2,
    "paperclip_company": "acme-corp",
    "paperclip_heartbeat_seq": 42,
    "approval_chain": ["manager-ops-01"]
  }
}
```

**Response** (escalation due to high risk):
```json
{
  "decision": "escalate",
  "attestation": "exo:sha256:9f3a...",
  "chain_hash": "0x1a2b3c4d...",
  "evaluated_rules": ["rule-prod-deploy", "rule-high-risk"],
  "effect_magnitude": 1.02
}
```

---

## 8. Future Work

- **Memory bridge**: Replace Paperclip's PARA file memory with WeftOS HNSW
  vector store, enabling semantic search across heartbeat history.
- **Budget enforcement**: Surface WeftOS cost tracking data in Paperclip's
  budget dashboard.
- **Multi-company isolation**: Map Paperclip companies to WeftOS kernel
  namespaces for tenant isolation.
- **Plugin marketplace**: Publish the adapter to Paperclip's Clipmart.
