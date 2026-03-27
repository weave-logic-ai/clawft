import type { KernelMetrics, HealthStatus } from '../types/kernel';

const HEALTH_STYLE: Record<HealthStatus, string> = {
  healthy: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40',
  degraded: 'bg-amber-500/20 text-amber-400 border-amber-500/40',
  critical: 'bg-red-500/20 text-red-400 border-red-500/40',
};

interface Props {
  metrics: KernelMetrics;
  health: HealthStatus;
}

function Stat({ label, value, unit }: { label: string; value: string | number; unit?: string }) {
  return (
    <div className="bg-gray-800/60 rounded-lg p-3 border border-gray-700">
      <p className="text-xs text-gray-400 mb-1">{label}</p>
      <p className="text-xl font-mono font-semibold text-gray-100">
        {value}
        {unit && <span className="text-sm text-gray-400 ml-1">{unit}</span>}
      </p>
    </div>
  );
}

export function MetricsPanel({ metrics, health }: Props) {
  return (
    <div className="space-y-3">
      <div className={`inline-block px-3 py-1 rounded-full text-xs font-semibold border ${HEALTH_STYLE[health]}`}>
        {health.toUpperCase()}
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Stat label="Processes" value={metrics.process_count} />
        <Stat label="Active Agents" value={metrics.agent_count} />
        <Stat label="Chain Height" value={metrics.chain_height} />
        <Stat label="Uptime" value={metrics.uptime_secs} unit="s" />
        <Stat label="DEMOCRITUS Ticks" value={metrics.democritus_tick_count} />
        <Stat label="Mesh Peers" value={metrics.mesh_peer_count} />
        <Stat label="Memory" value={metrics.memory_used_mb} unit="MB" />
        <Stat label="CPU" value={`${metrics.cpu_percent}%`} />
      </div>
    </div>
  );
}
