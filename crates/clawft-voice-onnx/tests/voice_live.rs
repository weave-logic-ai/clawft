//! Live native-voice tests: require the ONNX models + the `onnx` feature, so
//! they are `#[ignore]`d (the default `cargo test` run needs no weights). Run:
//!
//! ```text
//! cargo test -p clawft-voice-onnx --features onnx -- --ignored --nocapture
//! ```
//!
//! - parakeet-tdt-0.6b: place the sherpa-onnx bundle under
//!   `.weftos/models/parakeet/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/`
//!   (or set `WEFTOS_PARAKEET_DIR`).
//! - smart-turn-v3: place `smart-turn-v3.1-cpu.onnx` under
//!   `.weftos/models/smart-turn/` (or set `WEFTOS_SMART_TURN_MODEL`).
//!
//! The fixture `tests/fixtures/jfk_en.wav` is a committed 24 kHz mono clip of
//! the JFK line ("ask not what your country can do for you …"); the backends
//! resample/window it internally.

#![cfg(feature = "onnx")]

use std::path::PathBuf;

use clawft_channels::voice::stt::{SttBackend, Utterance};
use clawft_channels::voice::turn::EndpointModel;
use clawft_voice_onnx::{ParakeetStt, SmartTurnEndpoint};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn models() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.weftos/models")
}

fn read_wav(name: &str) -> (Vec<i16>, u32) {
    let path = fixtures().join(name);
    let mut r =
        hound::WavReader::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let sr = r.spec().sample_rate;
    (r.samples::<i16>().map(|s| s.unwrap()).collect(), sr)
}

/// Linear resample to 16 kHz (the `EndpointModel` trait contract feeds 16 kHz).
fn to_16k(s: &[i16], from: u32) -> Vec<i16> {
    if from == 16_000 {
        return s.to_vec();
    }
    let ratio = 16_000f64 / from as f64;
    let out = ((s.len() as f64) * ratio).round() as usize;
    (0..out)
        .map(|i| {
            let src = i as f64 / ratio;
            let i0 = src.floor() as usize;
            let f = (src - i0 as f64) as f32;
            let a = *s.get(i0).unwrap_or(&0) as f32;
            let b = *s.get(i0 + 1).unwrap_or(&(a as i16)) as f32;
            (a + (b - a) * f).round() as i16
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires parakeet ONNX bundle under .weftos/models/parakeet/"]
async fn parakeet_transcribes_fixture() {
    let dir = models().join("parakeet/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8");
    let stt = ParakeetStt::with_dir(&dir);
    assert!(stt.is_runtime_available(), "parakeet model failed to load");
    stt.warm().await.unwrap();

    let (samples, sr) = read_wav("jfk_en.wav");
    let text = stt
        .transcribe(&Utterance {
            samples,
            sample_rate: sr,
        })
        .await
        .unwrap()
        .to_lowercase();
    // The TDT decode must recover the salient content words.
    for w in ["country", "ask", "you"] {
        assert!(text.contains(w), "transcript missing {w:?}: {text:?}");
    }
}

#[tokio::test]
#[ignore = "requires smart-turn ONNX under .weftos/models/smart-turn/"]
async fn smart_turn_separates_complete_from_midword() {
    let path = models().join("smart-turn/smart-turn-v3.1-cpu.onnx");
    let ep = SmartTurnEndpoint::with_model(&path);
    assert!(ep.is_runtime_available(), "smart-turn model failed to load");

    let (raw, sr) = read_wav("jfk_en.wav");
    let s16 = to_16k(&raw, sr);
    // ~1.8 s ends a complete sentence ("…can do for you."); ~1.0 s cuts a word
    // mid-phrase ("…what your"). Complete should read far more "done".
    let complete = &s16[..(16_000 * 18 / 10).min(s16.len())];
    let midword = &s16[..(16_000).min(s16.len())];
    let p_complete = ep.completion_prob(complete, "").await;
    let p_midword = ep.completion_prob(midword, "").await;
    assert!(
        p_complete > 0.5,
        "complete sentence should read done (got {p_complete:.3})"
    );
    assert!(
        p_midword < 0.5,
        "mid-word cut should read open (got {p_midword:.3})"
    );
    assert!(
        p_complete - p_midword > 0.3,
        "complete ({p_complete:.3}) must clearly exceed mid-word ({p_midword:.3})"
    );
}
