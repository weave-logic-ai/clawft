# Phase S3: Polish + Advanced Features + Production + Browser WASM Integration

> **Element:** UI Development
> **Phase:** S3 (S3.1-S3.7)
> **Timeline:** Weeks 7-10
> **Priority:** P1 (delegation/monitoring), P1 (Tauri), P1 (production hardening), P1 (browser WASM), P2 (docs)
> **Crates:** `ui/src/` (React components), `clawft-services` (monitoring APIs), Tauri project, `clawft-wasm` (browser bridge)
> **Dependencies IN:** S1, S2 (complete dashboard), M1/M2 (FlowDelegator), D5/D6 (latency/cost), W-BROWSER Phase 5 (WASM entry points)
> **Blocks:** K6 (native shells), VS3 (voice UI integration via Canvas)
> **Status:** Planning

---

## 1. Overview

Phase S3 closes the UI development cycle with five parallel tracks: delegation monitoring and observability tooling (S3.1), advanced Canvas features (S3.2), production hardening (S3.3-S3.5), browser WASM integration (S3.6), and documentation (S3.7). This phase transforms the functional dashboard from S1/S2 into a production-grade application with mobile responsiveness, desktop distribution via Tauri, comprehensive security hardening, end-to-end test coverage, and a browser-only mode powered by the clawft WASM module.

S3 depends on the FlowDelegator subsystem (M1/M2) for delegation visibility, the latency recording pipeline (D5) for pipeline inspection, the cost attribution thread (D6) for per-user cost tracking, and the W-BROWSER workstream Phase 5 (WASM entry points) for browser-only mode. The Tauri desktop shell (S3.4) becomes the foundation for K6 cross-platform native shells. S3.6 is the critical bridge between W-BROWSER and W-UI workstreams, enabling the dashboard to run with no server at all.

---

## 2. Current Code

### Existing Dashboard (from S1/S2)

By the time S3 begins, the following are in place:

- `ui/src/` -- Full React + TypeScript + shadcn/ui project with Vite build
- `ui/src/routes/` -- Dashboard home, WebChat, agents, sessions, tools, skills, memory, config, cron, channels, canvas, delegation routes
- `ui/src/components/canvas/` -- Base CanvasRenderer with text, button, input, image, code, table element types
- `ui/src/hooks/use-websocket.ts` -- Reconnecting WebSocket client with topic subscription
- `ui/src/stores/` -- Zustand stores for agent, session, canvas, WebSocket state
- `ui/src/lib/api-client.ts` -- Fetch wrapper with Bearer token auth
- `ui/src/lib/ws-client.ts` -- WebSocket reconnecting client
- `ui/src/mocks/` -- MSW mock handlers for backend-independent development

### Existing Backend (from S1/S2 + dependency sprints)

- `clawft-services/src/api/` -- Axum router with REST endpoints for agents, sessions, tools, skills, memory, config, cron, channels
- WebSocket upgrade handler (`/ws`, `/ws/canvas`, `/ws/chat/:session`)
- Bearer token auth middleware
- CORS middleware via `tower-http::cors`

### No Existing Code for S3 Features

The following do not exist yet:
- Delegation monitoring UI components
- Pipeline stage inspector
- Token usage / cost tracking dashboards
- Chart / code editor / form Canvas element types
- Canvas history (undo/redo)
- PWA manifest and service worker
- Tauri project scaffold
- Production hardening middleware (CSP, rate limiting, heartbeat)
- Playwright E2E tests

---

## 3. Deliverables

### 3.0 Backend API Endpoints (Delegation & Monitoring)

Before specifying frontend components, this section defines the backend REST and WebSocket API contracts that S3.1 components consume. These endpoints are added to the existing Axum router in `clawft-services/src/api/`.

#### 3.0.1 Delegation REST Endpoints

**File:** `crates/clawft-services/src/api/delegation.rs` (NEW)

```rust
use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

/// Mount delegation API routes on the main router.
pub fn delegation_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/delegation/active", get(list_active_delegations))
        .route("/api/delegation/rules", get(list_delegation_rules))
        .route("/api/delegation/rules", patch(upsert_delegation_rule))
        .route("/api/delegation/rules/:name", delete(delete_delegation_rule))
        .route("/api/delegation/history", get(delegation_history))
        .route("/api/monitoring/token-usage", get(token_usage))
        .route("/api/monitoring/costs", get(cost_breakdown))
        .route("/api/monitoring/pipeline-runs", get(pipeline_runs))
}

// ── Request / Response Types ────────────────────────────────────────

/// Active delegation as returned by `GET /api/delegation/active`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDelegationResponse {
    pub task_id: String,
    pub session_key: String,
    pub target: String,           // "local" | "claude" | "flow"
    pub status: String,           // "pending" | "running" | "completed" | "failed"
    pub started_at: String,       // ISO 8601
    pub latency_ms: Option<u64>,
    pub tool_name: String,
    pub complexity: f32,          // 0.0 - 1.0
}

/// `GET /api/delegation/active` response.
#[derive(Debug, Serialize)]
pub struct ListActiveDelegationsResponse {
    pub delegations: Vec<ActiveDelegationResponse>,
    pub total_active: usize,
}

/// Delegation rule as stored in config and returned by `GET /api/delegation/rules`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationRulePayload {
    pub name: String,
    pub pattern: String,                    // Regex pattern
    pub target: String,                     // "local" | "claude" | "flow"
    pub complexity_threshold: f32,          // 0.0 - 1.0
    pub enabled: bool,
    pub priority: u32,                      // 0-100, lower = higher priority
}

/// `PATCH /api/delegation/rules` request body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertDelegationRuleRequest {
    pub name: String,
    pub pattern: String,
    pub target: String,
    pub complexity_threshold: f32,
    pub enabled: bool,
    pub priority: u32,
}

/// `GET /api/delegation/history` query parameters.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationHistoryQuery {
    /// Filter by session key (optional).
    pub session_key: Option<String>,
    /// Filter by delegation target (optional).
    pub target: Option<String>,
    /// Maximum number of results (default: 50).
    pub limit: Option<usize>,
    /// Offset for pagination (default: 0).
    pub offset: Option<usize>,
}

/// Single delegation history entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationHistoryEntry {
    pub task_id: String,
    pub session_key: String,
    pub target: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub latency_ms: Option<u64>,
    pub tool_name: String,
    pub complexity: f32,
    pub error: Option<String>,
}

/// `GET /api/delegation/history` response.
#[derive(Debug, Serialize)]
pub struct DelegationHistoryResponse {
    pub entries: Vec<DelegationHistoryEntry>,
    pub total: usize,
}
```

#### 3.0.2 Monitoring REST Endpoints

```rust
// ── Token Usage ─────────────────────────────────────────────────────

/// `GET /api/monitoring/token-usage` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageResponse {
    pub by_sessions: Vec<SessionTokenUsage>,
    pub by_providers: Vec<ProviderTokenUsage>,
    pub totals: TokenUsageTotals,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTokenUsage {
    pub session_key: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTokenUsage {
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub request_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_requests: u64,
}

// ── Cost Breakdown ──────────────────────────────────────────────────

/// `GET /api/monitoring/costs` query parameters.
#[derive(Debug, Deserialize)]
pub struct CostQuery {
    /// Time range: "daily" | "weekly" | "monthly".
    pub range: String,
}

/// `GET /api/monitoring/costs` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBreakdownResponse {
    pub total_cost: f64,
    pub providers: Vec<String>,
    pub entries: Vec<CostEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEntry {
    /// Period label: "2026-02-23" (daily), "W08" (weekly), "2026-02" (monthly).
    pub period: String,
    /// Cost per provider (same keys as `providers` array).
    pub costs: std::collections::HashMap<String, f64>,
}

// ── Pipeline Runs ───────────────────────────────────────────────────

/// `GET /api/monitoring/pipeline-runs` query parameters.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRunsQuery {
    /// Filter by session key (optional).
    pub session_key: Option<String>,
    /// Maximum results (default: 20).
    pub limit: Option<usize>,
}

/// `GET /api/monitoring/pipeline-runs` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRunsResponse {
    pub runs: Vec<PipelineRunEntry>,
}

/// A single pipeline run with per-stage timing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRunEntry {
    pub task_id: String,
    pub session_key: String,
    pub started_at: String,
    pub total_ms: Option<f64>,
    pub stages: PipelineStageTimings,
}

/// Timing data for each of the 6 pipeline stages.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageTimings {
    pub classifier: StageTimingEntry,
    pub router: StageTimingEntry,
    pub assembler: StageTimingEntry,
    pub transport: StageTimingEntry,
    pub scorer: StageTimingEntry,
    pub learner: StageTimingEntry,
}

/// Individual stage timing.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageTimingEntry {
    /// "pending" | "active" | "completed" | "error"
    pub status: String,
    pub duration_ms: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}
```

#### 3.0.3 WebSocket Event Types (Delegation)

These events are published to the existing `/ws` topic subscription system. Clients subscribe to `delegation:*` and `pipeline:*` topics.

```rust
/// WebSocket events for delegation monitoring.
/// Serialized as JSON and sent on the `/ws` connection.

/// Published when a delegation starts.
/// Topic: "delegation:start"
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationStartEvent {
    pub r#type: &'static str,      // "delegation:start"
    pub task_id: String,
    pub session_key: String,
    pub target: String,
    pub tool_name: String,
    pub complexity: f32,
    pub timestamp: String,
}

/// Published when a delegation completes (success or failure).
/// Topic: "delegation:complete"
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationCompleteEvent {
    pub r#type: &'static str,      // "delegation:complete"
    pub task_id: String,
    pub result: String,             // "success" | "error"
    pub latency_ms: u64,
    pub error: Option<String>,
    pub timestamp: String,
}

/// Published when a pipeline stage starts, completes, or errors.
/// Topic: "pipeline:stage"
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageEvent {
    pub r#type: &'static str,      // "pipeline:stage"
    pub task_id: String,
    pub session_key: String,
    pub stage: String,              // "classifier" | "router" | ... | "learner"
    pub status: String,             // "active" | "completed" | "error"
    pub duration_ms: Option<f64>,
    pub timestamp: String,
}
```

#### 3.0.4 API Endpoint Summary Table

| Method | Path | Request | Response | Description |
|--------|------|---------|----------|-------------|
| `GET` | `/api/delegation/active` | -- | `ListActiveDelegationsResponse` | List all in-flight delegations |
| `GET` | `/api/delegation/rules` | -- | `Vec<DelegationRulePayload>` | List all configured routing rules |
| `PATCH` | `/api/delegation/rules` | `UpsertDelegationRuleRequest` | `DelegationRulePayload` | Create or update a delegation rule |
| `DELETE` | `/api/delegation/rules/:name` | -- | `204 No Content` | Delete a delegation rule by name |
| `GET` | `/api/delegation/history` | `DelegationHistoryQuery` | `DelegationHistoryResponse` | Paginated delegation history |
| `GET` | `/api/monitoring/token-usage` | -- | `TokenUsageResponse` | Token usage by session and provider |
| `GET` | `/api/monitoring/costs` | `CostQuery` | `CostBreakdownResponse` | Cost breakdown by time range |
| `GET` | `/api/monitoring/pipeline-runs` | `PipelineRunsQuery` | `PipelineRunsResponse` | Recent pipeline execution timings |

---

### Task S3.1: Delegation & Monitoring (Week 7)

#### S3.1.1: Delegation Monitor View

Active delegations table showing all in-flight delegations with target, status, and latency.

```tsx
// ui/src/components/delegation/delegation-monitor.tsx

import { useEffect, useState } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { useWebSocket } from "@/hooks/use-websocket";
import { formatDistanceToNow } from "date-fns";

interface ActiveDelegation {
  taskId: string;
  sessionKey: string;
  target: "local" | "claude" | "flow";
  status: "pending" | "running" | "completed" | "failed";
  startedAt: string;
  latencyMs: number | null;
  toolName: string;
  complexity: number;
}

export function DelegationMonitor() {
  const [delegations, setDelegations] = useState<ActiveDelegation[]>([]);
  const { subscribe, lastMessage } = useWebSocket();

  useEffect(() => {
    subscribe(["delegation:start", "delegation:complete", "delegation:status"]);
  }, [subscribe]);

  useEffect(() => {
    if (!lastMessage) return;

    if (lastMessage.type === "delegation:start") {
      setDelegations((prev) => [
        ...prev,
        {
          taskId: lastMessage.taskId,
          sessionKey: lastMessage.sessionKey,
          target: lastMessage.target as ActiveDelegation["target"],
          status: "running",
          startedAt: new Date().toISOString(),
          latencyMs: null,
          toolName: lastMessage.toolName ?? "unknown",
          complexity: lastMessage.complexity ?? 0,
        },
      ]);
    }

    if (lastMessage.type === "delegation:complete") {
      setDelegations((prev) =>
        prev.map((d) =>
          d.taskId === lastMessage.taskId
            ? {
                ...d,
                status: lastMessage.result === "error" ? "failed" : "completed",
                latencyMs: lastMessage.latencyMs ?? null,
              }
            : d
        )
      );
    }
  }, [lastMessage]);

  const targetColor = (target: ActiveDelegation["target"]) => {
    switch (target) {
      case "local":
        return "secondary";
      case "claude":
        return "default";
      case "flow":
        return "outline";
    }
  };

  const statusColor = (status: ActiveDelegation["status"]) => {
    switch (status) {
      case "pending":
        return "secondary";
      case "running":
        return "default";
      case "completed":
        return "outline";
      case "failed":
        return "destructive";
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Active Delegations</h2>
        <Badge variant="secondary">
          {delegations.filter((d) => d.status === "running").length} active
        </Badge>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Task ID</TableHead>
            <TableHead>Tool</TableHead>
            <TableHead>Target</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Complexity</TableHead>
            <TableHead>Latency</TableHead>
            <TableHead>Started</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {delegations.map((d) => (
            <TableRow key={d.taskId}>
              <TableCell className="font-mono text-xs">
                {d.taskId.slice(0, 8)}
              </TableCell>
              <TableCell>{d.toolName}</TableCell>
              <TableCell>
                <Badge variant={targetColor(d.target)}>{d.target}</Badge>
              </TableCell>
              <TableCell>
                <Badge variant={statusColor(d.status)}>{d.status}</Badge>
              </TableCell>
              <TableCell>{(d.complexity * 100).toFixed(0)}%</TableCell>
              <TableCell>
                {d.latencyMs !== null ? `${d.latencyMs}ms` : "--"}
              </TableCell>
              <TableCell className="text-muted-foreground text-sm">
                {formatDistanceToNow(new Date(d.startedAt), {
                  addSuffix: true,
                })}
              </TableCell>
            </TableRow>
          ))}
          {delegations.length === 0 && (
            <TableRow>
              <TableCell
                colSpan={7}
                className="text-center text-muted-foreground py-8"
              >
                No active delegations
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
```

#### S3.1.2: Delegation Rule Editor

Form for creating and editing delegation rules with regex pattern matching, target selector, and complexity threshold slider.

```tsx
// ui/src/components/delegation/delegation-rule-editor.tsx

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useApiClient } from "@/hooks/use-api";

const delegationRuleSchema = z.object({
  name: z.string().min(1, "Rule name is required"),
  pattern: z.string().min(1, "Pattern is required").refine(
    (val) => {
      try {
        new RegExp(val);
        return true;
      } catch {
        return false;
      }
    },
    { message: "Invalid regex pattern" }
  ),
  target: z.enum(["local", "claude", "flow"]),
  complexityThreshold: z.number().min(0).max(1),
  enabled: z.boolean().default(true),
  priority: z.number().int().min(0).max(100).default(50),
});

type DelegationRule = z.infer<typeof delegationRuleSchema>;

interface DelegationRuleEditorProps {
  rule?: DelegationRule;
  onSave: (rule: DelegationRule) => void;
  onCancel: () => void;
}

export function DelegationRuleEditor({
  rule,
  onSave,
  onCancel,
}: DelegationRuleEditorProps) {
  const form = useForm<DelegationRule>({
    resolver: zodResolver(delegationRuleSchema),
    defaultValues: rule ?? {
      name: "",
      pattern: ".*",
      target: "local",
      complexityThreshold: 0.3,
      enabled: true,
      priority: 50,
    },
  });

  const { apiClient } = useApiClient();

  const onSubmit = async (data: DelegationRule) => {
    await apiClient.patch("/api/delegation/rules", data);
    onSave(data);
  };

  const patternValue = form.watch("pattern");
  const patternValid = (() => {
    try {
      new RegExp(patternValue);
      return true;
    } catch {
      return false;
    }
  })();

  return (
    <Card>
      <CardHeader>
        <CardTitle>{rule ? "Edit Rule" : "New Delegation Rule"}</CardTitle>
      </CardHeader>
      <CardContent>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Rule Name</FormLabel>
                  <FormControl>
                    <Input placeholder="e.g. complex-code-gen" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="pattern"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Tool Pattern (Regex)</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="e.g. ^(code_gen|refactor).*"
                      className={`font-mono ${
                        !patternValid ? "border-destructive" : ""
                      }`}
                      {...field}
                    />
                  </FormControl>
                  <FormDescription>
                    Regex pattern to match against tool names. Matched tools are
                    routed to the selected target.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="target"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Delegation Target</FormLabel>
                  <Select
                    onValueChange={field.onChange}
                    defaultValue={field.value}
                  >
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Select target" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="local">
                        Local (in-process)
                      </SelectItem>
                      <SelectItem value="claude">
                        Claude (API delegation)
                      </SelectItem>
                      <SelectItem value="flow">
                        Flow (claude-flow orchestration)
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="complexityThreshold"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>
                    Complexity Threshold:{" "}
                    {(field.value * 100).toFixed(0)}%
                  </FormLabel>
                  <FormControl>
                    <Slider
                      min={0}
                      max={1}
                      step={0.05}
                      value={[field.value]}
                      onValueChange={([v]) => field.onChange(v)}
                    />
                  </FormControl>
                  <FormDescription>
                    Tasks with complexity above this threshold are routed to the
                    selected target. Below threshold = handled locally.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="priority"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Priority: {field.value}</FormLabel>
                  <FormControl>
                    <Slider
                      min={0}
                      max={100}
                      step={1}
                      value={[field.value]}
                      onValueChange={([v]) => field.onChange(v)}
                    />
                  </FormControl>
                  <FormDescription>
                    Higher priority rules are evaluated first. Range: 0-100.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="flex justify-end gap-2">
              <Button type="button" variant="outline" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit">Save Rule</Button>
            </div>
          </form>
        </Form>
      </CardContent>
    </Card>
  );
}
```

