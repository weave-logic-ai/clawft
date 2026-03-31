//! SWIM-style heartbeat and failure detection for mesh (K6.5).
//!
//! Implements a simplified SWIM protocol for detecting node failures
//! in the mesh network. Each tick, a random peer is pinged directly;
//! if no response, indirect probes are sent via other peers.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Heartbeat configuration.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// How often to probe a peer (default 1s).
    pub probe_interval: Duration,
    /// Direct ping timeout (default 500ms).
    pub ping_timeout: Duration,
    /// Indirect probe timeout (default 1s).
    pub indirect_timeout: Duration,
    /// Number of indirect probe witnesses (default 3).
    pub indirect_witnesses: usize,
    /// Time in suspect state before marking unreachable (default 5s).
    pub suspect_timeout: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            probe_interval: Duration::from_secs(1),
            ping_timeout: Duration::from_millis(500),
            indirect_timeout: Duration::from_secs(1),
            indirect_witnesses: 3,
            suspect_timeout: Duration::from_secs(5),
        }
    }
}

/// Ping request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    /// Source node ID.
    pub from_node: String,
    /// Sequence number for correlation.
    pub sequence: u64,
    /// Whether this is an indirect ping (on behalf of another node).
    pub indirect: bool,
    /// If indirect, the original requester.
    pub on_behalf_of: Option<String>,
}

/// Ping response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    /// Responding node ID.
    pub from_node: String,
    /// Matching sequence number.
    pub sequence: u64,
    /// Process count on responding node.
    pub process_count: u32,
    /// Service count on responding node.
    pub service_count: u32,
}

/// Node health state from heartbeat monitoring.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatState {
    /// Node is healthy (responding to pings).
    Alive,
    /// Node missed direct ping, awaiting indirect probe.
    Suspect,
    /// Node confirmed unreachable (indirect probes also failed).
    Dead,
}

/// Heartbeat tracker for a single peer.
#[derive(Debug, Clone)]
pub struct PeerHeartbeat {
    /// Current health state.
    pub state: HeartbeatState,
    /// Last successful heartbeat time.
    pub last_seen: Instant,
    /// When suspect state was entered (if applicable).
    pub suspect_since: Option<Instant>,
    /// Consecutive missed pings.
    pub missed_count: u32,
    /// Last ping sequence number sent.
    pub last_ping_seq: u64,
}

impl PeerHeartbeat {
    pub fn new() -> Self {
        Self {
            state: HeartbeatState::Alive,
            last_seen: Instant::now(),
            suspect_since: None,
            missed_count: 0,
            last_ping_seq: 0,
        }
    }

    /// Record a successful heartbeat.
    pub fn record_alive(&mut self) {
        self.state = HeartbeatState::Alive;
        self.last_seen = Instant::now();
        self.suspect_since = None;
        self.missed_count = 0;
    }

    /// Record a missed heartbeat.
    pub fn record_miss(&mut self, config: &HeartbeatConfig) {
        self.missed_count += 1;
        match self.state {
            HeartbeatState::Alive => {
                self.state = HeartbeatState::Suspect;
                self.suspect_since = Some(Instant::now());
            }
            HeartbeatState::Suspect => {
                if let Some(since) = self.suspect_since
                    && since.elapsed() > config.suspect_timeout
                {
                    self.state = HeartbeatState::Dead;
                }
            }
            HeartbeatState::Dead => {} // already dead
        }
    }

    /// Check if this peer should be considered unreachable.
    pub fn is_unreachable(&self, config: &HeartbeatConfig) -> bool {
        self.state == HeartbeatState::Dead
            || self.last_seen.elapsed() > config.suspect_timeout * 3
    }
}

impl Default for PeerHeartbeat {
    fn default() -> Self {
        Self::new()
    }
}

/// Heartbeat tracker managing all peers.
pub struct HeartbeatTracker {
    config: HeartbeatConfig,
    peers: HashMap<String, PeerHeartbeat>,
    next_sequence: u64,
}

