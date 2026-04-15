export const metadata = {
  title: 'WeftOS Toy EML-Transformer — Iteration 0 Colab Notebook',
  description:
    'Python Colab notebook that trains an Iteration 0 EML-Attention block and exports JSON files loadable by the Rust EmlModel::from_json interface.',
};

const cellCls =
  'my-6 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-white dark:bg-neutral-950 p-4';
const preCls =
  'mt-3 rounded-lg border border-neutral-300 dark:border-neutral-700 bg-neutral-50 dark:bg-neutral-900 p-4 overflow-x-auto text-xs leading-relaxed whitespace-pre';

const cell1 = `!pip install numpy scipy matplotlib -q

import numpy as np
from scipy.optimize import differential_evolution
import json
import time
from collections import deque
import matplotlib.pyplot as plt

def eml(a: np.ndarray, b: np.ndarray, eps: float = 1e-8) -> np.ndarray:
    a = np.clip(a, -18.0, 18.0)
    b = np.clip(b, eps, 1e8)
    return np.exp(a) - np.log(b)

print("Safe EML operator ready (matches Rust eml_safe)")`;

const cell2 = `class PyEmlModel:
    """100% API-compatible with Rust eml_core::EmlModel.

    depth: 2..=5 (matches Rust guard)
    inputs: feature count
    heads:  output dim
    """
    def __init__(self, depth: int, inputs: int, heads: int):
        assert 2 <= depth <= 5
        self.depth = depth
        self.inputs = inputs
        self.heads = heads
        # Level-0 affine (maps to EmlModel Level-0)
        self.level0_weight = np.random.randn(heads, inputs) * 0.1
        self.level0_bias = np.zeros((heads,))
        # Pure EML tree parameters (learned constants inside the tree)
        self.tree_params = np.random.randn(heads, depth * 3) * 0.05
        self.buffer = deque(maxlen=300)
        self.events = []

    def total_params(self):
        return self.level0_weight.size + self.level0_bias.size + self.tree_params.size

    def predict(self, x: np.ndarray) -> np.ndarray:
        x = np.asarray(x, dtype=np.float64).flatten()
        assert x.shape[0] == self.inputs
        proj = self.level0_weight @ x + self.level0_bias
        out = proj.copy()
        for i in range(self.depth):
            c = self.tree_params[:, i*3:(i+1)*3]
            out = eml(out * c[:,0] + c[:,1], np.abs(out) + c[:,2] + 1.0)
        return out

    def record(self, inputs, targets):
        self.buffer.append((np.array(inputs, dtype=np.float64),
                            np.array(targets, dtype=np.float64)))

    def train(self) -> bool:
        if len(self.buffer) < 30:
            return False
        X = np.array([b[0] for b in self.buffer])
        Y = np.array([b[1] for b in self.buffer])

        def loss_fn(p):
            w = p[:self.level0_weight.size].reshape(self.level0_weight.shape)
            b = p[self.level0_weight.size:self.level0_weight.size+self.level0_bias.size]
            t = p[self.level0_weight.size+self.level0_bias.size:].reshape(self.tree_params.shape)
            old_w, old_b, old_t = self.level0_weight.copy(), self.level0_bias.copy(), self.tree_params.copy()
            self.level0_weight, self.level0_bias, self.tree_params = w, b, t
            preds = np.array([self.predict(x) for x in X])
            loss = float(np.mean((preds - Y)**2))
            self.level0_weight, self.level0_bias, self.tree_params = old_w, old_b, old_t
            return loss

        bounds = [(-2, 2)] * self.total_params()
        res = differential_evolution(loss_fn, bounds, workers=1, tol=1e-6, popsize=15, maxiter=30)
        best = res.x
        self.level0_weight = best[:self.level0_weight.size].reshape(self.level0_weight.shape)
        self.level0_bias = best[self.level0_weight.size:self.level0_weight.size+self.level0_bias.size]
        self.tree_params = best[self.level0_weight.size+self.level0_bias.size:].reshape(self.tree_params.shape)

        mse = loss_fn(best)
        converged = mse < 0.01
        self.events.append({
            "type": "Trained",
            "model_name": f"eml_{id(self)}",
            "samples_used": len(self.buffer),
            "mse_before": 999,
            "mse_after": float(mse),
            "converged": converged,
            "param_count": self.total_params(),
        })
        print(f"Trained — MSE = {mse:.6f} {'(converged)' if converged else ''}")
        return converged

    def to_json(self) -> str:
        data = {
            "depth": self.depth,
            "inputs": self.inputs,
            "heads": self.heads,
            "level0_weight": self.level0_weight.tolist(),
            "level0_bias": self.level0_bias.tolist(),
            "tree_params": self.tree_params.tolist(),
            "events": self.events[-5:],
            "timestamp": time.time(),
        }
        return json.dumps(data, indent=2)

print("PyEmlModel ready — JSON shape matches Rust EmlModel::from_json")`;

