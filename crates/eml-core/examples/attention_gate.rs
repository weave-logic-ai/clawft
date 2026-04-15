//! Iteration 0 go/no-go gate runner.
//!
//! Runs `run_benchmark` at the reference go/no-go shape and prints whether
//! each criterion passes. Exits with non-zero status if any criterion fails.
//!
//! Usage:
//!   cargo run --example attention_gate --features experimental-attention

#[cfg(feature = "experimental-attention")]
fn main() {
    use eml_core::run_benchmark;

    let b = match run_benchmark(
        /* d_model */ 8,
        /* d_k     */ 4,
        /* seq_len */ 4,
        /* depth   */ 3,
    ) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("benchmark error: {e}");
            std::process::exit(2);
        }
    };

    println!("ToyEmlAttention Iteration 0 — go/no-go gate");
    println!("shape: seq_len={} d_model={} d_k={} depth={}", b.seq_len, b.d_model, b.d_k, b.depth);
    println!("param_count: {}", b.param_count);
    println!("phase1 warmup:   {:>8} ns   (roundtrip={})", b.phase1_warmup_ns, b.phase1_serialize_roundtrip);
    println!(
        "phase2 converge: converged={}  final_mse={:.6}  rounds={}",
        b.phase2_converged, b.phase2_final_mse, b.phase2_training_rounds
    );
    println!(
        "phase3 compute:  mean={:>8} ns   p99={:>8} ns",
        b.phase3_inference_ns_mean, b.phase3_inference_ns_p99
    );
    println!("phase4 scaling:");
    for p in &b.phase4_scaling {
        println!(
            "  seq_len={} d_model={} params={:>5}  mean={:>8} ns",
            p.seq_len, p.d_model, p.param_count, p.inference_ns_mean
        );
    }

    // Go/no-go criteria from .planning/development_notes/eml_model_development_assessment.md
    let gate1 = b.phase2_converged && b.phase2_training_rounds <= 3;
    let gate2 = b.phase3_inference_ns_p99 <= 5_000;
    let gate3 = b.phase3_inference_ns_p99 > 0 && b.phase3_inference_ns_mean > 0;
    let gate4 = b.phase1_serialize_roundtrip;
    let gate5 = b.phase4_scaling.windows(2).all(|pair| {
        let small = pair[0].inference_ns_mean.max(1);
        let big = pair[1].inference_ns_mean;
        // Allow up to 16x growth per step; that's polynomial, not exponential
        big <= small * 16
    });

    println!();
    println!("--- gate ---");
    println!("G1 converges ≤ 3 rounds:       {}", tag(gate1));
    println!("G2 p99 ≤ 5 µs:                 {}", tag(gate2));
    println!("G3 timings finite:             {}", tag(gate3));
    println!("G4 serialization roundtrip:    {}", tag(gate4));
    println!("G5 polynomial scaling:         {}", tag(gate5));

    let all_pass = gate1 && gate2 && gate3 && gate4 && gate5;
    println!();
    println!("RESULT: {}", if all_pass { "PASS ✔" } else { "FAIL ✘" });
    if !all_pass {
        std::process::exit(1);
    }
}

#[cfg(feature = "experimental-attention")]
fn tag(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

#[cfg(not(feature = "experimental-attention"))]
fn main() {
    eprintln!("enable the `experimental-attention` feature: cargo run --example attention_gate --features experimental-attention");
    std::process::exit(2);
}
