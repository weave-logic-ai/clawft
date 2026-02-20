//! Cron scheduling service.
//!
//! Manages periodic jobs that fire [`InboundMessage`] events into the
//! message bus. Jobs are persisted via append-only JSONL storage and
//! scheduled using standard cron expressions.
//!
//! Uses the canonical [`CronJob`] type from [`clawft_types::cron`].
//! The CLI cron commands use the same JSONL storage format via the
//! synchronous helpers in [`storage`].

pub mod scheduler;
pub mod storage;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{RwLock, mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::error::{Result, ServiceError};
use clawft_types::cron::{CronJobState, CronPayload, CronSchedule, ScheduleKind};
use clawft_types::event::InboundMessage;
use scheduler::{CronJob, CronScheduler, compute_next_run};
use storage::CronStorage;

/// Cron scheduling service.
///
/// Manages cron jobs, persists them to disk, and fires them as
/// [`InboundMessage`] events via the provided channel.
pub struct CronService {
    scheduler: Arc<RwLock<CronScheduler>>,
    storage: CronStorage,
    message_tx: mpsc::Sender<InboundMessage>,
}

impl CronService {
    /// Create a new cron service.
    ///
    /// Loads any existing jobs from the JSONL storage file at `storage_path`.
    pub async fn new(
        storage_path: PathBuf,
        message_tx: mpsc::Sender<InboundMessage>,
    ) -> Result<Self> {
        let storage = CronStorage::new(storage_path);
        let mut scheduler = CronScheduler::new();

        // Load persisted jobs.
        let jobs = storage.load_jobs().await?;
        for job in jobs {
            if let Err(e) = scheduler.add_job(job) {
                warn!(error = %e, "skipping invalid persisted job");
            }
        }

        Ok(Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
            storage,
            message_tx,
        })
    }

    /// Add a new cron job.
    ///
    /// Returns the generated job ID.
    pub async fn add_job(&self, name: String, schedule: String, prompt: String) -> Result<String> {
        let id = format!("job-{}", uuid::Uuid::new_v4());
        let now = Utc::now();

        // Compute the first next_run.
        let next_run = compute_next_run(&schedule, &now)?;

        let job = CronJob {
            id: id.clone(),
            name,
            enabled: true,
            schedule: CronSchedule {
                kind: ScheduleKind::Cron,
                at_ms: None,
                every_ms: None,
                expr: Some(schedule),
                tz: Some("UTC".into()),
            },
            payload: CronPayload {
                message: prompt,
                ..Default::default()
            },
            state: CronJobState {
                next_run_at: next_run,
                ..Default::default()
            },
            created_at: now,
            updated_at: now,
            delete_after_run: false,
        };

        // Add to in-memory scheduler (validates the cron expression).
        self.scheduler.write().await.add_job(job.clone())?;

        // Persist.
        self.storage.append_create(&job).await?;

        info!(job_id = %id, "added cron job");
        Ok(id)
    }

    /// Remove a job by ID.
    pub async fn remove_job(&self, job_id: &str) -> Result<()> {
        self.scheduler.write().await.remove_job(job_id)?;
        self.storage.append_delete(job_id).await?;
        info!(job_id, "removed cron job");
        Ok(())
    }

    /// Enable or disable a job.
    pub async fn enable_job(&self, job_id: &str, enabled: bool) -> Result<()> {
        let mut sched = self.scheduler.write().await;
        let job = sched
            .get_job_mut(job_id)
            .ok_or_else(|| ServiceError::JobNotFound(job_id.to_string()))?;
        job.enabled = enabled;
        drop(sched);

        self.storage
            .append_update(job_id, "enabled", &serde_json::json!(enabled))
            .await?;
        info!(job_id, enabled, "updated cron job enabled state");
        Ok(())
    }

    /// List all registered jobs.
    pub async fn list_jobs(&self) -> Result<Vec<CronJob>> {
        Ok(self.scheduler.read().await.list_jobs())
    }

    /// Immediately fire a job, posting its prompt as an [`InboundMessage`].
    pub async fn run_job_now(&self, job_id: &str) -> Result<()> {
        let sched = self.scheduler.read().await;
        let job = sched
            .get_job(job_id)
            .ok_or_else(|| ServiceError::JobNotFound(job_id.to_string()))?;

        self.fire_job(job)?;
        drop(sched);

        // Update last_run.
        let now = Utc::now();
        self.scheduler.write().await.update_job_run(job_id, now)?;

        Ok(())
    }