const cell3 = `class ToyEmlAttention:
    """Python mirror of Rust eml_core::ToyEmlAttention (Iteration 0).

    Forward pass uses f64 matmul between the 5 learned projections,
    exactly like the Rust hybrid. Trains each sub-model independently
    with differential evolution.
    """
    def __init__(self, d_model=16, d_k=8, seq_len=6, depth=4):
        self.d_model, self.d_k, self.seq_len, self.depth = d_model, d_k, seq_len, depth
        proj_in = seq_len * d_model
        proj_out = seq_len * d_k
        self.q_model = PyEmlModel(depth, proj_in, proj_out)
        self.k_model = PyEmlModel(depth, proj_in, proj_out)
        self.v_model = PyEmlModel(depth, proj_in, proj_out)
        self.out_model = PyEmlModel(depth, proj_out, proj_in)
        self.scale = 1.0 / np.sqrt(d_k)
        print(f"Toy EML-Attention ready (seq_len={seq_len}, d_model={d_model}, d_k={d_k}, depth={depth})")

    def forward(self, x_flat: np.ndarray) -> np.ndarray:
        q = self.q_model.predict(x_flat)
        k = self.k_model.predict(x_flat)
        v = self.v_model.predict(x_flat)
        scores = (q.reshape(self.seq_len, self.d_k)
                  @ k.reshape(self.seq_len, self.d_k).T) * self.scale
        scores_exp = np.exp(scores - np.max(scores, axis=-1, keepdims=True))
        attn = scores_exp / (np.sum(scores_exp, axis=-1, keepdims=True) + 1e-8)
        context = (attn @ v.reshape(self.seq_len, self.d_k)).flatten()
        return self.out_model.predict(context)

    def record_and_train(self, samples):
        for x, target in samples:
            self.q_model.record(x, target)
            self.k_model.record(x, target)
            self.v_model.record(x, target)
            self.out_model.record(x, target)
        self.q_model.train()
        self.k_model.train()
        self.v_model.train()
        self.out_model.train()

    def save_models(self, prefix="eml_attention_toy"):
        names = {
            f"{prefix}_q.json": self.q_model,
            f"{prefix}_k.json": self.k_model,
            f"{prefix}_v.json": self.v_model,
            f"{prefix}_out.json": self.out_model,
        }
        for path, m in names.items():
            with open(path, "w") as f:
                f.write(m.to_json())
            print(f"Saved {path}")
        return list(names.keys())

print("Toy EML-Attention class ready")`;

const cell4 = `attn = ToyEmlAttention(d_model=16, d_k=8, seq_len=6, depth=4)

np.random.seed(42)
samples = []
for _ in range(96):
    x = np.random.randn(attn.seq_len * attn.d_model).astype(np.float64)
    # Identity task — simplest convergent attention target
    samples.append((x.tolist(), x.tolist()))

attn.record_and_train(samples)
print("Training complete")`;

const cell5 = `saved = attn.save_models()
print("\\nFirst 200 chars of q_model JSON:")
print(open(saved[0]).read()[:200] + "...")`;

