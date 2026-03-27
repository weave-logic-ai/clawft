//! Pub/sub topic routing for kernel IPC.
//!
//! The [`TopicRouter`] manages topic subscriptions and delivers
//! published messages to all subscribers. Topics are arbitrary
//! strings (e.g. "build-status", "test-results", "agent.spawned").
//!
//! Subscriptions are stored in a [`DashMap`] for lock-free concurrent
//! access. Dead subscribers (processes that have exited) are lazily
//! cleaned up during publish.

use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::process::{Pid, ProcessState, ProcessTable};

/// A topic subscription entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// The topic pattern this subscription matches.
    pub topic: String,

    /// The subscribing process's PID.
    pub subscriber_pid: Pid,

    /// Optional message filter (future use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Pub/sub topic router for kernel IPC.
///
/// Manages subscriptions as a mapping from topic name to list of
/// subscriber PIDs. Uses [`DashMap`] for lock-free concurrent access.
///
/// # Dead subscriber cleanup
///
/// When a message is published, the router checks each subscriber's
/// state in the process table. Subscribers that have exited are
/// automatically removed from the subscription list (lazy cleanup).
pub struct TopicRouter {
    /// Topic -> list of subscriber PIDs.
    subscriptions: DashMap<String, Vec<Pid>>,

    /// Process table for checking subscriber state.
    process_table: Arc<ProcessTable>,
}

impl TopicRouter {
    /// Create a new topic router.
    pub fn new(process_table: Arc<ProcessTable>) -> Self {
        Self {
            subscriptions: DashMap::new(),
            process_table,
        }
    }

    /// Subscribe a process to a topic.
    ///
    /// If the process is already subscribed, this is a no-op.
    pub fn subscribe(&self, pid: Pid, topic: &str) {
        debug!(pid, topic, "subscribing to topic");
        self.subscriptions
            .entry(topic.to_owned())
            .or_default()
            .push(pid);

        // Deduplicate
        if let Some(mut subs) = self.subscriptions.get_mut(topic) {
            subs.dedup();
        }
    }

    /// Unsubscribe a process from a topic.
    ///
    /// If the process is not subscribed, this is a no-op.
    /// Empty subscription lists are removed.
    pub fn unsubscribe(&self, pid: Pid, topic: &str) {
        debug!(pid, topic, "unsubscribing from topic");
        if let Some(mut subs) = self.subscriptions.get_mut(topic) {
            subs.retain(|&p| p != pid);
        }

        // Clean up empty topics
        self.subscriptions.retain(|_, subs| !subs.is_empty());
    }

    /// Get the list of running subscribers for a topic.
    ///
    /// Performs lazy cleanup: removes PIDs for processes that have
    /// exited. Returns only PIDs of running processes.
    pub fn live_subscribers(&self, topic: &str) -> Vec<Pid> {
        let mut live = Vec::new();
        let mut dead = Vec::new();

        if let Some(subs) = self.subscriptions.get(topic) {
            for &pid in subs.iter() {
                if self.is_alive(pid) {
                    live.push(pid);
                } else {
                    dead.push(pid);
                }
            }
        }

        // Lazy cleanup of dead subscribers
        if !dead.is_empty() {
            if let Some(mut subs) = self.subscriptions.get_mut(topic) {
                subs.retain(|p| !dead.contains(p));
            }
            warn!(
                topic,
                dead_count = dead.len(),
                "cleaned up dead subscribers"
            );
        }

        live
    }

    /// Get all subscribers for a topic (including potentially dead ones).
    ///
    /// Use [`TopicRouter::live_subscribers`] for a filtered list.
    pub fn subscribers(&self, topic: &str) -> Vec<Pid> {
        self.subscriptions
            .get(topic)
            .map(|subs| subs.clone())
            .unwrap_or_default()
    }

