//! xAI Realtime WebSocket client (ADR-074 V0–V2 / WEFT-689 / WEFT-690 / WEFT-691).
//!
//! Feature-gated behind `voice-xai`.
//!
//! ## V0 (WEFT-689)
//!
//! - Connect to `wss://api.x.ai/v1/realtime?model=…` with `Authorization: Bearer`
//! - Send a minimal `session.update` handshake
//! - Observe a first server event (or timeout)
//! - Disconnect cleanly
//!
//! ## V1 tool bridge (WEFT-690)
//!
//! - Register WeftOS function tools on `session.update` ([`session_update_payload_with_tools`])
//! - Map `response.function_call_arguments.done` → [`crate::voice::xai_tool_bridge::WindowIntent`]
//! - See [`crate::voice::xai_tool_bridge`] for catalog, handlers, and reply events
//!
//! ## V2 hybrid + metrics + graceful local fallback (WEFT-691)
//!
//! - Pre-dial selection via [`crate::voice::xai_health::select_voice_transport`]
//! - On connect failure: [`connect_hello_or_fallback`] → local path + metrics
//! - Hybrid composition: local STT/agent + xAI TTS when healthy
//!
//! ## AEC (cloud S2S)
//!
//! Full-duplex cloud voice still requires **AEC in front of the mic** before
//! uplink frames are sent. See [`crate::voice::xai_tool_bridge::AEC_IN_FRONT_OF_MIC_NOTE`].
//! This module does not open audio devices; the capture path must inject
//! `clawft-voice-aec` (honest readiness: [`crate::voice::xai_tool_bridge::aec_pipeline_ready`]).
//!
//! # Security
//!
//! - Never log API keys or raw audio bytes.
//! - Prefer reading the key from env (name from [`clawft_types::config::VoiceXaiConfig`])
//!   and pass it only into the Authorization header.
//! - Shell-like tools are not registered by default; when enabled they still
//!   go through [`clawft_types::security::CommandPolicy`].

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{AUTHORIZATION, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

use clawft_types::config::{
    VoiceXaiConfig, DEFAULT_XAI_REALTIME_ENDPOINT, DEFAULT_XAI_REALTIME_MODEL,
};

use super::xai_tool_bridge::{
    aec_pipeline_ready, default_weftos_tools, AEC_IN_FRONT_OF_MIC_NOTE,
};

/// How long to wait for the WebSocket handshake + first useful event.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long to wait after `session.update` for any JSON server event.
pub const DEFAULT_HELLO_EVENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Errors from the xAI Realtime V0 client.
#[derive(Debug, Error)]
pub enum XaiRealtimeError {
    #[error("missing xAI API key (do not log key material)")]
    MissingApiKey,

    #[error("invalid realtime URL: {0}")]
    InvalidUrl(String),

    #[error("failed to build WebSocket request: {0}")]
    RequestBuild(String),

    #[error("WebSocket connect failed: {0}")]
    Connect(String),

    #[error("WebSocket send failed: {0}")]
    Send(String),

    #[error("WebSocket receive failed: {0}")]
    Receive(String),

    #[error("timed out waiting for Realtime handshake / hello event")]
    Timeout,

    #[error("server closed the connection before hello completed")]
    ClosedEarly,

    #[error("server returned error event: {0}")]
    ServerError(String),
}

/// Connection parameters for a V0 hello session.
///
/// Build from [`VoiceXaiConfig`] + an API key obtained outside this type
/// (so the key is never stored on config structs that get logged).
#[derive(Debug, Clone)]
pub struct XaiRealtimeConnectParams {
    /// Full WebSocket URL including `?model=…` (safe to log).
    pub url: String,
    /// Bearer token. **Never log.**
    pub api_key: String,
    /// Optional voice name for `session.update` (default `eve`).
    pub voice: String,
    /// Optional instructions for `session.update`.
    pub instructions: String,
    /// When true, attach V1 WeftOS WindowIntent tools on `session.update`.
    pub register_tools: bool,
    /// Connect timeout.
    pub connect_timeout: Duration,
    /// Wait for first server event after session.update.
    pub hello_timeout: Duration,
}

impl XaiRealtimeConnectParams {
    /// Build from ADR-074 xAI config + raw API key (not logged).
    ///
    /// Defaults `register_tools = true` (V1 conductor demo path).
    pub fn from_voice_xai(xai: &VoiceXaiConfig, api_key: impl Into<String>) -> Self {
        Self {
            url: xai.realtime_url(),
            api_key: api_key.into(),
            voice: "eve".into(),
            instructions: default_conductor_instructions().into(),
            register_tools: true,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            hello_timeout: DEFAULT_HELLO_EVENT_TIMEOUT,
        }
    }

    /// Defaults pointing at production xAI Realtime (still needs a key).
    pub fn with_defaults(api_key: impl Into<String>) -> Self {
        Self::from_voice_xai(&VoiceXaiConfig::default(), api_key)
    }

    /// Disable tool registration (V0-style hello only).
    pub fn without_tools(mut self) -> Self {
        self.register_tools = false;
        self
    }
}

/// Default system instructions for CNVS-like conductor demos.
pub fn default_conductor_instructions() -> &'static str {
    "You are the WeftOS voice conductor. When the user asks to spawn agents, \
     open panes, focus windows, or summarize text, call the provided tools \
     (spawn_agent, focus_pane, summarize). Prefer visible agent panes. \
     Do not invent unrestricted shell access."
}

