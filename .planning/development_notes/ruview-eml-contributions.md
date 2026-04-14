# RuView x EML Coherence: Contribution Analysis

Date: 2026-04-04

## 1. What is RuView?

**Repository**: `github.com/ruvnet/RuView`
**Description**: WiFi DensePose -- a platform that turns commodity WiFi signals
(CSI / Channel State Information) into real-time human pose estimation, vital
sign monitoring (breathing rate, heart rate), and presence detection. No cameras.

**Languages**: Python (v1 prototype), Rust (production port in `rust-port/wifi-densepose-rs/`)

### Rust Crate Structure (18 crates)

| Crate | Purpose |
|-------|---------|
| wifi-densepose-core | Error types, traits, shared types |
| wifi-densepose-signal | CSI preprocessing, phase sanitization, feature extraction, motion detection |
| wifi-densepose-nn | DensePose inference via ONNX, tensor ops |
| wifi-densepose-vitals | Breathing/heart rate extraction, anomaly detection (Welford z-score) |
| wifi-densepose-train | Training pipeline: losses, domain factorization, rapid adaptation, LoRA |
| wifi-densepose-ruvector | Integration bridge to ruvector (signal, viewpoint, CRV, mat) |
| wifi-densepose-mat | WiFi sensing mat hardware |
| ruv-neural | Neural network utilities |
| wifi-densepose-wasm | WASM compilation target |
| wifi-densepose-wasm-edge | Edge WASM deployment |
| wifi-densepose-api | REST API server |
| wifi-densepose-cli | CLI tool |
| wifi-densepose-config | Configuration management |
| wifi-densepose-db | Data storage |
| wifi-densepose-desktop | Desktop application |
| wifi-densepose-hardware | ESP32 hardware interface |
| wifi-densepose-sensing-server | Sensing data server |
| wifi-densepose-wifiscan | WiFi scanner |

### Key Algorithms

1. **CSI Feature Extraction** (`wifi-densepose-signal/src/features.rs`):
   Amplitude mean/variance, phase differences, correlation matrices, Doppler
   shift via FFT, power spectral density.

2. **Motion Detection** (`wifi-densepose-signal/src/motion.rs`):
   Weighted combination of variance/correlation/phase/Doppler components with
   hardcoded weights (0.3/0.2/0.2/0.3) and confidence scoring.

3. **Human Presence Detection** (`wifi-densepose-signal/src/motion.rs`):
   Temporal smoothing via EMA, binary threshold on smoothed confidence.

4. **Vital Sign Anomaly Detection** (`wifi-densepose-vitals/src/anomaly.rs`):
   Welford online statistics, z-score thresholds, hardcoded clinical ranges
   (apnea < 4 BPM, tachycardia > 100 BPM, etc.).

5. **Domain Adaptation** (`wifi-densepose-train/src/domain.rs`):
   Gradient reversal layer, domain factorizer, adversarial schedule for
   cross-environment generalization (MERIDIAN Phase 2).

6. **Rapid Adaptation** (`wifi-densepose-train/src/rapid_adapt.rs`):
   Few-shot test-time training with contrastive/entropy losses, produces LoRA
   weight deltas for new environments.

7. **Training Losses** (`wifi-densepose-train/src/losses.rs`):
   Keypoint heatmap MSE + DensePose cross-entropy + teacher-student transfer,
   combined with fixed lambda weights.

---

## 2. EML Contribution Opportunities in RuView

### 2.1 Learned Motion Score Weights

**File**: `wifi-densepose-signal/src/motion.rs` (lines 39-54)

**Current state**: Motion score is computed as a hardcoded weighted sum:
```
total = 0.3 * variance + 0.2 * correlation + 0.2 * phase + 0.3 * doppler
```

**EML contribution**: Replace the fixed linear combination with a depth-4 EML
tree (same architecture as WeftOS `eml_coherence.rs`, 50 params). The EML model
would take the four component scores as input features and predict the motion
score. Train via coordinate descent on labeled presence/absence data.

**Benefit**: The EML operator `exp(x) - ln(y)` can approximate nonlinear
interactions between components that a linear combination cannot. For example,
high Doppler + low variance might indicate a swinging arm (local motion) while
high Doppler + high variance indicates walking (whole-body motion). The learned
model would discover these interaction effects.

**PR scope**: ~200 lines. Add `eml_motion.rs` to `wifi-densepose-signal`, wire
it into `MotionDetector` as an alternative scoring mode behind a feature flag.

### 2.2 Adaptive Anomaly Thresholds

**File**: `wifi-densepose-vitals/src/anomaly.rs` (lines 93-216)

**Current state**: Clinical thresholds are hardcoded constants (apnea < 4 BPM,
tachypnea > 30 BPM, tachycardia > 100 BPM, bradycardia < 50 BPM). Z-score
threshold is a constructor parameter (default 2.5). Severity is computed with
fixed linear formulas like `((hr - 100.0) / 80.0).clamp(0.3, 1.0)`.

