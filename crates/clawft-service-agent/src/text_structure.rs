//! Structure / data-type layer of classification v2 (design
//! `.planning/hermes-loop/classification-design.md` §D2.1).
//!
//! The keyword tier honestly extracts *typed entity spans* (numbers, dates,
//! times, durations, paths, URLs, quotes, code-ish tokens, and — only when an
//! enrolled-speaker vocabulary is supplied — speaker names) by regex/heuristic,
//! each with a per-span confidence, plus coarse *utterance-shape* flags
//! (multi-part, conditional, refers-to-prior). The `argument` (a command's
//! verb+object) is **left `None` at the keyword tier** — reliable
//! predicate-argument extraction needs the LLM (same honesty stance as `goal`),
//! so the enrichment tier fills it.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

/// Hard cap on extracted spans so a pathological turn can't flood the blob.
const MAX_ENTITIES: usize = 32;

/// The kind of a typed entity span (design §D2.1 table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    /// A bare number.
    Number,
    /// A date (`YYYY-MM-DD`, `M/D`, month-name + day).
    Date,
    /// A clock time (`HH:MM`, `N am/pm`).
    Time,
    /// A duration (`N ms|s|min|h|d|w`).
    Duration,
    /// A file-system path or code file name.
    Path,
    /// A URL.
    Url,
    /// A double-quoted string.
    Quote,
    /// A code-ish token (backtick span, `a::b`, `foo()`, `snake_case`).
    Code,
    /// An enrolled speaker's name.
    Speaker,
}

/// A typed entity span extracted from the turn text.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntitySpan {
    /// What kind of datum this is.
    pub kind: EntityKind,
    /// The matched surface text.
    pub text: String,
    /// Per-span confidence in `[0, 1]`.
    pub confidence: f32,
}

/// Coarse utterance-shape flags (design §D2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct UtteranceShape {
    /// The turn chains multiple parts (conjunction / sequencing — "and then").
    pub multi_part: bool,
    /// The turn is conditional ("if …", "when …", "unless …").
    pub conditional: bool,
    /// The turn refers to a prior turn ("that one", "like before").
    pub refers_prior: bool,
}

/// A command's predicate-argument structure. Keyword tier leaves this `None`;
/// the enrichment (LLM) tier fills it (design §D2.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Argument {
    /// The command's verb.
    pub verb: String,
    /// The command's object, when clear.
    pub object: Option<String>,
}

/// The structure/data-type layer of the classification blob (design §D2.1).
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Structure {
    /// Typed entity spans, in text order.
    pub entities: Vec<EntitySpan>,
    /// Coarse utterance-shape flags.
    pub shape: UtteranceShape,
    /// Predicate-argument structure (LLM tier only).
    pub argument: Option<Argument>,
}

impl Structure {
    /// Read a stored `structure` blob back into a [`Structure`], tolerating a
    /// partial / absent shape (used by the v1/v2 read path). A `v:1` blob has no
    /// `structure` key, so the caller supplies [`Structure::default`] instead.
    pub fn from_value(v: &Value) -> Option<Self> {
        let obj = v.as_object()?;
        let entities = obj
            .get("entities")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(entity_from_value).collect())
            .unwrap_or_default();
        let shape = obj
            .get("shape")
            .map(|s| UtteranceShape {
                multi_part: s.get("multi_part").and_then(Value::as_bool).unwrap_or(false),
                conditional: s.get("conditional").and_then(Value::as_bool).unwrap_or(false),
                refers_prior: s.get("refers_prior").and_then(Value::as_bool).unwrap_or(false),
            })
            .unwrap_or_default();
        let argument = obj.get("argument").and_then(|a| {
            let verb = a.get("verb")?.as_str()?.to_string();
            let object = a.get("object").and_then(Value::as_str).map(str::to_string);
            Some(Argument { verb, object })
        });
        Some(Self {
            entities,
            shape,
            argument,
        })
    }
}

fn entity_from_value(v: &Value) -> Option<EntitySpan> {
    let kind = match v.get("kind")?.as_str()? {
        "number" => EntityKind::Number,
        "date" => EntityKind::Date,
        "time" => EntityKind::Time,
        "duration" => EntityKind::Duration,
        "path" => EntityKind::Path,
        "url" => EntityKind::Url,
        "quote" => EntityKind::Quote,
        "code" => EntityKind::Code,
        "speaker" => EntityKind::Speaker,
        _ => return None,
    };
    Some(EntitySpan {
        kind,
        text: v.get("text")?.as_str()?.to_string(),
        confidence: v.get("confidence").and_then(Value::as_f64).unwrap_or(0.5) as f32,
    })
}

/// One entity-matcher: a compiled regex, the kind it yields, and a confidence.
struct Matcher {
    kind: EntityKind,
    re: Regex,
    confidence: f32,
}

