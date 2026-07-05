//! Voice CLI commands: setup, test-mic, test-speak, talk, wake.
//!
//! Provides `weft voice <subcommand>` for managing the voice pipeline.
//! All commands are gated behind the `voice` feature flag.

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct VoiceArgs {
    #[command(subcommand)]
    pub command: VoiceCommand,
}

#[derive(Debug, Subcommand)]
pub enum VoiceCommand {
    /// Set up voice pipeline (download models, test audio).
    Setup,

    /// Test microphone input.
    TestMic {
        /// Duration in seconds.
        #[arg(short, long, default_value = "5")]
        duration: u32,
    },

    /// Test speaker output.
    TestSpeak {
        /// Text to speak.
        #[arg(short, long, default_value = "Hello, I am ClawFT.")]
        text: String,
    },

    /// Start Talk Mode (continuous voice conversation).
    Talk,

    /// Listen-only mode (§W1.4): record + decompose + classify every turn WITHOUT
    /// the LLM brain or audio out, rendering the live process stream (partials,
    /// endpoint fire, finalized turn + its decomposition) as it happens. Turns
    /// anchor via `agent.turn.record`, so `weft voice watch` lights up in parallel.
    Listen,

    /// Watch the live voice process for a conversation (§W1.4): committed turns,
    /// their labels, and the per-utterance voice decomposition, rendered as they
    /// land via the ~1 Hz `conversation.graph` poll. Read-only — no mic, no
    /// brain; run it alongside `weft voice talk` or an `agent.turn.record` feed.
    Watch {
        /// Conversation id to watch (the Talk-Mode conv, e.g. `weft-talk`).
        #[arg(default_value = "weft-talk")]
        conv_id: String,
        /// Emit each newly-committed node's full record as JSON (one per line).
        #[arg(long)]
        json: bool,
        /// Committed-state poll cadence in milliseconds (ADR-067 D2 ~1 Hz).
        #[arg(long, default_value = "1000")]
        interval: u64,
    },

    /// Start the wake word daemon (listen for "Hey Weft").
    Wake,

    /// Install the wake word daemon as a system service.
    InstallService {
        /// Service manager to use (auto-detected if not specified).
        /// Supported values: "systemd", "launchd".
        #[arg(long)]
        manager: Option<String>,
    },
}

pub async fn handle_voice(args: VoiceArgs) -> anyhow::Result<()> {
    match args.command {
        VoiceCommand::Setup => {
            println!("Voice setup not yet implemented (requires VP validation)");
            println!("This will download STT/TTS/VAD models to ~/.clawft/models/voice/");
        }
        VoiceCommand::TestMic { duration } => {
            handle_test_mic(duration).await?;
        }
        VoiceCommand::TestSpeak { text } => {
            println!("Speaker test not yet implemented (requires sherpa-rs TTS)");
            println!("Would speak: \"{}\"", text);
        }
        VoiceCommand::Talk => {
            handle_talk().await?;
        }
        VoiceCommand::Listen => {
            handle_listen().await?;
        }
        VoiceCommand::Watch {
            conv_id,
            json,
            interval,
        } => {
            super::voice_watch::handle_watch(conv_id, json, interval).await?;
        }
        VoiceCommand::Wake => {
            handle_wake().await?;
        }
        VoiceCommand::InstallService { manager } => {
            handle_install_service(manager).await?;
        }
    }
    Ok(())
}

/// Run Talk Mode — the native ECC graph-walk voice conversation (ADR-062).
///
/// Constructs the full `weft talk` assembly via `clawft_voice_talk` — the P2
/// Talk-Mode loop orchestrator + the concrete native stack (parakeet STT,
/// smart-turn endpoint, ECAPA speaker, Kokoro+Orpheus TTS, LocalProvider→Hermes
/// brain) over a cpal mic+speaker — and runs the spoken loop until Ctrl+C.
///
/// The graph constructs cleanly with no weights staged (each native component
/// degrades gracefully); real transcription/synthesis needs the staged ONNX
/// models and the `voice-onnx` build feature (`weft` built with `--features
/// voice-onnx`) plus live Hermes at `:8090`.
async fn handle_talk() -> anyhow::Result<()> {
    use clawft_voice_talk::{TalkConfig, live::run_live_observed};
    use tokio_util::sync::CancellationToken;

    println!("=== ClawFT Talk Mode (native ECC graph-walk) ===");
    println!("Mic + speaker via cpal; Hermes brain at :8090.");
    println!("Press Ctrl+C to exit.\n");

    let config = TalkConfig {
        conv_id: "weft-talk".into(),
        base_system: "You are a terse, friendly voice assistant.".into(),
        ..TalkConfig::default()
    };

    // Mirror committed turns to the kernel daemon's `agent.turn.record`
    // RPC so the exchange lands on the substrate JSONL and anchors to the
    // witness chain / HNSW / causal graph per `[kernel.agent]`. Best-effort:
    // without a running daemon the conversation still works, unanchored.
    let recorder = spawn_turn_recorder(config.conv_id.clone()).await;
    match &recorder {
        Some(_) => println!("Turn anchoring: ON (daemon connected — turns recorded on chain)"),
        None => println!(
            "Turn anchoring: OFF (no kernel daemon — start one with `weaver kernel start` \
             to record turns on the chain)"
        ),
    }

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nReceived Ctrl+C, shutting down Talk Mode...");
            cancel_for_signal.cancel();
        }
    });

    run_live_observed(config, None, cancel, recorder)
        .await
        .map_err(|e| anyhow::anyhow!("talk mode: {e}"))?;

    println!("Talk Mode ended.");
    Ok(())
}

