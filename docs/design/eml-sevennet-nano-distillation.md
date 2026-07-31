# Design: EML knowledge distillation (SevenNet-Nano) — KG-017 / WEFT-356

**Status:** Implemented in `eml-core`  
**Date:** 2026-07-31  
**Branch:** `feat/weft-356-distill-eml`  
**Paper:** Oh et al., *SevenNet-Nano*, arXiv:2604.10887  
**Survey:** `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` (Paper 3)

## Problem

Depth-4 EML models (e.g. coherence: 50 params, multi-head) are the host
default for accuracy against Lanczos / graph ground truth. On WASM, ESP32,
and T0–T1 edge profiles, inference budget and binary size favor a shallower
tree. Cold-starting a depth-2 model on sparse edge labels underfits;
**knowledge distillation** transfers a host-trained teacher into a compact
student offline.

## Mapping from SevenNet-Nano

| Paper concept | WeftOS mapping |
|---------------|----------------|
| Teacher GNN (SevenNet-Omni) | Depth-4+ [`EmlModel`](../../crates/eml-core/src/model.rs) trained on graph features / embeddings / λ₂ |
| Student Nano | Depth-2 [`SevenNetNano`](../../crates/eml-core/src/distill.rs) |
| Soft targets from teacher | Synthetic inputs → `teacher.predict` → student `record` / `train` |
| Edge deployment | JSON weights + `EdgeEmlRuntime` selection |

GNN layers themselves are **out of scope** (domain-specific to atomistics).
Only the distillation paradigm is adopted.

## Pipeline (offline)

```text
 teacher (depth 4)          synthetic domain [lo, hi]
        │                            │
        │     LCG sample inputs ─────┘
        ▼
 teacher.predict(x)  ── soft targets ──►  student (depth 2).record
                                              │
                                              ▼
                                         student.train()
                                              │
                                              ▼
                                    DistillReport + SevenNetNano JSON
```

1. Draw `num_samples` (default 500) inputs from a uniform domain (default \[0, 1\]).
2. Query the teacher for multi-head soft targets (no raw labels required).
3. Train the student with existing coordinate descent (no autodiff, no Python).
4. Evaluate held-out fidelity → `DistillReport` (MAE, MSE, fidelity, compression).

Deterministic LCG (no `rand` crate) keeps offline runs reproducible and
WASM-safe.

## Edge runtime path

| API | Behavior |
|-----|----------|
| `SevenNetNano::from_teacher` | Offline distill + package |
| `SevenNetNano::from_json` | Load pre-distilled weights (envelope or bare `EmlModel` JSON) |
| `SevenNetNano::predict` | Inference only; params frozen after distill |
| `EdgeEmlRuntime::select` | On `wasm32`, prefer Nano when provided |
| `EdgeEmlRuntime::select_with_flag` | Runtime prefer-nano for ESP32 / firmware flags |
| `EdgeEmlRuntime::edge` / `host` | Force student / teacher |

Host training continues on the teacher; edge devices load Nano weights only.

## Metrics (“recall delta”)

For edge AC documentation:

- **`recall_delta`**: mean absolute error of student vs teacher on held-out
  synthetic points (alias of MAE in this pure-regression setting).
- **`fidelity`**: `1 - mae / (teacher_range + ε)` ∈ \[0, 1\].
- **`compression_ratio`**: `teacher_params / student_params`.

Depth-4 single-head: 46 params → depth-2: 26 params (~1.77×). Multi-head
heads add 2 params each on both models; relative compression is similar.

Exact numeric fidelity is **task-dependent** (teacher smoothness, domain
coverage, sample count). Unit tests assert mechanism correctness
(compression, finite outputs, freeze, roundtrip) and a loose MAE bound,
not a fixed 95% accuracy claim.

## Files

| Path | Role |
|------|------|
| `crates/eml-core/src/distill.rs` | Pipeline, `SevenNetNano`, `EdgeEmlRuntime`, tests |
| `crates/eml-core/src/lib.rs` | Re-exports |
| `crates/eml-core/src/model.rs` | `EmlModel::distill` delegates to free `distill` |
| `docs/src/content/docs/weftos/eml.mdx` | Public docs |

## Deferred

- Wire `EdgeEmlRuntime` into `EmlCoherenceModel` / HNSW-EML wrappers as a
  first-class dual-weight load path (host keeps depth-4; wasm crate loads Nano).
- Persist Nano weights next to teacher in CAS / OPFS for browser (WS16).
- Optional graph-feature sampler that walks real `GraphFeatures` /
  embedding rows instead of pure synthetic LCG (when batch exporters exist).
- Hard latency microbench on wasm32 (pair with WS16 browser budgets).

## Acceptance (WEFT-356)

| Criterion | Status |
|-----------|--------|
| Offline pipeline producing depth-2 weights | Done (`distill` / `SevenNetNano::from_teacher`) |
| Runtime path loading depth-2 on edge target | Done (`from_json`, `EdgeEmlRuntime`) |
| Recall delta vs depth-4 documented | Done (`DistillReport::recall_delta`, this note) |
| WASM target tested | Done (`cargo check -p eml-core --target wasm32-unknown-unknown`) |
| Pure Rust, synthetic train/eval + freeze tests | Done |
