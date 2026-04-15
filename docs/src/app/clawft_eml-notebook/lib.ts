// Pure-TS port of the Rust ToyEmlAttention + EmlModel training loop.
// Runs entirely in the browser — no Pyodide, no Colab, no Python.

export function emlSafe(x: number, y: number): number {
  const ex = Math.exp(Math.max(-20, Math.min(20, x)));
  const ly = Math.log(Math.max(Number.MIN_VALUE, y));
  return ex - ly;
}

export function numericalSoftmax(row: number[]): number[] {
  const m = Math.max(...row);
  const e = row.map((v) => Math.exp(v - m));
  const s = e.reduce((a, b) => a + b, 0);
  return s > 0 ? e.map((v) => v / s) : row.map(() => 1 / row.length);
}

// 32-bit Numerical Recipes LCG — reproducible random in [-1, 1].
export function makeRng(seed: number) {
  let s = seed >>> 0;
  return (): number => {
    s = (Math.imul(s, 1664525) + 1013904223) | 0;
    return ((s >>> 0) / 0x100000000) * 2 - 1;
  };
}

export interface EmlModelJson {
  model_name: string;
  depth: number;
  inputs: number;
  heads: number;
  level0_weight: number[][];
  level0_bias: number[];
  tree_params: number[][];
  trained: boolean;
  mse: number;
  timestamp: number;
}

export class PyEmlModel {
  depth: number;
  inputs: number;
  heads: number;
  w: Float64Array; // heads * inputs
  b: Float64Array; // heads
  tree: Float64Array; // heads * depth * 3
  trained = false;
  lastMse = Infinity;

  constructor(depth: number, inputs: number, heads: number, rng: () => number) {
    this.depth = depth;
    this.inputs = inputs;
    this.heads = heads;
    this.w = new Float64Array(heads * inputs);
    this.b = new Float64Array(heads);
    this.tree = new Float64Array(heads * depth * 3);
    for (let i = 0; i < this.w.length; i++) this.w[i] = rng() * 0.1;
    for (let i = 0; i < this.tree.length; i++) this.tree[i] = rng() * 0.05;
  }

  totalParams(): number {
    return this.w.length + this.b.length + this.tree.length;
  }

  predict(x: number[]): number[] {
    const out = new Float64Array(this.heads);
    for (let h = 0; h < this.heads; h++) {
      let acc = this.b[h];
      for (let i = 0; i < this.inputs; i++) acc += this.w[h * this.inputs + i] * x[i];
      let v = acc;
      for (let d = 0; d < this.depth; d++) {
        const c0 = this.tree[h * this.depth * 3 + d * 3];
        const c1 = this.tree[h * this.depth * 3 + d * 3 + 1];
        const c2 = this.tree[h * this.depth * 3 + d * 3 + 2];
        v = emlSafe(v * c0 + c1, Math.abs(v) + c2 + 1);
      }
      out[h] = v;
    }
    return Array.from(out);
  }

  // Gradient-free coordinate descent with random restarts.
  // Mirrors the Rust EmlModel::train philosophy: no backprop, no autodiff,
  // just per-parameter perturbation + accept-if-better.
  train(samples: { x: number[]; y: number[] }[], rng: () => number, opts?: {
    rounds?: number;
    stepInit?: number;
    stepMin?: number;
    stepDecay?: number;
  }): number {
    if (samples.length < 10) {
      this.lastMse = Infinity;
      return Infinity;
    }
    const rounds = opts?.rounds ?? 3;
    const stepInit = opts?.stepInit ?? 0.4;
    const stepMin = opts?.stepMin ?? 0.005;
    const stepDecay = opts?.stepDecay ?? 0.6;

    const mseOn = (): number => {
      let sum = 0;
      for (const s of samples) {
        const pred = this.predict(s.x);
        for (let h = 0; h < this.heads; h++) {
          const diff = pred[h] - s.y[h];
          sum += diff * diff;
        }
      }
      return sum / (samples.length * this.heads);
    };

    let bestMse = mseOn();

    for (let round = 0; round < rounds; round++) {
      let step = stepInit;
      while (step > stepMin) {
        let improvedThisStep = false;
        const paramArrays: Float64Array[] = [this.w, this.b, this.tree];
        for (const arr of paramArrays) {
          for (let i = 0; i < arr.length; i++) {
            const saved = arr[i];
            const delta = step * rng();
            arr[i] = saved + delta;
            const candidate = mseOn();
            if (candidate + 1e-12 < bestMse) {
              bestMse = candidate;
              improvedThisStep = true;
            } else {
              arr[i] = saved;
            }
          }
        }
        if (!improvedThisStep) step *= stepDecay;
      }
    }

    this.lastMse = bestMse;
    this.trained = bestMse < 0.1;
    return bestMse;
  }

