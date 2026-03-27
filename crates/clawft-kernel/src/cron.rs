//! Cron scheduling service for WeftOS kernel.
//!
//! Provides interval-based job scheduling with per-agent targeting.
//! Jobs fire on a regular interval and dispatch IPC messages to
//! target agents via the A2ARouter.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::health::HealthStatus;
use crate::process::Pid;
use crate::service::{ServiceType, SystemService};

/// A scheduled cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique job identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Fire every N seconds.
    pub interval_secs: u64,
    /// Command payload to send.
    pub command: String,
    /// Target agent PID (None = kernel).
    pub target_pid: Option<Pid>,
    /// Whether the job is active.
    pub enabled: bool,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// Last time the job fired.
    pub last_fired: Option<DateTime<Utc>>,
    /// Number of times the job has fired.
    pub fire_count: u64,
}

/// Result of a tick: which jobs fired.
#[derive(Debug)]
pub struct TickResult {
    /// Job IDs that fired during this tick.
    pub fired: Vec<String>,
}

/// Cron scheduling service.
///
/// Maintains a registry of interval-based jobs. The daemon calls
/// `tick()` periodically (e.g., every second) and the service fires
/// any overdue jobs by sending messages through the A2ARouter.
pub struct CronService {
    started: AtomicBool,
    jobs: Mutex<HashMap<String, CronJob>>,
}

impl CronService {
    pub fn new() -> Self {
        Self {
            started: AtomicBool::new(false),
            jobs: Mutex::new(HashMap::new()),
        }
    }

    /// Add a new cron job. Returns the created job.
    pub fn add_job(
        &self,
        name: String,
        interval_secs: u64,
        command: String,
        target_pid: Option<Pid>,
    ) -> CronJob {
        let job = CronJob {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            interval_secs,
            command,
            target_pid,
            enabled: true,
            created_at: Utc::now(),
            last_fired: None,
            fire_count: 0,
        };

        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job.id.clone(), job.clone());
        debug!(job_id = %job.id, name = %job.name, interval = job.interval_secs, "cron job added");
        job
    }

    /// Remove a job by ID. Returns the removed job if it existed.
    pub fn remove_job(&self, id: &str) -> Option<CronJob> {
        let mut jobs = self.jobs.lock().unwrap();
        let removed = jobs.remove(id);
        if let Some(ref j) = removed {
            debug!(job_id = %j.id, name = %j.name, "cron job removed");
        }
        removed
    }

    /// List all registered jobs.
    pub fn list_jobs(&self) -> Vec<CronJob> {
        let jobs = self.jobs.lock().unwrap();
        let mut list: Vec<CronJob> = jobs.values().cloned().collect();
        list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        list
    }

    /// Look up a single job by ID.
    pub fn get_job(&self, id: &str) -> Option<CronJob> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(id).cloned()
    }

    /// Tick the scheduler — check all jobs and collect those that are overdue.
    ///
    /// Returns the list of job IDs that should fire. The caller is responsible
    /// for actually dispatching the messages (to keep CronService decoupled
    /// from async A2ARouter).
    pub fn tick(&self) -> TickResult {
        let now = Utc::now();
        let mut fired = Vec::new();
        let mut jobs = self.jobs.lock().unwrap();

        for job in jobs.values_mut() {
            if !job.enabled {
                continue;
            }

            let should_fire = match job.last_fired {
                None => true, // Never fired — fire immediately
                Some(last) => {
                    let elapsed = (now - last).num_seconds();
                    elapsed >= job.interval_secs as i64
                }
            };

            if should_fire {
                job.last_fired = Some(now);
                job.fire_count += 1;
                fired.push(job.id.clone());
                debug!(
                    job_id = %job.id,
                    name = %job.name,
                    fire_count = job.fire_count,
                    "cron job fired"
                );
            }
        }

        TickResult { fired }
    }

    /// Get a snapshot of a job's current state (for dispatching after tick).
    pub fn job_snapshot(&self, id: &str) -> Option<CronJob> {
        self.get_job(id)
    }

    /// Number of registered jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }
}

impl Default for CronService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemService for CronService {
    fn name(&self) -> &str {
        "cron"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Cron
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.started.store(true, Ordering::Relaxed);
        tracing::info!("cron service started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.started.store(false, Ordering::Relaxed);
        tracing::info!("cron service stopped");
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.started.load(Ordering::Relaxed) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded("not started".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_jobs() {
        let svc = CronService::new();
        let j1 = svc.add_job("heartbeat".into(), 10, "ping".into(), Some(1));
        let j2 = svc.add_job("cleanup".into(), 60, "gc".into(), None);

        let jobs = svc.list_jobs();
        assert_eq!(jobs.len(), 2);
        assert_eq!(svc.job_count(), 2);

        let fetched = svc.get_job(&j1.id).unwrap();
        assert_eq!(fetched.name, "heartbeat");
        assert_eq!(fetched.interval_secs, 10);
        assert_eq!(fetched.target_pid, Some(1));

        let fetched2 = svc.get_job(&j2.id).unwrap();
        assert_eq!(fetched2.name, "cleanup");
    }

    #[test]
    fn remove_job() {
        let svc = CronService::new();
        let job = svc.add_job("temp".into(), 5, "check".into(), None);

        let removed = svc.remove_job(&job.id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "temp");
        assert_eq!(svc.job_count(), 0);

        // Remove again returns None
        assert!(svc.remove_job(&job.id).is_none());
    }

    #[test]
    fn tick_fires_new_jobs_immediately() {
        let svc = CronService::new();
        svc.add_job("fast".into(), 1, "ping".into(), None);

        let result = svc.tick();
        assert_eq!(result.fired.len(), 1);

        // Verify fire_count incremented
        let jobs = svc.list_jobs();
        assert_eq!(jobs[0].fire_count, 1);
        assert!(jobs[0].last_fired.is_some());
    }

    #[test]
    fn tick_respects_interval() {
        let svc = CronService::new();
        let job = svc.add_job("slow".into(), 3600, "check".into(), None);

        // First tick fires (never fired before)
        let result = svc.tick();
        assert_eq!(result.fired.len(), 1);

        // Second tick should NOT fire (interval not elapsed)
        let result2 = svc.tick();
        assert_eq!(result2.fired.len(), 0);
    }

    #[test]
    fn tick_skips_disabled_jobs() {
        let svc = CronService::new();
        let job = svc.add_job("disabled".into(), 1, "noop".into(), None);

        // Disable the job
        {
            let mut jobs = svc.jobs.lock().unwrap();
            jobs.get_mut(&job.id).unwrap().enabled = false;
        }

        let result = svc.tick();
        assert_eq!(result.fired.len(), 0);
    }

    #[test]
    fn empty_tick() {
        let svc = CronService::new();
        let result = svc.tick();
        assert!(result.fired.is_empty());
    }
}
