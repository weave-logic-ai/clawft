//! Real-time cross-project assessment mesh transport (K6.6 / SOP 3).
//!
//! Bridges the [`MeshCoordinator`](crate::assessment::mesh::MeshCoordinator)
//! with the mesh networking stack so that assessment summaries, gossip,
//! and full reports are exchanged over the wire in real-time rather than
//! through artifact files on disk.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐        FrameType::AssessmentSync        ┌─────────────────┐
//! │  Node A          │  ────────────────────────────────────>  │  Node B          │
//! │  AssessmentSvc   │                                         │  AssessmentSvc   │
//! │  MeshCoordinator │  <────────────────────────────────────  │  MeshCoordinator │
//! └─────────────────┘        gossip / broadcast / report       └─────────────────┘
//! ```
//!
//! Messages are serialized as JSON inside [`MeshFrame`] payloads with
//! `FrameType::AssessmentSync`. The [`AssessmentTransport`] holds a
//! reference to both the [`MeshCoordinator`] and the [`MeshRuntime`]'s
//! peer connection map, providing:
//!
//! - **`broadcast_to_peers`** -- push a gossip or `ReportAvailable`
//!   message to all connected peers.
//! - **`handle_incoming`** -- decode an incoming assessment frame and
//!   feed it into the coordinator's `handle_message`.
//! - **`request_report`** -- ask a specific peer for its latest full
//!   assessment report.
//! - **`gossip_tick`** -- periodic gossip loop driver (called by the
//!   mesh heartbeat tick or a dedicated timer).

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::assessment::{AssessmentDiff, AssessmentReport};
use crate::assessment::mesh::{AssessmentMessage, MeshCoordinator, PeerAssessmentState};
use crate::error::{KernelError, KernelResult};
use crate::mesh_framing::{FrameType, MeshFrame};

// ── Wire envelope ────────────────────────────────────────────────

/// Thin envelope wrapping [`AssessmentMessage`] with routing metadata.
///
/// This is the JSON payload inside a `FrameType::AssessmentSync` frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentEnvelope {
    /// Source node identifier.
    pub source_node: String,
    /// Monotonic sequence number for dedup.
    pub sequence: u64,
    /// The inner protocol message.
    pub message: AssessmentMessage,
}

impl AssessmentEnvelope {
    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> KernelResult<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| KernelError::Mesh(format!("assessment envelope serialize: {e}")))
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> KernelResult<Self> {
        serde_json::from_slice(data)
            .map_err(|e| KernelError::Mesh(format!("assessment envelope deserialize: {e}")))
    }

    /// Wrap this envelope in a [`MeshFrame`] for wire transmission.
    pub fn to_frame(&self) -> KernelResult<MeshFrame> {
        let payload = self.to_bytes()?;
        Ok(MeshFrame {
            frame_type: FrameType::AssessmentSync,
            payload,
        })
    }

    /// Decode an [`AssessmentEnvelope`] from a [`MeshFrame`].
    ///
    /// Returns `Err` if the frame type is not `AssessmentSync` or the
    /// payload cannot be deserialized.
    pub fn from_frame(frame: &MeshFrame) -> KernelResult<Self> {
        if frame.frame_type != FrameType::AssessmentSync {
            return Err(KernelError::Mesh(format!(
                "expected AssessmentSync frame, got {:?}",
                frame.frame_type
            )));
        }
        Self::from_bytes(&frame.payload)
    }
}

// ── Transport ────────────────────────────────────────────────────

/// Real-time assessment mesh transport.
///
/// Connects the assessment layer's [`MeshCoordinator`] with the K6
/// mesh networking stack. Holds an `Arc` reference to the coordinator
/// so the same coordinator is shared with [`AssessmentService`].
///
/// The transport does not own any network connections itself. Instead
/// it produces serialized [`MeshFrame`] bytes and accepts incoming
/// bytes, delegating actual send/recv to the caller (typically the
/// [`MeshRuntime`] or a dedicated assessment sync loop).
pub struct AssessmentTransport {
    /// Shared assessment mesh coordinator.
    coordinator: Arc<MeshCoordinator>,
    /// Next outbound sequence number (monotonic).
    next_seq: std::sync::atomic::AtomicU64,
    /// Latest local report (for answering `RequestReport` and gossip ticks).
    latest_report: Mutex<Option<AssessmentReport>>,
}

