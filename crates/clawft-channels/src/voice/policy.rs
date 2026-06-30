//! Spoken-answer policy (ADR-061 §3).
//!
//! The one **voice-specific** constraint on the agent loop: a voice turn is
//! *short* — a hard spoken-length token cap, 1–2 spoken sentences, no
//! markdown / lists / code. A voice agent talks; it does not read a document
//! aloud. This module shapes the request sent to the LLM and **enforces** the
//! constraint on the reply so an over-eager model can't produce a wall of
//! text the TTS layer would then have to speak.
//!
//! **Out of scope (ADR-061 §3, ADR-060):** tool-calling and web grounding are
//! the agent loop's concern, not the voice front's — this module never adds
//! tool plumbing.
//!
//! Provider-agnostic by design (like the STT/TTS/turn/speaker seams): the
//! [`VoiceLlm`] trait is implemented by the Talk-Mode controller (task 6.7)
//! over `clawft_llm::LocalProvider` — concretely
//! `LocalProvider::hermes_serving().with_num_ctx(n)` then
//! `complete_turn(req, ReasoningMode::Off)` (reasoning off for concise voice
//! answers; planning mode is for tool execution, which is out of scope here).
//! Keeping the dependency inverted means this module needs no clawft-llm dep
//! and stays unit-testable without a live endpoint.

use async_trait::async_trait;

use super::tts::split_sentences;
use super::types::VoiceError;

/// Per-turn reasoning hint. Mirrors `clawft_llm::ReasoningMode` without taking
/// the dependency; the controller maps it across at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReasoningHint {
    /// Reasoning off — concise spoken answer (the voice default).
    #[default]
    Off,
    /// Reasoning on — planning (not used for spoken answers; here for parity).
    Planning,
}

/// Voice-turn spoken-answer policy.
#[derive(Debug, Clone)]
pub struct VoiceAnswerPolicy {
    /// Hard cap on spoken-answer length in (estimated) tokens.
    pub max_tokens: u32,
    /// Maximum spoken sentences (ADR-061: 1–2).
    pub max_sentences: usize,
    /// Per-turn reasoning hint (Off for spoken answers).
    pub reasoning: ReasoningHint,
}

impl Default for VoiceAnswerPolicy {
    fn default() -> Self {
        Self {
            max_tokens: 64,
            max_sentences: 2,
            reasoning: ReasoningHint::Off,
        }
    }
}

/// A provider-agnostic, voice-shaped LLM request. The controller maps this
/// onto the concrete `LocalProvider` request type.
#[derive(Debug, Clone)]
pub struct VoiceTurnRequest {
    /// System prompt (base agent persona + the spoken-answer policy).
    pub system: String,
    /// The user's transcribed utterance.
    pub user: String,
    /// Private speaker/context fed to the LLM but **never spoken** (from the
    /// speaker registry, task 6.6).
    pub speaker_context: Option<String>,
    /// Hard max output tokens.
    pub max_tokens: u32,
    /// Per-turn reasoning hint.
    pub reasoning: ReasoningHint,
}

/// The LLM seam. Implemented by the Talk-Mode controller over the local
/// Hermes provider; mocked in tests.
#[async_trait]
pub trait VoiceLlm: Send + Sync {
    /// Complete one voice turn, returning the raw model text (pre-policy).
    async fn complete(&self, req: &VoiceTurnRequest) -> Result<String, VoiceError>;
}

impl VoiceAnswerPolicy {
    /// The canonical voice system-policy text appended to the agent persona.
    pub fn system_policy(&self) -> String {
        format!(
            "You are speaking out loud through a text-to-speech voice. Reply in \
             at most {} short spoken sentence(s). Be direct and conversational. \
             Do NOT use markdown, bullet lists, numbered lists, headings, tables, \
             code blocks, or URLs — none of that can be spoken. Keep the answer \
             under roughly {} tokens. If the answer is long, give the single most \
             useful sentence and offer to continue.",
            self.max_sentences, self.max_tokens
        )
    }

    /// Build a voice-shaped request from a transcript and optional base system
    /// persona + private speaker context.
    pub fn shape_request(
        &self,
        base_system: &str,
        transcript: &str,
        speaker_context: Option<String>,
    ) -> VoiceTurnRequest {
        let system = if base_system.trim().is_empty() {
            self.system_policy()
        } else {
            format!("{}\n\n{}", base_system.trim(), self.system_policy())
        };
        VoiceTurnRequest {
            system,
            user: transcript.to_string(),
            speaker_context,
            max_tokens: self.max_tokens,
            reasoning: self.reasoning,
        }
    }

    /// Enforce the spoken-answer constraint on a raw model reply: strip markup,
    /// clamp to `max_sentences`, and hard-cap to `max_tokens`. Idempotent and
    /// total — always returns something speakable (possibly empty).
    pub fn enforce(&self, raw: &str) -> String {
        let plain = strip_markup(raw);
        // Clamp sentence count first.
        let sentences = split_sentences(&plain);
        let kept: Vec<String> = sentences
            .into_iter()
            .take(self.max_sentences.max(1))
            .collect();
        let mut answer = kept.join(" ");
        // Hard token cap: if still over budget, trim to whole words within cap.
        if estimate_tokens(&answer) > self.max_tokens {
            answer = truncate_to_tokens(&answer, self.max_tokens);
        }
        answer.trim().to_string()
    }

    /// Shape → complete → enforce: the full spoken-answer path.
    pub async fn answer(
        &self,
        llm: &dyn VoiceLlm,
        base_system: &str,
        transcript: &str,
        speaker_context: Option<String>,
    ) -> Result<String, VoiceError> {
        let req = self.shape_request(base_system, transcript, speaker_context);
        let raw = llm.complete(&req).await?;
        Ok(self.enforce(&raw))
    }
}

