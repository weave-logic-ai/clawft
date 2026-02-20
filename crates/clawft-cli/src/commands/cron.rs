//! `weft cron` -- manage scheduled jobs.
//!
//! Provides subcommands for listing, creating, removing, enabling/disabling,
//! and manually running cron jobs. Jobs are stored in JSONL event-sourced
//! format, shared with the [`CronService`] in `clawft-services`.
//!
//! The storage file is located at `~/.clawft/cron.jsonl` (or
//! `~/.nanobot/cron.jsonl` as fallback).
//!
//! # Examples
//!
//! ```text
//! weft cron list
//! weft cron add --name "daily report" --schedule "0 9 * * *" --prompt "Generate report"
//! weft cron remove job-abc123
//! weft cron enable job-abc123
//! weft cron disable job-abc123
//! weft cron run job-abc123
//! ```

use std::path::{Path, PathBuf};
use std::str::FromStr;

use chrono::{TimeZone, Utc};
use comfy_table::{Table, presets::UTF8_FULL};

use clawft_types::config::Config;
use clawft_types::cron::{
    CronJob, CronJobState, CronPayload, CronSchedule, ScheduleKind,
};

/// Default cron store filename (JSONL, shared with CronService).
const CRON_STORE_FILENAME: &str = "cron.jsonl";

/// Legacy flat-JSON filename for migration detection.
const LEGACY_STORE_FILENAME: &str = "cron.json";

/// Resolve the cron store file path.
///
/// Tries `~/.clawft/cron.jsonl`, then `~/.nanobot/cron.jsonl`.
/// Returns the first path whose parent directory exists.
fn cron_store_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let clawft_path = home.join(".clawft").join(CRON_STORE_FILENAME);
        if clawft_path.parent().is_some_and(|p| p.exists()) {
            return clawft_path;
        }
        let nanobot_path = home.join(".nanobot").join(CRON_STORE_FILENAME);
        if nanobot_path.parent().is_some_and(|p| p.exists()) {
            return nanobot_path;
        }
        // Default to .clawft
        return clawft_path;
    }
    PathBuf::from(CRON_STORE_FILENAME)
}

/// Attempt migration from legacy flat-JSON format to JSONL.
///
/// If a `cron.json` file exists alongside the JSONL path and the JSONL
/// file does not exist, imports the jobs from the legacy file.
fn migrate_legacy_store(jsonl_path: &Path) {
    let legacy_path = jsonl_path.with_file_name(LEGACY_STORE_FILENAME);
    if !legacy_path.exists() || jsonl_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&legacy_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Try parsing as a CronStore (legacy flat JSON format).
    if let Ok(store) = serde_json::from_str::<clawft_types::cron::CronStore>(&content) {
        for job in &store.jobs {
            if let Err(e) =
                clawft_services::cron_service::storage::append_create_sync(jsonl_path, job)
            {
                eprintln!("warning: failed to migrate job '{}': {e}", job.id);
            }
        }
        let count = store.jobs.len();
        if count > 0 {
            eprintln!(
                "Migrated {count} job(s) from legacy {} to {}",
                legacy_path.display(),
                jsonl_path.display()
            );
        }
    }
}

/// Load jobs from the JSONL store.
fn load_jobs(path: &Path) -> anyhow::Result<Vec<CronJob>> {
    clawft_services::cron_service::storage::load_jobs_sync(path)
        .map_err(|e| anyhow::anyhow!("failed to load cron store at {}: {e}", path.display()))
}

/// Generate a unique job ID using UUID v4.
fn generate_job_id() -> String {
    format!("job-{}", uuid::Uuid::new_v4())
}

/// Format a millisecond timestamp as a human-readable string, or "-" if `None`.
fn format_ts(ms: Option<i64>) -> String {
    match ms {
        Some(ts) => {
            let dt = Utc.timestamp_millis_opt(ts);
            match dt.single() {
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                None => "-".into(),
            }
        }
        None => "-".into(),
    }
}

/// List all cron jobs in a table.
pub fn cron_list(_config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    migrate_legacy_store(&path);
    let jobs = load_jobs(&path)?;

    if jobs.is_empty() {
        println!("No cron jobs configured.");
        println!("  Store: {}", path.display());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["ID", "NAME", "SCHEDULE", "ENABLED", "LAST RUN", "NEXT RUN"]);

    for job in &jobs {
        let schedule_str = match job.schedule.kind {
            ScheduleKind::Cron => job.schedule.expr.as_deref().unwrap_or("-").to_owned(),
            ScheduleKind::Every => {
                if let Some(ms) = job.schedule.every_ms {
                    format!("every {ms}ms")
                } else {
                    "every ?".into()
                }
            }
            ScheduleKind::At => format_ts(job.schedule.at_ms),
        };

        let enabled_str = if job.enabled { "yes" } else { "no" };
        let last_run = format_ts(job.state.last_run_at_ms);
        let next_run = format_ts(job.state.next_run_at_ms);

        table.add_row([
            &job.id,
            &job.name,
            &schedule_str,
            enabled_str,
            &last_run,
            &next_run,
        ]);
    }

    println!("{table}");
    println!("  Store: {}", path.display());
    Ok(())
}