impl HeartbeatTracker {
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            peers: HashMap::new(),
            next_sequence: 1,
        }
    }

    /// Start tracking a new peer.
    pub fn add_peer(&mut self, node_id: String) {
        self.peers
            .entry(node_id)
            .or_default();
    }

    /// Remove a peer from tracking.
    pub fn remove_peer(&mut self, node_id: &str) {
        self.peers.remove(node_id);
    }

    /// Record a successful ping response.
    pub fn record_alive(&mut self, node_id: &str) {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.record_alive();
        }
    }

    /// Record a missed ping.
    pub fn record_miss(&mut self, node_id: &str) {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.record_miss(&self.config);
        }
    }

    /// Get the next ping sequence number.
    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        seq
    }

    /// Get all peers in suspect state.
    pub fn suspect_peers(&self) -> Vec<&str> {
        self.peers
            .iter()
            .filter(|(_, p)| p.state == HeartbeatState::Suspect)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Get all peers confirmed dead.
    pub fn dead_peers(&self) -> Vec<&str> {
        self.peers
            .iter()
            .filter(|(_, p)| p.state == HeartbeatState::Dead)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Number of tracked peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get health state for a specific peer.
    pub fn peer_state(&self, node_id: &str) -> Option<HeartbeatState> {
        self.peers.get(node_id).map(|p| p.state)
    }
}

/// Observability metrics for a mesh peer, used for affinity scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetrics {
    /// Node identifier.
    pub node_id: String,
    /// Smoothed round-trip time in milliseconds.
    pub rtt_ms: f64,
    /// Total messages sent to this peer.
    pub messages_sent: u64,
    /// Total messages received from this peer.
    pub messages_received: u64,
    /// Total bytes sent to this peer.
    pub bytes_sent: u64,
    /// Total bytes received from this peer.
    pub bytes_received: u64,
    /// Total errors encountered with this peer.
    pub errors: u64,
    /// Last update timestamp (epoch millis).
    pub last_updated: u64,
}