/// Estimate token count (~4 chars/token, the common heuristic).
pub fn estimate_tokens(text: &str) -> u32 {
    text.chars().count().div_ceil(4) as u32
}

/// Truncate to at most `max_tokens` estimated tokens on a word boundary.
fn truncate_to_tokens(text: &str, max_tokens: u32) -> String {
    let max_chars = (max_tokens as usize) * 4;
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = String::new();
    for word in text.split_whitespace() {
        let candidate = if out.is_empty() {
            word.to_string()
        } else {
            format!("{out} {word}")
        };
        if candidate.chars().count() > max_chars {
            break;
        }
        out = candidate;
    }
    out
}

/// Strip markdown / formatting that cannot be spoken. Conservative: removes
/// emphasis/code markers, heading/list/quote prefixes, and link syntax while
/// keeping the readable text.
fn strip_markup(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for raw_line in text.lines() {
        let mut line = raw_line.trim().to_string();
        // Drop fenced-code markers entirely.
        if line.starts_with("```") {
            continue;
        }
        // Heading / quote prefixes.
        line = line
            .trim_start_matches('#')
            .trim_start_matches('>')
            .trim_start()
            .to_string();
        // List bullets: -, *, + then space.
        if let Some(rest) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .or_else(|| line.strip_prefix("+ "))
        {
            line = rest.to_string();
        }
        // Numbered list "1. " / "12) ".
        line = strip_ordered_prefix(&line);
        if !line.is_empty() {
            lines.push(line);
        }
    }
    let joined = lines.join(" ");
    // Inline markers + link syntax, char by char.
    let mut out = String::with_capacity(joined.len());
    let mut chars = joined.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '`' | '#' => {}
            '[' => {
                // [text](url) -> text : keep until ']', drop the (url).
                for t in chars.by_ref() {
                    if t == ']' {
                        break;
                    }
                    out.push(t);
                }
                if chars.peek() == Some(&'(') {
                    for u in chars.by_ref() {
                        if u == ')' {
                            break;
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strip a leading ordered-list marker like `1. ` or `2) `.
fn strip_ordered_prefix(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
        let after = line[i + 1..].trim_start();
        return after.to_string();
    }
    line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_policy_mentions_constraints() {
        let p = VoiceAnswerPolicy::default();
        let s = p.system_policy();
        assert!(s.contains("2 short spoken"));
        assert!(s.to_lowercase().contains("markdown"));
        assert!(s.contains("64"));
    }

    #[test]
    fn shape_request_combines_system_and_carries_context() {
        let p = VoiceAnswerPolicy::default();
        let req = p.shape_request("You are Dave.", "what's up", Some("speaker=Alice".into()));
        assert!(req.system.starts_with("You are Dave."));
        assert!(req.system.contains("spoken"));
        assert_eq!(req.user, "what's up");
        assert_eq!(req.speaker_context.as_deref(), Some("speaker=Alice"));
        assert_eq!(req.max_tokens, 64);
        assert_eq!(req.reasoning, ReasoningHint::Off);
    }

    #[test]
    fn enforce_clamps_to_two_sentences() {
        let p = VoiceAnswerPolicy::default();
        let raw = "First sentence. Second sentence. Third sentence. Fourth.";
        let out = p.enforce(raw);
        assert_eq!(out, "First sentence. Second sentence.");
    }

    #[test]
    fn enforce_strips_markdown_and_lists() {
        let p = VoiceAnswerPolicy {
            max_sentences: 5,
            ..Default::default()
        };
        let raw =
            "# Heading\n- **bold** item\n- another `code` item\n1. numbered\nSee [docs](http://x).";
        let out = p.enforce(raw);
        assert!(!out.contains('#'));
        assert!(!out.contains('*'));
        assert!(!out.contains('`'));
        assert!(!out.contains('['));
        assert!(!out.contains("http"));
        assert!(out.contains("bold item"));
        assert!(out.contains("docs"));
    }

    #[test]
    fn enforce_hard_token_cap() {
        let p = VoiceAnswerPolicy {
            max_tokens: 5,
            max_sentences: 5,
            ..Default::default()
        };
        // One long run-on "sentence" (no terminators) well over 5 tokens.
        let raw = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda";
        let out = p.enforce(raw);
        assert!(
            estimate_tokens(&out) <= 5,
            "got {} tokens: {out:?}",
            estimate_tokens(&out)
        );
        assert!(!out.is_empty());
    }

    #[test]
    fn estimate_tokens_rough() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    struct WallOfTextLlm;
    #[async_trait]
    impl VoiceLlm for WallOfTextLlm {
        async fn complete(&self, _req: &VoiceTurnRequest) -> Result<String, VoiceError> {
            Ok("## Title\n\nHere is the answer in great detail. \
                It goes on. And on. And on with lists:\n- one\n- two\n- three."
                .into())
        }
    }

    #[tokio::test]
    async fn answer_enforces_over_eager_model() {
        let p = VoiceAnswerPolicy::default();
        let out = p
            .answer(&WallOfTextLlm, "You are Dave.", "explain it", None)
            .await
            .unwrap();
        assert!(!out.contains('#'));
        // At most max_sentences sentences.
        assert!(split_sentences(&out).len() <= p.max_sentences);
        assert!(!out.is_empty());
    }

    /// Live spoken-answer length gate against a real Hermes endpoint. Needs the
    /// LocalProvider + a served model (0.1 BLOCKED on model download); ignored.
    #[tokio::test]
    #[ignore = "requires live local Hermes endpoint (ADR-060 0.1 blocked on model download)"]
    async fn live_answer_is_concise() {
        // Wired by the Talk-Mode controller (6.7) over clawft_llm::LocalProvider.
    }
}