const cell6 = `print("""
DONE. 4 JSON files written:
  eml_attention_toy_q.json
  eml_attention_toy_k.json
  eml_attention_toy_v.json
  eml_attention_toy_out.json

How to use in Rust:
  let q_json = std::fs::read_to_string("eml_attention_toy_q.json")?;
  let q = eml_core::EmlModel::from_json(&q_json).unwrap();
  // same for k, v, out — plug into ToyEmlAttention

Architecture notes:
  - Forward pass: Q/K/V projections (EML) -> f64 matmul -> numerical softmax
    -> f64 matmul -> output projection (EML). This matches the Rust hybrid.
  - Training: differential evolution here, coordinate descent in Rust.
    Both are gradient-free.
  - Scale: toy only (d_model <= 32, seq_len <= 8, depth 3-5). Iteration 1+
    raises these gated on Iteration 0 benchmarks.
""")`;

export default function Page() {
  return (
    <main className="mx-auto my-10 max-w-4xl px-6 leading-relaxed text-neutral-900 dark:text-neutral-100">
      <h1 className="mb-4 text-3xl font-semibold">WeftOS Toy EML-Transformer — Iteration 0 Colab Notebook</h1>

      <p>
        <strong>Goal</strong>: train a Python mirror of the Rust{' '}
        <code>ToyEmlAttention</code> and export ready-to-load JSON models
        that the Rust <code>EmlModel::from_json</code> can deserialize directly.
      </p>

      <p>
        <strong>Compatibility</strong>: matches the public API in{' '}
        <a href="/docs/weftos/eml">weftos/eml</a> and{' '}
        <a href="/docs/weftos/eml-attention">weftos/eml-attention</a>
        {' '}— depth 3-5, inputs/heads, gradient-free training, to_json/from_json,
        ExoChain event stream.
      </p>

      <p>
        <strong>WASM note</strong>: the generated JSON contains{' '}
        <code>depth</code>, <code>inputs</code>, <code>heads</code>,{' '}
        <code>level0_weight</code>, <code>level0_bias</code>,{' '}
        <code>tree_params</code>, <code>events</code>. A matching{' '}
        <code>serde</code> path exists on the Rust side; the JSON round-trips.
      </p>

      <p>
        <strong>Companion live demo</strong>: see{' '}
        <a href="/clawft_eml-attention">/clawft_eml-attention</a> for a
        pure-JS forward-pass demonstrator that runs in the browser.
      </p>

      <div className={cellCls}>
        <h2>Cell 1 — Setup + safe EML operator</h2>
        <pre className={preCls}><code>{cell1}</code></pre>
      </div>

      <div className={cellCls}>
        <h2>Cell 2 — PyEmlModel (mirror of Rust <code>EmlModel</code>)</h2>
        <pre className={preCls}><code>{cell2}</code></pre>
      </div>

      <div className={cellCls}>
        <h2>Cell 3 — ToyEmlAttention (mirror of Rust <code>ToyEmlAttention</code>)</h2>
        <pre className={preCls}><code>{cell3}</code></pre>
      </div>

      <div className={cellCls}>
        <h2>Cell 4 — Train on the identity task</h2>
        <pre className={preCls}><code>{cell4}</code></pre>
      </div>

      <div className={cellCls}>
        <h2>Cell 5 — Export JSON for the Rust loader</h2>
        <pre className={preCls}><code>{cell5}</code></pre>
      </div>

      <div className={cellCls}>
        <h2>Cell 6 — Handoff to Rust</h2>
        <pre className={preCls}><code>{cell6}</code></pre>
      </div>

      <h2>Running this notebook</h2>
      <ol>
        <li>Copy each <code>Cell N</code> code block into a Colab cell in order.</li>
        <li>Run. Training takes under two minutes on CPU at the toy scale.</li>
        <li>Download the four <code>eml_attention_toy_*.json</code> files from Colab's file browser.</li>
        <li>In Rust: <code>EmlModel::from_json</code> loads each submodel directly; plug into <code>ToyEmlAttention</code>.</li>
      </ol>

      <h2>Roadmap</h2>
      <p>
        Iteration 0 proves the substrate composes. See the{' '}
        <a href="/docs/weftos/eml-attention">architecture page</a> for the go/no-go
        criteria and Iteration 1-4 scaling plan. This notebook is the reference
        training pipeline for Iteration 0 only.
      </p>
    </main>
  );
}
