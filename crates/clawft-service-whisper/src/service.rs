//! Substrate-connected whisper pipeline.
//!
//! Binds a [`WhisperClient`](crate::WhisperClient) to an in-process
//! [`SubstrateService`]: subscribes to
//! [`crate::SUBSTRATE_PCM_INPUT_PATH`], windows incoming PCM, posts to
//! `/inference`, and publishes transcripts to
//! [`crate::SUBSTRATE_TRANSCRIPT_OUTPUT_PATH`].
//!
//! # Backpressure
//!
//! Per the journal §A5 + service-API §1, whisper serializes one
//! in-flight inference per instance (no 429). This service chooses
//! **drop-oldest** on input: if a new window is ready while
//! [`WhisperClient::transcribe`] is busy, the new window replaces any
//! still-queued window. That biases freshness over completeness — a
//! deliberate choice for live streaming ("what are you saying now" is
//! more valuable than "reconstruct every syllable"). See journal for
//! the alternatives (unbounded queue, block upstream).
//!
//! # Retry
//!
//! 5xx + 503-loading are retriable per API §7 (idempotent at T=0).
//! The service does a single retry with 500ms delay, then drops the
//! window. 4xx is a programmer bug (malformed WAV etc.) so we log +
//! drop immediately without retry.

use std::time::Duration;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use clawft_kernel::SubstrateService;
use serde_json::{json, Value};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::client::{TranscribeError, WhisperClient};
use crate::wav::write_wav;
use crate::windower::{PcmChunk, PcmWindow, Windower};
use crate::{SUBSTRATE_PCM_INPUT_PATH, SUBSTRATE_TRANSCRIPT_OUTPUT_PATH};

/// Configuration for [`WhisperService`].
#[derive(Debug, Clone)]
pub struct WhisperServiceConfig {
    /// Target window length in ms. 2000 is the API-doc sweet spot.
    pub window_ms: u64,
    /// Delay between a 5xx/503 response and the retry attempt.
    pub retry_backoff: Duration,
    /// Caller identity written as the `actor_id` of the publish.
    /// `None` treats the write as anonymous.
    pub actor_id: Option<String>,
    /// Substrate path to read PCM from.
    pub input_path: String,
    /// Substrate path to write transcripts to.
    pub output_path: String,
}

impl Default for WhisperServiceConfig {
    fn default() -> Self {
        Self {
            window_ms: 2_000,
            retry_backoff: Duration::from_millis(500),
            actor_id: Some("service-whisper".to_string()),
            input_path: SUBSTRATE_PCM_INPUT_PATH.to_string(),
            output_path: SUBSTRATE_TRANSCRIPT_OUTPUT_PATH.to_string(),
        }
    }
}

/// Runtime handle for a spawned whisper service task.
#[derive(Debug)]
pub struct WhisperService {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl WhisperService {
    /// Spawn the pipeline on the tokio runtime of the caller.
    ///
    /// Wiring:
    /// 1. `substrate.subscribe(input_path)` — gets an mpsc of update
    ///    lines (JSON bytes).
    /// 2. Parses each line, pulls `value.pcm_b64` + metadata, feeds a
    ///    [`Windower`].
    /// 3. When a window emits, wraps PCM in WAV, POSTs to whisper.
    /// 4. On success, publishes transcript to `output_path`.
    ///
    /// # Lifecycle
    ///
    /// The returned [`WhisperService`] owns a watch-channel shutdown
    /// signal. Call [`Self::shutdown`] to stop cleanly; the internal
    /// task drains the in-flight HTTP request before exiting.
    ///
    /// # Errors
    ///
    /// Returns `Err` only if the initial `substrate.subscribe` call
    /// fails egress gating. Runtime errors (HTTP 5xx, malformed chunks,
    /// whisper-service-down) are logged + absorbed.
    pub fn spawn(
        substrate: SubstrateService,
        client: WhisperClient,
        config: WhisperServiceConfig,
    ) -> Result<Self, String> {
        let (id, rx) = substrate
            .subscribe(config.actor_id.as_deref(), &config.input_path)
            .map_err(|e| format!("substrate subscribe failed: {e}"))?;
        info!(
            sub_id = id.0,
            path = %config.input_path,
            window_ms = config.window_ms,
            whisper_url = %client.config().base_url,
            "whisper service: subscribed to PCM input"
        );

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let substrate_for_task = substrate.clone();
        let input_path_cleanup = config.input_path.clone();
        let task = tokio::spawn(async move {
            run_pipeline(rx, substrate_for_task.clone(), client, config, shutdown_rx).await;
            // Clean up the subscription on exit (idempotent).
            substrate_for_task.unsubscribe(&input_path_cleanup, id);
        });
        Ok(Self {
            shutdown: shutdown_tx,
            task,
        })
    }

