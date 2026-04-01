/**
 * KernelDataProvider — fetches real kernel data via Tauri invoke calls
 * and populates the StateStore. When not running inside Tauri, renders
 * a "Connect to kernel" placeholder instead of mock data.
 */

import { useEffect, useState, useCallback, useRef, type ReactNode } from 'react';
import { useStateStore } from './StateStore';

// ---------------------------------------------------------------------------
// Tauri detection
// ---------------------------------------------------------------------------

function isTauri(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

// ---------------------------------------------------------------------------
// Types for kernel responses
// ---------------------------------------------------------------------------

interface KernelStatusResponse {
  ok: boolean;
  data?: {
    version: string;
    uptime_secs: number;
    process_count: number;
    chain_height: number;
    health: string;
  };
}

interface KernelMetricsResponse {
  ok: boolean;
  data?: {
    cpu_percent: number;
    memory_used_mb: number;
    chain_height: number;
    process_count: number;
    agent_count: number;
    uptime_secs: number;
    mesh_peer_count: number;
  };
}

interface KernelProcessEntry {
  pid: number;
  agent_id: string;
  state: string;
}

interface KernelProcessesResponse {
  ok: boolean;
  data?: KernelProcessEntry[];
}

interface KernelCostResponse {
  ok: boolean;
  data?: {
    agents: Array<{
      agent_id: string;
      tokens_used: number;
      cost_usd: number;
      budget_remaining_usd: number;
    }>;
    total_cost_usd: number;
    total_budget_usd: number;
  };
}

// ---------------------------------------------------------------------------
// Provider component
// ---------------------------------------------------------------------------

interface KernelDataProviderProps {
  children: ReactNode;
  /** Poll interval in milliseconds (default: 2000) */
  pollInterval?: number;
  /** When true, allow mock data fallback even outside Tauri */
  allowMockFallback?: boolean;
}

type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error';

export function KernelDataProvider({
  children,
  pollInterval = 2000,
  allowMockFallback = false,
}: KernelDataProviderProps) {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const merge = useStateStore((s) => s.merge);
  const activeRef = useRef(false);

  const pollKernel = useCallback(async () => {
    if (!activeRef.current) return;
    if (!isTauri()) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');

      // Fetch status, metrics, processes, and cost data in parallel
      const [statusResp, metricsResp, processesResp, costResp] = await Promise.all([
        invoke<KernelStatusResponse>('kernel_status').catch(() => null),
        invoke<KernelMetricsResponse>('kernel_metrics').catch(() => null),
        invoke<KernelProcessesResponse>('kernel_processes').catch(() => null),
        invoke<KernelCostResponse>('kernel_cost_metrics').catch(() => null),
      ]);

      if (!activeRef.current) return;

      const entries: Record<string, unknown> = {};

      if (statusResp?.ok && statusResp.data) {
        entries['/kernel/version'] = statusResp.data.version;
        entries['/kernel/uptime_secs'] = statusResp.data.uptime_secs;
        entries['/kernel/process_count'] = statusResp.data.process_count;
        entries['/kernel/chain_height'] = statusResp.data.chain_height;
        entries['/kernel/health'] = statusResp.data.health;
      }

      if (metricsResp?.ok && metricsResp.data) {
        entries['/kernel/metrics/cpu_percent'] = metricsResp.data.cpu_percent;
        entries['/kernel/metrics/memory_used_mb'] = metricsResp.data.memory_used_mb;
        entries['/kernel/metrics/chain_height'] = metricsResp.data.chain_height;
        entries['/kernel/metrics/process_count'] = metricsResp.data.process_count;
        entries['/kernel/metrics/agent_count'] = metricsResp.data.agent_count;
        entries['/kernel/metrics/uptime_secs'] = metricsResp.data.uptime_secs;
        entries['/kernel/metrics/mesh_peer_count'] = metricsResp.data.mesh_peer_count;
      }

      if (processesResp?.ok && processesResp.data) {
        entries['/kernel/processes'] = processesResp.data.map((p) => ({
          pid: p.pid,
          agent_id: p.agent_id,
          state: p.state,
        }));
      }

      if (costResp?.ok && costResp.data) {
        entries['/kernel/metrics/cost'] = costResp.data;
      }

      if (Object.keys(entries).length > 0) {
        merge(entries);
        setConnectionState('connected');
        setErrorMessage(null);
      }
    } catch (err) {
      if (activeRef.current) {
        setConnectionState('error');
        setErrorMessage(err instanceof Error ? err.message : 'Failed to reach kernel');
      }
    }

    // Schedule next poll
    if (activeRef.current) {
      setTimeout(pollKernel, pollInterval);
    }
  }, [merge, pollInterval]);

  useEffect(() => {
    if (!isTauri() && !allowMockFallback) {
      setConnectionState('disconnected');
      return;
    }

    activeRef.current = true;
    setConnectionState('connecting');
    pollKernel();

    return () => {
      activeRef.current = false;
    };
  }, [pollKernel, allowMockFallback]);

  // When not in Tauri and mock fallback is disabled, show placeholder
  if (!isTauri() && !allowMockFallback) {
    return (
      <>
        <KernelDisconnectedBanner />
        {children}
      </>
    );
  }

  // Show connection error banner but still render children
  if (connectionState === 'error') {
    return (
      <>
        <KernelErrorBanner message={errorMessage} />
        {children}
      </>
    );
  }

  return <>{children}</>;
}

// ---------------------------------------------------------------------------
// Banners
// ---------------------------------------------------------------------------

function KernelDisconnectedBanner() {
  return (
    <div className="mx-auto max-w-5xl px-6 pt-4">
      <div className="flex items-center gap-3 rounded-lg border border-gray-700 bg-gray-800/80 px-4 py-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-gray-700">
          <svg
            className="h-4 w-4 text-gray-400"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={2}
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M13.19 8.688a4.5 4.5 0 011.242 7.244l-4.5 4.5a4.5 4.5 0 01-6.364-6.364l1.757-1.757m9.86-2.813a4.5 4.5 0 00-6.364-6.364L4.5 8.25"
            />
          </svg>
        </div>
        <div>
          <p className="text-sm font-medium text-gray-200">Connect to kernel</p>
          <p className="text-xs text-gray-400">
            Launch the WeftOS desktop app to stream live kernel data. Browser mode shows placeholder state.
          </p>
        </div>
      </div>
    </div>
  );
}

function KernelErrorBanner({ message }: { message: string | null }) {
  return (
    <div className="mx-auto max-w-5xl px-6 pt-4">
      <div className="flex items-center gap-3 rounded-lg border border-amber-700/50 bg-amber-900/20 px-4 py-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-full bg-amber-800/40">
          <svg
            className="h-4 w-4 text-amber-400"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={2}
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"
            />
          </svg>
        </div>
        <div>
          <p className="text-sm font-medium text-amber-200">Kernel connection error</p>
          <p className="text-xs text-amber-300/70">
            {message ?? 'Unable to reach the kernel. Retrying...'}
          </p>
        </div>
      </div>
    </div>
  );
}
