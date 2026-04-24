//! Integration test: substrate read / publish / subscribe / notify.

use std::sync::Arc;
use std::time::Duration;

use clawft_kernel::boot::Kernel;
use clawft_platform::NativePlatform;
use clawft_types::config::{AgentDefaults, AgentsConfig, Config, KernelConfig};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;

fn base_config() -> Config {
    Config {
        agents: AgentsConfig {
            defaults: AgentDefaults {
                workspace: "~/.clawft/workspace".into(),
                model: "test/model".into(),
                max_tokens: 1024,
                temperature: 0.5,
                max_tool_iterations: 5,
                memory_window: 10,
            },
        },
        ..Config::default()
    }
}

fn minimal_kernel_config() -> KernelConfig {
    KernelConfig {
        enabled: true,
        max_processes: 64,
        health_check_interval_secs: 5,
        cluster: None,
        chain: None,
        resource_tree: None,
        vector: None,
        profiles: None,
        pairing: None,
        mesh: None,
        anchor: None,
        ipc_tcp: None,
    }
}

async fn spawn_test_daemon() -> (tempfile::TempDir, std::path::PathBuf, watch::Sender<bool>) {
    let tmp = tempfile::tempdir().unwrap();
    let socket_path = tmp.path().join("kernel.sock");
    let platform = NativePlatform::new();
    let kernel = Kernel::boot(base_config(), minimal_kernel_config(), Arc::new(platform))
        .await
        .expect("kernel boot");
    let kernel = Arc::new(tokio::sync::RwLock::new(kernel));

    let listener = UnixListener::bind(&socket_path).unwrap();
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let accept_kernel = Arc::clone(&kernel);
    let accept_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let k = Arc::clone(&accept_kernel);
                            let tx = accept_shutdown_tx.clone();
                            tokio::spawn(clawft_weave::daemon::handle_connection(stream, k, tx));
                        }
                        Err(_) => break,
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() { break; }
                }
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    (tmp, socket_path, shutdown_tx)
}

async fn one_shot(
    socket: &std::path::Path,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let stream = UnixStream::connect(socket).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let req = serde_json::json!({ "id": "t", "method": method, "params": params });
    let mut line = serde_json::to_string(&req).unwrap();
    line.push('\n');
    writer.write_all(line.as_bytes()).await.unwrap();
    let mut ack = String::new();
    reader.read_line(&mut ack).await.unwrap();
    serde_json::from_str(ack.trim()).unwrap()
}

#[tokio::test]
async fn substrate_read_write_notify_roundtrip() {
    let (_tmp, socket, shutdown_tx) = spawn_test_daemon().await;

    // Empty read returns null value + tick=0.
    let r1 = one_shot(
        &socket,
        "substrate.read",
        serde_json::json!({ "path": "substrate/test/ping" }),
    )
    .await;
    assert_eq!(r1["ok"], serde_json::Value::Bool(true));
    assert!(r1["result"]["value"].is_null());
    assert_eq!(r1["result"]["tick"], 0);

    // Publish a value.
    let r2 = one_shot(
        &socket,
        "substrate.publish",
        serde_json::json!({ "path": "substrate/test/ping", "value": { "x": 7 } }),
    )
    .await;
    assert_eq!(r2["ok"], serde_json::Value::Bool(true));
    let tick_after_publish = r2["result"]["tick"].as_u64().unwrap();
    assert!(tick_after_publish > 0);

    // Read back the value.
    let r3 = one_shot(
        &socket,
        "substrate.read",
        serde_json::json!({ "path": "substrate/test/ping" }),
    )
    .await;
    assert_eq!(r3["result"]["value"]["x"], 7);
    assert_eq!(r3["result"]["tick"], tick_after_publish);

    // Notify bumps the tick but not the value.
    let r4 = one_shot(
        &socket,
        "substrate.notify",
        serde_json::json!({ "path": "substrate/test/ping" }),
    )
    .await;
    assert_eq!(r4["ok"], serde_json::Value::Bool(true));
    let tick_after_notify = r4["result"]["tick"].as_u64().unwrap();
    assert!(tick_after_notify > tick_after_publish);

    let r5 = one_shot(
        &socket,
        "substrate.read",
        serde_json::json!({ "path": "substrate/test/ping" }),
    )
    .await;
    assert_eq!(r5["result"]["value"]["x"], 7);
    assert_eq!(r5["result"]["tick"], tick_after_notify);

    let _ = shutdown_tx.send(true);
}

