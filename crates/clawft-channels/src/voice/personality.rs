//! Per-agent [`VoicePersonality`] lookup for TTS dispatch (WEFT-222).
//!
//! [`VoiceConfig::personalities`] is validated at config load but historically
//! never consulted on the speak path. This module is the live-stack consumer:
//! resolve `voice_id` / `speed` / `pitch` by `agent_id`, and **consume**
//! `greeting_prefix` exactly once at session start.

use clawft_types::config::{ResolvedTtsVoice, VoiceConfig, VoicePersonality};

/// Session-scoped TTS personality dispatch.
///
/// Holds a snapshot of the personalities map (plus global TTS defaults) and
/// the active `agent_id`. Greeting is tracked so it is spoken only once per
/// agent session — not on every turn.
#[derive(Debug, Clone)]
pub struct PersonalityTtsDispatch {
    /// Source config (personalities + global `tts` defaults for unknown agents).
    voice: VoiceConfig,
    /// Active agent whose voice parameters apply to the next synthesis.
    agent_id: String,
    /// Whether `greeting_prefix` has already been taken for the current agent.
    greeting_consumed: bool,
}

impl PersonalityTtsDispatch {
    /// Build from a full [`VoiceConfig`] and the session's speaking agent.
    pub fn new(voice: VoiceConfig, agent_id: impl Into<String>) -> Self {
        Self {
            voice,
            agent_id: agent_id.into(),
            greeting_consumed: false,
        }
    }

    /// Convenience: personalities map only (global TTS defaults empty).
    pub fn from_personalities(
        personalities: impl IntoIterator<Item = (String, VoicePersonality)>,
        agent_id: impl Into<String>,
    ) -> Self {
        let voice = VoiceConfig {
            personalities: personalities.into_iter().collect(),
            ..Default::default()
        };
        Self::new(voice, agent_id)
    }

    /// Active agent id.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Switch speaking agent; resets greeting consumption for the new agent.
    pub fn set_agent(&mut self, agent_id: impl Into<String>) {
        let next = agent_id.into();
        if next != self.agent_id {
            self.agent_id = next;
            self.greeting_consumed = false;
        }
    }

    /// Resolve `voice_id` / `speed` / `pitch` / provider for the active agent.
    pub fn resolve(&self) -> ResolvedTtsVoice {
        self.voice.resolve_tts_for_agent(&self.agent_id)
    }

    /// Resolve for an explicit agent id without changing session state.
    pub fn resolve_for(&self, agent_id: &str) -> ResolvedTtsVoice {
        self.voice.resolve_tts_for_agent(agent_id)
    }

    /// Take the session-start greeting for the active agent, if any and not
    /// yet consumed. Subsequent calls return `None` until [`set_agent`]
    /// switches to a different agent (which re-arms greeting for that agent).
    pub fn take_session_greeting(&mut self) -> Option<String> {
        if self.greeting_consumed {
            return None;
        }
        let greeting = self.voice.session_greeting_for(&self.agent_id);
        if greeting.is_some() {
            self.greeting_consumed = true;
        }
        greeting
    }

    /// Whether the greeting has already been consumed for the current agent.
    pub fn greeting_consumed(&self) -> bool {
        self.greeting_consumed
    }

    /// Build speak text for a turn. When `session_start` is true, consumes
    /// and prefixes `greeting_prefix` once.
    ///
    /// Returns `(resolved_voice, spoken_text)`.
    pub fn prepare_utterance(
        &mut self,
        body: &str,
        session_start: bool,
    ) -> (ResolvedTtsVoice, String) {
        let resolved = self.resolve();
        let text = if session_start {
            match self.take_session_greeting() {
                Some(g) => {
                    let body = body.trim();
                    if body.is_empty() {
                        g
                    } else {
                        format!("{g} {body}")
                    }
                }
                None => body.to_string(),
            }
        } else {
            body.to_string()
        };
        (resolved, text)
    }

    /// JSON body for a substrate/cloud TTS POST: `text` plus personality
    /// voice fields (`voice_id`, `speed`, `pitch`, …).
    pub fn synthesize_request_json(&self, text: &str) -> serde_json::Value {
        let resolved = self.resolve();
        let mut map = resolved.tts_request_fields();
        map.insert("text".into(), serde_json::Value::String(text.to_string()));
        serde_json::Value::Object(map)
    }