#### S3.1.3: Pipeline Stage Inspector

Real-time visualization of the 6-stage delegation pipeline (Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner) with timing bars per session.

```tsx
// ui/src/components/delegation/pipeline-inspector.tsx

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useWebSocket } from "@/hooks/use-websocket";
import { cn } from "@/lib/utils";

const PIPELINE_STAGES = [
  { id: "classifier", label: "Classifier", color: "bg-blue-500" },
  { id: "router", label: "Router", color: "bg-green-500" },
  { id: "assembler", label: "Assembler", color: "bg-yellow-500" },
  { id: "transport", label: "Transport", color: "bg-orange-500" },
  { id: "scorer", label: "Scorer", color: "bg-purple-500" },
  { id: "learner", label: "Learner", color: "bg-pink-500" },
] as const;

type StageId = (typeof PIPELINE_STAGES)[number]["id"];

interface StageEvent {
  stage: StageId;
  status: "pending" | "active" | "completed" | "error";
  startMs: number;
  durationMs: number | null;
  metadata?: Record<string, unknown>;
}

interface PipelineRun {
  taskId: string;
  sessionKey: string;
  stages: Record<StageId, StageEvent>;
  totalMs: number | null;
  startedAt: string;
}

export function PipelineInspector() {
  const [runs, setRuns] = useState<PipelineRun[]>([]);
  const [selectedSession, setSelectedSession] = useState<string>("all");
  const { subscribe, lastMessage } = useWebSocket();

  useEffect(() => {
    subscribe(["pipeline:stage"]);
  }, [subscribe]);

  useEffect(() => {
    if (!lastMessage || lastMessage.type !== "pipeline:stage") return;

    const { taskId, sessionKey, stage, status, durationMs, metadata } =
      lastMessage;

    setRuns((prev) => {
      const existing = prev.find((r) => r.taskId === taskId);
      if (existing) {
        return prev.map((r) =>
          r.taskId === taskId
            ? {
                ...r,
                stages: {
                  ...r.stages,
                  [stage]: {
                    ...r.stages[stage as StageId],
                    status,
                    durationMs: durationMs ?? r.stages[stage as StageId]?.durationMs ?? null,
                    metadata,
                  },
                },
                totalMs:
                  status === "completed" && stage === "learner"
                    ? Object.values(r.stages).reduce(
                        (sum, s) => sum + (s.durationMs ?? 0),
                        0
                      ) + (durationMs ?? 0)
                    : r.totalMs,
              }
            : r
        );
      }

      const emptyStages = Object.fromEntries(
        PIPELINE_STAGES.map((s) => [
          s.id,
          { stage: s.id, status: "pending" as const, startMs: 0, durationMs: null },
        ])
      ) as Record<StageId, StageEvent>;

      return [
        {
          taskId,
          sessionKey,
          stages: {
            ...emptyStages,
            [stage]: {
              stage,
              status,
              startMs: Date.now(),
              durationMs: durationMs ?? null,
              metadata,
            },
          },
          totalMs: null,
          startedAt: new Date().toISOString(),
        },
        ...prev,
      ].slice(0, 50); // Keep last 50 runs
    });
  }, [lastMessage]);

  const sessions = Array.from(new Set(runs.map((r) => r.sessionKey)));
  const filteredRuns =
    selectedSession === "all"
      ? runs
      : runs.filter((r) => r.sessionKey === selectedSession);

  const maxDuration = Math.max(
    ...filteredRuns.map((r) => r.totalMs ?? 0),
    100
  );

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Pipeline Inspector</CardTitle>
          <Select value={selectedSession} onValueChange={setSelectedSession}>
            <SelectTrigger className="w-[200px]">
              <SelectValue placeholder="Filter by session" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Sessions</SelectItem>
              {sessions.map((s) => (
                <SelectItem key={s} value={s}>
                  {s.slice(0, 12)}...
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Stage legend */}
        <div className="flex gap-3 flex-wrap">
          {PIPELINE_STAGES.map((stage) => (
            <div key={stage.id} className="flex items-center gap-1.5 text-sm">
              <div className={cn("w-3 h-3 rounded-sm", stage.color)} />
              <span>{stage.label}</span>
            </div>
          ))}
        </div>

        {/* Pipeline runs */}
        {filteredRuns.map((run) => (
          <div key={run.taskId} className="border rounded-lg p-3 space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="font-mono text-xs text-muted-foreground">
                {run.taskId.slice(0, 8)}
              </span>
              {run.totalMs !== null && (
                <Badge variant="outline">{run.totalMs.toFixed(0)}ms total</Badge>
              )}
            </div>

            {/* Timing bar visualization */}
            <div className="flex h-8 rounded-md overflow-hidden bg-muted">
              {PIPELINE_STAGES.map((stageDef) => {
                const stageData = run.stages[stageDef.id];
                const duration = stageData?.durationMs ?? 0;
                const widthPct =
                  maxDuration > 0 ? (duration / maxDuration) * 100 : 0;

                return (
                  <div
                    key={stageDef.id}
                    className={cn(
                      "h-full relative group transition-all",
                      stageData?.status === "active" && "animate-pulse",
                      stageData?.status === "error" && "bg-destructive",
                      stageData?.status === "completed" && stageDef.color,
                      stageData?.status === "pending" && "bg-muted"
                    )}
                    style={{ width: `${Math.max(widthPct, 1)}%` }}
                    title={`${stageDef.label}: ${duration}ms`}
                  >
                    {/* Tooltip on hover */}
                    <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 hidden group-hover:block z-10">
                      <div className="bg-popover text-popover-foreground text-xs rounded px-2 py-1 shadow-md whitespace-nowrap">
                        {stageDef.label}: {duration > 0 ? `${duration}ms` : "--"}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Stage details row */}
            <div className="grid grid-cols-6 gap-1 text-xs text-muted-foreground">
              {PIPELINE_STAGES.map((stageDef) => {
                const stageData = run.stages[stageDef.id];
                return (
                  <div key={stageDef.id} className="text-center">
                    <span className="block">
                      {stageData?.durationMs !== null
                        ? `${stageData?.durationMs}ms`
                        : "--"}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        ))}

        {filteredRuns.length === 0 && (
          <div className="text-center text-muted-foreground py-8">
            No pipeline events recorded. Delegations will appear here in
            real-time.
          </div>
        )}
      </CardContent>
    </Card>
  );
}
```

#### S3.1.4: Token Usage Dashboard

Per-session, per-provider, per-model token usage charts using recharts.

```tsx
// ui/src/components/monitoring/token-usage-dashboard.tsx

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from "recharts";
import { useApiClient } from "@/hooks/use-api";

interface TokenUsageBySession {
  sessionKey: string;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
}

interface TokenUsageByProvider {
  provider: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  requestCount: number;
}

interface TokenUsageData {
  bySessions: TokenUsageBySession[];
  byProviders: TokenUsageByProvider[];
  totals: {
    inputTokens: number;
    outputTokens: number;
    totalTokens: number;
    totalRequests: number;
  };
}

const CHART_COLORS = [
  "hsl(var(--chart-1))",
  "hsl(var(--chart-2))",
  "hsl(var(--chart-3))",
  "hsl(var(--chart-4))",
  "hsl(var(--chart-5))",
];

export function TokenUsageDashboard() {
  const [data, setData] = useState<TokenUsageData | null>(null);
  const { apiClient } = useApiClient();

  useEffect(() => {
    apiClient
      .get<TokenUsageData>("/api/monitoring/token-usage")
      .then(setData)
      .catch(console.error);
  }, [apiClient]);

  if (!data) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          Loading token usage data...
        </CardContent>
      </Card>
    );
  }

  const sessionData = data.bySessions
    .sort((a, b) => b.totalTokens - a.totalTokens)
    .slice(0, 15)
    .map((s) => ({
      session: s.sessionKey.slice(0, 10),
      "Input Tokens": s.inputTokens,
      "Output Tokens": s.outputTokens,
    }));

  const providerData = data.byProviders.map((p) => ({
    name: `${p.provider}/${p.model}`,
    value: p.totalTokens,
    requests: p.requestCount,
  }));

  return (
    <div className="space-y-4">
      {/* Summary cards */}
      <div className="grid grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Tokens
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {data.totals.totalTokens.toLocaleString()}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Input Tokens
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {data.totals.inputTokens.toLocaleString()}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Output Tokens
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {data.totals.outputTokens.toLocaleString()}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Requests
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {data.totals.totalRequests.toLocaleString()}
            </div>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="sessions">
        <TabsList>
          <TabsTrigger value="sessions">By Session</TabsTrigger>
          <TabsTrigger value="providers">By Provider/Model</TabsTrigger>
        </TabsList>

        <TabsContent value="sessions">
          <Card>
            <CardHeader>
              <CardTitle>Token Usage by Session (Top 15)</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={400}>
                <BarChart data={sessionData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis
                    dataKey="session"
                    tick={{ fontSize: 11 }}
                    angle={-45}
                    textAnchor="end"
                    height={60}
                  />
                  <YAxis />
                  <Tooltip />
                  <Legend />
                  <Bar
                    dataKey="Input Tokens"
                    stackId="a"
                    fill="hsl(var(--chart-1))"
                  />
                  <Bar
                    dataKey="Output Tokens"
                    stackId="a"
                    fill="hsl(var(--chart-2))"
                  />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="providers">
          <Card>
            <CardHeader>
              <CardTitle>Token Distribution by Provider/Model</CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={400}>
                <PieChart>
                  <Pie
                    data={providerData}
                    cx="50%"
                    cy="50%"
                    labelLine
                    label={({ name, percent }) =>
                      `${name} (${(percent * 100).toFixed(0)}%)`
                    }
                    outerRadius={150}
                    fill="#8884d8"
                    dataKey="value"
                  >
                    {providerData.map((_entry, index) => (
                      <Cell
                        key={`cell-${index}`}
                        fill={CHART_COLORS[index % CHART_COLORS.length]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value: number, _name: string, props) => [
                      `${value.toLocaleString()} tokens (${props.payload.requests} requests)`,
                      props.payload.name,
                    ]}
                  />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
```

#### S3.1.5: Cost Tracker

Daily/weekly/monthly cost breakdown by provider using recharts BarChart.

```tsx
// ui/src/components/monitoring/cost-tracker.tsx

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { useApiClient } from "@/hooks/use-api";

type TimeRange = "daily" | "weekly" | "monthly";

interface CostEntry {
  period: string; // ISO date or "Week N" or "Month YYYY-MM"
  providers: Record<string, number>; // provider name -> cost in USD
  totalCost: number;
}

interface CostData {
  entries: CostEntry[];
  providers: string[];
  totalCost: number;
}

const PROVIDER_COLORS: Record<string, string> = {
  anthropic: "hsl(var(--chart-1))",
  openai: "hsl(var(--chart-2))",
  local: "hsl(var(--chart-3))",
  "claude-flow": "hsl(var(--chart-4))",
};

export function CostTracker() {
  const [timeRange, setTimeRange] = useState<TimeRange>("daily");
  const [data, setData] = useState<CostData | null>(null);
  const { apiClient } = useApiClient();

  useEffect(() => {
    apiClient
      .get<CostData>(`/api/monitoring/costs?range=${timeRange}`)
      .then(setData)
      .catch(console.error);
  }, [apiClient, timeRange]);

  if (!data) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          Loading cost data...
        </CardContent>
      </Card>
    );
  }

  const chartData = data.entries.map((entry) => ({
    period: entry.period,
    ...entry.providers,
    Total: entry.totalCost,
  }));

  return (
    <div className="space-y-4">
      {/* Total cost summary */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium text-muted-foreground">
            Total Cost ({timeRange})
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold">${data.totalCost.toFixed(4)}</div>
        </CardContent>
      </Card>

      {/* Time range selector */}
      <Tabs
        value={timeRange}
        onValueChange={(v) => setTimeRange(v as TimeRange)}
      >
        <TabsList>
          <TabsTrigger value="daily">Daily</TabsTrigger>
          <TabsTrigger value="weekly">Weekly</TabsTrigger>
          <TabsTrigger value="monthly">Monthly</TabsTrigger>
        </TabsList>

        {(["daily", "weekly", "monthly"] as const).map((range) => (
          <TabsContent key={range} value={range}>
            <Card>
              <CardHeader>
                <CardTitle>
                  Cost Breakdown ({range.charAt(0).toUpperCase() + range.slice(1)})
                </CardTitle>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={400}>
                  <BarChart data={chartData}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis
                      dataKey="period"
                      tick={{ fontSize: 11 }}
                      angle={-45}
                      textAnchor="end"
                      height={60}
                    />
                    <YAxis
                      tickFormatter={(value) => `$${value.toFixed(3)}`}
                    />
                    <Tooltip
                      formatter={(value: number) => `$${value.toFixed(4)}`}
                    />
                    <Legend />
                    {data.providers.map((provider) => (
                      <Bar
                        key={provider}
                        dataKey={provider}
                        stackId="cost"
                        fill={
                          PROVIDER_COLORS[provider] ?? "hsl(var(--chart-5))"
                        }
                      />
                    ))}
                  </BarChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>
          </TabsContent>
        ))}
      </Tabs>
    </div>
  );
}
```

---

### Task S3.2: Advanced Canvas (Weeks 7-8)

#### S3.2.1: Chart Rendering (Recharts Integration)

Canvas chart element type and renderer using recharts.

```tsx
// ui/src/components/canvas/elements/canvas-chart-element.tsx

import {
  BarChart,
  Bar,
  LineChart,
  Line,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";

export interface CanvasChartData {
  type: "bar" | "line" | "pie";
  title?: string;
  data: Record<string, unknown>[];
  xKey: string;
  yKeys: string[];
  colors?: string[];
  width?: number;
  height?: number;
}

const DEFAULT_COLORS = [
  "#8884d8",
  "#82ca9d",
  "#ffc658",
  "#ff7300",
  "#0088fe",
  "#00c49f",
];

interface CanvasChartElementProps {
  chart: CanvasChartData;
}

export function CanvasChartElement({ chart }: CanvasChartElementProps) {
  const {
    type,
    title,
    data,
    xKey,
    yKeys,
    colors = DEFAULT_COLORS,
    width = 500,
    height = 300,
  } = chart;

  return (
    <div className="p-2" style={{ width, height: height + 40 }}>
      {title && (
        <h4 className="text-sm font-medium text-center mb-2">{title}</h4>
      )}
      <ResponsiveContainer width="100%" height={height}>
        {type === "bar" ? (
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey={xKey} tick={{ fontSize: 10 }} />
            <YAxis tick={{ fontSize: 10 }} />
            <Tooltip />
            <Legend />
            {yKeys.map((key, i) => (
              <Bar key={key} dataKey={key} fill={colors[i % colors.length]} />
            ))}
          </BarChart>
        ) : type === "line" ? (
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey={xKey} tick={{ fontSize: 10 }} />
            <YAxis tick={{ fontSize: 10 }} />
            <Tooltip />
            <Legend />
            {yKeys.map((key, i) => (
              <Line
                key={key}
                type="monotone"
                dataKey={key}
                stroke={colors[i % colors.length]}
              />
            ))}
          </LineChart>
        ) : (
          <PieChart>
            <Pie
              data={data}
              dataKey={yKeys[0]}
              nameKey={xKey}
              cx="50%"
              cy="50%"
              outerRadius={height / 3}
              label
            >
              {data.map((_entry, index) => (
                <Cell
                  key={`cell-${index}`}
                  fill={colors[index % colors.length]}
                />
              ))}
            </Pie>
            <Tooltip />
            <Legend />
          </PieChart>
        )}
      </ResponsiveContainer>
    </div>
  );
}
```

#### S3.2.2: Code Editor Element (Monaco)

Monaco editor embedded in Canvas for code editing interactions.

