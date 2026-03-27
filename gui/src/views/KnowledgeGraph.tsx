import { useEffect, useRef, useState, useMemo } from 'react';
import cytoscape from 'cytoscape';
import type { Core } from 'cytoscape';
import type { CausalNode, CausalEdge, CausalEdgeType, GraphData } from '../types/kernel';
import { NodeDetail } from '../components/NodeDetail';

// ---------------------------------------------------------------------------
// Edge color map
// ---------------------------------------------------------------------------
const EDGE_COLORS: Record<CausalEdgeType, string> = {
  Causes: '#ef4444',
  Enables: '#22c55e',
  Correlates: '#3b82f6',
  Follows: '#9ca3af',
  Inhibits: '#f97316',
  Contradicts: '#a855f7',
  EvidenceFor: '#14b8a6',
  TriggeredBy: '#eab308',
};

// Community background palette (muted, dark-theme friendly)
const COMMUNITY_COLORS = [
  '#1e3a5f', '#3b1f2b', '#1a3c34', '#3d2e1e', '#2d1b4e',
];

// ---------------------------------------------------------------------------
// Mock graph generator
// ---------------------------------------------------------------------------
function generateMockGraph(): GraphData {
  const EDGE_TYPES: CausalEdgeType[] = [
    'Causes', 'Enables', 'Correlates', 'Follows',
    'Inhibits', 'Contradicts', 'TriggeredBy', 'EvidenceFor',
  ];

  const labels = [
    'module:scheduler', 'module:registry', 'module:governance',
    'module:mesh-sync', 'module:exochain', 'module:ecc-index',
    'commit:a1b2c3', 'commit:d4e5f6', 'commit:789abc', 'commit:def012',
    'decision:D1', 'decision:D2', 'decision:D3', 'decision:D4', 'decision:D5',
    'agent:weaver-0', 'agent:coder-1', 'agent:reviewer-2', 'agent:planner-3',
    'config:max_agents', 'config:tick_rate', 'config:mesh_peers',
    'event:spawn', 'event:govern', 'event:chain-append',
    'service:registry', 'service:mesh', 'service:ecc',
    'rule:R1', 'rule:R2', 'rule:R3',
  ];

  // Assign communities: 0-9 = community 0, 10-19 = community 1, 20-29 = community 2
  const communities: number[][] = [
    Array.from({ length: 10 }, (_, i) => i),
    Array.from({ length: 10 }, (_, i) => i + 10),
    Array.from({ length: 10 }, (_, i) => i + 20),
  ];

  const nodes: CausalNode[] = labels.map((label, i) => ({
    id: i,
    label,
    metadata: { created: '2026-03-27', importance: +(Math.random() * 10).toFixed(1) },
    community: i < 10 ? 0 : i < 20 ? 1 : 2,
  }));

  // Generate ~50 edges, biased towards intra-community links
  const edges: CausalEdge[] = [];
  const seen = new Set<string>();
  while (edges.length < 50) {
    const src = Math.floor(Math.random() * nodes.length);
    let tgt: number;
    // 60% intra-community, 40% inter-community
    if (Math.random() < 0.6) {
      const comm = nodes[src].community!;
      const pool = communities[comm];
      tgt = pool[Math.floor(Math.random() * pool.length)];
    } else {
      tgt = Math.floor(Math.random() * nodes.length);
    }
    if (src === tgt) continue;
    const key = `${src}-${tgt}`;
    if (seen.has(key)) continue;
    seen.add(key);
    edges.push({
      source: src,
      target: tgt,
      edge_type: EDGE_TYPES[Math.floor(Math.random() * EDGE_TYPES.length)],
      weight: +(Math.random() * 0.9 + 0.1).toFixed(2),
    });
  }

  const lambda_2 = +(Math.random() * 0.4 + 0.5).toFixed(3);

  return { nodes, edges, lambda_2, communities };
}