    /// Underlying voice config (read-only).
    pub fn voice_config(&self) -> &VoiceConfig {
        &self.voice
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::config::VoicePersonality;

    fn two_agent_config() -> VoiceConfig {
        let mut cfg = VoiceConfig::default();
        cfg.personalities.insert(
            "alpha".into(),
            VoicePersonality {
                voice_id: "nova".into(),
                provider: "openai".into(),
                speed: 1.2,
                pitch: 0.1,
                greeting_prefix: Some("Alpha here.".into()),
                language: "en".into(),
            },
        );
        cfg.personalities.insert(
            "beta".into(),
            VoicePersonality {
                voice_id: "shimmer".into(),
                provider: "openai".into(),
                speed: 0.85,
                pitch: -0.4,
                greeting_prefix: Some("Beta speaking.".into()),
                language: "en".into(),
            },
        );
        cfg
    }

    #[test]
    fn two_personalities_resolve_distinct_voice_params() {
        let dispatch = PersonalityTtsDispatch::new(two_agent_config(), "alpha");
        let a = dispatch.resolve_for("alpha");
        let b = dispatch.resolve_for("beta");
        assert_eq!(a.voice_id, "nova");
        assert_eq!(b.voice_id, "shimmer");
        assert!((a.speed - 1.2).abs() < f32::EPSILON);
        assert!((b.speed - 0.85).abs() < f32::EPSILON);
        assert!((a.pitch - 0.1).abs() < f32::EPSILON);
        assert!((b.pitch - (-0.4)).abs() < f32::EPSILON);
        assert_ne!(a.voice_id, b.voice_id);
    }

    #[test]
    fn greeting_consumed_once_at_session_start() {
        let mut dispatch = PersonalityTtsDispatch::new(two_agent_config(), "alpha");
        let g1 = dispatch.take_session_greeting();
        assert_eq!(g1.as_deref(), Some("Alpha here."));
        assert!(dispatch.greeting_consumed());
        // Second take is a no-op (already spoken).
        assert!(dispatch.take_session_greeting().is_none());

        let (voice, text) = {
            // prepare_utterance with session_start after greeting already taken
            // must not re-prefix.
            let mut d = PersonalityTtsDispatch::new(two_agent_config(), "alpha");
            let (v1, t1) = d.prepare_utterance("Ready to help.", true);
            assert_eq!(v1.voice_id, "nova");
            assert_eq!(t1, "Alpha here. Ready to help.");
            let (v2, t2) = d.prepare_utterance("Next turn.", true);
            // greeting already consumed even if session_start is true again
            assert_eq!(v2.voice_id, "nova");
            assert_eq!(t2, "Next turn.");
            (v1, t1)
        };
        assert_eq!(voice.voice_id, "nova");
        assert!(text.starts_with("Alpha here."));
    }

    #[test]
    fn set_agent_rearms_greeting_and_switches_voice() {
        let mut dispatch = PersonalityTtsDispatch::new(two_agent_config(), "alpha");
        assert_eq!(
            dispatch.take_session_greeting().as_deref(),
            Some("Alpha here.")
        );

        dispatch.set_agent("beta");
        let r = dispatch.resolve();
        assert_eq!(r.voice_id, "shimmer");
        assert!((r.pitch - (-0.4)).abs() < f32::EPSILON);
        assert_eq!(
            dispatch.take_session_greeting().as_deref(),
            Some("Beta speaking.")
        );
        assert!(dispatch.take_session_greeting().is_none());
    }

    #[test]
    fn synthesize_request_json_includes_voice_fields() {
        let dispatch = PersonalityTtsDispatch::new(two_agent_config(), "beta");
        let body = dispatch.synthesize_request_json("hello world");
        assert_eq!(body["text"], "hello world");
        assert_eq!(body["voice_id"], "shimmer");
        assert!((body["speed"].as_f64().unwrap() - 0.85).abs() < 1e-5);
        assert!((body["pitch"].as_f64().unwrap() - (-0.4)).abs() < 1e-5);
        assert_eq!(body["provider"], "openai");
    }

    #[test]
    fn unknown_agent_falls_back_without_greeting() {
        let dispatch = PersonalityTtsDispatch::new(two_agent_config(), "gamma");
        let r = dispatch.resolve();
        assert!(!r.matched_agent);
        assert!(dispatch.voice_config().session_greeting_for("gamma").is_none());
    }
}
