//! Agent-side `SOUL.journal` write path (WEFT-330 / agent-core-v1.1).
//!
//! ## Why this exists
//!
//! F1 seeded `.clawft/SOUL.journal.md` and stamped a substrate
//! `soul_journal` [`DerivedWriteGrant`](https://docs). F2's
//! `weaver soul promote` reads pending observations from the
//! mesh-canonical topic `substrate/_derived/soul_journal/<ulid>` and
//! merges them into `SOUL.md` on confirmation. Until WEFT-330 the
//! *agent* never wrote into that journal — promote always saw an
//! empty set.
//!
//! This module is the agent half:
//!
//! 1. A documented **drift detection signal** (heuristic + explicit
//!    override) that decides *whether* a turn warrants a journal row.
//! 2. A [`SoulJournal`] trait the loop calls after a successful turn
//!    when a signal fires.
//! 3. In-core writers for tests ([`InMemorySoulJournal`]) and the
//!    offline / CLI path ([`FileSoulJournal`] appends markdown to
//!    `.clawft/SOUL.journal.md`).
//! 4. The production substrate writer lives in
//!    `clawft-service-agent::soul_journal` and publishes under the
//!    grant-gated `_derived/soul_journal/` topic.
//! 5. **WEFT-96 / WS-D1 read path:** [`SoulJournalReader`] lists pending
//!    observations on every identity turn (via
//!    [`super::identity::JournalAwareIdentityProvider`]). In-core
//!    [`InMemorySoulJournal`] implements both halves; production
//!    substrate reader lives next to the writer in service-agent.
//!
//! ## Drift detection signal (documented)
//!
//! The loop does **not** ask the model "did I drift?" on every turn.
//! Detection is deliberate and cheap:
//!
//! | Signal | When it fires | Source |
//! |--------|---------------|--------|
//! | **UserCorrection** | User message matches a correction / preference cue *and* the assistant produced a non-empty reply | [`detect_drift_signal`] |
//! | **Explicit** | Inbound metadata key [`SOUL_JOURNAL_OBSERVE_META_KEY`] is set (string content) | operator / test / future model path |
//! | **Synthetic** | Reserved for unit tests that inject an observation without a turn | tests |
//!
//! Heuristic cues (case-insensitive; any one match is enough):
//! `don't` / `do not`, `stop doing`, `instead of`, `rather than`,
//! `i prefer` / `i'd rather` / `i would rather`, `prefer `,
//! `please be`, `too verbose`, `too terse`, `from now on`,
//! `going forward`, `please remember`, `you should`.
//!
//! **Not model-decided today.** A future path may parse a structured
//! `<!-- soul:observe ... -->` block from the assistant reply; that is
//! a separate ticket. The Explicit metadata key is the forward-compat
//! seam for that path (the model or a post-processor can set it).
//!
//! ## Grant gating
//!
//! File appends do not need a substrate grant (they are process-local
//! durability for CLI / offline). Substrate publishes **must** go
//! through `SubstrateService::publish_gated_with_grants` under
//! `substrate/_derived/soul_journal/<entry-id>` so the F1
//! `soul_journal` `DerivedWriteGrant` is enforced. Without the grant
//! the write is rejected and the loop logs + continues (degrade,
//! don't crash the user-visible turn).
//!
//! ## Sandbox note
//!
//! Agent *tools* cannot write `.clawft/SOUL.journal.md` — the sandbox
//! denies those filenames unconditionally. This module is a
//! privileged loop-side path, not a tool.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use tracing::{debug, warn};

/// Inbound metadata key that forces a journal observation.
///
/// Value must be a JSON string (the observation body). Summary is
/// derived from the first line. Used by synthetic tests and as the
/// forward-compat seam for a future model-decided path.
pub const SOUL_JOURNAL_OBSERVE_META_KEY: &str = "soul_journal_observe";

/// Mesh-canonical substrate prefix for soul-journal entries (trailing
/// slash omitted). Matches the F1 grant topic `soul_journal` under
/// `substrate/_derived/`.
pub const SOUL_JOURNAL_SUBSTRATE_PREFIX: &str = "substrate/_derived/soul_journal";