  toJson(modelName: string): EmlModelJson {
    const w2d: number[][] = [];
    for (let h = 0; h < this.heads; h++) {
      const row: number[] = [];
      for (let i = 0; i < this.inputs; i++) row.push(this.w[h * this.inputs + i]);
      w2d.push(row);
    }
    const t2d: number[][] = [];
    for (let h = 0; h < this.heads; h++) {
      const row: number[] = [];
      for (let i = 0; i < this.depth * 3; i++) row.push(this.tree[h * this.depth * 3 + i]);
      t2d.push(row);
    }
    return {
      model_name: modelName,
      depth: this.depth,
      inputs: this.inputs,
      heads: this.heads,
      level0_weight: w2d,
      level0_bias: Array.from(this.b),
      tree_params: t2d,
      trained: this.trained,
      mse: this.lastMse,
      timestamp: Date.now() / 1000,
    };
  }
}

export class ToyEmlAttention {
  seqLen: number;
  dModel: number;
  dK: number;
  depth: number;
  q: PyEmlModel;
  k: PyEmlModel;
  v: PyEmlModel;
  out: PyEmlModel;
  scale: number;

  constructor(dModel: number, dK: number, seqLen: number, depth: number, seed: number) {
    this.seqLen = seqLen;
    this.dModel = dModel;
    this.dK = dK;
    this.depth = depth;
    const rng = makeRng(seed);
    const projIn = seqLen * dModel;
    const projOut = seqLen * dK;
    this.q = new PyEmlModel(depth, projIn, projOut, rng);
    this.k = new PyEmlModel(depth, projIn, projOut, rng);
    this.v = new PyEmlModel(depth, projIn, projOut, rng);
    this.out = new PyEmlModel(depth, projOut, projIn, rng);
    this.scale = 1 / Math.sqrt(dK);
  }

  paramCount(): number {
    return this.q.totalParams() + this.k.totalParams() + this.v.totalParams() + this.out.totalParams();
  }

  forward(x: number[]): { y: number[]; attn: number[][]; context: number[] } {
    const qp = this.q.predict(x);
    const kp = this.k.predict(x);
    const vp = this.v.predict(x);
    const n = this.seqLen;
    const d = this.dK;
    const scores: number[][] = [];
    for (let i = 0; i < n; i++) {
      const row: number[] = [];
      for (let j = 0; j < n; j++) {
        let s = 0;
        for (let r = 0; r < d; r++) s += qp[i * d + r] * kp[j * d + r];
        row.push(s * this.scale);
      }
      scores.push(row);
    }
    const attn = scores.map(numericalSoftmax);
    const ctx: number[] = new Array(n * d).fill(0);
    for (let i = 0; i < n; i++) {
      for (let r = 0; r < d; r++) {
        let s = 0;
        for (let j = 0; j < n; j++) s += attn[i][j] * vp[j * d + r];
        ctx[i * d + r] = s;
      }
    }
    const y = this.out.predict(ctx);
    return { y, attn, context: ctx };
  }

  // Iteration-0 training: only the out_model trains (Q/K/V stay frozen at init).
  // Matches the Rust behavior in crates/eml-core/src/attention.rs.
  trainOutModelOnly(
    samples: { x: number[]; target: number[] }[],
    rng: () => number,
    opts?: { rounds?: number },
  ): number[] {
    const curve: number[] = [];
    const training: { x: number[]; y: number[] }[] = [];
    for (const s of samples) {
      const { context } = this.forward(s.x);
      training.push({ x: context, y: s.target });
    }
    const rounds = opts?.rounds ?? 3;
    for (let r = 0; r < rounds; r++) {
      const mse = this.out.train(training, rng, { rounds: 1 });
      curve.push(mse);
    }
    return curve;
  }

  exportJsonBundle(prefix: string): Record<string, EmlModelJson> {
    return {
      [`${prefix}_q.json`]: this.q.toJson(`${prefix}_q`),
      [`${prefix}_k.json`]: this.k.toJson(`${prefix}_k`),
      [`${prefix}_v.json`]: this.v.toJson(`${prefix}_v`),
      [`${prefix}_out.json`]: this.out.toJson(`${prefix}_out`),
    };
  }
}

export function downloadJson(filename: string, payload: unknown) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], {
    type: 'application/json',
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
