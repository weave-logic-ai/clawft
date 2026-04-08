//! `weaver benchmark` — Standardized kernel performance benchmark.
//!
//! Exercises all kernel subsystems and produces a scored report.
//! Modeled on the Mentra benchmark pattern: measure, score, compare.

use std::time::Instant;

use clap::{Parser, Subcommand};
use clawft_rpc::DaemonClient;

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
    },
    /// Show results from the last benchmark run.
    Last,
}

/// Run the benchmark command.
pub async fn run(cmd: BenchCmd) -> anyhow::Result<()> {
    match cmd {
        BenchCmd::Run { format, iterations } => run_benchmark(&format, iterations).await,
        BenchCmd::Last => show_last().await,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BenchResult {
    name: String,
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
    results: Vec<BenchResult>,
    overall_score: f64,
    grade: String,
}

async fn run_benchmark(format: &str, iterations: u32) -> anyhow::Result<()> {
    let mut client = DaemonClient::connect()
        .await
        .ok_or_else(|| anyhow::anyhow!("no kernel running — start with: weaver kernel start"))?;

    println!("WeftOS Kernel Benchmark");
    println!("=======================");
    println!("Iterations per test: {iterations}");
    println!();

    let mut results = Vec::new();

    // 1. Kernel status (baseline RPC latency)
    results.push(bench_rpc(&mut client, "kernel.status", iterations).await);

    // 2. Process table operations
    results.push(bench_rpc(&mut client, "kernel.ps", iterations).await);

    // 3. Service registry
    results.push(bench_rpc(&mut client, "kernel.services", iterations).await);

    // 4. Kernel logs
    results.push(bench_rpc(&mut client, "kernel.logs", iterations).await);

    // 5. ECC status
    results.push(bench_rpc(&mut client, "ecc.status", iterations).await);

    // 6. ECC calibration (single call, already cached)
    results.push(bench_rpc(&mut client, "ecc.calibrate", iterations).await);

    // 7. ECC causal graph
    results.push(bench_rpc(&mut client, "ecc.causal", iterations).await);

    // 8. ECC HNSW search info
    results.push(bench_rpc(&mut client, "ecc.search", iterations).await);

    // 9. ECC tick status
    results.push(bench_rpc(&mut client, "ecc.tick", iterations).await);

    // 10. ECC crossrefs
    results.push(bench_rpc(&mut client, "ecc.crossrefs", iterations).await);

    // 11. Cron list
    results.push(bench_rpc(&mut client, "cron.list", iterations).await);

    // 12. Agent list
    results.push(bench_rpc(&mut client, "agent.list", iterations).await);

    // 13. Ping (raw transport baseline)
    results.push(bench_rpc(&mut client, "ping", iterations).await);

    // Get kernel version
    let version = client.simple_call("kernel.status").await
        .ok()
        .and_then(|r| r.result)
        .and_then(|v| v.get("version").and_then(|v| v.as_str().map(String::from)))
        .unwrap_or_else(|| "unknown".to_string());

    // Compute overall score (lower avg latency = higher score)
    let successful: Vec<&BenchResult> = results.iter()
        .filter(|r| r.status == "ok")
        .collect();

    let overall_score = if successful.is_empty() {
        0.0
    } else {
        let avg_latency: f64 = successful.iter().map(|r| r.avg_us).sum::<f64>()
            / successful.len() as f64;
        // Score: 100 for <100us avg, 0 for >10ms avg, linear scale
        ((10_000.0 - avg_latency) / 100.0).clamp(0.0, 100.0)
    };

    let grade = match overall_score as u32 {
        90..=100 => "A+",
        80..=89 => "A",
        70..=79 => "B",
        60..=69 => "C",
        50..=59 => "D",
        _ => "F",
    };

    let report = BenchReport {
        timestamp: chrono_now(),
        kernel_version: version,
        results: results.clone(),
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
        print_table(&report);
    }

    Ok(())
}

async fn bench_rpc(client: &mut DaemonClient, method: &str, iterations: u32) -> BenchResult {
    let mut latencies = Vec::with_capacity(iterations as usize);
    let mut errors = 0u32;

    for _ in 0..iterations {
        let start = Instant::now();
        let resp = client.simple_call(method).await;
        let elapsed = start.elapsed();
        let us = elapsed.as_micros() as f64;

        match resp {
            Ok(r) if r.ok => latencies.push(us),
            _ => errors += 1,
        }
    }

    if latencies.is_empty() {
        return BenchResult {
            name: method.to_string(),
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
        name: method.to_string(),
        iterations,
        total_ms: total / 1000.0,
        avg_us: avg,
        min_us: min,
        max_us: max,
        p95_us: p95,
        ops_per_sec: ops,
        status: if errors > 0 {
            format!("ok ({errors} errors)")
        } else {
            "ok".to_string()
        },
    }
}

fn print_table(report: &BenchReport) {
    println!("Kernel: {}", report.kernel_version);
    println!("Time:   {}", report.timestamp);
    println!();
    println!(
        "{:<20} {:>8} {:>8} {:>8} {:>8} {:>10} {:>8}",
        "Test", "Avg(us)", "Min(us)", "P95(us)", "Max(us)", "Ops/sec", "Status"
    );
    println!("{}", "-".repeat(82));

    for r in &report.results {
        println!(
            "{:<20} {:>8.0} {:>8.0} {:>8.0} {:>8.0} {:>10.0} {:>8}",
            r.name, r.avg_us, r.min_us, r.p95_us, r.max_us, r.ops_per_sec, r.status
        );
    }

    println!("{}", "-".repeat(82));
    println!();
    println!("Overall Score: {:.1}/100  Grade: {}", report.overall_score, report.grade);
    println!();
    println!("Report saved to: {}", clawft_rpc::runtime_dir().join("benchmarks/latest.json").display());
}

async fn show_last() -> anyhow::Result<()> {
    let report_path = clawft_rpc::runtime_dir().join("benchmarks/latest.json");
    if !report_path.exists() {
        anyhow::bail!("no benchmark results found — run: weaver benchmark run");
    }
    let data = std::fs::read_to_string(&report_path)?;
    let report: BenchReport = serde_json::from_str(&data)?;
    print_table(&report);
    Ok(())
}

fn chrono_now() -> String {
    // Simple ISO 8601 without chrono dep
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s since epoch", d.as_secs())
}