    /// Start the background scheduler loop.
    ///
    /// Checks for due jobs every 60 seconds. Exits when the
    /// cancellation token is triggered.
    pub async fn start(&self, cancel: CancellationToken) -> Result<()> {
        info!("cron service started");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("cron service shutting down");
                    return Ok(());
                }
                _ = interval.tick() => {
                    self.tick().await;
                }
            }
        }
    }

    /// Check for due jobs and fire them.
    async fn tick(&self) {
        let due_jobs = self.scheduler.read().await.get_due_jobs();

        for job in &due_jobs {
            debug!(job_id = %job.id, job_name = %job.name, "firing cron job");

            if let Err(e) = self.fire_job(job) {
                error!(job_id = %job.id, error = %e, "failed to fire cron job");
                continue;
            }

            let now = Utc::now();
            if let Err(e) = self.scheduler.write().await.update_job_run(&job.id, now) {
                error!(job_id = %job.id, error = %e, "failed to update job run time");
            }
        }
    }

    /// Post a job's prompt as an InboundMessage.
    fn fire_job(&self, job: &CronJob) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("job_id".to_string(), serde_json::json!(job.id));
        metadata.insert("job_name".to_string(), serde_json::json!(job.name));

        let msg = InboundMessage {
            channel: "cron".to_string(),
            sender_id: "system".to_string(),
            chat_id: job.id.clone(),
            content: job.payload.message.clone(),
            timestamp: Utc::now(),
            media: vec![],
            metadata,
        };

        self.message_tx
            .try_send(msg)
            .map_err(|_| ServiceError::ChannelClosed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup() -> (CronService, mpsc::Receiver<InboundMessage>) {
        let dir = std::env::temp_dir().join(format!("clawft-cron-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        let (tx, rx) = mpsc::channel(1024);
        let svc = CronService::new(path, tx).await.unwrap();
        (svc, rx)
    }

    #[tokio::test]
    async fn add_job_returns_unique_id() {
        let (svc, _rx) = setup().await;
        let id1 = svc
            .add_job("j1".into(), "0 0 * * * * *".into(), "p1".into())
            .await
            .unwrap();
        let id2 = svc
            .add_job("j2".into(), "0 0 * * * * *".into(), "p2".into())
            .await
            .unwrap();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn list_jobs_returns_all() {
        let (svc, _rx) = setup().await;
        svc.add_job("a".into(), "0 0 * * * * *".into(), "p".into())
            .await
            .unwrap();
        svc.add_job("b".into(), "0 0 * * * * *".into(), "p".into())
            .await
            .unwrap();
        let jobs = svc.list_jobs().await.unwrap();
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn remove_job_works() {
        let (svc, _rx) = setup().await;
        let id = svc
            .add_job("x".into(), "0 0 * * * * *".into(), "p".into())
            .await
            .unwrap();
        svc.remove_job(&id).await.unwrap();
        assert!(svc.list_jobs().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn enable_disable_job() {
        let (svc, _rx) = setup().await;
        let id = svc
            .add_job("x".into(), "0 0 * * * * *".into(), "p".into())
            .await
            .unwrap();

        svc.enable_job(&id, false).await.unwrap();
        let jobs = svc.list_jobs().await.unwrap();
        assert!(!jobs.iter().find(|j| j.id == id).unwrap().enabled);

        svc.enable_job(&id, true).await.unwrap();
        let jobs = svc.list_jobs().await.unwrap();
        assert!(jobs.iter().find(|j| j.id == id).unwrap().enabled);
    }

    #[tokio::test]
    async fn run_job_now_posts_message() {
        let (svc, mut rx) = setup().await;
        let id = svc
            .add_job("fire".into(), "0 0 * * * * *".into(), "hello".into())
            .await
            .unwrap();

        svc.run_job_now(&id).await.unwrap();

        let msg = rx.try_recv().unwrap();
        assert_eq!(msg.channel, "cron");
        assert_eq!(msg.sender_id, "system");
        assert_eq!(msg.chat_id, id);
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.metadata["job_name"], "fire");
    }

    #[tokio::test]
    async fn run_nonexistent_job_fails() {
        let (svc, _rx) = setup().await;
        let result = svc.run_job_now("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn add_job_with_invalid_cron_fails() {
        let (svc, _rx) = setup().await;
        let result = svc
            .add_job("bad".into(), "not valid".into(), "p".into())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn add_job_has_next_run_set() {
        let (svc, _rx) = setup().await;
        svc.add_job("scheduled".into(), "0 0 * * * * *".into(), "p".into())
            .await
            .unwrap();
        let jobs = svc.list_jobs().await.unwrap();
        assert!(jobs[0].state.next_run_at.is_some());
    }
}
