//! Dialogue-act taxonomy — the two-level act layer of classification v2
//! (design `.planning/hermes-loop/classification-design.md` §D2.1).
//!
//! v2 evolves the flat v1 `intent` axis into `act: { class, refined }`: a coarse
//! Searle-style [`SpeechActClass`] over a refined [`RefinedAct`]. The old
//! [`Intent`] enum is retained purely as the **projection** target — the blob
//! keeps serializing an `intent` key derived from `refined` so every v1 consumer
//! (graph glyph, UID readers) keeps working (design §D2.1 back-compat).
//!
//! Keyword-tier act detection ([`classify_act`]) extends the v1 surface-cue
//! logic: clarification request/provide, command, comment, acknowledgment, and
//! the carried-over correction / feedback / social / meta cues.

use serde::Serialize;
use serde_json::Value;

/// Legacy flat intent (v1) — retained as the projection target for the blob's
/// back-compat `intent` key (design §D2.1). Not set directly in v2; derived from
/// [`RefinedAct::intent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Intent {
    /// A request for information.
    Question,
    /// An imperative / verb-initial ask for an action.
    Request,
    /// A declarative statement.
    Statement,
    /// A correction of a prior turn.
    Correction,
    /// Evaluative feedback.
    Feedback,
    /// Social pleasantry.
    Social,
    /// Meta-conversational control.
    Meta,
}

/// Coarse speech-act class — the top level of the v2 act taxonomy (design §D2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechActClass {
    /// Seeks information.
    Interrogative,
    /// Seeks an action / controls the exchange.
    Directive,
    /// Commits the speaker to a state of the world.
    Assertive,
    /// Conveys attitude / social stance.
    Expressive,
    /// Commits the speaker to future action (LLM-tier only in v2).
    Commissive,
}

/// Refined dialogue act — the specific conversational move (design §D2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefinedAct {
    /// Asks for information (`?` without a clarification cue).
    Question,
    /// Asks the other party to clarify a prior turn.
    ClarificationRequest,
    /// Provides clarification of one's own prior turn.
    ClarificationProvide,
    /// A verb-initial imperative ask for an action.
    Command,
    /// A declarative assertion (the default).
    Comment,
    /// Corrects a prior turn.
    Correction,
    /// Evaluates the assistant or its output.
    Feedback,
    /// Backchannel acknowledgment ("ok", "got it").
    Acknowledgment,
    /// Social pleasantry — greeting / thanks / farewell.
    Social,
    /// Meta-conversational control.
    Meta,
}

impl RefinedAct {
    /// The coarse [`SpeechActClass`] this act rolls up to (design §D2.1 table).
    pub fn class(self) -> SpeechActClass {
        use RefinedAct::*;
        use SpeechActClass::*;
        match self {
            Question | ClarificationRequest => Interrogative,
            Command | Meta => Directive,
            ClarificationProvide | Comment | Correction => Assertive,
            Feedback | Acknowledgment | Social => Expressive,
        }
    }

    /// Project to the legacy v1 [`Intent`] for the blob's back-compat `intent`
    /// key (design §D2.1 table).
    pub fn intent(self) -> Intent {
        use RefinedAct::*;
        match self {
            Question | ClarificationRequest => Intent::Question,
            Command => Intent::Request,
            ClarificationProvide | Comment => Intent::Statement,
            Correction => Intent::Correction,
            Feedback => Intent::Feedback,
            Acknowledgment | Social => Intent::Social,
            Meta => Intent::Meta,
        }
    }

