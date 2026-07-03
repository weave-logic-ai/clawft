//! Live speak-path audio review: drive the REAL DualLayerTts (Kokoro fast
//! ack + Orpheus/Ollama slow answer) exactly as `speak_with_barge_in` does,
//! but into a WAV-writing sink — so the produced audio can be reviewed /
//! whisper-round-tripped without any cpal device in the loop. Splits the
//! "answer sounds broken" class of bug between TTS synthesis and the
//! playback layer.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use clawft_channels::voice::tts::{TtsChunk, TtsSink};
use clawft_channels::voice::types::VoiceError;
use clawft_voice_talk::native_dual_layer;

/// Collects all played chunks, resampled to nothing — raw as-played PCM
/// per chunk with its sample rate.
#[derive(Default)]
struct WavSink {
    chunks: Mutex<Vec<TtsChunk>>,
}

#[async_trait]
impl TtsSink for WavSink {
    async fn play_chunk(&self, chunk: &TtsChunk) -> Result<(), VoiceError> {
        self.chunks.lock().unwrap().push(chunk.clone());
        Ok(())
    }
    async fn flush(&self) {}
}

fn write_wav(path: &std::path::Path, pcm: &[i16], rate: u32) {
    let data_len = (pcm.len() * 2) as u32;
    let mut wav: Vec<u8> = Vec::with_capacity(44 + pcm.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        wav.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, wav).unwrap();
}

/// Simple linear resample to 24 kHz so ack (24k Kokoro) + answer (24k SNAC)
/// concatenate into one reviewable file even if rates ever diverge.
fn to_24k(samples: &[i16], rate: u32) -> Vec<i16> {
    if rate == 24_000 {
        return samples.to_vec();
    }
    let ratio = 24_000f64 / rate as f64;
    let out_len = (samples.len() as f64 * ratio) as usize;
    (0..out_len)
        .map(|i| {
            let src = i as f64 / ratio;
            let i0 = src.floor() as usize;
            let i1 = (i0 + 1).min(samples.len().saturating_sub(1));
            let frac = src - i0 as f64;
            let s = samples[i0] as f64 * (1.0 - frac) + samples[i1] as f64 * frac;
            s as i16
        })
        .collect()
}

/// Run it (operator):
/// ```text
/// cargo test -p clawft-voice-talk --features onnx --test speak_wav -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "requires staged Kokoro ONNX + live Ollama orpheus-tts + SNAC model"]
async fn live_speak_path_to_wav() {
    let tts = native_dual_layer().expect("dual layer builds");
    let sink = Arc::new(WavSink::default());

    let answer = "Two plus two equals four. That is basic arithmetic, \
                  and it never changes.";
    tts.speak(
        Some("One sec."),
        answer,
        sink.clone(),
        CancellationToken::new(),
    )
    .await
    .expect("speak path");

    let chunks = sink.chunks.lock().unwrap();
    assert!(!chunks.is_empty(), "speak produced no audio at all");
    let mut pcm: Vec<i16> = Vec::new();
    for c in chunks.iter() {
        pcm.extend(to_24k(&c.samples, c.sample_rate));
    }
    let secs = pcm.len() as f64 / 24_000.0;
    let path = std::env::temp_dir().join("clawft_speak_review.wav");
    write_wav(&path, &pcm, 24_000);
    println!(
        "speak review wav: {} ({} chunks, {:.1}s)",
        path.display(),
        chunks.len(),
        secs
    );
}