/// Result of a successful V0 hello path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XaiRealtimeHelloResult {
    /// URL that was dialed (no credentials).
    pub url: String,
    /// First non-audio server event `type` string, if any (e.g. `session.updated`).
    pub first_event_type: Option<String>,
    /// True when we completed connect + session.update + clean close.
    pub completed: bool,
}

/// Minimal `session.update` payload (OpenAI-Realtime-compatible shape).
///
/// Safe to log — contains no secrets or audio. No tools (V0 shape).
pub fn session_update_payload(voice: &str, instructions: &str) -> Value {
    session_update_payload_with_tools(voice, instructions, None)
}

/// `session.update` with optional custom function tools (V1 tool bridge).
///
/// When `tools` is `Some` and non-empty, they are attached under `session.tools`.
/// Safe to log — no secrets or audio.
pub fn session_update_payload_with_tools(
    voice: &str,
    instructions: &str,
    tools: Option<&[Value]>,
) -> Value {
    let mut session = json!({
        "voice": voice,
        "instructions": instructions,
        "turn_detection": { "type": "server_vad" },
        "audio": {
            "input": { "format": { "type": "audio/pcm", "rate": 16000 } },
            "output": { "format": { "type": "audio/pcm", "rate": 16000 } }
        }
    });
    if let Some(tools) = tools {
        if !tools.is_empty() {
            session
                .as_object_mut()
                .expect("session object")
                .insert("tools".into(), Value::Array(tools.to_vec()));
        }
    }
    json!({
        "type": "session.update",
        "session": session
    })
}

/// V1 conductor `session.update`: default WeftOS tools (spawn/focus/summarize).
pub fn session_update_with_weftos_tools(voice: &str, instructions: &str) -> Value {
    let tools = default_weftos_tools();
    session_update_payload_with_tools(voice, instructions, Some(&tools))
}

