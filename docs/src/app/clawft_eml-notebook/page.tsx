'use client';

import { useMemo, useState } from 'react';
import {
  ToyEmlAttention,
  downloadJson,
  makeRng,
} from './lib';

type TrainState = 'idle' | 'training' | 'done';

function MSESparkline({ values }: { values: number[] }) {
  if (values.length < 2) {
    return (
      <div className="text-xs text-neutral-600 dark:text-neutral-400">
        (sparkline appears once round 2 completes)
      </div>
    );
  }
  const w = 480;
  const h = 80;
  const pad = 8;
  // Log scale so small relative improvements stay visible.
  const logs = values.map((v) => Math.log10(Math.max(v, 1e-12)));
  const lmax = Math.max(...logs);
  const lmin = Math.min(...logs);
  const span = lmax - lmin || 1e-6;
  const pts = logs.map((l, i) => {
    const x = pad + (i / (values.length - 1)) * (w - 2 * pad);
    const y = pad + (1 - (l - lmin) / span) * (h - 2 * pad);
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return (
    <svg
      viewBox={`0 0 ${w} ${h}`}
      className="w-full h-20 rounded border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-950"
      aria-label="MSE over rounds, log scale"
    >
      <polyline
        points={pts.join(' ')}
        fill="none"
        className="stroke-neutral-900 dark:stroke-neutral-100"
        strokeWidth={1.5}
      />
      {pts.map((p, i) => {
        const [x, y] = p.split(',');
        return (
          <circle
            key={i}
            cx={x}
            cy={y}
            r={3}
            className="fill-neutral-900 dark:fill-neutral-100"
          />
        );
      })}
    </svg>
  );
}

export default function Page() {
  const [dModel, setDModel] = useState(8);
  const [dK, setDK] = useState(4);
  const [seqLen, setSeqLen] = useState(4);
  const [depth, setDepth] = useState(3);
  const [seed, setSeed] = useState(42);
  const [rounds, setRounds] = useState(3);
  const [sampleCount, setSampleCount] = useState(64);

  const [state, setState] = useState<TrainState>('idle');
  const [curve, setCurve] = useState<number[]>([]);
  const [finalMse, setFinalMse] = useState<number | null>(null);
  const [attn, setAttn] = useState<ToyEmlAttention | null>(null);
  const [log, setLog] = useState<string[]>([]);

  const paramCount = useMemo(() => {
    const a = new ToyEmlAttention(dModel, dK, seqLen, depth, seed);
    return a.paramCount();
  }, [dModel, dK, seqLen, depth, seed]);

  const train = async () => {
    setState('training');
    setCurve([]);
    setFinalMse(null);
    setLog([]);
    // Defer one tick so the button state repaints before heavy work begins.
    await new Promise((r) => setTimeout(r, 20));

    const append = (line: string) =>
      setLog((prev) => [...prev, line]);

    append(`init ToyEmlAttention(d_model=${dModel}, d_k=${dK}, seq_len=${seqLen}, depth=${depth}, seed=${seed})`);
    const rng = makeRng(seed + 1);
    const a = new ToyEmlAttention(dModel, dK, seqLen, depth, seed);
    append(`param_count = ${a.paramCount()}  (q=${a.q.totalParams()} k=${a.k.totalParams()} v=${a.v.totalParams()} out=${a.out.totalParams()})`);

    const samples: { x: number[]; target: number[] }[] = [];
    const dim = seqLen * dModel;
    for (let s = 0; s < sampleCount; s++) {
      const x: number[] = [];
      for (let i = 0; i < dim; i++) x.push(rng());
      // Target: per-sequence-position mean broadcast across d_model.
      // Low-rank and learnable given Iteration-0's frozen Q/K/V projections.
      const target: number[] = new Array(dim).fill(0);
      for (let i = 0; i < seqLen; i++) {
        let mean = 0;
        for (let j = 0; j < dModel; j++) mean += x[i * dModel + j];
        mean /= dModel;
        for (let j = 0; j < dModel; j++) target[i * dModel + j] = mean;
      }
      samples.push({ x, target });
    }
    append(`generated ${samples.length} (input, target) samples — target = per-position mean broadcast`);

    await new Promise((r) => setTimeout(r, 20));

    const trainingCurve = await a.trainOutModelOnly(samples, rng, {
      rounds,
      onStatus: (msg) => append(msg),
      onStart: ({ samples: n, params, trialsPerRound }) => {
        append(
          `train out_model: ${params} params, ${n} context pairs, 16-sample MSE subset, ${trialsPerRound} trials/round`,
        );
      },
      onRound: (round, mse, elapsedMs) => {
        append(`round ${round}/${rounds}   MSE = ${mse.toExponential(4)}   elapsed = ${elapsedMs.toFixed(0)} ms`);
        setCurve((prev) => [...prev, mse]);
      },
    });

    const final = trainingCurve[trainingCurve.length - 1] ?? null;
    setFinalMse(final);
    setAttn(a);
    if (final != null && final < 0.01) {
      append(`converged (MSE < 1e-2)`);
    } else {
      append(`final MSE = ${final != null ? final.toExponential(4) : 'n/a'}`);
    }
    append('done — download JSON below to load into Rust EmlModel::from_json');
    setState('done');
  };

  const download = (which: 'q' | 'k' | 'v' | 'out' | 'all') => {
    if (!attn) return;
    const bundle = attn.exportJsonBundle('eml_attention_toy');
    if (which === 'all') {
      for (const [name, payload] of Object.entries(bundle)) {
        downloadJson(name, payload);
      }
    } else {
      const name = `eml_attention_toy_${which}.json`;
      downloadJson(name, bundle[name]);
    }
  };

  // kept for compatibility with older references; not used in the new chart
  void (curve.length ? Math.max(...curve) : 1);

  return (
    <main className="mx-auto my-10 max-w-4xl px-6 leading-relaxed text-neutral-900 dark:text-neutral-100">
      <h1 className="mb-4 text-3xl font-semibold">
        WeftOS Toy EML-Transformer — live browser trainer
      </h1>
      <p className="mb-4">
        Trains an Iteration-0 EML-Attention block in your browser and exports
        Rust-loadable JSON. Pure TypeScript port of the Rust{' '}
        <code>ToyEmlAttention</code> at{' '}
        <code>crates/eml-core/src/attention.rs</code>. No Python, no Pyodide, no
        Colab — just click <strong>Train</strong> and download the resulting
        weights.
      </p>
      <p className="mb-4 text-sm text-neutral-600 dark:text-neutral-400">
        For the live forward-pass demo (no training), see{' '}
        <a href="/clawft_eml-attention" className="underline">
          /clawft_eml-attention
        </a>
        . For the architecture spec + go/no-go criteria see the{' '}
        <a href="/docs/weftos/eml-attention" className="underline">
          EML Attention docs
        </a>
        .
      </p>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Configuration</h2>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4">
        <label className="flex flex-col text-sm">
          <span className="font-mono">d_model: {dModel}</span>
          <input
            type="range"
            min={4}
            max={16}
            step={4}
            value={dModel}
            onChange={(e) => {
              const v = Number(e.target.value);
              setDModel(v);
              if (dK > v) setDK(v);
            }}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">d_k: {dK}</span>
          <input
            type="range"
            min={2}
            max={dModel}
            step={2}
            value={dK}
            onChange={(e) => setDK(Number(e.target.value))}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">seq_len: {seqLen}</span>
          <input
            type="range"
            min={2}
            max={6}
            step={1}
            value={seqLen}
            onChange={(e) => setSeqLen(Number(e.target.value))}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">depth: {depth}</span>
          <input
            type="range"
            min={3}
            max={5}
            step={1}
            value={depth}
            onChange={(e) => setDepth(Number(e.target.value))}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">rounds: {rounds}</span>
          <input
            type="range"
            min={1}
            max={5}
            step={1}
            value={rounds}
            onChange={(e) => setRounds(Number(e.target.value))}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm">
          <span className="font-mono">samples: {sampleCount}</span>
          <input
            type="range"
            min={16}
            max={128}
            step={16}
            value={sampleCount}
            onChange={(e) => setSampleCount(Number(e.target.value))}
            className="mt-1 w-full"
          />
        </label>
        <label className="flex flex-col text-sm sm:col-span-2">
          <span className="font-mono">seed: {seed}</span>
          <input
            type="number"
            value={seed}
            onChange={(e) => setSeed(Number(e.target.value))}
            className="mt-1 rounded border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-800 px-2 py-1 text-sm"
          />
        </label>
      </div>
      <p className="mt-2 text-sm text-neutral-600 dark:text-neutral-400">
        Parameter count at this shape: <strong>{paramCount}</strong>. The
        trainer only optimizes <code>out_model</code> per the Iteration-0
        spec; Q/K/V stay at their deterministic seeded init.
      </p>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Train</h2>
      <div className="flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={train}
          disabled={state === 'training'}
          className="rounded border border-neutral-300 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 px-4 py-2 text-sm font-medium hover:bg-neutral-200 dark:hover:bg-neutral-700 disabled:opacity-50"
        >
          {state === 'training' ? 'Training…' : 'Train'}
        </button>
        <span className="text-sm text-neutral-600 dark:text-neutral-400">
          Target: low-rank per-sequence-position mean (learnable given frozen
          Q/K/V). Train time scales with samples × params × rounds.
        </span>
      </div>

      {log.length > 0 && (
        <div className="mt-4">
          <h3 className="text-base font-semibold mb-2">Training log</h3>
          <pre
            className="rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 overflow-auto text-xs leading-relaxed whitespace-pre max-h-72"
            aria-live="polite"
          >
{log.join('\n')}
          </pre>
        </div>
      )}

      {curve.length > 0 && (
        <div className="mt-4 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4">
          <h3 className="text-base font-semibold mb-3">MSE per round</h3>
          <div className="overflow-x-auto">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="bg-neutral-100 dark:bg-neutral-800">
                  <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-left">Round</th>
                  <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-left">MSE</th>
                  <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-left">Δ vs prev</th>
                  <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-left">Δ vs round 1</th>
                </tr>
              </thead>
              <tbody>
                {curve.map((mse, i) => {
                  const prev = i > 0 ? curve[i - 1] : mse;
                  const first = curve[0];
                  const deltaPrev = i > 0 ? mse - prev : 0;
                  const deltaFirst = mse - first;
                  const pctFirst = first > 0 ? (1 - mse / first) * 100 : 0;
                  return (
                    <tr key={i}>
                      <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">{i + 1}</td>
                      <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">{mse.toExponential(4)}</td>
                      <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">
                        {i === 0 ? '—' : (deltaPrev >= 0 ? '+' : '') + deltaPrev.toExponential(2)}
                      </td>
                      <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">
                        {i === 0 ? '—' : `${deltaFirst.toExponential(2)} (${pctFirst.toFixed(1)}% ↓)`}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* SVG sparkline with y-axis in log space so small-but-real improvements remain visible. */}
          <div className="mt-4">
            <MSESparkline values={curve} />
          </div>

          <p className="mt-3 text-sm">
            Final MSE:{' '}
            <strong>{finalMse != null ? finalMse.toExponential(4) : '—'}</strong>
            {finalMse != null && finalMse < 0.01 && (
              <span className="ml-2 rounded bg-emerald-100 dark:bg-emerald-900 px-2 py-0.5 text-xs text-emerald-800 dark:text-emerald-100">
                converged
              </span>
            )}
          </p>
        </div>
      )}

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Export Rust-loadable JSON</h2>
      <p className="mb-3 text-sm text-neutral-600 dark:text-neutral-400">
        Each download contains the full weights for one submodel in a shape
        that round-trips with <code>eml_core::EmlModel</code>. You can train
        offline in the browser, commit the JSON to your repo, and{' '}
        <code>EmlModel::from_json</code> loads them directly.
      </p>
      <div className="flex flex-wrap gap-2">
        {(['q', 'k', 'v', 'out', 'all'] as const).map((which) => (
          <button
            key={which}
            type="button"
            onClick={() => download(which)}
            disabled={!attn}
            className="rounded border border-neutral-300 dark:border-neutral-700 bg-neutral-100 dark:bg-neutral-800 px-3 py-1.5 text-sm font-mono hover:bg-neutral-200 dark:hover:bg-neutral-700 disabled:opacity-50"
          >
            {which === 'all' ? 'download bundle' : `download ${which}_model`}
          </button>
        ))}
      </div>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Loading in Rust</h2>
      <pre className="mt-2 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 overflow-x-auto text-xs leading-relaxed whitespace-pre">
{`use eml_core::EmlModel;

let q_json = std::fs::read_to_string("eml_attention_toy_q.json")?;
let q = EmlModel::from_json(&q_json).unwrap();
// same for k, v, out — plug into ToyEmlAttention`}
      </pre>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">What Iteration 0 trains</h2>
      <ul className="list-disc pl-6 space-y-1 text-sm">
        <li>
          <code>out_model</code> trains via gradient-free coordinate descent on
          (context → target) pairs derived from the current forward pass.
        </li>
        <li>
          Q, K, V, and softmax sub-models stay at their deterministic seeded
          init. This matches the Rust <code>ToyEmlAttention::train</code>{' '}
          behavior and reflects Iteration-0 scope.
        </li>
        <li>
          End-to-end coordinate descent across all 5 sub-models is
          Iteration&nbsp;1. The identity task is provably unlearnable under
          Iteration 0 constraints — the default target here is per-position
          mean, which is low-rank enough to converge.
        </li>
      </ul>
    </main>
  );
}
