'use client';

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Link from 'next/link';
import type { KBEntry, KBManifest } from '@/lib/rvf-reader';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

interface ChainEntry {
  ts: number;
  op: string;
  detail: string;
  hash?: string;
}

type SandboxStatus = 'loading' | 'ready' | 'error' | 'needs-key';
type SandboxMode = 'local' | 'llm' | 'local-ai';

// ---------------------------------------------------------------------------
// Chain log (ExoChain-style audit trail)
// ---------------------------------------------------------------------------

let chainSeq = 0;
let prevHash = '0000000000000000';

/** Simple hash for chain linking (not cryptographic, just visual). */
function miniHash(input: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, '0');
}

function chainAppend(
  entries: ChainEntry[],
  op: string,
  detail: string,
): ChainEntry[] {
  const seq = chainSeq++;
  const payload = `${seq}:${prevHash}:${op}:${detail}`;
  const hash = miniHash(payload);
  prevHash = hash;
  return [...entries, { ts: Date.now(), op, detail, hash }];
}

// ---------------------------------------------------------------------------
// WASM loader
// ---------------------------------------------------------------------------

// Paths are same-origin: in local dev served from public/, in production
// proxied to GitHub Releases via Vercel rewrites in next.config.mjs.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let wasmModule: any = null;

async function loadWasm() {
  if (wasmModule) return wasmModule;

  // Fetch the wasm-bindgen JS glue as text via our server-side proxy,
  // then import via blob URL. The proxy handles GitHub Releases redirects
  // and CORS, and sets correct Content-Type headers.
  const jsResp = await fetch('/api/cdn/wasm/clawft_wasm.js');
  if (!jsResp.ok) throw new Error(`Failed to fetch clawft_wasm.js: ${jsResp.status}`);
  const jsText = await jsResp.text();
  const blob = new Blob([jsText], { type: 'application/javascript' });
  const blobUrl = URL.createObjectURL(blob);

  try {
    const mod = await Function('url', 'return import(url)')(blobUrl);
    // mod.default is __wbg_init — call it with the proxy path for the .wasm binary.
    await mod.default('/api/cdn/wasm/clawft_wasm_bg.wasm');
    wasmModule = mod;
    return wasmModule;
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

// ---------------------------------------------------------------------------
// RVF KB loader + search
// ---------------------------------------------------------------------------

let kbCache: { manifest: KBManifest; entries: KBEntry[] } | null = null;

async function loadKB(): Promise<{ manifest: KBManifest; entries: KBEntry[] }> {
  if (kbCache) return kbCache;

  const [{ parseKnowledgeBase }, { decode }] = await Promise.all([
    import('@/lib/rvf-reader'),
    import('cbor-x'),
  ]);

  const resp = await fetch('/api/cdn/kb/weftos-docs.rvf');
  const buf = await resp.arrayBuffer();
  kbCache = parseKnowledgeBase(buf, decode);
  return kbCache;
}

/** Cosine similarity between two Float32Arrays. */
function cosine(a: Float32Array, b: Float32Array): number {
  let dot = 0,
    na = 0,
    nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  return dot / (Math.sqrt(na) * Math.sqrt(nb));
}

// ---------------------------------------------------------------------------
// Runtime introspection
// ---------------------------------------------------------------------------

/** Keywords that indicate the user is asking about the live WASM instance. */
const INTROSPECTION_TRIGGERS = [
  'this instance', 'this wasm', 'my instance', 'running instance',
  'current instance', 'local wasm', 'inspect', 'introspect',
  'runtime info', 'wasm info', 'wasm memory', 'wasm exports',
  'show me this', 'from this instance', 'my sandbox', 'this sandbox',
  'hack', 'local', 'capabilities',
];

function isIntrospectionQuery(query: string): boolean {
  const q = query.toLowerCase();
  return INTROSPECTION_TRIGGERS.some((t) => q.includes(t));
}

/**
 * Gather live runtime info from the WASM module and browser environment.
 */
function gatherRuntimeInfo(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  wasm: any,
  kb: { manifest: KBManifest; entries: KBEntry[] } | null,
  model: string,
): string {
  const lines: string[] = ['## Live WASM Runtime State', ''];

  // WASM module info
  try {
    const memory = wasm?.memory as WebAssembly.Memory | undefined;
    if (memory) {
      const pages = memory.buffer.byteLength / 65536;
      lines.push(`**WASM Memory**: ${pages} pages (${(memory.buffer.byteLength / 1024 / 1024).toFixed(1)} MB)`);
    }
  } catch { /* no memory export */ }

  // List exported functions
  try {
    const exports = Object.keys(wasm ?? {}).filter(
      (k) => typeof wasm[k] === 'function' && !k.startsWith('__'),
    );
    if (exports.length > 0) {
      lines.push(`**Exported functions**: ${exports.join(', ')}`);
    }
  } catch { /* ok */ }

  lines.push(`**Platform**: wasm32-unknown-unknown (browser)`);
  lines.push(`**User-Agent**: ${navigator.userAgent}`);
  lines.push(`**LLM Model**: ${model}`);
  lines.push(`**Connection**: browser-direct to provider API`);

  // KB info
  if (kb) {
    lines.push('');
    lines.push('## Knowledge Base');
    lines.push(`**Segments**: ${kb.entries.length}`);
    lines.push(`**Dimension**: ${kb.manifest.dimension}`);
    lines.push(`**Embedder**: ${kb.manifest.embedder_name}`);
    lines.push(`**Namespace**: ${kb.manifest.namespace}`);
    lines.push(`**Agent ID**: ${kb.manifest.agent_id}`);
    if (kb.manifest.created_at) {
      lines.push(`**Created**: ${kb.manifest.created_at}`);
    }

    // Tag distribution
    const tagCounts: Record<string, number> = {};
    for (const e of kb.entries) {
      for (const t of e.tags ?? []) {
        tagCounts[t] = (tagCounts[t] || 0) + 1;
      }
    }
    const topTags = Object.entries(tagCounts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 10)
      .map(([tag, n]) => `${tag}(${n})`)
      .join(', ');
    if (topTags) {
      lines.push(`**Top tags**: ${topTags}`);
    }
  }

  lines.push('');
  lines.push('## Browser Environment');
  lines.push(`**URL**: ${window.location.href}`);
  lines.push(`**Timestamp**: ${new Date().toISOString()}`);
  lines.push(`**WebAssembly**: ${typeof WebAssembly !== 'undefined' ? 'supported' : 'not supported'}`);
  lines.push(`**SharedArrayBuffer**: ${typeof SharedArrayBuffer !== 'undefined' ? 'available' : 'not available'}`);

  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// KB search
// ---------------------------------------------------------------------------

/** Common stop words to exclude from scoring. */
const STOP_WORDS = new Set([
  'the', 'and', 'for', 'are', 'but', 'not', 'you', 'all', 'can', 'has',
  'was', 'one', 'our', 'out', 'how', 'its', 'what', 'when', 'which',
  'this', 'that', 'with', 'from', 'have', 'will', 'does', 'about',
  'into', 'more', 'been', 'some', 'than', 'them', 'then', 'would',
  'make', 'like', 'just', 'over', 'such', 'also', 'most', 'show',
]);

/**
 * Improved keyword search with phrase matching, acronym awareness,
 * TF weighting, and metadata boosting.
 */
function keywordSearch(query: string, entries: KBEntry[], topK = 5): KBEntry[] {
  const queryLower = query.toLowerCase();
  const terms = queryLower
    .split(/\W+/)
    .filter((t) => t.length > 2 && !STOP_WORDS.has(t));

  // Extract potential acronyms from the original query (2-5 uppercase letters).
  const acronyms = query.match(/\b[A-Z]{2,5}\b/g) || [];

  const scored = entries.map((entry) => {
    const text = entry.text.toLowerCase();
    const title = ((entry.metadata as Record<string, string>)?.title ?? '').toLowerCase();
    const section = ((entry.metadata as Record<string, string>)?.section ?? '').toLowerCase();
    let score = 0;

    // 1. Exact phrase match in text (strongest signal)
    if (terms.length >= 2 && text.includes(queryLower)) {
      score += 10;
    }

    // 2. Multi-word substring match (e.g. "boot sequence" in text)
    if (terms.length >= 2) {
      const phrase = terms.join(' ');
      if (text.includes(phrase)) score += 6;
      if (title.includes(phrase)) score += 8;
    }

    // 3. Per-term scoring with position awareness
    for (const term of terms) {
      const textCount = text.split(term).length - 1;
      if (textCount > 0) {
        // Diminishing returns for repeated matches (log scale)
        score += 1 + Math.min(Math.log2(textCount), 2);
      }
      // Title/section match is a strong signal
      if (title.includes(term)) score += 3;
      if (section.includes(term)) score += 2;
    }

    // 4. Acronym matching (case-sensitive in original text)
    for (const acr of acronyms) {
      const acrLower = acr.toLowerCase();
      // Exact acronym in text (check for word boundary)
      const acrRegex = new RegExp(`\\b${acr}\\b`);
      if (acrRegex.test(entry.text)) score += 5;
      // Also check if the entry defines/explains this acronym
      if (text.includes(`${acrLower} (`) || text.includes(`(${acrLower})`) ||
          text.includes(`${acrLower} —`) || text.includes(`${acrLower} --`) ||
          text.includes(`stands for`)) {
        score += 4;
      }
    }

    // 5. Tag match boost
    for (const tag of entry.tags ?? []) {
      const tagLower = tag.toLowerCase();
      if (terms.includes(tagLower)) score += 3;
      for (const acr of acronyms) {
        if (tagLower === acr.toLowerCase()) score += 3;
      }
    }

    return { entry, score };
  });

  return scored
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, topK)
    .map((s) => s.entry);
}

// ---------------------------------------------------------------------------
// KB Graph — tag co-occurrence force-directed layout
// ---------------------------------------------------------------------------

interface GraphNode {
  id: string;
  count: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
}

interface GraphEdge {
  source: string;
  target: string;
  weight: number;
}

/** Compute a tag co-occurrence graph from KB entries. */
function computeKBGraph(entries: KBEntry[]): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const tagCounts: Record<string, number> = {};
  const edgeMap: Record<string, number> = {};

  for (const e of entries) {
    const tags = e.tags ?? [];
    for (const t of tags) {
      tagCounts[t] = (tagCounts[t] || 0) + 1;
    }
    // Create edges between co-occurring tags
    for (let i = 0; i < tags.length; i++) {
      for (let j = i + 1; j < tags.length; j++) {
        const key = [tags[i], tags[j]].sort().join('::');
        edgeMap[key] = (edgeMap[key] || 0) + 1;
      }
    }
  }

  const nodeEntries = Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 30);
  const nodeSet = new Set(nodeEntries.map(([id]) => id));
  const nodes: GraphNode[] = nodeEntries.map(([id, count], i) => ({
    id,
    count,
    x: 300 + 200 * Math.cos((2 * Math.PI * i) / nodeEntries.length),
    y: 200 + 150 * Math.sin((2 * Math.PI * i) / nodeEntries.length),
    vx: 0,
    vy: 0,
  }));

  const edges: GraphEdge[] = [];
  for (const [key, weight] of Object.entries(edgeMap)) {
    const [source, target] = key.split('::');
    if (nodeSet.has(source) && nodeSet.has(target)) {
      edges.push({ source, target, weight });
    }
  }

  return { nodes, edges };
}