/// Build a tungstenite client request with Bearer auth.
///
/// The Authorization header is set; callers must not `Debug` the request
/// in production logs (tungstenite Request may include headers).
pub fn build_realtime_request(
    url: &str,
    api_key: &str,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, XaiRealtimeError> {
    if api_key.trim().is_empty() {
        return Err(XaiRealtimeError::MissingApiKey);
    }
    if url.trim().is_empty() {
        return Err(XaiRealtimeError::InvalidUrl("empty URL".into()));
    }

    let mut request = url
        .into_client_request()
        .map_err(|e| XaiRealtimeError::RequestBuild(e.to_string()))?;

    let auth = format!("Bearer {api_key}");
    let header_val = HeaderValue::from_str(&auth)
        .map_err(|e| XaiRealtimeError::RequestBuild(format!("invalid Authorization header: {e}")))?;
    request.headers_mut().insert(AUTHORIZATION, header_val);

    Ok(request)
}

/// Connect → `session.update` → wait for first JSON event → disconnect.
///
/// Network I/O. Unit tests mock payload/request builders; live connect is
/// gated by `XAI_API_KEY` in an `#[ignore]` integration test.
///
/// When `params.register_tools` is true, V1 WeftOS tools are included on
/// `session.update`. Does not open a mic; callers must ensure AEC is in front
/// of any live audio uplink ([`AEC_IN_FRONT_OF_MIC_NOTE`]).
///
/// Never logs the API key or binary/audio frames (binary frames are counted only).
pub async fn connect_hello(
    params: XaiRealtimeConnectParams,
) -> Result<XaiRealtimeHelloResult, XaiRealtimeError> {
    let url = params.url.clone();
    info!(%url, "xAI Realtime: connecting (WEFT-689/690 hello)");

    if !aec_pipeline_ready() {
        // Honest notice: hello path has no mic; live Talk-Mode must wire AEC.
        debug!(note = AEC_IN_FRONT_OF_MIC_NOTE, "xAI Realtime: AEC pipeline not marked ready");
    }

    let request = build_realtime_request(&params.url, &params.api_key)?;

    let (ws_stream, _response) = timeout(params.connect_timeout, tokio_tungstenite::connect_async(request))
        .await
        .map_err(|_| XaiRealtimeError::Timeout)?
        .map_err(|e| XaiRealtimeError::Connect(sanitize_ws_error(&e.to_string())))?;

    info!(%url, "xAI Realtime: WebSocket connected");

    let (mut write, mut read) = ws_stream.split();

    let tools = if params.register_tools {
        Some(default_weftos_tools())
    } else {
        None
    };
    let payload = session_update_payload_with_tools(
        &params.voice,
        &params.instructions,
        tools.as_deref(),
    );
    let tool_count = tools.as_ref().map(|t| t.len()).unwrap_or(0);
    let payload_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("session.update");
    debug!(
        event_type = %payload_type,
        tool_count,
        "xAI Realtime: sending session.update"
    );

    write
        .send(Message::Text(payload.to_string().into()))
        .await
        .map_err(|e| XaiRealtimeError::Send(e.to_string()))?;

    let first_event_type = match timeout(params.hello_timeout, wait_first_json_event(&mut read)).await
    {
        Ok(Ok(t)) => {
            info!(event_type = %t, "xAI Realtime: hello event received");
            Some(t)
        }
        Ok(Err(XaiRealtimeError::ServerError(msg))) => {
            // Still try clean close; surface the error.
            let _ = write.close().await;
            return Err(XaiRealtimeError::ServerError(msg));
        }
        Ok(Err(e)) => {
            let _ = write.close().await;
            return Err(e);
        }
        Err(_) => {
            // Timeout waiting for event: still count as connected for V0 probe
            // (some servers may not emit until audio/text is sent).
            warn!("xAI Realtime: no server event within hello timeout; disconnecting cleanly");
            None
        }
    };

    // Clean disconnect.
    if let Err(e) = write.close().await {
        debug!(error = %e, "xAI Realtime: close frame send failed (ignored)");
    }
    info!(%url, "xAI Realtime: disconnected cleanly");

    Ok(XaiRealtimeHelloResult {
        url,
        first_event_type,
        completed: true,
    })
}

/// Outcome of starting an xAI session with graceful local fallback (WEFT-691).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XaiSessionStart {
    /// Realtime hello succeeded; use cloud path.
    Xai(XaiRealtimeHelloResult),
    /// Connect/session failed; caller must use local ADR-061 pipeline.
    LocalFallback {
        /// Selection decision (always local + runtime notice).
        decision: clawft_types::config::VoiceTransportDecision,
        /// Sanitized error class for metrics/logging (no secrets).
        error_kind: &'static str,
    },
}

impl XaiSessionStart {
    /// True when the session should use the local pipeline.
    pub fn is_local_fallback(&self) -> bool {
        matches!(self, Self::LocalFallback { .. })
    }
}

/// Attempt [`connect_hello`]; on any error, fail open to local with metrics.
///
/// Does not panic. Prefer this over bare `connect_hello` for Talk-Mode so
/// disconnects never leave the product without a voice path.
pub async fn connect_hello_or_fallback(
    params: XaiRealtimeConnectParams,
    preferred_mode: clawft_types::config::VoiceMode,
    metrics: Option<&clawft_types::config::VoiceTransportMetrics>,
) -> XaiSessionStart {
    match connect_hello(params).await {
        Ok(hello) => {
            if let Some(m) = metrics {
                m.record_connect_ok();
                m.record_disconnect(); // hello path disconnects cleanly after probe
            }
            XaiSessionStart::Xai(hello)
        }
        Err(e) => {
            let error_kind = match &e {
                XaiRealtimeError::MissingApiKey => "missing_api_key",
                XaiRealtimeError::InvalidUrl(_) => "invalid_url",
                XaiRealtimeError::RequestBuild(_) => "request_build",
                XaiRealtimeError::Connect(_) => "connect",
                XaiRealtimeError::Send(_) => "send",
                XaiRealtimeError::Receive(_) => "receive",
                XaiRealtimeError::Timeout => "timeout",
                XaiRealtimeError::ClosedEarly => "closed_early",
                XaiRealtimeError::ServerError(_) => "server_error",
            };
            warn!(
                error_kind,
                "xAI Realtime: session start failed; graceful local fallback"
            );
            let decision =
                super::xai_health::graceful_local_fallback_after_failure(preferred_mode, metrics);
            XaiSessionStart::LocalFallback {
                decision,
                error_kind,
            }
        }
    }
}

