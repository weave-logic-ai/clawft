'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import type { KBEntry, KBManifest } from '@/lib/rvf-reader';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

type SandboxStatus = 'loading' | 'ready' | 'error' | 'needs-key';

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
// Component
// ---------------------------------------------------------------------------

export default function WasmSandbox() {
  const [status, setStatus] = useState<SandboxStatus>('loading');
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('openrouter/google/gemini-2.0-flash-001');
  const [kbLoaded, setKbLoaded] = useState(false);
  const [kbStats, setKbStats] = useState<string>('');
  const [error, setError] = useState<string>('');
  const scrollRef = useRef<HTMLDivElement>(null);
  const wasmRef = useRef<any>(null);

  // Scroll to bottom on new messages
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [messages]);

  const initWasm = async (key: string, mdl: string) => {
    const wasm = await loadWasm();
    wasmRef.current = wasm;

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

  // Load WASM + KB on mount
  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        // Load KB in background
        const kb = await loadKB();
        if (cancelled) return;
        setKbLoaded(true);
        setKbStats(`${kb.entries.length} segments, ${kb.manifest.dimension}-dim`);

        // Check for stored key
        const stored = localStorage.getItem('clawft-api-key');
        const storedModel = localStorage.getItem('clawft-model');
        if (storedModel) setModel(storedModel);

        if (stored) {
          setApiKey(stored);
          await initWasm(stored, storedModel || model);
          if (!cancelled) setStatus('ready');
        } else {
          setStatus('needs-key');
        }
      } catch (e: any) {
        if (!cancelled) {
          setError(e.message || String(e));
          setStatus('error');
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
      setStatus('ready');
      setMessages([
        {
          role: 'system',
          content: `Connected to clawft-wasm. Model: ${model}. KB loaded with ${kbStats}. Ask me anything about WeftOS.`,
        },
      ]);
    } catch (e: any) {
      setError(e.message || String(e));
      setStatus('error');
    }
  };

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || sending || !wasmRef.current) return;

    setSending(true);
    setInput('');
    setMessages((prev) => [...prev, { role: 'user', content: text }]);

    try {
      const introspecting = isIntrospectionQuery(text);

      // RAG: search KB for relevant context
      let context = '';
      if (kbCache) {
        const hits = keywordSearch(text, kbCache.entries, 8);
        if (hits.length > 0) {
          context = hits
            .map((h) => {
              const meta = h.metadata as Record<string, string>;
              return `### ${meta?.title ?? 'Doc'} — ${meta?.section ?? ''}\nSource: ${meta?.doc_url ?? meta?.source ?? ''}\n${h.text}`;
            })
            .join('\n\n');
        }
      }

      // Gather live runtime info when the user is asking about this instance
      const runtimeInfo = introspecting
        ? gatherRuntimeInfo(wasmRef.current, kbCache, model)
        : '';

      // Build the prompt with strong RAG constraints
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
      setMessages((prev) => [...prev, { role: 'assistant', content: reply }]);
    } catch (e: any) {
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: `Error: ${e.message || e}` },
      ]);
    } finally {
      setSending(false);
    }
  }, [input, sending]);

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
      <header className="flex items-center justify-between border-b border-fd-border px-4 py-3">
        <div className="flex items-center gap-3">
          <Link href="/" className="text-sm text-fd-muted-foreground hover:text-fd-foreground transition-colors">
            WeftOS
          </Link>
          <span className="text-fd-muted-foreground">/</span>
          <h1 className="text-lg font-semibold text-fd-foreground">clawft sandbox</h1>
          <span className="rounded-full bg-fd-accent px-2 py-0.5 text-xs text-fd-muted-foreground">
            wasm
          </span>
        </div>
        <div className="flex items-center gap-2 text-xs text-fd-muted-foreground">
          {kbLoaded && (
            <span className="rounded bg-fd-accent px-2 py-0.5" title="RVF Knowledge Base loaded">
              KB: {kbStats}
            </span>
          )}
          {status === 'ready' && messages.length > 0 && (
            <button
              onClick={handleReset}
              className="rounded bg-fd-accent px-2 py-0.5 hover:bg-fd-border transition-colors"
              title="New conversation"
            >
              New chat
            </button>
          )}
          <StatusIndicator status={status} />
        </div>
      </header>

      {/* API Key setup */}
      {(status === 'needs-key' || status === 'error') && (
        <div className="mx-auto w-full max-w-xl p-6">
          <div className="rounded-xl border border-fd-border bg-fd-card p-6">
            <h2 className="mb-2 text-lg font-semibold text-fd-card-foreground">
              Connect to an LLM
            </h2>
            <p className="mb-4 text-sm text-fd-muted-foreground">
              clawft-wasm runs entirely in your browser. Provide an API key to connect to an LLM
              provider. Your key stays in localStorage and is sent directly to the provider.
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

      {/* Chat area */}
      {status === 'ready' && (
        <>
          <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-4">
            <div className="mx-auto max-w-2xl space-y-4">
              {messages.length === 0 && (
                <div className="py-12 text-center">
                  <h2 className="mb-2 text-xl font-semibold text-fd-foreground">
                    WeftOS WASM Sandbox
                  </h2>
                  <p className="mb-6 text-sm text-fd-muted-foreground">
                    You're running clawft in your browser via WebAssembly, connected to the full
                    WeftOS documentation KB ({kbStats}). Ask anything.
                  </p>
                  <div className="flex flex-wrap justify-center gap-2">
                    {[
                      'What is WeftOS?',
                      'How does the ECC work?',
                      'Show me the boot sequence',
                      'What LLM providers are supported?',
                    ].map((q) => (
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
              )}

              {messages.map((msg, i) => (
                <ChatBubble key={i} message={msg} />
              ))}

              {sending && (
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
        </>
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
