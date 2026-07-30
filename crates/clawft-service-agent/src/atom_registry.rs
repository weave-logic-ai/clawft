//! Atom primary index — the Panopticon reverse map (ADR-069 / WEFT-641).
//!
//! Materialized at the [`KernelTurnAnchor`](crate::KernelTurnAnchor) seam
//! (the only moment `turn_id` / `chain_seq` / `uid` / `content_hash` are
//! co-present). Two O(1) forward maps (`by_seq`, `by_uid`) plus a durable
//! substrate sibling binding `chain_seq ↔ turn_id`.
//!
//! # Properties (hard requirements — ADR-069 D6)
//!
//! - **P1 non-participation**: never inserts into any projection; holds
//!   references only (no turn text / vectors).
//! - **P2 pruned timelines**: `locate` never fails because history moved on;
//!   disposition is updated, locators are never deleted.
//! - **P3 asymmetric / removable**: fire-and-forget mint; system is fully
//!   functional with the registry absent; `locate` / `audit` are read-only
//!   over projections.
//!
//! Slice order: `audit` first (catches live bugs like democritus
//! `chain_seq = 0`), then `record` + `locate` + RPC, then GUI (deferred).

use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::session_forest::turn_universal_id;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Lifecycle disposition of an atom — where it *ended up*, not whether it
/// survived onto the trunk (ADR-069 D3 / P2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    /// On the committed trunk.
    #[default]
    Committed,
    /// Superseded by a later write (e.g. voice Speculative → final).
    Superseded,
    /// Pruned from an L2 view / frontier reaper.
    Pruned,
    /// Abandoned COW / speculative branch (branch id on the locator).
    AbandonedBranch,
}

/// Coordinates of one atom across projections. References only — never
/// turn text or vectors (ADR-069 non-goal #1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomLocator {
    /// Global monotone witness sequence (`ChainManager::append` event.sequence).
    pub chain_seq: u64,
    /// Content-addressed identity (`UniversalNodeId` as 64-char hex).
    pub uid: String,
    /// Conversation namespace.
    pub conv_id: String,
    /// Substrate turn key → `_derived/chat/<conv_id>/turns/<turn_id>`.
    pub turn_id: String,
    /// Compact content hash (matches anchor / `ChunkMeta` integrity).
    pub content_hash: String,
    /// Turn role (`user` / `assistant` / `tool` / …).
    pub role: String,
    /// Event kind (`agent.chat.turn` / `agent.turn.record` / …).
    pub kind: String,
    /// Turn timestamp (ms).
    pub ts_ms: u64,
    /// Lifecycle disposition (P2).
    pub disposition: Disposition,
    /// Branch id when `disposition == AbandonedBranch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Causal graph numeric node id, if dual-written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causal_node: Option<u64>,
    /// SessionView sequence key (rebuildable from chain; ephemeral L2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_seq: Option<u64>,
    /// HNSW label (rebuildable; typically `chain_seq.to_string()`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hnsw_label: Option<String>,
}

/// Lookup key for [`AtomRegistry::locate`] — either durable key is O(1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomKey {
    /// By witness chain sequence.
    ByChainSeq(u64),
    /// By content-addressed uid (64-char hex of `UniversalNodeId`).
    ByUid(String),
}

/// One projection row fed into [`AtomRegistry::audit_projections`].
///
/// Used to catch the "looks alive but does not join back" class (e.g.
/// democritus HNSW rows with `chain_seq` hardcoded `0` — WEFT-642).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionAuditRow {
    /// Projection name (`ecc_brain_hnsw`, `democritus`, `session_view`, …).
    pub projection: String,
    /// Projection-local label / key (e.g. causal node id string).
    pub label: String,
    /// Declared join key; `None` or `Some(0)` when the projection cannot join.
    #[serde(default)]
    pub chain_seq: Option<u64>,
    /// Optional uid if the projection carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
}

/// Severity of a consistency finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Hard non-compliance (e.g. `chain_seq = 0` join key).
    Error,
    /// Soft drift (map disagreement, missing reverse key).
    Warning,
}

/// One finding from [`AtomRegistry::audit`] / [`AtomRegistry::audit_projections`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyIssue {
    /// Severity.
    pub severity: IssueSeverity,
    /// Machine-readable code.
    pub code: String,
    /// Human-readable description.
    pub message: String,
    /// Projection that produced the row, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection: Option<String>,
    /// Projection label, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// chain_seq involved, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_seq: Option<u64>,
}