impl AssessmentTransport {
    /// Create a new transport wrapping the given coordinator.
    pub fn new(coordinator: Arc<MeshCoordinator>) -> Self {
        Self {
            coordinator,
            next_seq: std::sync::atomic::AtomicU64::new(1),
            latest_report: Mutex::new(None),
        }
    }

    /// Return a reference to the inner coordinator.
    pub fn coordinator(&self) -> &MeshCoordinator {
        &self.coordinator
    }

    /// Shared coordinator handle (for status RPC / service wiring).
    pub fn coordinator_arc(&self) -> Arc<MeshCoordinator> {
        Arc::clone(&self.coordinator)
    }

    /// Cache the latest local report (enables RequestReport responses + ticks).
    pub fn publish_report(&self, report: AssessmentReport) {
        *self.latest_report.lock().unwrap() = Some(report);
    }

    /// Snapshot of the cached latest report.
    pub fn latest_report(&self) -> Option<AssessmentReport> {
        self.latest_report.lock().unwrap().clone()
    }

    // ── Outbound ─────────────────────────────────────────────────

    /// Build a gossip frame from the latest assessment report.
    ///
    /// Returns `None` if there is nothing to gossip (no report).
    pub fn build_gossip_frame(&self, report: &AssessmentReport) -> KernelResult<Vec<u8>> {
        let msg = self.coordinator.build_gossip(report);
        self.wrap_and_encode(msg)
    }

    /// Build a `ReportAvailable` broadcast frame.
    pub fn build_broadcast_frame(&self, report: &AssessmentReport) -> KernelResult<Vec<u8>> {
        let msg = self.coordinator.build_broadcast(report);
        self.wrap_and_encode(msg)
    }

    /// Build a `FindingDiff` frame that carries only changed findings.
    pub fn build_diff_frame(
        &self,
        report: &AssessmentReport,
        diff: &AssessmentDiff,
    ) -> KernelResult<Vec<u8>> {
        let msg = self.coordinator.build_finding_diff(report, diff);
        self.wrap_and_encode(msg)
    }

    /// Build a `RequestReport` frame addressed to a peer.
    pub fn build_request_frame(&self) -> KernelResult<Vec<u8>> {
        let msg = AssessmentMessage::RequestReport {
            requesting_node: self.coordinator.node_id().to_string(),
        };
        self.wrap_and_encode(msg)
    }

    /// Build a `FullReport` response frame.
    pub fn build_full_report_frame(&self, report: &AssessmentReport) -> KernelResult<Vec<u8>> {
        let msg = AssessmentMessage::FullReport {
            report: report.clone(),
        };
        self.wrap_and_encode(msg)
    }

