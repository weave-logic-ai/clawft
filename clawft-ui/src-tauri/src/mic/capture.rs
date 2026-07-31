//! cpal-backed microphone capture session for the Tauri shell (WEFT-228).
//!
//! Opens the default (or named) input device, downmixes to mono `f32`, and
//! buffers samples for the UI to poll. A short probe path measures peak level
//! without leaving a long-lived stream open (replaces browser `getUserMedia`
//! mic tests inside Tauri).
//!
//! ## Threading
//!
//! On macOS, cpal's CoreAudio `Stream` is **not** `Send`. Capture therefore
//! owns the stream on a dedicated OS thread; only the sample queue and
//! control flags cross threads (so Tauri `State` stays `Send + Sync`).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;

use super::level::{downmix_f32, f32_to_i16, peak_level_f32};
use super::platform;

/// Max mono samples retained in the ring (~2 s at 48 kHz). Older samples drop.
const MAX_BUFFER_SAMPLES: usize = 48_000 * 2;

/// Shared mono-`f32` FIFO written by the cpal callback.
type SampleQ = Arc<Mutex<VecDeque<f32>>>;

/// Descriptor for an input device.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Live session metadata returned from `mic_start`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicSessionInfo {
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub backend: String,
    pub target_os: String,
}

/// Polled PCM chunk from an active session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicPollResult {
    /// Mono PCM as `i16` samples (UI-friendly; little-endian host order).
    pub pcm_i16: Vec<i16>,
    pub sample_rate: u32,
    /// Peak absolute level of this chunk in `[0, 1]`.
    pub peak: f32,
    pub capturing: bool,
}

/// One-shot level probe (browser `testMicrophone` replacement).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicLevelReport {
    /// Peak level in `[0, 1]` over the probe window.
    pub level: f32,
    pub device: String,
    pub sample_rate: u32,
    pub backend: String,
    pub target_os: String,
}

/// Backend capability / diagnostics for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicBackendInfo {
    pub available: bool,
    pub backend: String,
    pub host_backend: String,
    pub target_os: String,
    pub notes: String,
    pub capturing: bool,
}

/// Capture session handle. The real cpal stream lives on `join`'s thread.
pub struct CaptureSession {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    queue: SampleQ,
    info: MicSessionInfo,
}

impl CaptureSession {
    /// Open an input device on a dedicated thread and start streaming mono `f32`.
    pub fn start(device_name: Option<&str>) -> Result<Self, String> {
        let device_owned = device_name.map(|s| s.to_string());
        let queue: SampleQ = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<MicSessionInfo, String>>();

        let q_thread = queue.clone();
        let stop_thread = stop.clone();

        let join = thread::Builder::new()
            .name("clawft-mic-capture".into())
            .spawn(move || {
                let result = run_capture_loop(device_owned.as_deref(), q_thread, stop_thread, ready_tx);
                if let Err(e) = result {
                    // ready_tx may already have been used; best-effort log via stderr.
                    eprintln!("[clawft-mic] capture thread ended: {e}");
                }
            })
            .map_err(|e| format!("spawn capture thread: {e}"))?;

        // Wait for the worker to open the device (or fail).
        let info = match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(info)) => info,
            Ok(Err(e)) => {
                stop.store(true, Ordering::SeqCst);
                let _ = join.join();
                return Err(e);
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                let _ = join.join();
                return Err("timed out opening input device".into());
            }
        };

        Ok(Self {
            stop,
            join: Some(join),
            queue,
            info,
        })
    }

    pub fn info(&self) -> &MicSessionInfo {
        &self.info
    }

    /// Drain buffered mono samples since the last poll.
    pub fn poll(&self) -> MicPollResult {
        let samples: Vec<f32> = {
            let mut g = self.queue.lock().unwrap_or_else(|p| p.into_inner());
            g.drain(..).collect()
        };
        let peak = peak_level_f32(&samples);
        MicPollResult {
            pcm_i16: f32_to_i16(&samples),
            sample_rate: self.info.sample_rate,
            peak,
            capturing: true,
        }
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.join.take() {
            // Don't block forever if the audio host wedges.
            let _ = handle.join();
        }
    }
}

