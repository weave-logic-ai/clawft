'use client';

import { useState, useMemo } from 'react';

/**
 * Live Iteration 0 EML-Attention demonstrator.
 *
 * This is a pure-JS mirror of the Rust ToyEmlAttention forward pass that
 * runs entirely in the browser. It is NOT the real WASM build — that lands
 * in 0.6.9. This page lets you explore the architecture (param counts,
 * forward-pass latency, softmax output) without downloading anything.
 */

// -- Pure-JS EML primitives -------------------------------------------------

function emlSafe(x: number, y: number): number {
  const ex = Math.exp(Math.max(-20, Math.min(20, x)));
  const ly = Math.log(Math.max(Number.MIN_VALUE, y));
  return ex - ly;
}

function numericalSoftmax(row: number[]): number[] {
  const m = Math.max(...row);
  const e = row.map((v) => Math.exp(v - m));
  const s = e.reduce((a, b) => a + b, 0);
  return s > 0 ? e.map((v) => v / s) : row.map(() => 1 / row.length);
}

// -- Deterministic LCG for reproducible random ------------------------------

// 32-bit Numerical Recipes LCG (no BigInt — targets ES2019+ cleanly).
function makeRng(seed: number) {
  let s = seed >>> 0;
  return (): number => {
    s = (Math.imul(s, 1664525) + 1013904223) | 0;
    return ((s >>> 0) / 0x100000000) * 2 - 1;
  };
}

// -- Iteration 0 attention (forward only) -----------------------------------

type EmlModel = {
  depth: number;
  inputs: number;
  heads: number;
  w: Float64Array; // level-0 affine
  b: Float64Array;
  tree: Float64Array; // depth * 3 per head
};

function makeModel(depth: number, inputs: number, heads: number, rng: () => number): EmlModel {
  const w = new Float64Array(heads * inputs);
  const b = new Float64Array(heads);
  const tree = new Float64Array(heads * depth * 3);
  for (let i = 0; i < w.length; i++) w[i] = rng() * 0.1;
  for (let i = 0; i < tree.length; i++) tree[i] = rng() * 0.05;
  return { depth, inputs, heads, w, b, tree };
}

function predict(m: EmlModel, x: number[]): number[] {
  const out = new Float64Array(m.heads);
  for (let h = 0; h < m.heads; h++) {
    let acc = m.b[h];
    for (let i = 0; i < m.inputs; i++) acc += m.w[h * m.inputs + i] * x[i];
    let v = acc;
    for (let d = 0; d < m.depth; d++) {
      const c0 = m.tree[h * m.depth * 3 + d * 3];
      const c1 = m.tree[h * m.depth * 3 + d * 3 + 1];
      const c2 = m.tree[h * m.depth * 3 + d * 3 + 2];
      v = emlSafe(v * c0 + c1, Math.abs(v) + c2 + 1);
    }
    out[h] = v;
  }
  return Array.from(out);
}

function paramCount(m: EmlModel): number {
  return m.w.length + m.b.length + m.tree.length;
}

type Attn = {
  seqLen: number;
  dModel: number;
  dK: number;
  depth: number;
  q: EmlModel;
  k: EmlModel;
  v: EmlModel;
  sm: EmlModel;
  out: EmlModel;
  scale: number;
};

function makeAttention(dModel: number, dK: number, seqLen: number, depth: number, seed: number): Attn {
  const rng = makeRng(seed);
  const projIn = seqLen * dModel;
  const projOut = seqLen * dK;
  return {
    seqLen,
    dModel,
    dK,
    depth,
    q: makeModel(depth, projIn, projOut, rng),
    k: makeModel(depth, projIn, projOut, rng),
    v: makeModel(depth, projIn, projOut, rng),
    sm: makeModel(Math.min(depth, 4), seqLen, seqLen, rng),
    out: makeModel(depth, projOut, projIn, rng),
    scale: 1 / Math.sqrt(dK),
  };
}

function attnParamCount(a: Attn): number {
  return paramCount(a.q) + paramCount(a.k) + paramCount(a.v) + paramCount(a.sm) + paramCount(a.out);
}

