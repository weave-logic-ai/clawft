//! Text-to-speech via sherpa-rs streaming synthesizer.
//!
//! # WEFT-222
//!
//! Per-agent voice parameters live in
//! [`clawft_types::config::VoiceConfig::personalities`]. Prefer the live
//! path in `clawft-channels::voice::{PersonalityTtsDispatch, SubstrateTts}`
//! for product Talk Mode. This scaffold engine accepts the same resolved
//! voice (`voice_id` / `speed` / `pitch`) so umbrella tests and any remaining
//! plugin callers stay consistent.

use clawft_types::config::{ResolvedTtsVoice, VoiceConfig, VoicePersonality};

/// TTS synthesis result.
#[derive(Debug, Clone)]
pub struct TtsResult {
    /// Audio samples (f32, mono, at configured sample rate).
    pub samples: Vec<f32>,
    /// Sample rate of the output audio.
    pub sample_rate: u32,
}

/// Abort handle for cancelling TTS playback.
#[derive(Clone)]
pub struct TtsAbortHandle {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TtsAbortHandle {
    pub fn new() -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for TtsAbortHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming text-to-speech engine.
///
/// Currently a stub -- real sherpa-rs integration is deferred to the
/// 0.8.x in-process voice backend (see ADR-053). Live product TTS is
/// `clawft-channels` / `clawft-voice-tts` (WEFT-671).
pub struct TextToSpeech {
    model_path: std::path::PathBuf,
    voice: String,
    speed: f32,
    pitch: f32,
    /// Greeting already spoken for this engine's session (WEFT-222).
    greeting_consumed: bool,
    greeting_prefix: Option<String>,
}

impl TextToSpeech {
    pub fn new(model_path: std::path::PathBuf, voice: String, speed: f32) -> Self {
        Self {
            model_path,
            voice,
            speed,
            pitch: 0.0,
            greeting_consumed: false,
            greeting_prefix: None,
        }
    }

    /// Construct from a resolved per-agent personality (WEFT-222).
    pub fn from_resolved(model_path: std::path::PathBuf, resolved: &ResolvedTtsVoice) -> Self {
        Self {
            model_path,
            voice: resolved.voice_id.clone(),
            speed: resolved.speed,
            pitch: resolved.pitch,
            greeting_consumed: false,
            greeting_prefix: resolved.greeting_prefix.clone(),
        }
    }

    /// Construct by looking up `agent_id` in [`VoiceConfig::personalities`].
    pub fn from_voice_config(
        model_path: std::path::PathBuf,
        config: &VoiceConfig,
        agent_id: &str,
    ) -> Self {
        Self::from_resolved(model_path, &config.resolve_tts_for_agent(agent_id))
    }

    /// Construct from a raw personality map entry.
    pub fn from_personality(
        model_path: std::path::PathBuf,
        agent_id: &str,
        personality: &VoicePersonality,
    ) -> Self {
        Self::from_resolved(
            model_path,
            &ResolvedTtsVoice::from_personality(agent_id, personality),
        )
    }

    /// Take session-start greeting once (if configured). Returns `None`
    /// after the first successful take.
    pub fn take_session_greeting(&mut self) -> Option<String> {
        if self.greeting_consumed {
            return None;
        }
        let g = self
            .greeting_prefix
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        if g.is_some() {
            self.greeting_consumed = true;
        }
        g
    }

    /// Text prepared for synthesis: prefixes greeting once when
    /// `session_start` is true.
    pub fn prepare_text(&mut self, body: &str, session_start: bool) -> String {
        if session_start {
            if let Some(g) = self.take_session_greeting() {
                let body = body.trim();
                if body.is_empty() {
                    return g;
                }
                return format!("{g} {body}");
            }
        }
        body.to_string()
    }

    /// Synthesize text to audio samples.
    pub fn synthesize(&self, _text: &str) -> Result<TtsResult, String> {
        // Stub: real sherpa-rs synthesis goes here
        Ok(TtsResult {
            samples: vec![],
            sample_rate: 16000,
        })
    }

    /// Synthesize with an abort handle for interruption support.
    pub fn synthesize_with_abort(
        &self,
        _text: &str,
        _abort: &TtsAbortHandle,
    ) -> Result<TtsResult, String> {
        // Stub
        Ok(TtsResult {
            samples: vec![],
            sample_rate: 16000,
        })
    }

    pub fn model_path(&self) -> &std::path::Path {
        &self.model_path
    }

    pub fn voice(&self) -> &str {
        &self.voice
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn two_personalities() -> VoiceConfig {
        let mut cfg = VoiceConfig::default();
        cfg.personalities = HashMap::from([
            (
                "alpha".into(),
                VoicePersonality {
                    voice_id: "amy".into(),
                    speed: 1.1,
                    pitch: 0.2,
                    greeting_prefix: Some("Alpha online.".into()),
                    ..Default::default()
                },
            ),
            (
                "beta".into(),
                VoicePersonality {
                    voice_id: "ryan".into(),
                    speed: 0.9,
                    pitch: -0.25,
                    greeting_prefix: Some("Beta online.".into()),
                    ..Default::default()
                },
            ),
        ]);
        cfg
    }

    #[test]
    fn from_voice_config_two_personalities() {
        let cfg = two_personalities();
        let path = PathBuf::from("/tmp/model");
        let alpha = TextToSpeech::from_voice_config(path.clone(), &cfg, "alpha");
        let beta = TextToSpeech::from_voice_config(path, &cfg, "beta");
        assert_eq!(alpha.voice(), "amy");
        assert_eq!(beta.voice(), "ryan");
        assert!((alpha.speed() - 1.1).abs() < f32::EPSILON);
        assert!((beta.speed() - 0.9).abs() < f32::EPSILON);
        assert!((alpha.pitch() - 0.2).abs() < f32::EPSILON);
        assert!((beta.pitch() - (-0.25)).abs() < f32::EPSILON);
    }

    #[test]
    fn greeting_consumed_once_at_session_start() {
        let cfg = two_personalities();
        let mut tts = TextToSpeech::from_voice_config(PathBuf::from("m"), &cfg, "alpha");
        assert_eq!(
            tts.prepare_text("Hello user.", true),
            "Alpha online. Hello user."
        );
        assert_eq!(tts.prepare_text("Second turn.", true), "Second turn.");
        assert!(tts.take_session_greeting().is_none());
    }
}
