//! Atom-level record verbalization for semantic embedding (WEFT-640).
//!
//! TabSTAR's useful idea (adopted, not licensed): before embedding a structured
//! node record, render it to a **stable semantic string** so the embedder sees
//! act / topic / emotion / goal / voice cues as natural language, not opaque
//! JSON field noise or raw floats.
//!
//! One function serves every atom type flowing through
//! [`SessionTier::index_turn`](crate::session_tier::SessionTier::index_turn):
//! turn text + optional classification-v2 blob + optional `VoiceAnalysis` JSON.
//! The string is what lands in the passage half of the e5 / Qwen3 space; the
//! free-text graft query stays raw and uses `embed_query`.
//!
//! # Shape (stable for golden tests)
//!
//! ```text
//! <turn text>
//! [act: question | topic: deploy | emotion: frustrated · high arousal (0.8) | goal: fix-oom]
//! [voice: frustrated · high arousal (0.8) · rising-pitch]
//! ```
//!
//! Classification / voice lines are omitted when their inputs are absent.
//! Scalar axes are **quantile-binned into words** (low / mid / high) so the
//! encoder can use them; raw f32s are never emitted alone.

use serde_json::Value;

use crate::turn_classifier::{ClassificationVector, Vad};

/// Render a turn atom to a stable semantic string for embedding.
///
/// - Always includes the turn text (trimmed).
/// - When `classification` is present (blob or already-parsed vector), appends
///   a compact act/topic/emotion/goal line.
/// - When `voice_analysis` JSON is present, appends a compact voice emotion +
///   prosody line (does not dump token arrays).
///
/// Empty text with no structured payload returns an empty string (caller should
/// skip indexing, matching `SessionTier::index_turn`).
pub fn verbalize_turn(
    text: &str,
    classification: Option<&Value>,
    voice_analysis: Option<&Value>,
) -> String {
    let body = text.trim();
    let mut parts: Vec<String> = Vec::new();
    if !body.is_empty() {
        parts.push(body.to_string());
    }

    if let Some(blob) = classification {
        if let Some(line) = verbalize_classification_value(blob) {
            parts.push(line);
        }
    }

    if let Some(va) = voice_analysis {
        if let Some(line) = verbalize_voice_analysis(va) {
            parts.push(line);
        }
    }

    parts.join("\n")
}

/// Verbalize a typed [`ClassificationVector`] (prefer this when the classifier
/// already ran on the anchor path).
pub fn verbalize_classification(cv: &ClassificationVector) -> String {
    let mut fields = Vec::new();

    // Prefer refined dialogue-act wire name; fall back to legacy intent label.
    let act_str = refined_act_label(cv);
    fields.push(format!("act: {act_str}"));

    if !cv.topic.is_empty() {
        fields.push(format!("topic: {}", cv.topic));
    }

    fields.push(format!("emotion: {}", verbalize_vad(&cv.emotion)));

    if let Some(goal) = cv.goal.as_deref().filter(|g| !g.is_empty()) {
        fields.push(format!("goal: {goal}"));
    }

    // Structure: surface a few entity kinds as compact tokens when present.
    let entity_kinds = entity_kind_summary(cv);
    if !entity_kinds.is_empty() {
        fields.push(format!("entities: {entity_kinds}"));
    }

    format!("[{}]", fields.join(" | "))
}

/// Verbalize a stored classification metadata blob (JSON).
pub fn verbalize_classification_value(value: &Value) -> Option<String> {
    ClassificationVector::from_metadata_value(value).map(|cv| verbalize_classification(&cv))
}

/// Verbalize a `VoiceAnalysis` JSON value (serde shape from clawft-channels).
///
/// Only the retrieval-relevant sections: emotion VAD + label, and a light
/// prosody cue when pitch trend / energy is present. Token arrays and capture
/// health are deliberately omitted (noise for semantic kNN).
pub fn verbalize_voice_analysis(va: &Value) -> Option<String> {
    let mut fields = Vec::new();

    if let Some(emo) = va.get("emotion") {
        let label = emo
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("neutral");
        let arousal = emo
            .get("arousal")
            .and_then(Value::as_f64)
            .unwrap_or(0.5) as f32;
        let valence = emo
            .get("valence")
            .and_then(Value::as_f64)
            .unwrap_or(0.0) as f32;
        let vad = Vad {
            valence,
            arousal,
            dominance: emo
                .get("dominance")
                .and_then(Value::as_f64)
                .unwrap_or(0.0) as f32,
            label: label.to_string(),
        };
        fields.push(verbalize_vad(&vad));
    }

    if let Some(prosody) = va.get("prosody") {
        if let Some(cue) = prosody_cue(prosody) {
            fields.push(cue);
        }
    }

    if fields.is_empty() {
        return None;
    }
    Some(format!("[voice: {}]", fields.join(" · ")))
}

// ── helpers ───────────────────────────────────────────────────────────────

fn verbalize_vad(vad: &Vad) -> String {
    let arousal_bin = bin_signed(vad.arousal);
    let valence_bin = bin_signed(vad.valence);
    // Prefer the human label; fall back to valence bin if label is empty.
    let label = if vad.label.is_empty() {
        valence_bin
    } else {
        vad.label.as_str()
    };
    format!(
        "{label} · {arousal_bin} arousal ({:.1})",
        vad.arousal
    )
}

