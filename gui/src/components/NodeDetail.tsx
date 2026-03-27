import type { CausalNode, CausalEdge } from '../types/kernel';

interface Props {
  node: CausalNode;
  edges: CausalEdge[];
  nodeMap: Map<number, CausalNode>;
  onClose: () => void;
}

const EDGE_COLORS: Record<string, string> = {
  Causes: 'text-red-400',
  Enables: 'text-emerald-400',
  Correlates: 'text-blue-400',
  Follows: 'text-gray-400',
  Inhibits: 'text-orange-400',
  Contradicts: 'text-purple-400',
  EvidenceFor: 'text-teal-400',
  TriggeredBy: 'text-yellow-400',
};

export function NodeDetail({ node, edges, nodeMap, onClose }: Props) {
  const connected = edges.filter(
    (e) => e.source === node.id || e.target === node.id,
  );
  const incoming = connected.filter((e) => e.target === node.id);
  const outgoing = connected.filter((e) => e.source === node.id);

  return (
    <div className="w-72 bg-gray-900 border border-gray-700 rounded-lg p-4 text-sm space-y-3 overflow-y-auto max-h-[calc(100vh-12rem)]">
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-gray-100 truncate">{node.label}</h3>
        <button
          onClick={onClose}
          className="text-gray-500 hover:text-gray-300 text-xs ml-2"
        >
          Close
        </button>
      </div>

      <div className="text-xs text-gray-400 space-y-1">
        <p>ID: {node.id}</p>
        {node.community !== undefined && <p>Community: {node.community}</p>}
      </div>

      {Object.keys(node.metadata).length > 0 && (
        <div>
          <p className="text-xs font-medium text-gray-300 mb-1">Metadata</p>
          <pre className="text-xs text-gray-400 bg-gray-800 rounded p-2 overflow-x-auto">
            {JSON.stringify(node.metadata, null, 2)}
          </pre>
        </div>
      )}

      {incoming.length > 0 && (
        <div>
          <p className="text-xs font-medium text-gray-300 mb-1">
            Incoming ({incoming.length})
          </p>
          <ul className="space-y-0.5">
            {incoming.map((e, i) => (
              <li key={i} className="text-xs">
                <span className={EDGE_COLORS[e.edge_type] ?? 'text-gray-400'}>
                  {e.edge_type}
                </span>{' '}
                <span className="text-gray-500">from</span>{' '}
                <span className="text-gray-300">
                  {nodeMap.get(e.source)?.label ?? e.source}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {outgoing.length > 0 && (
        <div>
          <p className="text-xs font-medium text-gray-300 mb-1">
            Outgoing ({outgoing.length})
          </p>
          <ul className="space-y-0.5">
            {outgoing.map((e, i) => (
              <li key={i} className="text-xs">
                <span className={EDGE_COLORS[e.edge_type] ?? 'text-gray-400'}>
                  {e.edge_type}
                </span>{' '}
                <span className="text-gray-500">to</span>{' '}
                <span className="text-gray-300">
                  {nodeMap.get(e.target)?.label ?? e.target}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
