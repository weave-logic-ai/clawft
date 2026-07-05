//! Enrichment classifier — the async LLM (Phase B) tier of the ADR-067 P2
//! classifier (design `.planning/hermes-loop/classification-design.md` §D4,
//! plan B2; taxonomy v2 §D2.1).
//!
//! After a turn commits with a keyword blob (Phase A), a cheap-model round-trip
//! refines the dialogue act, topic, emotion, and goal off the turn path and adds
//! the one thing the keyword tier honestly cannot: the command's **argument
//! structure** (verb + object). The node blob is then patched with `tier: "llm"`
//! (design §D1, §D4). This module owns that round-trip: it reuses the routing
//! crate's [`Classifier`] backend trait for the LLM call but emits a **separate**
//! v2 prompt and parses into the A1 [`ClassificationVector`] — it never touches
//! `ClassifierOutput` / `LlmClassifierRouter`, the routing contract (design §4).
//!
//! **Entity spans stay keyword-owned** (design §D2.1 — keyword owns the regex
//! spans, the LLM owns the act + argument). Rather than have the LLM re-list
//! spans (unreliable), the parser re-derives the structure deterministically from
//! the turn text via [`extract_structure`] and layers only the LLM `argument` on
//! top, so a patch never drops the keyword-extracted paths / urls / numbers.
//!
//! # Robustness
//!
//! Mirrors the router's hardening: fences are stripped; total parse failure /
//! backend error / empty body collapse to `None` (the caller keeps the durable
//! keyword blob); a *partial* object fills what parsed (act ⇒ nearest-or-comment,
//! topic ⇒ prior/`general`, emotion ⇒ neutral, goal/argument ⇒ `None`).

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use clawft_core::agent::context_router::Classifier;
use clawft_types::config::ClassificationConfig;

use crate::dialogue_act::{Act, RefinedAct};
use crate::text_structure::{extract_structure, Argument};
use crate::turn_classifier::{ClassificationVector, Tier, Vad, DEFAULT_TOPIC, TAXONOMY_VERSION};

/// System prompt for the v2 enrichment turn. Emits the refined dialogue act, a
/// refined topic, emotion, goal, and the argument structure — matching the §D2.1
/// taxonomy so the parsed result drops into the blob (class is re-derived from
/// the act; entity spans come from the keyword tier).
pub const ENRICHMENT_SYSTEM_PROMPT: &str = "\
You label one conversation turn for a graph view. Reply with JSON only:\n\
{ \"act\": \"question\"|\"clarification-request\"|\"clarification-provide\"|\"command\"\n\
        |\"comment\"|\"correction\"|\"feedback\"|\"acknowledgment\"|\"social\"|\"meta\",\n\
  \"topic\": \"<1-3 lowercase words>\",\n\
  \"emotion\": { \"valence\": <-1..1>, \"arousal\": <-1..1>, \"dominance\": <-1..1>, \"label\": \"<one word>\" },\n\
  \"goal\": \"<short active-goal phrase>\" | null,\n\
  \"argument\": { \"verb\": \"<imperative verb>\", \"object\": \"<what it acts on>\" } | null }\n\
\n\
act — the dialogue move:\n\
  question: asks for info      clarification-request: asks the other to clarify\n\
  clarification-provide: clarifies your own prior turn    command: asks for an action\n\
  comment: asserts a fact      correction: fixes a prior turn\n\
  feedback: evaluates the assistant   acknowledgment: ok / got it / makes sense\n\
  social: greeting / thanks    meta: talks about the conversation itself\n\
argument — for a command only: the verb + the object it acts on; null otherwise.\n\
emotion — valence (unpleasant..pleasant), arousal (calm..intense), dominance\n\
  (submissive..assertive), each in [-1,1]; label is one plain word.\n\
\n\
Examples:\n\
  \"what do you mean by that?\" -> {\"act\":\"clarification-request\",\"topic\":\"meaning\",\"emotion\":{\"valence\":0.0,\"arousal\":0.4,\"dominance\":0.0,\"label\":\"puzzled\"},\"goal\":null,\"argument\":null}\n\
  \"fix the SNAC decode\" -> {\"act\":\"command\",\"topic\":\"snac decode\",\"emotion\":{\"valence\":-0.1,\"arousal\":0.5,\"dominance\":0.3,\"label\":\"focused\"},\"goal\":\"fix the decoder\",\"argument\":{\"verb\":\"fix\",\"object\":\"the SNAC decode\"}}\n\
";

