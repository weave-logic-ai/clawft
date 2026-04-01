/**
 * StateStore — Zustand store that holds kernel-projected block state.
 *
 * State is keyed by JSON Pointer paths (e.g. "/kernel/metrics/cpu_percent").
 * When running inside Tauri, state syncs via `invoke` commands.
 * Outside Tauri it falls back to local mock data.
 */

import { create } from 'zustand';
import { isStateRef, isFormatStateRef } from './types';
import type { StateRef, FormatStateRef, PropValue } from './types';

// ---------------------------------------------------------------------------
// Store shape
// ---------------------------------------------------------------------------

interface StateStoreState {
  /** Flat map of JSON Pointer path to value */
  data: Record<string, unknown>;
  /** Set a value at a given path */
  set: (path: string, value: unknown) => void;
  /** Merge a partial tree (e.g. from a kernel snapshot) */
  merge: (entries: Record<string, unknown>) => void;
  /** Get a value, returning undefined if absent */
  get: (path: string) => unknown;
}

export const useStateStore = create<StateStoreState>((set, get) => ({
  data: {},

  set: (path, value) =>
    set((state) => ({ data: { ...state.data, [path]: value } })),

  merge: (entries) =>
    set((state) => ({ data: { ...state.data, ...entries } })),

  get: (path) => get().data[path],
}));

// ---------------------------------------------------------------------------
// Resolve a $state reference against the store
// ---------------------------------------------------------------------------

export function resolveStateRef(ref: StateRef, data: Record<string, unknown>): unknown {
  const val = data[ref.$state];
  if (val === undefined) return ref.$default ?? undefined;
  if (ref.$transform) return applyTransform(ref.$transform, val);
  return val;
}

export function resolveFormatStateRef(ref: FormatStateRef, data: Record<string, unknown>): string {
  const val = data[ref.$state];
  const str = val !== undefined ? String(val) : '';
  return ref.format.replace('{v}', str);
}

function applyTransform(name: string, value: unknown): unknown {
  switch (name) {
    case 'percent':
      return typeof value === 'number' ? `${value}%` : value;
    case 'humanBytes': {
      if (typeof value !== 'number') return value;
      const units = ['B', 'KB', 'MB', 'GB', 'TB'];
      let v = value;
      let i = 0;
      while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
      return `${v.toFixed(1)} ${units[i]}`;
    }
    case 'truncate':
      return typeof value === 'string' && value.length > 80
        ? value.slice(0, 77) + '...'
        : value;
    default:
      return value;
  }
}

// ---------------------------------------------------------------------------
// Resolve all props in a block element, substituting $state refs
// ---------------------------------------------------------------------------

export function resolveProps(
  props: Record<string, PropValue> | undefined,
  data: Record<string, unknown>,
): Record<string, unknown> {
  if (!props) return {};
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(props)) {
    result[key] = resolveValue(val, data);
  }
  return result;
}

function resolveValue(val: PropValue, data: Record<string, unknown>): unknown {
  if (val === null || typeof val === 'string' || typeof val === 'number' || typeof val === 'boolean') {
    return val;
  }
  if (Array.isArray(val)) {
    return val.map((v) => resolveValue(v, data));
  }
  if (isStateRef(val)) return resolveStateRef(val, data);
  if (isFormatStateRef(val)) return resolveFormatStateRef(val, data);
  // Plain object — recurse
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(val)) {
    out[k] = resolveValue(v as PropValue, data);
  }
  return out;
}

// ---------------------------------------------------------------------------
// Tauri bridge — syncs kernel state into the store
// ---------------------------------------------------------------------------

let syncActive = false;

/**
 * Start syncing kernel state from Tauri commands into the StateStore.
 * Falls back to no-op outside Tauri (the mock WS hook populates state instead).
 */
export async function startTauriSync(): Promise<void> {
  if (syncActive) return;
  if (!window.__TAURI_INTERNALS__) return;

  syncActive = true;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const poll = async () => {
      if (!syncActive) return;
      try {
        const [statusResp, metricsResp, processesResp, costResp] = await Promise.all([
          invoke<{ ok: boolean; data?: {
            version: string;
            uptime_secs: number;
            process_count: number;
            chain_height: number;
            health: string;
          } }>('kernel_status').catch(() => null),
          invoke<{ ok: boolean; data?: {
            cpu_percent: number;
            memory_used_mb: number;
            chain_height: number;
            process_count: number;
            agent_count: number;
            uptime_secs: number;
            mesh_peer_count: number;
          } }>('kernel_metrics').catch(() => null),
          invoke<{ ok: boolean; data?: Array<{
            pid: number;
            agent_id: string;
            state: string;
          }> }>('kernel_processes').catch(() => null),
          invoke<{ ok: boolean; data?: unknown }>('kernel_cost_metrics').catch(() => null),
        ]);

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
          useStateStore.getState().merge(entries);
        }
      } catch {
        // Tauri commands may not be available yet
      }
      setTimeout(poll, 2000);
    };
    poll();
  } catch {
    syncActive = false;
  }
}

export function stopTauriSync(): void {
  syncActive = false;
}