/// Run Listen-only Mode (§W1.4): the native capture + decomposition + classify
/// path with the LLM brain and audio-out disabled (`TalkConfig::listen_only`).
/// Renders the live process stream via [`LiveRenderObserver`] and mirrors each
/// finalized turn to `agent.turn.record` (fanned out with the recorder), so the
/// forest fills and `weft voice watch` lights up in parallel — no reply, no TTS.
async fn handle_listen() -> anyhow::Result<()> {
    use clawft_voice_talk::{ConversationObserver, TalkConfig, live::run_live_observed};
    use tokio_util::sync::CancellationToken;

    use super::voice_watch::{FanOutObserver, LiveRenderObserver};

    println!("=== ClawFT Listen Mode (observe-only — no brain, no audio out) ===");
    println!("Records + decomposes + classifies every turn and renders it live.");
    println!("Press Ctrl+C to exit.\n");

    let config = TalkConfig {
        conv_id: "weft-talk".into(),
        listen_only: true,
        ..TalkConfig::default()
    };

    // Anchor finalized turns to the daemon (best-effort) AND render them live.
    let recorder = spawn_turn_recorder(config.conv_id.clone()).await;
    match &recorder {
        Some(_) => println!("Turn anchoring: ON (turns recorded on the chain)"),
        None => println!(
            "Turn anchoring: OFF (no kernel daemon — start one with `weaver kernel start` \
             to record turns; the live stream still renders)"
        ),
    }

    // `run_live_observed` takes ONE extra observer; fan out recorder + renderer.
    let mut observers: Vec<std::sync::Arc<dyn ConversationObserver>> =
        vec![std::sync::Arc::new(LiveRenderObserver)];
    if let Some(recorder) = recorder {
        observers.push(recorder);
    }
    let observer: std::sync::Arc<dyn ConversationObserver> =
        std::sync::Arc::new(FanOutObserver(observers));

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nReceived Ctrl+C, shutting down Listen Mode...");
            cancel_for_signal.cancel();
        }
    });

    run_live_observed(config, None, cancel, Some(observer))
        .await
        .map_err(|e| anyhow::anyhow!("listen mode: {e}"))?;

    println!("Listen Mode ended.");
    Ok(())
}

/// Probe the microphone the live Talk-Mode capture would open: list input
/// devices, record for `duration` seconds, and print a per-half-second level
/// meter plus a verdict against the Talk-Mode VAD threshold (-45 dBFS).
async fn handle_test_mic(duration: u32) -> anyhow::Result<()> {
    use clawft_voice_talk::live::{list_input_devices, mic_probe};

    /// Talk-Mode's default VAD onset threshold (TalkConfig::default).
    const VAD_THRESHOLD_DBFS: f32 = -45.0;

    println!("Input devices:");
    for name in list_input_devices() {
        println!("  - {name}");
    }
    println!("\nRecording {duration}s from the default device — SPEAK NOW...\n");

    let report = tokio::task::spawn_blocking(move || mic_probe(None, duration))
        .await?
        .map_err(|e| anyhow::anyhow!("mic probe: {e}"))?;

    println!(
        "Opened: {} ({} Hz, {} ch)\n",
        report.device, report.native_rate, report.channels
    );
    for (i, db) in report.windows_dbfs.iter().enumerate() {
        let over = *db >= VAD_THRESHOLD_DBFS;
        let bar_len = ((db + 90.0).max(0.0) / 2.0) as usize;
        println!(
            "  {:>4.1}s  {:>7.1} dBFS  {}{}",
            (i as f32 + 1.0) * 0.5,
            db,
            "█".repeat(bar_len.min(40)),
            if over { "  << voice" } else { "" },
        );
    }
    println!();
    if report.windows_dbfs.is_empty() {
        println!("NO AUDIO DELIVERED — the device produced no samples at all.");
        println!("Check System Settings → Privacy & Security → Microphone for your terminal.");
    } else if report.peak_dbfs < VAD_THRESHOLD_DBFS {
        println!(
            "Peak {:.1} dBFS is BELOW the Talk-Mode VAD threshold ({VAD_THRESHOLD_DBFS} dBFS) — \
             the voice loop will never trigger at this level.",
            report.peak_dbfs
        );
        println!(
            "Fix: raise input volume in System Settings → Sound → Input, or select a \
             different mic (this probe used the system default)."
        );
    } else {
        println!(
            "Peak {:.1} dBFS clears the VAD threshold ({VAD_THRESHOLD_DBFS} dBFS) — \
             the voice loop should hear you on this device.",
            report.peak_dbfs
        );
    }
    Ok(())
}