/// Upper bound on the enrichment turn's `max_tokens`. Larger than the router's
/// 64-token cap because the v2 act + argument blob is bigger.
pub const DEFAULT_ENRICHMENT_MAX_TOKENS: u32 = 200;

/// The Phase-B async enrichment classifier (design §D4, §D2.1). Holds a
/// [`Classifier`] backend and the cheap-model override; produces a refined
/// [`ClassificationVector`] with `tier: Llm`.
pub struct EnrichmentClassifier {
    backend: Arc<dyn Classifier>,
    /// Cheap-model override (from `ClassificationConfig.model_override`); `None`
    /// uses whatever model the backend's `LlmClient` is pinned to.
    model_override: Option<String>,
    /// Hard cap on the enrichment turn's `max_tokens`.
    max_tokens: u32,
}

impl EnrichmentClassifier {
    /// Wrap a [`Classifier`] backend (typically `Arc<LlmClient>` via the
    /// native-only blanket impl in the routing crate).
    pub fn from_backend(backend: Arc<dyn Classifier>) -> Self {
        Self {
            backend,
            model_override: None,
            max_tokens: DEFAULT_ENRICHMENT_MAX_TOKENS,
        }
    }

    /// Build from a backend + the classification config, consuming
    /// `model_override` (design §D6 — the cheap-model pin for the `full` tier).
    pub fn from_config(backend: Arc<dyn Classifier>, cfg: &ClassificationConfig) -> Self {
        Self::from_backend(backend).with_model_override(cfg.model_override.clone())
    }

    /// Pin an enrichment model name (overrides the backend's default).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_override = Some(model.into());
        self
    }

    /// Set the model override from an `Option` (config-driven path).
    #[must_use]
    pub fn with_model_override(mut self, model: Option<String>) -> Self {
        self.model_override = model;
        self
    }

    /// Override the enrichment `max_tokens` cap (default
    /// [`DEFAULT_ENRICHMENT_MAX_TOKENS`]).
    #[must_use]
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Enrich one turn. `prev_topic` is the node's existing (keyword) topic, used
    /// as the topic fallback when the model omits one; the entity structure is
    /// re-derived from `text` so the keyword spans survive the patch (design
    /// §D2.1).
    ///
    /// Returns a refined [`ClassificationVector`] (`tier: Llm`) on success, or
    /// `None` on any backend failure, empty body, or total parse failure — the
    /// caller then keeps the durable keyword blob (design §D4).
    pub async fn enrich(
        &self,
        text: &str,
        prev_topic: Option<&str>,
    ) -> Option<ClassificationVector> {
        let raw = match self
            .backend
            .classify(text, self.max_tokens, self.model_override.as_deref())
            .await
        {
            Ok(body) if body.trim().is_empty() => {
                warn!("EnrichmentClassifier: empty response; keeping keyword blob");
                return None;
            }
            Ok(body) => body,
            Err(e) => {
                warn!(error = %e, "EnrichmentClassifier: backend call failed; keeping keyword blob");
                return None;
            }
        };
        match parse_enrichment_envelope(&raw, text, prev_topic) {
            Some(cv) => Some(cv),
            None => {
                warn!(body = %raw, "EnrichmentClassifier: malformed JSON; keeping keyword blob");
                None
            }
        }
    }
}