/// Ordered entity matchers. Priority matters: earlier matchers claim byte ranges
/// so a duration's digits are not also emitted as a bare number, a URL's slashes
/// are not a path, etc. (design §D2.1 — first claim wins).
static MATCHERS: LazyLock<Vec<Matcher>> = LazyLock::new(|| {
    let m = |kind, pat: &str, confidence| Matcher {
        kind,
        re: Regex::new(pat).expect("valid entity regex"),
        confidence,
    };
    vec![
        m(EntityKind::Url, r"(?i)\bhttps?://\S+", 0.95),
        m(EntityKind::Quote, r#""[^"]{1,200}""#, 0.90),
        m(EntityKind::Code, r"`[^`]{1,200}`", 0.80),
        // Path: a slashed path, or a bare filename with a code extension.
        m(EntityKind::Path, r"(?:~|\.\.?)?/?[\w.\-]+(?:/[\w.\-]+)+", 0.90),
        m(
            EntityKind::Path,
            r"\b[\w.\-]+\.(?:rs|toml|md|jsonc?|tsx?|jsx?|py|txt|ya?ml|lock|sh|rb|go|cpp|hpp|[ch])\b",
            0.90,
        ),
        m(EntityKind::Date, r"\b\d{4}-\d{2}-\d{2}\b", 0.85),
        m(
            EntityKind::Date,
            r"(?i)\b(?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\.?\s+\d{1,2}(?:st|nd|rd|th)?\b",
            0.85,
        ),
        m(EntityKind::Date, r"\b\d{1,2}/\d{1,2}(?:/\d{2,4})?\b", 0.80),
        m(EntityKind::Time, r"(?i)\b\d{1,2}:\d{2}(?::\d{2})?\s*(?:am|pm)?\b", 0.85),
        m(EntityKind::Time, r"(?i)\b\d{1,2}\s*(?:am|pm)\b", 0.85),
        m(
            EntityKind::Duration,
            r"(?i)\b\d+(?:\.\d+)?\s?(?:ms|secs?|seconds?|mins?|minutes?|hrs?|hours?|days?|weeks?|[smhdw])\b",
            0.80,
        ),
        m(EntityKind::Code, r"\b\w+(?:::\w+)+\b", 0.75),
        m(EntityKind::Code, r"\b[A-Za-z_]\w*\(\)", 0.75),
        m(EntityKind::Code, r"\b[a-z]+_[a-z0-9_]+\b", 0.70),
        m(EntityKind::Number, r"\b\d+(?:\.\d+)?\b", 0.70),
    ]
});

/// Extract the full structure layer from the turn text (design §D2.1).
/// `speakers` is the enrolled-speaker vocabulary — speaker spans are emitted only
/// when it is non-empty (the keyword tier never guesses names).
pub fn extract_structure(text: &str, speakers: &[String]) -> Structure {
    Structure {
        entities: extract_entities(text, speakers),
        shape: detect_shape(text),
        argument: None, // keyword tier is honest: predicate-argument needs the LLM
    }
}

fn extract_entities(text: &str, speakers: &[String]) -> Vec<EntitySpan> {
    let mut covered: Vec<(usize, usize)> = Vec::new();
    let mut out: Vec<EntitySpan> = Vec::new();
    let mut seen: std::collections::HashSet<(EntityKind, String)> = std::collections::HashSet::new();

    let mut push = |span: EntitySpan, s: usize, e: usize, covered: &mut Vec<(usize, usize)>| {
        if out.len() >= MAX_ENTITIES {
            return;
        }
        if covered.iter().any(|&(cs, ce)| s < ce && cs < e) {
            return; // overlaps an already-claimed range
        }
        let key = (span.kind, span.text.to_lowercase());
        if seen.insert(key) {
            covered.push((s, e));
            out.push(span);
        } else {
            covered.push((s, e)); // still claim the range so lower matchers skip it
        }
    };

    for matcher in MATCHERS.iter() {
        for m in matcher.re.find_iter(text) {
            let span = EntitySpan {
                kind: matcher.kind,
                text: m.as_str().to_string(),
                confidence: matcher.confidence,
            };
            push(span, m.start(), m.end(), &mut covered);
        }
    }

    // Speaker spans: only from the supplied enrolled vocabulary (never guessed).
    if !speakers.is_empty() {
        let lower = text.to_lowercase();
        for name in speakers {
            let needle = name.to_lowercase();
            if needle.is_empty() {
                continue;
            }
            if let Some(pos) = find_word(&lower, &needle) {
                push(
                    EntitySpan {
                        kind: EntityKind::Speaker,
                        text: name.clone(),
                        confidence: 0.90,
                    },
                    pos,
                    pos + needle.len(),
                    &mut covered,
                );
            }
        }
    }

    out
}

/// Find `needle` as a whole word in `haystack` (both lowercased), returning its
/// byte offset. Word boundary = non-alphanumeric (or string edge) on both sides.
fn find_word(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// Multi-part sequencing cues (coarse, conservative — bare "and" is excluded to
/// avoid false positives like "cats and dogs").
const MULTI_PART_CUES: &[&str] = &[
    " and then", " then ", "; ", " after that", " followed by", " next,", " next ",
];

/// Reference-to-prior phrases.
const REFERS_PRIOR_CUES: &[&str] = &[
    "that one",
    "like before",
    "as before",
    "the previous",
    "last time",
    "that thing",
    "you mentioned",
    "we discussed",
    "same as",
];

/// Conditional single-word markers (checked as whole tokens).
const CONDITIONAL_TOKENS: &[&str] = &["if", "when", "unless", "whenever"];

fn detect_shape(text: &str) -> UtteranceShape {
    let lower = text.to_lowercase();
    let sentence_terminators = lower.matches(['.', '!', '?']).count();

    let multi_part = MULTI_PART_CUES.iter().any(|c| lower.contains(c)) || sentence_terminators >= 2;

    let conditional = lower.contains("in case")
        || lower.contains("as long as")
        || lower.contains("provided that")
        || lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| CONDITIONAL_TOKENS.contains(&w));

    let refers_prior = REFERS_PRIOR_CUES.iter().any(|c| lower.contains(c));

    UtteranceShape {
        multi_part,
        conditional,
        refers_prior,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<EntityKind> {
        extract_structure(text, &[])
            .entities
            .iter()
            .map(|e| e.kind)
            .collect()
    }

    fn has_span(text: &str, kind: EntityKind, want: &str) -> bool {
        extract_structure(text, &[])
            .entities
            .iter()
            .any(|e| e.kind == kind && e.text == want)
    }

    #[test]
    fn extracts_paths() {
        assert!(has_span("edit crates/foo/bar.rs now", EntityKind::Path, "crates/foo/bar.rs"));
        assert!(has_span("open main.rs", EntityKind::Path, "main.rs"));
    }

    #[test]
    fn extracts_urls() {
        assert!(has_span(
            "see https://example.com/x for docs",
            EntityKind::Url,
            "https://example.com/x"
        ));
        // The URL claims its range → its digits are not a bare number.
        assert!(!kinds("go to https://a.b/123").contains(&EntityKind::Number));
    }

    #[test]
    fn extracts_numbers_dates_times_durations() {
        assert!(has_span("the port is 8090", EntityKind::Number, "8090"));
        assert!(has_span("ship by 2026-07-05 please", EntityKind::Date, "2026-07-05"));
        assert!(has_span("meet at 3:30pm", EntityKind::Time, "3:30pm"));
        assert!(has_span("wait 50ms then retry", EntityKind::Duration, "50ms"));
    }

    #[test]
    fn duration_digits_not_also_number() {
        // "50ms" is a duration; there should be no bare "50" number span.
        let ents = extract_structure("wait 50ms", &[]);
        assert!(ents.entities.iter().any(|e| e.kind == EntityKind::Duration));
        assert!(!ents.entities.iter().any(|e| e.kind == EntityKind::Number));
    }

    #[test]
    fn extracts_code_and_quotes() {
        assert!(has_span("call index_turn on commit", EntityKind::Code, "index_turn"));
        assert!(has_span("use clawft_kernel::CausalGraph", EntityKind::Code, "clawft_kernel::CausalGraph"));
        assert!(has_span(r#"set name to "voice""#, EntityKind::Quote, r#""voice""#));
    }

    #[test]
    fn speaker_only_with_vocab() {
        let vocab = vec!["alice".to_string(), "bob".to_string()];
        let s = extract_structure("ask alice about the plan", &vocab);
        assert!(s.entities.iter().any(|e| e.kind == EntityKind::Speaker && e.text == "alice"));
        // Without a vocab, no speaker is ever guessed.
        assert!(!extract_structure("ask alice about the plan", &[])
            .entities
            .iter()
            .any(|e| e.kind == EntityKind::Speaker));
    }

    #[test]
    fn utterance_shape_flags() {
        let s = extract_structure("fix the bug and then run the tests", &[]);
        assert!(s.shape.multi_part);
        // Live voice-session positive: sequenced request fires multi_part.
        assert!(extract_structure("pull the config then restart the daemon", &[]).shape.multi_part);
        let s = extract_structure("if the build fails, revert", &[]);
        assert!(s.shape.conditional);
        let s = extract_structure("use that one like before", &[]);
        assert!(s.shape.refers_prior);
        let s = extract_structure("just build it", &[]);
        assert!(!s.shape.multi_part && !s.shape.conditional && !s.shape.refers_prior);
    }

    #[test]
    fn argument_is_none_at_keyword_tier() {
        assert!(extract_structure("write a sort function", &[]).argument.is_none());
    }

    #[test]
    fn structure_from_value_roundtrips() {
        let s = extract_structure("open crates/x.rs at 3pm", &[]);
        let blob = serde_json::to_value(&s).unwrap();
        let back = Structure::from_value(&blob).unwrap();
        assert_eq!(back, s);
        // Absent shape/argument tolerated.
        let partial = serde_json::json!({ "entities": [] });
        let d = Structure::from_value(&partial).unwrap();
        assert_eq!(d, Structure::default());
    }
}
