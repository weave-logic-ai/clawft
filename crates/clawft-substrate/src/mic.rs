//! Microphone reference adapter — first physical-sensor adapter for
//! WeftOS substrate.
//!
//! Reads signed-16-bit little-endian mono PCM samples from a
//! configurable byte source (default: a file backing that a test
//! harness or a future host-audio bridge can feed) and emits RMS +
//! peak levels every 500 ms.
//!
//! ## Path layout (WEFT-418 / WEFT-438)
//!
//! Canonical emissions (always):
//! - `substrate/<node-id>/sensor/mic/summary` — full level object
//! - `substrate/<node-id>/sensor/mic/rms` — scalar `rms_db` for gauge
//!   discovery (`mic_discovery` matches `/sensor/mic/rms`)
//! - `substrate/<node-id>/sensor/mic/pcm_chunk` — windowed-Append PCM
//!   stream (whisper / classify input; shape matches
//!   `JOURNALED-SENSOR-MIC.md` §2.2 + whisper `PcmChunk`)
//!
//! Legacy dual-emit (default on until
//! [`crate::sensor_paths::LEGACY_FLAT_REMOVAL_VERSION`]):
//! - `substrate/sensor/mic` — pre-node-identity flat level path
//! - `substrate/sensor/mic/pcm_chunk` — pre-node-identity flat PCM path
//!
//! Default `node_id` is [`crate::sensor_paths::HOST_LOCAL_NODE_ID`].
//! Pass a provisioned node id via [`MicrophoneAdapter::with_node_id`]
//! for multi-node meshes.
//!
//! ## Characterization level (spectrometer principle)
//!
//! This adapter declares [`Characterization::Rate`]: it produces a
//! scalar level per window with no spectral structure. It cannot
//! distinguish a 440 Hz tone from pink noise at the same RMS. That
//! distinction is the upgrade path to `Characterization::Spectral`
//! — add an FFT pass and emit per-bin magnitudes alongside the
//! scalar. The adapter surface stays the same; consumers that care
//! about spectral content check `characterization` in the emitted
//! object.
//!
//! ## File-backed source (today) → host-audio source (next)
//!
//! The backing source is a plain raw-PCM file. Tests write a known
//! buffer to a tempfile and assert against the computed RMS. The
//! path is parameterised at construction so the same adapter can be
//! hooked up to:
//!
//! - a synthetic stream for dev panels (e.g. mix a test tone)
//! - a host-audio bridge writing live capture bytes into a named
//!   pipe or `/dev/shm/weftos/mic/stream.raw`
//! - a CPAL / ALSA / CoreAudio / WASAPI shim (future — this is what
//!   lights up the webcam mic on the user's own machine)
//!
//! No host-audio dep in this slice. Keeps the preview reviewable.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::adapter::{
    AdapterError, BufferPolicy, OntologyAdapter, PermissionReq, RefreshHint, Sensitivity, SubId,
    Subscription, TopicDecl,
};
use crate::delta::StateDelta;
use crate::healthcheck::{
    SensorHealthReport, SensorStatus, build_report_delta as build_health_delta,
};
use crate::physical::{
    Characterization, PhysicalSensorAdapter, SensorCalibration, SensorInterface,
};
use crate::sensor_paths::{
    DEFAULT_DUAL_EMIT_LEGACY, HOST_LOCAL_MIC_PCM_CHUNK, HOST_LOCAL_MIC_RMS, HOST_LOCAL_MIC_SUMMARY,
    HOST_LOCAL_NODE_ID, LEGACY_MIC_PATH, LEGACY_MIC_PCM_PATH, MIC_PCM_WINDOW_MAX_LEN,
    is_mic_open_topic, mic_level_emit_plan, mic_pcm_emit_plan,
};
// Used only by `tests::host_audio_direction_variant_compiles` via
// `use super::*` — keep the import so `cargo test` builds.
#[cfg(test)]
#[allow(unused_imports)]
use crate::physical::AudioDirection;

/// Window size in samples read per tick. At 16 kHz / 500 ms that's
/// 8000 samples = 16 000 bytes.
const WINDOW_SAMPLES: usize = 8000;
/// Default sample rate assumed when none is configured.
const DEFAULT_SAMPLE_RATE: u32 = 16_000;
/// Channel depth for the shared producer. Sized for dual-emit
/// (summary + rms + pcm + optional legacy level/pcm) + health + slack.
const CHAN: usize = 16;
/// Emission cadence — matches WINDOW_SAMPLES at DEFAULT_SAMPLE_RATE.
const TICK_MS: u64 = 500;

