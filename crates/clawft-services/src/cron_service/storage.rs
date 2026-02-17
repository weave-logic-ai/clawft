//! JSONL append-only persistence for cron jobs.
//!
//! Events are appended as newline-delimited JSON. On load, the event
//! log is replayed to reconstruct the current set of active jobs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tracing::warn;

use super::scheduler::CronJob;
use crate::error::Result;

/// Event types stored in the JSONL log.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StorageEvent {
    /// A new job was created.
    Create { job: CronJob },
    /// A field on an existing job was updated.
    Update {
        job_id: String,
        field: String,
        value: serde_json::Value,
    },
    /// A job was deleted.
    Delete { job_id: String },
}

/// JSONL append-only storage for cron job events.
pub struct CronStorage {
    path: PathBuf,
}

impl CronStorage {
    /// Create a new storage instance backed by the given file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Append a job creation event.
    pub async fn append_create(&self, job: &CronJob) -> Result<()> {
        let event = StorageEvent::Create { job: job.clone() };
        self.append_event(&event).await
    }

    /// Append a field update event.
    pub async fn append_update(&self, job_id: &str, field: &str, value: &serde_json::Value) -> Result<()> {
        let event = StorageEvent::Update {
            job_id: job_id.to_string(),
            field: field.to_string(),
            value: value.clone(),
        };
        self.append_event(&event).await
    }

    /// Append a deletion event.
    pub async fn append_delete(&self, job_id: &str) -> Result<()> {
        let event = StorageEvent::Delete {
            job_id: job_id.to_string(),
        };
        self.append_event(&event).await
    }

    /// Replay the event log and reconstruct all active jobs.
    ///
    /// Invalid lines are skipped with a warning.
    pub async fn load_jobs(&self) -> Result<Vec<CronJob>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&self.path).await?;
        let mut jobs = std::collections::HashMap::<String, CronJob>::new();

        for (line_no, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<StorageEvent>(line) {
                Ok(StorageEvent::Create { job }) => {
                    jobs.insert(job.id.clone(), job);
                }
                Ok(StorageEvent::Update {
                    job_id,
                    field,
                    value,
                }) => {
                    if let Some(job) = jobs.get_mut(&job_id) {
                        apply_field_update(job, &field, &value);
                    }
                }
                Ok(StorageEvent::Delete { job_id }) => {
                    jobs.remove(&job_id);
                }
                Err(e) => {
                    warn!(line = line_no + 1, error = %e, "skipping invalid JSONL line");
                }
            }
        }

        Ok(jobs.into_values().collect())
    }

    /// Append a serialized event followed by a newline.
    async fn append_event(&self, event: &StorageEvent) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut line = serde_json::to_string(event)?;
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }
}

/// Apply a field update to a job in memory.
fn apply_field_update(job: &mut CronJob, field: &str, value: &serde_json::Value) {
    match field {
        "enabled" => {
            if let Some(v) = value.as_bool() {
                job.enabled = v;
            }
        }
        "prompt" => {
            if let Some(v) = value.as_str() {
                job.prompt = v.to_string();
            }
        }
        "schedule" => {
            if let Some(v) = value.as_str() {
                job.schedule = v.to_string();
            }
        }
        "name" => {
            if let Some(v) = value.as_str() {
                job.name = v.to_string();
            }
        }
        "last_run" => {
            if let Ok(v) = serde_json::from_value::<Option<chrono::DateTime<chrono::Utc>>>(value.clone()) {
                job.last_run = v;
            }
        }
        "next_run" => {
            if let Ok(v) = serde_json::from_value::<Option<chrono::DateTime<chrono::Utc>>>(value.clone()) {
                job.next_run = v;
            }
        }
        _ => {
            warn!(field, "unknown field in storage update event");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_job(id: &str, name: &str) -> CronJob {
        CronJob {
            id: id.into(),
            name: name.into(),
            schedule: "0 0 * * * * *".into(),
            prompt: "test".into(),
            enabled: true,
            last_run: None,
            next_run: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn append_create_and_load() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        let storage = CronStorage::new(path);

        let job = make_job("j1", "test-job");
        storage.append_create(&job).await.unwrap();

        let jobs = storage.load_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "j1");
        assert_eq!(jobs[0].name, "test-job");

        // Cleanup.
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn append_update_and_reload() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        let storage = CronStorage::new(path);

        let job = make_job("j1", "test-job");
        storage.append_create(&job).await.unwrap();
        storage
            .append_update("j1", "enabled", &serde_json::json!(false))
            .await
            .unwrap();

        let jobs = storage.load_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert!(!jobs[0].enabled);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn append_delete_and_reload() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        let storage = CronStorage::new(path);

        storage.append_create(&make_job("j1", "a")).await.unwrap();
        storage.append_create(&make_job("j2", "b")).await.unwrap();
        storage.append_delete("j1").await.unwrap();

        let jobs = storage.load_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "j2");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn skip_invalid_jsonl_lines() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let job = make_job("j1", "valid");
        let valid_line = serde_json::to_string(&StorageEvent::Create { job }).unwrap();
        let content = format!("{valid_line}\nthis is garbage\n{{\n");
        tokio::fs::write(&path, content).await.unwrap();

        let storage = CronStorage::new(path);
        let jobs = storage.load_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "valid");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn empty_file_returns_empty_list() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(&path, "").await.unwrap();

        let storage = CronStorage::new(path);
        let jobs = storage.load_jobs().await.unwrap();
        assert!(jobs.is_empty());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn nonexistent_file_returns_empty_list() {
        let path = std::env::temp_dir().join(format!("clawft-test-nonexistent-{}.jsonl", uuid::Uuid::new_v4()));
        let storage = CronStorage::new(path);
        let jobs = storage.load_jobs().await.unwrap();
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn update_prompt_field() {
        let dir = std::env::temp_dir().join(format!("clawft-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("cron.jsonl");
        let storage = CronStorage::new(path);

        storage.append_create(&make_job("j1", "test")).await.unwrap();
        storage
            .append_update("j1", "prompt", &serde_json::json!("new prompt"))
            .await
            .unwrap();

        let jobs = storage.load_jobs().await.unwrap();
        assert_eq!(jobs[0].prompt, "new prompt");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