    /// Upgrade a legacy v1 `intent` string into a refined act (design §D2.1
    /// back-compat — used when reading a `v:1` blob). Unknown → [`Comment`].
    pub fn from_intent_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "question" => RefinedAct::Question,
            "request" => RefinedAct::Command,
            "statement" => RefinedAct::Comment,
            "correction" => RefinedAct::Correction,
            "feedback" => RefinedAct::Feedback,
            "social" => RefinedAct::Social,
            "meta" => RefinedAct::Meta,
            _ => RefinedAct::Comment,
        }
    }

    /// Exact match on the kebab-case wire name (used by the LLM parser).
    pub fn from_wire(s: &str) -> Option<Self> {
        use RefinedAct::*;
        Some(match s.trim().to_lowercase().as_str() {
            "question" => Question,
            "clarification-request" => ClarificationRequest,
            "clarification-provide" => ClarificationProvide,
            "command" => Command,
            "comment" => Comment,
            "correction" => Correction,
            "feedback" => Feedback,
            "acknowledgment" | "acknowledgement" => Acknowledgment,
            "social" => Social,
            "meta" => Meta,
            _ => return None,
        })
    }
}

/// The two-level dialogue act: coarse [`SpeechActClass`] + [`RefinedAct`]
/// (serialises to `{ "class": …, "refined": … }`, design §D2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Act {
    /// Coarse illocutionary class.
    pub class: SpeechActClass,
    /// Refined conversational move.
    pub refined: RefinedAct,
}

impl Act {
    /// Build an act from a refined move (the class is derived deterministically).
    pub fn new(refined: RefinedAct) -> Self {
        Self {
            class: refined.class(),
            refined,
        }
    }

    /// The legacy v1 [`Intent`] projection for the back-compat blob key.
    pub fn intent(self) -> Intent {
        self.refined.intent()
    }

    /// Read an `act` blob (`{class, refined}`), tolerating a missing/garbled
    /// `class` by re-deriving it from `refined`. Returns `None` only when
    /// `refined` is absent/unknown (caller falls back to the `intent` key).
    pub fn from_value(v: &Value) -> Option<Self> {
        let refined = RefinedAct::from_wire(v.get("refined")?.as_str()?)?;
        Some(Act::new(refined)) // class always re-derived (never trust a stale class)
    }
}

// ---------------------------------------------------------------------------
// Keyword-tier act detection
// ---------------------------------------------------------------------------

/// Verb-initial imperative openers → [`RefinedAct::Command`].
const IMPERATIVE_VERBS: &[&str] = &[
    "add", "build", "check", "create", "delete", "explain", "find", "fix", "generate", "give",
    "help", "implement", "list", "make", "move", "open", "refactor", "remove", "rename", "run",
    "show", "tell", "update", "write",
];

/// Leading tokens marking a [`RefinedAct::Correction`].
const CORRECTION_CUES: &[&str] = &["no", "nope", "nah", "actually", "wait"];

/// Social opener tokens → [`RefinedAct::Social`].
const SOCIAL_CUES: &[&str] = &[
    "hi", "hello", "hey", "yo", "thanks", "thank", "thx", "ty", "bye", "goodbye", "cheers", "lol",
    "haha", "gm",
];

/// Acknowledgment opener tokens → [`RefinedAct::Acknowledgment`].
const ACK_CUES: &[&str] = &[
    "ok", "okay", "k", "kk", "yep", "yeah", "yup", "sure", "understood", "roger", "ack", "noted",
    "cool",
];

/// Acknowledgment phrases → [`RefinedAct::Acknowledgment`].
const ACK_PHRASES: &[&str] = &["got it", "makes sense", "sounds good", "will do", "good to know"];

/// Meta-conversational control phrases → [`RefinedAct::Meta`].
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

/// Second-person modal advice → [`RefinedAct::Command`]. Live voice-session
/// calibration: "you should/need to <verb>" reads as a softened directive, not
/// evaluative feedback (design §D2.1).
const SECOND_PERSON_DIRECTIVE: &[&str] = &[
    "you should",
    "you need to",
    "you've got to",
    "you have to",
    "you gotta",
    "you must",
    "you ought to",
];

