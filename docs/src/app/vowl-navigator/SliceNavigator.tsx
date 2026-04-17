'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { zoom as d3Zoom, zoomIdentity, type ZoomTransform } from 'd3-zoom';
import { select } from 'd3-selection';

interface PositionedNode {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  label: string;
  node_type: string;
  iri?: string;
  shape: string;
  color: string;
  icon?: string;
  has_subgraph: boolean;
  metrics: Record<string, number>;
}

interface PositionedEdge {
  source_id: string;
  target_id: string;
  label?: string;
  edge_type: string;
  path: [number, number][];
  stroke: string;
  width: number;
  arrow: boolean;
}

interface GraphSlice {
  graph: {
    nodes: PositionedNode[];
    edges: PositionedEdge[];
    viewport: { x: number; y: number; width: number; height: number };
    schema_name: string;
  };
  expandable: string[];
  breadcrumbs: { id: string; label: string }[];
  total_nodes: number;
  depth: number;
}

interface SliceManifest {
  root: string;
  slices: Record<string, string>;
  total_nodes: number;
  total_edges: number;
}

interface SliceNavigatorProps {
  slicesPath?: string;
  width?: number;
  height?: number;
}

export function SliceNavigator({
  slicesPath = '/slices',
  width = 1100,
  height = 650,
}: SliceNavigatorProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [transform, setTransform] = useState<ZoomTransform>(zoomIdentity);
  const [slice, setSlice] = useState<GraphSlice | null>(null);
  const [manifest, setManifest] = useState<SliceManifest | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [breadcrumbs, setBreadcrumbs] = useState<{ id: string; label: string }[]>([]);

  // Load manifest on mount.
  useEffect(() => {
    fetch(`${slicesPath}/manifest.json`)
      .then((r) => r.json())
      .then((m: SliceManifest) => {
        setManifest(m);
        return fetch(`${slicesPath}/${m.root}`);
      })
      .then((r) => r.json())
      .then((s: GraphSlice) => {
        setSlice(s);
        setBreadcrumbs([]);
        setLoading(false);
      })
      .catch((e) => {
        setError(`Failed to load slices: ${e}`);
        setLoading(false);
      });
  }, [slicesPath]);

  // Zoom setup.
  useEffect(() => {
    if (!svgRef.current) return;
    const svg = select(svgRef.current);
    const zoomBehavior = d3Zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 4])
      .on('zoom', (event) => setTransform(event.transform));
    svg.call(zoomBehavior);
    return () => { svg.on('.zoom', null); };
  }, []);

  // Reset zoom when slice changes.
  useEffect(() => {
    if (!svgRef.current || !slice) return;
    const svg = select(svgRef.current);
    const vp = slice.graph.viewport;
    const scaleX = width / vp.width;
    const scaleY = height / vp.height;
    const scale = Math.min(scaleX, scaleY, 1.5) * 0.85;
    const tx = (width - vp.width * scale) / 2 - vp.x * scale;
    const ty = (height - vp.height * scale) / 2 - vp.y * scale;
    const newTransform = zoomIdentity.translate(tx, ty).scale(scale);
    svg.call(d3Zoom<SVGSVGElement, unknown>().transform as any, newTransform);
    setTransform(newTransform);
  }, [slice, width, height]);

  const drillInto = useCallback(
    (nodeId: string) => {
      if (!manifest || !slice) return;
      const sliceFile = manifest.slices[nodeId];
      if (!sliceFile) return;
      setLoading(true);
      setSelected(null);
      const node = slice.graph.nodes.find((n) => n.id === nodeId);
      const newCrumb = { id: nodeId, label: node?.label ?? nodeId.slice(0, 8) };

      fetch(`${slicesPath}/${sliceFile}`)
        .then((r) => r.json())
        .then((s: GraphSlice) => {
          setSlice(s);
          setBreadcrumbs((prev) => [...prev, newCrumb]);
          setLoading(false);
        })
        .catch(() => setLoading(false));
    },
    [manifest, slice, slicesPath],
  );

  const navigateTo = useCallback(
    (depth: number) => {
      if (depth < 0) return;
      setLoading(true);
      setSelected(null);
      if (depth === 0) {
        fetch(`${slicesPath}/${manifest?.root ?? 'root.json'}`)
          .then((r) => r.json())
          .then((s: GraphSlice) => {
            setSlice(s);
            setBreadcrumbs([]);
            setLoading(false);
          });
        return;
      }
      const targetCrumb = breadcrumbs[depth - 1];
      if (!targetCrumb || !manifest) return;
      const sliceFile = manifest.slices[targetCrumb.id];
      if (!sliceFile) return;
      fetch(`${slicesPath}/${sliceFile}`)
        .then((r) => r.json())
        .then((s: GraphSlice) => {
          setSlice(s);
          setBreadcrumbs((prev) => prev.slice(0, depth));
          setLoading(false);
        });
    },
    [manifest, breadcrumbs, slicesPath],
  );

  const selectedNode = slice?.graph.nodes.find((n) => n.id === selected);
  const expandableSet = new Set(slice?.expandable ?? []);

  if (error) {
    return <div className="text-red-500 p-4">{error}</div>;
  }

  return (
    <div className="flex flex-col gap-3">
      {/* Breadcrumbs */}
      <div className="flex items-center gap-1 text-sm">
        <button
          onClick={() => navigateTo(0)}
          className={`px-2 py-0.5 rounded ${breadcrumbs.length === 0 ? 'bg-indigo-600 text-white' : 'text-indigo-400 hover:text-indigo-300 hover:underline'}`}
        >
          Root
        </button>
        {breadcrumbs.map((crumb, i) => (
          <span key={crumb.id} className="flex items-center gap-1">
            <span className="text-neutral-500">/</span>
            <button
              onClick={() => navigateTo(i + 1)}
              className={`px-2 py-0.5 rounded ${i === breadcrumbs.length - 1 ? 'bg-indigo-600 text-white' : 'text-indigo-400 hover:text-indigo-300 hover:underline'}`}
            >
              {crumb.label.replace(/ \(\d+\)$/, '')}
            </button>
          </span>
        ))}
        {slice && (
          <span className="ml-auto text-xs text-neutral-500">
            Depth {breadcrumbs.length} &middot; {slice.graph.nodes.length} nodes &middot; {slice.total_nodes} total
          </span>
        )}
      </div>

      <div className="flex gap-3">
        {/* SVG Canvas */}
        <svg
          ref={svgRef}
          width={width}
          height={height}
          className="rounded border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-950"
          style={{ touchAction: 'none' }}
        >
          <defs>
            <marker id="arr" viewBox="0 0 10 10" refX={10} refY={5} markerWidth={6} markerHeight={6} orient="auto-start-reverse">
              <path d="M 0 0 L 10 5 L 0 10 z" fill="#888" />
            </marker>
          </defs>
          {loading && (
            <text x={width / 2} y={height / 2} textAnchor="middle" fill="#666" fontSize={14}>
              Loading...
            </text>
          )}
          <g transform={`translate(${transform.x},${transform.y}) scale(${transform.k})`}>
            {/* Edges */}
            {slice?.graph.edges.map((e, i) => {
              const src = slice.graph.nodes.find((n) => n.id === e.source_id);
              const tgt = slice.graph.nodes.find((n) => n.id === e.target_id);
              if (!src || !tgt) return null;
              return (
                <line
                  key={i}
                  x1={src.x}
                  y1={src.y}
                  x2={tgt.x}
                  y2={tgt.y}
                  stroke={e.stroke}
                  strokeWidth={e.width}
                  opacity={0.4}
                  markerEnd={e.arrow ? 'url(#arr)' : undefined}
                />
              );
            })}
            {/* Nodes */}
            {slice?.graph.nodes.map((n) => {
              const isExpand = expandableSet.has(n.id);
              const isSel = selected === n.id;
              const r = n.width / 2;

              return (
                <g
                  key={n.id}
                  transform={`translate(${n.x},${n.y})`}
                  className="cursor-pointer"
                  onClick={() => setSelected(n.id === selected ? null : n.id)}
                  onDoubleClick={() => isExpand && drillInto(n.id)}
                >
                  {n.shape === 'rect' || n.shape === 'Rect' ? (
                    <rect
                      x={-r}
                      y={-r * 0.7}
                      width={r * 2}
                      height={r * 1.4}
                      rx={4}
                      fill={n.color}
                      stroke={isSel ? '#ff6600' : '#fff'}
                      strokeWidth={isSel ? 3 : 1.5}
                      opacity={0.9}
                    />
                  ) : n.shape === 'hexagon' || n.shape === 'Hexagon' ? (
                    <polygon
                      points={hexPoints(r)}
                      fill={n.color}
                      stroke={isSel ? '#ff6600' : '#fff'}
                      strokeWidth={isSel ? 3 : 1.5}
                      opacity={0.9}
                    />
                  ) : (
                    <circle
                      r={r}
                      fill={n.color}
                      stroke={isSel ? '#ff6600' : '#fff'}
                      strokeWidth={isSel ? 3 : 1.5}
                      opacity={0.9}
                    />
                  )}
                  {isExpand && (
                    <text
                      x={r - 6}
                      y={-r + 10}
                      fontSize={10}
                      fill="#fff"
                      pointerEvents="none"
                    >
                      +
                    </text>
                  )}
                  <text
                    textAnchor="middle"
                    dy="0.35em"
                    fontSize={Math.min(11, r * 0.6)}
                    fill="#fff"
                    pointerEvents="none"
                    className="select-none"
                  >
                    {n.label.length > 20 ? n.label.slice(0, 18) + '\u2026' : n.label}
                  </text>
                </g>
              );
            })}
          </g>
        </svg>

        {/* Detail Panel */}
        {selectedNode && (
          <div className="w-64 shrink-0 rounded border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-950 p-3 text-sm overflow-y-auto max-h-[650px]">
            <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-2">
              {selectedNode.label}
            </h3>
            <dl className="space-y-1 text-neutral-600 dark:text-neutral-400">
              <dt className="font-medium text-xs uppercase text-neutral-500">Type</dt>
              <dd>{selectedNode.node_type}</dd>
              {selectedNode.iri && (
                <>
                  <dt className="font-medium text-xs uppercase text-neutral-500 mt-2">IRI</dt>
                  <dd className="break-all text-xs">{selectedNode.iri}</dd>
                </>
              )}
              <dt className="font-medium text-xs uppercase text-neutral-500 mt-2">Shape</dt>
              <dd>{selectedNode.shape}</dd>
            </dl>

            {expandableSet.has(selectedNode.id) && (
              <button
                onClick={() => drillInto(selectedNode.id)}
                className="mt-3 w-full px-3 py-1.5 rounded bg-indigo-600 hover:bg-indigo-500 text-white text-sm"
              >
                Drill into children
              </button>
            )}

            {/* Connected edges */}
            <h4 className="font-medium text-xs uppercase text-neutral-500 mt-3 mb-1">Connections</h4>
            <ul className="space-y-1 text-xs">
              {slice?.graph.edges
                .filter((e) => e.source_id === selectedNode.id || e.target_id === selectedNode.id)
                .map((e, i) => {
                  const otherId = e.source_id === selectedNode.id ? e.target_id : e.source_id;
                  const otherNode = slice.graph.nodes.find((n) => n.id === otherId);
                  const dir = e.source_id === selectedNode.id ? '\u2192' : '\u2190';
                  return (
                    <li key={i} className="text-neutral-600 dark:text-neutral-400">
                      <span className="text-neutral-400">{dir}</span>{' '}
                      <button
                        onClick={() => setSelected(otherId)}
                        className="text-blue-400 hover:underline"
                      >
                        {otherNode?.label ?? otherId.slice(0, 12)}
                      </button>
                      {e.label && <span className="text-neutral-500 ml-1">({e.label})</span>}
                    </li>
                  );
                })}
            </ul>

            <button
              onClick={() => setSelected(null)}
              className="mt-3 text-xs text-neutral-500 hover:text-neutral-300"
            >
              Clear
            </button>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="flex flex-wrap gap-4 text-xs text-neutral-500">
        <span>Double-click expandable nodes (+) to drill in</span>
        <span>&middot;</span>
        <span>Click breadcrumbs to navigate back</span>
        <span>&middot;</span>
        <span>Scroll to zoom, drag to pan</span>
      </div>
    </div>
  );
}

function hexPoints(r: number): string {
  const pts: string[] = [];
  for (let i = 0; i < 6; i++) {
    const angle = (Math.PI / 3) * i - Math.PI / 6;
    pts.push(`${(r * Math.cos(angle)).toFixed(1)},${(r * Math.sin(angle)).toFixed(1)}`);
  }
  return pts.join(' ');
}
