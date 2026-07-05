//! Turn classification — the keyword (sync) tier of the ADR-067 P2 classifier
//! (design `.planning/hermes-loop/classification-design.md`, plan A1).
//!
//! Every committed turn node on the ECC forest must carry a non-null, four-axis
//! `classification` blob so the graph view has stable strings for hue / glyph and
//! the floor has an arousal scalar (design §1, §5). This module supplies the
//! always-on, deterministic keyword extractor: pure CPU string ops, microseconds
//! per turn, safe on the `index_turn` witness path (design §D1).
//!
//! # Axes (taxonomy v1, design §D2)
//!
//! - **[`Intent`]** — closed 7-variant enum from surface cues (`?` → Question,
//!   verb-initial → Request, correction/social/feedback lexicons, Meta).
//! - **`topic`** — top non-stopword token via the graphify `tokenize` idiom, with
//!   a topic-continuity carry so per-turn flicker does not destabilise the GUI
//!   clusters (design §D2 caveat).
//! - **[`Vad`]** — valence/arousal/dominance + coarse label. Arousal is always
//!   present (default `0.5`) for the floor; `dominance` is always `0.0` because a
//!   keyword pass cannot infer it honestly (design §D2).
//! - **`goal`** — always `None` at this tier: inferring a goal from a single turn
//!   is unreliable and we do not fabricate one (design §D2). Spawn nodes fill it
//!   from `SpawnSpec.goal` separately (design §D5).
//!
//! The blob is emitted by [`ClassificationVector::to_metadata_value`] in exactly
//! the design §D2 shape (no wire change to the `conversation.graph` RPC).

use serde::Serialize;
use serde_json::Value;

/// Taxonomy version stamped into every blob (`"v": 1`, design §D2).
pub const TAXONOMY_VERSION: u8 = 1;

/// Neutral topic used when a turn carries no extractable token and there is no
/// prior topic to inherit (design §D2 topic-continuity).
pub const DEFAULT_TOPIC: &str = "general";

/// Conversational intent of a turn — the glyph axis for the graph view
/// (design §D2). Distinct from the routing archetype (`Reasoning`/`CodeGen`/…),
/// which is about tier complexity and stays untouched (design directive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Intent {
    /// A request for information — `?` or an interrogative opener.
    Question,
    /// An imperative / verb-initial ask for an action.
    Request,
    /// A declarative statement (the default when no other cue fires).
    Statement,
    /// A correction of a prior turn ("no", "actually", "wait", "i meant").
    Correction,
    /// Evaluative feedback on the assistant or its output ("you should", praise).
    Feedback,
    /// Social pleasantry — greeting, thanks, farewell.
    Social,
    /// Meta-conversational control ("start over", "nevermind", "new topic").
    Meta,
}

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

/// The full four-axis classification of a single turn (design §D2). Serialises
/// verbatim into the node metadata `classification` blob.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassificationVector {
    /// Conversational intent (glyph axis).
    pub intent: Intent,
    /// Short open-vocab topic tag (hue / cluster axis), `≤ 3` words.
    pub topic: String,
    /// Emotion VAD + label.
    pub emotion: Vad,
    /// Active goal thread, or `None` (always `None` at the keyword tier).
    pub goal: Option<String>,
    /// Provenance tier.
    pub tier: Tier,
    /// Taxonomy version.
    pub v: u8,
}

impl ClassificationVector {
    /// Produce the `classification` metadata blob in exactly the design §D2 shape
    /// (`{intent, topic, emotion:{valence,arousal,dominance,label}, goal, tier,
    /// v}`). Derived `Serialize` guarantees the key names and enum casing.
    pub fn to_metadata_value(&self) -> Value {
        serde_json::to_value(self).expect("ClassificationVector serialises")
    }
}

/// Extract the emotion arousal scalar from a causal node's metadata blob
/// (floor-arousal readiness, design §5). Returns `None` for a legacy node with no
/// `classification` (never a fabricated default), so the floor can distinguish a
/// classified turn from an unclassified one.
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

/// The always-on deterministic keyword classifier (design §D1, §D2). Holds no
/// state — continuity rides the `prev_topic` argument.
#[derive(Debug, Default, Clone)]
pub struct KeywordTurnClassifier;

impl KeywordTurnClassifier {
    /// Construct the keyword classifier (stateless).
    pub fn new() -> Self {
        Self
    }
}

