//! Mesh runtime orchestrator for WeftOS node-to-node communication (K6).
//!
//! The [`MeshRuntime`] wires together transport connections, serialization
//! via [`MeshIpcEnvelope`], and the local [`A2ARouter`] so that a
//! `RemoteNode` message target actually delivers across the network.

use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, warn};

use crate::a2a::A2ARouter;
use crate::error::{KernelError, KernelResult};
use crate::ipc::{KernelMessage, MessageTarget};
use crate::mesh_ipc::MeshIpcEnvelope;

/// A handle to a connected peer, holding the sender half of an mpsc
/// channel whose receiver is read by a background write loop.
pub struct PeerConnection {
    /// Remote node identifier.
    pub node_id: String,
    /// When the connection was established.
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Sender for outbound serialized messages.
    pub sender: tokio::sync::mpsc::Sender<Vec<u8>>,
}

/// The mesh runtime orchestrates transport, connections, and message bridging.
///
/// It maintains a set of active peer connections (keyed by node ID) and
/// provides the plumbing to:
/// 1. Send a [`MeshIpcEnvelope`] to a connected peer.
/// 2. Receive an envelope from a peer and inject it into the local
///    [`A2ARouter`].
pub struct MeshRuntime {
    /// Local node identifier.
    node_id: String,
    /// Active peer connections: node_id -> PeerConnection.
    peers: DashMap<String, PeerConnection>,
    /// Reference to the local A2A router for injecting remote messages.
    local_router: Option<Arc<A2ARouter>>,
}

