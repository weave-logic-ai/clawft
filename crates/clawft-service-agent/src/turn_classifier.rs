//! Turn classification — the keyword (sync) tier of the ADR-067 P2 classifier
//! (design `.planning/hermes-loop/classification-design.md`, plan A1 + §D2.1 v2).
//!
//! Every committed turn node on the ECC forest carries a non-null `classification`
//! blob so the graph view has stable strings for hue / glyph and the floor has an
//! arousal scalar (design §1, §5). This module supplies the always-on,
//! deterministic keyword extractor: pure CPU string ops, microseconds per turn,
//! safe on the `index_turn` witness path (design §D1).
//!
//! # Axes (taxonomy v2, design §D2 + §D2.1)
//!
//! - **`act`** — two-level dialogue act [`Act`] (`{class, refined}`) from
//!   [`dialogue_act`](crate::dialogue_act). The blob also serialises a legacy
//!   `intent` key projected from `act.refined` (v1 back-compat).
//! - **`topic`** — top non-stopword token via the graphify `tokenize` idiom, with
//!   a topic-continuity carry (design §D2 caveat).
//! - **`emotion`** — [`Vad`] valence/arousal/dominance + coarse label. Arousal is
//!   always present (default `0.5`) for the floor; `dominance` is always `0.0`.
//! - **`structure`** — typed entity spans + utterance-shape flags from
//!   [`text_structure`](crate::text_structure) (design §D2.1).
//! - **`goal`** — always `None` at this tier (no fabrication; design §D2).
//!
//! The blob is emitted by [`ClassificationVector::to_metadata_value`] in the
//! design §D2.1 shape; [`ClassificationVector::from_metadata_value`] reads it back
//! tolerating a `v:1` blob (no `act` / `structure`).

use serde::Serialize;
use serde_json::Value;

use crate::dialogue_act::{classify_act, Act, RefinedAct};
use crate::text_structure::{extract_structure, Structure};

/// Taxonomy version stamped into every blob (`"v": 2`, design §D2.1).
pub const TAXONOMY_VERSION: u8 = 2;

/// Neutral topic used when a turn carries no extractable token and there is no
/// prior topic to inherit (design §D2 topic-continuity).
pub const DEFAULT_TOPIC: &str = "general";

/// Provenance of a classification blob — lets consumers weight sources by
/// confidence (voice VAD > llm > keyword, design §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// The always-on deterministic keyword pass (this module).
    Keyword,
    /// The Phase-B async cheap-model refinement.
    Llm,
    /// Voice ECAPA VAD (authoritative emotion, ADR-061).
    Voice,
}

/// Emotion as valence/arousal/dominance scalars in `[-1, 1]` plus a coarse label
/// (design §D2). Arousal is always populated for the floor (design §5).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vad {
    /// Pleasantness, `[-1, 1]` (negative = unpleasant).
    pub valence: f32,
    /// Activation / intensity, `[-1, 1]` (default `0.5` neutral).
    pub arousal: f32,
    /// Control / assertiveness, `[-1, 1]`. Always `0.0` at the keyword tier.
    pub dominance: f32,
    /// Coarse human-readable label derived from the scalars.
    pub label: String,
}

impl Vad {
    /// A neutral VAD — the honest keyword default when no sentiment cue fires
    /// (arousal `0.5`, everything else `0.0`, label `"neutral"`, design §D2).
    pub fn neutral() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.5,
            dominance: 0.0,
            label: "neutral".to_string(),
        }
    }
}

/// The full classification of a single turn (design §D2 + §D2.1). Serialised into
/// the node metadata `classification` blob by [`Self::to_metadata_value`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassificationVector {
    /// Two-level dialogue act (glyph axis). The blob also carries a legacy
    /// `intent` key projected from `act.refined`.
    pub act: Act,
    /// Short open-vocab topic tag (hue / cluster axis), `≤ 3` words.
    pub topic: String,
    /// Emotion VAD + label.
    pub emotion: Vad,
    /// Structure / data-type layer — typed entity spans + utterance shape.
    pub structure: Structure,
    /// Active goal thread, or `None` (always `None` at the keyword tier).
    pub goal: Option<String>,
    /// Provenance tier.
    pub tier: Tier,
    /// Taxonomy version.
    pub v: u8,
}