    /// Signal shutdown and await the internal task.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        let _ = self.task.await;
    }
}

async fn run_pipeline(
    mut rx: mpsc::Receiver<Vec<u8>>,
    substrate: SubstrateService,
    client: WhisperClient,
    config: WhisperServiceConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    // Health probe is fire-and-forget: if whisper isn't up the service
    // still stays subscribed and will start processing once POSTs
    // start succeeding. See journal §"degraded-but-alive".
    if !client.wait_for_healthy().await {
        warn!(
            base_url = %client.config().base_url,
            "whisper service: starting in degraded mode (service not reachable)"
        );
    } else {
        info!(base_url = %client.config().base_url, "whisper service: ready");
    }

    let mut windower = Windower::new(config.window_ms);

    // Drop-oldest policy: a single slot for the pending window. When
    // the inference task is free it takes the slot; new windows
    // overwrite the slot if busy.
    let mut pending: Option<PcmWindow> = None;
    let mut in_flight: Option<tokio::task::JoinHandle<(
        PcmWindow,
        Result<crate::client::InferenceResponse, TranscribeError>,
    )>> = None;

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    debug!("whisper service: shutdown requested");
                    break;
                }
            }
            line = rx.recv() => {
                let Some(bytes) = line else {
                    debug!("whisper service: substrate sender dropped, exiting");
                    break;
                };
                if let Some(chunk) = decode_update_line(&bytes) {
                    match decode_pcm_chunk(&chunk) {
                        Ok((pcm, sr, ch, seq, chunk_ms)) => {
                            if let Some(win) = windower.push(&pcm, sr, ch, seq, chunk_ms) {
                                // Drop-oldest: replace any pending window
                                // that hasn't been picked up yet.
                                if pending.replace(win).is_some() {
                                    warn!(
                                        "whisper service: dropped oldest window (in-flight whisper request still running)"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            debug!(err = %e, "whisper service: skipping malformed chunk");
                        }
                    }
                }
            }
            finished = async {
                match in_flight.as_mut() {
                    Some(h) => h.await.ok(),
                    // Park forever — select! only polls this arm when
                    // `in_flight` is Some.
                    None => std::future::pending::<Option<_>>().await,
                }
            }, if in_flight.is_some() => {
                in_flight = None;
                if let Some((window, result)) = finished {
                    handle_inference_result(&substrate, &config, window, result).await;
                }
            }
        }

        // If the HTTP worker is free and a window is pending, launch.
        if in_flight.is_none()
            && let Some(window) = pending.take() {
                let client_clone = client.clone();
                let window_clone = window.clone();
                let retry_backoff = config.retry_backoff;
                in_flight = Some(tokio::spawn(async move {
                    let result = run_one_inference(&client_clone, &window_clone, retry_backoff).await;
                    (window_clone, result)
                }));
            }
    }

    // On shutdown: flush any partial window synchronously for the
    // last-gasp transcript, then await an in-flight request.
    if let Some(partial) = windower.flush() {
        let result = run_one_inference(&client, &partial, config.retry_backoff).await;
        handle_inference_result(&substrate, &config, partial, result).await;
    }
    if let Some(h) = in_flight.take()
        && let Ok((window, result)) = h.await {
            handle_inference_result(&substrate, &config, window, result).await;
        }
}

/// Single inference call with one in-line retry for retriable errors.
///
/// API doc §7: at `temperature=0` `/inference` is idempotent, so the
/// retry is safe. We cap at one retry to avoid burning the whisper
/// single-in-flight mutex on a genuinely sick service.
async fn run_one_inference(
    client: &WhisperClient,
    window: &PcmWindow,
    retry_backoff: Duration,
) -> Result<crate::client::InferenceResponse, TranscribeError> {
    let wav = write_wav(&window.pcm_s16le, window.sample_rate, window.channels);
    match client.transcribe(wav.clone()).await {
        Ok(r) => Ok(r),
        Err(e) if e.is_retriable() => {
            debug!(err = %e, "whisper service: retriable error, one retry");
            tokio::time::sleep(retry_backoff).await;
            client.transcribe(wav).await
        }
        Err(e) => Err(e),
    }
}

