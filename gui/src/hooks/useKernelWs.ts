import { useEffect, useRef, useState, useCallback } from 'react';
import type {
  KernelSnapshot,
  KernelMetrics,
  ProcessEntry,
  ChainEvent,
  ServiceEntry,
  HealthStatus,
  KernelCommand,
  KernelResponse,
} from '../types/kernel';

// ---------------------------------------------------------------------------
// Mock data generator — simulates what the real kernel WS would push
// ---------------------------------------------------------------------------

const AGENT_IDS = ['weaver-0', 'coder-1', 'reviewer-2', 'planner-3', 'mesh-4'];
const STATES: ProcessEntry['state'][] = ['Running', 'Running', 'Running', 'Starting', 'Suspended'];
const EVENT_KINDS = ['AgentSpawn', 'ConfigSet', 'ServiceRegister', 'GovernanceCheck', 'ChainAppend'];

let seqCounter = 100;

function randomHash(): string {
  return Array.from({ length: 12 }, () => Math.floor(Math.random() * 16).toString(16)).join('');
}

function mockSnapshot(): KernelSnapshot {
  const processes: ProcessEntry[] = AGENT_IDS.map((id, i) => ({
    pid: i + 1,
    agent_id: id,
    state: STATES[i],
    capabilities: {
      can_spawn: i < 2,
      can_govern: i === 0,
      can_mesh: true,
      max_children: i === 0 ? 8 : 4,
    },
  }));

  const events: ChainEvent[] = Array.from({ length: 10 }, (_, i) => {
    seqCounter += 1;
    return {
      seq: seqCounter,
      kind: EVENT_KINDS[i % EVENT_KINDS.length],
      timestamp: new Date(Date.now() - (10 - i) * 3000).toISOString(),
      hash: randomHash(),
    };
  });

  const metrics: KernelMetrics = {
    process_count: processes.length,
    agent_count: processes.filter((p) => p.state === 'Running').length,
    chain_height: seqCounter,
    uptime_secs: Math.floor(performance.now() / 1000),
    democritus_tick_count: Math.floor(Math.random() * 500) + 800,
    mesh_peer_count: 3 + Math.floor(Math.random() * 4),
    memory_used_mb: 64 + Math.floor(Math.random() * 32),
    cpu_percent: 5 + Math.floor(Math.random() * 20),
  };

  const services: ServiceEntry[] = [
    { name: 'registry', service_type: 'core', status: 'Running' },
    { name: 'governance', service_type: 'core', status: 'Running' },
    { name: 'mesh-sync', service_type: 'network', status: 'Running' },
    { name: 'ecc-index', service_type: 'cognitive', status: 'Running' },
  ];

  const health: HealthStatus = metrics.cpu_percent > 20 ? 'degraded' : 'healthy';

  return { metrics, processes, events, services, health };
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export interface UseKernelWsReturn {
  connected: boolean;
  metrics: KernelMetrics | null;
  processes: ProcessEntry[];
  events: ChainEvent[];
  services: ServiceEntry[];
  health: HealthStatus;
  sendCommand: (cmd: KernelCommand) => Promise<KernelResponse>;
}

/**
 * WebSocket hook for real-time kernel data.
 *
 * In this prototype the `url` parameter is accepted but unused — all data
 * is generated locally via mock snapshots so the GUI works without a running
 * kernel.  When the Rust kernel exposes a WS endpoint, swap the mock timer
 * for a real WebSocket connection.
 */
export function useKernelWs(_url: string): UseKernelWsReturn {
  const [connected, setConnected] = useState(false);
  const [metrics, setMetrics] = useState<KernelMetrics | null>(null);
  const [processes, setProcesses] = useState<ProcessEntry[]>([]);
  const [events, setEvents] = useState<ChainEvent[]>([]);
  const [services, setServices] = useState<ServiceEntry[]>([]);
  const [health, setHealth] = useState<HealthStatus>('healthy');
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    // Simulate connection establishment
    const connectTimeout = setTimeout(() => {
      setConnected(true);

      // Push a snapshot every 2 seconds
      intervalRef.current = setInterval(() => {
        const snap = mockSnapshot();
        setMetrics(snap.metrics);
        setProcesses(snap.processes);
        setEvents(snap.events);
        setServices(snap.services);
        setHealth(snap.health);
      }, 2000);

      // Fire first snapshot immediately
      const snap = mockSnapshot();
      setMetrics(snap.metrics);
      setProcesses(snap.processes);
      setEvents(snap.events);
      setServices(snap.services);
      setHealth(snap.health);
    }, 300);

    return () => {
      clearTimeout(connectTimeout);
      if (intervalRef.current) clearInterval(intervalRef.current);
      setConnected(false);
    };
  }, [_url]);

  /**
   * Send a command to the kernel and receive a mock response.
   * In the real implementation this would serialize to JSON and send over WS.
   */
  const sendCommand = useCallback(async (cmd: KernelCommand): Promise<KernelResponse> => {
    // Simulate network latency
    await new Promise((r) => setTimeout(r, 200 + Math.random() * 300));

    switch (cmd.kind) {
      case 'spawn_agent':
        return { ok: true, data: { pid: Math.floor(Math.random() * 900) + 100 } };
      case 'stop_agent':
        return { ok: true, data: { stopped: true } };
      case 'set_config':
        return { ok: true, data: { saved: true } };
      case 'query_chain':
        return {
          ok: true,
          data: Array.from({ length: Number(cmd.payload.limit) || 5 }, (_, i) => ({
            seq: Number(cmd.payload.from_seq || 0) + i,
            kind: EVENT_KINDS[i % EVENT_KINDS.length],
            timestamp: new Date().toISOString(),
            hash: randomHash(),
          })),
        };
      case 'register_service':
        return { ok: true, data: { registered: true } };
      default:
        return { ok: false, error: `Unknown command: ${cmd.kind}` };
    }
  }, []);

  return { connected, metrics, processes, events, services, health, sendCommand };
}
