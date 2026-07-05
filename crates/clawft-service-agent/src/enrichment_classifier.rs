//! Enrichment classifier — the async LLM (Phase B) tier of the ADR-067 P2
//! classifier (design `.planning/hermes-loop/classification-design.md` §D4,
//! plan B2).
//!
//! After a turn commits with a keyword blob (Phase A), a cheap-model round-trip
//! refines the four axes off the turn path and the node blob is patched with
//! `tier: "llm"` (design §D1, §D4). This module owns that round-trip: it reuses
//! the routing crate's [`Classifier`] backend trait for the LLM call (so local
//! llama-server and hosted backends both work) but emits a **separate** four-axis
//! prompt and parses into the A1 [`ClassificationVector`] — it never touches
//! `ClassifierOutput` / `LlmClassifierRouter`, which the routing contract owns
//! (design §4 directive).
//!
//! # Robustness
//!
//! Mirrors the router's hardening (`llm_classifier.rs`): fences are stripped,
//! total parse failure / backend error / empty body collapse to `None` (so the
//! caller keeps the durable keyword blob), and a *partial* object fills only what
//! parsed — a missing axis takes its honest default (topic ⇒ prior/`general`,
//! emotion ⇒ neutral, goal ⇒ `None`), a garbled intent maps to the nearest known
//! variant or the neutral `Statement`. The enrichment never blocks or corrupts a
//! committed turn.

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use clawft_core::agent::context_router::Classifier;
use clawft_types::config::ClassificationConfig;

use crate::turn_classifier::{
    ClassificationVector, Intent, Tier, Vad, DEFAULT_TOPIC, TAXONOMY_VERSION,
};

/// System prompt for the four-axis enrichment turn. Emits the §D2 taxonomy
/// exactly (same intent enum + emotion axes as the keyword tier) so the parsed
/// result is a drop-in replacement blob — only `tier`/`v` are stamped by us.
///
/// Kept compact: every enrichment round-trips a cheap model, so length trades
/// against latency and (hosted) cost. Two few-shot examples anchor the shape.
pub const ENRICHMENT_SYSTEM_PROMPT: &str = "\
You label a single conversation turn for a graph view. Reply with JSON only:\n\
{ \"intent\": \"question\"|\"request\"|\"statement\"|\"correction\"|\"feedback\"|\"social\"|\"meta\",\n\
  \"topic\": \"<1-3 lowercase words>\",\n\
  \"emotion\": { \"valence\": <-1..1>, \"arousal\": <-1..1>, \"dominance\": <-1..1>, \"label\": \"<one word>\" },\n\
  \"goal\": \"<short active-goal phrase>\" | null }\n\
\n\
intent — the conversational act:\n\
  question: asks for information   request: asks for an action\n\
  statement: asserts a fact        correction: fixes a prior turn\n\
  feedback: evaluates the assistant or its output\n\
  social: greeting / thanks / farewell   meta: talks about the conversation itself\n\
topic — the subject in 1-3 lowercase words (e.g. \"voice tts\", \"kernel scheduler\").\n\
emotion — valence (unpleasant..pleasant), arousal (calm..intense), dominance\n\
  (submissive..assertive), each in [-1,1]; label is one plain word.\n\
goal — the active task the user is pursuing, short phrase, or null if unclear.\n\
\n\
Examples:\n\
  \"how does the SNAC decode work?\" -> {\"intent\":\"question\",\"topic\":\"snac decode\",\"emotion\":{\"valence\":0.1,\"arousal\":0.5,\"dominance\":0.0,\"label\":\"curious\"},\"goal\":null}\n\
  \"no, that broke the build!\" -> {\"intent\":\"correction\",\"topic\":\"build\",\"emotion\":{\"valence\":-0.6,\"arousal\":0.8,\"dominance\":0.2,\"label\":\"frustrated\"},\"goal\":\"fix the build\"}\n\
";

/// Upper bound on the enrichment turn's `max_tokens`. Larger than the router's
/// `{archetype, complexity}` cap (64) because the four-axis blob is bigger, but
/// still tight enough to bound a runaway generation.
pub const DEFAULT_ENRICHMENT_MAX_TOKENS: u32 = 160;

/// The Phase-B async enrichment classifier (design §D4). Holds a [`Classifier`]
/// backend and the cheap-model override; produces a refined
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
    /// as the fallback when the model omits a topic.
    ///
    /// Returns a refined [`ClassificationVector`] (`tier: Llm`) on success, or
    /// `None` on any backend failure, empty body, or total parse failure — in
    /// which case the caller keeps the durable keyword blob (design §D4,
    /// best-effort).
    pub async fn enrich(&self, text: &str, prev_topic: Option<&str>) -> Option<ClassificationVector> {
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
        match parse_enrichment_envelope(&raw, prev_topic) {
            Some(cv) => Some(cv),
            None => {
                warn!(body = %raw, "EnrichmentClassifier: malformed JSON; keeping keyword blob");
                None
            }
        }
    }
}