/// How a drift observation was decided. Surfaced in the journal body
/// so operators can audit signal quality when promoting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftSignal {
    /// User-message heuristic matched `matched` (the cue substring).
    UserCorrection {
        /// The cue string that fired (lowercased).
        matched: String,
    },
    /// Forced via [`SOUL_JOURNAL_OBSERVE_META_KEY`] (or equivalent).
    Explicit {
        /// Free-form source tag (`"metadata"`, `"model"`, …).
        source: String,
    },
    /// Test-only injection — never produced by production detection.
    Synthetic,
}

impl DriftSignal {
    /// Short label for journal rendering / substrate payload.
    pub fn label(&self) -> &'static str {
        match self {
            DriftSignal::UserCorrection { .. } => "user_correction",
            DriftSignal::Explicit { .. } => "explicit",
            DriftSignal::Synthetic => "synthetic",
        }
    }
}

/// One pending identity-drift observation.
///
/// Substrate wire shape (value side of
/// `substrate/_derived/soul_journal/<entry_id>`):
/// ```json
/// { "summary": "...", "content": "...", "ts": "...",
///   "conv_id": "...", "signal": "user_correction" }
/// ```
/// `weaver soul promote` requires `content`; the rest is best-effort
/// metadata (see `clawft-weave::commands::soul_cmd::JournalEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftObservation {
    /// Stable entry id (ULID preferred). Empty string means "writer
    /// should mint one".
    pub entry_id: String,
    /// One-line human-readable summary.
    pub summary: String,
    /// Full observation body (markdown-friendly plain text).
    pub content: String,
    /// ISO-8601 (UTC) wall-clock. Empty → writer stamps now.
    pub ts: String,
    /// Conversation id that produced the observation (may be empty).
    pub conv_id: String,
    /// How the observation was decided.
    pub signal: DriftSignal,
}

impl DriftObservation {
    /// Build a synthetic observation for tests.
    pub fn synthetic(summary: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            entry_id: String::new(),
            summary: summary.into(),
            content: content.into(),
            ts: String::new(),
            conv_id: String::new(),
            signal: DriftSignal::Synthetic,
        }
    }

    /// Ensure `ts` / `entry_id` are populated. Returns a clone with
    /// defaults filled in (does not mutate `self` so callers can keep
    /// the original for assertions).
    pub fn finalized(&self) -> Self {
        let mut out = self.clone();
        if out.ts.is_empty() {
            out.ts = Utc::now().to_rfc3339();
        }
        if out.entry_id.is_empty() {
            // Compact, filesystem/substrate-safe id without pulling
            // the `ulid` crate into clawft-core. Substrate writers may
            // replace this with a true ULID before publish.
            let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
            out.entry_id = format!("obs-{nanos:x}");
        }
        out
    }

    /// Markdown block matching `chat-agent-v1.md` §7.3 journal format.
    pub fn to_markdown(&self) -> String {
        let o = self.finalized();
        let signal_detail = match &o.signal {
            DriftSignal::UserCorrection { matched } => format!("user_correction ({matched})"),
            DriftSignal::Explicit { source } => format!("explicit ({source})"),
            DriftSignal::Synthetic => "synthetic".to_string(),
        };
        let conv = if o.conv_id.is_empty() {
            "unknown".to_string()
        } else {
            o.conv_id.clone()
        };
        format!(
            "## {} [conversation {conv}]\n\
             **Observation:** {}\n\
             **Suggested update:** {}\n\
             **Source:** {signal_detail}\n\
             **Entry-id:** {}\n\n",
            o.ts, o.summary, o.content, o.entry_id
        )
    }

    /// Substrate / promote JSON value.
    pub fn to_substrate_value(&self) -> serde_json::Value {
        let o = self.finalized();
        serde_json::json!({
            "summary": o.summary,
            "content": o.content,
            "ts": o.ts,
            "conv_id": o.conv_id,
            "signal": o.signal.label(),
        })
    }
}

