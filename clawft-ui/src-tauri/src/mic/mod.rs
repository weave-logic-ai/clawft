//! Native microphone capture bridged to the clawft-ui webview (WEFT-228).
//!
//! Replaces the browser-only `navigator.mediaDevices.getUserMedia` path when
//! the dashboard runs inside the Tauri shell. Capture uses **cpal** (CoreAudio
//! / ALSA / WASAPI). Commands:
//!
//! | Command | Purpose |
//! |---------|---------|
//! | `mic_backend_info` | Diagnostics + whether capture is active |
//! | `mic_list_devices` | Enumerate input devices |
//! | `mic_start` | Begin continuous mono capture |
//! | `mic_stop` | Stop capture |
//! | `mic_poll` | Drain PCM + peak since last poll |
//! | `mic_test_level` | ~1 s peak probe (settings "Test Mic") |
//!
//! Unit tests for peak/downmix live in [`level`] and do not open a real mic.

mod capture;
mod level;
mod platform;

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::State;

pub use capture::{
    list_devices, probe_level, CaptureSession, MicBackendInfo, MicDeviceInfo, MicLevelReport,
    MicPollResult, MicSessionInfo,
};

/// Shared capture handle managed by Tauri.
pub struct MicState {
    session: Mutex<Option<CaptureSession>>,
}

impl MicState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }
}

impl Default for MicState {
    fn default() -> Self {
        Self::new()
    }
}

/// Uniform success/error envelope matching `shell_info` responses.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdResponse<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CmdResponse<T> {
    fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Backend / platform diagnostics for the voice settings panel.
#[tauri::command]
pub fn mic_backend_info(state: State<'_, MicState>) -> CmdResponse<MicBackendInfo> {
    let capturing = state
        .session
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);
    CmdResponse::success(MicBackendInfo {
        available: true,
        backend: "cpal".into(),
        host_backend: platform::host_backend().into(),
        target_os: platform::target_os().into(),
        notes: platform::privacy_notes().into(),
        capturing,
    })
}

/// List cpal input devices (default first when known).
#[tauri::command]
pub fn mic_list_devices() -> CmdResponse<Vec<MicDeviceInfo>> {
    match list_devices() {
        Ok(d) => CmdResponse::success(d),
        Err(e) => CmdResponse::err(e),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicStartArgs {
    /// Optional device name substring (case-insensitive). `None` = default.
    pub device: Option<String>,
}

/// Start continuous native capture. Stops any prior session first.
#[tauri::command]
pub fn mic_start(
    state: State<'_, MicState>,
    args: Option<MicStartArgs>,
) -> CmdResponse<MicSessionInfo> {
    let device = args.and_then(|a| a.device);
    let mut guard = match state.session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    // Drop previous session before opening a new one (releases the device).
    *guard = None;
    match CaptureSession::start(device.as_deref()) {
        Ok(session) => {
            let info = session.info().clone();
            *guard = Some(session);
            CmdResponse::success(info)
        }
        Err(e) => CmdResponse::err(e),
    }
}

/// Stop capture if running.
#[tauri::command]
pub fn mic_stop(state: State<'_, MicState>) -> CmdResponse<bool> {
    let mut guard = match state.session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let was_active = guard.is_some();
    *guard = None;
    CmdResponse::success(was_active)
}

/// Drain buffered PCM samples + peak from the active session.
#[tauri::command]
pub fn mic_poll(state: State<'_, MicState>) -> CmdResponse<MicPollResult> {
    let guard = match state.session.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    match guard.as_ref() {
        Some(session) => CmdResponse::success(session.poll()),
        None => CmdResponse::success(MicPollResult {
            pcm_i16: Vec::new(),
            sample_rate: 0,
            peak: 0.0,
            capturing: false,
        }),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicTestArgs {
    pub device: Option<String>,
    /// Probe window in milliseconds (default 1000).
    pub duration_ms: Option<u64>,
}

/// Open the mic for ~1 s, measure peak level, close (settings Test Mic).
///
/// Runs the blocking cpal probe on a worker thread so the UI stays responsive.
/// Returns `Result` because async Tauri commands that take `State` references
/// must return a `Result` (Tauri 2 constraint).
#[tauri::command]
pub async fn mic_test_level(
    state: State<'_, MicState>,
    args: Option<MicTestArgs>,
) -> Result<CmdResponse<MicLevelReport>, String> {
    // Don't race a long-lived session; pause it for the probe.
    {
        let mut guard = match state.session.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = None;
    }

    let device = args.as_ref().and_then(|a| a.device.clone());
    let ms = args.and_then(|a| a.duration_ms).unwrap_or(1000).clamp(100, 5000);

    let result = tauri::async_runtime::spawn_blocking(move || {
        probe_level(device.as_deref(), Duration::from_millis(ms))
    })
    .await
    .map_err(|e| format!("mic probe task failed: {e}"))?;

    Ok(match result {
        Ok(r) => CmdResponse::success(r),
        Err(e) => CmdResponse::err(e),
    })
}