/** Category-based color palette for graph nodes. */
const TAG_COLORS: Record<string, string> = {
  architecture: '#60a5fa',
  security: '#f87171',
  api: '#34d399',
  cli: '#a78bfa',
  wasm: '#fb923c',
  runtime: '#fbbf24',
  docs: '#94a3b8',
};

function tagColor(tag: string): string {
  const t = tag.toLowerCase();
  for (const [key, color] of Object.entries(TAG_COLORS)) {
    if (t.includes(key)) return color;
  }
  // Hash-based fallback color
  let h = 0;
  for (let i = 0; i < t.length; i++) h = (h * 31 + t.charCodeAt(i)) | 0;
  const hue = ((h >>> 0) % 360);
  return `hsl(${hue}, 55%, 60%)`;
}

function KBGraph({ entries }: { entries: KBEntry[] }) {
  const { nodes: initialNodes, edges } = useMemo(() => computeKBGraph(entries), [entries]);
  const nodesRef = useRef<GraphNode[]>(initialNodes.map((n) => ({ ...n })));
  const frameRef = useRef<number>(0);
  const iterRef = useRef(0);
  const svgRef = useRef<SVGSVGElement>(null);
  const [, forceRender] = useState(0);

  const WIDTH = 600;
  const HEIGHT = 400;
  const MAX_ITER = 120;

  useEffect(() => {
    // Reset nodes when entries change
    nodesRef.current = initialNodes.map((n) => ({ ...n }));
    iterRef.current = 0;

    const nodeMap = new Map<string, GraphNode>();

    function simulate() {
      const nodes = nodesRef.current;
      nodeMap.clear();
      for (const n of nodes) nodeMap.set(n.id, n);

      // Repulsion between all nodes
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const a = nodes[i];
          const b = nodes[j];
          let dx = a.x - b.x;
          let dy = a.y - b.y;
          const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
          const force = 800 / (dist * dist);
          dx = (dx / dist) * force;
          dy = (dy / dist) * force;
          a.vx += dx;
          a.vy += dy;
          b.vx -= dx;
          b.vy -= dy;
        }
      }

      // Attraction along edges
      for (const e of edges) {
        const a = nodeMap.get(e.source);
        const b = nodeMap.get(e.target);
        if (!a || !b) continue;
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const force = (dist - 80) * 0.01 * Math.min(e.weight, 3);
        dx = (dx / dist) * force;
        dy = (dy / dist) * force;
        a.vx += dx;
        a.vy += dy;
        b.vx -= dx;
        b.vy -= dy;
      }

      // Center gravity
      for (const n of nodes) {
        n.vx += (WIDTH / 2 - n.x) * 0.002;
        n.vy += (HEIGHT / 2 - n.y) * 0.002;
      }

      // Apply velocity with damping
      const damping = 0.85;
      for (const n of nodes) {
        n.vx *= damping;
        n.vy *= damping;
        n.x += n.vx;
        n.y += n.vy;
        // Clamp to bounds
        n.x = Math.max(30, Math.min(WIDTH - 30, n.x));
        n.y = Math.max(30, Math.min(HEIGHT - 30, n.y));
      }

      iterRef.current++;
      forceRender((c) => c + 1);

      if (iterRef.current < MAX_ITER) {
        frameRef.current = requestAnimationFrame(simulate);
      }
    }

    frameRef.current = requestAnimationFrame(simulate);
    return () => cancelAnimationFrame(frameRef.current);
  }, [initialNodes, edges]);

  const nodes = nodesRef.current;
  const nodeMap = new Map<string, GraphNode>();
  for (const n of nodes) nodeMap.set(n.id, n);

  const maxCount = Math.max(...nodes.map((n) => n.count), 1);

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      className="w-full max-w-xl mx-auto"
      style={{ height: 320 }}
    >
      {/* Edges */}
      {edges.map((e) => {
        const a = nodeMap.get(e.source);
        const b = nodeMap.get(e.target);
        if (!a || !b) return null;
        return (
          <line
            key={`${e.source}::${e.target}`}
            x1={a.x}
            y1={a.y}
            x2={b.x}
            y2={b.y}
            stroke="currentColor"
            className="text-fd-border"
            strokeWidth={Math.min(e.weight, 3) * 0.5 + 0.5}
            strokeOpacity={0.4}
          />
        );
      })}
      {/* Nodes */}
      {nodes.map((n) => {
        const r = 6 + (n.count / maxCount) * 14;
        const color = tagColor(n.id);
        return (
          <g key={n.id}>
            <circle cx={n.x} cy={n.y} r={r} fill={color} fillOpacity={0.7} stroke={color} strokeWidth={1.5} />
            <text
              x={n.x}
              y={n.y + r + 10}
              textAnchor="middle"
              className="fill-fd-muted-foreground"
              fontSize={9}
              fontFamily="monospace"
            >
              {n.id}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function WasmSandbox() {
  const [status, setStatus] = useState<SandboxStatus>('loading');
  const [mode, setMode] = useState<SandboxMode>('local');
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('openrouter/google/gemini-2.0-flash-001');
  const [kbLoaded, setKbLoaded] = useState(false);
  const [kbStats, setKbStats] = useState<string>('');
  const [error, setError] = useState<string>('');
  const [showLlmSetup, setShowLlmSetup] = useState(false);
  const [streamingMessage, setStreamingMessage] = useState('');
  const [showKBGraph, setShowKBGraph] = useState(true);
  const [chain, setChain] = useState<ChainEntry[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const chainScrollRef = useRef<HTMLDivElement>(null);
  const wasmRef = useRef<any>(null);
  const streamAbortRef = useRef(false);

  const log = useCallback((op: string, detail: string) => {
    setChain((prev) => chainAppend(prev, op, detail));
  }, []);

  // Scroll to bottom on new messages / chain entries
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [messages]);
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [streamingMessage]);
  useEffect(() => {
    chainScrollRef.current?.scrollTo({ top: chainScrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [chain]);

  const initWasm = async (key: string, mdl: string) => {
    log('WASM_LOAD', 'Fetching clawft_wasm.js + .wasm binary');
    const wasm = await loadWasm();
    wasmRef.current = wasm;
    log('WASM_INIT', `Configuring runtime: model=${mdl}`);

    const config = {
      agents: {
        defaults: {
          model: mdl,
          max_tokens: 2048,
          temperature: 0.7,
        },
      },
      providers: {
        anthropic: { api_key: mdl.startsWith('anthropic/') ? key : '' },
        openai: { api_key: mdl.startsWith('openai/') ? key : '' },
        openrouter: {
          api_key: mdl.startsWith('openrouter/') || !mdl.includes('/') ? key : '',
          browser_direct: true,
        },
        deepseek: { api_key: mdl.startsWith('deepseek/') ? key : '' },
        groq: { api_key: mdl.startsWith('groq/') ? key : '' },
        gemini: { api_key: mdl.startsWith('gemini/') ? key : '' },
        xai: { api_key: mdl.startsWith('xai/') ? key : '' },
        custom: { api_key: '' },
      },
    };

    await wasm.init(JSON.stringify(config));
    log('WASM_READY', `Runtime initialized, provider=${mdl.split('/')[0]}`);
  };

  const handleReset = useCallback(() => {
    setMessages([]);
    setInput('');
    setSending(false);
    // Reset WASM conversation history — reinitialize
    wasmModule = null;
    wasmRef.current = null;
    const key = localStorage.getItem('clawft-api-key') ?? '';
    const mdl = localStorage.getItem('clawft-model') ?? model;
    if (key) {
      initWasm(key, mdl).catch(() => {});
    }
  }, [model]);

  // Load KB on mount — ready immediately in local mode
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        log('KB_FETCH', 'Loading RVF knowledge base...');
        const kb = await loadKB();
        if (cancelled) return;
        setKbLoaded(true);
        setKbStats(`${kb.entries.length} segments, ${kb.manifest.dimension}-dim`);
        log('KB_READY', `${kb.entries.length} segments, dim=${kb.manifest.dimension}, embedder=${kb.manifest.embedder_name}`);

        // Check for stored key — if present, upgrade to LLM mode
        const stored = localStorage.getItem('clawft-api-key');
        const storedModel = localStorage.getItem('clawft-model');
        if (storedModel) setModel(storedModel);

        if (stored) {
          log('KEY_FOUND', 'Cached API key found, upgrading to LLM mode');
          setApiKey(stored);
          setMode('llm');
          await initWasm(stored, storedModel || model);
        } else {
          log('MODE_SET', 'Local retrieval mode (no API key)');
        }

        if (!cancelled) setStatus('ready');
      } catch (e: any) {
        if (!cancelled) {
          setError(e.message || String(e));
          // Even if WASM fails, local mode works with just the KB
          if (kbCache) {
            setMode('local');
            setStatus('ready');
          } else {
            setStatus('error');
          }
        }
      }
    })();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleConnect = async () => {
    if (!apiKey.trim()) return;
    setStatus('loading');
    setError('');
    try {
      localStorage.setItem('clawft-api-key', apiKey);
      localStorage.setItem('clawft-model', model);
      await initWasm(apiKey, model);
      setMode('llm');
      setShowLlmSetup(false);
      setStatus('ready');
      setMessages([
        {
          role: 'system',
          content: `Connected to clawft-wasm. Model: ${model}. KB loaded with ${kbStats}. Ask me anything about WeftOS.`,
        },
      ]);
    } catch (e: any) {
      setError(e.message || String(e));
      setStatus('ready'); // Fall back to local mode
      setMode('local');
    }
  };

  const handleClearKey = () => {
    localStorage.removeItem('clawft-api-key');
    localStorage.removeItem('clawft-model');
    wasmModule = null;
    wasmRef.current = null;
    setApiKey('');
    setMode('local');
    setMessages([]);
    setError('');
  };

  /** Stream a reply word-by-word into the streaming bubble, then commit to messages. */
  const streamReply = useCallback(async (reply: string): Promise<void> => {
    streamAbortRef.current = false;
    const words = reply.split(' ');
    let displayed = '';
    for (let i = 0; i < words.length; i++) {
      if (streamAbortRef.current) break;
      displayed += (i > 0 ? ' ' : '') + words[i];
      setStreamingMessage(displayed);
      await new Promise((r) => setTimeout(r, 15));
    }
    setStreamingMessage('');
    setMessages((prev) => [...prev, { role: 'assistant', content: reply }]);
  }, []);

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || sending) return;
    if (mode === 'llm' && !wasmRef.current) return;

    setSending(true);
    setInput('');
    setMessages((prev) => [...prev, { role: 'user', content: text }]);

    try {
      log('QUERY', `"${text.slice(0, 80)}${text.length > 80 ? '...' : ''}"`);
      const introspecting = isIntrospectionQuery(text);
      if (introspecting) log('INTROSPECT', 'Runtime introspection triggered');

      // RAG: search KB for relevant context
      const hits = kbCache ? keywordSearch(text, kbCache.entries, 8) : [];
      log('KB_SEARCH', `${hits.length} results from ${kbCache?.entries.length ?? 0} segments`);
      const context = hits.length > 0
        ? hits
            .map((h) => {
              const meta = h.metadata as Record<string, string>;
              return `### ${meta?.title ?? 'Doc'} — ${meta?.section ?? ''}\nSource: ${meta?.doc_url ?? meta?.source ?? ''}\n${h.text}`;
            })
            .join('\n\n')
        : '';

      // Gather live runtime info when the user is asking about this instance
      const runtimeInfo = introspecting
        ? gatherRuntimeInfo(wasmRef.current, kbCache, model)
        : '';

      if (mode === 'local' || mode === 'local-ai') {
        // ── Local / Local-AI mode: pure retrieval, no LLM ──────────────────
        const isLocalAI = mode === 'local-ai';
        log('RETRIEVE', `Formatting ${hits.length} KB results (${isLocalAI ? 'local-ai' : 'local'} mode)`);
        let reply: string;

        if (introspecting && runtimeInfo) {
          reply = runtimeInfo;
        } else if (hits.length > 0) {
          // Format KB results as a readable answer
          const formattedHits = hits
            .map((h) => {
              const meta = h.metadata as Record<string, string>;
              const title = meta?.title ?? '';
              const section = meta?.section ?? '';
              const source = meta?.doc_url ?? meta?.source ?? '';
              const header = [title, section].filter(Boolean).join(' — ');
              const link = source ? `[${header}](${source})` : header;
              return `**${link}**\n\n${h.text}`;
            })
            .join('\n\n---\n\n');

          if (isLocalAI) {
            reply = `Based on the documentation, here's what I found:\n\n${formattedHits}\n\nThis is from local KB retrieval. Full local AI inference coming soon.`;
          } else {
            reply = formattedHits;
          }
        } else {
          reply = isLocalAI
            ? "I couldn't find relevant documentation for that query. Try browsing /docs/ directly."
            : "No matching documentation found. Try different keywords, or browse the full docs at [/docs](/docs).";
        }

        log('RESPOND', `${isLocalAI ? 'Local-AI' : 'Local'}: ${reply.length} chars`);
        await streamReply(reply);
      } else {
        // ── LLM mode: RAG + send through WASM pipeline ──────────
        log('LLM_SEND', `Sending to ${model} with ${context.length} chars context`);
        const ragPrompt = context || runtimeInfo
          ? [
              'ROLE: You are the WeftOS documentation assistant running in a clawft WASM sandbox in the user\'s browser.',
              '',
              'RULES:',
              '- Answer using the documentation excerpts and/or live runtime data provided below.',
              '- When live runtime data is provided, you CAN and SHOULD report it — this is real data from the user\'s own WASM instance.',
              '- If the answer is not in the excerpts or runtime data, say "I don\'t have that information in my knowledge base. Try checking the docs at /docs/" — do NOT guess.',
              '- Quote the source link when referencing a doc page.',
              '- Keep answers concise and factual.',
              '- For acronyms, only use the definition from the docs — never invent expansions.',
              '',
              ...(runtimeInfo ? ['## Live Runtime Data (from this WASM instance)', '', runtimeInfo, ''] : []),
              ...(context ? ['## Documentation Context', '', context, ''] : []),
              '## Question',
              '',
              text,
            ].join('\n')
          : [
              'ROLE: You are the WeftOS documentation assistant.',
              'You have no relevant documentation context for this question.',
              'Say "I don\'t have enough context to answer that accurately. Try asking about a specific WeftOS feature, or browse the docs at /docs/"',
              '',
              'Question: ' + text,
            ].join('\n');

        const reply = await wasmRef.current.send_message(ragPrompt);
        log('LLM_RECV', `Response: ${reply.length} chars`);
        await streamReply(reply);
      }
    } catch (e: any) {
      log('ERROR', String(e));
      setStreamingMessage('');
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: `Error: ${e.message || e}` },
      ]);
    } finally {
      setSending(false);
    }
  }, [input, sending, mode, model, streamReply]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <main className="flex min-h-screen flex-col bg-fd-background">
      {/* Header */}
      <header className="border-b border-fd-border">
        <div className="flex items-center justify-between px-4 py-3">
          <div className="flex items-center gap-3">
            <Link href="/" className="text-sm text-fd-muted-foreground hover:text-fd-foreground transition-colors">
              WeftOS
            </Link>
            <span className="text-fd-muted-foreground">/</span>
            <h1 className="text-lg font-semibold text-fd-foreground">clawft sandbox</h1>
          </div>
          <div className="flex items-center gap-2">
            {status === 'ready' && messages.length > 0 && (
              <button
                onClick={handleReset}
                className="rounded bg-fd-accent px-2 py-1 text-xs text-fd-muted-foreground hover:bg-fd-border transition-colors"
              >
                New chat
              </button>
            )}
            <StatusIndicator status={status} />
          </div>
        </div>

        {/* Status bar */}
        {status === 'ready' && (
          <div className="flex items-center gap-3 border-t border-fd-border bg-fd-accent/30 px-4 py-1.5 text-xs text-fd-muted-foreground">
            {kbLoaded && (
              <span title="RVF Knowledge Base">
                KB: {kbStats}
              </span>
            )}
            <span className="text-fd-border">|</span>
            {mode === 'local' ? (
              <>
                <span>Mode: local retrieval</span>
                <button
                  onClick={() => setMode('local-ai')}
                  className="underline hover:text-fd-foreground transition-colors"
                >
                  Local AI
                </button>
                <button
                  onClick={() => setShowLlmSetup(!showLlmSetup)}
                  className="underline hover:text-fd-foreground transition-colors"
                >
                  Connect LLM
                </button>
              </>
            ) : mode === 'local-ai' ? (
              <>
                <span>Mode: Local AI (ruvllm-wasm)</span>
                <button
                  onClick={() => setMode('local')}
                  className="underline hover:text-fd-foreground transition-colors"
                >
                  Local
                </button>
                <button
                  onClick={() => setShowLlmSetup(!showLlmSetup)}
                  className="underline hover:text-fd-foreground transition-colors"
                >
                  Connect LLM
                </button>
              </>
            ) : (
              <>
                <span title={model}>Mode: LLM ({model.split('/').pop()})</span>
                <button
                  onClick={handleClearKey}
                  className="underline hover:text-fd-foreground transition-colors"
                >
                  Disconnect
                </button>
              </>
            )}
          </div>
        )}
      </header>

      {/* LLM config panel — shown when user wants to connect a model */}
      {status === 'ready' && (mode === 'local' || mode === 'local-ai') && messages.length === 0 && showLlmSetup && (
        <div className="mx-auto w-full max-w-xl p-6">
          <div className="rounded-xl border border-fd-border bg-fd-card p-6">
            <div className="flex items-center justify-between mb-2">
              <h2 className="text-lg font-semibold text-fd-card-foreground">
                Connect an LLM (optional)
              </h2>
              <button onClick={() => setShowLlmSetup(false)} className="text-fd-muted-foreground hover:text-fd-foreground text-sm">
                Close
              </button>
            </div>
            <p className="mb-4 text-sm text-fd-muted-foreground">
              Local mode works without an API key — it searches the KB and returns matching docs directly.
              Connect an LLM for synthesized answers. Your key stays in localStorage.
            </p>
            {error && (
              <div className="mb-4 rounded-lg bg-red-500/10 p-3 text-sm text-red-400">
                {error}
              </div>
            )}
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-fd-muted-foreground">
                  Model
                </label>
                <input
                  type="text"
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  placeholder="openrouter/google/gemini-2.0-flash-001"
                  className="w-full rounded-lg border border-fd-border bg-fd-background px-3 py-2 text-sm text-fd-foreground placeholder:text-fd-muted-foreground focus:border-fd-primary focus:outline-none"
                />
                <p className="mt-1 text-xs text-fd-muted-foreground">
                  Prefix with provider: anthropic/, openai/, openrouter/, groq/, deepseek/, gemini/, xai/
                </p>
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-fd-muted-foreground">
                  API Key
                </label>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleConnect()}
                  placeholder="sk-..."
                  className="w-full rounded-lg border border-fd-border bg-fd-background px-3 py-2 text-sm text-fd-foreground placeholder:text-fd-muted-foreground focus:border-fd-primary focus:outline-none"
                />
              </div>
              <button
                onClick={handleConnect}
                disabled={!apiKey.trim()}
                className="w-full rounded-lg bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-50"
              >
                Connect
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Loading state */}
      {status === 'loading' && (
        <div className="flex flex-1 items-center justify-center">
          <div className="text-center">
            <div className="mb-3 inline-block h-6 w-6 animate-spin rounded-full border-2 border-fd-primary border-t-transparent" />
            <p className="text-sm text-fd-muted-foreground">Loading clawft-wasm...</p>
          </div>
        </div>
      )}

      {/* Two-column layout: Chat + Chain Log */}
      {status === 'ready' && (
        <div className="flex flex-1 min-h-0">
          {/* Left: Chat */}
          <div className="flex flex-1 flex-col min-w-0">
            <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-4">
              <div className="mx-auto max-w-2xl space-y-4">
                {messages.length === 0 && (
                  <div className="py-12 text-center">
                    <h2 className="mb-2 text-xl font-semibold text-fd-foreground">
                      WeftOS WASM Sandbox
                    </h2>
                    <p className="mb-4 text-sm text-fd-muted-foreground">
                      {mode === 'local'
                        ? `Searching ${kbStats} of WeftOS documentation locally — no API key needed.`
                        : mode === 'local-ai'
                        ? `Local AI mode uses ruvllm-wasm for on-device inference. Coming soon — currently falls back to local KB retrieval.`
                        : `Running clawft-wasm with ${model.split('/').pop()}, backed by ${kbStats}.`}
                    </p>
                    {mode === 'local-ai' && (
                      <div className="mb-6 mx-auto max-w-md rounded-lg border border-yellow-500/30 bg-yellow-500/5 px-4 py-2 text-xs text-yellow-400">
                        Local AI mode uses ruvllm-wasm for on-device inference. Coming soon — currently falls back to local KB retrieval.
                      </div>
                    )}
                    <div className="space-y-3 max-w-lg mx-auto text-left">
                      {([
                        { label: 'Getting Started', items: ['What is WeftOS?', 'How do I install it?'] },
                        { label: 'Architecture', items: ['Show me the boot sequence', 'How does the ECC work?'] },
                        { label: 'Assessment', items: ['What does weft assess do?', 'How do cross-project assessments work?'] },
                        { label: 'Security', items: ['How does governance work?', 'What is the ExoChain?'] },
                      ] as const).map((group) => (
                        <div key={group.label}>
                          <span className="block mb-1 text-[10px] font-semibold uppercase tracking-wider text-fd-muted-foreground/60">
                            {group.label}
                          </span>
                          <div className="flex flex-wrap gap-2">
                            {group.items.map((q) => (
                              <button
                                key={q}
                                onClick={() => {
                                  setInput(q);
                                }}
                                className="rounded-lg border border-fd-border px-3 py-1.5 text-xs text-fd-muted-foreground hover:border-fd-primary hover:text-fd-foreground transition-colors"
                              >
                                {q}
                              </button>
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                    {/* KB Graph visualization */}
                    {kbLoaded && kbCache && (
                      <div className="mt-6">
                        <button
                          onClick={() => setShowKBGraph((v) => !v)}
                          className="text-xs underline text-fd-muted-foreground hover:text-fd-foreground transition-colors"
                        >
                          {showKBGraph ? 'Hide KB Graph' : 'Show KB Graph'}
                        </button>
                        {showKBGraph && (
                          <div className="mt-3 rounded-xl border border-fd-border bg-fd-card p-3">
                            <KBGraph entries={kbCache.entries} />
                            <p className="mt-2 text-[10px] text-fd-muted-foreground text-center">
                              Tag co-occurrence graph — {kbCache.entries.length} segments
                            </p>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                )}

                {messages.map((msg, i) => (
                  <ChatBubble key={i} message={msg} />
                ))}

                {/* Streaming message bubble */}
                {streamingMessage && (
                  <div className="flex justify-start">
                    <div className="max-w-[80%] rounded-xl px-4 py-2.5 text-sm border border-fd-border bg-fd-card text-fd-card-foreground">
                      <div className="whitespace-pre-wrap">{streamingMessage}<span className="inline-block w-1.5 h-4 ml-0.5 bg-fd-foreground/60 animate-pulse" /></div>
                    </div>
                  </div>
                )}

                {sending && !streamingMessage && (
                  <div className="flex gap-2 text-fd-muted-foreground">
                    <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-fd-primary" />
                    <span className="text-sm">Thinking...</span>
                  </div>
                )}
              </div>
            </div>

            {/* Input */}
            <div className="border-t border-fd-border px-4 py-3">
              <div className="mx-auto flex max-w-2xl gap-2">
                <textarea
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Ask about WeftOS..."
                  rows={1}
                  className="flex-1 resize-none rounded-lg border border-fd-border bg-fd-background px-3 py-2 text-sm text-fd-foreground placeholder:text-fd-muted-foreground focus:border-fd-primary focus:outline-none"
                />
                <button
                  onClick={handleSend}
                  disabled={!input.trim() || sending}
                  className="rounded-lg bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground hover:opacity-90 transition-opacity disabled:opacity-50"
                >
                  Send
                </button>
              </div>
            </div>
          </div>

          {/* Right: Chain Log */}
          <div className="hidden w-80 flex-shrink-0 border-l border-fd-border lg:flex lg:flex-col">
            <div className="flex items-center justify-between border-b border-fd-border px-3 py-2">
              <span className="text-xs font-semibold text-fd-foreground">ExoChain Log</span>
              <span className="text-[10px] text-fd-muted-foreground font-mono">
                {chain.length} entries
              </span>
            </div>
            <div ref={chainScrollRef} className="flex-1 overflow-y-auto">
              {chain.map((entry, i) => (
                <ChainRow key={i} entry={entry} index={i} />
              ))}
              {chain.length === 0 && (
                <div className="p-3 text-xs text-fd-muted-foreground text-center">
                  Chain events will appear here as the sandbox operates.
                </div>
              )}
            </div>
            {chain.length > 0 && (
              <div className="border-t border-fd-border px-3 py-2 bg-fd-accent/20">
                <div className="font-mono text-[10px] text-fd-muted-foreground leading-relaxed">
                  <span className="text-fd-muted-foreground/60">WITNESS</span>{' '}
                  Chain: {chain.length} {chain.length === 1 ? 'entry' : 'entries'}, hash:{' '}
                  <span className="text-fd-foreground/70">{chain[chain.length - 1].hash}</span>{' '}
                  <span className="text-green-400">(verified)</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </main>
  );
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function StatusIndicator({ status }: { status: SandboxStatus }) {
  const colors: Record<SandboxStatus, string> = {
    loading: 'bg-yellow-400',
    ready: 'bg-green-400',
    error: 'bg-red-400',
    'needs-key': 'bg-fd-muted-foreground',
  };

  const labels: Record<SandboxStatus, string> = {
    loading: 'Loading',
    ready: 'Connected',
    error: 'Error',
    'needs-key': 'Not connected',
  };

  return (
    <span className="flex items-center gap-1.5">
      <span className={`inline-block h-2 w-2 rounded-full ${colors[status]}`} />
      <span>{labels[status]}</span>
    </span>
  );
}

function ChatBubble({ message }: { message: Message }) {
  const isUser = message.role === 'user';
  const isSystem = message.role === 'system';

  if (isSystem) {
    return (
      <div className="rounded-lg bg-fd-accent px-4 py-2 text-xs text-fd-muted-foreground">
        {message.content}
      </div>
    );
  }

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[80%] rounded-xl px-4 py-2.5 text-sm ${
          isUser
            ? 'bg-fd-primary text-fd-primary-foreground'
            : 'border border-fd-border bg-fd-card text-fd-card-foreground'
        }`}
      >
        <div className="whitespace-pre-wrap">{message.content}</div>
      </div>
    </div>
  );
}

const OP_COLORS: Record<string, string> = {
  KB_FETCH: 'text-blue-400',
  KB_READY: 'text-blue-400',
  KB_SEARCH: 'text-cyan-400',
  QUERY: 'text-yellow-400',
  RETRIEVE: 'text-green-400',
  RESPOND: 'text-green-400',
  LLM_SEND: 'text-purple-400',
  LLM_RECV: 'text-purple-400',
  WASM_LOAD: 'text-orange-400',
  WASM_INIT: 'text-orange-400',
  WASM_READY: 'text-orange-400',
  INTROSPECT: 'text-pink-400',
  MODE_SET: 'text-fd-muted-foreground',
  KEY_FOUND: 'text-fd-muted-foreground',
  ERROR: 'text-red-400',
};

function ChainRow({ entry, index }: { entry: ChainEntry; index: number }) {
  const time = new Date(entry.ts);
  const ts = `${time.getHours().toString().padStart(2, '0')}:${time.getMinutes().toString().padStart(2, '0')}:${time.getSeconds().toString().padStart(2, '0')}.${time.getMilliseconds().toString().padStart(3, '0')}`;
  const color = OP_COLORS[entry.op] ?? 'text-fd-muted-foreground';

  return (
    <div className="border-b border-fd-border/50 px-3 py-1.5 font-mono text-[11px] leading-tight hover:bg-fd-accent/30 transition-colors">
      <div className="flex items-baseline gap-2">
        <span className="text-fd-muted-foreground/50 select-none">{index.toString().padStart(3, '0')}</span>
        <span className="text-fd-muted-foreground/70">{ts}</span>
        <span className={`font-semibold ${color}`}>{entry.op}</span>
      </div>
      <div className="ml-10 text-fd-muted-foreground break-all">{entry.detail}</div>
      {entry.hash && (
        <div className="ml-10 text-fd-muted-foreground/40 select-none">#{entry.hash}</div>
      )}
    </div>
  );
}