/// Declared topics.
///
/// The mic adapter declares level + PCM + healthcheck topics. Level and
/// PCM topics cover the WEFT-418/438 dual-emit set so `subscribe_adapter`
/// against any of the open targets shares the same producer loop:
///
/// - [`HOST_LOCAL_MIC_SUMMARY`] — canonical level object (host-local)
/// - [`HOST_LOCAL_MIC_RMS`] — canonical RMS scalar (host-local)
/// - [`HOST_LOCAL_MIC_PCM_CHUNK`] — windowed-Append PCM (host-local)
/// - [`LEGACY_MIC_PATH`] — flat legacy level shim (removal: 0.9.0)
/// - [`LEGACY_MIC_PCM_PATH`] — flat legacy PCM shim (removal: 0.9.0)
/// - `substrate/meta/adapter/mic/healthcheck` — per-sensor health
///   (HEALTHCHECK-CONTRACT.md §3; Public — no audio content)
///
/// `open()` also accepts any node-scoped
/// `…/sensor/mic/{summary,rms,pcm_chunk}` so multi-node callers do not
/// need a TopicDecl per node id.
pub const TOPICS: &[TopicDecl] = &[
    TopicDecl {
        path: HOST_LOCAL_MIC_SUMMARY,
        shape: "ontology://audio-level",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        // Audio-capture data CAN leak user content even at RMS level
        // (speech envelope is recoverable). `Capture` sensitivity per
        // ADR-012 forces a per-goal `CapabilityGrant` rather than a
        // one-off install-time prompt.
        sensitivity: Sensitivity::Capture,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
    TopicDecl {
        path: HOST_LOCAL_MIC_RMS,
        shape: "ontology://audio-level-rms",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        sensitivity: Sensitivity::Capture,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
    TopicDecl {
        // WEFT-418: windowed-Append PCM stream. Substrate trims the
        // front of the array to `MIC_PCM_WINDOW_MAX_LEN` on each Append
        // (drop-oldest ring — ~4 s at 2 Hz).
        path: HOST_LOCAL_MIC_PCM_CHUNK,
        shape: "ontology://audio-pcm-chunk",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        sensitivity: Sensitivity::Capture,
        buffer_policy: BufferPolicy::DropOldest,
        max_len: Some(MIC_PCM_WINDOW_MAX_LEN),
    },
    TopicDecl {
        // Legacy flat open target — dual-emitted until 0.9.0 (WEFT-438).
        path: LEGACY_MIC_PATH,
        shape: "ontology://audio-level",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        sensitivity: Sensitivity::Capture,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
    TopicDecl {
        // Legacy flat PCM open target — dual-emitted until 0.9.0 (WEFT-418).
        path: LEGACY_MIC_PCM_PATH,
        shape: "ontology://audio-pcm-chunk",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        sensitivity: Sensitivity::Capture,
        buffer_policy: BufferPolicy::DropOldest,
        max_len: Some(MIC_PCM_WINDOW_MAX_LEN),
    },
    TopicDecl {
        path: "substrate/meta/adapter/mic/healthcheck",
        shape: "ontology://sensor-health-report",
        refresh_hint: RefreshHint::Periodic { ms: TICK_MS },
        sensitivity: Sensitivity::Public,
        buffer_policy: BufferPolicy::Refuse,
        max_len: None,
    },
];

/// Permissions — ADR-012 capture channel. `Substrate::subscribe_adapter`
/// with a [`clawft_app::CapturePrivacyGate`] requires a matching
/// `Permission::Mic` session grant (WEFT-429).
pub const PERMISSIONS: &[PermissionReq] = &[PermissionReq::Mic];

type CancelTx = oneshot::Sender<()>;

struct Registry {
    next_id: u64,
    live: HashMap<SubId, CancelTx>,
}

impl Registry {
    fn new() -> Self {
        Self {
            next_id: 1,
            live: HashMap::new(),
        }
    }

    fn allocate(&mut self) -> SubId {
        let id = SubId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

/// Microphone adapter.
pub struct MicrophoneAdapter {
    reg: Mutex<Registry>,
    source_path: PathBuf,
    sample_rate: u32,
    /// Publishing node id — prefixes the canonical emission paths.
    node_id: String,
    /// When true, also emit the legacy flat `substrate/sensor/mic`
    /// path (WEFT-438 migration window).
    dual_emit_legacy: bool,
}

impl Default for MicrophoneAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MicrophoneAdapter {
    /// Build an adapter reading from the default file-backed source
    /// at `/tmp/weftos/mic/stream.raw`, 16 kHz mono s16le, under the
    /// host-local node id with legacy dual-emit enabled.
    pub fn new() -> Self {
        Self::with_source(
            PathBuf::from("/tmp/weftos/mic/stream.raw"),
            DEFAULT_SAMPLE_RATE,
        )
    }

    /// Build with an explicit source path + sample rate — used by
    /// tests (fake PCM fixture) and by a future synthetic-stream
    /// generator.
    pub fn with_source(source_path: PathBuf, sample_rate: u32) -> Self {
        Self {
            reg: Mutex::new(Registry::new()),
            source_path,
            sample_rate,
            node_id: HOST_LOCAL_NODE_ID.to_string(),
            dual_emit_legacy: DEFAULT_DUAL_EMIT_LEGACY,
        }
    }

    /// Override the publishing node id (default
    /// [`HOST_LOCAL_NODE_ID`]). Use the daemon's provisioned identity
    /// for multi-node meshes.
    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = node_id.into();
        self
    }

    /// Control legacy flat dual-emit (default
    /// [`DEFAULT_DUAL_EMIT_LEGACY`]). Pass `false` after the 0.9.0 cut
    /// or in tests that assert a pure node-scoped tree.
    pub fn with_dual_emit_legacy(mut self, dual: bool) -> Self {
        self.dual_emit_legacy = dual;
        self
    }

    /// Publishing node id this adapter emits under.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[async_trait]
impl OntologyAdapter for MicrophoneAdapter {
    fn id(&self) -> &'static str {
        "mic"
    }

    fn topics(&self) -> &'static [TopicDecl] {
        TOPICS
    }

    fn permissions(&self) -> &'static [PermissionReq] {
        PERMISSIONS
    }

    async fn open(&self, topic: &str, _args: Value) -> Result<Subscription, AdapterError> {
        // Level + PCM topics (legacy + node-scoped) and the healthcheck
        // share the same producer loop. Subscribing to any of them
        // yields the full dual-emit delta stream; the substrate routes
        // by path.
        let is_health = topic == "substrate/meta/adapter/mic/healthcheck";
        if !is_health && !is_mic_open_topic(topic, &self.node_id) {
            return Err(AdapterError::UnknownTopic(topic.into()));
        }
        let id = {
            let mut reg = self.reg.lock();
            reg.allocate()
        };
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel::<StateDelta>(CHAN);
        self.reg.lock().live.insert(id, cancel_tx);

        let source_path = self.source_path.clone();
        let sample_rate = self.sample_rate;
        let node_id = self.node_id.clone();
        let dual_emit_legacy = self.dual_emit_legacy;
        tokio::spawn(async move {
            poll_level(source_path, sample_rate, node_id, dual_emit_legacy, tx, cancel_rx).await;
        });
        Ok(Subscription { id, rx })
    }

    async fn close(&self, sub_id: SubId) -> Result<(), AdapterError> {
        let _ = self.reg.lock().live.remove(&sub_id);
        Ok(())
    }
}

#[async_trait]
impl PhysicalSensorAdapter for MicrophoneAdapter {
    fn model(&self) -> &'static str {
        // Preview stub — real adapters will report the actual device
        // model from CPAL / ALSA / CoreAudio.
        "file-backed s16le PCM (preview stub)"
    }

    fn interface(&self) -> SensorInterface {
        // FileBacked for the preview; real hardware flips to
        // HostAudio { Capture } when the CPAL bridge lands.
        SensorInterface::FileBacked {
            path: self.source_path.clone(),
        }
    }

    fn unit(&self) -> &'static str {
        "dBFS"
    }

    fn range(&self) -> (f64, f64) {
        // dBFS: -∞ (silence) to 0 (full-scale). We clamp the silence
        // floor to -120 dB in the emitted value for numerical
        // stability; the trait-level range is still the idealised
        // one.
        (f64::NEG_INFINITY, 0.0)
    }

    fn calibration(&self) -> SensorCalibration {
        SensorCalibration {
            scale: 1.0,
            offset: 0.0,
            reference: Some("s16le full-scale = ±32767; dBFS = 20 * log10(rms / 32768)".into()),
        }
    }

    fn characterization(&self) -> Characterization {
        // Honest: we emit a scalar level, not spectral bins.
        // Upgrading to Spectral means adding an FFT pass + emitting
        // per-bin magnitudes on a sibling topic.
        Characterization::Rate
    }
}

