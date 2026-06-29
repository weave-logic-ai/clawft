//! Native dual-layer TTS construction (ADR-062 Phase 4) — bind the concrete
//! node-renderers into the generic [`DualLayerTts`] that
//! [`NodeRenderer`](crate::NodeRenderer) drives (render.rs: Speculative→fast,
//! Committed→slow).
//!
//! - **fast** = [`KokoroTts`] — the preset Speculative ack (low TTFA).
//! - **slow** = [`OrpheusTts`] over Ollama → [`SnacOnnxDecoder`] — the expressive
//!   Committed answer, streamed gap-free.
//!
//! Both engines auto-discover their models/endpoints and degrade gracefully:
//! absent weights or a missing Ollama surface as a `VoiceError` *at render
//! time*, so construction never requires them and stays test-friendly. The
//! heavy `ort` dependency lives in `clawft-voice-tts` (behind its `onnx`
//! feature), keeping `clawft-channels` free of it (bridge-crate layering).

use std::sync::Arc;

use clawft_channels::voice::tts::DualLayerTts;
use clawft_channels::voice::types::VoiceError;
use clawft_voice_tts::{KokoroTts, OrpheusTts, SnacOnnxDecoder};

/// Build a [`DualLayerTts`] from the native node-renderers: Kokoro (fast,
/// preset ack) covering Orpheus-over-Ollama → SNAC decode (slow, expressive
/// answer). Models/endpoints are auto-discovered; nothing is required at build
/// time.
pub fn native_dual_layer() -> Result<DualLayerTts, VoiceError> {
    let fast = Arc::new(KokoroTts::new());
    let slow = Arc::new(OrpheusTts::new(Arc::new(SnacOnnxDecoder::new()))?);
    DualLayerTts::new(fast, slow)
}

/// Build a [`DualLayerTts`] against an explicit Ollama generate URL / model /
/// voice for the slow Orpheus layer, with Kokoro as the fast layer.
pub fn native_dual_layer_with_ollama(
    url: impl Into<String>,
    model: impl Into<String>,
    voice: impl Into<String>,
) -> Result<DualLayerTts, VoiceError> {
    let fast = Arc::new(KokoroTts::new());
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
        // fast=Fast (Kokoro) and slow=Slow (Orpheus), so a clean build proves
        // the tiers are wired correctly.
        native_dual_layer().expect("Kokoro=Fast + Orpheus=Slow tiers");
        native_dual_layer_with_ollama("http://127.0.0.1:11434/api/generate", "orpheus-tts", "dan")
            .expect("explicit Ollama endpoint builds");
    }
}