impl MeshRuntime {
    /// Create a new mesh runtime for the given local node.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            peers: DashMap::new(),
            local_router: None,
        }
    }

    /// Return this node's identifier.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Attach the local A2A router for incoming message injection.
    pub fn set_local_router(&mut self, router: Arc<A2ARouter>) {
        self.local_router = Some(router);
    }

    /// Register a peer connection using an already-established channel.
    ///
    /// This is the low-level entry point used after a TCP (or other
    /// transport) connection has been set up and the node-ID exchange
    /// has completed. Higher-level helpers like `connect_peer` build
    /// on top of this.
    pub fn add_peer(&self, node_id: String, sender: tokio::sync::mpsc::Sender<Vec<u8>>) {
        debug!(peer = %node_id, "adding peer connection");
        self.peers.insert(
            node_id.clone(),
            PeerConnection {
                node_id,
                connected_at: chrono::Utc::now(),
                sender,
            },
        );
    }

    /// Send a [`MeshIpcEnvelope`] to a connected peer.
    ///
    /// Serializes the envelope to JSON bytes and pushes them into the
    /// peer's outbound channel. Returns an error if the peer is not
    /// connected or the channel is closed/full.
    pub async fn send_to_peer(
        &self,
        node_id: &str,
        envelope: MeshIpcEnvelope,
    ) -> KernelResult<()> {
        let peer = self
            .peers
            .get(node_id)
            .ok_or_else(|| KernelError::Mesh(format!("peer not connected: {node_id}")))?;

        let bytes = envelope
            .to_bytes()
            .map_err(|e| KernelError::Mesh(format!("serialization error: {e}")))?;

        peer.sender
            .send(bytes)
            .await
            .map_err(|_| KernelError::Mesh(format!("send channel closed for peer {node_id}")))?;

        debug!(peer = %node_id, "sent envelope to peer");
        Ok(())
    }

    /// Build and send a message to a remote node.
    ///
    /// Wraps a [`KernelMessage`] in a [`MeshIpcEnvelope`] with the
    /// correct source/dest and sends it to the named peer.
    pub async fn route_to_remote(
        &self,
        node_id: &str,
        message: KernelMessage,
    ) -> KernelResult<()> {
        let envelope =
            MeshIpcEnvelope::new(self.node_id.clone(), node_id.to_string(), message);
        self.send_to_peer(node_id, envelope).await
    }

    /// Handle incoming raw bytes from a peer.
    ///
    /// Deserializes the data into a [`MeshIpcEnvelope`], then injects
    /// the inner [`KernelMessage`] into the local A2A router. The
    /// message target is unwrapped from `RemoteNode` if present so
    /// that the local router delivers to the correct local process.
    pub async fn handle_incoming(&self, data: &[u8]) -> KernelResult<()> {
        let envelope = MeshIpcEnvelope::from_bytes(data)
            .map_err(|e| KernelError::Mesh(format!("deserialization error: {e}")))?;

        debug!(
            from_node = %envelope.source_node,
            dest_node = %envelope.dest_node,
            "received mesh envelope"
        );

        let router = self.local_router.as_ref().ok_or_else(|| {
            KernelError::Mesh("no local router attached to mesh runtime".into())
        })?;

        // Unwrap the RemoteNode wrapper so the local router sees the
        // inner target (Process, Service, Topic, etc.).
        let mut message = envelope.message;
        if let MessageTarget::RemoteNode { target, .. } = message.target {
            message.target = *target;
        }

        router.send(message).await
    }

    /// Number of currently connected peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// List the node IDs of all connected peers.
    pub fn peer_ids(&self) -> Vec<String> {
        self.peers
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Disconnect a peer, dropping its send channel.
    pub fn disconnect_peer(&self, node_id: &str) {
        if self.peers.remove(node_id).is_some() {
            debug!(peer = %node_id, "disconnected peer");
        } else {
            warn!(peer = %node_id, "disconnect_peer: peer not found");
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{AgentCapabilities, CapabilityChecker};
    use crate::ipc::{KernelMessage, MessagePayload, MessageTarget};
    use crate::mesh_ipc::MeshIpcEnvelope;
    use crate::process::{ProcessEntry, ProcessState, ProcessTable, ResourceUsage};
    use crate::topic::TopicRouter;
    use tokio_util::sync::CancellationToken;

    /// Helper: build a minimal A2ARouter with one registered process and return
    /// (router, pid, inbox_receiver).
    fn make_router_with_process() -> (
        Arc<A2ARouter>,
        crate::process::Pid,
        tokio::sync::mpsc::Receiver<KernelMessage>,
    ) {
        let table = Arc::new(ProcessTable::new(64));
        let entry = ProcessEntry {
            pid: 0,
            agent_id: "test-agent".into(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let pid = table.insert(entry).unwrap();
        let checker = Arc::new(CapabilityChecker::new(table.clone()));
        let topics = Arc::new(TopicRouter::new(table.clone()));
        let router = Arc::new(A2ARouter::new(table, checker, topics));
        let rx = router.create_inbox(pid);
        (router, pid, rx)
    }

    // ── Test 1: empty peer list on creation ─────────────────────

    #[test]
    fn new_runtime_has_no_peers() {
        let rt = MeshRuntime::new("node-local".into());
        assert_eq!(rt.peer_count(), 0);
        assert!(rt.peer_ids().is_empty());
        assert_eq!(rt.node_id(), "node-local");
    }

    // ── Test 2: add mock peer, verify peer_count ────────────────

    #[test]
    fn add_peer_increases_count() {
        let rt = MeshRuntime::new("local".into());
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        rt.add_peer("peer-1".into(), tx);
        assert_eq!(rt.peer_count(), 1);
        assert_eq!(rt.peer_ids(), vec!["peer-1".to_string()]);
    }

    // ── Test 3: send envelope to peer (mock channel) ────────────

    #[tokio::test]
    async fn send_to_peer_delivers_serialized_bytes() {
        let rt = MeshRuntime::new("local".into());
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        rt.add_peer("peer-1".into(), tx);

        let msg = KernelMessage::text(0, MessageTarget::Broadcast, "hello");
        let envelope = MeshIpcEnvelope::new("local".into(), "peer-1".into(), msg);
        let expected_id = envelope.envelope_id.clone();

        rt.send_to_peer("peer-1", envelope).await.unwrap();

        let received = rx.recv().await.unwrap();
        let decoded = MeshIpcEnvelope::from_bytes(&received).unwrap();
        assert_eq!(decoded.envelope_id, expected_id);
        assert_eq!(decoded.source_node, "local");
        assert_eq!(decoded.dest_node, "peer-1");
    }

    // ── Test 4: receive envelope, inject into local router ──────

    #[tokio::test]
    async fn handle_incoming_injects_into_local_router() {
        let (router, pid, mut inbox_rx) = make_router_with_process();

        let mut rt = MeshRuntime::new("node-b".into());
        rt.set_local_router(router);

        // Build an envelope targeting a local process via RemoteNode wrapper
        let inner_target = MessageTarget::Process(pid);
        let remote_target = MessageTarget::RemoteNode {
            node_id: "node-b".into(),
            target: Box::new(inner_target),
        };
        let msg = KernelMessage::text(pid, remote_target, "from-remote");
        let envelope = MeshIpcEnvelope::new("node-a".into(), "node-b".into(), msg);
        let bytes = envelope.to_bytes().unwrap();

        rt.handle_incoming(&bytes).await.unwrap();

        let delivered = inbox_rx.try_recv().unwrap();
        assert!(matches!(delivered.target, MessageTarget::Process(p) if p == pid));
        match &delivered.payload {
            MessagePayload::Text(s) => assert_eq!(s, "from-remote"),
            other => panic!("expected Text payload, got: {other:?}"),
        }
    }

    // ── Test 5: disconnect peer ─────────────────────────────────

    #[test]
    fn disconnect_peer_removes_it() {
        let rt = MeshRuntime::new("local".into());
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        rt.add_peer("peer-1".into(), tx);
        assert_eq!(rt.peer_count(), 1);

        rt.disconnect_peer("peer-1");
        assert_eq!(rt.peer_count(), 0);
    }

    // ── Test 6: round-trip (serialize -> deserialize -> inject) ──

    #[tokio::test]
    async fn round_trip_send_receive() {
        let (router, pid, mut inbox_rx) = make_router_with_process();

        // "Node A" side: build the runtime and a mock peer channel
        let rt_a = MeshRuntime::new("node-a".into());
        let (tx, mut peer_rx) = tokio::sync::mpsc::channel(16);
        rt_a.add_peer("node-b".into(), tx);

        // Send from A to B
        let msg = KernelMessage::text(
            pid,
            MessageTarget::RemoteNode {
                node_id: "node-b".into(),
                target: Box::new(MessageTarget::Process(pid)),
            },
            "round-trip",
        );
        rt_a.route_to_remote("node-b", msg).await.unwrap();

        // "Node B" side: receive the bytes and inject
        let wire_bytes = peer_rx.recv().await.unwrap();
        let mut rt_b = MeshRuntime::new("node-b".into());
        rt_b.set_local_router(router);
        rt_b.handle_incoming(&wire_bytes).await.unwrap();

        let delivered = inbox_rx.try_recv().unwrap();
        match &delivered.payload {
            MessagePayload::Text(s) => assert_eq!(s, "round-trip"),
            other => panic!("expected Text payload, got: {other:?}"),
        }
    }

    // ── Test 7: node ID exchange metadata ───────────────────────

    #[test]
    fn peer_connection_stores_node_id_and_timestamp() {
        let rt = MeshRuntime::new("local".into());
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        rt.add_peer("remote-42".into(), tx);

        let entry = rt.peers.get("remote-42").unwrap();
        assert_eq!(entry.node_id, "remote-42");
        // connected_at should be very recent (within 1 second)
        let age = chrono::Utc::now() - entry.connected_at;
        assert!(age.num_seconds() < 2);
    }

    // ── Test 8: multiple peers ──────────────────────────────────

    #[test]
    fn multiple_peers_connected() {
        let rt = MeshRuntime::new("local".into());
        for i in 0..5 {
            let (tx, _rx) = tokio::sync::mpsc::channel(16);
            rt.add_peer(format!("peer-{i}"), tx);
        }
        assert_eq!(rt.peer_count(), 5);

        let mut ids = rt.peer_ids();
        ids.sort();
        assert_eq!(ids, vec!["peer-0", "peer-1", "peer-2", "peer-3", "peer-4"]);
    }

    // ── Test 9: send to unknown peer returns error ──────────────

    #[tokio::test]
    async fn send_to_unknown_peer_errors() {
        let rt = MeshRuntime::new("local".into());
        let msg = KernelMessage::text(0, MessageTarget::Broadcast, "hi");
        let envelope = MeshIpcEnvelope::new("local".into(), "ghost".into(), msg);

        let err = rt.send_to_peer("ghost", envelope).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("peer not connected"), "got: {msg}");
    }

    // ── Test 10: handle_incoming with malformed data ────────────

    #[tokio::test]
    async fn handle_incoming_malformed_data_errors() {
        let mut rt = MeshRuntime::new("local".into());
        let (router, _pid, _rx) = make_router_with_process();
        rt.set_local_router(router);

        let err = rt.handle_incoming(b"not valid json").await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("deserialization"), "got: {msg}");
    }

    // ── Test 11: handle_incoming without router errors ──────────

    #[tokio::test]
    async fn handle_incoming_without_router_errors() {
        let rt = MeshRuntime::new("local".into());
        let msg = KernelMessage::text(0, MessageTarget::Broadcast, "hi");
        let envelope = MeshIpcEnvelope::new("a".into(), "local".into(), msg);
        let bytes = envelope.to_bytes().unwrap();

        let err = rt.handle_incoming(&bytes).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no local router"), "got: {msg}");
    }

    // ── Test 12: disconnect nonexistent peer is safe ────────────

    #[test]
    fn disconnect_nonexistent_peer_is_noop() {
        let rt = MeshRuntime::new("local".into());
        rt.disconnect_peer("ghost"); // should not panic
        assert_eq!(rt.peer_count(), 0);
    }
}