```tsx
// ui/src/components/canvas/elements/canvas-code-element.tsx

import { useCallback, useRef } from "react";
import Editor, { type OnMount, type OnChange } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export interface CanvasCodeData {
  elementId: string;
  language: string;
  initialValue: string;
  readOnly?: boolean;
  title?: string;
  height?: number;
  submitLabel?: string;
}

interface CanvasCodeElementProps {
  code: CanvasCodeData;
  onSubmit?: (elementId: string, value: string) => void;
}

export function CanvasCodeElement({ code, onSubmit }: CanvasCodeElementProps) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const valueRef = useRef(code.initialValue);

  const handleMount: OnMount = (editor) => {
    editorRef.current = editor;
  };

  const handleChange: OnChange = (value) => {
    valueRef.current = value ?? "";
  };

  const handleSubmit = useCallback(() => {
    onSubmit?.(code.elementId, valueRef.current);
  }, [code.elementId, onSubmit]);

  return (
    <Card className="overflow-hidden">
      <CardHeader className="py-2 px-3 flex flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          {code.title && (
            <CardTitle className="text-sm">{code.title}</CardTitle>
          )}
          <Badge variant="secondary" className="text-xs">
            {code.language}
          </Badge>
        </div>
        {!code.readOnly && onSubmit && (
          <Button size="sm" variant="default" onClick={handleSubmit}>
            {code.submitLabel ?? "Submit"}
          </Button>
        )}
      </CardHeader>
      <CardContent className="p-0">
        <Editor
          height={code.height ?? 300}
          language={code.language}
          defaultValue={code.initialValue}
          onMount={handleMount}
          onChange={handleChange}
          options={{
            readOnly: code.readOnly ?? false,
            minimap: { enabled: false },
            lineNumbers: "on",
            scrollBeyondLastLine: false,
            fontSize: 13,
            tabSize: 2,
            automaticLayout: true,
            wordWrap: "on",
          }}
          theme="vs-dark"
        />
      </CardContent>
    </Card>
  );
}
```

#### S3.2.3: Form Builder

Agent-constructed multi-field forms with validation, rendered dynamically on Canvas.

```tsx
// ui/src/components/canvas/elements/canvas-form-element.tsx

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

interface FormFieldSpec {
  name: string;
  label: string;
  type: "text" | "number" | "email" | "textarea" | "select" | "boolean";
  required?: boolean;
  placeholder?: string;
  description?: string;
  defaultValue?: unknown;
  options?: { value: string; label: string }[]; // for select
  min?: number; // for number
  max?: number; // for number
}

export interface CanvasFormData {
  elementId: string;
  title: string;
  description?: string;
  fields: FormFieldSpec[];
  submitLabel?: string;
}

interface CanvasFormElementProps {
  formSpec: CanvasFormData;
  onSubmit?: (elementId: string, values: Record<string, unknown>) => void;
}

function buildZodSchema(fields: FormFieldSpec[]) {
  const shape: Record<string, z.ZodTypeAny> = {};

  for (const field of fields) {
    let schema: z.ZodTypeAny;

    switch (field.type) {
      case "number":
        schema = z.coerce.number();
        if (field.min !== undefined) schema = (schema as z.ZodNumber).min(field.min);
        if (field.max !== undefined) schema = (schema as z.ZodNumber).max(field.max);
        break;
      case "email":
        schema = z.string().email();
        break;
      case "boolean":
        schema = z.boolean();
        break;
      default:
        schema = z.string();
    }

    if (!field.required && field.type !== "boolean") {
      schema = schema.optional();
    }

    shape[field.name] = schema;
  }

  return z.object(shape);
}

export function CanvasFormElement({
  formSpec,
  onSubmit,
}: CanvasFormElementProps) {
  const schema = buildZodSchema(formSpec.fields);

  const defaults = Object.fromEntries(
    formSpec.fields.map((f) => [
      f.name,
      f.defaultValue ?? (f.type === "boolean" ? false : ""),
    ])
  );

  const form = useForm({
    resolver: zodResolver(schema),
    defaultValues: defaults,
  });

  const handleSubmit = (values: Record<string, unknown>) => {
    onSubmit?.(formSpec.elementId, values);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">{formSpec.title}</CardTitle>
        {formSpec.description && (
          <p className="text-sm text-muted-foreground">
            {formSpec.description}
          </p>
        )}
      </CardHeader>
      <CardContent>
        <Form {...form}>
          <form
            onSubmit={form.handleSubmit(handleSubmit)}
            className="space-y-4"
          >
            {formSpec.fields.map((fieldSpec) => (
              <FormField
                key={fieldSpec.name}
                control={form.control}
                name={fieldSpec.name}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>{fieldSpec.label}</FormLabel>
                    <FormControl>
                      {fieldSpec.type === "textarea" ? (
                        <Textarea
                          placeholder={fieldSpec.placeholder}
                          {...field}
                          value={field.value as string}
                        />
                      ) : fieldSpec.type === "select" ? (
                        <Select
                          onValueChange={field.onChange}
                          defaultValue={field.value as string}
                        >
                          <SelectTrigger>
                            <SelectValue
                              placeholder={fieldSpec.placeholder ?? "Select..."}
                            />
                          </SelectTrigger>
                          <SelectContent>
                            {fieldSpec.options?.map((opt) => (
                              <SelectItem key={opt.value} value={opt.value}>
                                {opt.label}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      ) : fieldSpec.type === "boolean" ? (
                        <Switch
                          checked={field.value as boolean}
                          onCheckedChange={field.onChange}
                        />
                      ) : (
                        <Input
                          type={fieldSpec.type}
                          placeholder={fieldSpec.placeholder}
                          {...field}
                          value={field.value as string}
                        />
                      )}
                    </FormControl>
                    {fieldSpec.description && (
                      <FormDescription>{fieldSpec.description}</FormDescription>
                    )}
                    <FormMessage />
                  </FormItem>
                )}
              />
            ))}
            <Button type="submit">
              {formSpec.submitLabel ?? "Submit"}
            </Button>
          </form>
        </Form>
      </CardContent>
    </Card>
  );
}
```

#### S3.2.4: Canvas History (Undo/Redo)

Undo/redo stack with replay timeline slider for Canvas state.

```tsx
// ui/src/stores/canvas-history-store.ts

import { create } from "zustand";
import type { CanvasCommand } from "@/lib/types";

interface CanvasSnapshot {
  timestamp: number;
  commands: CanvasCommand[];
  label?: string;
}

interface CanvasHistoryState {
  snapshots: CanvasSnapshot[];
  currentIndex: number;
  maxSnapshots: number;

  pushSnapshot: (commands: CanvasCommand[], label?: string) => void;
  undo: () => CanvasCommand[] | null;
  redo: () => CanvasCommand[] | null;
  canUndo: () => boolean;
  canRedo: () => boolean;
  seekTo: (index: number) => CanvasCommand[] | null;
  clear: () => void;
}

export const useCanvasHistoryStore = create<CanvasHistoryState>((set, get) => ({
  snapshots: [],
  currentIndex: -1,
  maxSnapshots: 200,

  pushSnapshot: (commands, label) => {
    const state = get();
    // Truncate any redo history when new action is taken
    const truncated = state.snapshots.slice(0, state.currentIndex + 1);
    const snapshot: CanvasSnapshot = {
      timestamp: Date.now(),
      commands: structuredClone(commands),
      label,
    };
    const updated = [...truncated, snapshot].slice(-state.maxSnapshots);
    set({
      snapshots: updated,
      currentIndex: updated.length - 1,
    });
  },

  undo: () => {
    const state = get();
    if (state.currentIndex <= 0) return null;
    const newIndex = state.currentIndex - 1;
    set({ currentIndex: newIndex });
    return state.snapshots[newIndex].commands;
  },

  redo: () => {
    const state = get();
    if (state.currentIndex >= state.snapshots.length - 1) return null;
    const newIndex = state.currentIndex + 1;
    set({ currentIndex: newIndex });
    return state.snapshots[newIndex].commands;
  },

  canUndo: () => get().currentIndex > 0,
  canRedo: () => get().currentIndex < get().snapshots.length - 1,

  seekTo: (index) => {
    const state = get();
    if (index < 0 || index >= state.snapshots.length) return null;
    set({ currentIndex: index });
    return state.snapshots[index].commands;
  },

  clear: () => set({ snapshots: [], currentIndex: -1 }),
}));
```

```tsx
// ui/src/components/canvas/canvas-history-toolbar.tsx

import { Slider } from "@/components/ui/slider";
import { Button } from "@/components/ui/button";
import { useCanvasHistoryStore } from "@/stores/canvas-history-store";

interface CanvasHistoryToolbarProps {
  onRestore: (commands: unknown[]) => void;
}

export function CanvasHistoryToolbar({ onRestore }: CanvasHistoryToolbarProps) {
  const { snapshots, currentIndex, undo, redo, canUndo, canRedo, seekTo } =
    useCanvasHistoryStore();

  const handleUndo = () => {
    const commands = undo();
    if (commands) onRestore(commands);
  };

  const handleRedo = () => {
    const commands = redo();
    if (commands) onRestore(commands);
  };

  const handleSeek = (value: number[]) => {
    const commands = seekTo(value[0]);
    if (commands) onRestore(commands);
  };

  return (
    <div className="flex items-center gap-3 px-4 py-2 border-t bg-background">
      <Button
        variant="outline"
        size="sm"
        onClick={handleUndo}
        disabled={!canUndo()}
        title="Undo (Ctrl+Z)"
      >
        Undo
      </Button>
      <Button
        variant="outline"
        size="sm"
        onClick={handleRedo}
        disabled={!canRedo()}
        title="Redo (Ctrl+Shift+Z)"
      >
        Redo
      </Button>

      {snapshots.length > 1 && (
        <div className="flex-1 flex items-center gap-3">
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            {currentIndex + 1} / {snapshots.length}
          </span>
          <Slider
            min={0}
            max={snapshots.length - 1}
            step={1}
            value={[currentIndex]}
            onValueChange={handleSeek}
            className="flex-1"
          />
          <span className="text-xs text-muted-foreground whitespace-nowrap">
            {snapshots[currentIndex]?.label ?? "Snapshot"}
          </span>
        </div>
      )}
    </div>
  );
}
```

#### S3.2.5: Canvas Export

PNG screenshot (html2canvas), HTML export, and JSON state export.

```tsx
// ui/src/components/canvas/canvas-export.tsx

import { useCallback, useRef } from "react";
import html2canvas from "html2canvas";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useCanvasStore } from "@/stores/canvas-store";

interface CanvasExportProps {
  canvasRef: React.RefObject<HTMLDivElement>;
}

export function CanvasExport({ canvasRef }: CanvasExportProps) {
  const { elements, commands } = useCanvasStore();

  const downloadBlob = useCallback((blob: Blob, filename: string) => {
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, []);

  const exportPng = useCallback(async () => {
    if (!canvasRef.current) return;
    const canvas = await html2canvas(canvasRef.current, {
      backgroundColor: null,
      scale: 2, // Retina quality
      useCORS: true,
    });
    canvas.toBlob((blob) => {
      if (blob) {
        const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, "-");
        downloadBlob(blob, `canvas-${timestamp}.png`);
      }
    }, "image/png");
  }, [canvasRef, downloadBlob]);

  const exportHtml = useCallback(() => {
    if (!canvasRef.current) return;
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ClawFT Canvas Export</title>
  <style>
    body { margin: 0; padding: 20px; font-family: system-ui, sans-serif; }
    .canvas-container { max-width: 1200px; margin: 0 auto; }
  </style>
</head>
<body>
  <div class="canvas-container">
    ${canvasRef.current.innerHTML}
  </div>
</body>
</html>`;
    const blob = new Blob([html], { type: "text/html" });
    const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, "-");
    downloadBlob(blob, `canvas-${timestamp}.html`);
  }, [canvasRef, downloadBlob]);

  const exportJson = useCallback(() => {
    const state = {
      version: 1,
      exportedAt: new Date().toISOString(),
      elements,
      commands,
    };
    const json = JSON.stringify(state, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, "-");
    downloadBlob(blob, `canvas-${timestamp}.json`);
  }, [elements, commands, downloadBlob]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm">
          Export
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onClick={exportPng}>
          Export as PNG
        </DropdownMenuItem>
        <DropdownMenuItem onClick={exportHtml}>
          Export as HTML
        </DropdownMenuItem>
        <DropdownMenuItem onClick={exportJson}>
          Export as JSON
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
```

---

### Task S3.3: Mobile Responsive + PWA (Week 8)

#### S3.3.1: Responsive Sidebar

Drawer on mobile (<768px), collapsible on tablet, full sidebar on desktop.

```tsx
// ui/src/components/layout/responsive-sidebar.tsx

import { useState, useEffect } from "react";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ResponsiveSidebarProps {
  children: React.ReactNode;
  navigationContent: React.ReactNode;
}

function useMediaQuery(query: string) {
  const [matches, setMatches] = useState(false);

  useEffect(() => {
    const mql = window.matchMedia(query);
    setMatches(mql.matches);
    const handler = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [query]);

  return matches;
}

export function ResponsiveSidebar({
  children,
  navigationContent,
}: ResponsiveSidebarProps) {
  const isMobile = useMediaQuery("(max-width: 767px)");
  const isTablet = useMediaQuery("(min-width: 768px) and (max-width: 1023px)");
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  // Mobile: Sheet/drawer
  if (isMobile) {
    return (
      <div className="flex flex-col h-screen">
        <header className="flex items-center gap-2 px-4 py-2 border-b">
          <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
            <SheetTrigger asChild>
              <Button variant="ghost" size="sm" aria-label="Open navigation">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="20"
                  height="20"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <line x1="4" x2="20" y1="12" y2="12" />
                  <line x1="4" x2="20" y1="6" y2="6" />
                  <line x1="4" x2="20" y1="18" y2="18" />
                </svg>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-64 p-0">
              <div className="h-full overflow-y-auto">{navigationContent}</div>
            </SheetContent>
          </Sheet>
          <span className="font-semibold">ClawFT</span>
        </header>
        <main className="flex-1 overflow-auto">{children}</main>
      </div>
    );
  }

  // Tablet: collapsible sidebar
  // Desktop: full sidebar (also collapsible)
  const sidebarWidth = collapsed ? "w-16" : isTablet ? "w-56" : "w-64";

  return (
    <div className="flex h-screen">
      <aside
        className={cn(
          "h-full border-r bg-background transition-all duration-200 flex flex-col",
          sidebarWidth
        )}
      >
        <div className="flex items-center justify-between px-3 py-2 border-b">
          {!collapsed && <span className="font-semibold text-sm">ClawFT</span>}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setCollapsed(!collapsed)}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={cn(
                "transition-transform",
                collapsed && "rotate-180"
              )}
            >
              <path d="m15 18-6-6 6-6" />
            </svg>
          </Button>
        </div>
        <div className="flex-1 overflow-y-auto">{navigationContent}</div>
      </aside>
      <main className="flex-1 overflow-auto">{children}</main>
    </div>
  );
}
```

#### S3.3.2: Mobile WebChat

Bottom-anchored input with swipe navigation between sessions.

```tsx
// ui/src/components/chat/mobile-web-chat.tsx

import { useState, useRef, useCallback } from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useSwipeable } from "react-swipeable";
import { useSessionStore } from "@/stores/session-store";
import { MessageBubble } from "./message-bubble";

interface MobileWebChatProps {
  sessionKeys: string[];
  currentSession: string;
  onSessionChange: (session: string) => void;
  onSend: (content: string) => void;
}

export function MobileWebChat({
  sessionKeys,
  currentSession,
  onSessionChange,
  onSend,
}: MobileWebChatProps) {
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const { sessions } = useSessionStore();

  const currentIndex = sessionKeys.indexOf(currentSession);
  const messages = sessions[currentSession]?.messages ?? [];

  const swipeHandlers = useSwipeable({
    onSwipedLeft: () => {
      if (currentIndex < sessionKeys.length - 1) {
        onSessionChange(sessionKeys[currentIndex + 1]);
      }
    },
    onSwipedRight: () => {
      if (currentIndex > 0) {
        onSessionChange(sessionKeys[currentIndex - 1]);
      }
    },
    trackMouse: false,
    trackTouch: true,
    delta: 50,
  });

  const handleSend = useCallback(() => {
    const trimmed = input.trim();
    if (!trimmed) return;
    onSend(trimmed);
    setInput("");
  }, [input, onSend]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex flex-col h-full" {...swipeHandlers}>
      {/* Session indicator */}
      <div className="flex items-center justify-center gap-1 py-1 border-b">
        {sessionKeys.map((key, i) => (
          <div
            key={key}
            className={`w-2 h-2 rounded-full transition-colors ${
              i === currentIndex ? "bg-primary" : "bg-muted"
            }`}
          />
        ))}
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1 px-3" ref={scrollRef}>
        <div className="py-2 space-y-3">
          {messages.map((msg, i) => (
            <MessageBubble key={i} message={msg} compact />
          ))}
        </div>
      </ScrollArea>

      {/* Bottom-anchored input */}
      <div className="border-t p-2 pb-safe bg-background">
        <div className="flex gap-2">
          <Input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type a message..."
            className="flex-1"
            autoComplete="off"
          />
          <Button onClick={handleSend} disabled={!input.trim()} size="icon">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="m22 2-7 20-4-9-9-4Z" />
              <path d="M22 2 11 13" />
            </svg>
          </Button>
        </div>
      </div>
    </div>
  );
}
```

#### S3.3.3: PWA Manifest

```json
// ui/public/manifest.json
{
  "name": "ClawFT Dashboard",
  "short_name": "ClawFT",
  "description": "AI Agent Management Dashboard",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#0a0a0a",
  "theme_color": "#7c3aed",
  "scope": "/",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512-maskable.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "maskable"
    }
  ]
}
```

#### S3.3.4: Service Worker

Offline dashboard shell with cache-first strategy for static assets.

```typescript
// ui/src/sw.ts
/// <reference lib="webworker" />

declare const self: ServiceWorkerGlobalScope;

const CACHE_NAME = "clawft-v1";
const STATIC_ASSETS = [
  "/",
  "/index.html",
  "/manifest.json",
  "/favicon.svg",
];