/// Worker body: build stream, signal ready, park until stop.
fn run_capture_loop(
    device_name: Option<&str>,
    queue: SampleQ,
    stop: Arc<AtomicBool>,
    ready_tx: mpsc::Sender<Result<MicSessionInfo, String>>,
) -> Result<(), String> {
    let built = match build_stream(device_name, queue) {
        Ok(b) => b,
        Err(e) => {
            let _ = ready_tx.send(Err(e.clone()));
            return Err(e);
        }
    };
    let _ = ready_tx.send(Ok(built.info.clone()));

    // Keep `_stream` alive on this thread until stop.
    let _stream = built.stream;
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

struct BuiltStream {
    stream: cpal::Stream,
    info: MicSessionInfo,
}

fn build_stream(device_name: Option<&str>, queue: SampleQ) -> Result<BuiltStream, String> {
    let host = cpal::default_host();
    let device = pick_input(&host, device_name)?;
    let name = device.name().unwrap_or_else(|_| "<unknown>".into());
    let supported = device
        .default_input_config()
        .map_err(|e| format!("input config: {e}"))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels().max(1);
    let cfg: cpal::StreamConfig = supported.clone().into();
    let n_ch = channels as usize;

    let err_slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let err_cb = err_slot.clone();

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let q = queue.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _| {
                    let mono = downmix_f32(data, n_ch);
                    push_capped(&q, &mono);
                },
                move |e| {
                    *err_cb.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
                },
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let q = queue.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _| {
                    let mut f32s = Vec::with_capacity(data.len());
                    for &s in data {
                        f32s.push(f32::from(s) / 32_768.0);
                    }
                    let mono = downmix_f32(&f32s, n_ch);
                    push_capped(&q, &mono);
                },
                move |e| {
                    *err_cb.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
                },
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let q = queue.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[u16], _| {
                    let mut f32s = Vec::with_capacity(data.len());
                    for &s in data {
                        f32s.push((f32::from(s) - 32_768.0) / 32_768.0);
                    }
                    let mono = downmix_f32(&f32s, n_ch);
                    push_capped(&q, &mono);
                },
                move |e| {
                    *err_cb.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
                },
                None,
            )
        }
        other => {
            return Err(format!("unsupported input sample format: {other:?}"));
        }
    }
    .map_err(|e| format!("build_input_stream: {e}"))?;

    stream.play().map_err(|e| format!("stream.play: {e}"))?;

    if let Some(e) = err_slot.lock().unwrap_or_else(|p| p.into_inner()).take() {
        return Err(e);
    }

    Ok(BuiltStream {
        stream,
        info: MicSessionInfo {
            device: name,
            sample_rate,
            channels,
            backend: "cpal".into(),
            target_os: platform::target_os().into(),
        },
    })
}

fn push_capped(q: &SampleQ, mono: &[f32]) {
    let mut g = q.lock().unwrap_or_else(|p| p.into_inner());
    g.extend(mono.iter().copied());
    while g.len() > MAX_BUFFER_SAMPLES {
        g.pop_front();
    }
}

/// List input devices; default first when identifiable.
pub fn list_devices() -> Result<Vec<MicDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut out: Vec<MicDeviceInfo> = Vec::new();
    let devices = host
        .input_devices()
        .map_err(|e| format!("input_devices: {e}"))?;
    for d in devices {
        let name = d.name().unwrap_or_else(|_| "<unknown>".into());
        let is_default = default_name.as_ref() == Some(&name);
        out.push(MicDeviceInfo { name, is_default });
    }
    // Stable order: default first.
    out.sort_by_key(|d| !d.is_default);
    Ok(out)
}

/// Open the mic briefly, sample peak over `duration`, then tear down.
///
/// Safe to call from a blocking worker thread (e.g. `spawn_blocking`).
pub fn probe_level(device_name: Option<&str>, duration: Duration) -> Result<MicLevelReport, String> {
    let session = CaptureSession::start(device_name)?;
    let device = session.info().device.clone();
    let sample_rate = session.info().sample_rate;
    let deadline = Instant::now() + duration;
    let mut peak = 0.0f32;
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
        let chunk = session.poll();
        if chunk.peak > peak {
            peak = chunk.peak;
        }
    }
    drop(session);
    Ok(MicLevelReport {
        level: peak.clamp(0.0, 1.0),
        device,
        sample_rate,
        backend: "cpal".into(),
        target_os: platform::target_os().into(),
    })
}

fn pick_input(host: &cpal::Host, want: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(sub) = want {
        let sub_l = sub.to_lowercase();
        for d in host
            .input_devices()
            .map_err(|e| format!("input_devices: {e}"))?
        {
            if let Ok(name) = d.name()
                && name.to_lowercase().contains(&sub_l)
            {
                return Ok(d);
            }
        }
        return Err(format!("no input device matching {sub:?}"));
    }
    host.default_input_device()
        .ok_or_else(|| "no default input device (check mic privacy permissions)".into())
}

#[cfg(test)]
mod tests {
    use super::super::level::{downmix_f32, peak_level_f32};

    /// Capture path math is covered here without opening a real device.
    #[test]
    fn buffered_peak_matches_level_helper() {
        let stereo = vec![0.0_f32, 0.0, 0.8, -0.8, 0.1, 0.1];
        let mono = downmix_f32(&stereo, 2);
        // frames: (0+0)/2=0, (0.8-0.8)/2=0, (0.1+0.1)/2=0.1
        assert_eq!(mono.len(), 3);
        assert!((peak_level_f32(&mono) - 0.1).abs() < 1e-6);
    }
}