/// Parse a four-axis enrichment envelope into a [`ClassificationVector`]
/// (`tier: Llm`). Returns `None` only when the body is not a JSON object at all
/// (so the caller keeps the keyword blob); a *partial* object fills what parsed
/// and defaults the rest (design §D4 "fill-what-parsed"). Pure — no I/O — so the
/// hardening matrix can be unit-tested without a backend.
pub fn parse_enrichment_envelope(body: &str, prev_topic: Option<&str>) -> Option<ClassificationVector> {
    let parsed: Value = serde_json::from_str(strip_fences(body)).ok()?;
    let obj = parsed.as_object()?;

    let intent = obj
        .get("intent")
        .and_then(|v| v.as_str())
        .map(parse_intent)
        .unwrap_or(Intent::Statement);

    let topic = obj
        .get("topic")
        .and_then(|v| v.as_str())
        .map(normalize_topic)
        .filter(|t| !t.is_empty())
        .or_else(|| prev_topic.map(str::to_string))
        .unwrap_or_else(|| DEFAULT_TOPIC.to_string());

    let emotion = parse_emotion(obj.get("emotion"));

    let goal = obj
        .get("goal")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Some(ClassificationVector {
        intent,
        topic,
        emotion,
        goal,
        tier: Tier::Llm,
        v: TAXONOMY_VERSION,
    })
}

/// Known intent names for the nearest-match fallback (design §D4
/// "wrong-enum → nearest-or-none").
const KNOWN_INTENTS: &[(&str, Intent)] = &[
    ("question", Intent::Question),
    ("request", Intent::Request),
    ("statement", Intent::Statement),
    ("correction", Intent::Correction),
    ("feedback", Intent::Feedback),
    ("social", Intent::Social),
    ("meta", Intent::Meta),
];

/// Parse an intent string: exact lowercase match, else the nearest known variant
/// (prefix/substring), else the neutral `Statement` default. Never fails the
/// whole envelope on a garbled enum.
fn parse_intent(s: &str) -> Intent {
    let lower = s.trim().to_lowercase();
    for (name, intent) in KNOWN_INTENTS {
        if lower == *name {
            return *intent;
        }
    }
    for (name, intent) in KNOWN_INTENTS {
        if lower.starts_with(name) || name.starts_with(lower.as_str()) || lower.contains(name) {
            return *intent;
        }
    }
    Intent::Statement
}