/// Evaluative feedback phrases → [`RefinedAct::Feedback`]. Note: second-person
/// "you should <verb>" is a directive ([`SECOND_PERSON_DIRECTIVE`]); only the
/// third-person "it should" and the praise lexicon stay feedback.
const FEEDBACK_PHRASES: &[&str] = &[
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

/// Clarification-request cues (used with a trailing `?`).
const CLARIFY_REQUEST_CUES: &[&str] = &[
    "what do you mean",
    "what does that mean",
    "which one",
    "you mean",
    "do you mean",
    "clarify",
    "not sure what you mean",
];

/// Bare clarification-request utterances (short backchannel questions).
const CLARIFY_REQUEST_BARE: &[&str] = &["what?", "huh?", "sorry?", "pardon?", "come again?"];

/// Clarification-provide cues (no trailing `?`).
const CLARIFY_PROVIDE_CUES: &[&str] = &[
    "i mean",
    "i meant",
    "what i meant",
    "in other words",
    "to be clear",
    "let me clarify",
    "by that i mean",
    "to clarify",
];

/// Classify the refined dialogue act from surface cues (design §D2.1). Precedence
/// resolves overlaps: question/clarification-request (`?`) → correction →
/// clarification-provide → meta → social → acknowledgment → feedback → command →
/// comment default.
pub fn classify_act(text: &str) -> Act {
    Act::new(classify_refined(text))
}

fn classify_refined(text: &str) -> RefinedAct {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();

    if CLARIFY_REQUEST_BARE.contains(&lower.as_str()) {
        return RefinedAct::ClarificationRequest;
    }
    if trimmed.ends_with('?') {
        if CLARIFY_REQUEST_CUES.iter().any(|c| lower.contains(c)) {
            return RefinedAct::ClarificationRequest;
        }
        return RefinedAct::Question;
    }

    let first = lower
        .split(|c: char| !c.is_alphanumeric())
        .find(|w| !w.is_empty())
        .unwrap_or("");

    if CORRECTION_CUES.contains(&first)
        || lower.contains("that's wrong")
        || lower.contains("thats wrong")
        || lower.contains("not what i")
    {
        return RefinedAct::Correction;
    }
    if CLARIFY_PROVIDE_CUES.iter().any(|c| lower.contains(c)) {
        return RefinedAct::ClarificationProvide;
    }
    if META_PHRASES.iter().any(|p| lower.contains(p)) {
        return RefinedAct::Meta;
    }
    if SOCIAL_CUES.contains(&first) || lower.contains("thank you") || lower.contains("good morning")
    {
        return RefinedAct::Social;
    }
    if ACK_CUES.contains(&first) || ACK_PHRASES.iter().any(|p| lower.contains(p)) {
        return RefinedAct::Acknowledgment;
    }
    // Second-person modal advice ("you should X") is a softened command, checked
    // before feedback so it lands as a directive (live-session calibration).
    if SECOND_PERSON_DIRECTIVE.iter().any(|p| lower.contains(p)) {
        return RefinedAct::Command;
    }
    if FEEDBACK_PHRASES.iter().any(|p| lower.contains(p)) {
        return RefinedAct::Feedback;
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
        return RefinedAct::Command;
    }
    RefinedAct::Comment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(text: &str) -> RefinedAct {
        classify_act(text).refined
    }

    #[test]
    fn question_vs_clarification_request() {
        assert_eq!(act("how does the loop work?"), RefinedAct::Question);
        assert_eq!(act("what do you mean by that?"), RefinedAct::ClarificationRequest);
        assert_eq!(act("which one did you change?"), RefinedAct::ClarificationRequest);
        assert_eq!(act("huh?"), RefinedAct::ClarificationRequest);
    }

    #[test]
    fn clarification_provide() {
        assert_eq!(act("i mean the second file"), RefinedAct::ClarificationProvide);
        assert_eq!(act("to be clear, the daemon not the CLI"), RefinedAct::ClarificationProvide);
        assert_eq!(act("in other words, cache it"), RefinedAct::ClarificationProvide);
    }

    #[test]
    fn command_and_comment() {
        assert_eq!(act("write a sort function"), RefinedAct::Command);
        assert_eq!(act("please fix the parser"), RefinedAct::Command);
        assert_eq!(act("the daemon runs on 8090"), RefinedAct::Comment);
    }

    #[test]
    fn correction_feedback_social_meta() {
        assert_eq!(act("no, that is wrong"), RefinedAct::Correction);
        assert_eq!(act("looks good to me"), RefinedAct::Feedback);
        assert_eq!(act("that works nicely"), RefinedAct::Feedback);
        assert_eq!(act("thanks so much"), RefinedAct::Social);
        assert_eq!(act("start over please"), RefinedAct::Meta);
    }

    #[test]
    fn second_person_advice_is_command() {
        // Live voice-session calibration: "you should <verb>" reads as a softened
        // directive, not evaluative feedback.
        assert_eq!(act("You should figure this out quickly."), RefinedAct::Command);
        assert_eq!(act("you should use a HashMap"), RefinedAct::Command);
        assert_eq!(act("you need to fix the parser"), RefinedAct::Command);
        assert_eq!(act("you have to restart the daemon"), RefinedAct::Command);
        assert_eq!(
            classify_act("you should refactor this").class,
            SpeechActClass::Directive
        );
    }

    #[test]
    fn disfluent_question_still_question() {
        // Live calibration: disfluencies/false-starts don't break the '?'
        // interrogative signal.
        assert_eq!(act("That uh that what do you call it?"), RefinedAct::Question);
    }

    #[test]
    fn acknowledgment() {
        assert_eq!(act("ok"), RefinedAct::Acknowledgment);
        assert_eq!(act("got it, moving on"), RefinedAct::Acknowledgment);
        assert_eq!(act("makes sense"), RefinedAct::Acknowledgment);
    }

    #[test]
    fn class_and_intent_projection() {
        assert_eq!(RefinedAct::ClarificationRequest.class(), SpeechActClass::Interrogative);
        assert_eq!(RefinedAct::Command.class(), SpeechActClass::Directive);
        assert_eq!(RefinedAct::Comment.class(), SpeechActClass::Assertive);
        assert_eq!(RefinedAct::Acknowledgment.class(), SpeechActClass::Expressive);
        // Old intent projection stays within the v1 7-value set.
        assert_eq!(RefinedAct::ClarificationRequest.intent(), Intent::Question);
        assert_eq!(RefinedAct::Command.intent(), Intent::Request);
        assert_eq!(RefinedAct::ClarificationProvide.intent(), Intent::Statement);
        assert_eq!(RefinedAct::Acknowledgment.intent(), Intent::Social);
    }

    #[test]
    fn v1_intent_upgrade_roundtrip() {
        assert_eq!(RefinedAct::from_intent_str("request"), RefinedAct::Command);
        assert_eq!(RefinedAct::from_intent_str("statement"), RefinedAct::Comment);
        assert_eq!(RefinedAct::from_intent_str("question"), RefinedAct::Question);
        assert_eq!(RefinedAct::from_intent_str("bogus"), RefinedAct::Comment);
    }

    #[test]
    fn act_serialises_class_and_refined() {
        let blob = serde_json::to_value(Act::new(RefinedAct::ClarificationRequest)).unwrap();
        assert_eq!(blob["class"], serde_json::json!("interrogative"));
        assert_eq!(blob["refined"], serde_json::json!("clarification-request"));
    }

    #[test]
    fn act_from_value_rederives_class() {
        // A stale/garbled class is ignored; class is re-derived from refined.
        let v = serde_json::json!({ "class": "wrong", "refined": "command" });
        let a = Act::from_value(&v).unwrap();
        assert_eq!(a.class, SpeechActClass::Directive);
        assert_eq!(a.refined, RefinedAct::Command);
        assert!(Act::from_value(&serde_json::json!({ "refined": "nope" })).is_none());
    }
}