async fn poll_level(
    source_path: PathBuf,
    sample_rate: u32,
    node_id: String,
    dual_emit_legacy: bool,
    tx: mpsc::Sender<StateDelta>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let mut ticker = tokio::time::interval(Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let level_plan = mic_level_emit_plan(&node_id, dual_emit_legacy);
    let pcm_plan = mic_pcm_emit_plan(&node_id, dual_emit_legacy);

    // File-read cursor that advances each tick. The file is expected
    // to be a rolling buffer (test, synthetic stream, or mmapped
    // audio bridge) — we read the tail.
    let mut cursor: u64 = 0;
    // Healthcheck producer state — counts ticks + errors so we can
    // emit the per-sensor health shape per HEALTHCHECK-CONTRACT.md §3.
    // Configured rate is `1000 / TICK_MS` Hz.
    let mut tick_count: u64 = 0;
    let mut error_count: u64 = 0;
    let mut last_emit_ts: u64 = 0;
    let mut last_error: Option<String> = None;
    // Monotonic PCM chunk sequence (WEFT-418) — independent of wall clock
    // so whisper/classify joiners can order windows without timing assumptions.
    let mut pcm_seq: u64 = 0;
    // Emit an initial `unknown`-status report so subscribers see the
    // healthcheck path populated immediately (contract §9 open question
    // 1: ship `unknown` for the pre-first-emit window).
    let configured_rate_hz = 1000.0 / TICK_MS as f64;
    let initial_report = SensorHealthReport::unknown(configured_rate_hz);
    if tx
        .send(build_health_delta("mic", &initial_report))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            _ = &mut cancel_rx => return,
            _ = ticker.tick() => {
                tick_count += 1;
                let measure = read_and_measure(&source_path, sample_rate, &mut cursor);
                let reason = measure
                    .level
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                // If the backing file is missing we emit nothing on the
                // primary topics. Otherwise we'd overwrite
                // externally-published values (e.g. from the ESP32 bridge
                // calling `substrate.publish`) on every tick with
                // `{available: false, reason: "source-missing"}`, which
                // has no `rms_db` key and breaks the gauge binding. The
                // healthcheck path *does* still update so subscribers
                // can see the sensor is unhealthy.
                let payload_published = if reason.as_deref() == Some("source-missing") {
                    error_count += 1;
                    last_error = Some("source-missing".into());
                    false
                } else {
                    // WEFT-438 dual-emit: summary object + rms scalar
                    // always; legacy flat when the migration flag is on.
                    let rms_scalar = measure
                        .level
                        .get("rms_db")
                        .cloned()
                        .unwrap_or(Value::Null);
                    let level_deltas = [
                        Some(StateDelta::Replace {
                            path: level_plan.summary.clone(),
                            value: measure.level.clone(),
                        }),
                        Some(StateDelta::Replace {
                            path: level_plan.rms.clone(),
                            value: rms_scalar,
                        }),
                        level_plan.legacy.as_ref().map(|legacy| StateDelta::Replace {
                            path: legacy.clone(),
                            value: measure.level.clone(),
                        }),
                    ];
                    for delta in level_deltas.into_iter().flatten() {
                        if tx.send(delta).await.is_err() {
                            return;
                        }
                    }

                    // WEFT-418: windowed-Append PCM chunk on the
                    // node-scoped path (+ optional legacy dual-emit).
                    // Only emit when we actually read samples — empty
                    // windows (EOF of a finite fixture) still publish
                    // level but skip PCM so whisper doesn't ingest silence
                    // spam from a drained file.
                    if let Some(pcm_bytes) = measure.pcm_bytes.as_ref()
                        && !pcm_bytes.is_empty()
                    {
                        pcm_seq = pcm_seq.saturating_add(1);
                        let chunk = build_pcm_chunk(
                            pcm_bytes,
                            sample_rate,
                            pcm_seq,
                            tick_count,
                        );
                        let pcm_deltas = [
                            Some(StateDelta::Append {
                                path: pcm_plan.pcm.clone(),
                                value: chunk.clone(),
                            }),
                            pcm_plan.legacy.as_ref().map(|legacy| StateDelta::Append {
                                path: legacy.clone(),
                                value: chunk,
                            }),
                        ];
                        for delta in pcm_deltas.into_iter().flatten() {
                            if tx.send(delta).await.is_err() {
                                return;
                            }
                        }
                    }
                    last_emit_ts = now_ms();
                    true
                };

                // Build + send the per-sensor health report. Cadence
                // policy: every tick is fine at 2 Hz (contract §3.3 —
                // "every emit, or at least every N=4 emits"); this keeps
                // the producer state machine simple.
                let observed_rate_hz = if payload_published {
                    // Tracking observed-rate from inside the producer is
                    // necessarily approximate (a true rolling-window
                    // measurement belongs to the §7 aggregator). For the
                    // preview we report `configured` when the last tick
                    // succeeded and zero otherwise — this is what
                    // `derive_status` needs to produce a useful status.
                    configured_rate_hz
                } else {
                    0.0
                };
                let mut report = SensorHealthReport {
                    status: SensorStatus::Unknown,
                    last_emit_ts,
                    configured_rate_hz,
                    observed_rate_hz,
                    error_count,
                    tick: tick_count,
                    since_ms: None,
                    last_error: last_error.clone(),
                    notes: reason.map(|r| format!("read-result: {r}")),
                };
                report.status = SensorHealthReport::derive_status(
                    observed_rate_hz,
                    configured_rate_hz,
                    if payload_published { 0 } else { 1 },
                );
                if tx.send(build_health_delta("mic", &report)).await.is_err() {
                    return;
                }
            }
        }
    }
}