/// Map a roughly `[-1, 1]` (or `[0, 1]` arousal-style) scalar to a word bin.
///
/// Bins: `low` (< −0.25 or < 0.33 for [0,1]-ish), `mid`, `high` (> 0.25 / 0.66).
/// Absolute value thresholds work for both signed VAD and [0,1] arousal.
fn bin_signed(x: f32) -> &'static str {
    if x < -0.25 {
        "low"
    } else if x > 0.25 && x < 0.66 {
        // mid-high for signed positive or mid for [0,1]
        if x <= 0.5 {
            "mid"
        } else {
            "high"
        }
    } else if x >= 0.66 {
        "high"
    } else if x >= -0.25 && x <= 0.25 {
        "mid"
    } else {
        "mid"
    }
}

fn refined_act_label(cv: &ClassificationVector) -> String {
    // serde wire name is kebab-case; Debug is PascalCase — map via from_wire inverse.
    use crate::dialogue_act::RefinedAct;
    match cv.act.refined {
        RefinedAct::Question => "question",
        RefinedAct::ClarificationRequest => "clarification-request",
        RefinedAct::ClarificationProvide => "clarification-provide",
        RefinedAct::Command => "command",
        RefinedAct::Comment => "comment",
        RefinedAct::Correction => "correction",
        RefinedAct::Feedback => "feedback",
        RefinedAct::Acknowledgment => "acknowledgment",
        RefinedAct::Social => "social",
        RefinedAct::Meta => "meta",
    }
    .to_string()
}

fn entity_kind_summary(cv: &ClassificationVector) -> String {
    use std::collections::BTreeSet;
    let mut kinds = BTreeSet::new();
    for span in &cv.structure.entities {
        kinds.insert(format!("{:?}", span.kind).to_lowercase());
    }
    kinds.into_iter().take(6).collect::<Vec<_>>().join(", ")
}

fn prosody_cue(prosody: &Value) -> Option<String> {
    // Best-effort: common field names from VoiceAnalysis ProsodyAnalysis.
    // Missing fields → no cue (honest).
    if let Some(trend) = prosody
        .get("pitch_trend")
        .or_else(|| prosody.get("f0_trend"))
        .and_then(|v| v.as_str())
    {
        return Some(format!("{trend}-pitch"));
    }
    if let Some(slope) = prosody
        .get("pitch_slope")
        .or_else(|| prosody.get("f0_slope"))
        .and_then(Value::as_f64)
    {
        if slope > 0.15 {
            return Some("rising-pitch".into());
        }
        if slope < -0.15 {
            return Some("falling-pitch".into());
        }
    }
    if let Some(energy) = prosody
        .get("energy_mean")
        .or_else(|| prosody.get("rms_mean"))
        .and_then(Value::as_f64)
    {
        if energy > 0.6 {
            return Some("high-energy".into());
        }
        if energy < 0.2 {
            return Some("low-energy".into());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialogue_act::{Act, RefinedAct};
    use crate::text_structure::Structure;
    use crate::turn_classifier::{Tier, TAXONOMY_VERSION};

    fn sample_cv() -> ClassificationVector {
        ClassificationVector {
            act: Act::new(RefinedAct::Question),
            topic: "deploy".into(),
            emotion: Vad {
                valence: -0.4,
                arousal: 0.8,
                dominance: 0.0,
                label: "frustrated".into(),
            },
            structure: Structure::default(),
            goal: Some("fix-oom".into()),
            tier: Tier::Keyword,
            v: TAXONOMY_VERSION,
        }
    }

    #[test]
    fn verbalize_text_only() {
        let s = verbalize_turn("hello world", None, None);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn verbalize_empty_is_empty() {
        assert!(verbalize_turn("  ", None, None).is_empty());
    }

    #[test]
    fn verbalize_classification_shape() {
        let line = verbalize_classification(&sample_cv());
        assert!(line.starts_with('['));
        assert!(line.contains("act:"));
        assert!(line.contains("topic: deploy"));
        assert!(line.contains("frustrated"));
        assert!(line.contains("arousal"));
        assert!(line.contains("goal: fix-oom"));
        // No raw JSON braces.
        assert!(!line.contains('{'));
    }

    #[test]
    fn verbalize_turn_joins_text_and_blob() {
        let blob = sample_cv().to_metadata_value();
        let s = verbalize_turn(
            "why did staging fall over again?",
            Some(&blob),
            None,
        );
        assert!(s.starts_with("why did staging fall over again?"));
        assert!(s.contains("topic: deploy"));
        assert!(s.contains("frustrated"));
    }

    #[test]
    fn verbalize_voice_emotion_bins() {
        let va = serde_json::json!({
            "emotion": {
                "label": "angry",
                "valence": -0.7,
                "arousal": 0.9,
                "dominance": 0.2
            },
            "prosody": { "pitch_slope": 0.4 }
        });
        let line = verbalize_voice_analysis(&va).expect("voice line");
        assert!(line.starts_with("[voice:"));
        assert!(line.contains("angry"));
        assert!(line.contains("rising-pitch"));
    }

    #[test]
    fn verbalize_voice_absent_sections_none() {
        let va = serde_json::json!({ "stt": { "model": "x" } });
        assert!(verbalize_voice_analysis(&va).is_none());
    }

    #[test]
    fn verbalize_is_stable_across_calls() {
        let blob = sample_cv().to_metadata_value();
        let a = verbalize_turn("same", Some(&blob), None);
        let b = verbalize_turn("same", Some(&blob), None);
        assert_eq!(a, b);
    }

    #[test]
    fn bin_signed_covers_low_mid_high() {
        assert_eq!(bin_signed(-0.8), "low");
        assert_eq!(bin_signed(0.0), "mid");
        assert_eq!(bin_signed(0.9), "high");
    }
}
