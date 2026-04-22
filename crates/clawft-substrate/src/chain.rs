//! `chain` reference adapter — promotes the tray's ExoChain chip from
//! "service named `chain`/`exochain` is registered" to "the daemon's
//! `chain.status` returns fresh data."
//!
//! Polls `chain.status`. On daemons built without the `exochain`
//! feature the RPC returns an error and the adapter emits
//! `{available: false}` so the tray can render amber/grey rather than
//! pretend the chain is up.
//!
//! ## Topic
//!
//! | Topic | Shape | Refresh |
//! |-------|-------|---------|
//! | `substrate/chain/status` | success → `{chain_id, sequence, event_count, checkpoint_count, last_hash}`; failure → `{available: false, reason}` | 3s |
//!
//! Public — chain head hash + sequence is public metadata, not user
//! content. Writes stay gated through governance (ADR-015); this
//! topic is read-only.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use clawft_rpc::DaemonClient;
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

use crate::adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
use crate::delta::StateDelta;

const CHAN: usize = 1;

/// Declared topic.
pub const TOPICS: &[TopicDecl] = &[TopicDecl {
    path: "substrate/chain/status",
    shape: "ontology://chain-status",
    refresh_hint: RefreshHint::Periodic { ms: 3000 },
    sensitivity: Sensitivity::Public,
    buffer_policy: BufferPolicy::Refuse,
    max_len: None,
}];

/// Permissions — none.
pub const PERMISSIONS: &[PermissionReq] = &[];

type CancelTx = oneshot::Sender<()>;

struct Registry {
    next_id: u64,
    live: HashMap<SubId, CancelTx>,
}

impl Registry {
    fn new() -> Self {
        Self {
            next_id: 1,
            live: HashMap::new(),
        }
    }

    fn allocate(&mut self) -> SubId {
        let id = SubId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

/// Chain adapter — calls the daemon's `chain.status` RPC verb.
pub struct ChainAdapter {
    reg: Mutex<Registry>,
}

impl Default for ChainAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainAdapter {
    /// Build a new adapter. Connects to the daemon on first poll.
    pub fn new() -> Self {
        Self {
            reg: Mutex::new(Registry::new()),
        }
    }
}

#[async_trait]
impl OntologyAdapter for ChainAdapter {
    fn id(&self) -> &'static str {
        "chain"
    }

    fn topics(&self) -> &'static [TopicDecl] {
        TOPICS
    }

    fn permissions(&self) -> &'static [PermissionReq] {
        PERMISSIONS
    }

    async fn open(
        &self,
        topic: &str,
        _args: Value,
    ) -> Result<Subscription, AdapterError> {
        if topic != "substrate/chain/status" {
            return Err(AdapterError::UnknownTopic(topic.into()));
        }
        let id = {
            let mut reg = self.reg.lock();
            reg.allocate()
        };
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel::<StateDelta>(CHAN);
        self.reg.lock().live.insert(id, cancel_tx);

        tokio::spawn(async move {
            poll_chain_status(tx, cancel_rx).await;
        });
        Ok(Subscription { id, rx })
    }

    async fn close(&self, sub_id: SubId) -> Result<(), AdapterError> {
        let _ = self.reg.lock().live.remove(&sub_id);
        Ok(())
    }
}

async fn poll_chain_status(
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let mut client: Option<DaemonClient> = None;
    let mut ticker = tokio::time::interval(Duration::from_secs(3));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                if client.is_none() {
                    client = DaemonClient::connect().await;
                }
                let Some(c) = client.as_mut() else {
                    let delta = StateDelta::Replace {
                        path: "substrate/chain/status".into(),
                        value: json!({ "available": false, "reason": "daemon-unreachable" }),
                    };
                    if tx.send(delta).await.is_err() {
                        return;
                    }
                    continue;
                };
                match c.simple_call("chain.status").await {
                    Ok(resp) if resp.ok => {
                        // Enrich the success response with an
                        // `available: true` flag so the tray binding
                        // can match a single predicate instead of
                        // distinguishing two shapes.
                        let mut value = resp.result.unwrap_or(Value::Null);
                        if let Value::Object(ref mut obj) = value {
                            obj.insert("available".into(), json!(true));
                        }
                        let delta = StateDelta::Replace {
                            path: "substrate/chain/status".into(),
                            value,
                        };
                        if tx.send(delta).await.is_err() {
                            return;
                        }
                    }
                    Ok(resp) => {
                        // chain.status returns `error` when the
                        // daemon was built without the `exochain`
                        // feature. Surface it as `available: false`.
                        let err = resp.error.unwrap_or_else(|| "unknown".into());
                        let delta = StateDelta::Replace {
                            path: "substrate/chain/status".into(),
                            value: json!({
                                "available": false,
                                "reason": err,
                            }),
                        };
                        if tx.send(delta).await.is_err() {
                            return;
                        }
                    }
                    Err(_e) => {
                        client = None;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adapter_open_unknown_topic_errors() {
        let a = ChainAdapter::new();
        let r = a.open("substrate/chain/bogus", Value::Null).await;
        assert!(matches!(r, Err(AdapterError::UnknownTopic(_))));
    }

    #[test]
    fn declares_single_topic() {
        let paths: Vec<&str> = TOPICS.iter().map(|t| t.path).collect();
        assert_eq!(paths, vec!["substrate/chain/status"]);
    }
}
