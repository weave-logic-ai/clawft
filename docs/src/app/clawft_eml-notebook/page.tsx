'use client';

import { useMemo, useState } from 'react';
import {
  BaselineAttention,
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

  const [mode, setMode] = useState<'iter0' | 'iter1' | 'baseline' | 'compare'>('compare');
  const [state, setState] = useState<TrainState>('idle');
  const [curve, setCurve] = useState<number[]>([]);
  const [finalMse, setFinalMse] = useState<number | null>(null);
  const [attn, setAttn] = useState<ToyEmlAttention | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [compareResult, setCompareResult] = useState<null | {
    eml: { params: number; baseline: number; final: number; p99us: number };
    base: { params: number; baseline: number; final: number; p99us: number };
  }>(null);

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

    let trainingCurve: number[] = [];
    setCompareResult(null);

    const rngA = makeRng(seed + 1);
    const rngB = makeRng(seed + 1);

    if (mode === 'iter1' || mode === 'compare') {
      append(mode === 'compare'
        ? 'backend: EML (SafeTree) — Iteration 1 joint e2e CD across Q/K/V/out'
        : 'mode: Iteration 1 — joint end-to-end coordinate descent across Q/K/V/out');
      trainingCurve = await a.trainEndToEnd(samples, rngA, {
        rounds,
        onStatus: (msg) => append(msg),
        onStart: ({ samples: n, params, trialsPerRound, baseline }) => {
          append(`e2e CD: ${params} params, ${n} samples, ${trialsPerRound} trials/round`);
          append(`baseline MSE = ${baseline.toExponential(4)}`);
        },
        onRound: (round, mse, elapsedMs) => {
          append(`[eml] round ${round}/${rounds}   MSE = ${mse.toExponential(4)}   elapsed = ${elapsedMs.toFixed(0)} ms`);
          setCurve((prev) => [...prev, mse]);
        },
      });
    } else if (mode === 'iter0') {
      append('mode: Iteration 0 — out_model-only self-distillation');
      trainingCurve = await a.trainOutModelOnly(samples, rngA, {
        rounds,
        onStatus: (msg) => append(msg),
        onStart: ({ samples: n, params, trialsPerRound }) => {
          append(`train out_model: ${params} params, ${n} context pairs, ${trialsPerRound} trials/round`);
        },
        onRound: (round, mse, elapsedMs) => {
          append(`round ${round}/${rounds}   MSE = ${mse.toExponential(4)}   elapsed = ${elapsedMs.toFixed(0)} ms`);
          setCurve((prev) => [...prev, mse]);
        },
      });
    }

    // Baseline (plain affine) leg — runs alone in baseline mode, in parallel-display in compare mode.
    let baselineCurve: number[] = [];
    let baselineStats = { params: 0, baseline: 0, final: 0 };
    if (mode === 'baseline' || mode === 'compare') {
      const base = new BaselineAttention(dModel, dK, seqLen, seed);
      append(mode === 'compare'
        ? 'backend: Baseline (plain affine) — joint e2e CD across Q/K/V/out'
        : 'mode: Baseline — plain affine attention, same CD optimizer');
      baselineCurve = await base.trainEndToEnd(samples, rngB, {
        rounds,
        onStatus: (msg) => append(msg),
        onStart: ({ samples: n, params, trialsPerRound, baseline }) => {
          append(`[baseline] ${params} params, ${n} samples, ${trialsPerRound} trials/round`);
          append(`[baseline] baseline MSE = ${baseline.toExponential(4)}`);
          baselineStats.params = params;
          baselineStats.baseline = baseline;
        },
        onRound: (round, mse, elapsedMs) => {
          append(`[baseline] round ${round}/${rounds}   MSE = ${mse.toExponential(4)}   elapsed = ${elapsedMs.toFixed(0)} ms`);
          if (mode === 'baseline') setCurve((prev) => [...prev, mse]);
        },
      });
      baselineStats.final = baselineCurve[baselineCurve.length - 1] ?? Infinity;
      if (mode === 'baseline') trainingCurve = baselineCurve;
    }

    // Compare mode: measure inference p99 for both + render side-by-side stats.
    if (mode === 'compare') {
      const base = new BaselineAttention(dModel, dK, seqLen, seed);
      const timeIt = (fn: () => void): number => {
        const lats: number[] = [];
        for (let i = 0; i < 128; i++) {
          const t = performance.now();
          fn();
          lats.push((performance.now() - t) * 1000); // µs
        }
        lats.sort((x, y) => x - y);
        return lats[Math.floor(128 * 0.99)];
      };
      const sampleX = samples[0].x;
      const emlP99 = timeIt(() => { a.forward(sampleX); });
      const baseP99 = timeIt(() => { base.forward(sampleX); });
      setCompareResult({
        eml: {
          params: a.paramCount(),
          baseline: trainingCurve[0] ?? 0,
          final: trainingCurve[trainingCurve.length - 1] ?? 0,
          p99us: emlP99,
        },
        base: {
          params: baselineStats.params || base.paramCount(),
          baseline: baselineStats.baseline,
          final: baselineStats.final,
          p99us: baseP99,
        },
      });
    }

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
        Trains a tiny attention block in your browser and exports Rust-loadable
        JSON. Pure TypeScript port of the Rust <code>ToyEmlAttention</code> at{' '}
        <code>crates/eml-core/src/attention.rs</code>. No Python, no Pyodide,
        no Colab — pick a mode, click <strong>Train</strong>, watch it learn,
        download the weights.
      </p>
      <p className="mb-6 text-sm text-neutral-600 dark:text-neutral-400">
        Forward-pass-only sibling at{' '}
        <a href="/clawft_eml-attention" className="underline">
          /clawft_eml-attention
        </a>
        . Architecture spec at{' '}
        <a href="/docs/weftos/eml-attention" className="underline">
          /docs/weftos/eml-attention
        </a>
        .
      </p>

      <details className="mb-6 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4">
        <summary className="cursor-pointer font-semibold">
          New here? What is this thing actually doing?
        </summary>
        <div className="mt-3 text-sm space-y-3 text-neutral-700 dark:text-neutral-300">
          <p>
            This page trains an <strong>attention block</strong> — the same
            building block used inside every modern transformer LLM (GPT,
            Claude, etc.) — but at toy scale (a few hundred parameters
            instead of billions).
          </p>
          <p>
            <strong>What attention does:</strong> given a sequence of tokens
            (here represented as random vectors), each output position is a
            weighted blend of every input position. The weights — the{' '}
            <em>attention pattern</em> — are computed dynamically per input.
            That dynamism is what lets transformers handle variable-length,
            context-dependent inputs.
          </p>
          <p>
            <strong>The five sub-models:</strong> attention computes three
            projections of the input (Q for Query, K for Key, V for Value),
            then uses Q and K to decide weights via{' '}
            <code>softmax(Q·Kᵀ / √dₖ)</code>, then blends V using those
            weights, then projects through a final output layer. The "Iter 0"
            variant only trains the output layer; "Iter 1" trains all four
            (Q/K/V/out) jointly.
          </p>
          <p>
            <strong>What training does:</strong> we feed in random input
            vectors paired with target outputs (here: per-sequence-position
            mean broadcast back across <code>d_model</code>), measure how
            wrong the current weights are (Mean Squared Error), then nudge
            weights to reduce that error. This page uses{' '}
            <strong>gradient-free coordinate descent</strong>: pick a random
            parameter, perturb it, keep the change if MSE drops, otherwise
            revert. No backprop, no autograd — just trial-and-error guided
            by error feedback. That's slower than gradient descent but
            simpler, deterministic, and small-substrate-friendly.
          </p>
          <p>
            <strong>Two substrates compared</strong> in "EML vs baseline"
            mode:
          </p>
          <ul className="list-disc pl-6 space-y-1">
            <li>
              <strong>EML (SafeTree)</strong>: each Q/K/V/out projection is
              a depth-N tree of <code>exp(x) − ln(y)</code> operators.
              Universal function approximator, weights snap to integers,
              human-readable trained models. WeftOS's bet for interpretable
              learned functions.
            </li>
            <li>
              <strong>Baseline (affine)</strong>: each projection is a plain{' '}
              <code>W·x + b</code> matrix multiply. Standard transformer
              math. Fewer parameters, faster, easier to train.
            </li>
          </ul>
          <p>
            Same training loop, same trial budget, same seed — only the
            substrate differs. Watch them converge side-by-side.
          </p>
        </div>
      </details>

      <h2 className="mt-8 mb-3 text-2xl font-semibold">Configuration</h2>
      <p className="mb-3 text-sm text-neutral-600 dark:text-neutral-400">
        These knobs change the shape of the attention block and the training
        budget. <code>d_model</code> is the input embedding dimension per
        token; <code>seq_len</code> is the number of tokens per sample;{' '}
        <code>d_k</code> is the Q/K/V projection width;{' '}
        <code>depth</code> is the EML tree depth (ignored by the affine
        baseline). <code>rounds</code> and <code>samples</code> control how
        long training runs. <code>seed</code> makes runs reproducible — same
        seed, same trajectory.
      </p>
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
      <p className="mb-3 text-sm text-neutral-600 dark:text-neutral-400">
        Pick a mode, then click Train. <strong>EML vs baseline</strong> runs
        both substrates back-to-back and shows a side-by-side comparison —
        this is the most informative mode and the default. The single-mode
        options run only one substrate so you can study its behavior in
        isolation.
      </p>
      <div className="flex flex-wrap items-center gap-3 mb-3">
        <span className="text-sm font-mono">mode:</span>
        <div className="inline-flex rounded border border-neutral-300 dark:border-neutral-700 overflow-hidden text-sm flex-wrap">
          {(['compare', 'iter1', 'iter0', 'baseline'] as const).map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMode(m)}
              className={`px-3 py-1.5 border-r border-neutral-300 dark:border-neutral-700 last:border-r-0 ${mode === m ? 'bg-neutral-200 dark:bg-neutral-700 font-semibold' : 'bg-neutral-50 dark:bg-neutral-900'}`}
            >
              {m === 'compare' ? 'EML vs baseline' : m === 'iter1' ? 'EML (joint e2e)' : m === 'iter0' ? 'EML (out only)' : 'baseline (affine)'}
            </button>
          ))}
        </div>
      </div>
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
          Target: per-position mean broadcast (low-rank, learnable under both modes).
          {mode === 'iter1' && ' Iter-1 trials scale with param count — budget ~5-10s on default shape.'}
        </span>
      </div>

      {compareResult && (
        <div className="mt-4 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4">
          <h3 className="text-base font-semibold mb-3">Head-to-head: EML vs Baseline</h3>
          <table className="w-full text-sm border-collapse">
            <thead>
              <tr className="bg-neutral-100 dark:bg-neutral-800">
                <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-left">metric</th>
                <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-right">EML (SafeTree)</th>
                <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 text-right">Baseline (affine)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">params</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.eml.params}</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.base.params}</td>
              </tr>
              <tr>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">baseline MSE</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.eml.baseline.toExponential(3)}</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.base.baseline.toExponential(3)}</td>
              </tr>
              <tr>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">final MSE</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.eml.final.toExponential(3)}</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.base.final.toExponential(3)}</td>
              </tr>
              <tr>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">MSE reduction</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">
                  {compareResult.eml.baseline > 0 ? ((1 - compareResult.eml.final / compareResult.eml.baseline) * 100).toFixed(1) + '%' : '—'}
                </td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">
                  {compareResult.base.baseline > 0 ? ((1 - compareResult.base.final / compareResult.base.baseline) * 100).toFixed(1) + '%' : '—'}
                </td>
              </tr>
              <tr>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono">forward p99 (µs)</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.eml.p99us.toFixed(2)}</td>
                <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1.5 font-mono text-right">{compareResult.base.p99us.toFixed(2)}</td>
              </tr>
            </tbody>
          </table>
          <p className="mt-3 text-sm text-neutral-600 dark:text-neutral-400">
            Same coordinate-descent optimizer, same trial budget, same seed — the only difference
            is the attention substrate. Live results in-browser; the Rust head-to-head produces
            comparable numbers (see release notes §v0.6.10).
          </p>
        </div>
      )}

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

      {(state === 'done' || compareResult) && (
        <>
          <h2 className="mt-10 mb-3 text-2xl font-semibold">
            Results & interpretation
          </h2>
          <ResultsInterpretation
            mode={mode}
            curve={curve}
            finalMse={finalMse}
            compareResult={compareResult}
            paramCount={paramCount}
            seqLen={seqLen}
            dModel={dModel}
            dK={dK}
          />
        </>
      )}

      <h2 className="mt-10 mb-3 text-2xl font-semibold">
        What's happening under the hood
      </h2>
      <div className="text-sm space-y-3 text-neutral-700 dark:text-neutral-300">
        <p>
          <strong>Forward pass — the attention math</strong>: every time the
          trainer evaluates MSE, the block runs this sequence:
        </p>
        <ol className="list-decimal pl-6 space-y-1">
          <li>
            <strong>Project to Q, K, V</strong>. The flattened input
            (<code>seq_len × d_model</code>) goes into three separate
            projection layers. Each produces a <code>seq_len × d_k</code>{' '}
            tensor. Q is what each position is "asking about", K is what each
            position "advertises", V is what each position will contribute if
            attended to.
          </li>
          <li>
            <strong>Score</strong>: for every pair of positions (i, j),
            compute <code>scoresᵢⱼ = (Qᵢ · Kⱼ) / √dₖ</code>. Bigger score
            means position i wants more of position j's value.
          </li>
          <li>
            <strong>Softmax</strong>: turn each row of scores into a
            probability distribution (rows sum to 1). This is the{' '}
            <em>attention pattern</em> — the dynamic, input-dependent
            routing.
          </li>
          <li>
            <strong>Blend V</strong>: compute{' '}
            <code>contextᵢ = Σⱼ attnᵢⱼ · Vⱼ</code>. Each position now
            contains a weighted blend of every position's value.
          </li>
          <li>
            <strong>Output projection</strong>: project context back to{' '}
            <code>d_model</code> dimensions per position. This is the
            block's output.
          </li>
        </ol>
        <p>
          Real transformers stack many of these blocks (12 in BERT-base, 96
          in GPT-3) with residual connections, layer norm, and a feed-forward
          network between attention layers. We're showing one block in
          isolation.
        </p>
        <p>
          <strong>Training — what the optimizer is actually doing</strong>:
        </p>
        <ol className="list-decimal pl-6 space-y-1">
          <li>
            Pick a random parameter index across the union of all layers.
          </li>
          <li>
            Generate a perturbation <code>δ</code> from an annealing schedule
            (big early, small late).
          </li>
          <li>
            Apply <code>δ</code>, run the forward pass on a 16-sample subset,
            measure MSE.
          </li>
          <li>
            If MSE is lower than the previous best, keep the change. Else
            revert.
          </li>
          <li>
            Repeat for the full trial budget. The fraction of accepted
            perturbations decreases as the model converges — early on, easy
            wins; late on, you need precise small steps to make progress.
          </li>
        </ol>
        <p>
          This is <strong>random coordinate descent with simulated
          annealing</strong>. It's the simplest possible gradient-free
          optimizer that still works on hundreds of parameters. Real LLMs
          use Adam (gradient-based) which is millions of times more sample-
          efficient — but requires backprop, which EML deliberately avoids
          because the <code>exp(x) − ln(y)</code> tree is hard to
          differentiate cleanly.
        </p>
        <p>
          <strong>What MSE means here</strong>: Mean Squared Error is the
          average of <code>(prediction − target)²</code> across all output
          dimensions and all samples. MSE = 0 means perfect prediction.
          MSE = 1.0 on this task means the predictions are about 1 unit
          off per dimension on average (since inputs are in [-1, 1]).
        </p>
      </div>

      <h2 className="mt-10 mb-3 text-2xl font-semibold">Why two substrates?</h2>
      <div className="text-sm space-y-3 text-neutral-700 dark:text-neutral-300">
        <p>
          <strong>EML</strong> (Exp-Minus-Log) replaces every multiply with
          a tree of <code>eml(x, y) = exp(x) − ln(y)</code> operations.
          Combined with the constant 1, this single operator can reconstruct{' '}
          <em>any</em> elementary mathematical function (Odrzywolel 2026,
          arXiv:2603.21852). Trained EML weights snap to exact integers,
          making the model human-readable as a closed-form formula. WeftOS
          has 12 production EML models doing scalar control-plane decisions.
        </p>
        <p>
          <strong>Baseline</strong> is just <code>W·x + b</code> — the
          standard transformer projection. No tree, no transcendental
          operators, just dot products. This is what every published
          transformer uses.
        </p>
        <p>
          <strong>The honest comparison</strong> at toy scale (see results
          above): plain affine attention converges much faster, runs ~3×
          faster on forward pass, and uses ~2× fewer parameters than EML.
          What EML buys at this scale is <em>interpretability</em> — the
          trained model is a closed-form expression, not an opaque weight
          tensor. Whether that tradeoff is worth it depends on what you're
          building. For sequence-modeling backbones, baseline wins. For
          control-plane scalar decisions (the existing 12 WeftOS wrappers),
          EML wins decisively.
        </p>
        <p>
          <strong>This is research, not production</strong>. The real
          finding from this experiment isn't "use EML attention" — it's
          "we now know EML's sweet spot ends around vector-output
          sequence work, and starts paying off again as scalar
          interpretable functions". See the{' '}
          <a href="/docs/weftos/eml-attention" className="underline">
            EML Attention docs
          </a>{' '}
          for the full reasoning and the upcoming "EML at the control-plane
          boundary" design pattern.
        </p>
      </div>
    </main>
  );
}

