//! `weft swarm` — multi-agent AgentBus / SwarmCoordinator demos (WEFT-185).
//!
//! Provides a production CLI surface that constructs an [`AgentBus`],
//! spawns worker message loops (runtime-backed identities), and runs
//! [`SwarmCoordinator::dispatch_subtask`] end-to-end.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Args, Subcommand};
use clawft_core::agent::runtime::AgentRuntime;
use clawft_core::agent_bus::{
    spawn_worker_loop, RuntimeBackedWorker, SwarmCoordinator, WorkerHandler,
};
use clawft_platform::NativePlatform;
use clawft_types::agent_bus::InterAgentMessage;
use clawft_types::config::{AgentDefaults, AgentsConfig};

/// Arguments for the `weft swarm` subcommand.
#[derive(Args, Debug)]
pub struct SwarmArgs {
    #[command(subcommand)]
    pub action: SwarmAction,
}

/// Subcommands for `weft swarm`.
#[derive(Subcommand, Debug)]
pub enum SwarmAction {
    /// Run the built-in coordinator → workers map/collect demo.
    Demo {
        /// Number of echo workers to spawn (default 3).
        #[arg(long, default_value_t = 3)]
        workers: usize,

        /// Optional workspace root for AgentRuntime isolation
        /// (defaults to a temp dir under the system temp folder).
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
}

/// Run the swarm subcommand.
pub async fn run(args: SwarmArgs) -> anyhow::Result<()> {
    match args.action {
        SwarmAction::Demo { workers, workspace } => run_demo(workers, workspace).await,
    }
}

async fn run_demo(workers: usize, workspace: Option<PathBuf>) -> anyhow::Result<()> {
    if workers == 0 {
        anyhow::bail!("--workers must be >= 1");
    }

    let workspace = match workspace {
        Some(p) => p,
        None => {
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            std::env::temp_dir().join(format!("weft-swarm-demo-{pid}-{nanos}"))
        }
    };
    std::fs::create_dir_all(&workspace)?;

    let platform = Arc::new(NativePlatform::new());
    let memory = Arc::new(clawft_core::agent::memory::MemoryStore::for_workspace(
        &workspace,
        platform.clone(),
    ));
    let skills = Arc::new(clawft_core::agent::skills::SkillsLoader::for_workspace(
        &workspace,
        platform.clone(),
    ));

    // Production wiring: AgentBus via SwarmCoordinator::with_capacity
    // (exercises the previously unused capacity constructor).
    let worker_ids: Vec<String> = (0..workers).map(|i| format!("worker-{i}")).collect();
    let (mut coord, bus) =
        SwarmCoordinator::with_capacity("demo-coord", worker_ids.clone(), 64);

    // Register coordinator inbox before dispatch.
    coord.ensure_registered().await;

    // Spawn worker loops backed by AgentRuntime isolation (sessions
    // namespaced under <workspace>/sessions/<agent_id>/).
    let mut handles = Vec::new();
    let mut runtimes = Vec::new();
    for id in &worker_ids {
        let cfg = AgentsConfig {
            defaults: AgentDefaults {
                workspace: workspace.display().to_string(),
                model: "demo-model".into(),
                max_tokens: 1024,
                temperature: 0.0,
                max_tool_iterations: 4,
                memory_window: 16,
            },
            ..AgentsConfig::default()
        };
        let runtime = AgentRuntime::for_agent(
            id.clone(),
            &workspace,
            platform.clone(),
            memory.clone(),
            skills.clone(),
            cfg,
        );
        let agent_id = runtime.agent_id().to_string();
        let handler: Arc<dyn WorkerHandler> =
            Arc::new(RuntimeBackedWorker::echo(agent_id.clone()));
        handles.push(spawn_worker_loop(bus.clone(), handler).await);
        runtimes.push(runtime);
        println!("  spawned worker loop agent_id={agent_id}");
    }
    // Keep runtimes alive until the end of the demo (isolation surface).
    let _runtimes = runtimes;

    println!(
        "swarm demo: coordinator={} workers={} workspace={}",
        coord.coordinator_id(),
        workers,
        workspace.display()
    );

    let results = coord
        .dispatch_and_collect(
            "map",
            serde_json::json!({
                "demo": "weft-swarm-demo",
                "ticket": "WEFT-185",
            }),
            Duration::from_secs(30),
            Duration::from_secs(5),
        )
        .await;

    let mut ok = 0usize;
    for (worker, result) in &results {
        match result {
            Ok(msg) => {
                ok += 1;
                println!(
                    "  ✓ {worker}: status={} task={}",
                    msg.payload
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?"),
                    msg.payload
                        .get("task")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                );
            }
            Err(e) => {
                println!("  ✗ {worker}: {e}");
            }
        }
    }

    // Shutdown workers cleanly.
    for worker in &worker_ids {
        let msg = InterAgentMessage::new(
            coord.coordinator_id(),
            worker,
            "shutdown",
            serde_json::Value::Null,
            Duration::from_secs(10),
        );
        let _ = bus.send(msg).await;
    }
    for h in handles {
        let _ = tokio::time::timeout(Duration::from_secs(2), h).await;
    }

    println!("swarm demo complete: {ok}/{workers} workers replied");
    if ok != workers {
        anyhow::bail!("expected {workers} replies, got {ok}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn demo_two_workers() {
        run_demo(2, None).await.expect("demo should succeed");
    }
}