// Install: pre-cache static shell
self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
  );
  self.skipWaiting();
});

// Activate: clean old caches
self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((key) => key !== CACHE_NAME)
          .map((key) => caches.delete(key))
      )
    )
  );
  self.clients.claim();
});

// Fetch: cache-first for static, network-first for API
self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);

  // Never cache API calls or WebSocket upgrades
  if (url.pathname.startsWith("/api") || url.pathname.startsWith("/ws")) {
    return;
  }

  // Cache-first for static assets
  event.respondWith(
    caches.match(event.request).then((cached) => {
      if (cached) return cached;

      return fetch(event.request).then((response) => {
        // Cache successful GET requests for static assets
        if (
          response.ok &&
          event.request.method === "GET" &&
          (url.pathname.endsWith(".js") ||
            url.pathname.endsWith(".css") ||
            url.pathname.endsWith(".woff2") ||
            url.pathname.endsWith(".png") ||
            url.pathname.endsWith(".svg"))
        ) {
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(event.request, clone);
          });
        }

        return response;
      });
    })
  );
});
```

#### S3.3.5: Push Notification Integration

Service worker push handler + WebSocket events to Notification API bridge.

```typescript
// ui/src/lib/push-notifications.ts

export async function requestNotificationPermission(): Promise<boolean> {
  if (!("Notification" in window)) return false;
  if (Notification.permission === "granted") return true;
  if (Notification.permission === "denied") return false;
  const result = await Notification.requestPermission();
  return result === "granted";
}

export function showNotification(title: string, options?: NotificationOptions) {
  if (Notification.permission !== "granted") return;

  // Prefer service worker notification for persistence
  if ("serviceWorker" in navigator && navigator.serviceWorker.controller) {
    navigator.serviceWorker.ready.then((registration) => {
      registration.showNotification(title, {
        icon: "/icons/icon-192.png",
        badge: "/icons/icon-192.png",
        ...options,
      });
    });
  } else {
    new Notification(title, options);
  }
}

// Bridge WebSocket events to notifications
export function bridgeWsToNotifications(
  wsEventType: string,
  wsPayload: Record<string, unknown>
) {
  switch (wsEventType) {
    case "agent:status":
      if (wsPayload.status === "error") {
        showNotification("Agent Error", {
          body: `Agent ${wsPayload.agentId} encountered an error`,
          tag: `agent-error-${wsPayload.agentId}`,
        });
      }
      break;

    case "delegation:complete":
      showNotification("Delegation Complete", {
        body: `Task ${(wsPayload.taskId as string).slice(0, 8)} finished: ${wsPayload.result}`,
        tag: `delegation-${wsPayload.taskId}`,
      });
      break;

    case "cron:fired":
      showNotification("Cron Job Fired", {
        body: `Job ${wsPayload.jobId} executed`,
        tag: `cron-${wsPayload.jobId}`,
      });
      break;
  }
}
```

---

### Task S3.4: Tauri Desktop Shell (Weeks 8-9)

#### S3.4.1: Tauri Project Initialization

```json
// ui/src-tauri/tauri.conf.json
{
  "$schema": "https://raw.githubusercontent.com/nicbarker/tauri/dev/packages/tauri-cli/schema.json",
  "productName": "ClawFT",
  "version": "0.1.0",
  "identifier": "dev.clawft.desktop",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "pnpm build",
    "beforeDevCommand": "pnpm dev"
  },
  "app": {
    "title": "ClawFT",
    "windows": [
      {
        "title": "ClawFT",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "transparent": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws://localhost:18789 http://localhost:18789; img-src 'self' data: blob:"
    },
    "trayIcon": {
      "iconPath": "icons/tray-icon.png",
      "iconAsTemplate": true,
      "tooltip": "ClawFT"
    }
  },
  "plugins": {
    "notification": {
      "all": true
    },
    "global-shortcut": {
      "all": true
    },
    "shell": {
      "scope": [
        {
          "name": "weft-gateway",
          "cmd": "weft",
          "args": ["gateway", "--port", "18789"]
        }
      ]
    }
  }
}
```

#### S3.4.2: System Tray with Agent Status

```rust
// ui/src-tauri/src/tray.rs

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Stopped,
    Error,
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Status: Stopped", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit ClawFT", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &status, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("ClawFT - Stopped")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_status(app: &AppHandle, status: AgentStatus) {
    let tooltip = match status {
        AgentStatus::Running => "ClawFT - Running",
        AgentStatus::Stopped => "ClawFT - Stopped",
        AgentStatus::Error => "ClawFT - Error",
    };

    // Update the tray tooltip via the tray handle
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
```

#### S3.4.3: Global Hotkey

```rust
// ui/src-tauri/src/hotkey.rs

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub fn register_global_hotkey(app: &AppHandle) -> tauri::Result<()> {
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let binding = shortcut.into_string();
                    if binding == "CmdOrCtrl+Shift+W" {
                        toggle_window(app);
                    }
                }
            })
            .build(),
    )?;

    app.global_shortcut().register("CmdOrCtrl+Shift+W")?;

    Ok(())
}

fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}
```

#### S3.4.4: Auto-Start Gateway

```rust
// ui/src-tauri/src/gateway.rs

use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandChild;

pub struct GatewayProcess(pub Mutex<Option<CommandChild>>);

pub fn start_gateway(app: &AppHandle) -> tauri::Result<()> {
    let shell = app.shell();
    let (mut rx, child) = shell
        .sidecar("weft-gateway")
        .expect("weft-gateway sidecar not configured")
        .args(["gateway", "--port", "18789"])
        .spawn()
        .expect("Failed to spawn weft gateway");

    // Store the child process handle for cleanup
    let state = app.state::<GatewayProcess>();
    *state.0.lock().unwrap() = Some(child);

    // Log gateway output in background
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                    log::info!("[gateway] {}", String::from_utf8_lossy(&line));
                }
                tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                    log::warn!("[gateway] {}", String::from_utf8_lossy(&line));
                }
                tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                    log::info!("[gateway] terminated: {:?}", payload);
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

pub fn stop_gateway(app: &AppHandle) {
    let state = app.state::<GatewayProcess>();
    if let Some(child) = state.0.lock().unwrap().take() {
        let _ = child.kill();
    }
}
```

#### S3.4.5: macOS Spotlight Integration

```rust
// ui/src-tauri/src/spotlight.rs

#[cfg(target_os = "macos")]
use tauri::AppHandle;

/// Register NSUserActivity for Spotlight indexing.
/// Allows users to search "ClawFT agent query" in Spotlight.
#[cfg(target_os = "macos")]
pub fn register_spotlight_activity(app: &AppHandle) {
    // NSUserActivity registration for Spotlight requires Objective-C bridge.
    // This is handled via the Info.plist NSUserActivityTypes key and
    // the Tauri deep-link plugin for universal links.
    //
    // Info.plist addition:
    //   <key>NSUserActivityTypes</key>
    //   <array>
    //     <string>dev.clawft.agent-query</string>
    //   </array>
    //
    // The frontend handles the activity via:
    //   window.__TAURI__.event.listen("deep-link://", handler)

    log::info!("macOS Spotlight activity registered for dev.clawft.agent-query");
    let _ = app; // Suppress unused warning; actual integration via Info.plist
}

#[cfg(not(target_os = "macos"))]
pub fn register_spotlight_activity(_app: &tauri::AppHandle) {
    // No-op on non-macOS platforms
}
```

#### S3.4.6: Native Notification Bridge

```rust
// ui/src-tauri/src/notifications.rs

use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub body: String,
    pub tag: Option<String>,
}