/// Parse the emotion object into a [`Vad`], clamping each scalar to `[-1, 1]` and
/// defaulting missing/non-finite axes (arousal `0.5`, others `0.0`, design §D2).
fn parse_emotion(v: Option<&Value>) -> Vad {
    let Some(obj) = v else {
        return Vad::neutral();
    };
    let label = obj
        .get("label")
        .and_then(|v| v.as_str())
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

    const VALID: &str = r#"{"intent":"question","topic":"snac decode","emotion":{"valence":0.1,"arousal":0.6,"dominance":0.0,"label":"curious"},"goal":null}"#;

    // -- Pure parse hardening matrix ---------------------------------------

    #[test]
    fn parses_valid_envelope() {
        let cv = parse_enrichment_envelope(VALID, None).expect("valid parses");
        assert_eq!(cv.intent, Intent::Question);
        assert_eq!(cv.topic, "snac decode");
        assert!((cv.emotion.arousal - 0.6).abs() < 1e-6);
        assert_eq!(cv.emotion.label, "curious");
        assert!(cv.goal.is_none());
        assert_eq!(cv.tier, Tier::Llm);
        assert_eq!(cv.v, TAXONOMY_VERSION);
    }

    #[test]
    fn strips_json_fences() {
        let fenced = format!("```json\n{VALID}\n```");
        let cv = parse_enrichment_envelope(&fenced, None).expect("fenced parses");
        assert_eq!(cv.intent, Intent::Question);
    }

    #[test]
    fn strips_bare_fences() {
        let fenced = format!("```\n{VALID}\n```");
        assert!(parse_enrichment_envelope(&fenced, None).is_some());
    }

    #[test]
    fn malformed_json_is_none() {
        assert!(parse_enrichment_envelope("not json, sorry", None).is_none());
        // A JSON scalar is not an object → None (keep the keyword blob).
        assert!(parse_enrichment_envelope("42", None).is_none());
    }

    #[test]
    fn partial_fills_what_parsed() {
        // Only intent present: topic ⇒ prev/general, emotion ⇒ neutral, goal ⇒ None.
        let cv = parse_enrichment_envelope(r#"{"intent":"request"}"#, Some("voice"))
            .expect("partial still parses");
        assert_eq!(cv.intent, Intent::Request);
        assert_eq!(cv.topic, "voice"); // inherited prev_topic
        assert_eq!(cv.emotion, Vad::neutral());
        assert!(cv.goal.is_none());
    }

    #[test]
    fn partial_topic_missing_no_prev_defaults_general() {
        let cv = parse_enrichment_envelope(r#"{"intent":"statement"}"#, None).unwrap();
        assert_eq!(cv.topic, DEFAULT_TOPIC);
    }

    #[test]
    fn wrong_enum_maps_to_nearest_or_statement() {
        // "questions" → nearest "question".
        let cv = parse_enrichment_envelope(r#"{"intent":"questions","topic":"x"}"#, None).unwrap();
        assert_eq!(cv.intent, Intent::Question);
        // "inquiry" matches nothing → neutral Statement default.
        let cv = parse_enrichment_envelope(r#"{"intent":"inquiry","topic":"x"}"#, None).unwrap();
        assert_eq!(cv.intent, Intent::Statement);
    }

    #[test]
    fn emotion_out_of_range_is_clamped() {
        let body = r#"{"intent":"statement","topic":"x","emotion":{"valence":9.0,"arousal":-5.0,"dominance":2.0,"label":"X"}}"#;
        let cv = parse_enrichment_envelope(body, None).unwrap();
        assert_eq!(cv.emotion.valence, 1.0);
        assert_eq!(cv.emotion.arousal, -1.0);
        assert_eq!(cv.emotion.dominance, 1.0);
        assert_eq!(cv.emotion.label, "x"); // lowercased
    }

    #[test]
    fn emotion_missing_defaults_neutral() {
        let cv = parse_enrichment_envelope(r#"{"intent":"statement","topic":"x"}"#, None).unwrap();
        assert_eq!(cv.emotion, Vad::neutral());
    }

    #[test]
    fn goal_empty_string_is_none() {
        let cv =
            parse_enrichment_envelope(r#"{"intent":"statement","topic":"x","goal":"  "}"#, None)
                .unwrap();
        assert!(cv.goal.is_none());
        let cv =
            parse_enrichment_envelope(r#"{"intent":"statement","topic":"x","goal":"fix build"}"#, None)
                .unwrap();
        assert_eq!(cv.goal.as_deref(), Some("fix build"));
    }

    #[test]
    fn topic_capped_to_three_words() {
        let cv =
            parse_enrichment_envelope(r#"{"intent":"statement","topic":"One Two Three Four Five"}"#, None)
                .unwrap();
        assert_eq!(cv.topic, "one two three");
    }

    // -- Blob-shape golden (reuse A1's §D2 shape, tier now llm) -------------

    #[test]
    fn enriched_blob_has_exact_keys_and_llm_tier() {
        let cv = parse_enrichment_envelope(VALID, None).unwrap();
        let blob = cv.to_metadata_value();
        let obj = blob.as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["emotion", "goal", "intent", "tier", "topic", "v"]);
        assert_eq!(obj["tier"], serde_json::json!("llm"));
        assert_eq!(obj["v"], serde_json::json!(1));
    }

    // -- Prompt snapshot ----------------------------------------------------

    #[test]
    fn prompt_names_all_axes_and_intents() {
        let p = ENRICHMENT_SYSTEM_PROMPT;
        assert!(p.starts_with("You label a single conversation turn"));
        assert!(p.contains("JSON only"));
        for intent in ["question", "request", "statement", "correction", "feedback", "social", "meta"] {
            assert!(p.contains(intent), "prompt must name intent {intent}");
        }
        for axis in ["valence", "arousal", "dominance", "label"] {
            assert!(p.contains(axis), "prompt must name emotion axis {axis}");
        }
        assert!(p.contains("\"topic\""));
        assert!(p.contains("\"goal\""));
    }

    // -- enrich() integration (backend failure + config consumption) --------

    #[tokio::test]
    async fn enrich_returns_vector_on_valid_body() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok(VALID)));
        let cv = ec.enrich("how does snac work?", None).await.expect("valid enriches");
        assert_eq!(cv.intent, Intent::Question);
        assert_eq!(cv.tier, Tier::Llm);
    }

    #[tokio::test]
    async fn enrich_none_on_backend_error() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier {
            response: Err("transport: refused".into()),
            seen_model: Mutex::new(None),
        }));
        assert!(ec.enrich("x", None).await.is_none());
    }

    #[tokio::test]
    async fn enrich_none_on_empty_body() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok("   \n ")));
        assert!(ec.enrich("x", None).await.is_none());
    }

    #[tokio::test]
    async fn enrich_none_on_malformed_body() {
        let ec = EnrichmentClassifier::from_backend(Arc::new(MockClassifier::ok("nonsense")));
        assert!(ec.enrich("x", None).await.is_none());
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
            Some("haiku-3.5"),
            "from_config must thread model_override into the backend call"
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