impl ClassificationVector {
    /// Produce the `classification` metadata blob in the design §D2.1 shape.
    /// Built by hand (not derived `Serialize`) so the legacy `intent` key is
    /// injected as the projection of `act.refined` (v1 back-compat).
    pub fn to_metadata_value(&self) -> Value {
        let mut m = serde_json::Map::new();
        m.insert(
            "intent".into(),
            serde_json::to_value(self.act.intent()).expect("intent serialises"),
        );
        m.insert("act".into(), serde_json::to_value(self.act).expect("act serialises"));
        m.insert("topic".into(), Value::String(self.topic.clone()));
        m.insert(
            "emotion".into(),
            serde_json::to_value(&self.emotion).expect("emotion serialises"),
        );
        m.insert(
            "structure".into(),
            serde_json::to_value(&self.structure).expect("structure serialises"),
        );
        m.insert(
            "goal".into(),
            self.goal
                .as_ref()
                .map_or(Value::Null, |g| Value::String(g.clone())),
        );
        m.insert("tier".into(), serde_json::to_value(self.tier).expect("tier serialises"));
        m.insert("v".into(), Value::from(self.v));
        Value::Object(m)
    }

    /// Read a stored `classification` blob back into a vector, tolerating a `v:1`
    /// blob (design §D2.1): a missing `act` is upgraded from the legacy `intent`
    /// key, and a missing `structure` defaults to empty. Returns `None` only when
    /// neither `act` nor `intent` is present.
    pub fn from_metadata_value(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        let act = obj
            .get("act")
            .and_then(Act::from_value)
            .or_else(|| {
                obj.get("intent")
                    .and_then(|i| i.as_str())
                    .map(|s| Act::new(RefinedAct::from_intent_str(s)))
            })?;
        let topic = obj
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_TOPIC)
            .to_string();
        let emotion = read_vad(obj.get("emotion"));
        let structure = obj
            .get("structure")
            .and_then(Structure::from_value)
            .unwrap_or_default();
        let goal = obj
            .get("goal")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let tier = obj
            .get("tier")
            .and_then(|v| v.as_str())
            .map(read_tier)
            .unwrap_or(Tier::Keyword);
        let v = obj.get("v").and_then(Value::as_u64).unwrap_or(1) as u8;
        Some(Self {
            act,
            topic,
            emotion,
            structure,
            goal,
            tier,
            v,
        })
    }
}

/// Read a stored `emotion` object into a [`Vad`], defaulting missing axes
/// (arousal `0.5`, others `0.0`, label `"neutral"`).
fn read_vad(v: Option<&Value>) -> Vad {
    let Some(o) = v else {
        return Vad::neutral();
    };
    Vad {
        valence: o.get("valence").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        arousal: o.get("arousal").and_then(Value::as_f64).unwrap_or(0.5) as f32,
        dominance: o.get("dominance").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        label: o
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("neutral")
            .to_string(),
    }
}

/// Parse a stored `tier` string (unknown → keyword).
fn read_tier(s: &str) -> Tier {
    match s {
        "llm" => Tier::Llm,
        "voice" => Tier::Voice,
        _ => Tier::Keyword,
    }
}

/// Extract the emotion arousal scalar from a causal node's metadata blob
/// (floor-arousal readiness, design §5). Returns `None` for a legacy node with no
/// `classification` (never a fabricated default).
pub fn arousal_of(node_meta: &Value) -> Option<f32> {
    node_meta
        .get("classification")?
        .get("emotion")?
        .get("arousal")?
        .as_f64()
        .map(|v| v as f32)
}

/// A turn classifier: maps `(role, text)` to a [`ClassificationVector`].
///
/// `prev_topic` carries the prior turn's topic for the continuity heuristic
/// (design §D2) — the wiring (A2) threads it from the conversation's `ConvForest`
/// so a stable topic survives per-turn token flicker.
pub trait TurnClassifier: Send + Sync {
    /// Classify one turn. `role` is the speaker (`"user"` / `"assistant"`), `text`
    /// the turn content, `prev_topic` the last turn's topic (if any).
    fn classify(&self, role: &str, text: &str, prev_topic: Option<&str>) -> ClassificationVector;
}

/// The always-on deterministic keyword classifier (design §D1, §D2, §D2.1).
/// Stateless except for an optional enrolled-speaker vocabulary used for speaker
/// entity spans (empty ⇒ speaker spans are never guessed).
#[derive(Debug, Default, Clone)]
pub struct KeywordTurnClassifier {
    speakers: Vec<String>,
}