/// Append-only journal seam used by [`super::loop_core::AgentLoop`].
///
/// Implementations:
/// - [`InMemorySoulJournal`] — tests.
/// - [`FileSoulJournal`] — offline / CLI durability
///   (`.clawft/SOUL.journal.md`).
/// - `SubstrateSoulJournal` (service-agent) — grant-gated substrate
///   publish under [`SOUL_JOURNAL_SUBSTRATE_PREFIX`].
///
/// Errors return `String` (same pattern as [`super::sink::ConversationSink`]);
/// the loop logs and continues so a journal failure never aborts chat.
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait SoulJournal: Send + Sync + 'static {
    /// Append one drift observation. Must be effectively append-only.
    async fn append(&self, observation: DriftObservation) -> Result<(), String>;
}

/// Pending journal observation as seen by the every-turn identity
/// path (WEFT-96 / WS-D1).
///
/// Decoded from substrate values under
/// [`SOUL_JOURNAL_SUBSTRATE_PREFIX`] (or in-memory / file mirrors).
/// Promote (`weaver soul promote`) still owns merging into `SOUL.md`;
/// identity loads only *consult* these rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingJournalEntry {
    /// Stable entry id (ULID on substrate; `obs-…` in-core fallback).
    pub entry_id: String,
    /// One-line human-readable summary.
    pub summary: String,
    /// Full observation body.
    pub content: String,
    /// ISO-8601 timestamp when known (empty otherwise).
    pub ts: String,
    /// Conversation id that produced the observation (may be empty).
    pub conv_id: String,
    /// Drift-signal label (`user_correction`, `explicit`, …).
    pub signal: String,
}

impl PendingJournalEntry {
    /// Decode a substrate JSON value (same shape as
    /// [`DriftObservation::to_substrate_value`]).
    ///
    /// Missing fields degrade to empty strings / content fallback so a
    /// partially-written row never fails the every-turn path.
    pub fn from_substrate_value(entry_id: impl Into<String>, value: &serde_json::Value) -> Self {
        let entry_id = entry_id.into();
        let content = value
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let summary = value
            .get("summary")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                let first = content.lines().next().unwrap_or("").trim();
                if first.len() > 80 {
                    format!("{}…", &first[..80])
                } else if first.is_empty() {
                    entry_id.clone()
                } else {
                    first.to_string()
                }
            });
        Self {
            entry_id,
            summary,
            content,
            ts: value
                .get("ts")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            conv_id: value
                .get("conv_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            signal: value
                .get("signal")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        }
    }

    /// Convert a finalized [`DriftObservation`] for the read path.
    pub fn from_observation(obs: &DriftObservation) -> Self {
        let o = obs.finalized();
        Self {
            entry_id: o.entry_id,
            summary: o.summary,
            content: o.content,
            ts: o.ts,
            conv_id: o.conv_id,
            signal: o.signal.label().to_string(),
        }
    }
}

/// Read half of the soul journal — consulted on every identity turn
/// (WEFT-96 / WS-D1).
///
/// Complements [`SoulJournal`] (write path / WEFT-330). Production
/// substrate impl lives in `clawft-service-agent`; in-core tests use
/// [`InMemorySoulJournal`].
///
/// Errors return `String` so [`super::identity::JournalAwareIdentityProvider`]
/// can map them onto [`super::identity::IdentityError`] variants
/// (WEFT-97 / WS-D5) under fail-closed mode.
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
pub trait SoulJournalReader: Send + Sync + 'static {
    /// List pending drift observations (mesh-canonical order preferred).
    ///
    /// Called once per turn by the journal-aware identity provider.
    /// Empty vec is success (no pending rows).
    async fn list_pending(&self) -> Result<Vec<PendingJournalEntry>, String>;
}

/// HashMap-backed [`SoulJournal`] for unit tests.
#[derive(Debug, Default)]
pub struct InMemorySoulJournal {
    entries: Mutex<Vec<DriftObservation>>,
}

