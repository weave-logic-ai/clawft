import { useState, useEffect } from 'react';
import { useKernelWs } from './hooks/useKernelWs';
import { Dashboard } from './views/Dashboard';
import { AdminForms } from './views/AdminForms';
import { KnowledgeGraph } from './views/KnowledgeGraph';
import { ComponentGenerator } from './views/ComponentGenerator';
import { BlockRenderer, useStateStore, startTauriSync, KernelDataProvider } from './engine';
import { ThemeSwitcher } from './components/ThemeSwitcher';
import type { BlockDescriptor } from './engine';

// Register all built-in block types
import './blocks';

type View = 'dashboard' | 'admin' | 'graph' | 'generate' | 'blocks';

// ---------------------------------------------------------------------------
// Detect Tauri environment
// ---------------------------------------------------------------------------

function isTauri(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

// ---------------------------------------------------------------------------
// Demo block descriptor — shows the block engine rendering a dashboard
// ---------------------------------------------------------------------------

const DEMO_DESCRIPTOR: BlockDescriptor = {
  version: '0.2.0',
  root: 'main',
  elements: {
    'main': {
      type: 'Tabs',
      props: {
        labels: ['Overview', 'Console', 'Code', 'Tree', 'Budget'],
        activeTab: 0,
      },
      children: ['overview', 'console', 'code-tab', 'tree-tab', 'budget-tab'],
    },
    'overview': {
      type: 'Column',
      props: { gap: 12 },
      children: ['title', 'metrics-row', 'proc-table'],
    },
    'title': {
      type: 'Markdown',
      props: {
        content: '## Kernel Dashboard\n\nReal-time overview powered by the **Lego Block Engine**.',
      },
    },
    'metrics-row': {
      type: 'Row',
      props: { gap: 12, wrap: true },
      children: ['cpu-metric', 'mem-metric', 'chain-metric', 'procs-metric'],
    },
    'cpu-metric': {
      type: 'Metric',
      props: { label: 'CPU', value: { $state: '/kernel/metrics/cpu_percent', $default: 0 }, unit: '%', threshold: { warn: 70, crit: 90 } },
    },
    'mem-metric': {
      type: 'Metric',
      props: { label: 'Memory', value: { $state: '/kernel/metrics/memory_used_mb', $default: 0 }, unit: 'MB' },
    },
    'chain-metric': {
      type: 'Metric',
      props: { label: 'Chain Height', value: { $state: '/kernel/metrics/chain_height', $default: 0 } },
    },
    'procs-metric': {
      type: 'Metric',
      props: { label: 'Processes', value: { $state: '/kernel/metrics/process_count', $default: 0 } },
    },
    'proc-table': {
      type: 'DataTable',
      props: {
        columns: [
          { key: 'pid', label: 'PID' },
          { key: 'agent_id', label: 'Agent' },
          { key: 'state', label: 'State' },
        ],
        rows: { $state: '/kernel/processes', $default: [] },
        sortable: true,
      },
    },
    'console': {
      type: 'ConsolePan',
      props: {},
    },
    'code-tab': {
      type: 'Column',
      props: { gap: 12 },
      children: ['code-example', 'spawn-btn'],
    },
    'code-example': {
      type: 'CodeEditor',
      props: {
        language: 'rust',
        readOnly: true,
        value: 'fn main() {\n    let kernel = WeftOSKernel::boot(config)?;\n    kernel.run_event_loop();\n}\n',
      },
    },
    'spawn-btn': {
      type: 'Button',
      props: { label: 'Spawn Coder Agent', variant: 'primary' },
      on: {
        press: {
          action: 'kernel_exec',
          params: { command: 'process.spawn coder' },
        },
      },
    },
    'tree-tab': {
      type: 'ResourceTree',
      props: { rootPath: '/project' },
    },
    'budget-tab': {
      type: 'Budget',
      props: {
        title: 'Agent Budget & Cost Tracking',
        stateKey: '/kernel/metrics/cost',
      },
    },
  },
};

function App() {
  const [view, setView] = useState<View>('blocks');
  const ws = useKernelWs('ws://localhost:9800/ws');
  const merge = useStateStore((s) => s.merge);
  const inTauri = isTauri();

  // When in Tauri, use KernelDataProvider for real data (handled below).
  // When NOT in Tauri, pump mock WebSocket data into the StateStore as fallback.
  useEffect(() => {
    if (inTauri) return; // Real data comes from KernelDataProvider
    if (!ws.metrics) return;
    const entries: Record<string, unknown> = {
      '/kernel/metrics/cpu_percent': ws.metrics.cpu_percent,
      '/kernel/metrics/memory_used_mb': ws.metrics.memory_used_mb,
      '/kernel/metrics/chain_height': ws.metrics.chain_height,
      '/kernel/metrics/process_count': ws.metrics.process_count,
      '/kernel/metrics/agent_count': ws.metrics.agent_count,
      '/kernel/metrics/uptime_secs': ws.metrics.uptime_secs,
      '/kernel/metrics/mesh_peer_count': ws.metrics.mesh_peer_count,
      '/kernel/processes': ws.processes.map((p) => ({
        pid: p.pid,
        agent_id: p.agent_id,
        state: p.state,
      })),
      '/kernel/health': ws.health,
    };
    merge(entries);
  }, [ws.metrics, ws.processes, ws.health, merge, inTauri]);

  // Start Tauri sync if available (legacy path — KernelDataProvider is preferred)
  useEffect(() => {
    startTauriSync();
  }, []);

  return (
    <KernelDataProvider>
      <div className="min-h-screen flex flex-col">
        {/* Header */}
        <header className="border-b border-gray-800 px-6 py-3 flex items-center justify-between">
          <h1 className="text-lg font-semibold tracking-tight text-gray-100">
            WeftOS <span className="text-gray-500 font-normal">K8 GUI</span>
          </h1>
          <div className="flex items-center gap-4">
            <ThemeSwitcher />
            <nav className="flex gap-1">
              {(['blocks', 'dashboard', 'admin', 'graph', 'generate'] as const).map((v) => (
                <button
                  key={v}
                  onClick={() => setView(v)}
                  className={`px-3 py-1 text-sm rounded ${
                    view === v
                      ? v === 'blocks' ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-white'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
                  }`}
                >
                  {v === 'blocks' ? 'Block Engine' : v === 'graph' ? 'Knowledge Graph' : v.charAt(0).toUpperCase() + v.slice(1)}
                </button>
              ))}
            </nav>
          </div>
        </header>

        {/* Content */}
        <main className={`flex-1 px-6 py-6 ${view === 'graph' ? 'max-w-7xl' : 'max-w-5xl'} w-full mx-auto`}>
          {view === 'blocks' && <BlockRenderer descriptor={DEMO_DESCRIPTOR} />}
          {view === 'dashboard' && <Dashboard ws={ws} />}
          {view === 'admin' && <AdminForms ws={ws} />}
          {view === 'graph' && <KnowledgeGraph />}
          {view === 'generate' && <ComponentGenerator />}
        </main>

        {/* Footer */}
        <footer className="border-t border-gray-800 px-6 py-2 text-xs text-gray-500 text-center">
          WeftOS K8 GUI v0.1.0 &middot; {inTauri ? 'Tauri Desktop' : 'Browser Mode'}
        </footer>
      </div>
    </KernelDataProvider>
  );
}

export default App;