impl KeywordTurnClassifier {
    /// Construct the keyword classifier with no enrolled speakers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach an enrolled-speaker vocabulary so speaker names surface as
    /// [`EntityKind::Speaker`](crate::text_structure::EntityKind::Speaker) spans
    /// (design §D2.1 — the keyword tier never guesses names without a vocab).
    #[must_use]
    pub fn with_speakers(mut self, speakers: Vec<String>) -> Self {
        self.speakers = speakers;
        self
    }
}

impl TurnClassifier for KeywordTurnClassifier {
    fn classify(&self, _role: &str, text: &str, prev_topic: Option<&str>) -> ClassificationVector {
        ClassificationVector {
            act: classify_act(text),
            topic: classify_topic(text, prev_topic),
            emotion: classify_emotion(text),
            structure: extract_structure(text, &self.speakers),
            goal: None, // keyword tier never fabricates a goal (design §D2)
            tier: Tier::Keyword,
            v: TAXONOMY_VERSION,
        }
    }
}

// ---------------------------------------------------------------------------
// Topic
// ---------------------------------------------------------------------------

/// Classify topic: the most-frequent non-stopword token, with a topic-continuity
/// carry (design §D2). Inherit `prev_topic` while it is still mentioned in this
/// turn; adopt the new top token only on a material shift (prev no longer
/// present); fall back to `prev_topic` / [`DEFAULT_TOPIC`] on an empty token set.
fn classify_topic(text: &str, prev_topic: Option<&str>) -> String {
    let tokens = tokenize(text);
    if tokens.is_empty() {
        return prev_topic.unwrap_or(DEFAULT_TOPIC).to_string();
    }
    // Continuity: if the prior topic is still present in this turn, keep it — a
    // per-turn top-token flickers, the carry stabilises the GUI cluster.
    if let Some(prev) = prev_topic
        && tokens.iter().any(|t| t == prev)
    {
        return prev.to_string();
    }
    top_token(&tokens)
}

/// The most frequent token; ties broken by first appearance (deterministic).
fn top_token(tokens: &[String]) -> String {
    let mut counts: Vec<(&str, usize, usize)> = Vec::new(); // (token, count, first_index)
    for (idx, tok) in tokens.iter().enumerate() {
        if let Some(entry) = counts.iter_mut().find(|(t, _, _)| *t == tok.as_str()) {
            entry.1 += 1;
        } else {
            counts.push((tok.as_str(), 1, idx));
        }
    }
    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)))
        .map(|(t, _, _)| t.to_string())
        .unwrap_or_else(|| DEFAULT_TOPIC.to_string())
}

/// Tokenize into lowercase keywords, filtering stop words.
///
/// Copied from the graphify topic extractor
/// (`clawft-graphify/src/conversation.rs::tokenize`, design §4 "reuse") so the
/// turn classifier's topic axis matches the entity-graph topic idiom without a
/// cross-crate dependency.
fn tokenize(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "shall", "should", "may", "might", "must", "can",
        "could", "about", "above", "after", "again", "all", "also", "am", "and", "any", "as", "at",
        "because", "before", "between", "both", "but", "by", "came", "come", "each", "for", "from",
        "get", "got", "he", "her", "here", "him", "his", "how", "i", "if", "in", "into", "it",
        "its", "just", "know", "let", "like", "make", "me", "more", "most", "my", "no", "not",
        "now", "of", "on", "one", "only", "or", "other", "our", "out", "over", "said", "same",
        "she", "so", "some", "still", "such", "take", "tell", "than", "that", "their", "them",
        "then", "there", "these", "they", "this", "those", "through", "to", "too", "under", "up",
        "very", "want", "what", "when", "where", "which", "while", "who", "why", "with", "you",
        "your",
    ];

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty() && w.len() > 1 && !STOP_WORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Emotion (VAD)
// ---------------------------------------------------------------------------

/// Positive-sentiment words (valence up).
const POSITIVE_WORDS: &[&str] = &[
    "good", "great", "love", "awesome", "nice", "perfect", "excellent", "happy", "thanks",
    "wonderful", "glad", "cool", "works", "yes", "amazing", "brilliant",
];