/// Normalize a cron expression to 7-field format required by the `cron` crate.
///
/// The `cron` crate expects: `sec min hour dom month dow year`.
/// Standard 5-field cron (`min hour dom month dow`) gets `0` prepended
/// for seconds and `*` appended for year. 6-field expressions get `*`
/// appended for year.
fn normalize_cron_expr(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    match fields.len() {
        5 => format!("0 {expr} *"),
        6 => format!("{expr} *"),
        _ => expr.to_owned(), // 7-field or invalid -- let the parser handle it
    }
}

/// Add a new cron job.
pub fn cron_add(
    name: String,
    schedule: String,
    prompt: String,
    _config: &Config,
) -> anyhow::Result<()> {
    // Normalize to 7-field format required by the cron crate.
    let normalized = normalize_cron_expr(&schedule);

    // Validate the cron expression.
    cron::Schedule::from_str(&normalized)
        .map_err(|e| anyhow::anyhow!("Invalid cron expression: {e}"))?;

    let path = cron_store_path();
    migrate_legacy_store(&path);

    let job_id = generate_job_id();
    let now_ms = Utc::now().timestamp_millis();

    let job = CronJob {
        id: job_id.clone(),
        name: name.clone(),
        enabled: true,
        schedule: CronSchedule {
            kind: ScheduleKind::Cron,
            at_ms: None,
            every_ms: None,
            expr: Some(normalized),
            tz: Some("UTC".into()),
        },
        payload: CronPayload {
            message: prompt,
            ..Default::default()
        },
        state: CronJobState::default(),
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        delete_after_run: false,
    };

    clawft_services::cron_service::storage::append_create_sync(&path, &job)
        .map_err(|e| anyhow::anyhow!("failed to write cron store: {e}"))?;

    println!("Cron job '{name}' created with ID: {job_id}");
    Ok(())
}

/// Remove a cron job by ID.
pub fn cron_remove(job_id: String, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    migrate_legacy_store(&path);
    let jobs = load_jobs(&path)?;

    if !jobs.iter().any(|j| j.id == job_id) {
        anyhow::bail!("cron job not found: {job_id}");
    }

    clawft_services::cron_service::storage::append_delete_sync(&path, &job_id)
        .map_err(|e| anyhow::anyhow!("failed to write cron store: {e}"))?;

    println!("Cron job '{job_id}' removed.");
    Ok(())
}

/// Enable or disable a cron job.
pub fn cron_enable(job_id: String, enabled: bool, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    migrate_legacy_store(&path);
    let jobs = load_jobs(&path)?;

    if !jobs.iter().any(|j| j.id == job_id) {
        anyhow::bail!("cron job not found: {job_id}");
    }

    clawft_services::cron_service::storage::append_update_sync(
        &path,
        &job_id,
        "enabled",
        &serde_json::json!(enabled),
    )
    .map_err(|e| anyhow::anyhow!("failed to write cron store: {e}"))?;

    let state = if enabled { "enabled" } else { "disabled" };
    println!("Cron job '{job_id}' {state}.");
    Ok(())
}