// ---------------------------------------------------------------------------
// Coherence gauge
// ---------------------------------------------------------------------------
function CoherenceGauge({ value }: { value: number }) {
  const pct = Math.min(Math.max(value, 0), 1) * 100;
  const color =
    value > 0.7 ? 'bg-emerald-500' : value > 0.5 ? 'bg-yellow-500' : 'bg-red-500';
  return (
    <div className="flex items-center gap-2 text-xs text-gray-400">
      <span>Lambda-2</span>
      <div className="w-24 h-2 bg-gray-700 rounded overflow-hidden">
        <div className={`h-full ${color} rounded`} style={{ width: `${pct}%` }} />
      </div>
      <span className="font-mono text-gray-300">{value.toFixed(3)}</span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main view
// ---------------------------------------------------------------------------
export function KnowledgeGraph() {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const [search, setSearch] = useState('');
  const [selectedNode, setSelectedNode] = useState<CausalNode | null>(null);
  const [graphData] = useState<GraphData>(() => generateMockGraph());

  const nodeMap = useMemo(
    () => new Map(graphData.nodes.map((n) => [n.id, n])),
    [graphData.nodes],
  );

  // Degree map for sizing
  const degreeMap = useMemo(() => {
    const m = new Map<number, number>();
    for (const n of graphData.nodes) m.set(n.id, 0);
    for (const e of graphData.edges) {
      m.set(e.source, (m.get(e.source) ?? 0) + 1);
      m.set(e.target, (m.get(e.target) ?? 0) + 1);
    }
    return m;
  }, [graphData]);

  // Init cytoscape
  useEffect(() => {
    if (!containerRef.current) return;

    const cy = cytoscape({
      container: containerRef.current,
      elements: [
        ...graphData.nodes.map((n) => ({
          data: {
            id: String(n.id),
            label: n.label,
            community: n.community ?? 0,
            degree: degreeMap.get(n.id) ?? 0,
          },
        })),
        ...graphData.edges.map((e, i) => ({
          data: {
            id: `e${i}`,
            source: String(e.source),
            target: String(e.target),
            edgeType: e.edge_type,
            weight: e.weight,
          },
        })),
      ],
      style: [
        {
          selector: 'node',
          style: {
            label: 'data(label)',
            'font-size': '9px',
            color: '#d1d5db',
            'text-valign': 'bottom',
            'text-margin-y': 4,
            'background-color': (ele: cytoscape.NodeSingular) =>
              COMMUNITY_COLORS[ele.data('community') % COMMUNITY_COLORS.length],
            'border-width': 1,
            'border-color': '#6b7280',
            width: (ele: cytoscape.NodeSingular) =>
              Math.max(20, 12 + ele.data('degree') * 4),
            height: (ele: cytoscape.NodeSingular) =>
              Math.max(20, 12 + ele.data('degree') * 4),
          } as cytoscape.Css.Node,
        },
        {
          selector: 'node.highlighted',
          style: {
            'border-width': 2,
            'border-color': '#f59e0b',
          },
        },
        {
          selector: 'node.dimmed',
          style: {
            opacity: 0.2,
          },
        },
        {
          selector: 'edge',
          style: {
            width: 1.5,
            'line-color': (ele: cytoscape.EdgeSingular) =>
              EDGE_COLORS[ele.data('edgeType') as CausalEdgeType] ?? '#6b7280',
            'target-arrow-color': (ele: cytoscape.EdgeSingular) =>
              EDGE_COLORS[ele.data('edgeType') as CausalEdgeType] ?? '#6b7280',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            'arrow-scale': 0.8,
            opacity: 0.7,
          } as cytoscape.Css.Edge,
        },
        {
          selector: 'edge.dimmed',
          style: {
            opacity: 0.08,
          },
        },
      ],
      layout: { name: 'cose', animate: false, nodeDimensionsIncludeLabels: true },
      minZoom: 0.3,
      maxZoom: 3,
    });

    cy.on('tap', 'node', (evt) => {
      const id = Number(evt.target.id());
      const node = nodeMap.get(id);
      if (node) setSelectedNode(node);
    });

    cy.on('tap', (evt) => {
      if (evt.target === cy) setSelectedNode(null);
    });

    cyRef.current = cy;

    return () => {
      cy.destroy();
      cyRef.current = null;
    };
  }, [graphData, degreeMap, nodeMap]);

  // Search filtering
  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    if (!search.trim()) {
      cy.elements().removeClass('dimmed highlighted');
      return;
    }
    const lower = search.toLowerCase();
    cy.nodes().forEach((n) => {
      const matches = (n.data('label') as string).toLowerCase().includes(lower);
      n.toggleClass('highlighted', matches);
      n.toggleClass('dimmed', !matches);
    });
    cy.edges().forEach((e) => {
      const srcMatch = (e.source().data('label') as string).toLowerCase().includes(lower);
      const tgtMatch = (e.target().data('label') as string).toLowerCase().includes(lower);
      e.toggleClass('dimmed', !srcMatch && !tgtMatch);
    });
  }, [search]);

  return (
    <div className="space-y-3">
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search nodes..."
          className="bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm text-gray-100 placeholder:text-gray-500 w-64 focus:outline-none focus:border-gray-500"
        />
        <CoherenceGauge value={graphData.lambda_2} />
      </div>

      {/* Edge legend */}
      <div className="flex flex-wrap gap-3 text-xs text-gray-400">
        {Object.entries(EDGE_COLORS).map(([type, color]) => (
          <span key={type} className="flex items-center gap-1">
            <span
              className="inline-block w-3 h-0.5 rounded"
              style={{ backgroundColor: color }}
            />
            {type}
          </span>
        ))}
      </div>

      {/* Graph + detail panel */}
      <div className="flex gap-3">
        <div
          ref={containerRef}
          className="flex-1 bg-gray-900 border border-gray-700 rounded-lg"
          style={{ height: 'calc(100vh - 16rem)' }}
        />
        {selectedNode && (
          <NodeDetail
            node={selectedNode}
            edges={graphData.edges}
            nodeMap={nodeMap}
            onClose={() => setSelectedNode(null)}
          />
        )}
      </div>
    </div>
  );
}