/// Negative-sentiment words (valence down).
const NEGATIVE_WORDS: &[&str] = &[
    "bad", "hate", "wrong", "broken", "error", "fail", "failed", "terrible", "awful", "angry",
    "frustrated", "annoying", "stupid", "ugh", "crash", "bug", "stuck", "confused", "useless",
];

/// Intensity words that raise arousal.
const INTENSITY_WORDS: &[&str] = &[
    "very", "really", "so", "extremely", "urgent", "asap", "now", "immediately", "please",
];

/// Derive the emotion VAD from a small sentiment/intensity lexicon plus surface
/// cues — exclamation, all-caps, repetition (design §D2). `dominance` is always
/// `0.0` (a keyword pass cannot infer it honestly).
fn classify_emotion(text: &str) -> Vad {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    let mut valence = 0.0f32;
    let mut arousal = 0.5f32;
    for w in &words {
        if POSITIVE_WORDS.contains(w) {
            valence += 0.3;
        }
        if NEGATIVE_WORDS.contains(w) {
            valence -= 0.3;
        }
        if INTENSITY_WORDS.contains(w) {
            arousal += 0.1;
        }
    }

    // Surface cues raise arousal: exclamation, all-caps shouting, elongation.
    arousal += 0.15 * text.matches('!').count() as f32;
    if has_caps_word(text) {
        arousal += 0.15;
    }
    if has_elongation(text) {
        arousal += 0.1;
    }

    let valence = valence.clamp(-1.0, 1.0);
    let arousal = arousal.clamp(-1.0, 1.0);
    Vad {
        valence,
        arousal,
        dominance: 0.0,
        label: emotion_label(valence, arousal),
    }
}

/// True if any word is an all-caps run of ≥ 3 letters (shouting cue).
fn has_caps_word(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric()).any(|w| {
        w.len() >= 3
            && w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && w.chars().any(|c| c.is_ascii_uppercase())
    })
}

/// True if any character repeats ≥ 3 times in a row (e.g. "soooo", "!!!").
fn has_elongation(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.windows(3).any(|w| w[0] == w[1] && w[1] == w[2] && w[0].is_alphanumeric())
}

