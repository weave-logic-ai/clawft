import { useState, type FormEvent } from 'react';
import type { UseKernelWsReturn } from '../hooks/useKernelWs';
import type { KernelResponse } from '../types/kernel';
import { FormField } from '../components/FormField';

interface Props {
  ws: UseKernelWsReturn;
}

/** Shared result display */
function ResultBox({ result, pending }: { result: KernelResponse | null; pending: boolean }) {
  if (pending) return <p className="text-xs text-gray-400 mt-2">Sending...</p>;
  if (!result) return null;
  return (
    <pre
      className={`mt-2 text-xs p-2 rounded border ${
        result.ok
          ? 'bg-emerald-900/20 border-emerald-700/40 text-emerald-300'
          : 'bg-red-900/20 border-red-700/40 text-red-300'
      } overflow-x-auto`}
    >
      {JSON.stringify(result, null, 2)}
    </pre>
  );
}

/** Generic form section wrapper */
function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-gray-800/40 border border-gray-700 rounded-lg p-4">
      <h3 className="text-sm font-semibold text-gray-200 mb-3">{title}</h3>
      {children}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Spawn Agent
// ---------------------------------------------------------------------------
function SpawnAgentForm({ ws }: { ws: UseKernelWsReturn }) {
  const [agentId, setAgentId] = useState('');
  const [result, setResult] = useState<KernelResponse | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!agentId.trim()) return;
    setPending(true);
    setResult(null);
    const res = await ws.sendCommand({ kind: 'spawn_agent', payload: { agent_id: agentId } });
    setResult(res);
    setPending(false);
  };

  return (
    <Section title="Spawn Agent">
      <form onSubmit={submit} className="flex items-end gap-3">
        <FormField label="Agent ID" id="spawn-agent-id" value={agentId} onChange={(e) => setAgentId(e.target.value)} placeholder="e.g. coder-5" />
        <button type="submit" disabled={pending} className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 disabled:opacity-40 text-sm rounded text-white">
          Spawn
        </button>
      </form>
      <ResultBox result={result} pending={pending} />
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Stop Agent
// ---------------------------------------------------------------------------
function StopAgentForm({ ws }: { ws: UseKernelWsReturn }) {
  const [pid, setPid] = useState('');
  const [result, setResult] = useState<KernelResponse | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!pid.trim()) return;
    setPending(true);
    setResult(null);
    const res = await ws.sendCommand({ kind: 'stop_agent', payload: { pid: Number(pid) } });
    setResult(res);
    setPending(false);
  };

  return (
    <Section title="Stop Agent">
      <form onSubmit={submit} className="flex items-end gap-3">
        <FormField label="PID" id="stop-pid" type="number" value={pid} onChange={(e) => setPid(e.target.value)} placeholder="e.g. 3" />
        <button type="submit" disabled={pending} className="px-4 py-1.5 bg-red-600 hover:bg-red-500 disabled:opacity-40 text-sm rounded text-white">
          Stop
        </button>
      </form>
      <ResultBox result={result} pending={pending} />
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Set Config
// ---------------------------------------------------------------------------
function SetConfigForm({ ws }: { ws: UseKernelWsReturn }) {
  const [ns, setNs] = useState('');
  const [key, setKey] = useState('');
  const [val, setVal] = useState('');
  const [result, setResult] = useState<KernelResponse | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!key.trim()) return;
    setPending(true);
    setResult(null);
    const res = await ws.sendCommand({ kind: 'set_config', payload: { namespace: ns, key, value: val } });
    setResult(res);
    setPending(false);
  };

  return (
    <Section title="Set Config">
      <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
        <FormField label="Namespace" id="cfg-ns" value={ns} onChange={(e) => setNs(e.target.value)} placeholder="e.g. kernel" />
        <FormField label="Key" id="cfg-key" value={key} onChange={(e) => setKey(e.target.value)} placeholder="e.g. max_agents" />
        <FormField label="Value" id="cfg-val" value={val} onChange={(e) => setVal(e.target.value)} placeholder="e.g. 16" />
        <button type="submit" disabled={pending} className="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-sm rounded text-white">
          Save
        </button>
      </form>
      <ResultBox result={result} pending={pending} />
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Query Chain
// ---------------------------------------------------------------------------
function QueryChainForm({ ws }: { ws: UseKernelWsReturn }) {
  const [fromSeq, setFromSeq] = useState('0');
  const [limit, setLimit] = useState('5');
  const [result, setResult] = useState<KernelResponse | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setPending(true);
    setResult(null);
    const res = await ws.sendCommand({ kind: 'query_chain', payload: { from_seq: Number(fromSeq), limit: Number(limit) } });
    setResult(res);
    setPending(false);
  };

  return (
    <Section title="Query Chain">
      <form onSubmit={submit} className="flex items-end gap-3">
        <FormField label="From Seq" id="chain-from" type="number" value={fromSeq} onChange={(e) => setFromSeq(e.target.value)} />
        <FormField label="Limit" id="chain-limit" type="number" value={limit} onChange={(e) => setLimit(e.target.value)} />
        <button type="submit" disabled={pending} className="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 disabled:opacity-40 text-sm rounded text-white">
          Query
        </button>
      </form>
      <ResultBox result={result} pending={pending} />
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Register Service
// ---------------------------------------------------------------------------
function RegisterServiceForm({ ws }: { ws: UseKernelWsReturn }) {
  const [name, setName] = useState('');
  const [stype, setStype] = useState('');
  const [result, setResult] = useState<KernelResponse | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setPending(true);
    setResult(null);
    const res = await ws.sendCommand({ kind: 'register_service', payload: { name, service_type: stype } });
    setResult(res);
    setPending(false);
  };

  return (
    <Section title="Register Service">
      <form onSubmit={submit} className="flex items-end gap-3">
        <FormField label="Name" id="svc-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. cache" />
        <FormField label="Type" id="svc-type" value={stype} onChange={(e) => setStype(e.target.value)} placeholder="e.g. storage" />
        <button type="submit" disabled={pending} className="px-4 py-1.5 bg-teal-600 hover:bg-teal-500 disabled:opacity-40 text-sm rounded text-white">
          Register
        </button>
      </form>
      <ResultBox result={result} pending={pending} />
    </Section>
  );
}

// ---------------------------------------------------------------------------
// Admin Forms root
// ---------------------------------------------------------------------------
export function AdminForms({ ws }: Props) {
  return (
    <div className="space-y-4">
      <p className="text-sm text-gray-400">
        Each form sends a command to the kernel and displays the response.
        Currently using mock data — wire to real kernel WS in follow-up.
      </p>
      <SpawnAgentForm ws={ws} />
      <StopAgentForm ws={ws} />
      <SetConfigForm ws={ws} />
      <QueryChainForm ws={ws} />
      <RegisterServiceForm ws={ws} />
    </div>
  );
}
