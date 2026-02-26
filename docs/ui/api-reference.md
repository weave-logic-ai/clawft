# API Reference

> REST and WebSocket endpoints for the ClawFT Axum backend.

All REST endpoints are prefixed with `/api`. Authentication is via Bearer token in the `Authorization` header. WebSocket endpoints are at `/ws`.

## Authentication

### Create Token

```
POST /api/auth/token
```

Creates a new Bearer token for API authentication.

**Response:**

```json
{ "token": "clawft_xxxxxxxxxx" }
```

**Example:**

```bash
curl -X POST http://localhost:18789/api/auth/token
```

### Verify Token

```
GET /api/auth/verify
```

Verify that the current Bearer token is valid.

**Headers:** `Authorization: Bearer <token>`

**Response:** `200 OK` if valid, `401 Unauthorized` if not.

---

## Agents

### List Agents

```
GET /api/agents
```

Returns all registered agents.

**Response:** `AgentSummary[]`

```json
[
  {
    "id": "agent-1",
    "name": "general-agent",
    "status": "running",
    "model": "claude-sonnet-4",
    "created_at": "2026-02-24T10:00:00Z"
  }
]
```

**TypeScript type:** `AgentSummary` from `lib/types.ts`

### Get Agent Detail

```
GET /api/agents/:id
```

Returns detailed information about a specific agent.

**Response:** `AgentDetail`

```json
{
  "id": "agent-1",
  "name": "general-agent",
  "status": "running",
  "model": "claude-sonnet-4",
  "created_at": "2026-02-24T10:00:00Z",
  "config": { "max_tokens": 4096 },
  "session_count": 3,
  "description": "General purpose assistant",
  "skills": ["code-review", "web-search"]
}
```

### Start Agent

```
POST /api/agents/:id/start
```

Start a stopped agent.

**Response:** `{ "ok": true }`

### Stop Agent

```
POST /api/agents/:id/stop
```

Stop a running agent.

**Response:** `{ "ok": true }`

---

## Sessions

### List Sessions

```
GET /api/sessions
```

**Response:** `SessionSummary[]`

```json
[
  {
    "key": "sess-abc-123",
    "agent_id": "agent-1",
    "message_count": 42,
    "updated_at": "2026-02-24T10:30:00Z"
  }
]
```

### Get Session Detail

```
GET /api/sessions/:key
```

Returns session metadata and all messages.

**Response:** `SessionDetail`

```json
{
  "key": "sess-abc-123",
  "agent_id": "agent-1",
  "message_count": 2,
  "updated_at": "2026-02-24T10:30:00Z",
  "messages": [
    { "role": "user", "content": "Hello", "timestamp": "2026-02-24T10:30:00Z" },
    { "role": "assistant", "content": "Hi!", "timestamp": "2026-02-24T10:30:01Z" }
  ]
}
```

### Create Session

```
POST /api/sessions
```

**Body:** `{ "agent_id": "agent-1" }`

**Response:** `SessionSummary`

### Export Session

```
GET /api/sessions/:key/export
```

**Response:** `{ "messages": ChatMessage[] }`

### Delete Session

```
DELETE /api/sessions/:key
```

**Response:** `{ "ok": true }`

### Send Message

```
POST /api/sessions/:key/messages
```

**Body:** `{ "content": "your message" }`

**Response:** `ChatMessage`

```json
{
  "role": "assistant",
  "content": "Response from the agent",
  "timestamp": "2026-02-24T10:30:01Z",
  "tool_calls": []
}
```

---

## Tools

### List Tools

```
GET /api/tools
```

**Response:** `ToolInfo[]`

```json
[
  { "name": "read_file", "description": "Read a file from the workspace" }
]
```

### Get Tool Schema

```
GET /api/tools/:name/schema
```

Returns the JSON Schema for a tool's parameters.

**Response:** `Record<string, unknown>` (JSON Schema object)

---

## Skills

### List Installed Skills

```
GET /api/skills
```

**Response:** `SkillData[]`

```json
[
  {
    "name": "code-review",
    "version": "1.2.0",
    "description": "Automated code review",
    "author": "clawft-team",
    "tags": ["review", "security"],
    "installed": true,
    "enabled": true
  }
]
```

### Install Skill

```
POST /api/skills/install
```

**Body:** `{ "id": "reg-001" }`

**Response:** `{ "success": true }`

### Uninstall Skill

```
DELETE /api/skills/:name
```

**Response:** `{ "success": true }`

### Search Registry

```
GET /api/skills/registry/search?q=deploy
```

**Response:** `RegistrySkill[]`

---

## Memory

### List Entries

```
GET /api/memory
```

**Response:** `MemoryEntry[]`

```json
[
  {
    "key": "pattern-auth",
    "value": "JWT with refresh tokens",
    "namespace": "patterns",
    "tags": ["auth", "jwt"],
    "updated_at": "2026-02-24T10:30:00Z"
  }
]
```

### Search Memory

```
GET /api/memory/search?q=auth&threshold=0.7
```

Returns entries matching the query with similarity scores.

**Response:** `MemoryEntry[]` (with `similarity` field populated)

### Create Entry

```
POST /api/memory
```

**Body:**

```json
{
  "key": "my-key",
  "value": "my value",
  "namespace": "default",
  "tags": ["tag1", "tag2"]
}
```

**Response:** `MemoryEntry`

### Delete Entry

```
DELETE /api/memory/:key
```

**Response:** `{ "success": true }`

---

## Config

### Get Configuration

```
GET /api/config
```

**Response:** `ConfigData`

```json
{
  "agents": { "defaults": { "model": "claude-sonnet-4", "max_tokens": 4096, "temperature": 0.7 } },
  "providers": { "anthropic": { "api_key_set": true, "api_base": "https://api.anthropic.com", "enabled": true } },
  "channels": { "discord": { "enabled": true } },
  "gateway": { "api_port": 3100, "api_enabled": true }
}
```