/// Drain until the first text JSON message with a `type` field.
///
/// Skips binary frames without logging their content (raw audio).
async fn wait_first_json_event<S>(read: &mut S) -> Result<String, XaiRealtimeError>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(msg) = read.next().await {
        let msg = msg.map_err(|e| XaiRealtimeError::Receive(e.to_string()))?;
        match msg {
            Message::Text(text) => {
                // Do not log full payload — may contain transcripts later.
                let v: Value = serde_json::from_str(&text)
                    .map_err(|e| XaiRealtimeError::Receive(format!("invalid JSON: {e}")))?;
                let event_type = v
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                if event_type == "error" {
                    // Prefer message field; never dump full error if it could contain secrets.
                    let msg = v
                        .pointer("/error/message")
                        .or_else(|| v.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("server error")
                        .to_string();
                    return Err(XaiRealtimeError::ServerError(msg));
                }
                return Ok(event_type);
            }
            Message::Binary(bytes) => {
                // Never log audio content.
                debug!(len = bytes.len(), "xAI Realtime: skipped binary frame");
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            Message::Close(_) => return Err(XaiRealtimeError::ClosedEarly),
        }
    }
    Err(XaiRealtimeError::ClosedEarly)
}

/// Strip potential credential fragments from error strings before logging/returning.
fn sanitize_ws_error(raw: &str) -> String {
    // Avoid echoing Authorization values if a library embeds them.
    let lower = raw.to_ascii_lowercase();
    if lower.contains("bearer ") || lower.contains("authorization") {
        "websocket error (details redacted — may contain credentials)".into()
    } else {
        raw.to_string()
    }
}

/// Load API key from env using [`VoiceXaiConfig::api_key_env_name`].
///
/// Returns [`XaiRealtimeError::MissingApiKey`] when unset/empty. Never logs the key.
pub fn load_api_key_from_env(xai: &VoiceXaiConfig) -> Result<String, XaiRealtimeError> {
    let name = xai.api_key_env_name();
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(XaiRealtimeError::MissingApiKey),
    }
}

/// Constants re-export for callers that only depend on channels.
pub fn default_endpoint() -> &'static str {
    DEFAULT_XAI_REALTIME_ENDPOINT
}

