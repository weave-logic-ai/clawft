import type {
  AgentSummary,
  AgentDetail,
  SessionSummary,
  SessionDetail,
  ToolInfo,
  SystemHealth,
  ChatMessage,
  SkillData,
  RegistrySkill,
  MemoryEntry,
  ConfigData,
  CronJob,
  ChannelStatus,
  ActiveDelegation,
  DelegationRule,
  PaginatedHistory,
  TokenUsageSummary,
  CostBreakdown,
  PipelineRun,
  VoiceStatusData,
  VoiceSettingsData,
} from "./types";

const API_URL = import.meta.env.VITE_API_URL || "";

let authToken: string | null = null;

export function setAuthToken(token: string) {
  authToken = token;
  localStorage.setItem("clawft-token", token);
}

export function getAuthToken(): string | null {
  if (!authToken) {
    authToken = localStorage.getItem("clawft-token");
  }
  return authToken;
}

async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const token = getAuthToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...((options?.headers as Record<string, string>) || {}),
  };

  const res = await fetch(`${API_URL}${path}`, {
    ...options,
    headers,
  });

  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`);
  }

  return res.json();
}

export const api = {
  agents: {
    list: () => apiFetch<AgentSummary[]>("/api/agents"),
    get: (id: string) => apiFetch<AgentDetail>(`/api/agents/${id}`),
    start: (id: string) =>
      apiFetch<{ ok: boolean }>(`/api/agents/${id}/start`, {
        method: "POST",
      }),
    stop: (id: string) =>
      apiFetch<{ ok: boolean }>(`/api/agents/${id}/stop`, {
        method: "POST",
      }),
  },
  sessions: {
    list: () => apiFetch<SessionSummary[]>("/api/sessions"),
    get: (key: string) => apiFetch<SessionDetail>(`/api/sessions/${key}`),
    create: (agentId: string) =>
      apiFetch<SessionSummary>("/api/sessions", {
        method: "POST",
        body: JSON.stringify({ agent_id: agentId }),
      }),
    export: (key: string) =>
      apiFetch<{ messages: ChatMessage[] }>(
        `/api/sessions/${key}/export`,
      ),
    delete: (key: string) =>
      apiFetch<{ ok: boolean }>(`/api/sessions/${key}`, {
        method: "DELETE",
      }),
  },
  chat: {
    send: (sessionKey: string, content: string) =>
      apiFetch<ChatMessage>(`/api/sessions/${sessionKey}/messages`, {
        method: "POST",
        body: JSON.stringify({ content }),
      }),
  },
  tools: {
    list: () => apiFetch<ToolInfo[]>("/api/tools"),
    schema: (name: string) =>
      apiFetch<Record<string, unknown>>(`/api/tools/${name}/schema`),
  },
  system: {
    health: () => apiFetch<SystemHealth>("/api/health"),
  },
  auth: {
    createToken: () =>
      apiFetch<{ token: string }>("/api/auth/token", { method: "POST" }),
  },
  skills: {
    list: () => apiFetch<SkillData[]>("/api/skills"),
    install: (id: string) =>
      apiFetch<{ success: boolean }>("/api/skills/install", {
        method: "POST",
        body: JSON.stringify({ id }),
      }),
    uninstall: (name: string) =>
      apiFetch<{ success: boolean }>(`/api/skills/${name}`, {
        method: "DELETE",
      }),
    searchRegistry: (q: string) =>
      apiFetch<RegistrySkill[]>(
        `/api/skills/registry/search?q=${encodeURIComponent(q)}`,
      ),
  },
  memory: {
    list: () => apiFetch<MemoryEntry[]>("/api/memory"),
    search: (q: string, threshold: number) =>
      apiFetch<MemoryEntry[]>(
        `/api/memory/search?q=${encodeURIComponent(q)}&threshold=${threshold}`,
      ),
    create: (entry: { key: string; value: string; namespace: string; tags: string[] }) =>
      apiFetch<MemoryEntry>("/api/memory", {
        method: "POST",
        body: JSON.stringify(entry),
      }),
    delete: (key: string) =>
      apiFetch<{ success: boolean }>(`/api/memory/${encodeURIComponent(key)}`, {
        method: "DELETE",
      }),
  },
  config: {
    get: () => apiFetch<ConfigData>("/api/config"),
    save: (config: ConfigData) =>
      apiFetch<{ success: boolean }>("/api/config", {
        method: "PUT",
        body: JSON.stringify(config),
      }),
  },
  cron: {
    list: () => apiFetch<CronJob[]>("/api/cron"),
    create: (job: Omit<CronJob, "id" | "status" | "last_run" | "next_run">) =>
      apiFetch<CronJob>("/api/cron", {
        method: "POST",
        body: JSON.stringify(job),
      }),
    update: (id: string, job: Partial<CronJob>) =>
      apiFetch<CronJob>(`/api/cron/${id}`, {
        method: "PUT",
        body: JSON.stringify(job),
      }),
    delete: (id: string) =>
      apiFetch<{ success: boolean }>(`/api/cron/${id}`, {
        method: "DELETE",
      }),
    runNow: (id: string) =>
      apiFetch<{ success: boolean }>(`/api/cron/${id}/run`, {
        method: "POST",
      }),
  },
  channels: {
    list: () => apiFetch<ChannelStatus[]>("/api/channels"),
  },
  delegation: {
    listActive: () =>
      apiFetch<ActiveDelegation[]>("/api/delegation/active"),
    listRules: () =>
      apiFetch<DelegationRule[]>("/api/delegation/rules"),
    upsertRule: (rule: DelegationRule) =>
      apiFetch<DelegationRule>("/api/delegation/rules", {
        method: "PATCH",
        body: JSON.stringify(rule),
      }),
    deleteRule: (name: string) =>
      apiFetch<{ success: boolean }>(
        `/api/delegation/rules/${encodeURIComponent(name)}`,
        { method: "DELETE" },
      ),
    history: (params: { session?: string; target?: string; offset: number; limit: number }) => {
      const qs = new URLSearchParams();
      if (params.session) qs.set("session", params.session);
      if (params.target) qs.set("target", params.target);
      qs.set("offset", String(params.offset));
      qs.set("limit", String(params.limit));
      return apiFetch<PaginatedHistory>(`/api/delegation/history?${qs.toString()}`);
    },
  },
  monitoring: {
    tokenUsage: () =>
      apiFetch<TokenUsageSummary>("/api/monitoring/token-usage"),
    costs: () =>
      apiFetch<CostBreakdown>("/api/monitoring/costs"),
    pipelineRuns: () =>
      apiFetch<PipelineRun[]>("/api/monitoring/pipeline-runs"),
  },
  voice: {
    status: () =>
      apiFetch<VoiceStatusData & { settings: VoiceSettingsData }>(
        "/api/voice/status",
      ),
    updateSettings: (settings: Partial<VoiceSettingsData>) =>
      apiFetch<{ success: boolean }>("/api/voice/settings", {
        method: "PUT",
        body: JSON.stringify(settings),
      }),
    testMic: () =>
      apiFetch<{ success: boolean; level: number }>("/api/voice/test-mic", {
        method: "POST",
      }),
    testSpeaker: () =>
      apiFetch<{ success: boolean }>("/api/voice/test-speaker", {
        method: "POST",
      }),
  },
};