/// Manually trigger a cron job.
pub fn cron_run(job_id: String, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    migrate_legacy_store(&path);
    let jobs = load_jobs(&path)?;

    let job = jobs
        .iter()
        .find(|j| j.id == job_id)
        .ok_or_else(|| anyhow::anyhow!("cron job not found: {job_id}"))?;

    println!("Triggering cron job '{}' ({})", job.name, job.id);
    println!("  Schedule: {:?}", job.schedule.kind);
    if let Some(ref expr) = job.schedule.expr {
        println!("  Expression: {expr}");
    }
    println!("  Prompt: {}", job.payload.message);
    println!();
    // Placeholder: actual job execution is wired in the integration phase.
    println!("[Cron job execution not yet wired -- see integration task]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_ts_none() {
        assert_eq!(format_ts(None), "-");
    }

    #[test]
    fn format_ts_valid() {
        // 2023-11-14 22:13:20 UTC = 1700000000000ms
        let result = format_ts(Some(1_700_000_000_000));
        assert!(result.contains("2023"));
        assert!(result.contains("22:13:20"));
    }

    #[test]
    fn format_ts_zero() {
        let result = format_ts(Some(0));
        assert!(result.contains("1970"));
    }

    #[test]
    fn generate_job_id_format() {
        let id = generate_job_id();
        assert!(id.starts_with("job-"));
        // UUID v4 format: job-xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        assert_eq!(id.len(), 4 + 36); // "job-" + UUID
    }

    #[test]
    fn generate_job_id_unique() {
        let id1 = generate_job_id();
        let id2 = generate_job_id();
        assert_ne!(id1, id2, "UUID-based IDs must be unique");
    }

    #[test]
    fn cron_store_path_returns_something() {
        let path = cron_store_path();
        assert!(path.to_string_lossy().contains("cron.jsonl"));
    }

    #[test]
    fn normalize_cron_expr_5_field() {
        // 5-field -> 7-field: prepend "0" seconds, append "*" year.
        let result = normalize_cron_expr("9 * * * Mon-Fri");
        assert_eq!(result, "0 9 * * * Mon-Fri *");
    }

    #[test]
    fn normalize_cron_expr_6_field() {
        // 6-field -> 7-field: append "*" year.
        let result = normalize_cron_expr("0 9 * * * Mon-Fri");
        assert_eq!(result, "0 9 * * * Mon-Fri *");
    }

    #[test]
    fn normalize_cron_expr_7_field() {
        // 7-field: pass through unchanged.
        let result = normalize_cron_expr("0 9 * * * Mon-Fri 2025");
        assert_eq!(result, "0 9 * * * Mon-Fri 2025");
    }

    #[test]
    fn cron_add_with_valid_expression_5_field() {
        // Standard 5-field cron expression, normalized to 7-field.
        let normalized = normalize_cron_expr("0 9 * * Mon-Fri");
        let result = cron::Schedule::from_str(&normalized);
        assert!(result.is_ok(), "Expected valid schedule, got: {result:?}");
    }

    #[test]
    fn cron_add_with_valid_expression_7_field() {
        // Already 7-field expression.
        let result = cron::Schedule::from_str("0 0 9 * * Mon-Fri *");
        assert!(result.is_ok(), "Expected valid schedule, got: {result:?}");
    }

    #[test]
    fn cron_add_with_invalid_expression() {
        let normalized = normalize_cron_expr("not a cron expression");
        let result = cron::Schedule::from_str(&normalized);
        assert!(result.is_err());
    }

    #[test]
    fn cron_list_with_empty_store() {
        // Smoke test: should not panic.
        let config = Config::default();
        let _ = cron_list(&config);
    }

    #[test]
    fn jsonl_roundtrip_via_sync_helpers() {
        let dir = std::env::temp_dir().join(format!(
            "clawft-cron-cli-test-{}",
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("cron.jsonl");

        let now_ms = Utc::now().timestamp_millis();
        let job = CronJob {
            id: "test-rt-1".into(),
            name: "roundtrip".into(),
            enabled: true,
            schedule: CronSchedule {
                kind: ScheduleKind::Cron,
                at_ms: None,
                every_ms: None,
                expr: Some("0 9 * * *".into()),
                tz: Some("UTC".into()),
            },
            payload: CronPayload::default(),
            state: CronJobState::default(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            delete_after_run: false,
        };

        clawft_services::cron_service::storage::append_create_sync(&path, &job).unwrap();
        let loaded = load_jobs(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-rt-1");
        assert_eq!(loaded[0].name, "roundtrip");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn enable_disable_via_jsonl() {
        let dir = std::env::temp_dir().join(format!(
            "clawft-cron-cli-test-{}",
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("cron.jsonl");

        let now_ms = Utc::now().timestamp_millis();
        let job = CronJob {
            id: "j1".into(),
            name: "test".into(),
            enabled: true,
            schedule: CronSchedule::default(),
            payload: CronPayload::default(),
            state: CronJobState::default(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            delete_after_run: false,
        };

        clawft_services::cron_service::storage::append_create_sync(&path, &job).unwrap();
        clawft_services::cron_service::storage::append_update_sync(
            &path,
            "j1",
            "enabled",
            &serde_json::json!(false),
        )
        .unwrap();

        let loaded = load_jobs(&path).unwrap();
        assert!(!loaded[0].enabled);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_via_jsonl() {
        let dir = std::env::temp_dir().join(format!(
            "clawft-cron-cli-test-{}",
            uuid::Uuid::new_v4()
        ));
        let path = dir.join("cron.jsonl");

        let now_ms = Utc::now().timestamp_millis();
        let make = |id: &str, name: &str| CronJob {
            id: id.into(),
            name: name.into(),
            enabled: true,
            schedule: CronSchedule::default(),
            payload: CronPayload::default(),
            state: CronJobState::default(),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            delete_after_run: false,
        };

        clawft_services::cron_service::storage::append_create_sync(&path, &make("j1", "a"))
            .unwrap();
        clawft_services::cron_service::storage::append_create_sync(&path, &make("j2", "b"))
            .unwrap();
        clawft_services::cron_service::storage::append_delete_sync(&path, "j1").unwrap();

        let loaded = load_jobs(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "j2");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