/// Result of a consistency audit (ADR-069 D8 / acceptance #5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyReport {
    /// True iff no `Error`-severity findings.
    pub ok: bool,
    /// Locators examined in the internal map-agreement pass.
    pub locators_checked: usize,
    /// Projection rows examined.
    pub projection_rows_checked: usize,
    /// Findings (errors + warnings).
    pub issues: Vec<ConsistencyIssue>,
}

impl ConsistencyReport {
    fn finalize(mut self) -> Self {
        self.ok = !self
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Error);
        self
    }
}

// ---------------------------------------------------------------------------
// AtomRegistry
// ---------------------------------------------------------------------------

/// Reverse-resolvable atom index (ADR-069 D4).
///
/// Thread-safe via `DashMap`. Optional at every call site — absence is a
/// supported mode (P3 removability).
#[derive(Debug, Default)]
pub struct AtomRegistry {
    by_seq: DashMap<u64, AtomLocator>,
    by_uid: DashMap<String, AtomLocator>,
    /// Durable substrate sibling: `chain_seq → turn_id` binding so the
    /// substrate path becomes reverse-resolvable without a chain scan.
    sibling_seq_to_turn: DashMap<u64, String>,
    /// Reverse sibling: `turn_id → chain_seq` (O(1) substrate join).
    sibling_turn_to_seq: DashMap<String, u64>,
}

