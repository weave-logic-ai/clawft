//! Knowledge distillation for edge EML — SevenNet-Nano (KG-017 / WEFT-356).
//!
//! Inspired by SevenNet-Nano (Oh et al., arXiv:2604.10887): a large teacher
//! is compressed into a compact student that learns from teacher predictions
//! rather than raw labels. In WeftOS this maps to:
//!
//! - **Teacher**: depth-4 (or deeper) [`EmlModel`] trained on ground truth
//!   (graph features, embeddings, Lanczos λ₂, etc.)
//! - **Student (SevenNet-Nano)**: depth-2 EML with fewer parameters for
//!   WASM / ESP32 / T0–T1 edge profiles
//!
//! # Offline pipeline
//!
//! 1. Sample synthetic inputs in a configurable domain (default \[0, 1\]).
//! 2. Query the teacher for soft targets.
//! 3. Train the student via coordinate descent on those targets.
//! 4. Report MAE / MSE / fidelity / compression vs the teacher.
//!
//! # Edge runtime
//!
//! Load pre-distilled depth-2 weights with [`SevenNetNano::from_json`] or
//! select teacher vs nano with [`EdgeEmlRuntime::select`].

use serde::{Deserialize, Serialize};

use crate::EmlModel;

/// Canonical student depth for SevenNet-Nano edge deployment.
pub const SEVENNET_NANO_DEPTH: usize = 2;

/// Default number of synthetic teacher queries for offline distillation.
pub const DEFAULT_DISTILL_SAMPLES: usize = 500;

/// Default evaluation sample count for fidelity reporting.
pub const DEFAULT_EVAL_POINTS: usize = 200;

/// Default LCG seed for reproducible synthetic sampling.
pub const DEFAULT_DISTILL_SEED: u64 = 0xCAFE_BABE_1234_5678;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for offline teacher → student distillation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillConfig {
    /// Student tree depth (must be `<` teacher depth; Nano default = 2).
    pub target_depth: usize,
    /// Synthetic samples drawn from the input domain for training.
    pub num_samples: usize,
    /// Inclusive lower bound of the uniform input domain.
    pub input_lo: f64,
    /// Exclusive-ish upper bound of the uniform input domain (mapped via LCG).
    pub input_hi: f64,
    /// LCG seed for synthetic sampling (reproducible offline runs).
    pub seed: u64,
    /// Held-out synthetic points used only for the fidelity report.
    pub eval_points: usize,
}

impl Default for DistillConfig {
    fn default() -> Self {
        Self {
            target_depth: SEVENNET_NANO_DEPTH,
            num_samples: DEFAULT_DISTILL_SAMPLES,
            input_lo: 0.0,
            input_hi: 1.0,
            seed: DEFAULT_DISTILL_SEED,
            eval_points: DEFAULT_EVAL_POINTS,
        }
    }
}

impl DistillConfig {
    /// Nano preset: depth-2 student, default sample budget.
    pub fn nano() -> Self {
        Self::default()
    }

    /// Builder: override sample count.
    pub fn with_samples(mut self, n: usize) -> Self {
        self.num_samples = n;
        self
    }

    /// Builder: override student depth.
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.target_depth = depth;
        self
    }

    /// Builder: override seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Builder: override input domain.
    pub fn with_domain(mut self, lo: f64, hi: f64) -> Self {
        self.input_lo = lo;
        self.input_hi = hi;
        self
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Metrics from an offline distillation run (teacher vs student fidelity).
///
/// "Recall delta" is framed as the mean absolute deviation of the student
/// from the teacher on a held-out synthetic set — the quantity to document
/// when comparing depth-4 vs depth-2 for edge deployment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillReport {
    pub teacher_depth: usize,
    pub student_depth: usize,
    pub teacher_params: usize,
    pub student_params: usize,
    /// `teacher_params / student_params` (higher = more compression).
    pub compression_ratio: f64,
    pub train_samples: usize,
    pub eval_points: usize,
    /// Mean absolute error (student vs teacher) on eval set.
    pub mae: f64,
    /// Mean squared error on eval set.
    pub mse: f64,
    /// Max absolute error on eval set.
    pub max_abs_err: f64,
    /// Approximate teacher output range on the eval set (max − min primary).
    pub teacher_range: f64,
    /// Fidelity in \[0, 1\]: `1 - mae / (teacher_range + ε)`. Higher is better.
    pub fidelity: f64,
    /// Mean absolute error relative to teacher magnitude — the documented
    /// "recall delta" vs the depth-4 teacher (lower is better).
    pub recall_delta: f64,
    /// Whether student coordinate descent reported convergence (MSE < 0.01).
    pub student_trained: bool,
}