impl InMemorySoulJournal {
    /// Empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of observations recorded so far (finalized copies).
    pub fn entries(&self) -> Vec<DriftObservation> {
        self.entries
            .lock()
            .map(|g| g.iter().map(|e| e.finalized()).collect())
            .unwrap_or_default()
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl SoulJournal for InMemorySoulJournal {
    async fn append(&self, observation: DriftObservation) -> Result<(), String> {
        let finalized = observation.finalized();
        self.entries
            .lock()
            .map_err(|e| format!("soul journal mutex poisoned: {e}"))?
            .push(finalized);
        Ok(())
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl SoulJournalReader for InMemorySoulJournal {
    async fn list_pending(&self) -> Result<Vec<PendingJournalEntry>, String> {
        Ok(self
            .entries()
            .into_iter()
            .map(|o| PendingJournalEntry::from_observation(&o))
            .collect())
    }
}

/// Appends markdown blocks to `<workspace>/.clawft/SOUL.journal.md`.
///
/// Creates the parent directory and a header if the file is missing.
/// This is the offline / CLI durability path; the production daemon
/// path prefers the substrate writer (grant-gated) and may dual-write
/// through a composite.
pub struct FileSoulJournal {
    journal_path: PathBuf,
}

impl FileSoulJournal {
    /// Rooted at a workspace directory (appends under `.clawft/`).
    pub fn for_workspace(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        Self {
            journal_path: workspace.join(".clawft").join("SOUL.journal.md"),
        }
    }

    /// Explicit path to the journal file.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            journal_path: path.into(),
        }
    }

    /// Path this writer targets.
    pub fn path(&self) -> &Path {
        &self.journal_path
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl SoulJournal for FileSoulJournal {
    async fn append(&self, observation: DriftObservation) -> Result<(), String> {
        let block = observation.to_markdown();
        if let Some(parent) = self.journal_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create journal dir {}: {e}", parent.display()))?;
        }
        if !self.journal_path.exists() {
            let header = "# SOUL.journal\n\n\
                Append-only journal of identity-drift observations. \
                Promote via `weaver soul promote`.\n\n";
            std::fs::write(&self.journal_path, header)
                .map_err(|e| format!("seed journal {}: {e}", self.journal_path.display()))?;
        }
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)
            .map_err(|e| format!("open journal {}: {e}", self.journal_path.display()))?;
        file.write_all(block.as_bytes())
            .map_err(|e| format!("append journal {}: {e}", self.journal_path.display()))?;
        debug!(
            path = %self.journal_path.display(),
            entry_id = %observation.finalized().entry_id,
            "soul journal: appended file entry"
        );
        Ok(())
    }
}

/// Composite writer: substrate (primary) + optional file mirror.
///
/// File failures are logged and do not fail the composite when the
/// primary write succeeded. If primary fails, the error is returned
/// (file is still attempted best-effort so offline durability is not
/// lost when substrate is briefly unavailable).
pub struct CompositeSoulJournal {
    primary: std::sync::Arc<dyn SoulJournal>,
    mirror: Option<std::sync::Arc<dyn SoulJournal>>,
}

impl CompositeSoulJournal {
    /// Primary writer only.
    pub fn new(primary: std::sync::Arc<dyn SoulJournal>) -> Self {
        Self {
            primary,
            mirror: None,
        }
    }

    /// Primary + best-effort file (or other) mirror.
    pub fn with_mirror(
        primary: std::sync::Arc<dyn SoulJournal>,
        mirror: std::sync::Arc<dyn SoulJournal>,
    ) -> Self {
        Self {
            primary,
            mirror: Some(mirror),
        }
    }
}

#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
impl SoulJournal for CompositeSoulJournal {
    async fn append(&self, observation: DriftObservation) -> Result<(), String> {
        // Finalize once so both writers share the same entry_id/ts.
        let observation = observation.finalized();
        let primary_result = self.primary.append(observation.clone()).await;
        if let Some(ref mirror) = self.mirror
            && let Err(e) = mirror.append(observation).await
        {
            warn!(error = %e, "soul journal: mirror append failed (non-fatal)");
        }
        primary_result
    }
}

// ── Drift detection ──────────────────────────────────────────────

/// Case-insensitive correction / preference cues. Order is
/// longest-first so multi-word phrases win over substrings when we
/// report the matched cue.
const DRIFT_CUES: &[&str] = &[
    "i would rather",
    "please remember",
    "going forward",
    "from now on",
    "stop doing",
    "rather than",
    "instead of",
    "i'd rather",
    "too verbose",
    "too terse",
    "please be",
    "you should",
    "i prefer",
    "do not",
    "prefer ",
    "don't",
];

/// Inspect a completed turn for a drift signal.
///
/// Returns `None` when neither the explicit metadata override nor the
/// user-correction heuristic fires. See module docs for the signal
/// table.
///
/// `user_content` is the inbound user message text.
/// `assistant_content` is the final assistant reply (empty → no
/// heuristic fire; explicit metadata still works).
/// `metadata` is the inbound message metadata map.
pub fn detect_drift_signal(
    user_content: &str,
    assistant_content: &str,
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> Option<(DriftSignal, String /* content body */)> {
    // Explicit override wins — forward-compat for model-decided path.
    if let Some(v) = metadata.get(SOUL_JOURNAL_OBSERVE_META_KEY) {
        if let Some(body) = v.as_str() {
            let body = body.trim();
            if !body.is_empty() {
                return Some((
                    DriftSignal::Explicit {
                        source: "metadata".into(),
                    },
                    body.to_string(),
                ));
            }
        } else if let Some(obj) = v.as_object() {
            // Allow { "content": "...", "summary": "..." } shape.
            if let Some(body) = obj.get("content").and_then(|c| c.as_str()) {
                let body = body.trim();
                if !body.is_empty() {
                    return Some((
                        DriftSignal::Explicit {
                            source: "metadata".into(),
                        },
                        body.to_string(),
                    ));
                }
            }
        }
    }

    if assistant_content.trim().is_empty() {
        return None;
    }

    let lower = user_content.to_ascii_lowercase();
    for cue in DRIFT_CUES {
        if lower.contains(cue) {
            return Some((
                DriftSignal::UserCorrection {
                    matched: (*cue).trim().to_string(),
                },
                format!(
                    "User correction cue `{cue}` in conversation. \
                     User said: {}\n\nAssistant replied: {}",
                    truncate(user_content, 400),
                    truncate(assistant_content, 400)
                ),
            ));
        }
    }
    None
}

/// Build a [`DriftObservation`] from a detected signal + turn context.
pub fn observation_from_signal(
    signal: DriftSignal,
    content: String,
    conv_id: &str,
) -> DriftObservation {
    let summary = summarize_content(&content);
    DriftObservation {
        entry_id: String::new(),
        summary,
        content,
        ts: String::new(),
        conv_id: conv_id.to_string(),
        signal,
    }
}

fn summarize_content(content: &str) -> String {
    let first = content.lines().next().unwrap_or("").trim();
    if first.len() > 80 {
        format!("{}…", &first[..80])
    } else if first.is_empty() {
        "identity drift observation".into()
    } else {
        first.to_string()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn heuristic_fires_on_prefer_cue() {
        let meta = HashMap::new();
        let hit = detect_drift_signal(
            "Please don't use bullet lists; I prefer narrative paragraphs.",
            "Got it — I'll stick to prose.",
            &meta,
        );
        assert!(hit.is_some());
        let (sig, body) = hit.unwrap();
        match sig {
            DriftSignal::UserCorrection { matched } => {
                assert!(
                    matched == "don't" || matched == "i prefer" || matched == "prefer",
                    "unexpected cue: {matched}"
                );
            }
            other => panic!("expected UserCorrection, got {other:?}"),
        }
        assert!(body.contains("User correction cue"));
    }

    #[test]
    fn heuristic_skips_when_assistant_empty() {
        let meta = HashMap::new();
        let hit = detect_drift_signal("I prefer short answers.", "", &meta);
        assert!(hit.is_none());
    }

    #[test]
    fn heuristic_skips_neutral_user_message() {
        let meta = HashMap::new();
        let hit = detect_drift_signal("What is the weather?", "Sunny and 72°F.", &meta);
        assert!(hit.is_none());
    }

    #[test]
    fn explicit_metadata_overrides_heuristic() {
        let mut meta = HashMap::new();
        meta.insert(
            SOUL_JOURNAL_OBSERVE_META_KEY.to_string(),
            serde_json::json!("noticed bias toward over-apologizing"),
        );
        let hit = detect_drift_signal("hello", "hi there", &meta).expect("explicit");
        assert_eq!(
            hit.0,
            DriftSignal::Explicit {
                source: "metadata".into()
            }
        );
        assert_eq!(hit.1, "noticed bias toward over-apologizing");
    }

    #[tokio::test]
    async fn in_memory_journal_records_entries() {
        let j = InMemorySoulJournal::new();
        j.append(DriftObservation::synthetic(
            "terse preference",
            "user wants shorter answers",
        ))
        .await
        .unwrap();
        let entries = j.entries();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].entry_id.is_empty());
        assert!(!entries[0].ts.is_empty());
        assert_eq!(entries[0].summary, "terse preference");
    }

    #[tokio::test]
    async fn in_memory_journal_reader_lists_pending() {
        let j = InMemorySoulJournal::new();
        j.append(DriftObservation::synthetic("a", "body-a"))
            .await
            .unwrap();
        j.append(DriftObservation::synthetic("b", "body-b"))
            .await
            .unwrap();
        let pending = SoulJournalReader::list_pending(&j).await.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].summary, "a");
        assert_eq!(pending[1].content, "body-b");
        assert_eq!(pending[0].signal, "synthetic");
    }