#[tauri::command]
pub fn send_native_notification(
    app: AppHandle,
    payload: NotificationPayload,
) -> Result<(), String> {
    app.notification()
        .builder()
        .title(&payload.title)
        .body(&payload.body)
        .show()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

#### S3.4.7: Tauri Main Entry

```rust
// ui/src-tauri/src/main.rs

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gateway;
mod hotkey;
mod notifications;
mod spotlight;
mod tray;

use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(gateway::GatewayProcess(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            notifications::send_native_notification,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Start gateway process
            gateway::start_gateway(&handle)?;

            // Set up system tray
            tray::create_tray(&handle)?;

            // Register global hotkey
            hotkey::register_global_hotkey(&handle)?;

            // macOS Spotlight integration
            spotlight::register_spotlight_activity(&handle);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide instead of close on window close button (keep in tray)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ClawFT desktop app");
}
```

---

### Task S3.5: Production Hardening (Week 9)

#### S3.5.1: CSP Headers (Rust Middleware)

Content-Security-Policy middleware for static file serving.

```rust
// crates/clawft-services/src/api/middleware/csp.rs

use axum::{
    http::{header, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

/// Content-Security-Policy middleware for static file serving.
///
/// Applies strict CSP headers to prevent XSS and data injection attacks.
pub async fn csp_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(request).await;

    let csp = [
        "default-src 'self'",
        "script-src 'self'",
        "style-src 'self' 'unsafe-inline'",  // shadcn/tailwind needs inline styles
        "img-src 'self' data: blob:",
        "font-src 'self'",
        "connect-src 'self' ws://localhost:18789 wss://localhost:18789",
        "frame-ancestors 'none'",
        "base-uri 'self'",
        "form-action 'self'",
    ]
    .join("; ");

    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_str(&csp).unwrap(),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
}
```

#### S3.5.2: XSS Protection (DOMPurify)

```typescript
// ui/src/lib/sanitize.ts

import DOMPurify from "dompurify";

/**
 * Sanitize user-supplied HTML content to prevent XSS.
 * Used for rendering markdown output, agent messages with HTML, etc.
 */
export function sanitizeHtml(dirty: string): string {
  return DOMPurify.sanitize(dirty, {
    ALLOWED_TAGS: [
      "p", "br", "strong", "em", "code", "pre", "blockquote",
      "ul", "ol", "li", "a", "h1", "h2", "h3", "h4", "h5", "h6",
      "table", "thead", "tbody", "tr", "th", "td",
      "span", "div", "img",
    ],
    ALLOWED_ATTR: ["href", "src", "alt", "class", "title", "target"],
    ALLOW_DATA_ATTR: false,
    ADD_ATTR: ["target"],
  });
}

/**
 * Strip all HTML tags, returning plain text only.
 * Used for notification bodies, meta descriptions, etc.
 */
export function stripHtml(dirty: string): string {
  return DOMPurify.sanitize(dirty, { ALLOWED_TAGS: [] });
}
```

#### S3.5.3: Rate Limiting (Rust Middleware)

```rust
// crates/clawft-services/src/api/middleware/rate_limit.rs

use axum::{
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<RateLimiterInner>>,
}

struct RateLimiterInner {
    requests: HashMap<String, Vec<Instant>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner {
                requests: HashMap::new(),
                max_requests,
                window,
            })),
        }
    }

    async fn check(&self, key: &str) -> bool {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();
        let cutoff = now - inner.window;

        let entries = inner.requests.entry(key.to_string()).or_default();
        entries.retain(|t| *t > cutoff);

        if entries.len() >= inner.max_requests {
            return false;
        }

        entries.push(now);
        true
    }
}

pub async fn rate_limit_middleware<B>(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<B>,
    next: Next<B>,
    limiter: RateLimiter,
) -> Response {
    let key = addr.ip().to_string();

    if !limiter.check(&key).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Try again later.",
        )
            .into_response();
    }

    next.run(request).await
}
```

#### S3.5.4: WebSocket Heartbeat

```rust
// crates/clawft-services/src/api/ws/heartbeat.rs

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::time::interval;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(90);

/// Run WebSocket connection with ping/pong heartbeat.
///
/// Sends ping every 30 seconds. If no pong received within 90 seconds,
/// considers the connection dead and closes it.
pub async fn run_with_heartbeat<F, Fut>(
    socket: WebSocket,
    handler: F,
)
where
    F: FnOnce(WebSocket) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let (mut sender, mut receiver) = socket.split();
    let last_pong = Arc::new(Mutex::new(Instant::now()));

    let pong_tracker = last_pong.clone();

    // Heartbeat sender task
    let heartbeat = tokio::spawn(async move {
        let mut ticker = interval(HEARTBEAT_INTERVAL);
        loop {
            ticker.tick().await;
            if sender.send(Message::Ping(vec![1, 2, 3, 4])).await.is_err() {
                break;
            }

            let elapsed = pong_tracker.lock().await.elapsed();
            if elapsed > CLIENT_TIMEOUT {
                log::warn!("WebSocket client timed out (no pong for {:?})", elapsed);
                let _ = sender.close().await;
                break;
            }
        }
    });

    // Pong receiver wrapper
    let pong_updater = last_pong.clone();
    let wrapped_receiver = async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Pong(_)) => {
                    *pong_updater.lock().await = Instant::now();
                }
                Ok(msg) => {
                    // Forward non-pong messages to handler
                    // In practice, reassemble the socket or use channels
                    let _ = msg;
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = heartbeat => {},
        _ = wrapped_receiver => {},
    }
}

use std::sync::Arc;
use tokio::sync::Mutex;
```

#### S3.5.5: Error Boundary Components

```tsx
// ui/src/components/common/error-boundary.tsx

import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    this.setState({ errorInfo });
    this.props.onError?.(error, errorInfo);
    console.error("[ErrorBoundary]", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <Card className="m-4 border-destructive">
          <CardHeader>
            <CardTitle className="text-destructive">
              Something went wrong
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-muted-foreground">
              {this.state.error?.message ?? "An unexpected error occurred."}
            </p>
            {this.state.errorInfo && (
              <details className="text-xs">
                <summary className="cursor-pointer text-muted-foreground">
                  Stack trace
                </summary>
                <pre className="mt-2 overflow-auto max-h-48 p-2 bg-muted rounded text-xs">
                  {this.state.errorInfo.componentStack}
                </pre>
              </details>
            )}
            <Button variant="outline" onClick={this.handleReset}>
              Try Again
            </Button>
          </CardContent>
        </Card>
      );
    }

    return this.props.children;
  }
}
```

#### S3.5.6: E2E Tests with Playwright

```typescript
// ui/tests/e2e/dashboard.spec.ts

import { test, expect } from "@playwright/test";

test.describe("Dashboard Navigation", () => {
  test.beforeEach(async ({ page }) => {
    // Use mock API mode for E2E tests
    await page.goto("/");
  });

  test("should display sidebar navigation", async ({ page }) => {
    await expect(page.getByRole("navigation")).toBeVisible();
    await expect(page.getByText("Dashboard")).toBeVisible();
    await expect(page.getByText("Agents")).toBeVisible();
    await expect(page.getByText("Chat")).toBeVisible();
    await expect(page.getByText("Canvas")).toBeVisible();
    await expect(page.getByText("Sessions")).toBeVisible();
  });

  test("should navigate to agent management", async ({ page }) => {
    await page.getByText("Agents").click();
    await expect(page).toHaveURL(/.*agents/);
    await expect(page.getByText("Agent Management")).toBeVisible();
  });

  test("should display dashboard health cards", async ({ page }) => {
    await expect(page.getByText("Active Agents")).toBeVisible();
    await expect(page.getByText("Active Sessions")).toBeVisible();
    await expect(page.getByText("Channels")).toBeVisible();
  });

  test("should toggle dark/light theme", async ({ page }) => {
    const themeToggle = page.getByRole("button", { name: /theme/i });
    await themeToggle.click();
    await expect(page.locator("html")).toHaveClass(/dark/);
    await themeToggle.click();
    await expect(page.locator("html")).not.toHaveClass(/dark/);
  });
});

test.describe("WebChat", () => {
  test("should send and receive messages", async ({ page }) => {
    await page.goto("/chat");

    // Select or create a session
    const input = page.getByPlaceholder("Type a message...");
    await expect(input).toBeVisible();

    // Send a message
    await input.fill("Hello, agent!");
    await input.press("Enter");

    // Verify message appears in the chat
    await expect(page.getByText("Hello, agent!")).toBeVisible();

    // Wait for agent response (from mock)
    await expect(
      page.locator('[data-testid="message-bubble-assistant"]').first()
    ).toBeVisible({ timeout: 5000 });
  });

  test("should display tool call cards", async ({ page }) => {
    await page.goto("/chat");

    // Trigger a message that causes a tool call (mock returns tool call)
    const input = page.getByPlaceholder("Type a message...");
    await input.fill("Search for files");
    await input.press("Enter");

    // Tool call card should appear
    await expect(
      page.locator('[data-testid="tool-call-card"]').first()
    ).toBeVisible({ timeout: 5000 });
  });
});

test.describe("Canvas", () => {
  test("should render canvas elements", async ({ page }) => {
    await page.goto("/canvas");

    // Canvas container should be visible
    await expect(page.locator('[data-testid="canvas-renderer"]')).toBeVisible();
  });

  test("should handle canvas interactions", async ({ page }) => {
    await page.goto("/canvas");

    // Wait for a button element to render (from mock)
    const canvasButton = page
      .locator('[data-testid="canvas-element-button"]')
      .first();

    if (await canvasButton.isVisible()) {
      await canvasButton.click();
      // Interaction should be logged
      await expect(page.getByText(/interaction sent/i)).toBeVisible({
        timeout: 3000,
      });
    }
  });

  test("should export canvas as PNG", async ({ page }) => {
    await page.goto("/canvas");

    // Open export menu
    const exportBtn = page.getByRole("button", { name: "Export" });
    if (await exportBtn.isVisible()) {
      await exportBtn.click();
      await page.getByText("Export as PNG").click();

      // Download should have been triggered (check download event)
      const downloadPromise = page.waitForEvent("download");
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/canvas-.*\.png/);
    }
  });
});

test.describe("Auth Flow", () => {
  test("should redirect to login when unauthenticated", async ({ page }) => {
    // Clear any stored tokens
    await page.evaluate(() => localStorage.clear());
    await page.goto("/agents");

    // Should redirect to login or show auth prompt
    await expect(
      page.getByText(/login|authenticate|token/i).first()
    ).toBeVisible({ timeout: 3000 });
  });

  test("should accept token from URL parameter", async ({ page }) => {
    await page.goto("/?token=test-token-12345");

    // Token should be stored and user redirected to dashboard
    const storedToken = await page.evaluate(() =>
      localStorage.getItem("clawft-auth-token")
    );
    expect(storedToken).toBe("test-token-12345");
  });
});
```

#### S3.5.7: Bundle Analysis Configuration

```typescript
// ui/vite.config.ts (additions for bundle analysis)

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { visualizer } from "rollup-plugin-visualizer";

export default defineConfig(({ mode }) => ({
  plugins: [
    react(),
    mode === "analyze" &&
      visualizer({
        open: true,
        filename: "dist/bundle-stats.html",
        gzipSize: true,
        brotliSize: true,
      }),
  ].filter(Boolean),
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Split heavy deps into separate chunks for caching
          "vendor-react": ["react", "react-dom"],
          "vendor-charts": ["recharts"],
          "vendor-editor": ["@monaco-editor/react"],
          "vendor-ui": [
            "@radix-ui/react-dialog",
            "@radix-ui/react-dropdown-menu",
            "@radix-ui/react-tabs",
            "@radix-ui/react-select",
          ],
        },
      },
    },
    target: "es2020",
    minify: "esbuild",
    sourcemap: mode !== "production",
  },
  server: {
    proxy: {
      "/api": { target: "http://localhost:18789" },
      "/ws": { target: "ws://localhost:18789", ws: true },
    },
  },
}));
```

#### S3.5.8: Accessibility (axe-core Integration)

```typescript
// ui/tests/e2e/accessibility.spec.ts

import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const pages = [
  { name: "Dashboard", path: "/" },
  { name: "Agents", path: "/agents" },
  { name: "Chat", path: "/chat" },
  { name: "Canvas", path: "/canvas" },
  { name: "Sessions", path: "/sessions" },
  { name: "Tools", path: "/tools" },
  { name: "Skills", path: "/skills" },
  { name: "Memory", path: "/memory" },
  { name: "Config", path: "/config" },
];

for (const pageInfo of pages) {
  test(`${pageInfo.name} page should have no critical accessibility violations`, async ({
    page,
  }) => {
    await page.goto(pageInfo.path);

    // Wait for page to fully load
    await page.waitForLoadState("networkidle");

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .exclude(".monaco-editor") // Monaco has its own a11y handling
      .analyze();

    // Filter to critical and serious violations only
    const critical = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious"
    );

    if (critical.length > 0) {
      const summary = critical
        .map(
          (v) =>
            `[${v.impact}] ${v.id}: ${v.description} (${v.nodes.length} instances)`
        )
        .join("\n");
      console.error(`Accessibility violations on ${pageInfo.name}:\n${summary}`);
    }

    expect(critical).toHaveLength(0);
  });
}
```

#### S3.5.9: Tailscale Auth Provider (Rust Middleware)

X-Tailscale-User-* header extraction middleware for multi-user authentication via Tailscale.

```rust
// crates/clawft-services/src/api/middleware/tailscale_auth.rs

use axum::{
    extract::Request,
    http::{header::HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// User identity extracted from Tailscale proxy headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleUser {
    pub login: String,
    pub name: String,
    pub profile_pic: Option<String>,
    pub tailnet: Option<String>,
}

impl TailscaleUser {
    /// Extract user identity from Tailscale proxy headers.
    ///
    /// Tailscale HTTPS proxy sets these headers:
    /// - X-Tailscale-User-Login: user@example.com
    /// - X-Tailscale-User-Name: Display Name
    /// - X-Tailscale-User-Profile-Pic: https://...
    /// - X-Tailscale-User-Tailnet: example.com
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let login = headers
            .get("X-Tailscale-User-Login")?
            .to_str()
            .ok()?
            .to_string();

        let name = headers
            .get("X-Tailscale-User-Name")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(&login)
            .to_string();

        let profile_pic = headers
            .get("X-Tailscale-User-Profile-Pic")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let tailnet = headers
            .get("X-Tailscale-User-Tailnet")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Some(Self {
            login,
            name,
            profile_pic,
            tailnet,
        })
    }
}

/// Middleware that extracts Tailscale user identity from proxy headers.
///
/// If Tailscale headers are present, the user identity is stored as
/// a request extension. If not present and `require_tailscale` is true,
/// returns 401 Unauthorized.
pub async fn tailscale_auth_middleware(
    request: Request,
    next: Next,
) -> Response {
    let user = TailscaleUser::from_headers(request.headers());

    // If running behind Tailscale, headers will be present.
    // If not behind Tailscale (local dev), fall through to Bearer auth.
    if let Some(user) = user {
        let mut request = request;
        request.extensions_mut().insert(user);
        next.run(request).await
    } else {
        // No Tailscale headers -- fall through to next auth layer
        next.run(request).await
    }
}

/// Strict Tailscale auth -- rejects requests without Tailscale identity.
pub async fn require_tailscale_middleware(
    request: Request,
    next: Next,
) -> Response {
    let user = TailscaleUser::from_headers(request.headers());

    match user {
        Some(user) => {
            let mut request = request;
            request.extensions_mut().insert(user);
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            "Tailscale authentication required. Access this service via your Tailscale network.",
        )
            .into_response(),
    }
}
```

#### S3.5.10: Multi-User Session Isolation

Per-user session scoping via auth context.

```rust
// crates/clawft-services/src/api/middleware/session_isolation.rs

use axum::{
    extract::{Path, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::tailscale_auth::TailscaleUser;

/// Middleware that scopes session access to the authenticated user.
///
/// When multi-user mode is enabled (Tailscale auth active), users can
/// only access sessions they own. Session ownership is determined by
/// the session's `owner` field matching the authenticated user's login.
pub async fn session_isolation_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Extract user from request extensions (set by tailscale_auth or bearer_auth)
    let user = request.extensions().get::<TailscaleUser>().cloned();

    // If no user identity (single-user mode), allow all access
    if user.is_none() {
        return next.run(request).await;
    }

    let user = user.unwrap();

    // For session-specific routes, verify ownership
    let path = request.uri().path().to_string();
    if path.starts_with("/api/sessions/") {
        // Extract session key from path
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 4 {
            let session_key = parts[3];

            // Session keys are prefixed with user login in multi-user mode
            // Format: "user@example.com:session-key"
            let expected_prefix = format!("{}:", user.login);
            if !session_key.starts_with(&expected_prefix) && session_key.contains(':') {
                return (
                    StatusCode::FORBIDDEN,
                    "Access denied: session belongs to another user.",
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}
```

```tsx
// ui/src/lib/auth-context.tsx

import { createContext, useContext, useState, useEffect, useMemo } from "react";
import { useApiClient } from "@/hooks/use-api";

interface AuthUser {
  login: string;
  name: string;
  profilePic?: string;
  tailnet?: string;
  authMethod: "token" | "tailscale";
}

interface AuthContextValue {
  user: AuthUser | null;
  isMultiUser: boolean;
  sessionPrefix: string;
  isLoading: boolean;
}

const AuthContext = createContext<AuthContextValue>({
  user: null,
  isMultiUser: false,
  sessionPrefix: "",
  isLoading: true,
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const { apiClient } = useApiClient();

  useEffect(() => {
    apiClient
      .get<{ user: AuthUser; multiUser: boolean }>("/api/auth/verify")
      .then((data) => {
        setUser(data.user);
      })
      .catch(() => {
        setUser(null);
      })
      .finally(() => {
        setIsLoading(false);
      });
  }, [apiClient]);

  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      isMultiUser: user?.authMethod === "tailscale",
      sessionPrefix: user ? `${user.login}:` : "",
      isLoading,
    }),
    [user, isLoading]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  return useContext(AuthContext);
}
```

---

## 4. Tasks

### 4.1 Consolidated Task Table

| # | Task ID | Phase | Description | Priority | Location | Est. |
|---|---------|-------|-------------|----------|----------|------|
| 1 | S3.1.1 | S3.1 Delegation | Delegation monitor: active delegations table with WS real-time updates | P1 | `ui/src/components/delegation/delegation-monitor.tsx` | 4h |
| 2 | S3.1.2 | S3.1 Delegation | Rule editor: regex pattern, target selector, complexity threshold slider, zod validation | P1 | `ui/src/components/delegation/delegation-rule-editor.tsx` | 6h |
| 3 | S3.1.3 | S3.1 Delegation | Pipeline inspector: 6-stage real-time timing bar visualization per session | P1 | `ui/src/components/monitoring/pipeline-inspector.tsx` | 8h |
| 4 | S3.1.4 | S3.1 Delegation | Token usage dashboard: per-session/provider/model usage charts via recharts | P1 | `ui/src/components/monitoring/token-usage-dashboard.tsx` | 6h |
| 5 | S3.1.5 | S3.1 Delegation | Cost tracker: daily/weekly/monthly cost breakdown by provider, date range selector | P1 | `ui/src/components/monitoring/cost-tracker.tsx` | 6h |
| 6 | S3.2.1 | S3.2 Canvas | Chart rendering: bar/line/pie chart Canvas element type via recharts | P2 | `ui/src/components/canvas/elements/canvas-chart-element.tsx` | 6h |
| 7 | S3.2.2 | S3.2 Canvas | Code editor element: Monaco lazy-loaded Canvas element with language + submit | P2 | `ui/src/components/canvas/elements/canvas-code-element.tsx` | 8h |
| 8 | S3.2.3 | S3.2 Canvas | Form builder: agent-driven multi-field form with zod schema validation | P2 | `ui/src/components/canvas/elements/canvas-form-element.tsx` | 8h |
| 9 | S3.2.4 | S3.2 Canvas | Canvas history: undo/redo stack with Zustand store, timeline slider, replay | P2 | `ui/src/stores/canvas-history-store.ts` | 6h |
| 10 | S3.2.5 | S3.2 Canvas | Canvas export: PNG (html2canvas), HTML serialization, JSON state download | P2 | `ui/src/components/canvas/canvas-export.tsx` | 4h |
| 11 | S3.3.1 | S3.3 Mobile | Responsive sidebar: Sheet/drawer on mobile (<768px), collapsible on tablet/desktop | P2 | `ui/src/components/layout/responsive-sidebar.tsx` | 4h |
| 12 | S3.3.2 | S3.3 Mobile | Mobile WebChat: bottom-anchored input, swipe session navigation (react-swipeable) | P2 | `ui/src/components/chat/mobile-web-chat.tsx` | 6h |
| 13 | S3.3.3 | S3.3 Mobile | PWA manifest: `manifest.json` with app metadata, icons, display standalone | P2 | `ui/public/manifest.json` | 2h |
| 14 | S3.3.4 | S3.3 Mobile | Service worker: static asset caching, offline shell, cache versioning | P2 | `ui/public/service-worker.js` | 4h |
| 15 | S3.3.5 | S3.3 Mobile | Push notifications: WS event bridge to browser Notification API | P2 | `ui/src/lib/push-notifications.ts` | 4h |
| 16 | S3.4.1 | S3.4 Tauri | Tauri project init: `cargo tauri init`, `tauri.conf.json`, dev + build scripts | P2 | `ui/src-tauri/` | 4h |
| 17 | S3.4.2 | S3.4 Tauri | System tray icon: status indicator (green/yellow/red), menu (Show/Status/Quit) | P2 | `ui/src-tauri/src/tray.rs` | 4h |
| 18 | S3.4.3 | S3.4 Tauri | Global hotkey: Cmd+Shift+W (macOS) / Ctrl+Shift+W (Win/Linux) toggle visibility | P2 | `ui/src-tauri/src/hotkey.rs` | 2h |
| 19 | S3.4.4 | S3.4 Tauri | Auto-start gateway: spawn `weft gateway` as sidecar on app launch, manage lifecycle | P2 | `ui/src-tauri/src/gateway.rs` | 6h |
| 20 | S3.4.5 | S3.4 Tauri | macOS Spotlight: register Spotlight Importers for quick agent queries (best-effort) | P3 | `ui/src-tauri/src/spotlight.rs` | 4h |
| 21 | S3.4.6 | S3.4 Tauri | Native notifications: bridge WS events to OS notification center via Tauri API | P2 | `ui/src-tauri/src/notifications.rs` | 4h |
| 22 | S3.4.7 | S3.4 Tauri | Tauri main entry: wire tray, hotkey, gateway, notifications into `main.rs` setup | P2 | `ui/src-tauri/src/main.rs` | 2h |
| 23 | S3.5.1 | S3.5 Hardening | CSP headers: Content-Security-Policy middleware with strict policy | P1 | `crates/clawft-services/src/api/middleware/csp.rs` | 4h |
| 24 | S3.5.2 | S3.5 Hardening | XSS protection: DOMPurify integration for user/agent content sanitization | P1 | `ui/src/lib/sanitize.ts` | 2h |
| 25 | S3.5.3 | S3.5 Hardening | Rate limiting: per-IP sliding window middleware (in-memory, configurable limits) | P1 | `crates/clawft-services/src/api/middleware/rate_limit.rs` | 6h |
| 26 | S3.5.4 | S3.5 Hardening | WebSocket heartbeat: 30s ping interval, 90s dead connection cleanup | P1 | `crates/clawft-services/src/api/ws_heartbeat.rs` | 4h |
| 27 | S3.5.5 | S3.5 Hardening | Error boundaries: React ErrorBoundary with fallback UI, reset, and error reporting | P1 | `ui/src/components/common/error-boundary.tsx` | 4h |
| 28 | S3.5.6 | S3.5 Hardening | E2E Playwright tests: dashboard, WebChat, Canvas, auth flow test suites | P1 | `ui/tests/e2e/` | 12h |
| 29 | S3.5.7 | S3.5 Hardening | Bundle analysis: `rollup-plugin-visualizer` config, vendor chunk splitting | P2 | `ui/vite.config.ts` | 2h |
| 30 | S3.5.8 | S3.5 Hardening | Accessibility audit: axe-core Playwright integration, WCAG AA automated checks | P1 | `ui/tests/e2e/accessibility.spec.ts` | 4h |
| 31 | S3.5.9 | S3.5 Hardening | Tailscale auth: X-Tailscale-User header extraction and validation middleware | P1 | `crates/clawft-services/src/api/middleware/tailscale.rs` | 4h |
| 32 | S3.5.10 | S3.5 Hardening | Multi-user session isolation: per-user session key prefixing and access control | P1 | `crates/clawft-services/src/api/middleware/session_isolation.rs` + `ui/src/lib/auth-context.tsx` | 6h |

**Subtotal S3.1-S3.5: 32 tasks** (5 delegation + 5 canvas + 5 mobile/PWA + 7 Tauri + 10 production hardening)

**Subtotal S3.1-S3.5 effort: 160 hours** (~20 developer-days across Weeks 7-9)

**S3.6 Browser WASM: 12 tasks, 57 hours** (~7 developer-days, Weeks 9-10)

**S3.7 Documentation: 4 tasks, 22 hours** (~3 developer-days, Week 10)

**Grand total S3: 48 tasks, 239 hours** (~30 developer-days across 4 weeks)

### 4.2 Concurrency Plan

S3 tasks can be parallelized across three tracks:

| Track | Tasks | Dependencies |
|-------|-------|-------------|
| **Frontend Polish** | S3.1, S3.2, S3.3 (React components) | S1/S2 complete; can use MSW mocks for M1/M2/D5/D6 data |
| **Tauri Desktop** | S3.4 (Rust + Tauri config) | S1/S2 `ui/dist/` available; independent of backend changes |
| **Production Hardening** | S3.5 (Rust middleware + Playwright tests) | S1/S2 API routes exist; CSP/rate-limit are additive middleware |

Within each track:

- **S3.1** (delegation + monitoring): All 5 components are independent React components. Can be built in parallel by up to 3 developers.
- **S3.2** (advanced Canvas): Chart, code editor, form builder, history, and export are independent. Can parallelize all 5.
- **S3.3** (mobile + PWA): Responsive sidebar and mobile chat can run in parallel. PWA manifest + service worker are independent.
- **S3.4** (Tauri): Sequential -- scaffold first (S3.4.1), then tray/hotkey/gateway in parallel (S3.4.2-S3.4.4), then platform-specific (S3.4.5-S3.4.6).
- **S3.5** (hardening): CSP, rate limiting, heartbeat, and Tailscale auth are independent Axum middleware layers. Error boundaries and Playwright tests can run in parallel.

### 4.3 Critical Path

```
Week 7:  S3.1.1-S3.1.5 (Delegation) + S3.2.1-S3.2.3 (Chart/Code/Form)
         |
Week 8:  S3.2.4-S3.2.5 (History/Export) + S3.3.1-S3.3.5 (Mobile/PWA)
         + S3.4.1-S3.4.4 (Tauri scaffold + tray/hotkey/gateway)
         |
Week 9:  S3.4.5-S3.4.7 (Tauri platform-specific + entry)
         + S3.5.1-S3.5.10 (Production hardening + E2E + accessibility)
         + S3.6.1-S3.6.6 (Backend adapter interface + WASM adapter)
         |
Week 10: S3.6.7-S3.6.12 (Browser config UI + hook migration + caching)
         + S3.7.1-S3.7.4 (Documentation)
```

### 4.4 S3.6 Concurrency Notes

S3.6 (Browser WASM Integration) can run in parallel with S3.5 (Production Hardening):
- S3.6.1-S3.6.5 (adapter interface, feature detection) are independent of S3.5
- S3.6.6 (ModeProvider) requires S1.2 scaffolding but NOT S3.5 middleware
- S3.6.7 (Browser config UI) is independent
- S3.6.10 (hook migration) is the critical bottleneck -- touches all existing hooks
- S3.6.3 (WasmAdapter) can be developed against a mock WASM module until W-BROWSER Phase 5 delivers

---

## 5. Tests Required

### S3.1 Tests -- Delegation & Monitoring

| Test | Description |
|------|-------------|
| `test_delegation_monitor_renders_empty_state` | DelegationMonitor renders "No active delegations" when no events received. |
| `test_delegation_monitor_adds_active_delegation` | Simulate `delegation:start` WS event; verify row appears with correct target and "running" status. |
| `test_delegation_monitor_completes_delegation` | Simulate `delegation:start` then `delegation:complete`; verify status changes to "completed" and latency shown. |
| `test_delegation_rule_editor_validates_regex` | Enter invalid regex in pattern field; verify form shows validation error. |
| `test_delegation_rule_editor_submits_valid_rule` | Fill all fields with valid data; submit; verify API call made with correct payload. |
| `test_pipeline_inspector_renders_stages` | Simulate `pipeline:stage` events for all 6 stages; verify timing bars render with correct widths. |
| `test_token_usage_renders_charts` | Mock `/api/monitoring/token-usage` response; verify BarChart and PieChart render. |
| `test_cost_tracker_switches_time_range` | Click daily/weekly/monthly tabs; verify API called with correct `range` parameter. |

### S3.2 Tests -- Advanced Canvas

| Test | Description |
|------|-------------|
| `test_canvas_chart_renders_bar` | Provide bar chart data; verify recharts BarChart renders with correct bars. |
| `test_canvas_chart_renders_pie` | Provide pie chart data; verify PieChart renders with correct segments. |
| `test_canvas_code_element_mounts_monaco` | Render CanvasCodeElement; verify Monaco editor mounts with correct language. |
| `test_canvas_code_element_submits_value` | Edit code in Monaco; click submit; verify onSubmit called with current value. |
| `test_canvas_form_renders_all_field_types` | Provide form spec with text, number, select, boolean fields; verify all render. |
| `test_canvas_form_validates_required_fields` | Submit form with empty required fields; verify validation errors shown. |
| `test_canvas_history_undo_redo` | Push 3 snapshots; undo twice; verify currentIndex is 0; redo once; verify currentIndex is 1. |
| `test_canvas_history_truncates_on_new_action` | Push 3 snapshots; undo to index 1; push new snapshot; verify redo history is truncated. |
| `test_canvas_export_png_triggers_download` | Click "Export as PNG"; verify html2canvas called and download triggered. |
| `test_canvas_export_json_matches_state` | Click "Export as JSON"; verify downloaded JSON contains current elements and commands. |

### S3.3 Tests -- Mobile + PWA

| Test | Description |
|------|-------------|
| `test_responsive_sidebar_mobile_drawer` | Set viewport to 375px; verify sidebar renders as Sheet/drawer. |
| `test_responsive_sidebar_desktop_expanded` | Set viewport to 1280px; verify sidebar renders expanded. |
| `test_mobile_chat_send_message` | Render MobileWebChat; type message; press Enter; verify onSend called. |
| `test_mobile_chat_swipe_navigation` | Simulate swipe left; verify onSessionChange called with next session. |
| `test_pwa_manifest_valid` | Validate manifest.json has required fields: name, start_url, display, icons. |
| `test_service_worker_caches_static_assets` | Install service worker; verify STATIC_ASSETS are cached. |

### S3.4 Tests -- Tauri Desktop

| Test | Description |
|------|-------------|
| `test_gateway_process_starts` | Call `start_gateway`; verify child process spawned. |
| `test_gateway_process_stops_on_cleanup` | Start then stop gateway; verify child process killed. |
| `test_tray_creates_with_menu` | Call `create_tray`; verify tray icon created with Show/Status/Quit items. |
| `test_notification_payload_deserializes` | Deserialize NotificationPayload from JSON; verify fields correct. |

### S3.5 Tests -- Production Hardening

| Test | Description |
|------|-------------|
| `test_csp_headers_present` | Send request through csp_middleware; verify Content-Security-Policy header set. |
| `test_csp_includes_frame_ancestors_none` | Verify CSP includes `frame-ancestors 'none'`. |
| `test_rate_limiter_allows_under_limit` | Make 99 requests (limit 100); all should return 200. |
| `test_rate_limiter_blocks_over_limit` | Make 101 requests (limit 100); 101st should return 429. |
| `test_rate_limiter_resets_after_window` | Make 100 requests; wait for window to expire; next request should succeed. |
| `test_tailscale_user_from_headers` | Construct HeaderMap with X-Tailscale-User-* headers; verify TailscaleUser extracted. |
| `test_tailscale_user_missing_login_returns_none` | HeaderMap without X-Tailscale-User-Login; verify None returned. |
| `test_require_tailscale_rejects_without_headers` | Send request without Tailscale headers; verify 401 returned. |
| `test_session_isolation_allows_own_session` | User "alice@example.com" accesses "alice@example.com:session1"; verify allowed. |
| `test_session_isolation_blocks_other_session` | User "alice@example.com" accesses "bob@example.com:session1"; verify 403 returned. |
| `test_error_boundary_catches_render_error` | Component throws during render; verify error boundary shows fallback. |
| `test_error_boundary_reset_clears_error` | Click "Try Again" in error boundary; verify children re-render. |
| `test_sanitize_html_strips_script_tags` | Input contains `<script>`; verify output has no script tags. |
| `test_sanitize_html_allows_safe_tags` | Input contains `<strong>`, `<a>`, `<code>`; verify these are preserved. |
| `test_e2e_dashboard_navigation` | Playwright: navigate sidebar links; verify correct routes load. |
| `test_e2e_webchat_send_receive` | Playwright: send message; verify response appears. |
| `test_e2e_canvas_render_interact` | Playwright: verify canvas elements render; click button element. |
| `test_e2e_auth_flow` | Playwright: verify token-based auth redirects work. |
| `test_accessibility_no_critical_violations` | Playwright + axe-core: verify no critical WCAG AA violations on all pages. |

---

## 6. Acceptance Criteria

- [ ] Delegation monitor shows real-time active delegations with target, status, latency
- [ ] Delegation rule editor validates regex patterns and submits rules via API
- [ ] Pipeline inspector visualizes 6-stage pipeline with timing bars per session
- [ ] Token usage dashboard renders per-session and per-provider charts
- [ ] Cost tracker shows daily/weekly/monthly cost breakdown by provider
- [ ] Canvas supports chart elements (bar, line, pie) via recharts
- [ ] Canvas supports embedded Monaco code editor with submit action
- [ ] Canvas supports dynamic form builder with validation
- [ ] Canvas undo/redo works correctly with timeline slider
- [ ] Canvas exports to PNG, HTML, and JSON
- [ ] Sidebar is responsive: drawer on mobile, collapsible on tablet/desktop
- [ ] Mobile WebChat has bottom-anchored input and swipe session navigation
- [ ] PWA manifest is valid and service worker caches static assets
- [ ] Push notifications work for agent errors, delegation completions, cron fires
- [ ] Tauri project builds for macOS, Linux, Windows
- [ ] System tray shows agent status indicator
- [ ] Global hotkey (Cmd+Shift+W / Ctrl+Shift+W) toggles window
- [ ] Gateway auto-starts on Tauri app launch
- [ ] CSP headers present on all static file responses
- [ ] XSS protection via DOMPurify on user content rendering
- [ ] Rate limiting returns 429 when limit exceeded
- [ ] WebSocket heartbeat pings every 30s; dead connections cleaned up after 90s
- [ ] Error boundaries catch render errors and show graceful fallback
- [ ] Playwright E2E tests pass for dashboard, WebChat, Canvas, auth flow
- [ ] Bundle analysis produces report; vendor chunks split correctly
- [ ] No critical or serious WCAG AA accessibility violations (axe-core)
- [ ] Tailscale auth extracts user identity from X-Tailscale-User-* headers
- [ ] Multi-user session isolation prevents cross-user session access
- [ ] `pnpm build` produces bundle under 200KB gzipped (excluding vendor chunks)
- [ ] `pnpm test` -- all unit tests pass
- [ ] `pnpm test:e2e` -- all Playwright tests pass

### S3.6 Browser WASM Integration

- [ ] `BackendAdapter` interface is implemented by both `AxumAdapter` and `WasmAdapter`
- [ ] Existing UI hooks use `useBackend()` instead of direct api-client calls
- [ ] WASM module loads in browser with visible progress indicator
- [ ] Browser config UI encrypts API key with Web Crypto before IndexedDB storage
- [ ] WebChat works end-to-end in WASM mode (user -> WASM pipeline -> LLM -> response)
- [ ] Routes unavailable in WASM mode are hidden from navigation
- [ ] Feature detection warns users about missing browser capabilities
- [ ] Auto-detection correctly falls back from Axum to WASM when server is unreachable
- [ ] Service worker caches .wasm binary for offline support

### S3.7 Documentation

- [ ] UI developer guide covers project setup through contribution workflow
- [ ] API reference documents all REST and WS endpoints with examples
- [ ] Browser mode guide includes per-provider CORS setup instructions
- [ ] Deployment guide covers Docker, CDN, reverse proxy, Tauri, single-binary, and browser-only modes

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Monaco editor bundle size (~2MB) blows budget | High | Medium | Dynamic import via `React.lazy()`; separate vendor chunk; only loaded on Canvas pages with code elements. |
| Tauri cross-platform differences (tray, hotkey, sidecar) | Medium | Medium | Feature-flag platform-specific code (`#[cfg(target_os)]`). Test on all 3 platforms in CI. macOS Spotlight is best-effort. |
| html2canvas incomplete rendering for complex Canvas | Medium | Low | Canvas export is best-effort. Complex elements (Monaco, recharts) may not render perfectly in PNG. Offer JSON export as reliable alternative. |
| Service worker caching causes stale UI after deploy | Medium | Medium | Version the cache name (`clawft-v1`). Activate event cleans old caches. Users see update on next reload. |
| Rate limiter state lost on server restart (in-memory) | Low | Low | Acceptable for single-instance deployment. For multi-instance, migrate to Redis-backed rate limiter post-MVP. |
| Tailscale header spoofing when not behind Tailscale proxy | Medium | High | CSP headers prevent JS injection. Backend should only trust Tailscale headers when `tailscale_auth_required: true` is set in config AND the server binds to Tailscale interface only. Document in deployment guide. |
| react-swipeable conflicts with Canvas touch events | Low | Medium | Only enable swipe handlers in MobileWebChat; Canvas page disables swipe navigation. |
| Playwright tests flaky due to WebSocket timing | Medium | Low | Use MSW mock mode for E2E; mock WebSocket with deterministic responses. Add `waitForSelector` with generous timeouts. |
| WASM binary size exceeds 500KB gzipped | Medium | Medium | Tree-shake with `wasm-opt -Oz`. Service worker caches .wasm binary. Use `WebAssembly.compileStreaming()` for parallel download+compile. |
| CORS blocks direct LLM calls in browser mode | High | High | Anthropic supports direct browser access header. Other providers require CORS proxy. Config UI provides prominent setup guidance. |
| W-BROWSER Phase 5 WASM entry points not ready for S3.6 | Medium | Medium | S3.6 adapter can be developed against mock WASM module. Real integration tested when W-BROWSER delivers. |
| API key exposure risk in browser IndexedDB | Medium | High | Web Crypto AES-256-GCM encryption. Non-extractable CryptoKey. UI warns users about browser key exposure. |
| WASM init blocks main thread causing jank | Medium | Medium | Loading spinner during compile+init. Move to Web Worker if init > 2s. `compileStreaming()` for parallel download. |

---

## 8. Phase S3.6: Browser WASM Integration (Weeks 9-10)

> **CRITICAL**: This phase is the bridge between W-BROWSER and W-UI workstreams. It enables the
> dashboard to run in two modes: connected to an Axum backend (server mode) or connected to a
> WASM module loaded in the browser (browser-only mode, no server required).

### 8.1 Overview

Phase S3.6 introduces a backend adapter abstraction that decouples the React UI from the specific
transport layer. All existing views (Dashboard, Agents, WebChat, Canvas, Sessions, Tools, Memory,
Config) work identically regardless of whether data flows through REST/WebSocket to an Axum
server or through wasm-bindgen to an in-browser WASM module running `AgentLoop<BrowserPlatform>`.

The WASM module is built by the W-BROWSER workstream (Phases 1-5). S3.6 consumes the WASM
entry points defined in `.planning/wasm-browser/04-browser-platform-spec.md`:

- `init(config_json: &str) -> Result<(), JsValue>` -- Initialize platform and agent loop
- `send_message(text: &str) -> Result<String, JsValue>` -- Send message through pipeline
- `set_env(key: &str, value: &str)` -- Set environment variable
- `on_response(callback: JsValue)` -- Register streaming response callback (future)

### 8.2 Current Code (Prerequisites)

From S1/S2/S3.1-S3.5:

- `ui/src/lib/api-client.ts` -- Fetch wrapper with Bearer auth, configurable `VITE_API_URL`
- `ui/src/lib/ws-client.ts` -- Reconnecting WebSocket with exponential backoff
- `ui/src/hooks/use-api.ts` -- React hook providing apiClient instance
- `ui/src/hooks/use-websocket.ts` -- React hook providing WebSocket subscription
- `ui/src/stores/*.ts` -- Zustand stores for agent, session, canvas, delegation state
- `ui/src/mocks/*.ts` -- MSW mock handlers for all API endpoints

From W-BROWSER:

- `crates/clawft-wasm/src/lib.rs` -- WASM entry points (`init`, `send_message`, `set_env`)
- `crates/clawft-wasm/pkg/` -- wasm-pack output (clawft_wasm.js + clawft_wasm_bg.wasm)
- `crates/clawft-platform/src/browser/` -- `BrowserPlatform` (HttpClient, FileSystem, Environment)

### 8.3 Deliverables

#### 8.3.1 Backend Adapter Interface

**File**: `ui/src/lib/backend-adapter.ts`

The adapter interface defines the contract that both Axum and WASM backends must satisfy.
All existing hooks and stores call through this interface rather than directly using
`api-client.ts` or `ws-client.ts`.

```typescript
// ui/src/lib/backend-adapter.ts

export type BackendMode = "axum" | "wasm" | "mock";

export interface AgentInfo {
  id: string;
  name: string;
  status: "running" | "stopped" | "error";
  model: string;
}

export interface SessionInfo {
  key: string;
  messageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface ToolInfo {
  name: string;
  description: string;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
  toolCalls?: ToolCallRecord[];
}

export interface ToolCallRecord {
  toolName: string;
  arguments: Record<string, unknown>;
  result?: string;
}

export interface MemoryEntry {
  key: string;
  value: string;
  namespace: string;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

export interface BackendCapabilities {
  /** Whether this backend supports channel management (Axum only). */
  channels: boolean;
  /** Whether this backend supports cron scheduling (Axum only). */
  cron: boolean;
  /** Whether this backend supports delegation monitoring (Axum only). */
  delegation: boolean;
  /** Whether this backend supports multi-user auth (Axum only). */
  multiUser: boolean;
  /** Whether this backend supports skill installation from ClawHub (Axum only). */
  skillInstall: boolean;
  /** Whether this backend supports real-time WebSocket events. */
  realtime: boolean;
  /** Whether the backend is fully initialized and ready. */
  ready: boolean;
}

/**
 * Abstract interface for UI-to-backend communication.
 *
 * Implemented by AxumAdapter (REST/WS) and WasmAdapter (wasm-bindgen).
 * All React hooks and stores call through this interface.
 */
export interface BackendAdapter {
  /** Which mode this adapter operates in. */
  readonly mode: BackendMode;

  /** What this backend supports. */
  readonly capabilities: BackendCapabilities;

  /** Initialize the backend. Called once at app startup. */
  init(config?: Record<string, unknown>): Promise<void>;

  /** Shut down the backend gracefully. */
  dispose(): Promise<void>;

  // -- Agents --
  listAgents(): Promise<AgentInfo[]>;
  getAgent(id: string): Promise<AgentInfo | null>;
  startAgent(id: string): Promise<void>;
  stopAgent(id: string): Promise<void>;

  // -- Sessions --
  listSessions(): Promise<SessionInfo[]>;
  getSessionMessages(key: string): Promise<ChatMessage[]>;
  deleteSession(key: string): Promise<void>;

  // -- Chat --
  sendMessage(sessionKey: string, content: string): Promise<ChatMessage>;
  onMessage(callback: (msg: ChatMessage) => void): () => void;

  // -- Tools --
  listTools(): Promise<ToolInfo[]>;
  getToolSchema(name: string): Promise<Record<string, unknown> | null>;

  // -- Memory --
  listMemory(namespace?: string): Promise<MemoryEntry[]>;
  searchMemory(query: string, namespace?: string, threshold?: number): Promise<MemoryEntry[]>;
  writeMemory(key: string, value: string, namespace?: string, tags?: string[]): Promise<void>;
  deleteMemory(key: string, namespace?: string): Promise<void>;

  // -- Config (read-only in WASM mode) --
  getConfig(): Promise<Record<string, unknown>>;
  updateConfig?(patch: Record<string, unknown>): Promise<void>;

  // -- Events (noop in WASM mode without WS) --
  subscribe?(topics: string[]): void;
  unsubscribe?(topics: string[]): void;
  onEvent?(callback: (event: { topic: string; data: unknown }) => void): () => void;
}
```

#### 8.3.2 Axum Adapter

**File**: `ui/src/lib/adapters/axum-adapter.ts`

Wraps the existing `api-client.ts` and `ws-client.ts` behind the `BackendAdapter` interface.
This is a thin adapter -- most methods delegate directly to the existing fetch/WS clients.

```typescript
// ui/src/lib/adapters/axum-adapter.ts

import type { BackendAdapter, BackendCapabilities, BackendMode } from "../backend-adapter";
import { ApiClient } from "../api-client";
import { WsClient } from "../ws-client";

export class AxumAdapter implements BackendAdapter {
  readonly mode: BackendMode = "axum";
  readonly capabilities: BackendCapabilities = {
    channels: true,
    cron: true,
    delegation: true,
    multiUser: true,
    skillInstall: true,
    realtime: true,
    ready: false,
  };

  private api: ApiClient;
  private ws: WsClient;

  constructor(apiUrl: string, wsUrl: string, token?: string) {
    this.api = new ApiClient(apiUrl, token);
    this.ws = new WsClient(wsUrl, token);
  }

  async init(): Promise<void> {
    this.ws.connect();
    (this.capabilities as { ready: boolean }).ready = true;
  }

  async dispose(): Promise<void> {
    this.ws.disconnect();
    (this.capabilities as { ready: boolean }).ready = false;
  }

  // Each method delegates to this.api.get/post/patch/delete
  // and this.ws.subscribe/onMessage as appropriate.
  // Full implementation follows existing api-client.ts patterns.
  // ...
}
```

#### 8.3.3 WASM Adapter

**File**: `ui/src/lib/adapters/wasm-adapter.ts`

Bridges the React UI to the in-browser WASM module. Loads the WASM binary,
calls `init(config_json)`, and translates `BackendAdapter` method calls into
`send_message()` invocations with appropriate routing.

```typescript
// ui/src/lib/adapters/wasm-adapter.ts

import type {
  BackendAdapter,
  BackendCapabilities,
  BackendMode,
  ChatMessage,
  AgentInfo,
  SessionInfo,
  ToolInfo,
  MemoryEntry,
} from "../backend-adapter";

interface ClawftWasm {
  init(config_json: string): Promise<void>;
  send_message(text: string): Promise<string>;
  set_env(key: string, value: string): void;
}

export class WasmAdapter implements BackendAdapter {
  readonly mode: BackendMode = "wasm";
  readonly capabilities: BackendCapabilities = {
    channels: false,
    cron: false,
    delegation: false,
    multiUser: false,
    skillInstall: false,
    realtime: false, // No WebSocket in browser-only mode
    ready: false,
  };

  private wasm: ClawftWasm | null = null;
  private messageCallbacks: Array<(msg: ChatMessage) => void> = [];
  private sessions: Map<string, ChatMessage[]> = new Map();
  private config: Record<string, unknown> = {};

  constructor(
    private wasmUrl: string = "/clawft_wasm.js",
    private onProgress?: (phase: string, pct: number) => void,
  ) {}

  async init(config?: Record<string, unknown>): Promise<void> {
    this.onProgress?.("download", 0);

    // Dynamic import of wasm-bindgen generated JS
    const wasmModule = await import(/* @vite-ignore */ this.wasmUrl);

    this.onProgress?.("compile", 30);
    await wasmModule.default(); // Loads and compiles .wasm binary

    this.onProgress?.("init", 70);
    this.wasm = wasmModule;

    if (config) {
      this.config = config;
      await this.wasm!.init(JSON.stringify(config));
    }

    this.onProgress?.("ready", 100);
    (this.capabilities as { ready: boolean }).ready = true;
  }

  async dispose(): Promise<void> {
    this.wasm = null;
    (this.capabilities as { ready: boolean }).ready = false;
  }

  // -- Chat (primary interaction) --

  async sendMessage(sessionKey: string, content: string): Promise<ChatMessage> {
    if (!this.wasm) throw new Error("WASM module not initialized");

    const userMsg: ChatMessage = {
      role: "user",
      content,
      timestamp: new Date().toISOString(),
    };

    // Store user message in local session
    const messages = this.sessions.get(sessionKey) ?? [];
    messages.push(userMsg);
    this.sessions.set(sessionKey, messages);

    // Send through WASM pipeline
    const responseText = await this.wasm.send_message(content);

    const assistantMsg: ChatMessage = {
      role: "assistant",
      content: responseText,
      timestamp: new Date().toISOString(),
    };

    messages.push(assistantMsg);
    this.messageCallbacks.forEach((cb) => cb(assistantMsg));

    return assistantMsg;
  }

  onMessage(callback: (msg: ChatMessage) => void): () => void {
    this.messageCallbacks.push(callback);
    return () => {
      this.messageCallbacks = this.messageCallbacks.filter((cb) => cb !== callback);
    };
  }

  // -- Agents (single agent in WASM mode) --

  async listAgents(): Promise<AgentInfo[]> {
    return [
      {
        id: "browser-agent",
        name: "Browser Agent",
        status: this.capabilities.ready ? "running" : "stopped",
        model: (this.config as Record<string, Record<string, string>>)
          ?.defaults?.model ?? "unknown",
      },
    ];
  }

  async getAgent(id: string): Promise<AgentInfo | null> {
    const agents = await this.listAgents();
    return agents.find((a) => a.id === id) ?? null;
  }

  async startAgent(_id: string): Promise<void> {
    // Agent is always running in WASM mode once initialized
  }

  async stopAgent(_id: string): Promise<void> {
    // Cannot stop the agent in WASM mode (it is the runtime)
  }

  // -- Sessions (in-memory in WASM mode) --

  async listSessions(): Promise<SessionInfo[]> {
    return Array.from(this.sessions.entries()).map(([key, msgs]) => ({
      key,
      messageCount: msgs.length,
      createdAt: msgs[0]?.timestamp ?? new Date().toISOString(),
      updatedAt: msgs[msgs.length - 1]?.timestamp ?? new Date().toISOString(),
    }));
  }

  async getSessionMessages(key: string): Promise<ChatMessage[]> {
    return this.sessions.get(key) ?? [];
  }

  async deleteSession(key: string): Promise<void> {
    this.sessions.delete(key);
  }

  // -- Tools (from WASM registry, stubbed until WASM exposes tool list) --

  async listTools(): Promise<ToolInfo[]> {
    // In browser mode, tools are the browser-safe subset
    return [
      { name: "read_file", description: "Read file from OPFS workspace" },
      { name: "write_file", description: "Write file to OPFS workspace" },
      { name: "edit_file", description: "Edit file in OPFS workspace" },
      { name: "list_directory", description: "List OPFS directory contents" },
      { name: "web_search", description: "Search the web" },
      { name: "web_fetch", description: "Fetch URL content" },
      { name: "memory_read", description: "Read from memory store" },
      { name: "memory_write", description: "Write to memory store" },
    ];
  }

  async getToolSchema(_name: string): Promise<Record<string, unknown> | null> {
    return null; // Schema introspection deferred to future WASM entry point
  }

  // -- Memory (delegates to WASM via send_message with memory commands) --

  async listMemory(_namespace?: string): Promise<MemoryEntry[]> {
    // Memory operations go through the agent as tool calls in WASM mode
    return [];
  }

  async searchMemory(
    _query: string,
    _namespace?: string,
    _threshold?: number,
  ): Promise<MemoryEntry[]> {
    return [];
  }

  async writeMemory(
    _key: string,
    _value: string,
    _namespace?: string,
    _tags?: string[],
  ): Promise<void> {
    // Delegated to agent tool call
  }

  async deleteMemory(_key: string, _namespace?: string): Promise<void> {
    // Delegated to agent tool call
  }

  // -- Config --

  async getConfig(): Promise<Record<string, unknown>> {
    return this.config;
  }
}
```

#### 8.3.4 WASM Loader with Progress

**File**: `ui/src/lib/wasm-loader.ts`

```typescript
// ui/src/lib/wasm-loader.ts

export type LoadPhase = "download" | "compile" | "init" | "ready" | "error";

export interface LoadProgress {
  phase: LoadPhase;
  percent: number;
  message: string;
}

/**
 * Check if the browser supports all features needed for WASM mode.
 */
export async function checkWasmCapabilities(): Promise<{
  supported: boolean;
  missing: string[];
}> {
  const missing: string[] = [];

  if (typeof WebAssembly === "undefined") {
    missing.push("WebAssembly");
  }

  // Check OPFS support
  if (!("storage" in navigator) || !("getDirectory" in navigator.storage)) {
    missing.push("Origin Private File System (OPFS)");
  }

  // Check Web Crypto for API key encryption
  if (!("crypto" in globalThis) || !("subtle" in globalThis.crypto)) {
    missing.push("Web Crypto API");
  }

  return {
    supported: missing.length === 0,
    missing,
  };
}

/**
 * Load WASM binary with streaming compilation and progress tracking.
 */
export async function loadWasmModule(
  wasmJsUrl: string,
  onProgress: (progress: LoadProgress) => void,
): Promise<unknown> {
  try {
    onProgress({ phase: "download", percent: 0, message: "Downloading WASM module..." });

    // Dynamic import of the wasm-bindgen JS glue
    const module = await import(/* @vite-ignore */ wasmJsUrl);

    onProgress({ phase: "compile", percent: 40, message: "Compiling WASM binary..." });

    // wasm-bindgen default export initializes the WASM module
    await module.default();

    onProgress({ phase: "ready", percent: 100, message: "WASM module ready" });

    return module;
  } catch (error) {
    onProgress({
      phase: "error",
      percent: 0,
      message: `Failed to load WASM: ${error instanceof Error ? error.message : String(error)}`,
    });
    throw error;
  }
}
```

#### 8.3.5 Feature Detection

**File**: `ui/src/lib/feature-detect.ts`

```typescript
// ui/src/lib/feature-detect.ts

export interface FeatureReport {
  webAssembly: boolean;
  opfs: boolean;
  webCrypto: boolean;
  serviceWorker: boolean;
  indexedDb: boolean;
  fetchStreaming: boolean;
}

/**
 * Detect browser features required for each mode.
 *
 * Axum mode requires: fetch (always available in modern browsers)
 * WASM mode requires: WebAssembly, OPFS, Web Crypto, IndexedDB
 */
export async function detectFeatures(): Promise<FeatureReport> {
  return {
    webAssembly: typeof WebAssembly !== "undefined",
    opfs:
      typeof navigator !== "undefined" &&
      "storage" in navigator &&
      typeof (navigator.storage as { getDirectory?: unknown }).getDirectory === "function",
    webCrypto:
      typeof globalThis.crypto !== "undefined" &&
      typeof globalThis.crypto.subtle !== "undefined",
    serviceWorker: "serviceWorker" in navigator,
    indexedDb: typeof indexedDB !== "undefined",
    fetchStreaming:
      typeof ReadableStream !== "undefined" &&
      typeof Response !== "undefined" &&
      typeof Response.prototype.body !== "undefined",
  };
}

/**
 * Check whether WASM mode can run in this browser.
 */
export function canRunWasmMode(features: FeatureReport): {
  ok: boolean;
  warnings: string[];
  errors: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!features.webAssembly) {
    errors.push("WebAssembly is not supported in this browser.");
  }

  if (!features.webCrypto) {
    errors.push("Web Crypto API is required for secure API key storage.");
  }

  if (!features.indexedDb) {
    errors.push("IndexedDB is required for configuration storage.");
  }

  if (!features.opfs) {
    warnings.push(
      "Origin Private File System is not available. File operations will use in-memory storage (data lost on reload)."
    );
  }

  if (!features.fetchStreaming) {
    warnings.push(
      "Fetch streaming is not available. LLM responses will not stream incrementally."
    );
  }

  return {
    ok: errors.length === 0,
    warnings,
    errors,
  };
}
```

#### 8.3.6 Mode Context Provider

**File**: `ui/src/lib/mode-context.tsx`

React context that provides the active `BackendAdapter` and capabilities to all components.
Routes and UI elements check `capabilities` to show/hide features.

```tsx
// ui/src/lib/mode-context.tsx

import { createContext, useContext, useState, useEffect, useMemo } from "react";
import type { BackendAdapter, BackendCapabilities, BackendMode } from "./backend-adapter";
import { AxumAdapter } from "./adapters/axum-adapter";
import { WasmAdapter } from "./adapters/wasm-adapter";
import type { LoadProgress } from "./wasm-loader";

interface ModeContextValue {
  adapter: BackendAdapter;
  mode: BackendMode;
  capabilities: BackendCapabilities;
  isReady: boolean;
  loadProgress: LoadProgress | null;
}

const ModeContext = createContext<ModeContextValue | null>(null);

interface ModeProviderProps {
  children: React.ReactNode;
}

/**
 * Determines backend mode from environment variables and initializes
 * the appropriate adapter.
 *
 * - VITE_BACKEND_MODE=axum  -> AxumAdapter (default)
 * - VITE_BACKEND_MODE=wasm  -> WasmAdapter
 * - VITE_BACKEND_MODE=auto  -> Try Axum, fall back to WASM
 */
export function ModeProvider({ children }: ModeProviderProps) {
  const [adapter, setAdapter] = useState<BackendAdapter | null>(null);
  const [loadProgress, setLoadProgress] = useState<LoadProgress | null>(null);

  useEffect(() => {
    const mode = (import.meta.env.VITE_BACKEND_MODE as string) ?? "axum";
    const apiUrl = (import.meta.env.VITE_API_URL as string) ?? "http://localhost:18789";
    const wsUrl = apiUrl.replace(/^http/, "ws") + "/ws";

    async function initAdapter() {
      if (mode === "wasm") {
        const wasmAdapter = new WasmAdapter("/clawft_wasm.js", (phase, pct) =>
          setLoadProgress({ phase: phase as LoadProgress["phase"], percent: pct, message: phase })
        );
        // Config loaded from IndexedDB by the WASM config UI component
        await wasmAdapter.init();
        setAdapter(wasmAdapter);
      } else if (mode === "auto") {
        try {
          const response = await fetch(`${apiUrl}/api/auth/verify`, { method: "HEAD" });
          if (response.ok) {
            const axumAdapter = new AxumAdapter(apiUrl, wsUrl);
            await axumAdapter.init();
            setAdapter(axumAdapter);
          } else {
            throw new Error("Axum not reachable");
          }
        } catch {
          const wasmAdapter = new WasmAdapter("/clawft_wasm.js", (phase, pct) =>
            setLoadProgress({ phase: phase as LoadProgress["phase"], percent: pct, message: phase })
          );
          await wasmAdapter.init();
          setAdapter(wasmAdapter);
        }
      } else {
        // Default: Axum mode
        const token = localStorage.getItem("clawft-auth-token") ?? undefined;
        const axumAdapter = new AxumAdapter(apiUrl, wsUrl, token);
        await axumAdapter.init();
        setAdapter(axumAdapter);
      }
    }

    initAdapter().catch(console.error);
  }, []);

  const value = useMemo<ModeContextValue | null>(() => {
    if (!adapter) return null;
    return {
      adapter,
      mode: adapter.mode,
      capabilities: adapter.capabilities,
      isReady: adapter.capabilities.ready,
      loadProgress,
    };
  }, [adapter, loadProgress]);

  if (!value) {
    // Show loading state while adapter initializes
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center space-y-4">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto" />
          <p className="text-muted-foreground">
            {loadProgress?.message ?? "Connecting to backend..."}
          </p>
          {loadProgress && (
            <div className="w-64 bg-muted rounded-full h-2">
              <div
                className="bg-primary h-2 rounded-full transition-all duration-300"
                style={{ width: `${loadProgress.percent}%` }}
              />
            </div>
          )}
        </div>
      </div>
    );
  }

  return <ModeContext.Provider value={value}>{children}</ModeContext.Provider>;
}

/**
 * Hook to access the current backend adapter and capabilities.
 */
export function useBackend(): ModeContextValue {
  const ctx = useContext(ModeContext);
  if (!ctx) throw new Error("useBackend must be used within ModeProvider");
  return ctx;
}

/**
 * Hook to check if a specific capability is available.
 *
 * Usage: const hasCron = useCapability("cron");
 */
export function useCapability(cap: keyof BackendCapabilities): boolean {
  const { capabilities } = useBackend();
  return capabilities[cap];
}
```

#### 8.3.7 Browser Config UI

**File**: `ui/src/components/wasm/browser-config.tsx`

Config setup screen shown on first launch in WASM mode. Handles:
- API key input with Web Crypto encryption before IndexedDB storage
- Provider selection (Anthropic direct, OpenAI via proxy, local Ollama)
- CORS proxy URL configuration
- Model selection

```tsx
// ui/src/components/wasm/browser-config.tsx

import { useState, useCallback } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";

interface BrowserConfigProps {
  onConfigured: (config: Record<string, unknown>) => void;
}

const PROVIDERS = [
  { value: "anthropic", label: "Anthropic (direct browser access)", browserDirect: true },
  { value: "openai", label: "OpenAI (requires CORS proxy)", browserDirect: false },
  { value: "ollama", label: "Ollama (local, http://localhost:11434)", browserDirect: true },
  { value: "lmstudio", label: "LM Studio (local, http://localhost:1234)", browserDirect: true },
  { value: "custom", label: "Custom OpenAI-compatible", browserDirect: false },
];

const MODELS: Record<string, string[]> = {
  anthropic: ["claude-sonnet-4-5-20250929", "claude-opus-4-20250514", "claude-haiku-3-5-20241022"],
  openai: ["gpt-4o", "gpt-4o-mini", "o3-mini"],
  ollama: ["llama3.3", "mistral", "codellama"],
  lmstudio: ["loaded-model"],
  custom: [],
};

/**
 * Encrypt API key using Web Crypto AES-256-GCM.
 * Returns base64-encoded ciphertext + IV.
 */
async function encryptApiKey(apiKey: string): Promise<string> {
  const key = await crypto.subtle.generateKey(
    { name: "AES-GCM", length: 256 },
    false, // non-extractable
    ["encrypt", "decrypt"]
  );

  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encoded = new TextEncoder().encode(apiKey);
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },
    key,
    encoded
  );

  // Store the CryptoKey in IndexedDB (non-extractable, stays in browser)
  const db = await openConfigDb();
  const tx = db.transaction("crypto-keys", "readwrite");
  tx.objectStore("crypto-keys").put(key, "api-key-encryption");
  await new Promise((resolve) => { tx.oncomplete = resolve; });

  // Return IV + ciphertext as base64
  const combined = new Uint8Array(iv.length + new Uint8Array(ciphertext).length);
  combined.set(iv);
  combined.set(new Uint8Array(ciphertext), iv.length);
  return btoa(String.fromCharCode(...combined));
}

async function openConfigDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("clawft-config", 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains("config")) {
        db.createObjectStore("config");
      }
      if (!db.objectStoreNames.contains("crypto-keys")) {
        db.createObjectStore("crypto-keys");
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export function BrowserConfig({ onConfigured }: BrowserConfigProps) {
  const [provider, setProvider] = useState("anthropic");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [corsProxy, setCorsProxy] = useState("");
  const [customBaseUrl, setCustomBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);

  const selectedProvider = PROVIDERS.find((p) => p.value === provider);
  const needsProxy = !selectedProvider?.browserDirect;

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const encryptedKey = apiKey ? await encryptApiKey(apiKey) : undefined;

      const config: Record<string, unknown> = {
        defaults: {
          model: model || MODELS[provider]?.[0] || "claude-sonnet-4-5-20250929",
          max_tokens: 4096,
        },
        providers: {
          [provider]: {
            api_key: apiKey, // Plain key passed to WASM init (encrypted copy in IndexedDB)
            base_url:
              provider === "ollama"
                ? "http://localhost:11434/v1"
                : provider === "lmstudio"
                ? "http://localhost:1234/v1"
                : customBaseUrl || undefined,
            browser_direct: selectedProvider?.browserDirect ?? false,
            cors_proxy: needsProxy && corsProxy ? corsProxy : undefined,
          },
        },
        routing: { strategy: "static" },
      };

      // Store encrypted config in IndexedDB
      const db = await openConfigDb();
      const tx = db.transaction("config", "readwrite");
      tx.objectStore("config").put(
        { ...config, providers: { [provider]: { encrypted_key: encryptedKey } } },
        "current"
      );
      await new Promise((resolve) => { tx.oncomplete = resolve; });

      onConfigured(config);
    } finally {
      setSaving(false);
    }
  }, [provider, apiKey, model, corsProxy, customBaseUrl, selectedProvider, needsProxy, onConfigured]);

  return (
    <div className="max-w-lg mx-auto mt-12 space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Browser Mode Setup</CardTitle>
          <p className="text-sm text-muted-foreground">
            Configure your LLM provider. Your API key is encrypted and stored
            locally in your browser. It is sent directly from your browser to the
            provider -- no server involved.
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Provider</Label>
            <Select value={provider} onValueChange={setProvider}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PROVIDERS.map((p) => (
                  <SelectItem key={p.value} value={p.value}>
                    {p.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {provider !== "ollama" && provider !== "lmstudio" && (
            <div className="space-y-2">
              <Label>API Key</Label>
              <Input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-..."
              />
              <p className="text-xs text-muted-foreground">
                Your key is sent directly from your browser to {provider}. Use a
                key with restricted permissions for browser usage.
              </p>
            </div>
          )}

          <div className="space-y-2">
            <Label>Model</Label>
            <Select value={model} onValueChange={setModel}>
              <SelectTrigger>
                <SelectValue placeholder="Select model" />
              </SelectTrigger>
              <SelectContent>
                {(MODELS[provider] ?? []).map((m) => (
                  <SelectItem key={m} value={m}>
                    {m}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {needsProxy && (
            <div className="space-y-2">
              <Label>CORS Proxy URL</Label>
              <Input
                value={corsProxy}
                onChange={(e) => setCorsProxy(e.target.value)}
                placeholder="https://your-proxy.example.com/"
              />
              <p className="text-xs text-muted-foreground">
                {provider} does not support direct browser access. Route API
                calls through a CORS proxy that adds the required headers.
              </p>
            </div>
          )}

          {provider === "custom" && (
            <div className="space-y-2">
              <Label>Base URL</Label>
              <Input
                value={customBaseUrl}
                onChange={(e) => setCustomBaseUrl(e.target.value)}
                placeholder="https://api.example.com/v1"
              />
            </div>
          )}

          <Button onClick={handleSave} disabled={saving} className="w-full">
            {saving ? "Saving..." : "Save and Start"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
```

#### 8.3.8 Vite WASM Build Configuration

**File**: `ui/vite.config.ts` (additions)

```typescript
// Additions to existing ui/vite.config.ts for WASM support

export default defineConfig(({ mode }) => ({
  // ... existing config ...

  optimizeDeps: {
    // Exclude WASM glue from pre-bundling (loaded dynamically)
    exclude: ["clawft_wasm"],
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // ... existing chunks ...
          // WASM adapter is loaded only in WASM mode
          "vendor-wasm": mode === "wasm" ? [] : [],
        },
      },
    },
    // Enable top-level await for WASM initialization
    target: "es2022",
  },

  // Copy WASM binary to dist if building for WASM mode
  ...(mode === "wasm" && {
    publicDir: "public",
    // The WASM files (clawft_wasm.js + clawft_wasm_bg.wasm) are placed in public/
    // and served as static assets. They are NOT bundled by Vite.
  }),
}));
```

#### 8.3.9 Route Gating

Routes that are not available in WASM mode are hidden from the sidebar and redirect
to the dashboard if accessed directly. This is handled by the `ModeProvider` context.

```tsx
// ui/src/components/layout/navigation.tsx (modifications)
//
// Wrap route links with capability checks:

import { useCapability } from "@/lib/mode-context";

function NavigationLinks() {
  const hasCron = useCapability("cron");
  const hasChannels = useCapability("channels");
  const hasDelegation = useCapability("delegation");

  return (
    <nav>
      {/* Always available */}
      <NavLink to="/">Dashboard</NavLink>
      <NavLink to="/agents">Agents</NavLink>
      <NavLink to="/chat">Chat</NavLink>
      <NavLink to="/canvas">Canvas</NavLink>
      <NavLink to="/sessions">Sessions</NavLink>
      <NavLink to="/tools">Tools</NavLink>
      <NavLink to="/skills">Skills</NavLink>
      <NavLink to="/memory">Memory</NavLink>
      <NavLink to="/config">Config</NavLink>

      {/* Axum-only routes */}
      {hasCron && <NavLink to="/cron">Cron</NavLink>}
      {hasChannels && <NavLink to="/channels">Channels</NavLink>}
      {hasDelegation && <NavLink to="/delegation">Delegation</NavLink>}
    </nav>
  );
}
```

### 8.4 Tasks

| # | Task ID | Description | Priority | Location | Est. |
|---|---------|-------------|----------|----------|------|
| 33 | S3.6.1 | Define `BackendAdapter` interface with all method signatures | P1 | `ui/src/lib/backend-adapter.ts` | 4h |
| 34 | S3.6.2 | Implement `AxumAdapter` wrapping existing api-client + ws-client | P1 | `ui/src/lib/adapters/axum-adapter.ts` | 6h |
| 35 | S3.6.3 | Implement `WasmAdapter` with wasm-bindgen bridge | P1 | `ui/src/lib/adapters/wasm-adapter.ts` | 8h |
| 36 | S3.6.4 | WASM loader with progress tracking and streaming compilation | P1 | `ui/src/lib/wasm-loader.ts` | 4h |
| 37 | S3.6.5 | Feature detection and capability checking | P1 | `ui/src/lib/feature-detect.ts` | 3h |
| 38 | S3.6.6 | `ModeProvider` context with auto-detection and adapter initialization | P1 | `ui/src/lib/mode-context.tsx` | 6h |
| 39 | S3.6.7 | Browser config UI with API key encryption and provider setup | P1 | `ui/src/components/wasm/browser-config.tsx` | 8h |
| 40 | S3.6.8 | Vite WASM build config and public asset setup | P2 | `ui/vite.config.ts` | 3h |
| 41 | S3.6.9 | Route gating: hide unavailable routes in WASM mode | P1 | `ui/src/components/layout/navigation.tsx` | 3h |
| 42 | S3.6.10 | Migrate existing hooks to use `useBackend()` instead of direct api-client | P1 | `ui/src/hooks/*.ts` | 8h |
| 43 | S3.6.11 | Service worker: cache .wasm binary alongside static assets | P2 | `ui/src/sw.ts` | 2h |
| 44 | S3.6.12 | CSP update: add `'wasm-unsafe-eval'` for browser-only mode | P1 | `ui/vite.config.ts`, `crates/clawft-services/src/api/middleware/csp.rs` | 2h |

**Total S3.6: 12 tasks, estimated 57 hours** (~7 developer-days)

### 8.5 Tests

| Test | Description | Type |
|------|-------------|------|
| `test_backend_adapter_axum_list_agents` | AxumAdapter.listAgents() calls GET /api/agents and returns parsed result | Unit |
| `test_backend_adapter_wasm_list_agents` | WasmAdapter.listAgents() returns single browser agent | Unit |
| `test_wasm_adapter_send_message` | WasmAdapter.sendMessage() calls wasm.send_message() and returns ChatMessage | Unit |
| `test_wasm_adapter_session_persistence` | Messages accumulate in WasmAdapter.sessions map across calls | Unit |
| `test_feature_detect_modern_browser` | detectFeatures() returns all true in modern Chrome | Unit |
| `test_feature_detect_missing_opfs` | detectFeatures() returns opfs=false when navigator.storage.getDirectory missing | Unit |
| `test_can_run_wasm_mode_errors` | canRunWasmMode() returns errors when WebAssembly unavailable | Unit |
| `test_can_run_wasm_mode_warnings` | canRunWasmMode() returns warnings when OPFS unavailable | Unit |
| `test_mode_provider_axum_default` | ModeProvider selects AxumAdapter when VITE_BACKEND_MODE=axum | Component |
| `test_mode_provider_wasm_mode` | ModeProvider selects WasmAdapter when VITE_BACKEND_MODE=wasm | Component |
| `test_route_gating_hides_cron` | Navigation hides Cron link when capabilities.cron=false | Component |
| `test_route_gating_shows_cron` | Navigation shows Cron link when capabilities.cron=true | Component |
| `test_browser_config_saves_encrypted` | BrowserConfig saves encrypted API key to IndexedDB | Integration |
| `test_browser_config_provider_selection` | Provider dropdown shows CORS proxy input for OpenAI | Component |
| `test_wasm_loader_progress` | loadWasmModule() emits progress callbacks in order | Unit |
| `test_e2e_wasm_mode_chat` | Playwright: WASM mode loads, send message, receive response | E2E |

### 8.6 Exit Criteria

- [ ] `BackendAdapter` interface is implemented by both `AxumAdapter` and `WasmAdapter`
- [ ] Existing UI hooks use `useBackend()` instead of direct api-client calls
- [ ] WASM module loads in browser with visible progress indicator
- [ ] Browser config UI encrypts API key with Web Crypto before IndexedDB storage
- [ ] WebChat works end-to-end in WASM mode (user -> WASM pipeline -> LLM -> response)
- [ ] Routes unavailable in WASM mode are hidden from navigation
- [ ] Feature detection warns users about missing browser capabilities
- [ ] Auto-detection correctly falls back from Axum to WASM when server is unreachable
- [ ] Service worker caches .wasm binary for offline support
- [ ] All existing Axum-mode functionality works unchanged after adapter refactor

---

## 9. Phase S3.7: Documentation + Developer Guide (Week 10)

### 9.1 Overview

S3.7 produces four documentation deliverables that cover the UI architecture, API contracts,
browser-only mode, and deployment options. Documentation is placed in `docs/ui/` per project
convention.

### 9.2 Deliverables

#### 9.2.1 UI Developer Guide

**File**: `docs/ui/developer-guide.md`

Contents:
- Project structure: `ui/src/` directory layout (routes, components, hooks, stores, lib, mocks)
- Tech stack: Vite, React, TypeScript, Tailwind CSS v4, shadcn/ui, TanStack Router/Query, Zustand
- Getting started: `pnpm install`, `pnpm dev`, `VITE_MOCK_API=true pnpm dev`
- Adding a new route: file-based routing with TanStack Router
- Adding a new component: shadcn/ui component patterns, Tailwind conventions
- Adding a new API endpoint: Rust handler -> TypeScript types -> MSW mock -> React hook
- State management: Zustand store patterns, when to use vs TanStack Query
- MSW mock patterns: adding new mock handlers, fixture data
- Testing: Vitest for unit tests, Playwright for E2E, axe-core for accessibility
- Code style: ESLint + Prettier config, import conventions, naming conventions

#### 9.2.2 API Reference

**File**: `docs/ui/api-reference.md`

Contents:
- Auth: `POST /api/auth/token`, `GET /api/auth/verify`
- Agents: `GET/PATCH /api/agents`, `POST /api/agents/:id/start`, `POST /api/agents/:id/stop`
- Sessions: `GET/DELETE /api/sessions`
- Tools: `GET /api/tools`, `GET /api/tools/:name/schema`
- Canvas: `/ws/canvas` protocol, REST state/snapshot endpoints
- Skills: `GET/DELETE /api/skills`, `GET /api/skills/hub/search`, `POST /api/skills/hub/install`
- Memory: `GET/POST/DELETE /api/memory`, `POST /api/memory/search`
- Config: `GET/PATCH /api/config`, `POST /api/config/validate`
- Cron: `GET/POST/DELETE/PATCH /api/cron`
- Channels: `GET /api/channels`, `POST /api/channels/:name/restart`
- Delegation: `GET /api/delegation/active`, `GET/PATCH/DELETE /api/delegation/rules`
- Monitoring: `GET /api/monitoring/token-usage`, `GET /api/monitoring/costs`, `GET /api/monitoring/pipeline-runs`
- WebSocket: `/ws` topic subscription protocol, event types

Each endpoint includes: method, path, request schema, response schema, example curl command,
TypeScript type reference, and error codes.

#### 9.2.3 Browser Mode Guide

**File**: `docs/ui/browser-mode.md`

Contents:
- What is browser mode: WASM module runs clawft agent loop directly in the browser
- Requirements: modern browser (Chrome 102+, Firefox 111+, Safari 15.2+)
- Building WASM module: `wasm-pack build crates/clawft-wasm --target web --features browser`
- Deploying browser-only UI: copy `ui/dist/` + WASM files to static hosting
- Provider CORS setup per provider (Anthropic, OpenAI, Ollama, LM Studio)
- API key security considerations
- Feature limitations vs Axum mode
- Troubleshooting: CORS errors, WASM loading failures, OPFS issues

#### 9.2.4 Deployment Guide

**File**: `docs/ui/deployment.md`

Contents:
- Docker deployment: `ui/Dockerfile` (nginx:alpine serving `dist/`)
- CDN deployment: upload `dist/` to S3/Cloudflare/Vercel
- Reverse proxy: nginx/Caddy config proxying `/api` + `/ws` to Axum backend
- Single-binary: `weft ui` embedding dist/ via rust-embed
- Tauri packaging: `cargo tauri build` for macOS/Linux/Windows
- Browser-only: static hosting with WASM files, no backend required
- Environment variables: `VITE_API_URL`, `VITE_BACKEND_MODE`, `VITE_MOCK_API`

### 9.3 Tasks

| # | Task ID | Description | Priority | Location | Est. |
|---|---------|-------------|----------|----------|------|
| 45 | S3.7.1 | Write UI developer guide | P2 | `docs/ui/developer-guide.md` | 8h |
| 46 | S3.7.2 | Write API reference with all endpoints | P2 | `docs/ui/api-reference.md` | 6h |
| 47 | S3.7.3 | Write browser mode guide with CORS setup per provider | P2 | `docs/ui/browser-mode.md` | 4h |
| 48 | S3.7.4 | Write deployment guide covering all modes | P2 | `docs/ui/deployment.md` | 4h |

**Total S3.7: 4 tasks, estimated 22 hours** (~3 developer-days)

### 9.4 Exit Criteria

- [ ] UI developer guide covers project setup through contribution workflow
- [ ] API reference documents all REST and WS endpoints with examples
- [ ] Browser mode guide includes per-provider CORS setup instructions
- [ ] Deployment guide covers Docker, CDN, reverse proxy, Tauri, single-binary, and browser-only modes
- [ ] All code examples in docs are tested and match current implementation
