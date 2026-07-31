//! Native dual-layer TTS construction (ADR-062 Phase 4) — bind the concrete
//! node-renderers into the generic [`DualLayerTts`] that
//! [`NodeRenderer`](crate::NodeRenderer) drives (render.rs: Speculative→fast,
//! Committed→slow).
//!
//! - **fast** = [`build_native_fast_engine`] — WEFT-613 selection: prefer
//!   Chatterbox cloned-voice when runtime-ready, else **Kokoro** (0.8 default).
//!   Preference: `WEFTOS_FAST_TTS` / `voice.tts.fast_engine` (`auto|kokoro|chatterbox`).
//! - **slow** = [`OrpheusTts`] over Ollama → [`SnacOnnxDecoder`] — the expressive
//!   Committed answer, streamed gap-free.
//!
//! Both engines auto-discover their models/endpoints and degrade gracefully:
//! absent weights or a missing Ollama surface as a `VoiceError` *at render
//! time*, so construction never requires them and stays test-friendly. The
//! heavy `ort` dependency lives in `clawft-voice-tts` (behind its `onnx`
//! feature), keeping `clawft-channels` free of it (bridge-crate layering).

use clawft_channels::voice::tts::DualLayerTts;
use clawft_channels::voice::types::VoiceError;
use clawft_voice_tts::{
    FastEnginePreference, OrpheusTts, SnacOnnxDecoder, build_native_fast_engine,
    build_native_fast_engine_with,
};
use std::sync::Arc;

/// Build a [`DualLayerTts`] from the native node-renderers: selected fast tier
/// (clone when ready, else Kokoro) covering Orpheus-over-Ollama → SNAC decode
/// (slow, expressive answer). Models/endpoints are auto-discovered; nothing is
/// required at build time. Fast preference comes from `WEFTOS_FAST_TTS` (default
/// `auto`).
pub fn native_dual_layer() -> Result<DualLayerTts, VoiceError> {
    let fast = build_native_fast_engine();
    let slow = Arc::new(OrpheusTts::new(Arc::new(SnacOnnxDecoder::new()))?);
    DualLayerTts::new(fast, slow)
}

/// Like [`native_dual_layer`] with an explicit fast-tier preference (config /
/// david profile hook: `voice.tts.fast_engine`).
pub fn native_dual_layer_with_fast_pref(
    preference: FastEnginePreference,
) -> Result<DualLayerTts, VoiceError> {
    let fast = build_native_fast_engine_with(preference);
    let slow = Arc::new(OrpheusTts::new(Arc::new(SnacOnnxDecoder::new()))?);
    DualLayerTts::new(fast, slow)
}

/// Build a [`DualLayerTts`] against an explicit Ollama generate URL / model /
/// voice for the slow Orpheus layer, with the selected native fast engine.
pub fn native_dual_layer_with_ollama(
    url: impl Into<String>,
    model: impl Into<String>,
    voice: impl Into<String>,
) -> Result<DualLayerTts, VoiceError> {
    let fast = build_native_fast_engine();
    let slow = Arc::new(OrpheusTts::with_endpoint(
        url,
        model,
        voice,
        Arc::new(SnacOnnxDecoder::new()),
    )?);
    DualLayerTts::new(fast, slow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_dual_layer_satisfies_tier_contract() {
        // Constructs without weights/Ollama; DualLayerTts::new enforces
        // fast=Fast and slow=Slow, so a clean build proves the tiers are wired.
        native_dual_layer().expect("fast + Orpheus=Slow tiers");
        native_dual_layer_with_fast_pref(FastEnginePreference::Kokoro)
            .expect("force Kokoro fast + Orpheus slow");
        native_dual_layer_with_fast_pref(FastEnginePreference::Auto)
            .expect("auto fast (0.8 → Kokoro) + Orpheus slow");
        native_dual_layer_with_ollama("http://127.0.0.1:11434/api/generate", "orpheus-tts", "dan")
            .expect("explicit Ollama endpoint builds");
    }
}