/// Build the whisper/classify-compatible PCM chunk object.
///
/// Wire shape accepts both journal fields (`pcm_b64`, `encoding: s16le`)
/// and the ESP32/whisper wire fields (`data`, `encoding: base64`,
/// `format: i16le`). See `JOURNALED-SENSOR-MIC.md` §2.2 and
/// `clawft_service_whisper::windower::PcmChunk`.
fn build_pcm_chunk(pcm_bytes: &[u8], sample_rate: u32, seq: u64, tick: u64) -> Value {
    let samples = (pcm_bytes.len() / 2) as u64;
    let b64 = B64.encode(pcm_bytes);
    json!({
        // Whisper / ESP32 wire fields
        "data": b64.clone(),
        "encoding": "base64",
        "format": "i16le",
        "sample_rate": sample_rate,
        "channels": 1,
        "samples": samples,
        "start_ts_ms": seq.saturating_mul(TICK_MS),
        // Journal / publish_wav aliases (pcm_b64 ≡ data; seq ≡ start_ts_ms)
        "pcm_b64": b64,
        "seq": seq,
        "chunk_ms": TICK_MS,
        "tick": tick,
    })
}

/// Result of one window read: level summary object + optional raw PCM.
struct MeasureResult {
    level: Value,
    /// Raw s16le bytes for this window when the source delivered audio.
    /// `None` when the source is missing / reset / empty.
    pcm_bytes: Option<Vec<u8>>,
}

