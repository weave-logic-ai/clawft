export type AgentStatus = "running" | "stopped" | "error";

export interface AgentSummary {
  id: string;
  name: string;
  status: AgentStatus;
  model?: string;
  created_at: string;
}

export interface AgentDetail extends AgentSummary {
  config: Record<string, unknown>;
  session_count: number;
  description?: string;
  skills?: string[];
}

export interface SessionSummary {
  key: string;
  agent_id: string;
  message_count: number;
  updated_at: string;
}

export interface SessionDetail extends SessionSummary {
  messages: ChatMessage[];
}

export interface ChatMessage {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  timestamp: string;
  tool_calls?: ToolCallInfo[];
}

export interface ToolCallInfo {
  id: string;
  name: string;
  arguments: string;
  result?: string;
}

export interface ToolInfo {
  name: string;
  description: string;
  schema?: Record<string, unknown>;
}

export interface SystemHealth {
  status: "healthy" | "degraded" | "down";
  uptime_seconds: number;
  memory_usage_mb: number;
  agent_count: number;
  session_count: number;
  tool_count: number;
}

// ── Canvas types ────────────────────────────────────────────────

export interface CanvasElementData {
  type: string;
  [key: string]: unknown;
}

export interface CanvasCommandData {
  command: string;
  id?: string;
  element?: CanvasElementData;
  position?: number;
  commands?: CanvasCommandData[];
}

export interface CanvasInteractionData {
  interaction: string;
  element_id: string;
  [key: string]: unknown;
}

// ── Advanced Canvas types (S3.2) ────────────────────────────────

export interface ChartDataPoint {
  label: string;
  value: number;
}

export type ChartType = "bar" | "line" | "pie";

export interface ChartCanvasElement extends CanvasElementData {
  type: "chart";
  data: ChartDataPoint[];
  chartType: ChartType;
  title?: string;
  colors?: string[];
}

export interface CodeCanvasElement extends CanvasElementData {
  type: "code_editor";
  code: string;
  language: string;
  editable?: boolean;
  lineNumbers?: boolean;
}

export type FormFieldType = "text" | "number" | "select" | "checkbox" | "textarea";

export interface FormFieldSchema {
  name: string;
  type: FormFieldType;
  label: string;
  required?: boolean;
  options?: string[];
  min?: number;
  max?: number;
  placeholder?: string;
}

export interface FormCanvasElement extends CanvasElementData {
  type: "form_advanced";
  fields: FormFieldSchema[];
  submitAction?: string;
}

// ── Skill types ─────────────────────────────────────────────────

export interface SkillData {
  name: string;
  version: string;
  description: string;
  author?: string;
  tags: string[];
  installed: boolean;
  enabled: boolean;
}

export interface RegistrySkill {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  stars: number;
  tags: string[];
  signed: boolean;
}

// ── Memory types ────────────────────────────────────────────────

export interface MemoryEntry {
  key: string;
  value: string;
  namespace: string;
  tags: string[];
  updated_at: string;
  similarity?: number;
}

// ── Config types ────────────────────────────────────────────────

export interface ConfigData {
  agents: {
    defaults: {
      model: string;
      max_tokens: number;
      temperature: number;
    };
  };
  providers: Record<
    string,
    {
      api_key_set: boolean;
      api_base: string;
      enabled: boolean;
    }
  >;
  channels: Record<string, { enabled: boolean }>;
  gateway: {
    api_port: number;
    api_enabled: boolean;
  };
}

// ── Cron types ──────────────────────────────────────────────────

export interface CronJob {
  id: string;
  name: string;
  schedule: string;
  enabled: boolean;
  status: "idle" | "running" | "failed";
  last_run?: string;
  next_run?: string;
  payload?: string;
}

// ── Channel types ───────────────────────────────────────────────

export interface ChannelStatus {
  name: string;
  type: "discord" | "telegram" | "slack" | "web" | "voice";
  status: "connected" | "disconnected" | "error";
  message_count: number;
  last_activity?: string;
  routes_to?: string;
}

// ── Delegation types ───────────────────────────────────────────

export interface ActiveDelegation {
  task_id: string;
  session_key: string;
  target: string;
  status: "pending" | "running" | "completed" | "failed";
  started_at: string;
  latency_ms?: number;
  tool_name: string;
  complexity: number;
}

export interface DelegationRule {
  name: string;
  pattern: string;
  target: string;
  complexity_threshold: number;
  enabled: boolean;
  priority: number;
}

export interface DelegationHistoryEntry {
  task_id: string;
  session_key: string;
  target: string;
  tool_name: string;
  status: "pending" | "running" | "completed" | "failed";
  started_at: string;
  completed_at?: string;
  latency_ms?: number;
  complexity: number;
}

export interface PaginatedHistory {
  items: DelegationHistoryEntry[];
  total: number;
  limit: number;
  offset: number;
}

// ── Monitoring types ───────────────────────────────────────────

export interface TokenUsage {
  provider: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  request_count: number;
}

export interface SessionTokenUsage {
  session_key: string;
  input_tokens: number;
  output_tokens: number;
  request_count: number;
}

export interface TokenUsageSummary {
  total_input: number;
  total_output: number;
  total_requests: number;
  by_provider: TokenUsage[];
  by_session: SessionTokenUsage[];
}

export interface ProviderCost {
  provider: string;
  model: string;
  input_cost_usd: number;
  output_cost_usd: number;
  total_cost_usd: number;
}

export interface TierCost {
  tier: number;
  label: string;
  request_count: number;
  total_cost_usd: number;
}

export interface CostBreakdown {
  total_cost_usd: number;
  by_provider: ProviderCost[];
  by_tier: TierCost[];
}

export interface PipelineRun {
  id: string;
  session_key: string;
  model: string;
  complexity: number;
  latency_ms: number;
  status: "success" | "error";
  timestamp: string;
}

// ── Voice types ──────────────────────────────────────────────────

export interface VoiceStatusData {
  state: "idle" | "listening" | "processing" | "speaking";
  talkModeActive: boolean;
  wakeWordEnabled: boolean;
}

export interface VoiceSettingsData {
  enabled: boolean;
  wakeWordEnabled: boolean;
  language: string;
  echoCancel: boolean;
  noiseSuppression: boolean;
  pushToTalk: boolean;
}