**EML contribution**: Two EML models:
1. **Severity scorer**: Takes (vital value, z-score, reading count, signal
   quality) as features, predicts severity [0,1]. Trained on clinician-labeled
   severity ratings. Replaces the hardcoded linear severity formulas.
2. **Threshold adapter**: Takes (running mean, running std, window size, signal
   quality) as features, predicts optimal z-score threshold. Trained on
   false-positive/false-negative feedback. Replaces the fixed 2.5 threshold.

**Benefit**: Clinical thresholds vary by patient demographics (athlete resting
HR ~45 is normal, elderly patient HR ~45 is concerning). An EML model trained on
per-patient data would learn personalized thresholds without manual tuning.

**PR scope**: ~300 lines. Add `eml_anomaly.rs` to `wifi-densepose-vitals`,
integrate behind a `learned-thresholds` feature flag.

### 2.3 Detection Confidence Scoring

**File**: `wifi-densepose-signal/src/motion.rs` (confidence calculation)
**File**: `v1/src/core/csi_processor.py` (lines 452-461)

**Current state**: Detection confidence combines three binary indicators with
fixed weights:
```
confidence = 0.4 * amplitude_indicator + 0.3 * phase_indicator + 0.3 * motion_indicator
```

**EML contribution**: Replace with an EML tree taking continuous feature values
(amplitude mean, phase std, motion score, SNR, subcarrier count, Doppler energy)
and predicting calibrated probability of human presence. The multi-head variant
could simultaneously output (probability, location-confidence, pose-readiness).

**Benefit**: Current approach loses information by binarizing indicators. EML
would operate on the full continuous feature space and learn nonlinear decision
boundaries. The uncertainty head would provide calibrated confidence intervals.

**PR scope**: ~250 lines. Extend `MotionDetector` with `EmlConfidenceScorer`.

### 2.4 Loss Weight Auto-Tuning

**File**: `wifi-densepose-train/src/losses.rs` (lines 49-66)

**Current state**: Training loss weights are fixed:
```
L_total = 0.3 * L_keypoint + 0.6 * L_densepose + 0.1 * L_transfer
```

**EML contribution**: An EML model that takes (epoch, keypoint_loss,
densepose_loss, transfer_loss, learning_rate, gradient_norm) as features and
predicts optimal loss weights for the next training step. This is a form of
learned curriculum -- the model discovers when to emphasize transfer learning
early and shift to pose refinement later.

**Benefit**: Manual loss weight tuning is one of the biggest pain points in
multi-task learning. EML coordinate descent is cheap enough to run every 100
steps during training.

**PR scope**: ~200 lines. Add `eml_scheduler.rs` to `wifi-densepose-train`.

### 2.5 Signal Quality Scoring

**File**: `wifi-densepose-signal/src/features.rs`

**Current state**: No unified signal quality metric. Individual features
(amplitude RMS, dynamic range, phase coherence) are extracted but not combined
into a single quality score.

**EML contribution**: A quality-scoring EML model taking all extracted features
as input and predicting a [0,1] signal quality score. Train on expert-labeled
CSI frames rated as "good"/"marginal"/"unusable". Use the quality score to:
- Gate downstream processing (skip pose estimation on low-quality frames)
- Weight subcarrier contributions in vital sign extraction
- Drive adaptive sampling rate (increase rate when quality drops)

**Benefit**: Currently the system processes all frames equally regardless of
signal quality. A learned quality gate would save compute and improve accuracy.

**PR scope**: ~200 lines. Add `eml_quality.rs` to `wifi-densepose-signal`.

---

## 3. Ruvector Crate EML Opportunities

### 3.1 ruvector-attention

**Location**: `/tmp/ruvector-latest/crates/ruvector-attention/`

**What it does**: Implements attention mechanisms: scaled dot-product,
multi-head, graph attention, hyperbolic attention, FlashAttention-3, MLA with
KV-cache compression, Mamba selective state spaces, speculative decoding. Also
includes sheaf attention (coherence-gated transformer per ADR-015).

**Where EML fits**: The attention score computation in
`attention::ScaledDotProductAttention` uses the standard
`softmax(QK^T / sqrt(d))` formula. An EML model could learn a
**domain-specific attention kernel** that replaces or augments the dot-product
with a learned similarity function. Specifically:

- Input features: (query norm, key norm, dot product, angular distance,
  Euclidean distance, dimension)
- Output: attention weight

This would be most valuable in the **sheaf attention** module where coherence
gating already involves learned scoring. The EML model could serve as the
coherence gate itself -- predicting whether two attention heads should
communicate.

**PR scope**: ~300 lines. Add `eml_kernel.rs` providing `EmlAttentionKernel`
implementing the `Attention` trait, usable as a drop-in alternative.

**Expected benefit**: 5-15% accuracy improvement on domain-specific tasks where
the standard dot-product attention is suboptimal (hierarchical data, temporal
sequences with non-uniform spacing like CSI frames).

### 3.2 ruvector-solver

