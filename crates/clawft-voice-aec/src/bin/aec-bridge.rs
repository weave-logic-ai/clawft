//! `aec-bridge` — native WebRTC-AEC full-duplex audio bridge.
//!
//! Owns mic capture + speaker playback (cpal/CoreAudio) and runs the
//! WebRTC APM (AEC3 + NS + AGC). STDIO protocol mirrors
//! `voicelab/native/vpio.py`:
//!   stdin  <- int16 mono 16 kHz LE : audio to play (also the AEC reference)
//!   stdout -> int16 mono 16 kHz LE : cleaned, echo-cancelled mic
//!   stderr -> status / errors only
//!   SIGUSR1-> barge-in flush (drop queued playback + render reference)
//!
//! Device override: `--device <name-substr>` or env `VOICELAB_AUDIO_DEVICE`
//! (applies to the input mic). `--out-device <name>` overrides the speaker.

use std::process::ExitCode;

use clawft_voice_aec::{run, Config};

fn main() -> ExitCode {
    let mut cfg = Config::default();

    // env first (lowest precedence), then args override.
    if let Ok(v) = std::env::var("VOICELAB_AUDIO_DEVICE")
        && !v.is_empty()
    {
        cfg.input_device = Some(v);
    }

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--device" | "-d" => cfg.input_device = args.next(),
            "--out-device" => cfg.output_device = args.next(),
            "-h" | "--help" => {
                eprintln!(
                    "aec-bridge — native WebRTC-AEC audio bridge\n\
                     usage: aec-bridge [--device <mic-name-substr>] [--out-device <name-substr>]\n\
                     stdin: int16 mono 16k LE (play + AEC ref); stdout: int16 mono 16k LE (cleaned mic)\n\
                     SIGUSR1: barge-in flush. env VOICELAB_AUDIO_DEVICE overrides --device."
                );
                return ExitCode::SUCCESS;
            }
            other => eprintln!("[aec] ignoring unknown arg: {other}"),
        }
    }

    match run(cfg) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[aec] ERROR: {e}");
            ExitCode::from(3)
        }
    }
}