impl DistillReport {
    /// Human-readable one-liner for design notes / CI logs.
    pub fn summary(&self) -> String {
        format!(
            "SevenNet-Nano d{}→d{}: params {}→{} ({:.2}x), MAE={:.4}, MSE={:.4}, \
             fidelity={:.3}, recall_delta={:.4}, trained={}",
            self.teacher_depth,
            self.student_depth,
            self.teacher_params,
            self.student_params,
            self.compression_ratio,
            self.mae,
            self.mse,
            self.fidelity,
            self.recall_delta,
            self.student_trained
        )
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from edge-runtime construction / loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeEmlError {
    /// Student depth must be shallower than the teacher.
    DepthNotShallower { teacher: usize, student: usize },
    /// Expected Nano depth (2) but got another.
    NotNanoDepth { got: usize },
    /// JSON deserialize failed.
    InvalidJson,
    /// Input dimension mismatch.
    InputMismatch { expected: usize, got: usize },
}

impl core::fmt::Display for EdgeEmlError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DepthNotShallower { teacher, student } => write!(
                f,
                "student depth ({student}) must be less than teacher depth ({teacher})"
            ),
            Self::NotNanoDepth { got } => {
                write!(f, "SevenNet-Nano expects depth {SEVENNET_NANO_DEPTH}, got {got}")
            }
            Self::InvalidJson => write!(f, "invalid EML / SevenNet-Nano JSON"),
            Self::InputMismatch { expected, got } => {
                write!(f, "expected {expected} inputs, got {got}")
            }
        }
    }
}

impl std::error::Error for EdgeEmlError {}

// ---------------------------------------------------------------------------
// Sampling helpers (no `rand` dependency)
// ---------------------------------------------------------------------------

/// Deterministic LCG used for synthetic distillation inputs.
#[derive(Debug, Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Uniform in \[0, 1).
    fn next_unit(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 33) as f64 / (1u64 << 31) as f64
    }

    fn next_in_range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_unit() * (hi - lo)
    }
}

fn sample_inputs(rng: &mut Lcg, input_count: usize, lo: f64, hi: f64) -> Vec<f64> {
    (0..input_count)
        .map(|_| rng.next_in_range(lo, hi))
        .collect()
}

// ---------------------------------------------------------------------------
// Free distill API (matches docs/src/.../eml.mdx)
// ---------------------------------------------------------------------------

/// Distill `teacher` into a shallower student (default path for KG-017).
///
/// Creates a new [`EmlModel`] with `target_depth` and trains it to mimic
/// teacher outputs on `num_samples` synthetic inputs drawn from \[0, 1\].
///
/// # Panics
/// Panics if `target_depth >= teacher.depth()` or depth is outside {2..=5}.
///
/// # Example
/// ```
/// use eml_core::{EmlModel, distill};
///
/// let mut teacher = EmlModel::new(4, 3, 1);
/// for i in 0..80 {
///     let x = [i as f64 / 80.0, 0.5, 0.25];
///     let y = x[0] + x[1] * 0.5;
///     teacher.record(&x, &[Some(y)]);
/// }
/// teacher.train();
///
/// let student = distill(&teacher, 2, 200);
/// assert_eq!(student.depth(), 2);
/// assert!(student.param_count() < teacher.param_count());
/// ```
pub fn distill(teacher: &EmlModel, target_depth: usize, num_samples: usize) -> EmlModel {
    let (student, _report) = distill_with_report(
        teacher,
        &DistillConfig {
            target_depth,
            num_samples,
            ..DistillConfig::default()
        },
    );
    student
}