impl TurnClassifier for KeywordTurnClassifier {
    fn classify(&self, _role: &str, text: &str, prev_topic: Option<&str>) -> ClassificationVector {
        ClassificationVector {
            intent: classify_intent(text),
            topic: classify_topic(text, prev_topic),
            emotion: classify_emotion(text),
            goal: None, // keyword tier never fabricates a goal (design §D2)
            tier: Tier::Keyword,
            v: TAXONOMY_VERSION,
        }
    }
}

// ---------------------------------------------------------------------------
// Intent
// ---------------------------------------------------------------------------

/// Verb-initial imperative openers → [`Intent::Request`] (design §D2).
const IMPERATIVE_VERBS: &[&str] = &[
    "add", "build", "check", "create", "delete", "explain", "find", "fix", "generate", "give",
    "help", "implement", "list", "make", "move", "open", "refactor", "remove", "rename", "run",
    "show", "tell", "update", "write",
];

/// Leading tokens / phrases that mark a [`Intent::Correction`].
const CORRECTION_CUES: &[&str] = &["no", "nope", "nah", "actually", "wait"];

/// Social opener tokens → [`Intent::Social`].
const SOCIAL_CUES: &[&str] = &[
    "hi", "hello", "hey", "yo", "thanks", "thank", "thx", "ty", "bye", "goodbye", "cheers", "lol",
    "haha", "gm",
];

/// Meta-conversational control phrases → [`Intent::Meta`].
const META_PHRASES: &[&str] = &[
    "start over",
    "nevermind",
    "never mind",
    "scratch that",
    "forget it",
    "ignore that",
    "new topic",
    "reset",
];

/// Evaluative feedback phrases → [`Intent::Feedback`].
const FEEDBACK_PHRASES: &[&str] = &[
    "you should",
    "it should",
    "that works",
    "that's good",
    "thats good",
    "looks good",
    "lgtm",
    "well done",
    "good job",
    "nice work",
    "good work",
    "great job",
];