/// Non-blocking [`ConversationObserver`] that forwards user / committed-
/// assistant turns into an unbounded queue; a background poster task posts
/// each to the daemon's `agent.turn.record` RPC.
struct TurnRecordObserver {
    tx: tokio::sync::mpsc::UnboundedSender<(&'static str, String, Option<serde_json::Value>)>,
}

impl clawft_voice_talk::ConversationObserver for TurnRecordObserver {
    fn observe(&self, event: clawft_voice_talk::ConversationEvent) {
        use clawft_voice_talk::ConversationEvent;
        let triple = match event {
            ConversationEvent::UserTurn {
                text,
                voice_analysis,
                ..
            } => {
                // Wave 1 §W1.2: serialize the per-utterance decomposition onto
                // the wire so the daemon's `index_turn` stores it as a sibling
                // key and merges its emotion axis (tier:"voice"). Best-effort —
                // a serialization failure drops only the record, never the turn.
                let va = voice_analysis
                    .as_deref()
                    .and_then(|v| serde_json::to_value(v).ok());
                ("user", text, va)
            }
            ConversationEvent::CommittedReply { answer } => ("assistant", answer, None),
            // Speculative acks are superseded by the committed reply and
            // barge-ins prune in-forest; neither is a durable turn.
            _ => return,
        };
        // Receiver gone (daemon died and the poster exited) — drop silently;
        // anchoring is best-effort by design.
        let _ = self.tx.send(triple);
    }
}

/// Connect to the kernel daemon and spawn the poster task. Returns `None`
/// (anchoring disabled) when no daemon is reachable.
async fn spawn_turn_recorder(
    conv_id: String,
) -> Option<std::sync::Arc<dyn clawft_voice_talk::ConversationObserver>> {
    use clawft_rpc::{DaemonClient, Request};

    let mut client = DaemonClient::connect().await?;
    let (tx, mut rx) =
        tokio::sync::mpsc::unbounded_channel::<(&'static str, String, Option<serde_json::Value>)>();

    tokio::spawn(async move {
        while let Some((role, content, voice_analysis)) = rx.recv().await {
            let mut turn = serde_json::json!({ "role": role, "content": content });
            if let Some(va) = voice_analysis {
                turn["voice_analysis"] = va;
            }
            let params = serde_json::json!({
                "conv_id": conv_id,
                "channel": "voice.talk",
                "turns": [turn],
            });
            // One reconnect attempt per turn: a dropped socket loses no
            // event, a stopped daemon ends anchoring for the session.
            let mut posted = false;
            for attempt in 0..2 {
                let request = Request::with_params("agent.turn.record", params.clone());
                match client.call(request).await {
                    Ok(resp) => {
                        if let Err(e) = resp.into_result() {
                            tracing::warn!(error = %e, role, "agent.turn.record rejected");
                        }
                        posted = true;
                        break;
                    }
                    Err(e) if attempt == 0 => {
                        tracing::debug!(error = %e, "agent.turn.record transport error; reconnecting");
                        match DaemonClient::connect().await {
                            Some(c) => client = c,
                            None => break,
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "agent.turn.record transport error after reconnect");
                    }
                }
            }
            if !posted && DaemonClient::connect().await.is_none() {
                tracing::warn!("kernel daemon gone — voice turn anchoring stopped for this session");
                return;
            }
        }
    });

    Some(std::sync::Arc::new(TurnRecordObserver { tx }))
}

/// Run the wake word daemon -- continuously listen for "Hey Weft".
///
/// Creates a WakeDaemon and runs until Ctrl+C. When the wake word
/// is detected (after real rustpotter integration), Talk Mode will
/// be activated automatically.
async fn handle_wake() -> anyhow::Result<()> {
    use clawft_plugin::traits::CancellationToken;
    use clawft_plugin::voice::{WakeDaemon, WakeWordConfig};

    println!("=== ClawFT Wake Word Daemon ===");
    println!("Wake word daemon started (stub - listening for 'Hey Weft')");
    println!("Press Ctrl+C to exit.\n");

    let config = WakeWordConfig::default();
    let mut daemon = WakeDaemon::new(config)?;
    let cancel = CancellationToken::new();

    // Handle Ctrl+C for graceful shutdown.
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        println!("\nReceived Ctrl+C, shutting down wake word daemon...");
        cancel_for_signal.cancel();
    });