    /// List all topics with their subscriber counts.
    pub fn list_topics(&self) -> Vec<(String, usize)> {
        self.subscriptions
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().len()))
            .collect()
    }

    /// List all topics a specific PID is subscribed to.
    pub fn topics_for_pid(&self, pid: Pid) -> Vec<String> {
        self.subscriptions
            .iter()
            .filter(|entry| entry.value().contains(&pid))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get the total number of active topics.
    pub fn topic_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Check whether a topic has any subscribers.
    pub fn has_subscribers(&self, topic: &str) -> bool {
        self.subscriptions
            .get(topic)
            .is_some_and(|subs| !subs.is_empty())
    }

    /// Remove all subscriptions for a PID (used during process cleanup).
    pub fn unsubscribe_all(&self, pid: Pid) {
        debug!(pid, "unsubscribing from all topics");
        for mut entry in self.subscriptions.iter_mut() {
            entry.value_mut().retain(|&p| p != pid);
        }
        // Clean up empty topics
        self.subscriptions.retain(|_, subs| !subs.is_empty());
    }

    /// Check whether a PID corresponds to an alive process.
    fn is_alive(&self, pid: Pid) -> bool {
        self.process_table
            .get(pid)
            .is_some_and(|entry| !matches!(entry.state, ProcessState::Exited(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::AgentCapabilities;
    use crate::process::{ProcessEntry, ResourceUsage};
    use tokio_util::sync::CancellationToken;

    fn make_router_with_processes(count: usize) -> (TopicRouter, Vec<Pid>) {
        let table = Arc::new(ProcessTable::new(64));
        let mut pids = Vec::new();
        for i in 0..count {
            let entry = ProcessEntry {
                pid: 0,
                agent_id: format!("agent-{i}"),
                state: ProcessState::Running,
                capabilities: AgentCapabilities::default(),
                resource_usage: ResourceUsage::default(),
                cancel_token: CancellationToken::new(),
                parent_pid: None,
            };
            let pid = table.insert(entry).unwrap();
            pids.push(pid);
        }
        (TopicRouter::new(table), pids)
    }

    #[test]
    fn subscribe_and_list() {
        let (router, pids) = make_router_with_processes(2);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[1], "build");

        let subs = router.subscribers("build");
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&pids[0]));
        assert!(subs.contains(&pids[1]));
    }

    #[test]
    fn subscribe_idempotent() {
        let (router, pids) = make_router_with_processes(1);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[0], "build");

        let subs = router.subscribers("build");
        assert_eq!(subs.len(), 1);
    }

    #[test]
    fn unsubscribe() {
        let (router, pids) = make_router_with_processes(2);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[1], "build");

        router.unsubscribe(pids[0], "build");

        let subs = router.subscribers("build");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], pids[1]);
    }

    #[test]
    fn unsubscribe_nonexistent_is_noop() {
        let (router, _pids) = make_router_with_processes(0);
        router.unsubscribe(999, "build"); // Should not panic
        assert!(router.subscribers("build").is_empty());
    }

    #[test]
    fn unsubscribe_removes_empty_topic() {
        let (router, pids) = make_router_with_processes(1);
        router.subscribe(pids[0], "build");
        router.unsubscribe(pids[0], "build");

        assert_eq!(router.topic_count(), 0);
    }

    #[test]
    fn list_topics() {
        let (router, pids) = make_router_with_processes(2);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[0], "test");
        router.subscribe(pids[1], "build");

        let topics = router.list_topics();
        assert_eq!(topics.len(), 2);

        let build_count = topics
            .iter()
            .find(|(t, _)| t == "build")
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(build_count, 2);
    }

    #[test]
    fn topics_for_pid() {
        let (router, pids) = make_router_with_processes(1);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[0], "test");
        router.subscribe(pids[0], "deploy");

        let topics = router.topics_for_pid(pids[0]);
        assert_eq!(topics.len(), 3);
    }

    #[test]
    fn has_subscribers() {
        let (router, pids) = make_router_with_processes(1);
        assert!(!router.has_subscribers("build"));

        router.subscribe(pids[0], "build");
        assert!(router.has_subscribers("build"));
    }

    #[test]
    fn live_subscribers_filters_dead() {
        let table = Arc::new(ProcessTable::new(64));

        // Create a running process
        let entry1 = ProcessEntry {
            pid: 0,
            agent_id: "alive".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let pid1 = table.insert(entry1).unwrap();

        // Create a dead process
        let entry2 = ProcessEntry {
            pid: 0,
            agent_id: "dead".to_owned(),
            state: ProcessState::Running,
            capabilities: AgentCapabilities::default(),
            resource_usage: ResourceUsage::default(),
            cancel_token: CancellationToken::new(),
            parent_pid: None,
        };
        let pid2 = table.insert(entry2).unwrap();
        table.update_state(pid2, ProcessState::Exited(0)).unwrap();

        let router = TopicRouter::new(table);
        router.subscribe(pid1, "build");
        router.subscribe(pid2, "build");

        // All subscribers includes dead
        assert_eq!(router.subscribers("build").len(), 2);

        // Live subscribers excludes dead and cleans up
        let live = router.live_subscribers("build");
        assert_eq!(live.len(), 1);
        assert_eq!(live[0], pid1);

        // After cleanup, subscribers list is also cleaned
        assert_eq!(router.subscribers("build").len(), 1);
    }

    #[test]
    fn unsubscribe_all() {
        let (router, pids) = make_router_with_processes(2);
        router.subscribe(pids[0], "build");
        router.subscribe(pids[0], "test");
        router.subscribe(pids[1], "build");

        router.unsubscribe_all(pids[0]);

        assert!(router.topics_for_pid(pids[0]).is_empty());
        assert_eq!(router.subscribers("build").len(), 1);
        assert_eq!(router.topic_count(), 1); // "test" removed (empty)
    }

    #[test]
    fn subscription_serde_roundtrip() {
        let sub = Subscription {
            topic: "build".into(),
            subscriber_pid: 42,
            filter: Some("status:*".into()),
        };
        let json = serde_json::to_string(&sub).unwrap();
        let restored: Subscription = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.topic, "build");
        assert_eq!(restored.subscriber_pid, 42);
        assert_eq!(restored.filter, Some("status:*".into()));
    }
}
