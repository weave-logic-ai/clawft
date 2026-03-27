import type { ProcessEntry } from '../types/kernel';

const STATE_COLORS: Record<ProcessEntry['state'], string> = {
  Running: 'text-emerald-400',
  Starting: 'text-amber-400',
  Suspended: 'text-orange-400',
  Exited: 'text-gray-500',
};

interface Props {
  processes: ProcessEntry[];
}

export function ProcessTable({ processes }: Props) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-700 text-gray-400 text-left">
            <th className="py-2 pr-4">PID</th>
            <th className="py-2 pr-4">Agent</th>
            <th className="py-2 pr-4">State</th>
            <th className="py-2 pr-4">Spawn</th>
            <th className="py-2 pr-4">Govern</th>
            <th className="py-2">Mesh</th>
          </tr>
        </thead>
        <tbody>
          {processes.map((p) => (
            <tr key={p.pid} className="border-b border-gray-800 hover:bg-gray-800/50">
              <td className="py-1.5 pr-4 font-mono">{p.pid}</td>
              <td className="py-1.5 pr-4 font-mono">{p.agent_id}</td>
              <td className={`py-1.5 pr-4 font-semibold ${STATE_COLORS[p.state]}`}>{p.state}</td>
              <td className="py-1.5 pr-4">{p.capabilities.can_spawn ? 'Y' : '-'}</td>
              <td className="py-1.5 pr-4">{p.capabilities.can_govern ? 'Y' : '-'}</td>
              <td className="py-1.5">{p.capabilities.can_mesh ? 'Y' : '-'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