    daemon.run(cancel).await?;

    println!("Wake word daemon stopped.");
    Ok(())
}

/// Install the wake word daemon as a platform service.
///
/// Auto-detects the platform (Linux/macOS/Windows) and installs the
/// appropriate service definition. On Linux this is a systemd user unit;
/// on macOS it is a launchd plist in ~/Library/LaunchAgents.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_install_service(manager: Option<String>) -> anyhow::Result<()> {
    let detected = manager.unwrap_or_else(detect_service_manager);

    match detected.as_str() {
        "systemd" => install_systemd_service().await,
        "launchd" => install_launchd_service().await,
        other => {
            println!("Unsupported service manager: {}", other);
            println!("Manual installation required.");
            println!();
            println!("On Windows:");
            println!("  1. Open Task Scheduler");
            println!("  2. Create a new task that runs: weft voice wake --daemon");
            println!("  3. Set it to run at startup");
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn handle_install_service(_manager: Option<String>) -> anyhow::Result<()> {
    println!("Service installation is not available on WASM targets.");
    Ok(())
}

/// Detect the service manager for the current platform.
fn detect_service_manager() -> String {
    if cfg!(target_os = "macos") {
        "launchd".to_string()
    } else if cfg!(target_os = "linux") {
        "systemd".to_string()
    } else {
        "unsupported".to_string()
    }
}

/// Install a systemd user service for the wake word daemon.
async fn install_systemd_service() -> anyhow::Result<()> {
    use std::path::PathBuf;

    let home =
        std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let service_dir = PathBuf::from(&home)
        .join(".config")
        .join("systemd")
        .join("user");

    // Create the service directory if it doesn't exist.
    tokio::fs::create_dir_all(&service_dir).await?;

    let service_path = service_dir.join("clawft-wake.service");
    let service_content = include_str!("../../../../scripts/clawft-wake.service");

    tokio::fs::write(&service_path, service_content).await?;
    println!("Installed service file to: {}", service_path.display());

    // Try to enable and start the service.
    println!("Enabling clawft-wake.service...");
    let enable_result = tokio::process::Command::new("systemctl")
        .args(["--user", "enable", "clawft-wake.service"])
        .status()
        .await;

    match enable_result {
        Ok(status) if status.success() => {
            println!("Service enabled successfully.");
            println!();
            println!("Commands:");
            println!("  systemctl --user start clawft-wake   # Start now");
            println!("  systemctl --user stop clawft-wake    # Stop");
            println!("  systemctl --user status clawft-wake  # Check status");
            println!("  journalctl --user -u clawft-wake     # View logs");
        }
        _ => {
            println!("Could not enable service via systemctl.");
            println!("You may need to enable it manually:");
            println!("  systemctl --user enable clawft-wake.service");
            println!("  systemctl --user start clawft-wake.service");
        }
    }

    Ok(())
}

/// Install a launchd plist for the wake word daemon (macOS).
async fn install_launchd_service() -> anyhow::Result<()> {
    use std::path::PathBuf;

    let home =
        std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let agents_dir = PathBuf::from(&home).join("Library").join("LaunchAgents");

    // Create the LaunchAgents directory if it doesn't exist.
    tokio::fs::create_dir_all(&agents_dir).await?;

    let plist_path = agents_dir.join("com.clawft.wake.plist");
    let plist_content = include_str!("../../../../scripts/com.clawft.wake.plist");

    tokio::fs::write(&plist_path, plist_content).await?;
    println!("Installed plist to: {}", plist_path.display());

    // Try to load the service.
    println!("Loading com.clawft.wake...");
    let load_result = tokio::process::Command::new("launchctl")
        .args(["load", &plist_path.to_string_lossy()])
        .status()
        .await;

    match load_result {
        Ok(status) if status.success() => {
            println!("Service loaded successfully.");
            println!();
            println!("Commands:");
            println!("  launchctl start com.clawft.wake   # Start now");
            println!("  launchctl stop com.clawft.wake    # Stop");
            println!("  launchctl list | grep clawft      # Check status");
        }
        _ => {
            println!("Could not load service via launchctl.");
            println!("You may need to load it manually:");
            println!("  launchctl load {}", plist_path.display());
        }
    }

    Ok(())
}
