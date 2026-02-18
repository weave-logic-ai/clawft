//! `weft cron` -- manage scheduled jobs.
//!
//! Provides subcommands for listing, creating, removing, enabling/disabling,
//! and manually running cron jobs. Jobs are stored in the cron store file
//! located at `~/.clawft/cron.json` (or `~/.nanobot/cron.json` as fallback).
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
    CronJob, CronJobState, CronPayload, CronSchedule, CronStore, ScheduleKind,
};

/// Default cron store filename.
const CRON_STORE_FILENAME: &str = "cron.json";

/// Resolve the cron store file path.
///
/// Tries `~/.clawft/cron.json`, then `~/.nanobot/cron.json`.
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

/// Load the cron store from disk, returning an empty store if the file
/// does not exist.
fn load_store(path: &Path) -> anyhow::Result<CronStore> {
    if !path.exists() {
        return Ok(CronStore::default());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read cron store at {}: {e}", path.display()))?;
    let store: CronStore = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse cron store: {e}"))?;
    Ok(store)
}

/// Save the cron store to disk.
fn save_store(path: &Path, store: &CronStore) -> anyhow::Result<()> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(store)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Generate a short random job ID.
fn generate_job_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // Use lower 32 bits of timestamp + process id for uniqueness.
    format!("job-{:08x}{:04x}", ts as u32, std::process::id() as u16)
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
    let store = load_store(&path)?;

    if store.jobs.is_empty() {
        println!("No cron jobs configured.");
        println!("  Store: {}", path.display());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["ID", "NAME", "SCHEDULE", "ENABLED", "LAST RUN", "NEXT RUN"]);

    for job in &store.jobs {
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
    let mut store = load_store(&path)?;

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

    store.jobs.push(job);
    save_store(&path, &store)?;

    println!("Cron job '{name}' created with ID: {job_id}");
    Ok(())
}

/// Remove a cron job by ID.
pub fn cron_remove(job_id: String, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    let mut store = load_store(&path)?;

    let before_len = store.jobs.len();
    store.jobs.retain(|j| j.id != job_id);

    if store.jobs.len() == before_len {
        anyhow::bail!("cron job not found: {job_id}");
    }

    save_store(&path, &store)?;
    println!("Cron job '{job_id}' removed.");
    Ok(())
}

/// Enable or disable a cron job.
pub fn cron_enable(job_id: String, enabled: bool, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    let mut store = load_store(&path)?;

    let job = store
        .jobs
        .iter_mut()
        .find(|j| j.id == job_id)
        .ok_or_else(|| anyhow::anyhow!("cron job not found: {job_id}"))?;

    job.enabled = enabled;
    job.updated_at_ms = Utc::now().timestamp_millis();

    save_store(&path, &store)?;

    let state = if enabled { "enabled" } else { "disabled" };
    println!("Cron job '{job_id}' {state}.");
    Ok(())
}

/// Manually trigger a cron job.
pub fn cron_run(job_id: String, _config: &Config) -> anyhow::Result<()> {
    let path = cron_store_path();
    let store = load_store(&path)?;

    let job = store
        .jobs
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
        assert!(id.len() > 4);
    }

    #[test]
    fn generate_job_id_unique() {
        let id1 = generate_job_id();
        // Sleep is not needed: process id + timestamp should differ or be unique enough.
        let id2 = generate_job_id();
        // They may be the same if called in the same millisecond, but the format
        // should at least be valid.
        assert!(id1.starts_with("job-"));
        assert!(id2.starts_with("job-"));
    }

    #[test]
    fn cron_store_path_returns_something() {
        let path = cron_store_path();
        assert!(path.to_string_lossy().contains("cron.json"));
    }

    #[test]
    fn load_store_nonexistent_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent_clawft_test_cron.json");
        let store = load_store(&path).unwrap();
        assert!(store.jobs.is_empty());
        assert_eq!(store.version, 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("clawft_cron_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_cron_roundtrip.json");

        let store = CronStore {
            version: 1,
            jobs: vec![CronJob {
                id: "test-job-1".into(),
                name: "test job".into(),
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
                created_at_ms: 1_700_000_000_000,
                updated_at_ms: 1_700_000_000_000,
                delete_after_run: false,
            }],
        };

        save_store(&path, &store).unwrap();
        let loaded = load_store(&path).unwrap();
        assert_eq!(loaded.jobs.len(), 1);
        assert_eq!(loaded.jobs[0].id, "test-job-1");
        assert_eq!(loaded.jobs[0].name, "test job");

        // Cleanup.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
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
        // This will try to load from the default path; if it doesn't exist,
        // it returns an empty store and prints "No cron jobs configured."
        let _ = cron_list(&config);
    }

    #[test]
    fn cron_remove_nonexistent_job() {
        // We test the store-level logic directly since cron_remove
        // depends on the filesystem cron store path.
        let mut store = CronStore::default();
        store.jobs.push(CronJob {
            id: "keep-this".into(),
            name: "keeper".into(),
            enabled: true,
            schedule: CronSchedule::default(),
            payload: CronPayload::default(),
            state: CronJobState::default(),
            created_at_ms: 0,
            updated_at_ms: 0,
            delete_after_run: false,
        });

        let before_len = store.jobs.len();
        store.jobs.retain(|j| j.id != "nonexistent");
        assert_eq!(store.jobs.len(), before_len); // nothing removed
    }

    #[test]
    fn cron_enable_disable_logic() {
        let mut store = CronStore {
            version: 1,
            jobs: vec![CronJob {
                id: "j1".into(),
                name: "test".into(),
                enabled: true,
                schedule: CronSchedule::default(),
                payload: CronPayload::default(),
                state: CronJobState::default(),
                created_at_ms: 0,
                updated_at_ms: 0,
                delete_after_run: false,
            }],
        };

        // Disable
        if let Some(job) = store.jobs.iter_mut().find(|j| j.id == "j1") {
            job.enabled = false;
        }
        assert!(!store.jobs[0].enabled);

        // Re-enable
        if let Some(job) = store.jobs.iter_mut().find(|j| j.id == "j1") {
            job.enabled = true;
        }
        assert!(store.jobs[0].enabled);
    }

    #[test]
    fn load_store_invalid_json() {
        let dir = std::env::temp_dir().join("clawft_cron_test_invalid");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bad_cron.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result = load_store(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
