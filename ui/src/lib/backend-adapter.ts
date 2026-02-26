/**
 * BackendAdapter interface defining the contract for both Axum and WASM backends.
 *
 * All React hooks and stores communicate through this interface rather than
 * directly using api-client.ts or ws-client.ts. This enables the UI to run
 * in two modes:
 *   - "axum": connected to an Axum REST/WS backend (server mode)
 *   - "wasm": connected to a WASM module loaded in the browser (no server)
 *   - "mock": using MSW mock handlers (development)
 */

// ---------------------------------------------------------------------------
// Mode and capability types
// ---------------------------------------------------------------------------

export type BackendMode = "axum" | "wasm" | "mock";

export interface BackendCapabilities {
  /** Whether this backend supports channel management (Axum only). */
  channels: boolean;
  /** Whether this backend supports cron scheduling (Axum only). */
  cron: boolean;
  /** Whether this backend supports delegation monitoring (Axum only). */
  delegation: boolean;
  /** Whether this backend supports multi-user auth (Axum only). */
  multiUser: boolean;
  /** Whether this backend supports skill installation from registry (Axum only). */
  skillInstall: boolean;
  /** Whether this backend supports real-time WebSocket events. */
  realtime: boolean;
  /** Whether this backend supports monitoring dashboards (Axum only). */
  monitoring: boolean;
  /** Whether the backend is fully initialized and ready. */
  ready: boolean;
}

// ---------------------------------------------------------------------------
// Domain types used across adapters
// ---------------------------------------------------------------------------

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
  role: "user" | "assistant" | "system" | "tool";
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
  similarity?: number;
}

// ---------------------------------------------------------------------------
// BackendAdapter interface
// ---------------------------------------------------------------------------

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
  searchMemory(
    query: string,
    namespace?: string,
    threshold?: number,
  ): Promise<MemoryEntry[]>;
  writeMemory(
    key: string,
    value: string,
    namespace?: string,
    tags?: string[],
  ): Promise<void>;
  deleteMemory(key: string, namespace?: string): Promise<void>;

  // -- Config (read-only in WASM mode) --
  getConfig(): Promise<Record<string, unknown>>;
  updateConfig?(patch: Record<string, unknown>): Promise<void>;

  // -- Events (noop in WASM mode without WS) --
  subscribe?(topics: string[]): void;
  unsubscribe?(topics: string[]): void;
  onEvent?(
    callback: (event: { topic: string; data: unknown }) => void,
  ): () => void;
}