pub fn default_model() -> &'static str {
    DEFAULT_XAI_REALTIME_MODEL
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::config::{
        resolve_voice_transport, VoiceMode, ResolvedVoiceTransport, VOICE_NOTICE_XAI_NO_KEY,
    };

    #[test]
    fn session_update_payload_shape() {
        let p = session_update_payload("eve", "hello");
        assert_eq!(p["type"], "session.update");
        assert_eq!(p["session"]["voice"], "eve");
        assert_eq!(p["session"]["instructions"], "hello");
        assert_eq!(p["session"]["turn_detection"]["type"], "server_vad");
        assert!(p["session"].get("tools").is_none());
        // No secrets in payload.
        let s = p.to_string();
        assert!(!s.to_ascii_lowercase().contains("bearer"));
        assert!(!s.contains("api_key"));
    }

    #[test]
    fn session_update_with_weftos_tools_registers_core() {
        let p = session_update_with_weftos_tools("eve", default_conductor_instructions());
        assert_eq!(p["type"], "session.update");
        let tools = p["session"]["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"spawn_agent"));
        assert!(names.contains(&"focus_pane"));
        assert!(names.contains(&"summarize"));
        assert!(!names.contains(&"shell_intent"));
    }

    #[test]
    fn connect_params_register_tools_by_default() {
        let p = XaiRealtimeConnectParams::with_defaults("k");
        assert!(p.register_tools);
        assert!(!p.without_tools().register_tools);
    }

    #[test]
    fn build_request_rejects_empty_key() {
        let err = build_realtime_request("wss://api.x.ai/v1/realtime?model=x", "").unwrap_err();
        assert!(matches!(err, XaiRealtimeError::MissingApiKey));
    }

    #[test]
    fn build_request_sets_authorization_without_logging_key() {
        let key = "xai-test-secret-do-not-log";
        let req = build_realtime_request(
            "wss://api.x.ai/v1/realtime?model=grok-voice-latest",
            key,
        )
        .unwrap();
        let auth = req
            .headers()
            .get(AUTHORIZATION)
            .expect("Authorization header")
            .to_str()
            .unwrap();
        assert_eq!(auth, format!("Bearer {key}"));
        // Error Display / Debug of our error type never embeds the key for MissingApiKey.
        let err = XaiRealtimeError::MissingApiKey;
        assert!(!format!("{err}").contains(key));
        assert!(!format!("{err:?}").contains(key));
    }

    #[test]
    fn sanitize_redacts_bearer_errors() {
        let s = sanitize_ws_error("HTTP 401 Authorization: Bearer super-secret");
        assert!(!s.contains("super-secret"));
        assert!(s.contains("redacted"));
    }

    #[test]
    fn connect_params_from_voice_xai() {
        let xai = VoiceXaiConfig::default();
        let p = XaiRealtimeConnectParams::from_voice_xai(&xai, "k");
        assert!(p.url.contains("wss://api.x.ai/v1/realtime"));
        assert!(p.url.contains("model=grok-voice-latest"));
        assert_eq!(p.api_key, "k");
    }

    #[test]
    fn selection_integrates_with_channels_feature() {
        // Documented integration point: config selection before dialing.
        let d = resolve_voice_transport(VoiceMode::XaiS2s, false, true);
        assert_eq!(d.transport, ResolvedVoiceTransport::Local);
        assert_eq!(d.notice, Some(VOICE_NOTICE_XAI_NO_KEY));
    }

    #[test]
    fn default_endpoint_and_model_constants() {
        assert_eq!(default_endpoint(), "wss://api.x.ai/v1/realtime");
        assert_eq!(default_model(), "grok-voice-latest");
    }

    #[tokio::test]
    async fn connect_hello_or_fallback_uses_local_on_missing_key() {
        use clawft_types::config::{
            VoiceFallbackReason, VoiceTransportMetrics, VOICE_NOTICE_XAI_RUNTIME_FALLBACK,
        };

        let metrics = VoiceTransportMetrics::new();
        // Empty key → build_realtime_request fails without network.
        let params = XaiRealtimeConnectParams::with_defaults("").without_tools();
        let start = connect_hello_or_fallback(params, VoiceMode::XaiHybrid, Some(&metrics)).await;
        assert!(start.is_local_fallback());
        match start {
            XaiSessionStart::LocalFallback {
                decision,
                error_kind,
            } => {
                assert_eq!(decision.transport, ResolvedVoiceTransport::Local);
                assert_eq!(
                    decision.fallback_reason,
                    Some(VoiceFallbackReason::RuntimeFailure)
                );
                assert_eq!(decision.notice, Some(VOICE_NOTICE_XAI_RUNTIME_FALLBACK));
                assert_eq!(error_kind, "missing_api_key");
            }
            XaiSessionStart::Xai(_) => panic!("expected local fallback"),
        }
        let snap = metrics.snapshot();
        assert_eq!(snap.connect_fail, 1);
        assert_eq!(snap.fallback_runtime, 1);
        assert_eq!(snap.select_local, 1);
    }

    /// Live network hello. Skipped in CI; run manually with XAI_API_KEY.
    ///
    /// ```bash
    /// XAI_API_KEY=... cargo test -p clawft-channels --features voice-xai \
    ///   xai_realtime_live_hello -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "network: requires XAI_API_KEY; skipped in CI"]
    async fn xai_realtime_live_hello() {
        let xai = VoiceXaiConfig::default();
        let key = load_api_key_from_env(&xai).expect("XAI_API_KEY must be set for live test");
        let params = XaiRealtimeConnectParams::from_voice_xai(&xai, key);
        let result = connect_hello(params).await.expect("connect_hello");
        assert!(result.completed);
        // Prefer observing a session.* event, but V0 accepts connect-only.
        eprintln!(
            "xAI hello ok: url={} first_event={:?}",
            result.url, result.first_event_type
        );
    }
}
