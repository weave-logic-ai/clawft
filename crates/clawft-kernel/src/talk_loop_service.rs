//! `SystemService` impl + the native async run loop for the
//! [`TalkModeLoop`](super::TalkModeLoop). Split from `talk_loop.rs` to stay
//! under the 500-line ceiling (CLAUDE.md). A child module can reach the parent's
//! private fields, so the impl still touches `self.tick` directly.

use std::sync::Arc;

use async_trait::async_trait;

use super::TalkModeLoop;
use crate::health::HealthStatus;
use crate::service::{ServiceType, SystemService};

#[async_trait]
impl SystemService for TalkModeLoop {
    fn name(&self) -> &str {
        "voice.talk_mode"
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Core
    }

    async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.tick.set_running(true);
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.tick.set_running(false);
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.tick.is_running() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded("talk-mode tick not running".into())
        }
    }
}

/// Drive the Talk-Mode loop on the ADR-047 self-calibrating cadence until
/// `cancel` fires or the tick is stopped. Modeled on
/// [`run_democritus_loop`](crate::cognitive_tick::run_democritus_loop) but for
/// turn-taking. Spawned by the daemon (Phase 6). Native only (tokio/tokio-util).
#[cfg(feature = "native")]
pub async fn run_talk_loop(loop_: Arc<TalkModeLoop>, cancel: tokio_util::sync::CancellationToken) {
    loop_.tick.set_running(true);
    tracing::info!("Talk-Mode loop started");
    loop {
        let interval_ms = loop_.tick.current_interval_ms();
        if interval_ms == 0 {
            tracing::warn!("Talk-Mode loop: tick interval is 0, exiting");
            break;
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms as u64)) => {}
        }
        if !loop_.tick.is_running() {
            break;
        }
        loop_.tick();
    }
    loop_.tick.set_running(false);
    tracing::info!("Talk-Mode loop exited");
}