/// Parse a v2 enrichment envelope into a [`ClassificationVector`] (`tier: Llm`).
/// `text` re-derives the entity structure (keyword spans survive the patch);
/// `prev_topic` is the topic fallback. Returns `None` only when the body is not a
/// JSON object (caller keeps the keyword blob); a partial object fills what parsed
/// (design §D2.1). Pure — the hardening matrix unit-tests it without a backend.
pub fn parse_enrichment_envelope(
    body: &str,
    text: &str,
    prev_topic: Option<&str>,
) -> Option<ClassificationVector> {
    let parsed: Value = serde_json::from_str(strip_fences(body)).ok()?;
    let obj = parsed.as_object()?;

    let act = obj
        .get("act")
        .and_then(Value::as_str)
        .map(parse_refined_act)
        .or_else(|| {
            obj.get("intent")
                .and_then(Value::as_str)
                .map(RefinedAct::from_intent_str)
        })
        .map(Act::new)
        .unwrap_or_else(|| Act::new(RefinedAct::Comment));

    let topic = obj
        .get("topic")
        .and_then(Value::as_str)
        .map(normalize_topic)
        .filter(|t| !t.is_empty())
        .or_else(|| prev_topic.map(str::to_string))
        .unwrap_or_else(|| DEFAULT_TOPIC.to_string());

    let emotion = parse_emotion(obj.get("emotion"));

    let goal = obj
        .get("goal")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    // Keyword-owned entity spans + shape, re-derived from the text; the LLM only
    // contributes the argument (design §D2.1). No speaker vocab at this tier.
    let mut structure = extract_structure(text, &[]);
    structure.argument = parse_argument(obj.get("argument"));

    Some(ClassificationVector {
        act,
        topic,
        emotion,
        structure,
        goal,
        tier: Tier::Llm,
        v: TAXONOMY_VERSION,
    })
}

/// Parse a refined-act string: exact kebab-case match, else nearest known act by
/// substring, else the neutral `Comment` default (design §D2.1 nearest-or-none).
fn parse_refined_act(s: &str) -> RefinedAct {
    if let Some(a) = RefinedAct::from_wire(s) {
        return a;
    }
    let lower = s.trim().to_lowercase();
    for name in [
        "clarification-request",
        "clarification-provide",
        "question",
        "command",
        "comment",
        "correction",
        "feedback",
        "acknowledgment",
        "social",
        "meta",
    ] {
        if (lower.contains(name) || name.contains(lower.as_str()))
            && let Some(a) = RefinedAct::from_wire(name)
        {
            return a;
        }
    }
    RefinedAct::Comment
}

/// Parse the `argument` object into an [`Argument`]; `verb` is required.
fn parse_argument(v: Option<&Value>) -> Option<Argument> {
    let obj = v?.as_object()?;
    let verb = obj
        .get("verb")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let object = obj
        .get("object")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some(Argument { verb, object })
}

/// Parse the emotion object into a [`Vad`], clamping each scalar to `[-1, 1]` and
/// defaulting missing/non-finite axes (arousal `0.5`, others `0.0`).
fn parse_emotion(v: Option<&Value>) -> Vad {
    let Some(obj) = v else {
        return Vad::neutral();
    };
    let label = obj
        .get("label")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .unwrap_or_else(|| "neutral".to_string());
    Vad {
        valence: clamp_axis(obj.get("valence"), 0.0),
        arousal: clamp_axis(obj.get("arousal"), 0.5),
        dominance: clamp_axis(obj.get("dominance"), 0.0),
        label,
    }
}

/// Read one emotion scalar, coercing absent/non-finite to `default` and clamping
/// to `[-1, 1]`.
fn clamp_axis(v: Option<&Value>, default: f32) -> f32 {
    v.and_then(Value::as_f64)
        .map(|f| f as f32)
        .filter(|f| f.is_finite())
        .unwrap_or(default)
        .clamp(-1.0, 1.0)
}