// --------------------------------------------------------------------------
// Results & interpretation component
// --------------------------------------------------------------------------

function ResultsInterpretation(props: {
  mode: 'iter0' | 'iter1' | 'baseline' | 'compare';
  curve: number[];
  finalMse: number | null;
  compareResult: null | {
    eml: { params: number; baseline: number; final: number; p99us: number };
    base: { params: number; baseline: number; final: number; p99us: number };
  };
  paramCount: number;
  seqLen: number;
  dModel: number;
  dK: number;
}) {
  const { mode, curve, finalMse, compareResult, paramCount, seqLen, dModel, dK } = props;

  if (mode === 'compare' && compareResult) {
    return <CompareScore data={compareResult} />;
  }

  if (curve.length === 0 || finalMse === null) {
    return (
      <p className="text-sm text-neutral-600 dark:text-neutral-400">
        Train first to see results.
      </p>
    );
  }

  const baseline = curve[0];
  const reduction = baseline > 0 ? (1 - finalMse / baseline) * 100 : 0;
  const grade = scoreReduction(reduction, finalMse);

  return (
    <div className="rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 space-y-4 text-sm">
      <div>
        <div className="text-lg font-semibold mb-1">
          Score: <span className={gradeClass(grade.tier)}>{grade.tier}</span>
          <span className="ml-2 text-neutral-500 dark:text-neutral-400 font-normal">
            ({reduction.toFixed(1)}% MSE reduction)
          </span>
        </div>
        <p className="text-neutral-700 dark:text-neutral-300">{grade.summary}</p>
      </div>

      <div>
        <h3 className="font-semibold mb-1">Run details</h3>
        <ul className="list-disc pl-6 space-y-0.5 text-neutral-700 dark:text-neutral-300">
          <li>Mode: <code>{modeLabel(mode)}</code></li>
          <li>
            Shape: <code>seq_len={seqLen}, d_model={dModel}, d_k={dK}</code>{' '}
            ({paramCount} EML params total)
          </li>
          <li>
            Starting MSE: <code>{baseline.toExponential(3)}</code>
          </li>
          <li>
            Final MSE: <code>{finalMse.toExponential(3)}</code>
          </li>
          <li>
            Improvement: <strong>{reduction.toFixed(1)}%</strong> reduction
            over {curve.length} round(s)
          </li>
        </ul>
      </div>

      <div>
        <h3 className="font-semibold mb-1">What this tells us</h3>
        <ul className="list-disc pl-6 space-y-1 text-neutral-700 dark:text-neutral-300">
          {grade.bullets.map((b, i) => (
            <li key={i}>{b}</li>
          ))}
        </ul>
      </div>

      <div>
        <h3 className="font-semibold mb-1">Knobs to try next</h3>
        <ul className="list-disc pl-6 space-y-1 text-neutral-700 dark:text-neutral-300">
          {grade.suggestions.map((s, i) => (
            <li key={i}>{s}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function CompareScore({
  data,
}: {
  data: {
    eml: { params: number; baseline: number; final: number; p99us: number };
    base: { params: number; baseline: number; final: number; p99us: number };
  };
}) {
  const emlReduction = data.eml.baseline > 0 ? (1 - data.eml.final / data.eml.baseline) * 100 : 0;
  const baseReduction = data.base.baseline > 0 ? (1 - data.base.final / data.base.baseline) * 100 : 0;
  const winnerMse = data.eml.final < data.base.final ? 'EML' : 'Baseline';
  const winnerSpeed = data.eml.p99us < data.base.p99us ? 'EML' : 'Baseline';
  const winnerParams = data.eml.params < data.base.params ? 'EML' : 'Baseline';
  const speedRatio = Math.max(data.eml.p99us, data.base.p99us) / Math.min(data.eml.p99us, data.base.p99us);
  const paramRatio = Math.max(data.eml.params, data.base.params) / Math.min(data.eml.params, data.base.params);

  return (
    <div className="rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 space-y-4 text-sm">
      <div>
        <div className="text-lg font-semibold mb-1">
          Comparison verdict
        </div>
        <p className="text-neutral-700 dark:text-neutral-300">
          Same training loop, same seed, same data — the only difference is
          the substrate. {winnerMse} reaches lower MSE
          ({winnerMse === 'EML' ? data.eml.final.toExponential(2) : data.base.final.toExponential(2)} vs{' '}
          {winnerMse === 'EML' ? data.base.final.toExponential(2) : data.eml.final.toExponential(2)}),{' '}
          {winnerSpeed} is faster on forward pass ({speedRatio.toFixed(2)}× ahead),
          and {winnerParams} uses fewer parameters ({paramRatio.toFixed(2)}× fewer).
        </p>
      </div>

      <div>
        <h3 className="font-semibold mb-1">Per-substrate score</h3>
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="bg-neutral-100 dark:bg-neutral-800">
              <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 text-left">substrate</th>
              <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 text-right">grade</th>
              <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 text-right">MSE reduction</th>
              <th className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 text-right">final MSE</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono">EML (SafeTree)</td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">
                <span className={gradeClass(scoreReduction(emlReduction, data.eml.final).tier)}>
                  {scoreReduction(emlReduction, data.eml.final).tier}
                </span>
              </td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">{emlReduction.toFixed(1)}%</td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">{data.eml.final.toExponential(2)}</td>
            </tr>
            <tr>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono">Baseline (affine)</td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">
                <span className={gradeClass(scoreReduction(baseReduction, data.base.final).tier)}>
                  {scoreReduction(baseReduction, data.base.final).tier}
                </span>
              </td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">{baseReduction.toFixed(1)}%</td>
              <td className="border border-neutral-300 dark:border-neutral-700 px-3 py-1 font-mono text-right">{data.base.final.toExponential(2)}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div>
        <h3 className="font-semibold mb-1">What this means</h3>
        <ul className="list-disc pl-6 space-y-1 text-neutral-700 dark:text-neutral-300">
          {winnerMse === 'Baseline' && baseReduction > 50 && (
            <li>
              Plain affine attention converges <strong>much</strong> faster
              than EML at this shape. This is the expected result documented
              in the v0.6.10 release notes: at toy scale, EML's nested{' '}
              <code>exp/ln</code> trees create a noisier optimization
              landscape than plain dot products.
            </li>
          )}
          {winnerMse === 'EML' && (
            <li>
              EML reached lower MSE this run — interesting and not the
              typical pattern. Try a different seed; if EML consistently
              wins at your shape, you may have found a regime where the
              EML tree's expressive power outweighs its training overhead.
            </li>
          )}
          {speedRatio > 2 && (
            <li>
              <strong>{winnerSpeed}</strong> is {speedRatio.toFixed(1)}×
              faster on forward pass. EML pays for nested transcendentals
              (exp + ln per tree level per head); baseline is one
              multiply-accumulate per head per input dim.
            </li>
          )}
          {paramRatio > 1.5 && (
            <li>
              EML uses {paramRatio.toFixed(1)}× more parameters. The extra
              params are tree constants — they'd snap to exact integers
              after long enough training, becoming a closed-form formula
              rather than a weight matrix. Baseline weights stay as
              opaque floats.
            </li>
          )}
          <li>
            <strong>Interpretation key</strong>: this experiment isn't
            asking "which is better" in the abstract — it's asking "which
            substrate fits this workload". For sequence modeling, baseline
            wins. For interpretable scalar decisions (the 12 existing WeftOS
            EML wrappers), EML wins. The WeftOS bet is <em>both</em>:
            baseline-style attention for the data plane, EML for the control
            plane around it.
          </li>
        </ul>
      </div>
    </div>
  );
}

function scoreReduction(reductionPct: number, finalMse: number): {
  tier: string;
  summary: string;
  bullets: string[];
  suggestions: string[];
} {
  if (finalMse < 0.01) {
    return {
      tier: 'A — converged',
      summary: 'Final MSE below 0.01 — the model has effectively memorized the task.',
      bullets: [
        'Output predictions are within ~10% of targets on average.',
        'For toy tasks like per-position-mean, this is the gold-standard outcome.',
        'A real transformer at this MSE would be considered overfit unless the train/eval distributions are matched.',
      ],
      suggestions: [
        'Try a harder shape (larger seq_len or d_model) to see how convergence scales.',
        'Switch to "EML vs baseline" mode to compare against plain affine attention.',
        'Drop seed by 1 to see if convergence is robust across initializations.',
      ],
    };
  }
  if (reductionPct >= 50) {
    return {
      tier: 'B — strongly converging',
      summary: '50%+ MSE reduction — the model is learning the task structure even if it hasn\'t memorized it yet.',
      bullets: [
        'The training signal is reaching the parameters effectively.',
        'Per-position-mean is a low-rank target; the model is recovering most of that structure.',
        'Final MSE is still meaningful — about ~30-50% of inputs would be misranked vs perfect.',
      ],
      suggestions: [
        'Bump rounds from 3 to 5 to see if convergence continues or plateaus.',
        'Try increasing trial budget by raising sample count (more diverse training data).',
        'Compare with baseline mode at the same shape — is EML or affine learning more?',
      ],
    };
  }
  if (reductionPct >= 5) {
    return {
      tier: 'C — partial convergence',
      summary: 'Some MSE reduction, but the model is far from memorizing the task.',
      bullets: [
        'Coordinate descent is finding improvements but slowly.',
        'EML at this shape often plateaus here — single-param perturbation has diminishing returns past ~5% reduction.',
        'This is the expected Iteration-2 ceiling for joint EML training; multi-param coordinated perturbation (Iteration 3, future) would push further.',
      ],
      suggestions: [
        'Try a smaller shape (seq_len=2, d_model=4) — easier task, sharper convergence signal.',
        'Switch to baseline mode — affine attention typically reaches 70%+ on this same task.',
        'Increase rounds; the annealing schedule shrinks step size each round, so later rounds make finer adjustments.',
      ],
    };
  }
  if (reductionPct >= 0) {
    return {
      tier: 'D — barely moving',
      summary: 'Almost no MSE reduction. The optimizer is running but the model isn\'t learning meaningfully.',
      bullets: [
        'Either the trial budget is too small for the parameter count, or the substrate is in a noise floor.',
        'EML at high d_model with low trial counts often shows this — there are too many parameters to perturb meaningfully in 5000 trials.',
        'Each accepted perturbation only improves MSE by ~1e-6 because the loss landscape is smooth and shallow.',
      ],
      suggestions: [
        'Drop d_model and seq_len to the minimum (4 and 2) to verify the optimizer can find improvements at all.',
        'Switch to "baseline (affine)" mode to confirm the training loop and target function work correctly.',
        'For real EML training at this scale, you\'d want multi-param coordinated perturbation — coming in Iteration 3.',
      ],
    };
  }
  return {
    tier: 'F — regressing',
    summary: 'Final MSE is higher than starting MSE. The optimizer made the model worse.',
    bullets: [
      'This shouldn\'t happen with the current loop (we revert any non-improving perturbation), so it usually indicates noise from the small MSE-eval subset.',
      'The 16-sample subset means individual round MSE values can fluctuate even when the underlying model improves.',
      'Try increasing samples to 128 — more diverse subset → more stable MSE estimate.',
    ],
    suggestions: [
      'Re-run with a different seed; this is rare and usually doesn\'t repeat.',
      'Increase samples to stabilize MSE measurements.',
      'Switch to baseline mode to verify the rest of the training loop is sane.',
    ],
  };
}

function gradeClass(tier: string): string {
  if (tier.startsWith('A')) return 'text-emerald-600 dark:text-emerald-400';
  if (tier.startsWith('B')) return 'text-blue-600 dark:text-blue-400';
  if (tier.startsWith('C')) return 'text-amber-600 dark:text-amber-400';
  if (tier.startsWith('D')) return 'text-orange-600 dark:text-orange-400';
  return 'text-rose-600 dark:text-rose-400';
}

function modeLabel(mode: 'iter0' | 'iter1' | 'baseline' | 'compare'): string {
  switch (mode) {
    case 'iter0': return 'EML — Iteration 0 (out_model only)';
    case 'iter1': return 'EML — Iteration 1 (joint e2e CD across Q/K/V/out)';
    case 'baseline': return 'Baseline — plain affine attention';
    case 'compare': return 'EML vs baseline — head-to-head';
  }
}
