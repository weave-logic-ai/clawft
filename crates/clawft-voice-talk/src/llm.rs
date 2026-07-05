//! [`VoiceLlm`] binding over the local Hermes provider (ADR-060 / ADR-061 §3).

use async_trait::async_trait;

use clawft_channels::voice::policy::{ReasoningHint, VoiceLlm, VoiceTurnRequest};
use clawft_channels::voice::types::VoiceError;
use clawft_llm::{ChatMessage, ChatRequest, LocalProvider, ProviderError, ReasoningMode};

/// [`VoiceLlm`] implemented over `clawft_llm::LocalProvider`.
///
/// Builds an OpenAI-shaped [`ChatRequest`] from the voice-shaped
/// [`VoiceTurnRequest`] (system policy + private speaker context + user
/// transcript, hard `max_tokens`) and runs it through
/// [`LocalProvider::complete_turn`] with the per-turn reasoning mode (off for
/// concise spoken answers). The spoken-answer policy then enforces brevity on
/// the returned text.
pub struct LocalProviderVoiceLlm {
    provider: LocalProvider,
    model: String,
    temperature: f64,
}

impl LocalProviderVoiceLlm {
    /// Bind to the ADR-060 local Hermes serving recipe (port 8090, 32k ctx).
    pub fn hermes() -> Self {
        Self::new(LocalProvider::hermes_serving())
    }

    /// Bind to an explicit provider. The model name is taken from the
    /// provider's default model.
    pub fn new(provider: LocalProvider) -> Self {
        let model = provider
            .config()
            .default_model
            .clone()
            .unwrap_or_else(|| "local".to_string());
        Self {
            provider,
            model,
            temperature: 0.3,
        }
    }

    /// Override the sampling temperature (default 0.3 — steady for short
    /// spoken answers).
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = t;
        self
    }

    /// Build the OpenAI-shaped chat request from a voice turn. Pure (no I/O) so
    /// it is unit-testable without a live endpoint.
    fn build_request(&self, req: &VoiceTurnRequest) -> ChatRequest {
        let mut messages = Vec::with_capacity(3);
        messages.push(ChatMessage::system(req.system.clone()));
        if let Some(ctx) = &req.speaker_context {
            // Private context the model sees but must never speak.
            messages.push(ChatMessage::system(ctx.clone()));
        }
        messages.push(ChatMessage::user(req.user.clone()));

        ChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: Some(req.max_tokens as i32),
            temperature: Some(self.temperature),
            tools: Vec::new(), // tool-calling is OUT of voice scope (ADR-061 §3)
            stream: None,
            tool_choice: None, // voice never forces tools (ADR-061 §3)
        }
    }
}

fn map_reasoning(hint: ReasoningHint) -> ReasoningMode {
    match hint {
        ReasoningHint::Off => ReasoningMode::Off,
        ReasoningHint::Planning => ReasoningMode::On,
    }
}

fn map_err(e: ProviderError) -> VoiceError {
    match e {
        ProviderError::ServerError { status, body } => VoiceError::Server { status, body },
        ProviderError::ModelNotFound(m) => VoiceError::Server {
            status: 404,
            body: m,
        },
        ProviderError::InvalidResponse(m) => VoiceError::Malformed(m),
        other => VoiceError::Transport(other.to_string()),
    }
}

#[async_trait]
impl VoiceLlm for LocalProviderVoiceLlm {
    async fn complete(&self, req: &VoiceTurnRequest) -> Result<String, VoiceError> {
        let chat = self.build_request(req);
        let resp = self
            .provider
            .complete_turn(&chat, map_reasoning(req.reasoning))
            .await
            .map_err(map_err)?;
        let text = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| VoiceError::Malformed("LLM response had no content".into()))?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_channels::voice::policy::VoiceAnswerPolicy;

    #[test]
    fn build_request_shapes_messages_and_caps_tokens() {
        let llm = LocalProviderVoiceLlm::hermes();
        let policy = VoiceAnswerPolicy::default();
        let req = policy.shape_request(
            "You are Dave.",
            "what's the weather",
            Some("The current speaker is Alice (id spk-0).".into()),
        );
        let chat = llm.build_request(&req);

        // system policy + speaker context + user => 3 messages.
        assert_eq!(chat.messages.len(), 3);
        assert_eq!(chat.messages[0].role, "system");
        assert!(
            chat.messages[0]
                .content
                .as_deref()
                .unwrap()
                .contains("spoken")
        );
        assert_eq!(chat.messages[1].role, "system");
        assert!(
            chat.messages[1]
                .content
                .as_deref()
                .unwrap()
                .contains("Alice")
        );
        assert_eq!(chat.messages[2].role, "user");
        assert_eq!(
            chat.messages[2].content.as_deref(),
            Some("what's the weather")
        );
        assert_eq!(chat.max_tokens, Some(policy.max_tokens as i32));
        assert!(chat.tools.is_empty(), "tool-calling is out of voice scope");
        assert!(!chat.model.is_empty());
    }

    #[test]
    fn build_request_without_speaker_context_has_two_messages() {
        let llm = LocalProviderVoiceLlm::hermes();
        let req = VoiceAnswerPolicy::default().shape_request("", "hi", None);
        let chat = llm.build_request(&req);
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].role, "system");
        assert_eq!(chat.messages[1].role, "user");
    }

    #[test]
    fn reasoning_maps_off_for_voice() {
        assert_eq!(map_reasoning(ReasoningHint::Off), ReasoningMode::Off);
        assert_eq!(map_reasoning(ReasoningHint::Planning), ReasoningMode::On);
    }

    /// Live Hermes round-trip. Requires the ADR-060 serving recipe up at
    /// http://127.0.0.1:8090/v1; ignored by default.
    #[tokio::test]
    #[ignore = "requires live local Hermes endpoint (ADR-060)"]
    async fn live_hermes_completes_a_voice_turn() {
        let llm = LocalProviderVoiceLlm::hermes();
        let req = VoiceAnswerPolicy::default().shape_request(
            "You are a terse voice assistant.",
            "say hello in one short sentence",
            None,
        );
        let out = llm.complete(&req).await.unwrap();
        assert!(!out.trim().is_empty());
    }
}