/// Lowercase + collapse a topic to at most 3 words (design §D2 "≤3 words").
fn normalize_topic(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip an optional ```` ```json ```` / ```` ``` ```` fence, mirroring the
/// router's `parse_classifier_envelope` (`llm_classifier.rs`).
fn strip_fences(body: &str) -> &str {
    let trimmed = body.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let stripped = stripped.trim_start_matches('\n').trim();
    stripped.strip_suffix("```").unwrap_or(stripped).trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialogue_act::SpeechActClass;
    use crate::text_structure::EntityKind;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Mock backend: returns a canned body and records the `model_override` it
    /// was handed (so the config-consumption path can be asserted).
    struct MockClassifier {
        response: Result<String, String>,
        seen_model: Mutex<Option<String>>,
    }

    impl MockClassifier {
        fn ok(body: &str) -> Self {
            Self {
                response: Ok(body.to_string()),
                seen_model: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl Classifier for MockClassifier {
        async fn classify(
            &self,
            _user_content: &str,
            _max_tokens: u32,
            model_override: Option<&str>,
        ) -> Result<String, String> {
            *self.seen_model.lock().unwrap() = model_override.map(str::to_string);
            self.response.clone()
        }
    }

    const VALID: &str = r#"{"act":"command","topic":"snac decode","emotion":{"valence":-0.1,"arousal":0.6,"dominance":0.3,"label":"focused"},"goal":"fix the decoder","argument":{"verb":"fix","object":"the SNAC decode"}}"#;

    // The turn text drives the re-derived structure layer.
    const TEXT: &str = "fix src/snac.rs decode";

    // -- Pure parse hardening matrix ---------------------------------------

    #[test]
    fn parses_valid_v2_envelope() {
        let cv = parse_enrichment_envelope(VALID, TEXT, None).expect("valid parses");
        assert_eq!(cv.act.refined, RefinedAct::Command);
        assert_eq!(cv.act.class, SpeechActClass::Directive);
        assert_eq!(cv.topic, "snac decode");
        assert!((cv.emotion.arousal - 0.6).abs() < 1e-6);
        assert_eq!(cv.goal.as_deref(), Some("fix the decoder"));
        let arg = cv.structure.argument.as_ref().expect("argument parsed");
        assert_eq!(arg.verb, "fix");
        assert_eq!(arg.object.as_deref(), Some("the SNAC decode"));
        assert_eq!(cv.tier, Tier::Llm);
        assert_eq!(cv.v, TAXONOMY_VERSION);
    }

    #[test]
    fn structure_derived_from_text_with_llm_argument() {
        // Path entity comes from the text (keyword-owned); argument from the LLM.
        let cv = parse_enrichment_envelope(VALID, TEXT, None).unwrap();
        assert!(cv
            .structure
            .entities
            .iter()
            .any(|e| e.kind == EntityKind::Path && e.text == "src/snac.rs"));
        assert!(cv.structure.argument.is_some());
    }

    #[test]
    fn clarification_request_act() {
        let body = r#"{"act":"clarification-request","topic":"meaning"}"#;
        let cv = parse_enrichment_envelope(body, "what do you mean?", None).unwrap();
        assert_eq!(cv.act.refined, RefinedAct::ClarificationRequest);
        assert_eq!(cv.act.class, SpeechActClass::Interrogative);
    }

    #[test]
    fn strips_fences() {
        let fenced = format!("```json\n{VALID}\n```");
        assert!(parse_enrichment_envelope(&fenced, TEXT, None).is_some());
        let bare = format!("```\n{VALID}\n```");
        assert!(parse_enrichment_envelope(&bare, TEXT, None).is_some());
    }

    #[test]
    fn malformed_is_none() {
        assert!(parse_enrichment_envelope("not json", TEXT, None).is_none());
        assert!(parse_enrichment_envelope("42", TEXT, None).is_none());
    }

    #[test]
    fn partial_fills_defaults() {
        // Only act present: topic ⇒ prev/general, emotion ⇒ neutral, goal/arg ⇒ None.
        let cv = parse_enrichment_envelope(r#"{"act":"command"}"#, "do it", None).unwrap();
        assert_eq!(cv.act.refined, RefinedAct::Command);
        assert_eq!(cv.topic, DEFAULT_TOPIC);
        assert_eq!(cv.emotion, Vad::neutral());
        assert!(cv.goal.is_none());
        assert!(cv.structure.argument.is_none());
        // With a prev topic, the topic falls back to it.
        let cv = parse_enrichment_envelope(r#"{"act":"command"}"#, "do it", Some("decode")).unwrap();
        assert_eq!(cv.topic, "decode");
    }

    #[test]
    fn wrong_act_maps_to_nearest_or_comment() {
        // "commanding" → nearest "command".
        let cv = parse_enrichment_envelope(r#"{"act":"commanding"}"#, TEXT, None).unwrap();
        assert_eq!(cv.act.refined, RefinedAct::Command);
        // gibberish → Comment default.
        let cv = parse_enrichment_envelope(r#"{"act":"zzzzz"}"#, TEXT, None).unwrap();
        assert_eq!(cv.act.refined, RefinedAct::Comment);
    }

    #[test]
    fn legacy_intent_key_still_upgrades() {
        // An LLM that emits the old v1 `intent` key is still parsed.
        let cv = parse_enrichment_envelope(r#"{"intent":"request","topic":"x"}"#, TEXT, None).unwrap();
        assert_eq!(cv.act.refined, RefinedAct::Command);
    }

    #[test]
    fn emotion_out_of_range_clamped() {
        let body = r#"{"act":"comment","topic":"x","emotion":{"valence":9.0,"arousal":-5.0,"dominance":2.0,"label":"X"}}"#;
        let cv = parse_enrichment_envelope(body, TEXT, None).unwrap();
        assert_eq!(cv.emotion.valence, 1.0);
        assert_eq!(cv.emotion.arousal, -1.0);
        assert_eq!(cv.emotion.dominance, 1.0);
        assert_eq!(cv.emotion.label, "x");
    }

    #[test]
    fn argument_needs_verb() {
        // Object-only argument (no verb) is dropped.
        let cv =
            parse_enrichment_envelope(r#"{"act":"command","argument":{"object":"x"}}"#, TEXT, None)
                .unwrap();
        assert!(cv.structure.argument.is_none());
    }

    // -- Blob-shape golden (v2, tier llm) ----------------------------------

    #[test]
    fn enriched_blob_has_v2_keys_and_llm_tier() {
        let cv = parse_enrichment_envelope(VALID, TEXT, None).unwrap();
        let blob = cv.to_metadata_value();
        let obj = blob.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["act", "emotion", "goal", "intent", "structure", "tier", "topic", "v"]
        );
        assert_eq!(obj["tier"], serde_json::json!("llm"));
        assert_eq!(obj["v"], serde_json::json!(2));
        assert_eq!(obj["intent"], serde_json::json!("request")); // command → request
        assert_eq!(obj["structure"]["argument"]["verb"], serde_json::json!("fix"));
    }

    // -- Prompt snapshot ----------------------------------------------------

    #[test]
    fn prompt_names_acts_and_argument() {
        let p = ENRICHMENT_SYSTEM_PROMPT;
        assert!(p.contains("JSON only"));
        for act in [
            "question",
            "clarification-request",
            "clarification-provide",
            "command",
            "comment",
            "correction",
            "feedback",
            "acknowledgment",
            "social",
            "meta",
        ] {
            assert!(p.contains(act), "prompt must name act {act}");
        }
        assert!(p.contains("\"argument\""));
        assert!(p.contains("\"verb\""));
        assert!(p.contains("\"object\""));
    }

    // -- enrich() integration ----------------------------------------------

    #[tokio::test]
    async fn enrich_returns_vector_on_valid_body() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok(VALID)));
        let cv = ec.enrich("fix snac", None).await.expect("valid enriches");
        assert_eq!(cv.act.refined, RefinedAct::Command);
        assert_eq!(cv.tier, Tier::Llm);
    }

    #[tokio::test]
    async fn enrich_none_on_failures() {
        let err = EnrichmentClassifier::from_backend(Arc::new(MockClassifier {
            response: Err("transport".into()),
            seen_model: Mutex::new(None),
        }));
        assert!(err.enrich("x", None).await.is_none());
        let empty = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok("  ")));
        assert!(empty.enrich("x", None).await.is_none());
        let bad = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok("nonsense")));
        assert!(bad.enrich("x", None).await.is_none());
    }

    #[tokio::test]
    async fn from_config_consumes_model_override() {
        let cfg = ClassificationConfig {
            model_override: Some("haiku-3.5".into()),
            ..ClassificationConfig::default()
        };
        let backend = Arc::new(MockClassifier::ok(VALID));
        let ec = EnrichmentClassifier::from_config(backend.clone(), &cfg);
        ec.enrich("x", None).await;
        assert_eq!(
            backend.seen_model.lock().unwrap().as_deref(),
            Some("haiku-3.5")
        );
    }

    #[test]
    fn builder_defaults() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok("{}")));
        assert!(ec.model_override.is_none());
        assert_eq!(ec.max_tokens, DEFAULT_ENRICHMENT_MAX_TOKENS);
        let ec = ec.with_model("m").with_max_tokens(99);
        assert_eq!(ec.model_override.as_deref(), Some("m"));
        assert_eq!(ec.max_tokens, 99);
    }
}
