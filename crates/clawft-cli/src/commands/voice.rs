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
            println!("Microphone test not yet implemented (requires cpal)");
            println!("Would record for {} seconds from default mic", duration);
        }
        VoiceCommand::TestSpeak { text } => {
            println!("Speaker test not yet implemented (requires sherpa-rs TTS)");
            println!("Would speak: \"{}\"", text);
        }
        VoiceCommand::Talk => {
            handle_talk().await?;
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
    use clawft_voice_talk::{TalkConfig, live::run_live};
    use tokio_util::sync::CancellationToken;

    println!("=== ClawFT Talk Mode (native ECC graph-walk) ===");
    println!("Mic + speaker via cpal; Hermes brain at :8090.");
    println!("Press Ctrl+C to exit.\n");

    let config = TalkConfig {
        conv_id: "weft-talk".into(),
        base_system: "You are a terse, friendly voice assistant.".into(),
        ..TalkConfig::default()
    };

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nReceived Ctrl+C, shutting down Talk Mode...");
            cancel_for_signal.cancel();
        }
    });

    run_live(config, None, cancel)
        .await
        .map_err(|e| anyhow::anyhow!("talk mode: {e}"))?;

    println!("Talk Mode ended.");
    Ok(())
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