/// Wall-clock millis since UNIX epoch — small helper kept private so
/// the system-time read isn't sprinkled through the poller body.
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Read up to `WINDOW_SAMPLES` from the file at `cursor`, compute
/// RMS + peak in dBFS, and capture the raw PCM bytes for the
/// windowed-Append topic. Advances `cursor` to the new end-of-read
/// position.
fn read_and_measure(source_path: &Path, sample_rate: u32, cursor: &mut u64) -> MeasureResult {
    let Ok(mut file) = std::fs::File::open(source_path) else {
        return MeasureResult {
            level: json!({
                "available": false,
                "reason": "source-missing",
                "characterization": Characterization::Rate.as_str(),
            }),
            pcm_bytes: None,
        };
    };
    if file.seek(SeekFrom::Start(*cursor)).is_err() {
        // File truncated / replaced — reset and return a 'stream
        // reset' marker.
        *cursor = 0;
        return MeasureResult {
            level: json!({
                "available": false,
                "reason": "stream-reset",
                "characterization": Characterization::Rate.as_str(),
            }),
            pcm_bytes: None,
        };
    }
    let mut buf = vec![0u8; WINDOW_SAMPLES * 2];
    let n = file.read(&mut buf).unwrap_or(0);
    let samples_read = n / 2;
    *cursor += n as u64;
    if samples_read == 0 {
        return MeasureResult {
            level: json!({
                "available": true,
                "rms_db": -120.0,
                "peak_db": -120.0,
                "sample_rate": sample_rate,
                "samples_in_window": 0,
                "characterization": Characterization::Rate.as_str(),
            }),
            pcm_bytes: None,
        };
    }

    let (rms_db, peak_db) = rms_and_peak_dbfs(&buf[..n]);
    buf.truncate(n);
    MeasureResult {
        level: json!({
            "available": true,
            "rms_db": rms_db,
            "peak_db": peak_db,
            "sample_rate": sample_rate,
            "samples_in_window": samples_read,
            "characterization": Characterization::Rate.as_str(),
        }),
        pcm_bytes: Some(buf),
    }
}

/// Compute RMS + peak level of a signed-16-bit little-endian PCM
/// byte buffer, in dBFS. Silence floor clamped to -120 dB.
fn rms_and_peak_dbfs(bytes: &[u8]) -> (f64, f64) {
    let sample_count = bytes.len() / 2;
    if sample_count == 0 {
        return (-120.0, -120.0);
    }
    let mut sum_sq: f64 = 0.0;
    let mut peak: i32 = 0;
    for chunk in bytes.chunks_exact(2) {
        let s = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        sum_sq += (s as f64) * (s as f64);
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
    }
    let rms = (sum_sq / sample_count as f64).sqrt();
    let rms_db = sample_to_dbfs(rms);
    let peak_db = sample_to_dbfs(peak as f64);
    (rms_db, peak_db)
}