/// Offline distillation pipeline with fidelity metrics.
///
/// Returns the trained student and a [`DistillReport`] suitable for design
/// notes and CI assertions (compression, MAE, recall delta).
///
/// # Panics
/// Panics if `config.target_depth >= teacher.depth()` or depth not in {2..=5}.
pub fn distill_with_report(teacher: &EmlModel, config: &DistillConfig) -> (EmlModel, DistillReport) {
    assert!(
        config.target_depth < teacher.depth(),
        "student depth ({}) must be less than teacher depth ({})",
        config.target_depth,
        teacher.depth()
    );

    let mut student = EmlModel::new(
        config.target_depth,
        teacher.input_count(),
        teacher.head_count(),
    );
    student.set_model_name(format!(
        "sevennet-nano-d{}-from-d{}",
        config.target_depth,
        teacher.depth()
    ));

    let mut rng = Lcg::new(config.seed);
    let n = config.num_samples.max(50);
    for _ in 0..n {
        let inputs = sample_inputs(&mut rng, teacher.input_count(), config.input_lo, config.input_hi);
        let teacher_out = teacher.predict(&inputs);
        let targets: Vec<Option<f64>> = teacher_out.into_iter().map(Some).collect();
        student.record(&inputs, &targets);
    }

    let student_trained = student.train();

    let report = evaluate_fidelity(
        teacher,
        &student,
        config.eval_points.max(20),
        config.seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
        config.input_lo,
        config.input_hi,
        n,
        student_trained,
    );

    (student, report)
}

/// Compare student vs teacher on held-out synthetic points.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_fidelity(
    teacher: &EmlModel,
    student: &EmlModel,
    eval_points: usize,
    seed: u64,
    input_lo: f64,
    input_hi: f64,
    train_samples: usize,
    student_trained: bool,
) -> DistillReport {
    assert_eq!(
        teacher.input_count(),
        student.input_count(),
        "teacher/student input_count mismatch"
    );
    assert_eq!(
        teacher.head_count(),
        student.head_count(),
        "teacher/student head_count mismatch"
    );

    let mut rng = Lcg::new(seed);
    let mut sum_abs = 0.0;
    let mut sum_sq = 0.0;
    let mut max_abs = 0.0;
    let mut t_min = f64::INFINITY;
    let mut t_max = f64::NEG_INFINITY;
    let mut count = 0usize;

    for _ in 0..eval_points {
        let inputs = sample_inputs(&mut rng, teacher.input_count(), input_lo, input_hi);
        let t = teacher.predict(&inputs);
        let s = student.predict(&inputs);
        for (tv, sv) in t.iter().zip(s.iter()) {
            let err = (tv - sv).abs();
            sum_abs += err;
            sum_sq += err * err;
            if err > max_abs {
                max_abs = err;
            }
            count += 1;
        }
        // Track primary-head range for fidelity scaling.
        if let Some(&tp) = t.first() {
            if tp < t_min {
                t_min = tp;
            }
            if tp > t_max {
                t_max = tp;
            }
        }
    }

    let mae = if count > 0 {
        sum_abs / count as f64
    } else {
        0.0
    };
    let mse = if count > 0 {
        sum_sq / count as f64
    } else {
        0.0
    };
    let teacher_range = if t_max.is_finite() && t_min.is_finite() {
        (t_max - t_min).max(0.0)
    } else {
        0.0
    };
    let fidelity = (1.0 - mae / (teacher_range + 1e-6)).clamp(0.0, 1.0);
    let teacher_params = teacher.param_count();
    let student_params = student.param_count();
    let compression_ratio = if student_params > 0 {
        teacher_params as f64 / student_params as f64
    } else {
        0.0
    };

    DistillReport {
        teacher_depth: teacher.depth(),
        student_depth: student.depth(),
        teacher_params,
        student_params,
        compression_ratio,
        train_samples,
        eval_points,
        mae,
        mse,
        max_abs_err: max_abs,
        teacher_range,
        fidelity,
        recall_delta: mae,
        student_trained,
    }
}

