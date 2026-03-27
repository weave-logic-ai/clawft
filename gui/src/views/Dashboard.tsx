import type { UseKernelWsReturn } from '../hooks/useKernelWs';
import { MetricsPanel } from '../components/MetricsPanel';
import { ProcessTable } from '../components/ProcessTable';

interface Props {
  ws: UseKernelWsReturn;
}

export function Dashboard({ ws }: Props) {
  const { connected, metrics, processes, events, services, health } = ws;

  return (
    <div className="space-y-6">
      {/* Connection indicator */}
      <div className="flex items-center gap-2 text-sm">
        <span className={`inline-block w-2 h-2 rounded-full ${connected ? 'bg-emerald-400' : 'bg-red-400'}`} />
        <span className="text-gray-400">{connected ? 'Connected to kernel' : 'Connecting...'}</span>
      </div>

      {/* Metrics */}
      {metrics ? (
        <MetricsPanel metrics={metrics} health={health} />
      ) : (
        <p className="text-gray-500 text-sm">Waiting for metrics...</p>
      )}

      {/* Process table */}
      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-2">Process Table</h2>
        {processes.length > 0 ? (
          <ProcessTable processes={processes} />
        ) : (
          <p className="text-gray-500 text-sm">No processes</p>
        )}
      </section>

      {/* Services */}
      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-2">Services</h2>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          {services.map((s) => (
            <div
              key={s.name}
              className="bg-gray-800/60 border border-gray-700 rounded px-3 py-2 text-sm"
            >
              <p className="font-mono text-gray-100">{s.name}</p>
              <p className="text-xs text-gray-400">
                {s.service_type} &middot;{' '}
                <span
                  className={
                    s.status === 'Running'
                      ? 'text-emerald-400'
                      : s.status === 'Stopped'
                        ? 'text-gray-500'
                        : 'text-red-400'
                  }
                >
                  {s.status}
                </span>
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Chain events */}
      <section>
        <h2 className="text-lg font-semibold text-gray-200 mb-2">Recent Chain Events</h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-700 text-gray-400 text-left">
                <th className="py-2 pr-4">Seq</th>
                <th className="py-2 pr-4">Kind</th>
                <th className="py-2 pr-4">Time</th>
                <th className="py-2">Hash</th>
              </tr>
            </thead>
            <tbody>
              {events.map((e) => (
                <tr key={e.seq} className="border-b border-gray-800 hover:bg-gray-800/50">
                  <td className="py-1 pr-4 font-mono">{e.seq}</td>
                  <td className="py-1 pr-4">{e.kind}</td>
                  <td className="py-1 pr-4 text-gray-400 text-xs">
                    {new Date(e.timestamp).toLocaleTimeString()}
                  </td>
                  <td className="py-1 font-mono text-xs text-gray-500">{e.hash}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
