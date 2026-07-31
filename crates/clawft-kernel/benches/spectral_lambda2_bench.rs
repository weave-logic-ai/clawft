//! Spectral lambda₂ path comparison (WEFT-361 / KG-004).
//!
//! Compares three estimators of algebraic connectivity on synthetic
//! graphs at configurable scale:
//!
//! | Path | Complexity | Module |
//! |------|------------|--------|
//! | Lanczos | O(k·m) sparse Krylov | [`CausalGraph::spectral_analysis`] |
//! | RFF | O(m · features) | [`CausalGraph::spectral_analysis_rff`] |
//! | EML | O(1) features | [`EmlCoherenceModel::predict_lambda2`] |
//!
//! Hand-rolled (`harness = false`, plain `fn main()`) matching
//! `vector_backend_bench`: criterion's repeated-sampling model multiplies
//! the cost of building large causal graphs and does not fit a single
//! parameterized run that emits custom JSON (latency + relative error).
//!
//! ## Graph families
//!
//! - **cycle** (default): unit-weight cycle `C_n`. Analytical
//!   `lambda_2 = 2 (1 − cos(2π/n))` gives a ground-truth reference at
//!   every scale without a dense eigensolve.
//! - **path**: unit-weight path `P_n`, analytical
//!   `lambda_2 = 2 (1 − cos(π/n))`.
//! - **er**: sparse Erdős–Rényi-style ring-plus-chords (expected degree
//!   ~`WEFT_BENCH_AVG_DEG`, default 6). Reference = Lanczos only
//!   (no closed form).
//!
//! ## Running
//!
//! ```text
//! # Default: n ∈ {1_000, 10_000}; skip 100K (slow)
//! cargo bench -p clawft-kernel --bench spectral_lambda2_bench
//!
//! # Include 100K (optional; Lanczos may take a while)
//! WEFT_BENCH_SIZES=1000,10000,100000 cargo bench -p clawft-kernel \
//!     --bench spectral_lambda2_bench
//!
//! # Skip Lanczos at large n (report RFF/EML only + analytical ref)
//! WEFT_BENCH_SKIP_LANCZOS_ABOVE=5000 cargo bench -p clawft-kernel \
//!     --bench spectral_lambda2_bench
//! ```
//!
//! Env vars (all optional):
//! - `WEFT_BENCH_SIZES` — comma-separated node counts (default `1000,10000`)
//! - `WEFT_BENCH_FAMILY` — `cycle` | `path` | `er` (default `cycle`)
//! - `WEFT_BENCH_AVG_DEG` — target average degree for `er` (default `6`)
//! - `WEFT_BENCH_RFF_FEATURES` — RFF feature count (default `64`)
//! - `WEFT_BENCH_RFF_ITERS` — RFF Jacobi iters (default `50`)
//! - `WEFT_BENCH_LANCZOS_ITERS` — Lanczos k (default `50`)
//! - `WEFT_BENCH_SKIP_LANCZOS_ABOVE` — skip Lanczos when n exceeds this
//!   (default `20000`; set very high to always run)
//! - `WEFT_BENCH_WARMUP` — warmup runs discarded before timing (default `1`)
//! - `WEFT_BENCH_REPEATS` — timed repeats; median reported (default `3`)
//!
//! ## Decision rule (documented on `SpectralMethod`)
//!
//! - `n < 10_000` → Lanczos
//! - `10_000 ≤ n < 100_000` → RFF
//! - `n ≥ 100_000` → EML (tick-rate); re-anchor with RFF occasionally

use std::env;
use std::time::{Duration, Instant};

use serde::Serialize;

use clawft_kernel::eml_coherence::{EmlCoherenceModel, GraphFeatures};
use clawft_kernel::{
    select_spectral_method, CausalEdgeType, CausalGraph, SpectralMethod, SPECTRAL_EML_MIN_NODES,
    SPECTRAL_RFF_MIN_NODES,
};

// ── Config ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GraphFamily {
    Cycle,
    Path,
    Er,
}

impl GraphFamily {
    fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "path" => Self::Path,
            "er" | "erdos" | "erdos-renyi" => Self::Er,
            _ => Self::Cycle,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Cycle => "cycle",
            Self::Path => "path",
            Self::Er => "er",
        }
    }
}

struct BenchConfig {
    sizes: Vec<usize>,
    family: GraphFamily,
    avg_deg: usize,
    rff_features: usize,
    rff_iters: usize,
    lanczos_iters: usize,
    skip_lanczos_above: usize,
    warmup: usize,
    repeats: usize,
}

