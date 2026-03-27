/** Agent capability flags — mirrors Rust AgentCapabilities */
export interface AgentCapabilities {
  can_spawn: boolean;
  can_govern: boolean;
  can_mesh: boolean;
  max_children: number;
}

/** Process-table entry — mirrors Rust ProcessEntry */
export type ProcessState = 'Starting' | 'Running' | 'Suspended' | 'Exited';

export interface ProcessEntry {
  pid: number;
  agent_id: string;
  state: ProcessState;
  capabilities: AgentCapabilities;
}

/** Kernel-level metrics — mirrors Rust KernelMetrics */
export interface KernelMetrics {
  process_count: number;
  agent_count: number;
  chain_height: number;
  uptime_secs: number;
  democritus_tick_count: number;
  mesh_peer_count: number;
  memory_used_mb: number;
  cpu_percent: number;
}

/** Service registry entry */
export type ServiceStatus = 'Running' | 'Stopped' | 'Error';

export interface ServiceEntry {
  name: string;
  service_type: string;
  status: ServiceStatus;
}

/** Governance EffectVector */
export interface EffectVector {
  risk: number;
  fairness: number;
  privacy: number;
  novelty: number;
  security: number;
}

/** Governance decision record */
export interface GovernanceDecision {
  rule_id: string;
  action: string;
  allowed: boolean;
  effect_vector: EffectVector;
}

/** ExoChain event */
export interface ChainEvent {
  seq: number;
  kind: string;
  timestamp: string;
  hash: string;
}

/** Config entry for admin forms */
export interface ConfigEntry {
  key: string;
  namespace: string;
  value: string | number | boolean;
}

/** Node health status */
export type HealthStatus = 'healthy' | 'degraded' | 'critical';

/** Aggregate snapshot pushed over WebSocket */
export interface KernelSnapshot {
  metrics: KernelMetrics;
  processes: ProcessEntry[];
  events: ChainEvent[];
  services: ServiceEntry[];
  health: HealthStatus;
}

/** Causal graph node */
export interface CausalNode {
  id: number;
  label: string;
  metadata: Record<string, unknown>;
  community?: number;
}

/** Causal graph edge */
export type CausalEdgeType =
  | 'Causes'
  | 'Enables'
  | 'Correlates'
  | 'Follows'
  | 'Inhibits'
  | 'Contradicts'
  | 'TriggeredBy'
  | 'EvidenceFor';

export interface CausalEdge {
  source: number;
  target: number;
  edge_type: CausalEdgeType;
  weight: number;
}

/** Full causal graph payload */
export interface GraphData {
  nodes: CausalNode[];
  edges: CausalEdge[];
  lambda_2: number;
  communities: number[][];
}

/** Command envelope sent TO the kernel */
export interface KernelCommand {
  kind: string;
  payload: Record<string, unknown>;
}

/** Response envelope received FROM the kernel */
export interface KernelResponse {
  ok: boolean;
  data?: unknown;
  error?: string;
}