impl AtomRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Shared handle (daemon / tests).
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Number of recorded locators (`by_seq` cardinality).
    pub fn len(&self) -> usize {
        self.by_seq.len()
    }

    /// True when no locators have been recorded.
    pub fn is_empty(&self) -> bool {
        self.by_seq.is_empty()
    }

    /// Mint and record a locator in one shot (anchor seam helper).
    ///
    /// Fire-and-forget: never returns an error. `uid` is derived via
    /// [`turn_universal_id`](crate::session_forest::turn_universal_id).
    pub fn mint_and_record(
        &self,
        conv_id: &str,
        turn_id: &str,
        chain_seq: u64,
        role: &str,
        content: &str,
        content_hash: &str,
        kind: &str,
        ts_ms: u64,
    ) {
        let uid = turn_universal_id(conv_id, chain_seq, content);
        let loc = AtomLocator {
            chain_seq,
            uid: uid.to_string(),
            conv_id: conv_id.to_string(),
            turn_id: turn_id.to_string(),
            content_hash: content_hash.to_string(),
            role: role.to_string(),
            kind: kind.to_string(),
            ts_ms,
            disposition: Disposition::Committed,
            branch: None,
            causal_node: None,
            // Rebuildable ephemeral refs — coordinates only (D5).
            view_seq: Some(chain_seq),
            hnsw_label: Some(chain_seq.to_string()),
        };
        self.record(loc);
    }

    /// Insert / overwrite a locator (once, at anchor time). Updates both
    /// forward maps and the durable substrate sibling.
    ///
    /// Never deletes prior locators under a different key — lifecycle
    /// transitions call [`set_disposition`](Self::set_disposition) instead.
    pub fn record(&self, loc: AtomLocator) {
        self.sibling_seq_to_turn
            .insert(loc.chain_seq, loc.turn_id.clone());
        self.sibling_turn_to_seq
            .insert(loc.turn_id.clone(), loc.chain_seq);
        self.by_uid.insert(loc.uid.clone(), loc.clone());
        self.by_seq.insert(loc.chain_seq, loc);
    }

    /// O(1) reverse resolve. Never fails solely because disposition is
    /// `Pruned` / `Superseded` / `AbandonedBranch` (P2).
    pub fn locate(&self, key: AtomKey) -> Option<AtomLocator> {
        match key {
            AtomKey::ByChainSeq(seq) => self.by_seq.get(&seq).map(|e| e.clone()),
            AtomKey::ByUid(uid) => self.by_uid.get(&uid).map(|e| e.clone()),
        }
    }

    /// Durable substrate sibling: `chain_seq → turn_id`.
    pub fn turn_id_for_seq(&self, chain_seq: u64) -> Option<String> {
        self.sibling_seq_to_turn
            .get(&chain_seq)
            .map(|e| e.clone())
    }

    /// Durable substrate sibling: `turn_id → chain_seq`.
    pub fn chain_seq_for_turn(&self, turn_id: &str) -> Option<u64> {
        self.sibling_turn_to_seq.get(turn_id).map(|e| *e)
    }

    /// Update disposition without removing the locator (P2).
    ///
    /// Returns `true` if a locator was found and updated.
    pub fn set_disposition(&self, key: AtomKey, disposition: Disposition, branch: Option<String>) -> bool {
        let Some(mut loc) = self.locate(key) else {
            return false;
        };
        loc.disposition = disposition;
        loc.branch = branch;
        self.record(loc);
        true
    }

    /// Internal map-agreement audit (no projection samples).
    pub fn audit(&self) -> ConsistencyReport {
        self.audit_projections(&[])
    }

    /// Full consistency audit: map agreement + projection joinability.
    ///
    /// Flags the democritus / ECC-brain-HNSW `chain_seq = 0` class when
    /// such rows are supplied in `rows` (ADR-069 acceptance #5).
    pub fn audit_projections(&self, rows: &[ProjectionAuditRow]) -> ConsistencyReport {
        let mut report = ConsistencyReport {
            ok: true,
            locators_checked: 0,
            projection_rows_checked: rows.len(),
            issues: Vec::new(),
        };

        // 1. by_seq ↔ by_uid agreement + sibling consistency.
        for entry in self.by_seq.iter() {
            report.locators_checked += 1;
            let seq = *entry.key();
            let loc = entry.value();

            match self.by_uid.get(&loc.uid) {
                None => {
                    report.issues.push(ConsistencyIssue {
                        severity: IssueSeverity::Error,
                        code: "missing_uid_index".into(),
                        message: format!(
                            "chain_seq={seq} present in by_seq but uid={} missing from by_uid",
                            loc.uid
                        ),
                        projection: None,
                        label: None,
                        chain_seq: Some(seq),
                    });
                }
                Some(other) if other.chain_seq != loc.chain_seq || other.uid != loc.uid => {
                    report.issues.push(ConsistencyIssue {
                        severity: IssueSeverity::Error,
                        code: "locate_mismatch".into(),
                        message: format!(
                            "by_seq[{seq}] and by_uid[{}] disagree (seq {} vs {}, uid {} vs {})",
                            loc.uid, loc.chain_seq, other.chain_seq, loc.uid, other.uid
                        ),
                        projection: None,
                        label: None,
                        chain_seq: Some(seq),
                    });
                }
                Some(_) => {}
            }

            match self.sibling_seq_to_turn.get(&seq) {
                Some(turn) if turn.as_str() == loc.turn_id => {}
                Some(turn) => {
                    let sibling_turn = turn.as_str();
                    report.issues.push(ConsistencyIssue {
                        severity: IssueSeverity::Error,
                        code: "sibling_turn_mismatch".into(),
                        message: format!(
                            "sibling seq→turn for {seq} is '{sibling_turn}', locator has '{}'",
                            loc.turn_id
                        ),
                        projection: Some("substrate_sibling".into()),
                        label: Some(loc.turn_id.clone()),
                        chain_seq: Some(seq),
                    });
                }
                None => {
                    report.issues.push(ConsistencyIssue {
                        severity: IssueSeverity::Error,
                        code: "sibling_missing".into(),
                        message: format!("no substrate sibling binding for chain_seq={seq}"),
                        projection: Some("substrate_sibling".into()),
                        label: Some(loc.turn_id.clone()),
                        chain_seq: Some(seq),
                    });
                }
            }
        }

        // 2. Projection joinability (the democritus chain_seq=0 class).
        for row in rows {
            let bad_zero = row.chain_seq == Some(0);
            let missing = row.chain_seq.is_none();
            if bad_zero || missing {
                report.issues.push(ConsistencyIssue {
                    severity: IssueSeverity::Error,
                    code: if bad_zero {
                        "chain_seq_zero".into()
                    } else {
                        "chain_seq_missing".into()
                    },
                    message: format!(
                        "projection '{}' label='{}' cannot join back to the atom spine \
                         (chain_seq={:?}) — non-compliant with ADR-069 D2",
                        row.projection, row.label, row.chain_seq
                    ),
                    projection: Some(row.projection.clone()),
                    label: Some(row.label.clone()),
                    chain_seq: row.chain_seq,
                });
                continue;
            }

            // Optional: if we have locators, warn when a projection claims a
            // chain_seq the registry does not know (rebuild lag is ok → warn).
            if let Some(seq) = row.chain_seq
                && !self.by_seq.is_empty()
                && !self.by_seq.contains_key(&seq)
            {
                report.issues.push(ConsistencyIssue {
                    severity: IssueSeverity::Warning,
                    code: "unregistered_chain_seq".into(),
                    message: format!(
                        "projection '{}' label='{}' references chain_seq={seq} \
                         not present in the atom registry (rebuild lag or mint miss)",
                        row.projection, row.label
                    ),
                    projection: Some(row.projection.clone()),
                    label: Some(row.label.clone()),
                    chain_seq: Some(seq),
                });
            }
        }

        report.finalize()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_locator(seq: u64, turn: &str, uid_suffix: &str) -> AtomLocator {
        AtomLocator {
            chain_seq: seq,
            // Deterministic fake uid for pure registry tests (not blake3).
            uid: format!("{:0>64}", uid_suffix),
            conv_id: "c1".into(),
            turn_id: turn.into(),
            content_hash: "deadbeef".into(),
            role: "user".into(),
            kind: "agent.chat.turn".into(),
            ts_ms: 1_000 + seq,
            disposition: Disposition::Committed,
            branch: None,
            causal_node: Some(seq),
            view_seq: Some(seq),
            hnsw_label: Some(seq.to_string()),
        }
    }

    #[test]
    fn locate_by_seq_and_uid_agree_o1() {
        let reg = AtomRegistry::new();
        let loc = sample_locator(42, "t-42", "aa");
        let uid = loc.uid.clone();
        reg.record(loc.clone());

        let a = reg.locate(AtomKey::ByChainSeq(42)).expect("by seq");
        let b = reg.locate(AtomKey::ByUid(uid)).expect("by uid");
        assert_eq!(a, b);
        assert_eq!(a.chain_seq, 42);
        assert_eq!(a.turn_id, "t-42");
        assert_eq!(reg.turn_id_for_seq(42).as_deref(), Some("t-42"));
        assert_eq!(reg.chain_seq_for_turn("t-42"), Some(42));
    }

    #[test]
    fn pruned_disposition_still_locatable() {
        let reg = AtomRegistry::new();
        let loc = sample_locator(7, "t-7", "bb");
        reg.record(loc);
        assert!(reg.set_disposition(
            AtomKey::ByChainSeq(7),
            Disposition::Pruned,
            None
        ));
        let got = reg.locate(AtomKey::ByChainSeq(7)).expect("still there");
        assert_eq!(got.disposition, Disposition::Pruned);
        // uid path also sees the update
        let by_uid = reg
            .locate(AtomKey::ByUid(got.uid.clone()))
            .expect("by uid");
        assert_eq!(by_uid.disposition, Disposition::Pruned);
    }

    #[test]
    fn audit_flags_chain_seq_zero_projection_row() {
        let reg = AtomRegistry::new();
        // Healthy locator so the internal pass is green.
        reg.record(sample_locator(1, "t-1", "cc"));

        // democritus-style row: HNSW keyed by causal node, chain_seq hardcoded 0.
        let rows = [ProjectionAuditRow {
            projection: "ecc_brain_hnsw".into(),
            label: "293".into(),
            chain_seq: Some(0),
            uid: None,
        }];
        let report = reg.audit_projections(&rows);
        assert!(!report.ok, "chain_seq=0 must fail audit: {report:?}");
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "chain_seq_zero" && i.severity == IssueSeverity::Error),
            "expected chain_seq_zero error: {report:?}"
        );
        assert_eq!(report.locators_checked, 1);
        assert_eq!(report.projection_rows_checked, 1);
    }

    #[test]
    fn audit_clean_when_maps_agree_and_projections_joinable() {
        let reg = AtomRegistry::new();
        reg.record(sample_locator(3, "t-3", "dd"));
        let rows = [ProjectionAuditRow {
            projection: "session_view".into(),
            label: "3".into(),
            chain_seq: Some(3),
            uid: None,
        }];
        let report = reg.audit_projections(&rows);
        assert!(report.ok, "expected clean audit: {report:?}");
        assert!(report.issues.is_empty());
    }

    #[test]
    fn mint_and_record_derives_stable_uid() {
        let reg = AtomRegistry::new();
        reg.mint_and_record(
            "conv-a",
            "turn-ulid",
            99,
            "assistant",
            "hello world",
            "hash16charsxxxx",
            "agent.chat.turn",
            1_700_000_000_000,
        );
        let by_seq = reg.locate(AtomKey::ByChainSeq(99)).expect("minted");
        let by_uid = reg
            .locate(AtomKey::ByUid(by_seq.uid.clone()))
            .expect("uid path");
        assert_eq!(by_seq, by_uid);
        assert_eq!(by_seq.conv_id, "conv-a");
        assert_eq!(by_seq.turn_id, "turn-ulid");
        assert_eq!(by_seq.disposition, Disposition::Committed);
        // content-addressed: same inputs → same uid
        let uid2 = turn_universal_id("conv-a", 99, "hello world").to_string();
        assert_eq!(by_seq.uid, uid2);
        assert_eq!(reg.turn_id_for_seq(99).as_deref(), Some("turn-ulid"));
    }

    #[test]
    fn locate_unknown_returns_none() {
        let reg = AtomRegistry::new();
        assert!(reg.locate(AtomKey::ByChainSeq(1)).is_none());
        assert!(reg.locate(AtomKey::ByUid("00".repeat(32))).is_none());
    }
}