/// Map (valence, arousal) to a coarse label. High arousal `> 0.6`; valence bands
/// at `±0.15`. Kept purely a function of the scalars (no act coupling).
fn emotion_label(valence: f32, arousal: f32) -> String {
    let high = arousal > 0.6;
    let label = if valence > 0.15 {
        if high {
            "excited"
        } else {
            "pleased"
        }
    } else if valence < -0.15 {
        if high {
            "frustrated"
        } else {
            "unhappy"
        }
    } else if high {
        "engaged"
    } else {
        "neutral"
    };
    label.to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialogue_act::{RefinedAct, SpeechActClass};
    use crate::text_structure::EntityKind;

    fn classify(text: &str) -> ClassificationVector {
        KeywordTurnClassifier::new().classify("user", text, None)
    }

    // -- Composed act + structure ------------------------------------------

    #[test]
    fn classify_sets_act_and_structure() {
        let cv = classify("open crates/foo/bar.rs and then run the tests");
        assert_eq!(cv.act.refined, RefinedAct::Command);
        assert_eq!(cv.act.class, SpeechActClass::Directive);
        assert!(cv.structure.shape.multi_part, "'and then' → multi-part");
        assert!(cv
            .structure
            .entities
            .iter()
            .any(|e| e.kind == EntityKind::Path && e.text == "crates/foo/bar.rs"));
    }

    // -- Topic extraction + continuity (design §8) --------------------------

    #[test]
    fn topic_is_top_token() {
        let cv = classify("the voice pipeline handles voice synthesis");
        assert_eq!(cv.topic, "voice");
    }

    #[test]
    fn topic_empty_falls_back() {
        let cv = KeywordTurnClassifier::new().classify("user", "is it?", None);
        assert_eq!(cv.topic, DEFAULT_TOPIC);
    }

    #[test]
    fn topic_continuity_inherits_when_still_mentioned() {
        let cv = KeywordTurnClassifier::new().classify(
            "user",
            "the decoder decoder also touches voice",
            Some("voice"),
        );
        assert_eq!(cv.topic, "voice");
    }

    #[test]
    fn topic_continuity_shifts_on_material_change() {
        let cv = KeywordTurnClassifier::new().classify(
            "user",
            "the kernel scheduler stalled",
            Some("voice"),
        );
        assert_ne!(cv.topic, "voice");
        assert!(matches!(cv.topic.as_str(), "kernel" | "scheduler" | "stalled"));
    }

    // -- Emotion VAD lexicon cases (design §8) ------------------------------

    #[test]
    fn exclamation_raises_arousal() {
        let plain = classify("the build passed").emotion.arousal;
        let shout = classify("the build passed!!!").emotion.arousal;
        assert!(shout > plain);
        assert!(shout > 0.6);
    }

    #[test]
    fn positive_and_negative_lexicon() {
        assert!(classify("this is great and awesome").emotion.valence > 0.15);
        assert!(classify("this is broken and terrible").emotion.valence < -0.15);
    }

    #[test]
    fn dominance_zero_arousal_default() {
        let cv = classify("the file exists");
        assert_eq!(cv.emotion.dominance, 0.0);
        assert_eq!(cv.emotion.arousal, 0.5);
    }

    #[test]
    fn goal_is_always_none_at_keyword_tier() {
        assert!(classify("fix the bug now").goal.is_none());
    }

    // -- Blob shape golden (v2 keys) ----------------------------------------

    #[test]
    fn blob_shape_v2_keys() {
        let blob = classify("how does the voice loop work?").to_metadata_value();
        let obj = blob.as_object().expect("blob is an object");
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["act", "emotion", "goal", "intent", "structure", "tier", "topic", "v"]
        );

        // Legacy intent key still projected from the refined act.
        assert_eq!(obj["intent"], serde_json::json!("question"));
        // New act layer.
        assert_eq!(obj["act"]["class"], serde_json::json!("interrogative"));
        assert_eq!(obj["act"]["refined"], serde_json::json!("question"));
        // Structure layer present with its sub-shape.
        assert!(obj["structure"]["entities"].is_array());
        assert!(obj["structure"]["shape"]["multi_part"].is_boolean());
        assert_eq!(obj["structure"]["argument"], Value::Null);

        assert_eq!(obj["tier"], serde_json::json!("keyword"));
        assert_eq!(obj["v"], serde_json::json!(2));
        assert_eq!(obj["goal"], Value::Null);

        let emo = obj["emotion"].as_object().expect("emotion object");
        let mut emo_keys: Vec<&str> = emo.keys().map(|s| s.as_str()).collect();
        emo_keys.sort_unstable();
        assert_eq!(emo_keys, ["arousal", "dominance", "label", "valence"]);
    }

    // -- v1-blob read tolerance (design §D2.1) ------------------------------

    #[test]
    fn from_metadata_value_upgrades_v1_blob() {
        // A legacy v1 blob: flat `intent`, no `act` / `structure`.
        let v1 = serde_json::json!({
            "intent": "request",
            "topic": "voice",
            "emotion": { "valence": 0.0, "arousal": 0.5, "dominance": 0.0, "label": "neutral" },
            "goal": null,
            "tier": "keyword",
            "v": 1
        });
        let cv = ClassificationVector::from_metadata_value(&v1).expect("v1 blob reads");
        // intent → refined act upgrade.
        assert_eq!(cv.act.refined, RefinedAct::Command);
        assert_eq!(cv.act.class, SpeechActClass::Directive);
        // Missing structure defaults to empty.
        assert_eq!(cv.structure, Structure::default());
        assert_eq!(cv.topic, "voice");
        assert_eq!(cv.v, 1);
    }

    #[test]
    fn to_from_metadata_value_roundtrips_v2() {
        let cv = classify("write the parser in src/parse.rs");
        let back = ClassificationVector::from_metadata_value(&cv.to_metadata_value())
            .expect("v2 blob reads");
        assert_eq!(back.act, cv.act);
        assert_eq!(back.structure, cv.structure);
        assert_eq!(back.topic, cv.topic);
        assert_eq!(back.v, 2);
    }

    // -- arousal_of round-trip (design §5, §8) ------------------------------

    #[test]
    fn arousal_of_reads_classified_node() {
        let blob = classify("this is BROKEN!!!").to_metadata_value();
        let node_meta = serde_json::json!({ "classification": blob, "state": "frontier" });
        let a = arousal_of(&node_meta).expect("classified node carries arousal");
        assert!(a > 0.6);
    }

    #[test]
    fn arousal_of_returns_none_for_legacy_node() {
        let legacy = serde_json::json!({ "state": "frontier", "role": "user" });
        assert!(arousal_of(&legacy).is_none());
    }
}