/// Convert an absolute s16 linear value into dBFS, clamped at -120.
fn sample_to_dbfs(v: f64) -> f64 {
    if v <= 0.5 {
        return -120.0;
    }
    let db = 20.0 * (v / 32768.0).log10();
    if db < -120.0 { -120.0 } else { db }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_pcm(dir: &TempDir, name: &str, samples: &[i16]) -> PathBuf {
        let path = dir.path().join(name);
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn silence_reports_floor_level() {
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; 1024]);
        let mut cursor = 0u64;
        let m = read_and_measure(&path, 16000, &mut cursor);
        assert_eq!(m.level["available"], true);
        assert_eq!(m.level["rms_db"], -120.0);
        assert_eq!(m.level["peak_db"], -120.0);
        assert!(m.pcm_bytes.as_ref().is_some_and(|b| !b.is_empty()));
    }

    #[test]
    fn full_scale_sine_peaks_near_zero_dbfs() {
        let dir = TempDir::new().unwrap();
        // A full-scale square wave — peak = 32767, RMS = 32767.
        let samples: Vec<i16> = (0..1024)
            .map(|i| if i % 2 == 0 { 32767 } else { -32767 })
            .collect();
        let path = write_pcm(&dir, "fullscale.raw", &samples);
        let mut cursor = 0u64;
        let m = read_and_measure(&path, 16000, &mut cursor);
        let peak = m.level["peak_db"].as_f64().unwrap();
        let rms = m.level["rms_db"].as_f64().unwrap();
        // Square at full scale: peak ~ 0 dBFS, RMS ~ 0 dBFS.
        assert!(peak > -0.1, "peak_db = {peak}");
        assert!(rms > -0.1, "rms_db = {rms}");
    }

    #[test]
    fn half_scale_sine_rms_is_minus_nine_dbfs_ish() {
        // A 50%-amplitude square wave. RMS = 16384; 20*log10(16384/32768) = -6.02 dBFS.
        let dir = TempDir::new().unwrap();
        let samples: Vec<i16> = (0..1024)
            .map(|i| if i % 2 == 0 { 16384 } else { -16384 })
            .collect();
        let path = write_pcm(&dir, "halfscale.raw", &samples);
        let mut cursor = 0u64;
        let m = read_and_measure(&path, 16000, &mut cursor);
        let rms = m.level["rms_db"].as_f64().unwrap();
        assert!((rms + 6.02).abs() < 0.1, "expected ~-6.02 dBFS, got {rms}");
    }

    #[test]
    fn missing_source_emits_unavailable() {
        let m = read_and_measure(Path::new("/nonexistent/weftos/mic.raw"), 16000, &mut 0u64);
        assert_eq!(m.level["available"], false);
        assert_eq!(m.level["reason"], "source-missing");
        assert_eq!(m.level["characterization"], Characterization::Rate.as_str());
        assert!(m.pcm_bytes.is_none());
    }

    #[test]
    fn cursor_advances_between_reads() {
        let dir = TempDir::new().unwrap();
        // Two full windows back-to-back: first silent, second
        // full-scale. Using exactly WINDOW_SAMPLES per window so each
        // read consumes one window's worth and the next read picks up
        // from the right offset.
        let mut samples: Vec<i16> = vec![0i16; WINDOW_SAMPLES];
        samples.extend((0..WINDOW_SAMPLES).map(|i| if i % 2 == 0 { 32767 } else { -32767 }));
        let path = write_pcm(&dir, "twohalf.raw", &samples);

        let mut cursor = 0u64;
        let v1 = read_and_measure(&path, 16000, &mut cursor);
        let v2 = read_and_measure(&path, 16000, &mut cursor);
        // First window is silent (-120).
        assert_eq!(v1.level["rms_db"], -120.0);
        // Second window jumps to ~0 dBFS.
        let second_rms = v2.level["rms_db"].as_f64().unwrap();
        assert!(second_rms > -0.1, "second rms_db = {second_rms}");
    }

    #[test]
    fn build_pcm_chunk_shape_is_whisper_compatible() {
        // Round-trip: samples → build_pcm_chunk → fields whisper expects.
        let samples: Vec<u8> = (0i16..4).flat_map(|s| s.to_le_bytes()).collect();
        let chunk = build_pcm_chunk(&samples, 16_000, 3, 7);
        assert_eq!(chunk["sample_rate"], 16_000);
        assert_eq!(chunk["channels"], 1);
        assert_eq!(chunk["samples"], 4);
        assert_eq!(chunk["seq"], 3);
        assert_eq!(chunk["chunk_ms"], TICK_MS);
        assert_eq!(chunk["tick"], 7);
        assert_eq!(chunk["encoding"], "base64");
        assert_eq!(chunk["format"], "i16le");
        assert_eq!(chunk["data"], chunk["pcm_b64"]);
        let decoded = B64.decode(chunk["data"].as_str().unwrap()).unwrap();
        assert_eq!(decoded, samples);
    }

    #[tokio::test]
    async fn adapter_open_unknown_topic_errors() {
        let a = MicrophoneAdapter::new();
        let r = a.open("substrate/sensor/bogus", Value::Null).await;
        assert!(matches!(r, Err(AdapterError::UnknownTopic(_))));
    }

    #[tokio::test]
    async fn adapter_open_accepts_legacy_and_canonical_level_and_pcm_topics() {
        let a = MicrophoneAdapter::new();
        for topic in [
            LEGACY_MIC_PATH,
            LEGACY_MIC_PCM_PATH,
            HOST_LOCAL_MIC_SUMMARY,
            HOST_LOCAL_MIC_RMS,
            HOST_LOCAL_MIC_PCM_CHUNK,
            "substrate/n-other/sensor/mic/summary",
            "substrate/n-other/sensor/mic/pcm_chunk",
        ] {
            let r = a.open(topic, Value::Null).await;
            assert!(r.is_ok(), "open({topic}) should succeed: {r:?}");
        }
    }

    #[test]
    fn physical_trait_declares_rate_characterization() {
        let a = MicrophoneAdapter::new();
        // The point of this test: the sensor honestly declares it
        // can't discriminate signal content — only magnitude.
        assert_eq!(a.characterization(), Characterization::Rate);
        assert_eq!(a.unit(), "dBFS");
    }

    #[test]
    fn file_backed_interface_roundtrips_path() {
        let a = MicrophoneAdapter::with_source(PathBuf::from("/tmp/weftos/demo.raw"), 48000);
        match a.interface() {
            SensorInterface::FileBacked { path } => {
                assert_eq!(path, PathBuf::from("/tmp/weftos/demo.raw"));
            }
            other => panic!("expected FileBacked, got {other:?}"),
        }
    }

    #[test]
    fn host_audio_direction_variant_compiles() {
        // Sanity check that `AudioDirection::Capture` is exposed for
        // the future CPAL bridge — no logic, just that the variant
        // round-trips.
        let d = AudioDirection::Capture;
        assert_eq!(d, AudioDirection::Capture);
    }

    #[test]
    fn declares_payload_pcm_and_healthcheck_topics() {
        // WEFT-418: host-local summary + rms + pcm_chunk + legacy flats + healthcheck.
        let paths: Vec<&str> = TOPICS.iter().map(|t| t.path).collect();
        assert!(paths.contains(&HOST_LOCAL_MIC_SUMMARY));
        assert!(paths.contains(&HOST_LOCAL_MIC_RMS));
        assert!(paths.contains(&HOST_LOCAL_MIC_PCM_CHUNK));
        assert!(paths.contains(&LEGACY_MIC_PATH));
        assert!(paths.contains(&LEGACY_MIC_PCM_PATH));
        assert!(paths.contains(&"substrate/meta/adapter/mic/healthcheck"));

        let pcm = TOPICS
            .iter()
            .find(|t| t.path == HOST_LOCAL_MIC_PCM_CHUNK)
            .expect("pcm topic");
        assert_eq!(pcm.buffer_policy, BufferPolicy::DropOldest);
        assert_eq!(pcm.max_len, Some(MIC_PCM_WINDOW_MAX_LEN));
    }

    #[tokio::test]
    async fn open_succeeds_for_healthcheck_topic() {
        // WEFT-432: subscribers can open the healthcheck topic directly.
        let a = MicrophoneAdapter::new();
        let r = a
            .open("substrate/meta/adapter/mic/healthcheck", Value::Null)
            .await;
        assert!(r.is_ok(), "open(healthcheck) should succeed: {r:?}");
    }

    #[tokio::test]
    async fn poller_emits_initial_unknown_health_report() {
        // WEFT-432: the contract calls for an `unknown` pre-first-emit
        // report so subscribers see the path populated immediately.
        // Verify it's the very first delta the poller produces.
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; 1024]);
        let a = MicrophoneAdapter::with_source(path, 16000);
        let mut sub = a
            .open(HOST_LOCAL_MIC_SUMMARY, Value::Null)
            .await
            .expect("open");

        // The very first delta must be the unknown-status health
        // report — sent before the ticker has fired.
        let first = tokio::time::timeout(std::time::Duration::from_millis(500), sub.rx.recv())
            .await
            .expect("recv timed out")
            .expect("channel closed");
        match first {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/meta/adapter/mic/healthcheck");
                assert_eq!(value["status"], "unknown");
                assert_eq!(value["configured_rate_hz"], 1000.0 / TICK_MS as f64);
                assert_eq!(value["error_count"], 0);
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn poller_dual_emits_summary_rms_and_legacy() {
        // WEFT-438: one level tick writes summary + rms (+ legacy).
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; WINDOW_SAMPLES]);
        let a = MicrophoneAdapter::with_source(path, 16000);
        let mut sub = a
            .open(HOST_LOCAL_MIC_SUMMARY, Value::Null)
            .await
            .expect("open");

        // Drain the initial health report.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sub.rx.recv())
            .await
            .expect("recv timed out")
            .expect("channel closed");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut saw_summary = false;
        let mut saw_rms = false;
        let mut saw_legacy = false;
        while std::time::Instant::now() < deadline {
            let delta =
                tokio::time::timeout(std::time::Duration::from_millis(700), sub.rx.recv()).await;
            let Ok(Some(delta)) = delta else { continue };
            if let StateDelta::Replace { path, value } = delta {
                if path == HOST_LOCAL_MIC_SUMMARY {
                    assert_eq!(value["available"], true);
                    assert!(value.get("rms_db").is_some());
                    saw_summary = true;
                } else if path == HOST_LOCAL_MIC_RMS {
                    assert!(value.as_f64().is_some() || value.as_i64().is_some());
                    saw_rms = true;
                } else if path == LEGACY_MIC_PATH {
                    assert_eq!(value["available"], true);
                    saw_legacy = true;
                }
            }
            if saw_summary && saw_rms && saw_legacy {
                break;
            }
        }
        assert!(
            saw_summary && saw_rms && saw_legacy,
            "expected dual-emit summary+rms+legacy (got summary={saw_summary} rms={saw_rms} legacy={saw_legacy})"
        );
    }

    #[tokio::test]
    async fn poller_appends_pcm_chunk_on_node_scoped_path() {
        // WEFT-418: one tick Appends a PCM window to
        // substrate/<node>/sensor/mic/pcm_chunk (+ legacy when dual-emit on).
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; WINDOW_SAMPLES]);
        let a = MicrophoneAdapter::with_source(path, 16000);
        let mut sub = a
            .open(HOST_LOCAL_MIC_PCM_CHUNK, Value::Null)
            .await
            .expect("open");

        // Drain the initial health report.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sub.rx.recv())
            .await
            .expect("recv timed out")
            .expect("channel closed");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut saw_canonical_pcm = false;
        let mut saw_legacy_pcm = false;
        while std::time::Instant::now() < deadline {
            let delta =
                tokio::time::timeout(std::time::Duration::from_millis(700), sub.rx.recv()).await;
            let Ok(Some(delta)) = delta else { continue };
            if let StateDelta::Append { path, value } = delta {
                if path == HOST_LOCAL_MIC_PCM_CHUNK {
                    assert!(value.get("pcm_b64").is_some() || value.get("data").is_some());
                    assert_eq!(value["sample_rate"], 16_000);
                    assert_eq!(value["channels"], 1);
                    assert_eq!(value["chunk_ms"], TICK_MS);
                    assert!(value["seq"].as_u64().unwrap_or(0) >= 1);
                    assert_eq!(value["samples"], WINDOW_SAMPLES as u64);
                    saw_canonical_pcm = true;
                } else if path == LEGACY_MIC_PCM_PATH {
                    saw_legacy_pcm = true;
                }
            }
            if saw_canonical_pcm && saw_legacy_pcm {
                break;
            }
        }
        assert!(
            saw_canonical_pcm && saw_legacy_pcm,
            "expected Append on canonical+legacy pcm (canonical={saw_canonical_pcm} legacy={saw_legacy_pcm})"
        );
    }

    #[tokio::test]
    async fn poller_skips_legacy_pcm_when_dual_emit_disabled() {
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; WINDOW_SAMPLES]);
        let a = MicrophoneAdapter::with_source(path, 16000).with_dual_emit_legacy(false);
        let mut sub = a
            .open(HOST_LOCAL_MIC_PCM_CHUNK, Value::Null)
            .await
            .expect("open");

        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sub.rx.recv())
            .await
            .expect("recv")
            .expect("channel");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut saw_canonical = false;
        let mut saw_legacy = false;
        while std::time::Instant::now() < deadline {
            let delta =
                tokio::time::timeout(std::time::Duration::from_millis(700), sub.rx.recv()).await;
            let Ok(Some(delta)) = delta else { continue };
            if let StateDelta::Append { path, .. } = delta {
                if path == HOST_LOCAL_MIC_PCM_CHUNK {
                    saw_canonical = true;
                }
                if path == LEGACY_MIC_PCM_PATH {
                    saw_legacy = true;
                }
            }
            if saw_canonical {
                let extra = tokio::time::timeout(
                    std::time::Duration::from_millis(600),
                    sub.rx.recv(),
                )
                .await;
                if let Ok(Some(StateDelta::Append { path, .. })) = extra
                    && path == LEGACY_MIC_PCM_PATH
                {
                    saw_legacy = true;
                }
                break;
            }
        }
        assert!(saw_canonical, "expected canonical pcm Append");
        assert!(!saw_legacy, "legacy pcm must stay silent when dual-emit off");
    }

    #[tokio::test]
    async fn poller_skips_legacy_when_dual_emit_disabled() {
        let dir = TempDir::new().unwrap();
        let path = write_pcm(&dir, "silence.raw", &[0i16; WINDOW_SAMPLES]);
        let a = MicrophoneAdapter::with_source(path, 16000).with_dual_emit_legacy(false);
        let mut sub = a
            .open(HOST_LOCAL_MIC_SUMMARY, Value::Null)
            .await
            .expect("open");

        // Drain initial health.
        let _ = tokio::time::timeout(std::time::Duration::from_millis(500), sub.rx.recv())
            .await
            .expect("recv")
            .expect("channel");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut saw_summary = false;
        let mut saw_legacy = false;
        while std::time::Instant::now() < deadline {
            let delta =
                tokio::time::timeout(std::time::Duration::from_millis(700), sub.rx.recv()).await;
            let Ok(Some(delta)) = delta else { continue };
            if let StateDelta::Replace { path, .. } = delta {
                if path == HOST_LOCAL_MIC_SUMMARY {
                    saw_summary = true;
                }
                if path == LEGACY_MIC_PATH {
                    saw_legacy = true;
                }
            }
            if saw_summary {
                // Give one more tick window to catch a stray legacy emit.
                let extra = tokio::time::timeout(
                    std::time::Duration::from_millis(600),
                    sub.rx.recv(),
                )
                .await;
                if let Ok(Some(StateDelta::Replace { path, .. })) = extra
                    && path == LEGACY_MIC_PATH
                {
                    saw_legacy = true;
                }
                break;
            }
        }
        assert!(saw_summary, "expected canonical summary emit");
        assert!(!saw_legacy, "legacy path must stay silent when dual-emit off");
    }

    #[tokio::test]
    async fn poller_emits_degraded_health_report_when_source_missing() {
        // WEFT-432: when the backing audio source is missing, the
        // poller must surface that on the healthcheck path (status:
        // degraded — error counter increments while observed rate
        // drops to zero). This is honest reporting per the contract:
        // the sensor *can* tell you it tried to read and failed, even
        // before the §4.2 "down for 10s" threshold trips.
        // Level paths are NOT overwritten when the source is missing
        // (covered by `missing_source_emits_unavailable`).
        let nonexistent = PathBuf::from("/nonexistent/weftos/mic-test.raw");
        let a = MicrophoneAdapter::with_source(nonexistent, 16000);
        let mut sub = a
            .open("substrate/meta/adapter/mic/healthcheck", Value::Null)
            .await
            .expect("open");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut got_unhealthy = false;
        while std::time::Instant::now() < deadline {
            let delta =
                tokio::time::timeout(std::time::Duration::from_millis(700), sub.rx.recv()).await;
            let Ok(Some(delta)) = delta else { continue };
            if let StateDelta::Replace { path, value } = delta
                && path == "substrate/meta/adapter/mic/healthcheck"
                && (value["status"] == "degraded" || value["status"] == "down")
                && value["error_count"].as_u64().unwrap_or(0) > 0
            {
                got_unhealthy = true;
                break;
            }
        }
        assert!(
            got_unhealthy,
            "expected at least one degraded/down-status health report with error_count > 0"
        );
    }
}