impl BenchConfig {
    fn from_env() -> Self {
        let sizes = env::var("WEFT_BENCH_SIZES")
            .ok()
            .map(|s| {
                s.split(',')
                    .filter_map(|p| p.trim().parse::<usize>().ok())
                    .filter(|&n| n >= 2)
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| vec![1_000, 10_000]);

        Self {
            sizes,
            family: GraphFamily::parse(
                &env::var("WEFT_BENCH_FAMILY").unwrap_or_else(|_| "cycle".into()),
            ),
            avg_deg: env::var("WEFT_BENCH_AVG_DEG")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(6)
                .max(2),
            rff_features: env::var("WEFT_BENCH_RFF_FEATURES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(64)
                .max(4),
            rff_iters: env::var("WEFT_BENCH_RFF_ITERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50)
                .max(10),
            lanczos_iters: env::var("WEFT_BENCH_LANCZOS_ITERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50)
                .max(5),
            skip_lanczos_above: env::var("WEFT_BENCH_SKIP_LANCZOS_ABOVE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20_000),
            warmup: env::var("WEFT_BENCH_WARMUP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
            repeats: env::var("WEFT_BENCH_REPEATS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3)
                .max(1),
        }
    }
}

// ── Synthetic graphs ─────────────────────────────────────────────────────

/// Deterministic LCG for ER chord selection (no rand dep required in bench).
fn lcg_next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed
}

fn build_graph(n: usize, family: GraphFamily, avg_deg: usize) -> CausalGraph {
    let g = CausalGraph::new();
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        ids.push(g.add_node(format!("n{i}"), serde_json::json!({})));
    }

    match family {
        GraphFamily::Cycle => {
            for i in 0..n {
                g.link(
                    ids[i],
                    ids[(i + 1) % n],
                    CausalEdgeType::Causes,
                    1.0,
                    0,
                    0,
                );
            }
        }
        GraphFamily::Path => {
            for i in 0..(n - 1) {
                g.link(ids[i], ids[i + 1], CausalEdgeType::Causes, 1.0, 0, 0);
            }
        }
        GraphFamily::Er => {
            // Backbone path keeps the graph connected.
            for i in 0..(n - 1) {
                g.link(ids[i], ids[i + 1], CausalEdgeType::Causes, 1.0, 0, 0);
            }
            // Extra chords until average degree ~ avg_deg.
            // Path already contributes ~2 to average degree.
            let target_extra_edges = ((avg_deg.saturating_sub(2)) * n) / 2;
            let mut seed = 0xC0FFEE_u64.wrapping_add(n as u64);
            let mut added = 0usize;
            let mut attempts = 0usize;
            while added < target_extra_edges && attempts < target_extra_edges.saturating_mul(20) {
                attempts += 1;
                let i = (lcg_next(&mut seed) as usize) % n;
                let j = (lcg_next(&mut seed) as usize) % n;
                if i == j {
                    continue;
                }
                let (a, b) = if i < j { (i, j) } else { (j, i) };
                // Skip immediate path neighbours (already linked).
                if b == a + 1 {
                    continue;
                }
                g.link(ids[a], ids[b], CausalEdgeType::Correlates, 1.0, 0, 0);
                added += 1;
            }
        }
    }

    g
}

/// Closed-form algebraic connectivity for unit-weight cycle C_n.
fn analytical_cycle_lambda2(n: usize) -> f64 {
    let nf = n as f64;
    2.0 * (1.0 - (2.0 * std::f64::consts::PI / nf).cos())
}

/// Closed-form algebraic connectivity for unit-weight path P_n.
fn analytical_path_lambda2(n: usize) -> f64 {
    let nf = n as f64;
    2.0 * (1.0 - (std::f64::consts::PI / nf).cos())
}

fn analytical_ref(family: GraphFamily, n: usize) -> Option<f64> {
    match family {
        GraphFamily::Cycle => Some(analytical_cycle_lambda2(n)),
        GraphFamily::Path => Some(analytical_path_lambda2(n)),
        GraphFamily::Er => None,
    }
}

// ── Timing helpers ───────────────────────────────────────────────────────

fn median_duration(samples: &mut [Duration]) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

fn time_median<F>(warmup: usize, repeats: usize, mut f: F) -> (Duration, f64)
where
    F: FnMut() -> f64,
{
    for _ in 0..warmup {
        let _ = f();
    }
    let mut samples = Vec::with_capacity(repeats);
    let mut last = 0.0f64;
    for _ in 0..repeats {
        let t0 = Instant::now();
        last = f();
        samples.push(t0.elapsed());
    }
    (median_duration(&mut samples), last)
}

// ── Output ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MethodOutcome {
    method: String,
    n: usize,
    family: String,
    policy: String,
    elapsed_us: u64,
    lambda_2: f64,
    /// Relative error vs analytical reference when available.
    rel_err_analytical: Option<f64>,
    /// Relative error vs Lanczos reference when Lanczos ran.
    rel_err_lanczos: Option<f64>,
    skipped: bool,
    skip_reason: Option<String>,
}

#[derive(Serialize)]
struct SizeSummary {
    n: usize,
    family: String,
    policy: String,
    analytical_lambda_2: Option<f64>,
    lanczos_lambda_2: Option<f64>,
    methods: Vec<MethodOutcome>,
}

fn rel_err(est: f64, reference: f64) -> f64 {
    let denom = reference.abs().max(1e-12);
    (est - reference).abs() / denom
}

fn print_table(summaries: &[SizeSummary]) {
    eprintln!();
    eprintln!(
        "{:>8}  {:>8}  {:>10}  {:>12}  {:>12}  {:>10}  {:>10}",
        "n", "method", "policy", "time_us", "lambda_2", "err_anal", "err_lanc"
    );
    eprintln!("{}", "-".repeat(86));
    for s in summaries {
        for m in &s.methods {
            if m.skipped {
                eprintln!(
                    "{:>8}  {:>8}  {:>10}  {:>12}  {:>12}  {:>10}  {:>10}  ({})",
                    m.n,
                    m.method,
                    m.policy,
                    "SKIP",
                    "-",
                    "-",
                    "-",
                    m.skip_reason.as_deref().unwrap_or("")
                );
                continue;
            }
            let ea = m
                .rel_err_analytical
                .map(|e| format!("{e:.4}"))
                .unwrap_or_else(|| "-".into());
            let el = m
                .rel_err_lanczos
                .map(|e| format!("{e:.4}"))
                .unwrap_or_else(|| "-".into());
            eprintln!(
                "{:>8}  {:>8}  {:>10}  {:>12}  {:>12.6}  {:>10}  {:>10}",
                m.n, m.method, m.policy, m.elapsed_us, m.lambda_2, ea, el
            );
        }
    }
    eprintln!();
    eprintln!(
        "Decision rule: n < {SPECTRAL_RFF_MIN_NODES} → Lanczos; \
         {SPECTRAL_RFF_MIN_NODES} ≤ n < {SPECTRAL_EML_MIN_NODES} → RFF; \
         n ≥ {SPECTRAL_EML_MIN_NODES} → EML"
    );
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() {
    let cfg = BenchConfig::from_env();
    eprintln!(
        "[spectral_lambda2_bench] family={} sizes={:?} rff_features={} \
         lanczos_iters={} skip_lanczos_above={} warmup={} repeats={}",
        cfg.family.as_str(),
        cfg.sizes,
        cfg.rff_features,
        cfg.lanczos_iters,
        cfg.skip_lanczos_above,
        cfg.warmup,
        cfg.repeats
    );

    let mut summaries = Vec::new();

    for &n in &cfg.sizes {
        let policy = select_spectral_method(n);
        eprintln!(
            "[spectral_lambda2_bench] building {} n={} (policy={})...",
            cfg.family.as_str(),
            n,
            policy.as_str()
        );
        let t_build = Instant::now();
        let g = build_graph(n, cfg.family, cfg.avg_deg);
        eprintln!(
            "[spectral_lambda2_bench] built in {:.1}ms (nodes={} edges={})",
            t_build.elapsed().as_secs_f64() * 1000.0,
            g.node_count(),
            g.edge_count()
        );

        let analytical = analytical_ref(cfg.family, n);
        let mut methods = Vec::new();
        let mut lanczos_ref: Option<f64> = None;

        // ── Lanczos ──────────────────────────────────────────────────
        if n > cfg.skip_lanczos_above {
            methods.push(MethodOutcome {
                method: SpectralMethod::Lanczos.as_str().into(),
                n,
                family: cfg.family.as_str().into(),
                policy: policy.as_str().into(),
                elapsed_us: 0,
                lambda_2: 0.0,
                rel_err_analytical: None,
                rel_err_lanczos: None,
                skipped: true,
                skip_reason: Some(format!(
                    "n={n} > WEFT_BENCH_SKIP_LANCZOS_ABOVE={}",
                    cfg.skip_lanczos_above
                )),
            });
        } else {
            let k = cfg.lanczos_iters.min(n.saturating_sub(1).max(1));
            let (elapsed, lambda_2) = time_median(cfg.warmup, cfg.repeats, || {
                g.spectral_analysis(k).lambda_2
            });
            lanczos_ref = Some(lambda_2);
            methods.push(MethodOutcome {
                method: SpectralMethod::Lanczos.as_str().into(),
                n,
                family: cfg.family.as_str().into(),
                policy: policy.as_str().into(),
                elapsed_us: elapsed.as_micros() as u64,
                lambda_2,
                rel_err_analytical: analytical.map(|a| rel_err(lambda_2, a)),
                rel_err_lanczos: Some(0.0),
                skipped: false,
                skip_reason: None,
            });
        }

        // ── RFF ──────────────────────────────────────────────────────
        {
            let (elapsed, lambda_2) = time_median(cfg.warmup, cfg.repeats, || {
                g.spectral_analysis_rff(cfg.rff_features, cfg.rff_iters)
                    .lambda_2
            });
            methods.push(MethodOutcome {
                method: SpectralMethod::Rff.as_str().into(),
                n,
                family: cfg.family.as_str().into(),
                policy: policy.as_str().into(),
                elapsed_us: elapsed.as_micros() as u64,
                lambda_2,
                rel_err_analytical: analytical.map(|a| rel_err(lambda_2, a)),
                rel_err_lanczos: lanczos_ref.map(|r| rel_err(lambda_2, r)),
                skipped: false,
                skip_reason: None,
            });
        }

        // ── EML (untrained density fallback; O(1) ceiling) ───────────
        {
            let model = EmlCoherenceModel::new();
            let features = GraphFeatures::from_causal_graph(&g);
            // Include feature extraction in the timed path — that is the
            // real O(n) cost before the O(1) predict.
            let (elapsed, lambda_2) = time_median(cfg.warmup, cfg.repeats, || {
                let f = GraphFeatures::from_causal_graph(&g);
                model.predict_lambda2(&f)
            });
            let _ = features; // kept for potential future trained-model path
            methods.push(MethodOutcome {
                method: SpectralMethod::Eml.as_str().into(),
                n,
                family: cfg.family.as_str().into(),
                policy: policy.as_str().into(),
                elapsed_us: elapsed.as_micros() as u64,
                lambda_2,
                rel_err_analytical: analytical.map(|a| rel_err(lambda_2, a)),
                rel_err_lanczos: lanczos_ref.map(|r| rel_err(lambda_2, r)),
                skipped: false,
                skip_reason: None,
            });
        }

        // ── Auto dispatcher (executes Lanczos or RFF) ────────────────
        {
            let (elapsed, (chosen, lambda_2)) = {
                // Custom timing: return both method and value.
                for _ in 0..cfg.warmup {
                    let _ = g.spectral_analysis_auto();
                }
                let mut samples = Vec::with_capacity(cfg.repeats);
                let mut last = (SpectralMethod::Lanczos, 0.0f64);
                for _ in 0..cfg.repeats {
                    let t0 = Instant::now();
                    let (m, r) = g.spectral_analysis_auto();
                    last = (m, r.lambda_2);
                    samples.push(t0.elapsed());
                }
                (median_duration(&mut samples), last)
            };
            methods.push(MethodOutcome {
                method: format!("auto:{}", chosen.as_str()),
                n,
                family: cfg.family.as_str().into(),
                policy: policy.as_str().into(),
                elapsed_us: elapsed.as_micros() as u64,
                lambda_2,
                rel_err_analytical: analytical.map(|a| rel_err(lambda_2, a)),
                rel_err_lanczos: lanczos_ref.map(|r| rel_err(lambda_2, r)),
                skipped: false,
                skip_reason: None,
            });
        }

        let summary = SizeSummary {
            n,
            family: cfg.family.as_str().into(),
            policy: policy.as_str().into(),
            analytical_lambda_2: analytical,
            lanczos_lambda_2: lanczos_ref,
            methods,
        };
        println!(
            "{}",
            serde_json::to_string(&summary).expect("SizeSummary serializes")
        );
        summaries.push(summary);
    }

    print_table(&summaries);
}