// ---------------------------------------------------------------------------
// SevenNet-Nano — edge student surface
// ---------------------------------------------------------------------------

/// Compact depth-2 EML student for edge / WASM / ESP32 deployment.
///
/// Produced offline via [`SevenNetNano::from_teacher`] (or free
/// [`distill`]) and loaded at runtime via [`SevenNetNano::from_json`].
/// Parameters are frozen after distillation — inference only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SevenNetNano {
    /// Underlying depth-2 (or configured) student model.
    model: EmlModel,
    /// Teacher depth recorded at distillation time (if known).
    #[serde(default)]
    teacher_depth: Option<usize>,
    /// Optional last fidelity snapshot (for ops / debugging).
    #[serde(default)]
    last_report: Option<DistillReport>,
}

impl SevenNetNano {
    /// Canonical Nano depth.
    pub const DEPTH: usize = SEVENNET_NANO_DEPTH;

    /// Distill a teacher into a SevenNet-Nano student (offline pipeline).
    pub fn from_teacher(teacher: &EmlModel, config: DistillConfig) -> (Self, DistillReport) {
        let (model, report) = distill_with_report(teacher, &config);
        let nano = Self {
            model,
            teacher_depth: Some(teacher.depth()),
            last_report: Some(report.clone()),
        };
        (nano, report)
    }

    /// Wrap an already-trained student; enforces Nano depth by default.
    pub fn from_model(model: EmlModel) -> Result<Self, EdgeEmlError> {
        if model.depth() != SEVENNET_NANO_DEPTH {
            return Err(EdgeEmlError::NotNanoDepth {
                got: model.depth(),
            });
        }
        Ok(Self {
            model,
            teacher_depth: None,
            last_report: None,
        })
    }

    /// Wrap any shallower student without enforcing depth == 2.
    pub fn from_student(model: EmlModel, teacher_depth: Option<usize>) -> Result<Self, EdgeEmlError> {
        if let Some(td) = teacher_depth {
            if model.depth() >= td {
                return Err(EdgeEmlError::DepthNotShallower {
                    teacher: td,
                    student: model.depth(),
                });
            }
        }
        Ok(Self {
            model,
            teacher_depth,
            last_report: None,
        })
    }

    /// Load pre-distilled weights (edge runtime path).
    pub fn from_json(json: &str) -> Result<Self, EdgeEmlError> {
        // Accept either a full SevenNetNano envelope or a bare EmlModel JSON.
        if let Ok(nano) = serde_json::from_str::<SevenNetNano>(json) {
            if nano.model.depth() != SEVENNET_NANO_DEPTH {
                return Err(EdgeEmlError::NotNanoDepth {
                    got: nano.model.depth(),
                });
            }
            return Ok(nano);
        }
        let model = EmlModel::from_json(json).ok_or(EdgeEmlError::InvalidJson)?;
        Self::from_model(model)
    }