impl PeerMetrics {
    /// Create new zeroed metrics for a peer.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            rtt_ms: 0.0,
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            errors: 0,
            last_updated: 0,
        }
    }

    /// Affinity score (lower is better). Used for service resolution preference.
    pub fn affinity_score(&self) -> f64 {
        let error_rate = if self.messages_sent > 0 {
            self.errors as f64 / self.messages_sent as f64
        } else {
            0.0
        };
        self.rtt_ms + error_rate * 1000.0
    }

    /// Record a successful message send.
    pub fn record_send(&mut self, bytes: u64) {
        self.messages_sent += 1;
        self.bytes_sent += bytes;
    }

    /// Record a received message.
    pub fn record_recv(&mut self, bytes: u64) {
        self.messages_received += 1;
        self.bytes_received += bytes;
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.errors += 1;
    }

    /// Update RTT measurement using exponential moving average.
    pub fn update_rtt(&mut self, rtt_ms: f64) {
        self.rtt_ms = self.rtt_ms * 0.8 + rtt_ms * 0.2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_request_serde_roundtrip() {
        let req = PingRequest {
            from_node: "node-a".to_string(),
            sequence: 42,
            indirect: true,
            on_behalf_of: Some("node-b".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: PingRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.from_node, "node-a");
        assert_eq!(restored.sequence, 42);
        assert!(restored.indirect);
        assert_eq!(restored.on_behalf_of, Some("node-b".to_string()));
    }

    #[test]
    fn ping_response_serde_roundtrip() {
        let resp = PingResponse {
            from_node: "node-b".to_string(),
            sequence: 42,
            process_count: 5,
            service_count: 3,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let restored: PingResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.from_node, "node-b");
        assert_eq!(restored.sequence, 42);
        assert_eq!(restored.process_count, 5);
        assert_eq!(restored.service_count, 3);
    }

    #[test]
    fn heartbeat_config_defaults() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.probe_interval, Duration::from_secs(1));
        assert_eq!(config.ping_timeout, Duration::from_millis(500));
        assert_eq!(config.indirect_timeout, Duration::from_secs(1));
        assert_eq!(config.indirect_witnesses, 3);
        assert_eq!(config.suspect_timeout, Duration::from_secs(5));
    }

    #[test]
    fn peer_heartbeat_alive_to_suspect() {
        let config = HeartbeatConfig::default();
        let mut peer = PeerHeartbeat::new();
        assert_eq!(peer.state, HeartbeatState::Alive);

        peer.record_miss(&config);
        assert_eq!(peer.state, HeartbeatState::Suspect);
        assert_eq!(peer.missed_count, 1);
        assert!(peer.suspect_since.is_some());
    }

    #[test]
    fn peer_heartbeat_suspect_to_dead() {
        // Use a zero-duration suspect timeout so the transition is immediate.
        let config = HeartbeatConfig {
            suspect_timeout: Duration::from_secs(0),
            ..HeartbeatConfig::default()
        };
        let mut peer = PeerHeartbeat::new();

        // First miss: Alive -> Suspect
        peer.record_miss(&config);
        assert_eq!(peer.state, HeartbeatState::Suspect);

        // Second miss with zero timeout: Suspect -> Dead
        peer.record_miss(&config);
        assert_eq!(peer.state, HeartbeatState::Dead);
    }

    #[test]
    fn record_alive_resets_state() {
        let config = HeartbeatConfig::default();
        let mut peer = PeerHeartbeat::new();

        peer.record_miss(&config);
        assert_eq!(peer.state, HeartbeatState::Suspect);

        peer.record_alive();
        assert_eq!(peer.state, HeartbeatState::Alive);
        assert_eq!(peer.missed_count, 0);
        assert!(peer.suspect_since.is_none());
    }

    #[test]
    fn tracker_add_remove_peers() {
        let mut tracker = HeartbeatTracker::new(HeartbeatConfig::default());
        tracker.add_peer("node-a".to_string());
        tracker.add_peer("node-b".to_string());
        assert_eq!(tracker.peer_count(), 2);

        tracker.remove_peer("node-a");
        assert_eq!(tracker.peer_count(), 1);
        assert!(tracker.peer_state("node-a").is_none());
        assert_eq!(tracker.peer_state("node-b"), Some(HeartbeatState::Alive));
    }

    #[test]
    fn tracker_suspect_and_dead_peers() {
        let config = HeartbeatConfig {
            suspect_timeout: Duration::from_secs(0),
            ..HeartbeatConfig::default()
        };
        let mut tracker = HeartbeatTracker::new(config);
        tracker.add_peer("node-a".to_string());
        tracker.add_peer("node-b".to_string());
        tracker.add_peer("node-c".to_string());

        // node-a: one miss -> suspect
        tracker.record_miss("node-a");
        // node-b: two misses with zero timeout -> dead
        tracker.record_miss("node-b");
        tracker.record_miss("node-b");
        // node-c: stays alive

        let suspects = tracker.suspect_peers();
        assert_eq!(suspects.len(), 1);
        assert!(suspects.contains(&"node-a"));

        let dead = tracker.dead_peers();
        assert_eq!(dead.len(), 1);
        assert!(dead.contains(&"node-b"));
    }

    // ── PeerMetrics tests ─────────────────────────────────────────

    #[test]
    fn peer_metrics_affinity_score_zero_errors() {
        let mut m = PeerMetrics::new("n1".into());
        m.rtt_ms = 10.0;
        m.messages_sent = 100;
        m.errors = 0;
        // score = rtt + 0 = 10.0
        assert!((m.affinity_score() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn peer_metrics_affinity_score_with_errors() {
        let mut m = PeerMetrics::new("n1".into());
        m.rtt_ms = 5.0;
        m.messages_sent = 100;
        m.errors = 10;
        // error_rate = 10/100 = 0.1, score = 5.0 + 0.1*1000 = 105.0
        assert!((m.affinity_score() - 105.0).abs() < f64::EPSILON);
    }

    #[test]
    fn peer_metrics_affinity_no_messages() {
        let m = PeerMetrics::new("n1".into());
        // No messages sent: error_rate = 0, score = 0
        assert!((m.affinity_score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn peer_metrics_record_send_recv() {
        let mut m = PeerMetrics::new("n1".into());
        m.record_send(100);
        m.record_send(200);
        assert_eq!(m.messages_sent, 2);
        assert_eq!(m.bytes_sent, 300);

        m.record_recv(50);
        assert_eq!(m.messages_received, 1);
        assert_eq!(m.bytes_received, 50);
    }

    #[test]
    fn peer_metrics_record_error() {
        let mut m = PeerMetrics::new("n1".into());
        m.record_error();
        m.record_error();
        assert_eq!(m.errors, 2);
    }

    #[test]
    fn peer_metrics_update_rtt_ema() {
        let mut m = PeerMetrics::new("n1".into());
        m.rtt_ms = 100.0;
        m.update_rtt(50.0);
        // EMA: 100*0.8 + 50*0.2 = 80 + 10 = 90
        assert!((m.rtt_ms - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn peer_metrics_serde_roundtrip() {
        let mut m = PeerMetrics::new("node-x".into());
        m.rtt_ms = 15.5;
        m.messages_sent = 42;
        let json = serde_json::to_string(&m).unwrap();
        let restored: PeerMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.node_id, "node-x");
        assert!((restored.rtt_ms - 15.5).abs() < f64::EPSILON);
        assert_eq!(restored.messages_sent, 42);
    }

    #[test]
    fn next_sequence_increments() {
        let mut tracker = HeartbeatTracker::new(HeartbeatConfig::default());
        assert_eq!(tracker.next_sequence(), 1);
        assert_eq!(tracker.next_sequence(), 2);
        assert_eq!(tracker.next_sequence(), 3);
    }
}