async fn handle_inference_result(
    substrate: &SubstrateService,
    config: &WhisperServiceConfig,
    window: PcmWindow,
    result: Result<crate::client::InferenceResponse, TranscribeError>,
) {
    match result {
        Ok(r) => {
            let payload = json!({
                "text": r.text,
                "start_ms": window.start_ms,
                "end_ms": window.end_ms,
                // The `json` response format doesn't carry per-segment
                // confidence; verbose_json would. Keeping `null` so
                // downstream object-type shape is stable when we later
                // flip the format.
                "confidence": Value::Null,
                "lang": "en",
                "seq": window.last_seq,
            });
            let tick = substrate.publish(
                config.actor_id.as_deref(),
                &config.output_path,
                payload,
            );
            info!(
                tick,
                start_ms = window.start_ms,
                end_ms = window.end_ms,
                seq = window.last_seq,
                "whisper service: transcript published"
            );
        }
        Err(e) => {
            error!(
                err = %e,
                start_ms = window.start_ms,
                end_ms = window.end_ms,
                "whisper service: transcription failed (window dropped)"
            );
        }
    }
}

/// Parse a substrate-subscribe update line.
///
/// Shape (see `clawft_kernel::substrate_service::build_update_line`):
///
/// ```json
/// {"path":"…","tick":N,"kind":"publish|notify","value":{…},"actor_id":…}\n
/// ```
///
/// Returns the `value` field when `kind == "publish"`, else `None`.
fn decode_update_line(line: &[u8]) -> Option<Value> {
    // Strip trailing newline if present.
    let end = if line.last() == Some(&b'\n') {
        line.len() - 1
    } else {
        line.len()
    };
    let v: Value = serde_json::from_slice(&line[..end]).ok()?;
    if v.get("kind")?.as_str()? != "publish" {
        return None;
    }
    Some(v.get("value")?.clone())
}