    /// Serialize for edge packaging / CAS / OPFS.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("SevenNetNano serialization should not fail")
    }

    /// Multi-head prediction.
    pub fn predict(&self, inputs: &[f64]) -> Result<Vec<f64>, EdgeEmlError> {
        if inputs.len() != self.model.input_count() {
            return Err(EdgeEmlError::InputMismatch {
                expected: self.model.input_count(),
                got: inputs.len(),
            });
        }
        Ok(self.model.predict(inputs))
    }

    /// Primary-head prediction.
    pub fn predict_primary(&self, inputs: &[f64]) -> Result<f64, EdgeEmlError> {
        Ok(self.predict(inputs)?[0])
    }

    /// Borrow the underlying model (read-only edge path).
    pub fn model(&self) -> &EmlModel {
        &self.model
    }

    pub fn depth(&self) -> usize {
        self.model.depth()
    }

    pub fn param_count(&self) -> usize {
        self.model.param_count()
    }

    pub fn input_count(&self) -> usize {
        self.model.input_count()
    }

    pub fn head_count(&self) -> usize {
        self.model.head_count()
    }

    pub fn teacher_depth(&self) -> Option<usize> {
        self.teacher_depth
    }

    pub fn last_report(&self) -> Option<&DistillReport> {
        self.last_report.as_ref()
    }

    /// Snapshot of current params (for freeze-step tests).
    pub fn params_snapshot(&self) -> Vec<f64> {
        self.model.params_slice().to_vec()
    }

    /// True when compiling for wasm32 (browser or WASI) — edge profile.
    #[inline]
    pub fn is_edge_compile_target() -> bool {
        cfg!(target_arch = "wasm32")
    }
}

// ---------------------------------------------------------------------------
// Edge runtime selection
// ---------------------------------------------------------------------------

/// Runtime that prefers a Nano student on edge targets when available.
#[derive(Debug, Clone)]
pub enum EdgeEmlRuntime {
    /// Full-fidelity teacher (host / server).
    Teacher(EmlModel),
    /// Compressed SevenNet-Nano student (WASM / ESP32 / T0–T1).
    Nano(SevenNetNano),
}

impl EdgeEmlRuntime {
    /// Select Nano on wasm32 when provided; otherwise keep the teacher.
    ///
    /// Hosts that always want the student can call [`Self::edge`].
    /// Non-wasm edge profiles (ESP32 firmware flags) should use
    /// [`Self::select_with_flag`].
    pub fn select(teacher: EmlModel, nano: Option<SevenNetNano>) -> Self {
        if SevenNetNano::is_edge_compile_target() {
            if let Some(n) = nano {
                return Self::Nano(n);
            }
        }
        Self::Teacher(teacher)
    }

    /// Force the edge (Nano) path regardless of target architecture.
    pub fn edge(nano: SevenNetNano) -> Self {
        Self::Nano(nano)
    }

    /// Force the teacher path.
    pub fn host(teacher: EmlModel) -> Self {
        Self::Teacher(teacher)
    }

    /// Prefer Nano when `prefer_nano` is true (runtime override for edge
    /// device profiles that are not wasm32, e.g. ESP32 firmware flags).
    pub fn select_with_flag(teacher: EmlModel, nano: Option<SevenNetNano>, prefer_nano: bool) -> Self {
        if prefer_nano {
            if let Some(n) = nano {
                return Self::Nano(n);
            }
        }
        Self::select(teacher, nano)
    }

