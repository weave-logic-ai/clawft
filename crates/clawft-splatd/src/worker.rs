//! The single pipeline worker — pulls job ids off the queue, runs the
//! pipeline, fires the completion callback (shasta-os-020 `awaitArtifact`).

use crate::state::SharedState;
use clawft_splat_pipeline::{JobStatus, detect_job_input, run_job};
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

pub async fn worker_loop(state: SharedState, mut rx: mpsc::UnboundedReceiver<String>) {
    while let Some(job_id) = rx.recv().await {
        state.queue_depth.fetch_sub(1, Ordering::Relaxed);
        let dirs = state.job_dirs(&job_id);

        let Some(mut rec) = state.jobs.lock().await.get(&job_id).cloned() else {
            tracing::warn!(job = %job_id, "queued job missing from index, skipping");
            continue;
        };

        // Input was placed at ingest time: `input.*` video or `session/` image-set.
        let input = match detect_job_input(&dirs.root) {
            Some(i) => i,
            None => {
                rec.status = JobStatus::Failed;
                rec.error = Some(
                    "no video (input.*) or image-set session/ in job workspace".into(),
                );
                let _ = rec.save(&dirs.root);
                state.jobs.lock().await.insert(job_id.clone(), rec);
                state.jobs_failed.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        tracing::info!(
            job = %job_id,
            kind = ?input.kind(),
            input = %input.path().display(),
            "job started"
        );
        run_job(&state.cfg, &dirs, &mut rec, &input).await;

        match rec.status {
            JobStatus::Done => state.jobs_done.fetch_add(1, Ordering::Relaxed),
            _ => state.jobs_failed.fetch_add(1, Ordering::Relaxed),
        };

        if let Some(url) = rec.callback_url.clone() {
            fire_callback(&url, &rec).await;
        }

        state.jobs.lock().await.insert(job_id.clone(), rec);
        tracing::info!(job = %job_id, "job finished");
    }
}

/// POST the terminal JobRecord to the callback. Best-effort with small
/// retries — the consumer can always fall back to polling GET /v1/jobs/{id}.
async fn fire_callback(url: &str, rec: &clawft_splat_pipeline::JobRecord) {
    let client = reqwest::Client::new();
    for attempt in 1..=3u32 {
        match client.post(url).json(rec).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(%url, "callback delivered");
                return;
            }
            Ok(resp) => tracing::warn!(%url, status = %resp.status(), attempt, "callback rejected"),
            Err(e) => tracing::warn!(%url, error = %e, attempt, "callback failed"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
    }
    tracing::error!(%url, "callback undeliverable after 3 attempts (consumer should poll)");
}