    /// Drain the pending broadcast (if any) from the coordinator and
    /// return it as encoded frame bytes ready for the wire.
    pub fn drain_pending(&self) -> Option<Vec<u8>> {
        let msg = self.coordinator.take_pending_broadcast()?;
        match self.wrap_and_encode(msg) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                warn!(error = %e, "failed to encode pending assessment broadcast");
                None
            }
        }
    }

    // ── Inbound ──────────────────────────────────────────────────

    /// Try to decode an `AssessmentSync` payload from raw peer bytes.
    ///
    /// Accepts either a full [`MeshFrame::encode`] buffer
    /// (`[4-byte len][type][payload]`) or a type-prefixed buffer
    /// (`[type][payload]` as produced after transport length stripping).
    /// Returns `Ok(None)` when the bytes are not an AssessmentSync frame
    /// (caller should fall through to IPC envelope handling).
    pub fn try_extract_payload(data: &[u8]) -> KernelResult<Option<Vec<u8>>> {
        if data.is_empty() {
            return Ok(None);
        }
        // JSON MeshIpcEnvelope starts with '{' — not a framed message.
        if data[0] == b'{' {
            return Ok(None);
        }
        // Full encode: [4-len][type][payload]
        if data.len() >= 5 {
            let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if len + 4 == data.len() && data[4] == FrameType::AssessmentSync as u8 {
                let frame = MeshFrame::decode(&data[4..])
                    .map_err(|e| KernelError::Mesh(format!("assessment frame decode: {e}")))?;
                return Ok(Some(frame.payload));
            }
        }
        // Type-prefixed only: [type][payload]
        if data[0] == FrameType::AssessmentSync as u8 {
            let frame = MeshFrame::decode(data)
                .map_err(|e| KernelError::Mesh(format!("assessment frame decode: {e}")))?;
            return Ok(Some(frame.payload));
        }
        Ok(None)
    }

    /// Handle raw peer bytes if they carry AssessmentSync; otherwise
    /// return `Ok(None)` so the mesh runtime can treat them as IPC.
    ///
    /// On AssessmentSync, returns `Ok(Some(response_bytes))` when a
    /// reply should be sent, or `Ok(Some(empty))` is not used — `Ok(Some(None))`
    /// isn't valid. Convention: `Ok(Some(opt_reply))` where outer Some
    /// means "this was assessment traffic".
    pub fn try_handle_raw(&self, data: &[u8]) -> KernelResult<Option<Option<Vec<u8>>>> {
        match Self::try_extract_payload(data)? {
            Some(payload) => {
                let reply = self.handle_incoming(&payload)?;
                Ok(Some(reply))
            }
            None => Ok(None),
        }
    }

    /// Handle an incoming `AssessmentSync` frame payload.
    ///
    /// Decodes the envelope, feeds the inner message to the coordinator,
    /// and returns an optional response message (encoded as frame bytes)
    /// that should be sent back to the source peer. `RequestReport` is
    /// answered with a `FullReport` when a local report has been published.
    pub fn handle_incoming(&self, frame_payload: &[u8]) -> KernelResult<Option<Vec<u8>>> {
        let envelope = AssessmentEnvelope::from_bytes(frame_payload)?;
        debug!(
            from = %envelope.source_node,
            seq = envelope.sequence,
            "received assessment sync message"
        );

        // Answer RequestReport from the published local report before
        // coordinator handling (which returns None for this variant).
        if matches!(
            &envelope.message,
            AssessmentMessage::RequestReport { .. }
        ) {
            if let Some(report) = self.latest_report() {
                return Ok(Some(self.build_full_report_frame(&report)?));
            }
            return Ok(None);
        }

        let response = self.coordinator.handle_message(envelope.message);
        match response {
            Some(resp_msg) => {
                let bytes = self.wrap_and_encode(resp_msg)?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }

    // ── Gossip tick ──────────────────────────────────────────────

    /// Drive one gossip tick.
    ///
    /// If the assessment service has a latest report, this builds a
    /// gossip message and returns it as encoded frame bytes for
    /// broadcast to all peers. The caller is responsible for sending
    /// the bytes to each connected peer.
    ///
    /// Returns `None` if there is no report to gossip about.
    pub fn gossip_tick(&self, latest_report: Option<&AssessmentReport>) -> Option<Vec<u8>> {
        let report = latest_report?;
        match self.build_gossip_frame(report) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                warn!(error = %e, "failed to build gossip frame");
                None
            }
        }
    }

    // ── Peer state access ────────────────────────────────────────

    /// Return a snapshot of all known peer assessment states.
    pub fn peer_states(&self) -> Vec<PeerAssessmentState> {
        self.coordinator.peer_states()
    }

    // ── Internal ─────────────────────────────────────────────────

    fn next_sequence(&self) -> u64 {
        self.next_seq
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn wrap_and_encode(&self, message: AssessmentMessage) -> KernelResult<Vec<u8>> {
        let envelope = AssessmentEnvelope {
            source_node: self.coordinator.node_id().to_string(),
            sequence: self.next_sequence(),
            message,
        };
        let frame = envelope.to_frame()?;
        frame
            .encode()
            .map_err(|e| KernelError::Mesh(format!("frame encode: {e}")))
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assessment::{AssessmentReport, AssessmentSummary};
    use chrono::Utc;

    fn make_coordinator(node_id: &str, project: &str) -> Arc<MeshCoordinator> {
        Arc::new(MeshCoordinator::new(node_id.into(), project.into()))
    }

    fn make_report() -> AssessmentReport {
        AssessmentReport {
            timestamp: Utc::now(),
            scope: "full".into(),
            project: "/tmp/test-project".into(),
            files_scanned: 100,
            summary: AssessmentSummary {
                total_files: 100,
                coherence_score: 0.92,
                rust_files: 60,
                lines_of_code: 15_000,
                ..Default::default()
            },
            findings: vec![],
            analyzers_run: vec!["complexity".into(), "security".into(), "topology".into()],
        }
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let env = AssessmentEnvelope {
            source_node: "node-a".into(),
            sequence: 42,
            message: AssessmentMessage::Gossip {
                node_id: "node-a".into(),
                project_name: "proj-a".into(),
                last_assessment: Some("2026-04-04T00:00:00Z".into()),
                finding_count: 5,
                analyzer_count: 3,
            },
        };
        let bytes = env.to_bytes().unwrap();
        let restored = AssessmentEnvelope::from_bytes(&bytes).unwrap();
        assert_eq!(restored.source_node, "node-a");
        assert_eq!(restored.sequence, 42);
    }

    #[test]
    fn envelope_to_frame_uses_correct_type() {
        let env = AssessmentEnvelope {
            source_node: "n".into(),
            sequence: 1,
            message: AssessmentMessage::RequestReport {
                requesting_node: "n".into(),
            },
        };
        let frame = env.to_frame().unwrap();
        assert_eq!(frame.frame_type, FrameType::AssessmentSync);
    }

    #[test]
    fn envelope_from_frame_rejects_wrong_type() {
        let frame = MeshFrame {
            frame_type: FrameType::Heartbeat,
            payload: vec![],
        };
        let err = AssessmentEnvelope::from_frame(&frame).unwrap_err();
        assert!(err.to_string().contains("expected AssessmentSync"));
    }

    #[test]
    fn build_gossip_frame_roundtrip() {
        let coord = make_coordinator("node-a", "proj-a");
        let transport = AssessmentTransport::new(coord.clone());
        let report = make_report();

        let bytes = transport.build_gossip_frame(&report).unwrap();
        // Decode: first 4 bytes are length prefix, then type + payload.
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let frame = MeshFrame::decode(&bytes[4..4 + len]).unwrap();
        assert_eq!(frame.frame_type, FrameType::AssessmentSync);

        let envelope = AssessmentEnvelope::from_bytes(&frame.payload).unwrap();
        assert_eq!(envelope.source_node, "node-a");
        match &envelope.message {
            AssessmentMessage::Gossip {
                node_id,
                project_name,
                analyzer_count,
                ..
            } => {
                assert_eq!(node_id, "node-a");
                assert_eq!(project_name, "proj-a");
                assert_eq!(*analyzer_count, 3);
            }
            other => panic!("expected Gossip, got: {other:?}"),
        }
    }

    #[test]
    fn build_broadcast_frame_roundtrip() {
        let coord = make_coordinator("node-b", "proj-b");
        let transport = AssessmentTransport::new(coord);
        let report = make_report();

        let bytes = transport.build_broadcast_frame(&report).unwrap();
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let frame = MeshFrame::decode(&bytes[4..4 + len]).unwrap();
        let envelope = AssessmentEnvelope::from_bytes(&frame.payload).unwrap();

        match &envelope.message {
            AssessmentMessage::ReportAvailable {
                node_id,
                files_scanned,
                coherence_score,
                ..
            } => {
                assert_eq!(node_id, "node-b");
                assert_eq!(*files_scanned, 100);
                assert!(*coherence_score > 0.9);
            }
            other => panic!("expected ReportAvailable, got: {other:?}"),
        }
    }

    #[test]
    fn build_request_frame_roundtrip() {
        let coord = make_coordinator("requester", "proj");
        let transport = AssessmentTransport::new(coord);

        let bytes = transport.build_request_frame().unwrap();
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let frame = MeshFrame::decode(&bytes[4..4 + len]).unwrap();
        let envelope = AssessmentEnvelope::from_bytes(&frame.payload).unwrap();

        match &envelope.message {
            AssessmentMessage::RequestReport { requesting_node } => {
                assert_eq!(requesting_node, "requester");
            }
            other => panic!("expected RequestReport, got: {other:?}"),
        }
    }

    #[test]
    fn build_full_report_frame_roundtrip() {
        let coord = make_coordinator("responder", "proj");
        let transport = AssessmentTransport::new(coord);
        let report = make_report();

        let bytes = transport.build_full_report_frame(&report).unwrap();
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let frame = MeshFrame::decode(&bytes[4..4 + len]).unwrap();
        let envelope = AssessmentEnvelope::from_bytes(&frame.payload).unwrap();

        match &envelope.message {
            AssessmentMessage::FullReport { report: r } => {
                assert_eq!(r.files_scanned, 100);
                assert_eq!(r.analyzers_run.len(), 3);
            }
            other => panic!("expected FullReport, got: {other:?}"),
        }
    }

    #[test]
    fn handle_incoming_gossip_updates_coordinator() {
        let coord_a = make_coordinator("node-a", "proj-a");
        let transport_a = AssessmentTransport::new(coord_a.clone());
        let report = make_report();

        // Build gossip bytes on node A.
        let gossip_bytes = transport_a.build_gossip_frame(&report).unwrap();

        // Decode the frame payload (skip the 4-byte length + 1-byte type).
        let len = u32::from_be_bytes([
            gossip_bytes[0],
            gossip_bytes[1],
            gossip_bytes[2],
            gossip_bytes[3],
        ]) as usize;
        let frame = MeshFrame::decode(&gossip_bytes[4..4 + len]).unwrap();

        // Feed it into node B's transport.
        let coord_b = make_coordinator("node-b", "proj-b");
        let transport_b = AssessmentTransport::new(coord_b.clone());
        let response = transport_b.handle_incoming(&frame.payload).unwrap();
        assert!(response.is_none(), "gossip should not produce a response");

        // Coordinator B should now know about node A.
        let peers = coord_b.peer_states();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "node-a");
        assert_eq!(peers[0].project_name, "proj-a");
        assert_eq!(peers[0].analyzer_count, 3);
    }

    #[test]
    fn handle_incoming_broadcast_updates_coordinator() {
        let coord_a = make_coordinator("node-a", "proj-a");
        let transport_a = AssessmentTransport::new(coord_a);
        let report = make_report();

        let broadcast_bytes = transport_a.build_broadcast_frame(&report).unwrap();
        let len = u32::from_be_bytes([
            broadcast_bytes[0],
            broadcast_bytes[1],
            broadcast_bytes[2],
            broadcast_bytes[3],
        ]) as usize;
        let frame = MeshFrame::decode(&broadcast_bytes[4..4 + len]).unwrap();

        let coord_b = make_coordinator("node-b", "proj-b");
        let transport_b = AssessmentTransport::new(coord_b.clone());
        transport_b.handle_incoming(&frame.payload).unwrap();

        let peers = coord_b.peer_states();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].finding_count, 0); // report has 0 findings
    }

    #[test]
    fn sequence_numbers_increment() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);

        let s1 = transport.next_sequence();
        let s2 = transport.next_sequence();
        let s3 = transport.next_sequence();

        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(s3, 3);
    }

    #[test]
    fn drain_pending_returns_none_when_empty() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);
        assert!(transport.drain_pending().is_none());
    }

    #[test]
    fn drain_pending_returns_bytes_when_set() {
        let coord = make_coordinator("n", "p");
        let report = make_report();
        let gossip = coord.build_gossip(&report);
        coord.set_pending_broadcast(gossip);

        let transport = AssessmentTransport::new(coord);
        let bytes = transport.drain_pending();
        assert!(bytes.is_some());
        // Second drain should be empty.
        assert!(transport.drain_pending().is_none());
    }

    #[test]
    fn gossip_tick_with_report_produces_frame() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);
        let report = make_report();

        let bytes = transport.gossip_tick(Some(&report));
        assert!(bytes.is_some());
    }

    #[test]
    fn gossip_tick_without_report_returns_none() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);
        assert!(transport.gossip_tick(None).is_none());
    }

    #[test]
    fn peer_states_empty_initially() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);
        assert!(transport.peer_states().is_empty());
    }

    #[test]
    fn two_node_assessment_exchange() {
        // Simulate a full assessment exchange between two nodes.
        let coord_a = make_coordinator("node-a", "proj-a");
        let coord_b = make_coordinator("node-b", "proj-b");
        let transport_a = AssessmentTransport::new(coord_a.clone());
        let transport_b = AssessmentTransport::new(coord_b.clone());

        let report_a = make_report();

        // 1. Node A completes an assessment and broadcasts.
        let broadcast = transport_a.build_broadcast_frame(&report_a).unwrap();
        let len =
            u32::from_be_bytes([broadcast[0], broadcast[1], broadcast[2], broadcast[3]]) as usize;
        let frame = MeshFrame::decode(&broadcast[4..4 + len]).unwrap();
        transport_b.handle_incoming(&frame.payload).unwrap();

        // 2. Node A also gossips.
        let gossip = transport_a.build_gossip_frame(&report_a).unwrap();
        let len = u32::from_be_bytes([gossip[0], gossip[1], gossip[2], gossip[3]]) as usize;
        let frame = MeshFrame::decode(&gossip[4..4 + len]).unwrap();
        transport_b.handle_incoming(&frame.payload).unwrap();

        // 3. Node B now knows about Node A.
        let peers = transport_b.peer_states();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "node-a");
        assert_eq!(peers[0].project_name, "proj-a");
        assert_eq!(peers[0].analyzer_count, 3);

        // 4. Node B requests the full report.
        let req = transport_b.build_request_frame().unwrap();
        let len = u32::from_be_bytes([req[0], req[1], req[2], req[3]]) as usize;
        let frame = MeshFrame::decode(&req[4..4 + len]).unwrap();
        // Node A receives the request -- coordinator returns None
        // (caller is responsible for fetching the report).
        let response = transport_a.handle_incoming(&frame.payload).unwrap();
        assert!(response.is_none());

        // 5. Node A sends the full report back.
        let full = transport_a.build_full_report_frame(&report_a).unwrap();
        let len = u32::from_be_bytes([full[0], full[1], full[2], full[3]]) as usize;
        let frame = MeshFrame::decode(&full[4..4 + len]).unwrap();
        let resp = transport_b.handle_incoming(&frame.payload).unwrap();
        assert!(resp.is_none()); // FullReport is handled by caller
    }

    fn make_finding(file: &str, msg: &str) -> crate::assessment::Finding {
        crate::assessment::Finding {
            severity: "warning".into(),
            category: "size".into(),
            file: file.into(),
            line: Some(1),
            message: msg.into(),
        }
    }

    #[test]
    fn build_diff_frame_only_changed_findings() {
        use crate::assessment::analyzer::diff_reports;

        let coord = make_coordinator("node-a", "proj-a");
        let transport = AssessmentTransport::new(coord);
        let mut prev = make_report();
        prev.findings = vec![make_finding("a.rs", "old")];
        let mut curr = make_report();
        curr.findings = vec![
            make_finding("a.rs", "old"),
            make_finding("b.rs", "new issue"),
        ];
        let diff = diff_reports(&curr, &prev);
        assert_eq!(diff.findings_new.len(), 1);
        assert!(diff.findings_resolved.is_empty());

        let bytes = transport.build_diff_frame(&curr, &diff).unwrap();
        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let frame = MeshFrame::decode(&bytes[4..4 + len]).unwrap();
        let envelope = AssessmentEnvelope::from_bytes(&frame.payload).unwrap();
        match envelope.message {
            AssessmentMessage::FindingDiff {
                findings_new,
                findings_resolved,
                finding_count,
                ..
            } => {
                assert_eq!(findings_new.len(), 1);
                assert_eq!(findings_new[0].file, "b.rs");
                assert!(findings_resolved.is_empty());
                assert_eq!(finding_count, 2);
            }
            other => panic!("expected FindingDiff, got {other:?}"),
        }
    }

    #[test]
    fn try_extract_payload_from_full_encode_and_type_prefix() {
        let coord = make_coordinator("n", "p");
        let transport = AssessmentTransport::new(coord);
        let report = make_report();
        let full = transport.build_gossip_frame(&report).unwrap();
        let payload = AssessmentTransport::try_extract_payload(&full)
            .unwrap()
            .expect("full encode should extract");
        assert!(!payload.is_empty());

        // type-prefix only form
        let len = u32::from_be_bytes([full[0], full[1], full[2], full[3]]) as usize;
        let type_prefixed = &full[4..4 + len];
        let payload2 = AssessmentTransport::try_extract_payload(type_prefixed)
            .unwrap()
            .expect("type-prefix should extract");
        assert_eq!(payload, payload2);

        // JSON IPC must not be mistaken for assessment frames
        assert!(
            AssessmentTransport::try_extract_payload(br#"{"source_node":"x"}"#)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn queue_mesh_propagation_prefers_finding_diff() {
        use crate::assessment::AssessmentService;
        use std::io::Write;

        let coord = make_coordinator("node-a", "proj-a");
        let svc = AssessmentService::with_mesh_coordinator(coord.clone());

        let mut prev = make_report();
        prev.findings = vec![make_finding("a.rs", "old")];
        let dir = tempfile::tempdir().unwrap();
        let prev_path = dir.path().join("prev.json");
        let mut f = std::fs::File::create(&prev_path).unwrap();
        f.write_all(serde_json::to_string(&prev).unwrap().as_bytes())
            .unwrap();
        svc.set_previous_report_path(prev_path);

        let mut curr = make_report();
        curr.findings = vec![
            make_finding("a.rs", "old"),
            make_finding("b.rs", "new"),
        ];
        svc.queue_mesh_propagation(&curr);

        let pending = coord.take_pending_broadcast().expect("pending set");
        match pending {
            AssessmentMessage::FindingDiff {
                findings_new,
                findings_resolved,
                ..
            } => {
                assert_eq!(findings_new.len(), 1);
                assert!(findings_resolved.is_empty());
            }
            other => panic!("expected FindingDiff, got {other:?}"),
        }
    }

    // ── TCP integration test ─────────────────────────────────────

    #[tokio::test]
    async fn tcp_assessment_sync_between_two_nodes() {
        use crate::mesh::MeshTransport;
        use crate::mesh_tcp::TcpTransport;

        let transport_tcp = TcpTransport;
        let mut listener = transport_tcp.listen("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let coord_a = make_coordinator("node-a", "proj-a");
        let assess_a = AssessmentTransport::new(coord_a);
        let report_a = make_report();

        let coord_b = make_coordinator("node-b", "proj-b");

        // Node A connects and sends a gossip frame.
        let gossip_bytes = assess_a.build_gossip_frame(&report_a).unwrap();
        let node_a_task = tokio::spawn(async move {
            let mut stream = TcpTransport.connect(&addr.to_string()).await.unwrap();
            // The gossip_bytes include the 4-byte length prefix from
            // MeshFrame::encode. The TCP transport adds its own length
            // prefix, so we send the full encoded frame.
            stream.send(&gossip_bytes).await.unwrap();
            stream.close().await.unwrap();
        });

        // Node B accepts and reads the gossip.
        let (mut stream, _) = listener.accept().await.unwrap();
        let data = stream.recv().await.unwrap();

        // Parse the MeshFrame from the received data.
        let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let frame = MeshFrame::decode(&data[4..4 + len]).unwrap();
        assert_eq!(frame.frame_type, FrameType::AssessmentSync);

        // Feed into node B's transport.
        let assess_b = AssessmentTransport::new(coord_b.clone());
        let resp = assess_b.handle_incoming(&frame.payload).unwrap();
        assert!(resp.is_none());

        // Verify node B learned about node A.
        let peers = coord_b.peer_states();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "node-a");

        node_a_task.await.unwrap();
    }
}