    pub fn is_nano(&self) -> bool {
        matches!(self, Self::Nano(_))
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Teacher(t) => t.depth(),
            Self::Nano(n) => n.depth(),
        }
    }

    pub fn param_count(&self) -> usize {
        match self {
            Self::Teacher(t) => t.param_count(),
            Self::Nano(n) => n.param_count(),
        }
    }

    pub fn predict(&self, inputs: &[f64]) -> Vec<f64> {
        match self {
            Self::Teacher(t) => t.predict(inputs),
            Self::Nano(n) => n.model().predict(inputs),
        }
    }

    pub fn predict_primary(&self, inputs: &[f64]) -> f64 {
        self.predict(inputs)[0]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn trained_teacher(depth: usize, inputs: usize, heads: usize) -> EmlModel {
        let mut teacher = EmlModel::new(depth, inputs, heads);
        for i in 0..120 {
            let x: Vec<f64> = (0..inputs)
                .map(|d| ((i + d * 17) as f64 / 120.0).fract())
                .collect();
            let primary = x.iter().sum::<f64>() / inputs as f64;
            let targets: Vec<Option<f64>> = (0..heads)
                .map(|h| Some(primary * (1.0 + 0.1 * h as f64)))
                .collect();
            teacher.record(&x, &targets);
        }
        teacher.train();
        // Ensure non-zero function even if train did not fully converge.
        if !teacher.is_trained() {
            for (i, p) in teacher.params_slice_mut().iter_mut().enumerate() {
                *p = ((i as f64) * 0.37).sin() * 0.5;
            }
            teacher.mark_trained(true);
        }
        teacher
    }

    #[test]
    fn free_distill_depth_4_to_2() {
        let teacher = trained_teacher(4, 3, 1);
        let student = distill(&teacher, 2, 300);
        assert_eq!(student.depth(), 2);
        assert_eq!(student.input_count(), 3);
        assert!(student.param_count() < teacher.param_count());
        let y = student.predict_primary(&[0.2, 0.4, 0.6]);
        assert!(y.is_finite());
    }

    #[test]
    fn distill_with_report_metrics() {
        let teacher = trained_teacher(4, 2, 1);
        let config = DistillConfig::nano().with_samples(400).with_seed(42);
        let (student, report) = distill_with_report(&teacher, &config);

        assert_eq!(report.teacher_depth, 4);
        assert_eq!(report.student_depth, 2);
        assert_eq!(student.depth(), 2);
        assert!(report.compression_ratio > 1.0);
        assert!(report.mae.is_finite());
        assert!(report.mse.is_finite());
        assert!(report.fidelity >= 0.0 && report.fidelity <= 1.0);
        assert_eq!(report.recall_delta, report.mae);
        // Mechanism sanity: MAE must stay bounded (not explode).
        assert!(
            report.mae < 100.0,
            "MAE too large: {} ({})",
            report.mae,
            report.summary()
        );
        assert!(!report.summary().is_empty());
    }

    #[test]
    fn sevennet_nano_from_teacher_and_json_roundtrip() {
        let teacher = trained_teacher(4, 2, 2);
        let (nano, report) = SevenNetNano::from_teacher(&teacher, DistillConfig::nano().with_samples(250));
        assert_eq!(nano.depth(), 2);
        assert_eq!(nano.teacher_depth(), Some(4));
        assert!(nano.last_report().is_some());
        assert!(report.student_params < report.teacher_params);

        let json = nano.to_json();
        let loaded = SevenNetNano::from_json(&json).expect("roundtrip");
        assert_eq!(loaded.depth(), 2);
        assert_eq!(loaded.param_count(), nano.param_count());

        let a = nano.predict(&[0.3, 0.7]).unwrap();
        let b = loaded.predict(&[0.3, 0.7]).unwrap();
        assert_eq!(a.len(), 2);
        for (x, y) in a.iter().zip(b.iter()) {
            assert!((x - y).abs() < 1e-12);
        }
    }

    #[test]
    fn sevennet_nano_from_bare_eml_json() {
        let teacher = trained_teacher(4, 2, 1);
        let student = distill(&teacher, 2, 200);
        let json = student.to_json();
        let nano = SevenNetNano::from_json(&json).expect("bare model json");
        assert_eq!(nano.depth(), 2);
    }

    #[test]
    fn from_model_rejects_non_nano_depth() {
        let m = EmlModel::new(3, 2, 1);
        let err = SevenNetNano::from_model(m).unwrap_err();
        assert_eq!(err, EdgeEmlError::NotNanoDepth { got: 3 });
    }

    #[test]
    fn freeze_step_inference_does_not_mutate_params() {
        let teacher = trained_teacher(4, 2, 1);
        let (nano, _) = SevenNetNano::from_teacher(&teacher, DistillConfig::nano().with_samples(200));
        let before = nano.params_snapshot();
        for i in 0..50 {
            let x = [i as f64 / 50.0, 1.0 - i as f64 / 50.0];
            let _ = nano.predict(&x).unwrap();
        }
        let after = nano.params_snapshot();
        assert_eq!(before, after, "inference must freeze student params");
    }

    #[test]
    fn freeze_step_teacher_unchanged_by_distill() {
        let teacher = trained_teacher(4, 2, 1);
        let before = teacher.params_slice().to_vec();
        let _ = distill(&teacher, 2, 150);
        let after = teacher.params_slice().to_vec();
        assert_eq!(before, after, "distill must not mutate teacher");
    }

    #[test]
    fn edge_runtime_force_nano_has_fewer_params() {
        let teacher = trained_teacher(4, 3, 1);
        let (nano, report) = SevenNetNano::from_teacher(&teacher, DistillConfig::nano().with_samples(200));
        let rt = EdgeEmlRuntime::edge(nano);
        assert!(rt.is_nano());
        assert_eq!(rt.depth(), 2);
        assert!(rt.param_count() < teacher.param_count());
        assert!(rt.predict_primary(&[0.1, 0.2, 0.3]).is_finite());
        assert!(report.compression_ratio > 1.0);
    }

    #[test]
    fn edge_runtime_select_with_flag() {
        let teacher = trained_teacher(4, 2, 1);
        let (nano, _) = SevenNetNano::from_teacher(&teacher, DistillConfig::nano().with_samples(150));
        let host = EdgeEmlRuntime::select_with_flag(teacher.clone(), Some(nano.clone()), false);
        assert!(!host.is_nano());
        assert_eq!(host.depth(), 4);

        let edge = EdgeEmlRuntime::select_with_flag(teacher, Some(nano), true);
        assert!(edge.is_nano());
        assert_eq!(edge.depth(), 2);
    }

    #[test]
    fn edge_runtime_host_keeps_teacher() {
        let teacher = trained_teacher(4, 2, 1);
        let rt = EdgeEmlRuntime::host(teacher);
        assert!(!rt.is_nano());
        assert_eq!(rt.depth(), 4);
    }

    #[test]
    fn compression_depth4_to_depth2_params() {
        // Single-head: depth-4 has 46 params, depth-2 has 26.
        let teacher = EmlModel::new(4, 7, 1);
        let student = EmlModel::new(2, 7, 1);
        assert_eq!(teacher.param_count(), 46);
        assert_eq!(student.param_count(), 26);
        assert!((teacher.param_count() as f64 / student.param_count() as f64) > 1.5);
    }

    #[test]
    #[should_panic(expected = "student depth")]
    fn distill_same_depth_panics() {
        let teacher = EmlModel::new(4, 3, 1);
        let _ = distill(&teacher, 4, 50);
    }

    #[test]
    fn multi_head_distill_preserves_head_count() {
        // Use a param-seeded teacher (skip full CD) so this stays fast in CI.
        let mut teacher = EmlModel::new(4, 2, 3);
        for (i, p) in teacher.params_slice_mut().iter_mut().enumerate() {
            *p = ((i as f64) * 0.41).sin() * 0.4;
        }
        teacher.mark_trained(true);

        let (nano, report) = SevenNetNano::from_teacher(
            &teacher,
            DistillConfig::nano().with_samples(80),
        );
        assert_eq!(nano.head_count(), 3);
        let pred = nano.predict(&[0.5, 0.5]).unwrap();
        assert_eq!(pred.len(), 3);
        for v in pred {
            assert!(v.is_finite());
        }
        assert_eq!(report.student_depth, 2);
    }

    #[test]
    fn wasm_edge_flag_is_bool() {
        // Pure compile-time cfg; just ensure the API is callable on host.
        let _ = SevenNetNano::is_edge_compile_target();
        assert!(!SevenNetNano::is_edge_compile_target() || cfg!(target_arch = "wasm32"));
    }
}
