//! Configuration and secrets service (K5-G1).
//!
//! Provides [`ConfigService`] for runtime configuration management with
//! change notification, and encrypted secret storage. Backed by in-memory
//! stores (tree integration deferred to when `exochain` feature is enabled).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::KernelError;
use crate::health::HealthStatus;
use crate::process::Pid;
use crate::service::{ServiceType, SystemService};

// ---------------------------------------------------------------------------
// ConfigChange
// ---------------------------------------------------------------------------

/// A change notification for a configuration key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    /// Configuration namespace.
    pub namespace: String,
    /// Configuration key.
    pub key: String,
    /// Previous value (if any).
    pub old_value: Option<serde_json::Value>,
    /// New value (if any -- `None` for deletions).
    pub new_value: Option<serde_json::Value>,
    /// PID of the process that made the change.
    pub changed_by: Pid,
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SecretRef
// ---------------------------------------------------------------------------

/// Metadata about a stored secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRef {
    /// Secret namespace.
    pub namespace: String,
    /// Secret key.
    pub key: String,
    /// When the secret expires.
    pub expires_at: DateTime<Utc>,
    /// PIDs allowed to read this secret.
    pub scoped_to: Vec<Pid>,
}

// ---------------------------------------------------------------------------
// ConfigService
// ---------------------------------------------------------------------------

/// Configuration and secrets service.
///
/// Stores configuration values at `/kernel/config/{namespace}/{key}` and
/// secrets at `/kernel/secrets/{namespace}/{key}` (encrypted at rest).
/// Supports change notification via subscriptions.
pub struct ConfigService {
    /// Config store: "namespace/key" -> value.
    configs: DashMap<String, serde_json::Value>,
    /// Secret store: "namespace/key" -> encrypted bytes.
    secrets: DashMap<String, Vec<u8>>,
    /// Secret metadata: "namespace/key" -> SecretRef.
    secret_refs: DashMap<String, SecretRef>,
    /// Change subscribers: namespace -> list of subscription queues.
    subscribers: DashMap<String, Vec<Arc<RwLock<Vec<ConfigChange>>>>>,
    /// Encryption key (derived from genesis in production).
    encryption_key: [u8; 32],
    /// Change log for auditing.
    change_log: RwLock<Vec<ConfigChange>>,
    /// Total config sets.
    set_count: AtomicU64,
}

impl ConfigService {
    /// Create a new config service with a given encryption key.
    pub fn new(encryption_key: [u8; 32]) -> Self {
        Self {
            configs: DashMap::new(),
            secrets: DashMap::new(),
            secret_refs: DashMap::new(),
            subscribers: DashMap::new(),
            encryption_key,
            change_log: RwLock::new(Vec::new()),
            set_count: AtomicU64::new(0),
        }
    }

    /// Create a config service with a default (zero) encryption key (testing).
    pub fn new_default() -> Self {
        Self::new([0u8; 32])
    }

    // ── Config operations ─────────────────────────────────────────