#[tokio::test]
async fn substrate_list_returns_prefix_children() {
    let (_tmp, socket, shutdown_tx) = spawn_test_daemon().await;

    // Empty list before any publish.
    let empty = one_shot(
        &socket,
        "substrate.list",
        serde_json::json!({ "prefix": "substrate/list-test", "depth": 1 }),
    )
    .await;
    assert_eq!(empty["ok"], serde_json::Value::Bool(true));
    assert!(empty["result"]["children"].as_array().unwrap().is_empty());

    // Seed two children and one grandchild.
    for (path, value) in [
        ("substrate/list-test/mic", serde_json::json!({ "rms_db": -20 })),
        ("substrate/list-test/tof", serde_json::json!({ "frame": 1 })),
        (
            "substrate/list-test/mic/history",
            serde_json::json!([1, 2, 3]),
        ),
    ] {
        let r = one_shot(
            &socket,
            "substrate.publish",
            serde_json::json!({ "path": path, "value": value }),
        )
        .await;
        assert_eq!(r["ok"], serde_json::Value::Bool(true), "publish {path}");
    }

    // depth = 1 (default) — expect two direct children, mic having one grandchild.
    let r = one_shot(
        &socket,
        "substrate.list",
        serde_json::json!({ "prefix": "substrate/list-test", "depth": 1 }),
    )
    .await;
    assert_eq!(r["ok"], serde_json::Value::Bool(true));
    let children = r["result"]["children"].as_array().unwrap();
    assert_eq!(children.len(), 2, "{children:?}");
    let mic = children
        .iter()
        .find(|c| c["path"] == "substrate/list-test/mic")
        .unwrap();
    assert_eq!(mic["has_value"], serde_json::Value::Bool(true));
    assert_eq!(mic["child_count"], 1);
    let tof = children
        .iter()
        .find(|c| c["path"] == "substrate/list-test/tof")
        .unwrap();
    assert_eq!(tof["has_value"], serde_json::Value::Bool(true));
    assert_eq!(tof["child_count"], 0);
    assert!(r["result"]["tick"].as_u64().unwrap() > 0);

    // Default depth: omit field → treat as 1 (per protocol default).
    let r2 = one_shot(
        &socket,
        "substrate.list",
        serde_json::json!({ "prefix": "substrate/list-test" }),
    )
    .await;
    assert_eq!(r2["result"]["children"].as_array().unwrap().len(), 2);

    // depth = 2 — flat list including the grandchild.
    let r3 = one_shot(
        &socket,
        "substrate.list",
        serde_json::json!({ "prefix": "substrate/list-test", "depth": 2 }),
    )
    .await;
    let paths: Vec<&str> = r3["result"]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"substrate/list-test/mic/history"));

    let _ = shutdown_tx.send(true);
}

#[tokio::test]
async fn substrate_subscribe_streams_updates() {
    let (_tmp, socket, shutdown_tx) = spawn_test_daemon().await;

    // Open a streaming subscribe.
    let stream = UnixStream::connect(&socket).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let sub = serde_json::json!({
        "id": "sub",
        "method": "substrate.subscribe",
        "params": { "path": "substrate/test/stream" },
    });
    let mut line = serde_json::to_string(&sub).unwrap();
    line.push('\n');
    writer.write_all(line.as_bytes()).await.unwrap();
    let mut ack = String::new();
    reader.read_line(&mut ack).await.unwrap();
    let ack_v: serde_json::Value = serde_json::from_str(ack.trim()).unwrap();
    assert_eq!(ack_v["ok"], serde_json::Value::Bool(true));

    // Publish twice; subscriber should see both in order.
    one_shot(
        &socket,
        "substrate.publish",
        serde_json::json!({ "path": "substrate/test/stream", "value": 1 }),
    )
    .await;
    one_shot(
        &socket,
        "substrate.notify",
        serde_json::json!({ "path": "substrate/test/stream" }),
    )
    .await;

    let mut buf1 = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut buf1))
        .await
        .unwrap()
        .unwrap();
    let first: serde_json::Value = serde_json::from_str(buf1.trim()).unwrap();
    assert_eq!(first["kind"], "publish");
    assert_eq!(first["value"], 1);

    let mut buf2 = String::new();
    tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut buf2))
        .await
        .unwrap()
        .unwrap();
    let second: serde_json::Value = serde_json::from_str(buf2.trim()).unwrap();
    assert_eq!(second["kind"], "notify");

    let _ = shutdown_tx.send(true);
}