    #[test]
    fn pending_entry_from_substrate_value_round_trip() {
        let obs = DriftObservation::synthetic("sum", "full content body");
        let value = obs.to_substrate_value();
        let pending = PendingJournalEntry::from_substrate_value("01HZXTEST", &value);
        assert_eq!(pending.entry_id, "01HZXTEST");
        assert_eq!(pending.summary, "sum");
        assert_eq!(pending.content, "full content body");
        assert_eq!(pending.signal, "synthetic");
        assert!(!pending.ts.is_empty());
    }

    #[test]
    fn pending_entry_from_substrate_value_tolerates_sparse_payload() {
        let value = serde_json::json!({ "content": "only content present" });
        let pending = PendingJournalEntry::from_substrate_value("sparse-1", &value);
        assert_eq!(pending.content, "only content present");
        assert_eq!(pending.summary, "only content present");
        assert!(pending.ts.is_empty());
        assert!(pending.signal.is_empty());
    }

    #[tokio::test]
    async fn file_journal_appends_markdown() {
        let tmp = tempfile::tempdir().unwrap();
        let j = FileSoulJournal::for_workspace(tmp.path());
        let mut obs = DriftObservation::synthetic(
            "noticed bias toward verbose answers",
            "prefer narrative paragraphs over bullet lists",
        );
        obs.conv_id = "conv-test".into();
        j.append(obs).await.unwrap();

        let body = std::fs::read_to_string(j.path()).unwrap();
        assert!(body.contains("# SOUL.journal"));
        assert!(body.contains("noticed bias toward verbose answers"));
        assert!(body.contains("**Entry-id:**"));
        assert!(body.contains("conversation conv-test"));
    }

    #[tokio::test]
    async fn composite_writes_primary_and_mirror() {
        let primary = Arc::new(InMemorySoulJournal::new());
        let mirror = Arc::new(InMemorySoulJournal::new());
        let composite = CompositeSoulJournal::with_mirror(primary.clone(), mirror.clone());
        composite
            .append(DriftObservation::synthetic("s", "c"))
            .await
            .unwrap();
        assert_eq!(primary.entries().len(), 1);
        assert_eq!(mirror.entries().len(), 1);
        assert_eq!(
            primary.entries()[0].entry_id,
            mirror.entries()[0].entry_id,
            "composite must share finalized entry_id across writers"
        );
    }

    #[test]
    fn substrate_value_shape_matches_promote_decoder() {
        let mut obs = DriftObservation::synthetic("summary line", "full body content");
        obs.ts = "2026-04-27T12:00:00Z".into();
        let v = obs.to_substrate_value();
        assert_eq!(v["summary"], "summary line");
        assert_eq!(v["content"], "full body content");
        assert_eq!(v["ts"], "2026-04-27T12:00:00Z");
        assert_eq!(v["signal"], "synthetic");
    }
}