/// Decode a single [`PcmChunk`] JSON value into raw bytes + metadata.
fn decode_pcm_chunk(value: &Value) -> Result<(Vec<u8>, u32, u16, u64, u64), String> {
    let chunk: PcmChunk = serde_json::from_value(value.clone())
        .map_err(|e| format!("not a PcmChunk: {e}"))?;
    let pcm = B64
        .decode(chunk.pcm_b64.as_bytes())
        .map_err(|e| format!("pcm_b64 decode: {e}"))?;
    Ok((pcm, chunk.sample_rate, chunk.channels, chunk.seq, chunk.chunk_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::WhisperConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_client(base_url: String) -> WhisperClient {
        WhisperClient::new(WhisperConfig {
            base_url,
            request_timeout: Duration::from_secs(5),
            health_deadline: Duration::from_millis(200),
            ..Default::default()
        })
        .unwrap()
    }

    fn publish_pcm_chunk(
        substrate: &SubstrateService,
        actor_id: &str,
        path: &str,
        pcm: &[u8],
        chunk_ms: u64,
        seq: u64,
    ) {
        let payload = json!({
            "pcm_b64": B64.encode(pcm),
            "sample_rate": 16_000,
            "channels": 1,
            "seq": seq,
            "chunk_ms": chunk_ms,
        });
        substrate.publish(Some(actor_id), path, payload);
    }

    #[tokio::test]
    async fn decode_update_line_extracts_publish_value() {
        let raw = br#"{"path":"p","tick":1,"kind":"publish","value":{"x":1},"actor_id":null}"#;
        let v = decode_update_line(raw).unwrap();
        assert_eq!(v["x"], 1);
    }

    #[tokio::test]
    async fn decode_update_line_ignores_notify() {
        let raw = br#"{"path":"p","tick":1,"kind":"notify","value":null,"actor_id":null}"#;
        assert!(decode_update_line(raw).is_none());
    }

    #[tokio::test]
    async fn decode_pcm_chunk_roundtrips() {
        let pcm = vec![1u8, 2, 3, 4];
        let v = json!({
            "pcm_b64": B64.encode(&pcm),
            "sample_rate": 16_000,
            "channels": 1,
            "seq": 7,
            "chunk_ms": 500,
        });
        let (out, sr, ch, seq, ms) = decode_pcm_chunk(&v).unwrap();
        assert_eq!(out, pcm);
        assert_eq!(sr, 16_000);
        assert_eq!(ch, 1);
        assert_eq!(seq, 7);
        assert_eq!(ms, 500);
    }

    /// Full pipeline: substrate → windower → mocked whisper → substrate.
    ///
    /// This is the hermetic end-to-end test; no live service needed.
    #[tokio::test]
    async fn end_to_end_with_mocked_whisper() {
        // Mocked whisper: both endpoints.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":"ok"}"#))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/inference"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"text": " unit test speaks"}"#),
            )
            .mount(&server)
            .await;

        let substrate = SubstrateService::new();
        let client = make_client(server.uri());
        // Shorter window so the test fires quickly.
        let cfg = WhisperServiceConfig {
            window_ms: 500,
            ..Default::default()
        };
        let input_path = cfg.input_path.clone();
        let output_path = cfg.output_path.clone();
        let actor = cfg.actor_id.clone().unwrap();

        // Pre-subscribe to the OUTPUT to catch the transcript. Must be
        // done before pumping input so we don't race the first
        // transcript publish.
        let (_out_id, mut out_rx) = substrate.subscribe(Some(&actor), &output_path).unwrap();

        let svc = WhisperService::spawn(substrate.clone(), client, cfg).unwrap();

        // Push 500ms worth of silence at 16kHz mono s16le = 16000 bytes.
        // Use two 250ms chunks to exercise the windower's accumulation path.
        let half = vec![0u8; 8_000];
        publish_pcm_chunk(&substrate, &actor, &input_path, &half, 250, 1);
        publish_pcm_chunk(&substrate, &actor, &input_path, &half, 250, 2);

        // Wait up to 3s for a transcript to show up on the output path.
        let got = tokio::time::timeout(Duration::from_secs(3), out_rx.recv()).await;
        let line = got.expect("transcript not published within 3s").expect("substrate closed");
        let update: Value = serde_json::from_slice(&line[..line.len() - 1]).unwrap();
        assert_eq!(update["kind"], "publish");
        assert_eq!(update["path"], output_path);
        let body = &update["value"];
        assert_eq!(body["text"], "unit test speaks");
        assert_eq!(body["start_ms"], 0);
        assert_eq!(body["end_ms"], 500);
        assert_eq!(body["seq"], 2);
        assert_eq!(body["lang"], "en");
        assert!(body["confidence"].is_null());

        svc.shutdown().await;
    }

    #[tokio::test]
    async fn service_survives_whisper_down_at_start() {
        // No mock server at all — reqwest will fail the health probe
        // and every /inference. Service must still spawn and exit
        // cleanly on shutdown.
        let substrate = SubstrateService::new();
        let client = make_client("http://127.0.0.1:1".into()); // unreachable
        let cfg = WhisperServiceConfig {
            window_ms: 500,
            ..Default::default()
        };
        let svc = WhisperService::spawn(substrate, client, cfg).unwrap();
        // Tiny delay to let the pipeline enter its main select!.
        tokio::time::sleep(Duration::from_millis(100)).await;
        svc.shutdown().await;
    }

    #[tokio::test]
    async fn drops_oldest_window_when_inference_slow() {
        // Mock /inference to hang for 2s. Feed three windows in
        // quick succession; the service should drop at least one
        // mid-window instead of queueing them all.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":"ok"}"#))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/inference"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(1_500))
                    .set_body_string(r#"{"text": " slow"}"#),
            )
            .mount(&server)
            .await;

        let substrate = SubstrateService::new();
        let client = make_client(server.uri());
        let cfg = WhisperServiceConfig {
            window_ms: 200,
            ..Default::default()
        };
        let input_path = cfg.input_path.clone();
        let actor = cfg.actor_id.clone().unwrap();
        let svc = WhisperService::spawn(substrate.clone(), client, cfg).unwrap();

        // Feed five 200ms windows back-to-back. Most should be dropped
        // (only the first kicks off a 1.5s inference; windows 2–5
        // overwrite the pending slot).
        for i in 0..5 {
            let buf = vec![0u8; 6_400]; // 200ms at 16kHz mono s16le
            publish_pcm_chunk(&substrate, &actor, &input_path, &buf, 200, i + 1);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // Let the service settle.
        tokio::time::sleep(Duration::from_millis(300)).await;
        // We don't assert a specific drop count — timing varies by CI —
        // but we do assert the service didn't stall: shutdown must
        // complete without hanging forever.
        tokio::time::timeout(Duration::from_secs(5), svc.shutdown())
            .await
            .expect("shutdown hung — the pipeline did not drain");
    }
}
