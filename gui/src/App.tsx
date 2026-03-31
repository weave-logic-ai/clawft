import { useState } from 'react';
import { useKernelWs } from './hooks/useKernelWs';
import { Dashboard } from './views/Dashboard';
import { AdminForms } from './views/AdminForms';
import { KnowledgeGraph } from './views/KnowledgeGraph';
import { ComponentGenerator } from './views/ComponentGenerator';

type View = 'dashboard' | 'admin' | 'graph' | 'generate';

function App() {
  const [view, setView] = useState<View>('dashboard');
  const ws = useKernelWs('ws://localhost:9800/ws');

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-3 flex items-center justify-between">
        <h1 className="text-lg font-semibold tracking-tight text-gray-100">
          WeftOS <span className="text-gray-500 font-normal">K8 GUI</span>
        </h1>
        <nav className="flex gap-1">
          <button
            onClick={() => setView('dashboard')}
            className={`px-3 py-1 text-sm rounded ${
              view === 'dashboard'
                ? 'bg-gray-700 text-white'
                : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
            }`}
          >
            Dashboard
          </button>
          <button
            onClick={() => setView('admin')}
            className={`px-3 py-1 text-sm rounded ${
              view === 'admin'
                ? 'bg-gray-700 text-white'
                : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
            }`}
          >
            Admin
          </button>
          <button
            onClick={() => setView('graph')}
            className={`px-3 py-1 text-sm rounded ${
              view === 'graph'
                ? 'bg-gray-700 text-white'
                : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
            }`}
          >
            Knowledge Graph
          </button>
          <button
            onClick={() => setView('generate')}
            className={`px-3 py-1 text-sm rounded ${
              view === 'generate'
                ? 'bg-indigo-600 text-white'
                : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800'
            }`}
          >
            Generate
          </button>
        </nav>
      </header>

      {/* Content */}
      <main className={`flex-1 px-6 py-6 ${view === 'graph' ? 'max-w-7xl' : 'max-w-5xl'} w-full mx-auto`}>
        {view === 'dashboard' && <Dashboard ws={ws} />}
        {view === 'admin' && <AdminForms ws={ws} />}
        {view === 'graph' && <KnowledgeGraph />}
        {view === 'generate' && <ComponentGenerator />}
      </main>

      {/* Footer */}
      <footer className="border-t border-gray-800 px-6 py-2 text-xs text-gray-500 text-center">
        WeftOS K8 GUI v0.1.0 &middot; {window.__TAURI_INTERNALS__ ? 'Tauri Desktop' : 'Browser Mode'}
      </footer>
    </div>
  );
}

export default App;
