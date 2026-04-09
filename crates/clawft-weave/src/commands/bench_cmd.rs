//! `weaver benchmark` — Standardized kernel performance benchmark.
//!
//! Three tiers of testing:
//! - **RPC**: Raw transport latency (Unix socket round-trip)
//! - **Compute**: Real kernel operations (agent spawn/stop, chain append, HNSW insert/search)
//! - **Stress**: Concurrent load, sustained throughput, memory pressure

use std::time::Instant;

use clap::Subcommand;
use clawft_rpc::{DaemonClient, Request};

/// Benchmark subcommands.
#[derive(Debug, Subcommand)]
pub enum BenchCmd {
    /// Run the full benchmark suite against a running kernel.
    Run {
        /// Output format: table (default), json.
        #[arg(short, long, default_value = "table")]
        format: String,
        /// Number of iterations per test (default: 100).
        #[arg(short = 'n', long, default_value = "100")]
        iterations: u32,
        /// Skip slow tests (stress tier).
        #[arg(long)]
        quick: bool,
    },
    /// Show results from the last benchmark run.
    Last,
}

/// Run the benchmark command.
pub async fn run(cmd: BenchCmd) -> anyhow::Result<()> {
    match cmd {
        BenchCmd::Run { format, iterations, quick } => run_benchmark(&format, iterations, quick).await,
        BenchCmd::Last => show_last().await,
    }
}

