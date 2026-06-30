//! Concrete native-component bindings for [`TalkSession`](crate::TalkSession)
//! (ADR-062 Phase 6.1) — the all-native, no-Python stack.
//!
//! [`native_components`] assembles the config-selectable native backends:
//! parakeet STT, smart-turn endpoint, ECAPA speaker, the Kokoro+Orpheus
//! [`DualLayerTts`], and the Hermes [`LocalProviderVoiceLlm`]. Every one
//! auto-discovers its model/endpoint and degrades gracefully when absent, so
//! this builds the full graph cleanly with no weights staged — the assembly
//! contract for `weft talk`.
//!
//! The playback `sink` and barge-in `audio` are **injected** (not built here):
//! the cpal mic/speaker wiring is heavyweight and lives behind the `live-audio`
//! feature ([`crate::live`]); a WAV-driven or recording sink needs no devices.

use std::sync::Arc;

use clawft_channels::voice::talkmode::AudioControl;
use clawft_channels::voice::tts::TtsSink;
use clawft_channels::voice::turn::SemanticEndpointer;
use clawft_channels::voice::types::VoiceError;
use clawft_voice_onnx::{EcapaEmbedder, ParakeetStt, SmartTurnEndpoint};

use crate::llm::LocalProviderVoiceLlm;
use crate::session::{TalkComponents, TalkConfig};
use crate::tts::native_dual_layer;

/// Build the native component set (parakeet / smart-turn / ECAPA / Kokoro+
/// Orpheus / Hermes) for `config`, with an injected playback `sink` and
/// barge-in `audio`. Construction requires no weights or live endpoints.
pub fn native_components(
    config: &TalkConfig,
    sink: Arc<dyn TtsSink>,
    audio: Arc<dyn AudioControl>,
) -> Result<TalkComponents<SmartTurnEndpoint>, VoiceError> {
    let endpointer = SemanticEndpointer::new(
        SmartTurnEndpoint::new(),
        config.sample_rate,
        config.short_silence_ms,
        config.max_silence_ms,
        config.endpoint_threshold,
    );
    let embedder: Arc<dyn clawft_channels::voice::speaker::SpeakerEmbedder> =
        Arc::new(EcapaEmbedder::new());
    Ok(TalkComponents {
        endpointer,
        stt: Arc::new(ParakeetStt::new()),
        embedder: Some(embedder),
        llm: Arc::new(LocalProviderVoiceLlm::hermes()),
        tts: native_dual_layer()?,
        sink,
        audio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_channels::voice::talkmode::NoopAudioControl;
    use clawft_channels::voice::tts::TtsChunk;

    /// A playback sink that drops everything — proves the native stack
    /// assembles and runs without any audio device.
    struct NullSink;
    #[async_trait::async_trait]
    impl TtsSink for NullSink {
        async fn play_chunk(&self, _chunk: &TtsChunk) -> Result<(), VoiceError> {
            Ok(())
        }
        async fn flush(&self) {}
    }

    #[test]
    fn native_stack_assembles_without_weights() {
        let cfg = TalkConfig::default();
        let comps = native_components(&cfg, Arc::new(NullSink), Arc::new(NoopAudioControl))
            .expect("native components build with no weights/endpoint");
        assert!(comps.embedder.is_some(), "ECAPA speaker is wired");
    }
}
