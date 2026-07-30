//! WEFT-185 swarm demo — coordinator fan-out over AgentBus worker loops.
//!
//! Run:
//! ```bash
//! cargo run -p clawft-core --example swarm_demo --features native
//! ```

use clawft_core::agent_bus::run_swarm_demo;

#[tokio::main]
async fn main() {
    let workers: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    println!("WEFT-185 swarm demo — spawning {workers} workers…");
    let summary = run_swarm_demo(workers).await;
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());

    let ok = summary["replies"]
        .as_array()
        .map(|a| a.iter().filter(|r| r["ok"] == true).count())
        .unwrap_or(0);
    if ok != workers {
        eprintln!("demo incomplete: {ok}/{workers} replies");
        std::process::exit(1);
    }
    println!("demo ok: {ok}/{workers} workers replied");
}