**Location**: `/tmp/ruvector-latest/crates/ruvector-solver/`

**What it does**: Iterative sparse linear solvers (Neumann series, conjugate
gradient, forward/backward push, random walk, BMSSP). Solves `Ax = b` for
sparse CSR matrices.

**Where EML fits**: As a **preconditioner selector and parameter tuner**.
Currently each solver has fixed convergence parameters (tolerance, max
iterations). An EML model could:

1. **Solver selection**: Take matrix features (nnz, dimension, spectral radius
   estimate, diagonal dominance ratio) and predict which solver will converge
   fastest. The router module already exists (`solver/src/router.rs`) but likely
   uses heuristics.

2. **Convergence prediction**: Predict expected iteration count from matrix
   features, allowing early termination or solver switching if actual iterations
   exceed prediction by more than the uncertainty bound.

**PR scope**: ~250 lines. Add `eml_router.rs` to the solver crate, wire into
the existing router.

**Expected benefit**: 2-5x speedup on heterogeneous workloads by avoiding
slow solver choices. The EML model trains on the solver's own runtime data.

### 3.3 ruvector-hyperbolic-hnsw

**Location**: `/tmp/ruvector-latest/crates/ruvector-hyperbolic-hnsw/`

**What it does**: Hyperbolic (Poincare ball) embeddings with HNSW index.
Features tangent-space pruning, per-shard curvature, dual-space indexing.

**Where EML fits**: **Curvature prediction**. The per-shard curvature parameter
controls how tightly the hyperbolic space curves. Currently this appears to be
set manually or via simple heuristics. An EML model could:

- Input features: (shard depth, node count, avg pairwise distance, hierarchy
  depth, leaf ratio, branching factor)
- Output: optimal curvature parameter for the shard

Train on reconstruction loss: for each curvature setting, measure how well the
hyperbolic distances preserve the ground-truth hierarchy distances, then train
the EML model to predict the curvature that minimizes reconstruction error.

**PR scope**: ~200 lines. Add `eml_curvature.rs` to the crate.

**Expected benefit**: Automatic curvature tuning would eliminate a manual
hyperparameter while improving recall on deep leaves (the stated goal of
hyperbolic embeddings).

### 3.4 mcp-brain

**Location**: `/tmp/ruvector-latest/crates/mcp-brain/`

**What it does**: MCP server for shared knowledge across Claude Code sessions.
Stores knowledge as RVF cognitive containers with witness chains. Provides
semantic search, quality voting (Bayesian update), drift detection, and
cross-domain transfer.

**Where EML fits**: **Knowledge quality scoring**. The `brain_vote` tool uses
Bayesian updating for quality gating. An EML model could learn a more nuanced
quality predictor:

- Input features: (embedding similarity to query, vote count, mean vote score,
  age in hours, category match, tag overlap, content length, code snippet
  present)
- Output: predicted usefulness score

The EML model would train on actual usage data -- knowledge items that were
retrieved and led to successful outcomes get positive labels, items retrieved
but not used get negative labels.

**PR scope**: ~200 lines. Add `eml_quality.rs` to `mcp-brain/src/`.

**Expected benefit**: Better ranking of search results than pure embedding
similarity, especially for knowledge items with few votes.

---

## 4. Cross-Cutting Theme: EML as Universal Scoring Function

All of the above opportunities share a pattern: **replacing hardcoded
linear combinations or fixed thresholds with a learned O(1) function**.
The WeftOS EML implementation (`clawft-kernel/src/eml_coherence.rs`) already
demonstrates this pattern for graph coherence prediction.

A reusable `eml-core` crate could be extracted with:
- The `eml(x, y)` operator
- The depth-4 multi-head architecture (50 params)
- Coordinate descent training
- Serialization/deserialization for model persistence
- A `record(features, target)` / `train()` / `predict(features)` API

This crate would be a dependency for both RuView and ruvector, and would
also be the canonical WeftOS contribution to the ruvnet ecosystem.

### Prioritized Contribution Roadmap

| Priority | Target | Effort | Impact |
|----------|--------|--------|--------|
| P0 | Extract `eml-core` crate from WeftOS | 1 day | Enables all below |
| P1 | RuView motion score weights (2.1) | 1 day | Validates approach on real data |
| P1 | RuView anomaly thresholds (2.2) | 1 day | Clinical safety improvement |
| P2 | ruvector-solver router (3.2) | 1 day | Performance win, easy to measure |
| P2 | RuView signal quality (2.5) | 1 day | Compute savings |
| P3 | ruvector-attention kernel (3.1) | 2 days | Research-grade contribution |
| P3 | ruvector-hyperbolic-hnsw curvature (3.3) | 1 day | Eliminates hyperparameter |
| P3 | mcp-brain quality scoring (3.4) | 1 day | UX improvement |
| P4 | RuView loss weight tuning (2.4) | 1 day | Training improvement |
| P4 | RuView confidence scoring (2.3) | 1 day | Incremental accuracy gain |