### Update Configuration

```
PUT /api/config
```

**Body:** Full `ConfigData` object (replaces existing configuration).

**Response:** `{ "success": true }`

---

## Cron

### List Jobs

```
GET /api/cron
```

**Response:** `CronJob[]`

### Create Job

```
POST /api/cron
```

**Body:**

```json
{
  "name": "my-job",
  "schedule": "0 * * * *",
  "enabled": true
}
```

**Response:** `CronJob`

### Update Job

```
PUT /api/cron/:id
```

**Body:** Partial `CronJob` fields to update.

### Delete Job

```
DELETE /api/cron/:id
```

**Response:** `{ "success": true }`

### Run Job Now

```
POST /api/cron/:id/run
```

**Response:** `{ "success": true }`

---

## Channels

### List Channels

```
GET /api/channels
```

**Response:** `ChannelStatus[]`

```json
[
  {
    "name": "discord-main",
    "type": "discord",
    "status": "connected",
    "message_count": 1247,
    "last_activity": "2026-02-24T10:28:00Z",
    "routes_to": "general-agent"
  }
]
```

---

## Delegation

### List Active Delegations

```
GET /api/delegation/active
```

**Response:** `ActiveDelegation[]`

```json
[
  {
    "task_id": "del-001",
    "session_key": "sess-abc-123",
    "target": "claude-sonnet-4",
    "status": "running",
    "started_at": "2026-02-24T10:30:00Z",
    "latency_ms": 1250,
    "tool_name": "code-review",
    "complexity": 0.72
  }
]
```

### List Delegation Rules

```
GET /api/delegation/rules
```

**Response:** `DelegationRule[]`

### Upsert Delegation Rule

```
PATCH /api/delegation/rules
```

**Body:** `DelegationRule`

**Response:** `DelegationRule`

### Delete Delegation Rule

```
DELETE /api/delegation/rules/:name
```

**Response:** `{ "success": true }`

### Delegation History

```
GET /api/delegation/history?session=sess-abc&target=claude&offset=0&limit=50
```

All query parameters are optional.

**Response:**

```json
{
  "items": [DelegationHistoryEntry],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```

---

## Monitoring

### Token Usage

```
GET /api/monitoring/token-usage
```

**Response:** `TokenUsageSummary`

```json
{
  "total_input": 334000,
  "total_output": 113000,
  "total_requests": 460,
  "by_provider": [
    { "provider": "anthropic", "model": "claude-sonnet-4", "input_tokens": 245000, "output_tokens": 82000, "total_tokens": 327000, "request_count": 142 }
  ],
  "by_session": [
    { "session_key": "sess-abc-123", "input_tokens": 180000, "output_tokens": 65000, "request_count": 95 }
  ]
}
```

### Cost Breakdown

```
GET /api/monitoring/costs
```

**Response:** `CostBreakdown`

```json
{
  "total_cost_usd": 2.018,
  "by_provider": [
    { "provider": "anthropic", "model": "claude-sonnet-4", "input_cost_usd": 0.735, "output_cost_usd": 1.23, "total_cost_usd": 1.965 }
  ],
  "by_tier": [
    { "tier": 1, "label": "Agent Booster (WASM)", "request_count": 1240, "total_cost_usd": 0 }
  ]
}
```

### Pipeline Runs

```
GET /api/monitoring/pipeline-runs
```

**Response:** `PipelineRun[]`

```json
[
  {
    "id": "run-001",
    "session_key": "sess-abc-123",
    "model": "claude-sonnet-4",
    "complexity": 0.72,
    "latency_ms": 3200,
    "status": "success",
    "timestamp": "2026-02-24T10:30:00Z"
  }
]
```

---

## Voice

### Get Voice Status

```
GET /api/voice/status
```

**Response:**

```json
{
  "state": "idle",
  "talkModeActive": false,
  "wakeWordEnabled": false,
  "settings": {
    "enabled": false,
    "wakeWordEnabled": false,
    "language": "en",
    "echoCancel": true,
    "noiseSuppression": true,
    "pushToTalk": false
  }
}
```

### Update Voice Settings

```
PUT /api/voice/settings
```

**Body:** Partial `VoiceSettingsData`

### Test Microphone

```
POST /api/voice/test-mic
```

**Response:** `{ "success": true, "level": 0.6 }`

### Test Speaker

```
POST /api/voice/test-speaker
```

**Response:** `{ "success": true }`

---

## Health

### Health Check

```
GET /api/health
```

**Response:**

```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "memory_usage_mb": 128,
  "agent_count": 3,
  "session_count": 12,
  "tool_count": 8
}
```

---

## WebSocket

### Connection

```
ws://localhost:18789/ws
```

The WebSocket connection supports topic-based pub/sub messaging.

### Subscribe to Topics

```json
{ "type": "subscribe", "topic": "agent:status" }
```

### Unsubscribe

```json
{ "type": "unsubscribe", "topic": "agent:status" }
```

### Event Types

| Topic | Description | Payload |
|-------|-------------|---------|
| `agent:status` | Agent status changes | `{ agentId, status }` |
| `chat:message` | New chat message | `ChatMessage` |
| `canvas:update` | Canvas element update | `CanvasCommandData` |
| `pipeline:stage` | Pipeline stage progress | `{ taskId, stage, status, durationMs }` |
| `delegation:update` | Delegation status change | `ActiveDelegation` |

### Specialized WebSocket Endpoints

| Endpoint | Description |
|----------|-------------|
| `/ws/canvas` | Canvas-specific real-time updates |
| `/ws/chat/:session` | Session-specific chat stream |