/// Classify intent from surface cues. Precedence (design §D2): question mark →
/// correction → meta → social → feedback → imperative → statement default.
fn classify_intent(text: &str) -> Intent {
    let trimmed = text.trim();
    if trimmed.ends_with('?') {
        return Intent::Question;
    }
    let lower = trimmed.to_lowercase();
    let first = lower
        .split(|c: char| !c.is_alphanumeric())
        .find(|w| !w.is_empty())
        .unwrap_or("");

    if CORRECTION_CUES.contains(&first)
        || lower.contains("i meant")
        || lower.contains("that's wrong")
        || lower.contains("thats wrong")
        || lower.contains("not what i")
    {
        return Intent::Correction;
    }
    if META_PHRASES.iter().any(|p| lower.contains(p)) {
        return Intent::Meta;
    }
    if SOCIAL_CUES.contains(&first) || lower.contains("thank you") || lower.contains("good morning")
    {
        return Intent::Social;
    }
    if FEEDBACK_PHRASES.iter().any(|p| lower.contains(p)) {
        return Intent::Feedback;
    }
    // Imperative: verb-initial, allowing a leading politeness token.
    let head = match first {
        "please" | "pls" | "kindly" => lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .nth(1)
            .unwrap_or(""),
        other => other,
    };
    if IMPERATIVE_VERBS.contains(&head) {
        return Intent::Request;
    }
    Intent::Statement
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
/// at `±0.15`. Kept purely a function of the scalars (no intent coupling).
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

    fn classify(text: &str) -> ClassificationVector {
        KeywordTurnClassifier::new().classify("user", text, None)
    }

    // -- Intent cues (design §8) -------------------------------------------

    #[test]
    fn question_mark_is_question() {
        assert_eq!(classify("how does this work?").intent, Intent::Question);
        assert_eq!(classify("can you fix the parser?").intent, Intent::Question);
    }

    #[test]
    fn imperative_is_request() {
        assert_eq!(classify("write a function to sort").intent, Intent::Request);
        assert_eq!(classify("fix the SNAC decode").intent, Intent::Request);
        assert_eq!(classify("please add a test").intent, Intent::Request);
    }

    #[test]
    fn correction_lexicon() {
        assert_eq!(classify("no, that is not right").intent, Intent::Correction);
        assert_eq!(classify("actually the file is empty").intent, Intent::Correction);
        assert_eq!(classify("wait let me reconsider").intent, Intent::Correction);
        assert_eq!(classify("i meant the other module").intent, Intent::Correction);
    }

    #[test]
    fn social_lexicon() {
        assert_eq!(classify("thanks for the help").intent, Intent::Social);
        assert_eq!(classify("hi there").intent, Intent::Social);
        assert_eq!(classify("thank you so much").intent, Intent::Social);
        assert_eq!(classify("bye").intent, Intent::Social);
    }

    #[test]
    fn feedback_lexicon() {
        assert_eq!(classify("you should use a HashMap").intent, Intent::Feedback);
        assert_eq!(classify("that works nicely").intent, Intent::Feedback);
        assert_eq!(classify("looks good to me").intent, Intent::Feedback);
    }

    #[test]
    fn meta_lexicon() {
        assert_eq!(classify("start over please").intent, Intent::Meta);
        assert_eq!(classify("nevermind").intent, Intent::Meta);
        assert_eq!(classify("let's switch to a new topic").intent, Intent::Meta);
    }

    #[test]
    fn statement_default() {
        assert_eq!(classify("the daemon runs on port 8090").intent, Intent::Statement);
    }

    // -- Topic extraction + continuity (design §8) --------------------------

    #[test]
    fn topic_is_top_token() {
        // "voice" appears twice → top token.
        let cv = classify("the voice pipeline handles voice synthesis");
        assert_eq!(cv.topic, "voice");
    }

    #[test]
    fn topic_empty_falls_back() {
        // All stop words / too-short → default topic, no prior.
        let cv = KeywordTurnClassifier::new().classify("user", "is it?", None);
        assert_eq!(cv.topic, DEFAULT_TOPIC);
    }

    #[test]
    fn topic_continuity_inherits_when_still_mentioned() {
        // prev topic "voice" still appears → carried even if not the top token.
        let cv = KeywordTurnClassifier::new().classify(
            "user",
            "the decoder decoder also touches voice",
            Some("voice"),
        );
        assert_eq!(cv.topic, "voice");
    }

    #[test]
    fn topic_continuity_shifts_on_material_change() {
        // prev topic "voice" absent from this turn → adopt the new top token.
        let cv =
            KeywordTurnClassifier::new().classify("user", "the kernel scheduler stalled", Some("voice"));
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
    fn caps_raises_arousal() {
        assert!(classify("this is BROKEN").emotion.arousal > 0.5);
    }

    #[test]
    fn positive_lexicon_lifts_valence() {
        assert!(classify("this is great and awesome").emotion.valence > 0.15);
    }

    #[test]
    fn negative_lexicon_drops_valence() {
        assert!(classify("this is broken and terrible").emotion.valence < -0.15);
    }

    #[test]
    fn valence_stays_in_range() {
        let cv = classify("great great great great great awesome awesome awesome awesome");
        assert!(cv.emotion.valence <= 1.0 && cv.emotion.valence >= -1.0);
    }

    #[test]
    fn dominance_is_always_zero_and_arousal_default_neutral() {
        let cv = classify("the file exists");
        assert_eq!(cv.emotion.dominance, 0.0);
        assert_eq!(cv.emotion.arousal, 0.5); // no cue → neutral default
    }

    #[test]
    fn goal_is_always_none_at_keyword_tier() {
        assert!(classify("fix the bug now").goal.is_none());
    }

    // -- Blob shape golden (design §8: assert exact JSON keys) --------------

    #[test]
    fn blob_shape_has_exact_keys() {
        let blob = classify("how does the voice loop work?").to_metadata_value();
        let obj = blob.as_object().expect("blob is an object");
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["emotion", "goal", "intent", "tier", "topic", "v"]);

        let emo = obj["emotion"].as_object().expect("emotion is an object");
        let mut emo_keys: Vec<&str> = emo.keys().map(|s| s.as_str()).collect();
        emo_keys.sort_unstable();
        assert_eq!(emo_keys, ["arousal", "dominance", "label", "valence"]);

        // Enum casing + version stamp per §D2.
        assert_eq!(obj["intent"], serde_json::json!("question"));
        assert_eq!(obj["tier"], serde_json::json!("keyword"));
        assert_eq!(obj["v"], serde_json::json!(1));
        assert_eq!(obj["goal"], Value::Null);
    }

    // -- arousal_of round-trip (design §5, §8) ------------------------------

    #[test]
    fn arousal_of_reads_classified_node() {
        let blob = classify("this is BROKEN!!!").to_metadata_value();
        // Mirror the causal node metadata shape (classification nested under key).
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