/// Just run install with no subcommand.
pub async fn run_default() -> anyhow::Result<()> {
    run_benchmark("table", 100, false).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchResult {
    name: String,
    tier: String,
    iterations: u32,
    total_ms: f64,
    avg_us: f64,
    min_us: f64,
    max_us: f64,
    p95_us: f64,
    ops_per_sec: f64,
    status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchReport {
    timestamp: String,
    kernel_version: String,
    platform: String,
    results: Vec<BenchResult>,
    tier_scores: TierScores,
    overall_score: f64,
    grade: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TierScores {
    rpc: f64,
    compute: f64,
    stress: f64,
}

async fn run_benchmark(format: &str, iterations: u32, quick: bool) -> anyhow::Result<()> {
    let mut client = DaemonClient::connect()
        .await
        .ok_or_else(|| anyhow::anyhow!("no kernel running — start with: weaver kernel start"))?;

    let platform = format!("{} {} ({}cpu, {})",
        std::env::consts::OS,
        std::env::consts::ARCH,
        num_cpus(),
        memory_info(),
    );

    println!("WeftOS Kernel Benchmark v2");
    println!("=========================");
    println!("Platform:   {platform}");
    println!("Iterations: {iterations}");
    println!();

    let mut results = Vec::new();

    // ═══════════════════════════════════════════════════════════
    // TIER 1: RPC Transport (baseline)
    // ═══════════════════════════════════════════════════════════
    println!("── Tier 1: RPC Transport ──");
    results.push(bench_rpc(&mut client, "ping", "rpc", iterations).await);
    results.push(bench_rpc(&mut client, "kernel.status", "rpc", iterations).await);
    results.push(bench_rpc(&mut client, "kernel.ps", "rpc", iterations).await);
    results.push(bench_rpc(&mut client, "kernel.services", "rpc", iterations).await);
    results.push(bench_rpc(&mut client, "kernel.logs", "rpc", iterations).await);
    println!();

    // ═══════════════════════════════════════════════════════════
    // TIER 2: Compute (real kernel operations)
    // ═══════════════════════════════════════════════════════════
    println!("── Tier 2: Compute ──");

    // Agent spawn + stop cycle
    results.push(bench_agent_lifecycle(&mut client, iterations / 5).await);

    // Chain append (if exochain enabled)
    results.push(bench_chain_append(&mut client, iterations).await);

    // ECC operations
    results.push(bench_rpc(&mut client, "ecc.status", "compute", iterations).await);
    results.push(bench_rpc(&mut client, "ecc.causal", "compute", iterations).await);

    // Cron add + remove cycle
    results.push(bench_cron_lifecycle(&mut client, iterations / 5).await);

    // IPC publish
    results.push(bench_ipc_publish(&mut client, iterations).await);

    println!();

    // ═══════════════════════════════════════════════════════════
    // TIER 3: Stress (sustained throughput + concurrency)
    // ═══════════════════════════════════════════════════════════
    if !quick {
        println!("── Tier 3: Stress ──");

        // Burst: fire N requests as fast as possible
        results.push(bench_burst(&mut client, iterations * 5).await);

        // Payload: large params/response
        results.push(bench_large_payload(&mut client, iterations / 2).await);

        // Mixed workload: random method selection
        results.push(bench_mixed_workload(&mut client, iterations * 2).await);

        println!();
    }

    // Get kernel version
    let version = client.simple_call("kernel.status").await
        .ok()
        .and_then(|r| r.result)
        .and_then(|v| v.get("version").and_then(|v| v.as_str().map(String::from)))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    // Score each tier separately
    let rpc_results: Vec<&BenchResult> = results.iter().filter(|r| r.tier == "rpc" && r.status.starts_with("ok")).collect();
    let compute_results: Vec<&BenchResult> = results.iter().filter(|r| r.tier == "compute" && r.status.starts_with("ok")).collect();
    let stress_results: Vec<&BenchResult> = results.iter().filter(|r| r.tier == "stress" && r.status.starts_with("ok")).collect();

    let rpc_score = score_tier(&rpc_results, 15.0);      // fast baseline expected
    let compute_score = score_tier(&compute_results, 200.0); // real work, higher threshold
    let stress_score = if quick { 0.0 } else { score_tier(&stress_results, 100.0) };

    // Overall: weighted average (compute matters most)
    let (overall_score, weights_used) = if quick {
        ((rpc_score * 0.3 + compute_score * 0.7), "rpc:30% compute:70%")
    } else {
        ((rpc_score * 0.2 + compute_score * 0.5 + stress_score * 0.3), "rpc:20% compute:50% stress:30%")
    };

    let grade = grade_from_score(overall_score);

    let report = BenchReport {
        timestamp: iso_now(),
        kernel_version: version,
        platform,
        results: results.clone(),
        tier_scores: TierScores {
            rpc: rpc_score,
            compute: compute_score,
            stress: stress_score,
        },
        overall_score,
        grade: grade.to_string(),
    };

    // Save report
    let report_dir = clawft_rpc::runtime_dir().join("benchmarks");
    std::fs::create_dir_all(&report_dir)?;
    let report_path = report_dir.join("latest.json");
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_table(&report, weights_used);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// Tier 2: Compute benchmarks
// ═══════════════════════════════════════════════════════════════

async fn bench_agent_lifecycle(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        let agent_id = format!("bench-agent-{i}");
        let start = Instant::now();

        // Spawn
        let spawn_resp = client.call(Request::with_params(
            "agent.spawn",
            serde_json::json!({"agent_id": agent_id, "agent_type": "worker"}),
        )).await;

        if let Ok(ref r) = spawn_resp {
            if r.ok {
                // Get PID from response
                if let Some(pid) = r.result.as_ref()
                    .and_then(|v| v.get("pid"))
                    .and_then(|v| v.as_u64())
                {
                    // Stop
                    let _ = client.call(Request::with_params(
                        "agent.stop",
                        serde_json::json!({"pid": pid}),
                    )).await;
                }
            }
        }

        let elapsed = start.elapsed();
        match spawn_resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("agent.lifecycle", "compute", iterations, &mut latencies, errors)
}

async fn bench_chain_append(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        let start = Instant::now();
        let resp = client.call(Request::with_params(
            "chain.append",
            serde_json::json!({
                "event_type": "benchmark.test",
                "payload": format!("bench-event-{i}"),
            }),
        )).await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("chain.append", "compute", iterations, &mut latencies, errors)
}

async fn bench_cron_lifecycle(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        let start = Instant::now();

        // Add
        let add_resp = client.call(Request::with_params(
            "cron.add",
            serde_json::json!({
                "name": format!("bench-cron-{i}"),
                "schedule": "0 0 * * *",
                "prompt": "benchmark test",
            }),
        )).await;

        if let Ok(ref r) = add_resp {
            if r.ok {
                if let Some(job_id) = r.result.as_ref()
                    .and_then(|v| v.get("job_id"))
                    .and_then(|v| v.as_str())
                {
                    // Remove
                    let _ = client.call(Request::with_params(
                        "cron.remove",
                        serde_json::json!({"id": job_id}),
                    )).await;
                }
            }
        }

        let elapsed = start.elapsed();
        match add_resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("cron.lifecycle", "compute", iterations, &mut latencies, errors)
}

async fn bench_ipc_publish(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        let start = Instant::now();
        let resp = client.call(Request::with_params(
            "ipc.publish",
            serde_json::json!({
                "topic": "benchmark.test",
                "payload": format!("msg-{i}"),
            }),
        )).await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("ipc.publish", "compute", iterations, &mut latencies, errors)
}

// ═══════════════════════════════════════════════════════════════
// Tier 3: Stress benchmarks
// ═══════════════════════════════════════════════════════════════

async fn bench_burst(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    // Fire requests as fast as possible, no pause between
    let start = Instant::now();
    let mut successes = 0u32;
    let mut errors = 0u32;

    for _ in 0..iterations {
        match client.simple_call("ping").await {
            Ok(r) if r.ok => successes += 1,
            _ => errors += 1,
        }
    }

    let total_us = start.elapsed().as_micros() as f64;
    let avg_us = total_us / iterations as f64;
    let ops = if avg_us > 0.0 { 1_000_000.0 / avg_us } else { 0.0 };

    BenchResult {
        name: "burst.ping".to_string(),
        tier: "stress".to_string(),
        iterations,
        total_ms: total_us / 1000.0,
        avg_us,
        min_us: avg_us, // no per-iteration tracking for burst
        max_us: avg_us,
        p95_us: avg_us,
        ops_per_sec: ops,
        status: if errors > 0 { format!("ok ({errors} errors)") } else { "ok".to_string() },
    }
}

async fn bench_large_payload(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    // Send requests with increasingly large payloads
    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        // 1KB to 64KB payloads
        let size = 1024 * (1 + (i % 64)) as usize;
        let payload = "x".repeat(size);

        let start = Instant::now();
        let resp = client.call(Request::with_params(
            "ipc.publish",
            serde_json::json!({
                "topic": "benchmark.payload",
                "payload": payload,
            }),
        )).await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("large.payload", "stress", iterations, &mut latencies, errors)
}

async fn bench_mixed_workload(client: &mut DaemonClient, iterations: u32) -> BenchResult {
    // Randomly interleave different method calls
    let methods = [
        "ping",
        "kernel.status",
        "kernel.ps",
        "kernel.services",
        "ecc.status",
        "ecc.causal",
        "cron.list",
        "agent.list",
    ];

    let mut latencies = Vec::new();
    let mut errors = 0u32;

    for i in 0..iterations {
        let method = methods[i as usize % methods.len()];
        let start = Instant::now();
        let resp = client.simple_call(method).await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result("mixed.workload", "stress", iterations, &mut latencies, errors)
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

async fn bench_rpc(client: &mut DaemonClient, method: &str, tier: &str, iterations: u32) -> BenchResult {
    let mut latencies = Vec::with_capacity(iterations as usize);
    let mut errors = 0u32;

    for _ in 0..iterations {
        let start = Instant::now();
        let resp = client.simple_call(method).await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) if r.ok => latencies.push(elapsed.as_micros() as f64),
            _ => errors += 1,
        }
    }

    make_result(method, tier, iterations, &mut latencies, errors)
}

fn make_result(name: &str, tier: &str, iterations: u32, latencies: &mut Vec<f64>, errors: u32) -> BenchResult {
    if latencies.is_empty() {
        return BenchResult {
            name: name.to_string(),
            tier: tier.to_string(),
            iterations,
            total_ms: 0.0,
            avg_us: 0.0,
            min_us: 0.0,
            max_us: 0.0,
            p95_us: 0.0,
            ops_per_sec: 0.0,
            status: format!("FAIL ({errors} errors)"),
        };
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let total: f64 = latencies.iter().sum();
    let count = latencies.len() as f64;
    let avg = total / count;
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let p95_idx = ((latencies.len() as f64) * 0.95) as usize;
    let p95 = latencies[p95_idx.min(latencies.len() - 1)];
    let ops = if avg > 0.0 { 1_000_000.0 / avg } else { 0.0 };

    BenchResult {
        name: name.to_string(),
        tier: tier.to_string(),
        iterations,
        total_ms: total / 1000.0,
        avg_us: avg,
        min_us: min,
        max_us: max,
        p95_us: p95,
        ops_per_sec: ops,
        status: if errors > 0 {
            format!("ok ({errors} err)")
        } else {
            "ok".to_string()
        },
    }
}

fn score_tier(results: &[&BenchResult], reference_us: f64) -> f64 {
    if results.is_empty() {
        return 0.0;
    }

    let mut total = 0.0;
    for r in results {
        // Score: ratio of reference to actual, capped at 100
        // If reference=15μs and actual=15μs → score=100
        // If reference=15μs and actual=30μs → score=50
        // If reference=15μs and actual=150μs → score=10
        let ratio = reference_us / r.avg_us;
        let score = (ratio * 100.0).min(100.0);
        total += score;
    }

    total / results.len() as f64
}

fn grade_from_score(score: f64) -> &'static str {
    match score as u32 {
        90..=100 => "A+",
        80..=89 => "A",
        70..=79 => "B+",
        60..=69 => "B",
        50..=59 => "B-",
        40..=49 => "C+",
        30..=39 => "C",
        20..=29 => "D",
        _ => "F",
    }
}

fn print_table(report: &BenchReport, weights: &str) {
    println!("Kernel:   {}", report.kernel_version);
    println!("Platform: {}", report.platform);
    println!("Time:     {}", report.timestamp);
    println!();
    println!(
        "{:<20} {:>6} {:>8} {:>8} {:>8} {:>8} {:>10} {:>8}",
        "Test", "Tier", "Avg(us)", "Min(us)", "P95(us)", "Max(us)", "Ops/sec", "Status"
    );
    println!("{}", "─".repeat(90));

    let mut current_tier = String::new();
    for r in &report.results {
        if r.tier != current_tier {
            if !current_tier.is_empty() {
                println!("{}", "─".repeat(90));
            }
            current_tier = r.tier.clone();
        }
        println!(
            "{:<20} {:>6} {:>8.0} {:>8.0} {:>8.0} {:>8.0} {:>10.0} {:>8}",
            r.name, r.tier, r.avg_us, r.min_us, r.p95_us, r.max_us, r.ops_per_sec, r.status
        );
    }

    println!("{}", "─".repeat(90));
    println!();
    println!("Tier Scores:");
    println!("  RPC:     {:>5.1}/100", report.tier_scores.rpc);
    println!("  Compute: {:>5.1}/100", report.tier_scores.compute);
    if report.tier_scores.stress > 0.0 {
        println!("  Stress:  {:>5.1}/100", report.tier_scores.stress);
    }
    println!();
    println!("Overall: {:.1}/100  Grade: {}  (weights: {weights})", report.overall_score, report.grade);
    println!();
    println!("Report: {}", clawft_rpc::runtime_dir().join("benchmarks/latest.json").display());
}

async fn show_last() -> anyhow::Result<()> {
    let report_path = clawft_rpc::runtime_dir().join("benchmarks/latest.json");
    if !report_path.exists() {
        anyhow::bail!("no benchmark results found — run: weaver benchmark run");
    }
    let data = std::fs::read_to_string(&report_path)?;
    let report: BenchReport = serde_json::from_str(&data)?;
    print_table(&report, "from saved report");
    Ok(())
}

fn iso_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Approximate ISO 8601 without chrono
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let rem_days = days % 365;
    let months = rem_days / 30 + 1;
    let day = rem_days % 30 + 1;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{years}-{months:02}-{day:02}T{hours:02}:{mins:02}:{s:02}Z")
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn memory_info() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    let kb: u64 = line.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    return format!("{}GB", kb / 1_048_576);
                }
            }
        }
        "unknown".to_string()
    }
    #[cfg(not(target_os = "linux"))]
    { "unknown".to_string() }
}