    /// Set a configuration value.
    pub fn set(
        &self,
        namespace: &str,
        key: &str,
        value: serde_json::Value,
        changed_by: Pid,
    ) -> Result<(), KernelError> {
        let config_key = format!("{namespace}/{key}");
        let old_value = self.configs.get(&config_key).map(|v| v.value().clone());
        self.configs.insert(config_key, value.clone());

        let change = ConfigChange {
            namespace: namespace.to_string(),
            key: key.to_string(),
            old_value,
            new_value: Some(value),
            changed_by,
            timestamp: Utc::now(),
        };

        // Notify subscribers.
        self.notify_subscribers(namespace, &change);

        // Record in change log.
        if let Ok(mut log) = self.change_log.write() {
            log.push(change);
        }
        self.set_count.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get a configuration value.
    pub fn get(&self, namespace: &str, key: &str) -> Option<serde_json::Value> {
        let config_key = format!("{namespace}/{key}");
        self.configs.get(&config_key).map(|v| v.value().clone())
    }

    /// Delete a configuration value.
    pub fn delete(
        &self,
        namespace: &str,
        key: &str,
        changed_by: Pid,
    ) -> Result<(), KernelError> {
        let config_key = format!("{namespace}/{key}");
        let old_value = self.configs.remove(&config_key).map(|(_, v)| v);

        let change = ConfigChange {
            namespace: namespace.to_string(),
            key: key.to_string(),
            old_value,
            new_value: None,
            changed_by,
            timestamp: Utc::now(),
        };
        self.notify_subscribers(namespace, &change);
        Ok(())
    }

    /// List all config keys in a namespace.
    pub fn list_keys(&self, namespace: &str) -> Vec<String> {
        let prefix = format!("{namespace}/");
        self.configs
            .iter()
            .filter(|e| e.key().starts_with(&prefix))
            .map(|e| e.key()[prefix.len()..].to_string())
            .collect()
    }

    // ── Subscription ──────────────────────────────────────────────

    /// Subscribe to changes in a namespace.
    ///
    /// Returns a shared reference to the change queue. Callers can read
    /// accumulated changes from the returned `Arc<RwLock<Vec<ConfigChange>>>`.
    pub fn subscribe(&self, namespace: &str) -> Arc<RwLock<Vec<ConfigChange>>> {
        let queue = Arc::new(RwLock::new(Vec::new()));
        self.subscribers
            .entry(namespace.to_string())
            .or_default()
            .push(queue.clone());
        queue
    }

    /// Notify all subscribers for a namespace.
    fn notify_subscribers(&self, namespace: &str, change: &ConfigChange) {
        if let Some(mut subs) = self.subscribers.get_mut(namespace) {
            subs.retain(|queue| {
                if let Ok(mut q) = queue.write() {
                    q.push(change.clone());
                    true
                } else {
                    false // remove dead subscribers
                }
            });
        }
    }

    // ── Secret operations ─────────────────────────────────────────

    /// Store an encrypted secret.
    pub fn set_secret(
        &self,
        namespace: &str,
        key: &str,
        value: &[u8],
        scoped_to: Vec<Pid>,
    ) -> Result<(), KernelError> {
        let secret_key = format!("{namespace}/{key}");

        // Simple XOR encryption (production would use AEAD).
        let encrypted = self.xor_encrypt(value);
        self.secrets.insert(secret_key.clone(), encrypted);

        let secret_ref = SecretRef {
            namespace: namespace.to_string(),
            key: key.to_string(),
            expires_at: Utc::now() + Duration::hours(24),
            scoped_to,
        };
        self.secret_refs.insert(secret_key, secret_ref);
        Ok(())
    }

    /// Retrieve a secret (decrypted). Checks PID authorization and expiry.
    pub fn get_secret(
        &self,
        namespace: &str,
        key: &str,
        requester_pid: Pid,
    ) -> Result<Vec<u8>, KernelError> {
        let secret_key = format!("{namespace}/{key}");

        let secret_ref = self
            .secret_refs
            .get(&secret_key)
            .ok_or_else(|| KernelError::Service("secret not found".into()))?;

        // Check authorization.
        if !secret_ref.scoped_to.is_empty()
            && !secret_ref.scoped_to.contains(&requester_pid)
        {
            return Err(KernelError::CapabilityDenied {
                pid: requester_pid,
                action: "read_secret".into(),
                reason: format!("PID {} not authorized for secret {secret_key}", requester_pid),
            });
        }

        // Check expiry.
        if Utc::now() > secret_ref.expires_at {
            return Err(KernelError::Service("secret expired".into()));
        }

        let encrypted = self
            .secrets
            .get(&secret_key)
            .ok_or_else(|| KernelError::Service("secret data missing".into()))?;

        Ok(self.xor_decrypt(&encrypted))
    }

    /// Simple XOR encryption with the key (for testing; production uses AEAD).
    fn xor_encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, b)| b ^ self.encryption_key[i % 32])
            .collect()
    }

    /// Decrypt XOR-encrypted data.
    fn xor_decrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR is symmetric.
        self.xor_encrypt(data)
    }

    /// Get the change log (for auditing).
    pub fn change_log(&self) -> Vec<ConfigChange> {
        self.change_log.read().map(|l| l.clone()).unwrap_or_default()
    }

    /// Total number of config sets performed.
    pub fn set_count(&self) -> u64 {
        self.set_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl SystemService for ConfigService {
    fn name(&self) -> &str {
        "config-service"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("config service started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            configs = self.configs.len(),
            secrets = self.secrets.len(),
            "config service stopped"
        );
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u64) -> Pid {
        n
    }

    #[test]
    fn set_and_get_config() {
        let svc = ConfigService::new_default();
        svc.set("app", "timeout", serde_json::json!(30), pid(1)).unwrap();
        let val = svc.get("app", "timeout").unwrap();
        assert_eq!(val, serde_json::json!(30));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let svc = ConfigService::new_default();
        assert!(svc.get("app", "missing").is_none());
    }

    #[test]
    fn delete_config() {
        let svc = ConfigService::new_default();
        svc.set("app", "key", serde_json::json!("val"), pid(1)).unwrap();
        svc.delete("app", "key", pid(1)).unwrap();
        assert!(svc.get("app", "key").is_none());
    }

    #[test]
    fn list_keys_in_namespace() {
        let svc = ConfigService::new_default();
        svc.set("ns", "a", serde_json::json!(1), pid(1)).unwrap();
        svc.set("ns", "b", serde_json::json!(2), pid(1)).unwrap();
        svc.set("other", "c", serde_json::json!(3), pid(1)).unwrap();
        let mut keys = svc.list_keys("ns");
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn config_change_notification() {
        let svc = ConfigService::new_default();
        let sub = svc.subscribe("watch");
        svc.set("watch", "flag", serde_json::json!(true), pid(1)).unwrap();
        let changes = sub.read().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key, "flag");
        assert_eq!(changes[0].new_value, Some(serde_json::json!(true)));
    }

    #[test]
    fn config_change_includes_old_value() {
        let svc = ConfigService::new_default();
        let sub = svc.subscribe("ver");
        svc.set("ver", "v", serde_json::json!(1), pid(1)).unwrap();
        svc.set("ver", "v", serde_json::json!(2), pid(1)).unwrap();
        let changes = sub.read().unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[1].old_value, Some(serde_json::json!(1)));
        assert_eq!(changes[1].new_value, Some(serde_json::json!(2)));
    }

    #[test]
    fn secret_set_and_get() {
        let key = [0xAB; 32];
        let svc = ConfigService::new(key);
        svc.set_secret("creds", "api_key", b"secret123", vec![pid(1)])
            .unwrap();
        let val = svc.get_secret("creds", "api_key", pid(1)).unwrap();
        assert_eq!(val, b"secret123");
    }

    #[test]
    fn secret_encrypted_at_rest() {
        let key = [0xAB; 32];
        let svc = ConfigService::new(key);
        svc.set_secret("creds", "pass", b"plaintext", vec![pid(1)])
            .unwrap();
        // Verify stored data is not plaintext.
        let stored = svc.secrets.get("creds/pass").unwrap();
        assert_ne!(stored.as_slice(), b"plaintext");
    }

    #[test]
    fn unauthorized_pid_cannot_read_secret() {
        let svc = ConfigService::new_default();
        svc.set_secret("creds", "key", b"val", vec![pid(1)]).unwrap();
        let result = svc.get_secret("creds", "key", pid(99));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("denied") || err.contains("authorized"), "got: {err}");
    }

    #[test]
    fn empty_scope_allows_any_pid() {
        let svc = ConfigService::new_default();
        svc.set_secret("open", "key", b"val", vec![]).unwrap();
        let val = svc.get_secret("open", "key", pid(42)).unwrap();
        assert_eq!(val, b"val");
    }

    #[test]
    fn change_log_recorded() {
        let svc = ConfigService::new_default();
        svc.set("ns", "k", serde_json::json!("v"), pid(1)).unwrap();
        let log = svc.change_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].namespace, "ns");
    }

    #[tokio::test]
    async fn system_service_impl() {
        let svc = ConfigService::new_default();
        assert_eq!(svc.name(), "config-service");
        assert_eq!(svc.service_type(), ServiceType::Core);
        svc.start().await.unwrap();
        assert_eq!(svc.health_check().await, HealthStatus::Healthy);
        svc.stop().await.unwrap();
    }
}