function forward(a: Attn, x: number[]): { y: number[]; attnMatrix: number[][] } {
  const q = predict(a.q, x);
  const k = predict(a.k, x);
  const v = predict(a.v, x);
  const { seqLen: n, dK: d, scale } = a;
  const scores: number[][] = [];
  for (let i = 0; i < n; i++) {
    const row: number[] = [];
    for (let j = 0; j < n; j++) {
      let s = 0;
      for (let r = 0; r < d; r++) s += q[i * d + r] * k[j * d + r];
      row.push(s * scale);
    }
    scores.push(row);
  }
  const attnMatrix = scores.map(numericalSoftmax);
  const ctx: number[] = new Array(n * d).fill(0);
  for (let i = 0; i < n; i++) {
    for (let r = 0; r < d; r++) {
      let s = 0;
      for (let j = 0; j < n; j++) s += attnMatrix[i][j] * v[j * d + r];
      ctx[i * d + r] = s;
    }
  }
  const y = predict(a.out, ctx);
  return { y, attnMatrix };
}

// -- Component --------------------------------------------------------------

export default function Page() {
  const [dModel, setDModel] = useState(8);
  const [dK, setDK] = useState(4);
  const [seqLen, setSeqLen] = useState(4);
  const [depth, setDepth] = useState(3);
  const [seed, setSeed] = useState(42);

  const attn = useMemo(
    () => makeAttention(dModel, dK, seqLen, depth, seed),
    [dModel, dK, seqLen, depth, seed],
  );

  const params = useMemo(() => attnParamCount(attn), [attn]);

  const demo = useMemo(() => {
    const rng = makeRng(seed + 1);
    const x: number[] = [];
    for (let i = 0; i < seqLen * dModel; i++) x.push(rng());
    const n = 64;
    const timings: number[] = [];
    let last: { y: number[]; attnMatrix: number[][] } | null = null;
    for (let i = 0; i < n; i++) {
      const t = performance.now();
      last = forward(attn, x);
      timings.push((performance.now() - t) * 1000); // µs
    }
    timings.sort((a, b) => a - b);
    const mean = timings.reduce((a, b) => a + b, 0) / n;
    const p99 = timings[Math.floor(n * 0.99)];
    return { x, y: last!.y, attn: last!.attnMatrix, mean, p99 };
  }, [attn, seqLen, dModel, seed]);

  return (
    <main className="mx-auto my-10 max-w-4xl px-6 leading-relaxed text-neutral-900 dark:text-neutral-100">
      <h1 className="mb-4 text-3xl font-semibold">Toy EML-Attention — Iteration 0 (pure-JS demo)</h1>
      <p className="mb-4">
        Live forward-pass demonstrator for the Iteration 0 EML-Attention block. This runs a
        pure-JS mirror of the Rust <code>ToyEmlAttention</code> entirely in the browser — the real
        WASM build lands in 0.6.9. See the <a href="/docs/weftos/eml-attention" className="underline">architecture page</a>
        {' '}for details and the <a href="/clawft_eml-notebook" className="underline">Colab notebook</a> for a trainable
        Python reference that exports JSON directly loadable by the Rust <code>EmlModel</code>.
      </p>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Configuration</h2>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4">
        <label className="flex flex-col text-sm">
          <span className="font-mono">d_model: {dModel}</span>
          <input type="range" min={4} max={32} step={4} value={dModel} onChange={(e) => {
            const v = Number(e.target.value);
            setDModel(v);
            if (dK > v) setDK(v);
          }} className="mt-1 w-full" />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">d_k: {dK}</span>
          <input type="range" min={2} max={dModel} step={2} value={dK} onChange={(e) => setDK(Number(e.target.value))} className="mt-1 w-full" />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">seq_len: {seqLen}</span>
          <input type="range" min={2} max={8} step={1} value={seqLen} onChange={(e) => setSeqLen(Number(e.target.value))} className="mt-1 w-full" />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">depth: {depth}</span>
          <input type="range" min={3} max={5} step={1} value={depth} onChange={(e) => setDepth(Number(e.target.value))} className="mt-1 w-full" />
        </label>
        <label className="flex flex-col text-sm sm:col-span-2">
          <span className="font-mono">seed: {seed}</span>
          <input type="number" value={seed} onChange={(e) => setSeed(Number(e.target.value))} className="mt-1 rounded border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 px-2 py-1 text-sm" />
        </label>
      </div>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Architecture summary</h2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-sm">
          <thead>
            <tr className="bg-neutral-100 dark:bg-neutral-800">
              <th className={thCls}>Submodel</th>
              <th className={thCls}>Shape</th>
              <th className={thCls}>Params</th>
            </tr>
          </thead>
          <tbody>
            <tr><td className={tdCls}>q_model</td><td className={tdCls}>({seqLen}·{dModel}) → ({seqLen}·{dK})</td><td className={tdCls}>{paramCount(attn.q)}</td></tr>
            <tr><td className={tdCls}>k_model</td><td className={tdCls}>({seqLen}·{dModel}) → ({seqLen}·{dK})</td><td className={tdCls}>{paramCount(attn.k)}</td></tr>
            <tr><td className={tdCls}>v_model</td><td className={tdCls}>({seqLen}·{dModel}) → ({seqLen}·{dK})</td><td className={tdCls}>{paramCount(attn.v)}</td></tr>
            <tr><td className={tdCls}>softmax_model</td><td className={tdCls}>{seqLen} → {seqLen}</td><td className={tdCls}>{paramCount(attn.sm)}</td></tr>
            <tr><td className={tdCls}>out_model</td><td className={tdCls}>({seqLen}·{dK}) → ({seqLen}·{dModel})</td><td className={tdCls}>{paramCount(attn.out)}</td></tr>
            <tr className="bg-neutral-50 dark:bg-neutral-900">
              <td className={`${tdCls} font-semibold`}>total</td>
              <td className={tdCls}>—</td>
              <td className={`${tdCls} font-semibold`}>{params}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Forward-pass performance (pure JS, 64 iterations)</h2>
      <p>
        mean: <strong>{demo.mean.toFixed(2)} µs</strong> &nbsp;·&nbsp;
        p99: <strong>{demo.p99.toFixed(2)} µs</strong>
      </p>
      <p className="text-sm text-neutral-600 dark:text-neutral-400">
        Rust p99 target: ≤ 5 µs at (seq_len=4, d_model=8, d_k=4, depth=3). JS is slower than the
        Rust build because of boxed-number overhead and the lack of SIMD — treat these numbers as
        an upper bound.
      </p>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Attention matrix (softmax output)</h2>
      <p className="text-sm text-neutral-600 dark:text-neutral-400">
        Rows sum to 1. Untrained projections → near-uniform attention pattern. After training,
        expect the diagonal or a sparser pattern, depending on the task.
      </p>
      <pre className={preCls}>
{demo.attn.map((row, i) => `row ${i}: [${row.map((v) => v.toFixed(3)).join(', ')}]`).join('\n')}
      </pre>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Output (last forward pass)</h2>
      <p className="text-sm text-neutral-600 dark:text-neutral-400">First 16 dims of the {seqLen}·{dModel}-wide output.</p>
      <pre className={preCls}>
[{demo.y.slice(0, 16).map((v) => v.toFixed(4)).join(', ')}{demo.y.length > 16 ? ', …' : ''}]
      </pre>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Next steps</h2>
      <ul className="list-disc pl-6 space-y-1">
        <li>0.6.9 will swap this JS port for the real Rust/WASM build, same interface.</li>
        <li>Training (gradient-free coordinate descent) runs in Rust via <code>ToyEmlAttention::train</code>; this page demonstrates forward pass only.</li>
        <li>See the <a href="/clawft_eml-notebook" className="underline">Python Colab notebook</a> to train offline and export JSON for the Rust loader.</li>
      </ul>
    </main>
  );
}

const thCls = 'border border-neutral-300 dark:border-neutral-700 px-3 py-2 text-left';
const tdCls = 'border border-neutral-300 dark:border-neutral-700 px-3 py-1.5';
const preCls = 'mt-2 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 overflow-x-auto text-xs leading-relaxed';
