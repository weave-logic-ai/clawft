//! In-process cpal device I/O wired to the AEC and the `clawft-channels`
//! voice seams (ADR-062 Phase 5).
//!
//! Two concrete I/O paths that the stdio [`bridge`](crate::bridge) does not
//! cover, for a consumer (`clawft-voice-talk`) that owns its own capture +
//! playback:
//!
//! - **[`AecTtsSink`]** — a [`TtsSink`] that plays streamed [`TtsChunk`]s
//!   gap-free *and* pushes every played sample into the shared
//!   [`AecProcessor`] as the **render reference**, so the mic path cancels
//!   the bot's own voice. [`flush`](TtsSink::flush) drops queued + in-flight
//!   playback and the stale render reference together (barge-in).
//! - **[`run_capture`]** — cpal mic → downmix → 16 kHz → the *same*
//!   [`AecProcessor`] (`process_capture`) → a [`CaptureProcessor`], which
//!   runs the VAD/endpoint gate and emits impulses. Sharing one
//!   `Arc<Mutex<AecProcessor>>` between the sink and the capture loop is what
//!   makes echo cancellation work: playback feeds the reference, capture
//!   subtracts it.
//!
//! This module is gated behind the `device` feature (it pulls cpal + the
//! channels voice traits). The default workspace build never compiles it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use clawft_channels::voice::{CaptureProcessor, TtsChunk, TtsSink, VoiceError};

use crate::consts::TARGET_SR;
use crate::dsp::{AecProcessor, LinResampler};

/// cpal output `TtsSink` that doubles as the AEC render-reference feeder.
///
/// Played audio is buffered at 16 kHz in [`play_queue`](Self::play_queue)
/// (the live output stream drains it, resampling to the device rate) and is
/// simultaneously pushed into the shared [`AecProcessor`] so the captured
/// mic cancels the echo. Barge-in [`flush`](TtsSink::flush) clears both.
pub struct AecTtsSink {
    /// 16 kHz mono playback queue drained by the cpal output stream.
    play_q: Arc<Mutex<VecDeque<i16>>>,
    /// Shared AEC processor — played audio becomes the render reference.
    aec: Arc<Mutex<AecProcessor>>,
    /// Lazy input→16 kHz resampler for chunks not already at 16 kHz; keyed
    /// by input rate so a constant-rate stream stays gap-free.
    resampler: Mutex<Option<(u32, LinResampler)>>,
    /// Wall-clock instant when everything queued so far will have left the
    /// speaker. Audio plays in real time, so queue length ÷ 16 kHz extends
    /// this deadline on every `play_chunk`; `wait_drained` sleeps until it
    /// passes. (Polling `play_q` is NOT a drain signal — the output stream
    /// slurps it into an internal buffer far ahead of real time.)
    playback_until: Mutex<std::time::Instant>,
}

impl AecTtsSink {
    /// Build a sink sharing `aec` with the capture loop. Pass the same
    /// `Arc<Mutex<AecProcessor>>` to [`run_capture`] so echo cancellation
    /// closes the loop.
    pub fn new(aec: Arc<Mutex<AecProcessor>>) -> Self {
        Self {
            play_q: Arc::new(Mutex::new(VecDeque::new())),
            aec,
            resampler: Mutex::new(None),
            playback_until: Mutex::new(std::time::Instant::now()),
        }
    }

    /// Shared handle to the 16 kHz playback queue. The live output stream
    /// drains it; tests inspect it as the stand-in "device".
    pub fn play_queue(&self) -> Arc<Mutex<VecDeque<i16>>> {
        self.play_q.clone()
    }

    /// Resample a chunk to 16 kHz mono `i16` (identity at 16 kHz).
    fn to_16k(&self, chunk: &TtsChunk) -> Vec<i16> {
        if chunk.sample_rate == TARGET_SR {
            return chunk.samples.clone();
        }
        let f: Vec<f32> = chunk
            .samples
            .iter()
            .map(|&s| f32::from(s) / 32_768.0)
            .collect();
        let mut guard = self.resampler.lock().unwrap();
        let stale = guard.as_ref().is_none_or(|(r, _)| *r != chunk.sample_rate);
        if stale {
            *guard = Some((
                chunk.sample_rate,
                LinResampler::new(chunk.sample_rate, TARGET_SR),
            ));
        }
        let mut out = Vec::new();
        guard.as_mut().unwrap().1.process(&f, &mut out);
        out.iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32_767.0) as i16)
            .collect()
    }
}

#[async_trait]
impl TtsSink for AecTtsSink {
    async fn play_chunk(&self, chunk: &TtsChunk) -> Result<(), VoiceError> {
        let pcm = self.to_16k(chunk);
        if pcm.is_empty() {
            return Ok(());
        }
        // Feed the render reference first so the canceller already has the
        // played audio when the mic picks up its echo.
        self.aec.lock().unwrap().push_render(&pcm);
        let dur = std::time::Duration::from_micros(pcm.len() as u64 * 1_000_000 / u64::from(TARGET_SR));
        self.play_q.lock().unwrap().extend(pcm);
        {
            let mut until = self.playback_until.lock().unwrap();
            let base = (*until).max(std::time::Instant::now());
            *until = base + dur;
        }
        Ok(())
    }

    async fn flush(&self) {
        self.play_q.lock().unwrap().clear();
        self.aec.lock().unwrap().flush();
        *self.playback_until.lock().unwrap() = std::time::Instant::now();
    }

    async fn wait_drained(&self) {
        // Sleep until the queued-audio deadline passes, then a short
        // hangover for the DAC/room reverb to go quiet so the tail of our
        // own speech cannot be captured as a user turn.
        loop {
            let until = *self.playback_until.lock().unwrap();
            let now = std::time::Instant::now();
            if now >= until {
                break;
            }
            tokio::time::sleep((until - now).min(std::time::Duration::from_millis(50))).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Shared mono-`f32` FIFO (cpal callback → processing loop).
type SampleQ = Arc<Mutex<VecDeque<f32>>>;

/// Pick a cpal input device by case-insensitive name substring, or the
/// system default.
/// Names of all available input devices on the default host, with the
/// default device (the one [`run_capture`] opens when no substring is
/// given) first when identifiable.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let mut names: Vec<String> = host
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();
    if let Some(def) = default_name {
        names.retain(|n| *n != def);
        names.insert(0, format!("{def} (default)"));
    }
    names
}

/// Report from [`mic_probe`]: which device was opened and what raw input
/// levels it saw. No AEC, no VAD — straight off the stream, for diagnosing
/// "it doesn't hear me".
#[derive(Debug)]
pub struct MicProbeReport {
    /// Device name actually opened (same selection as [`run_capture`]).
    pub device: String,
    /// The device's native sample rate.
    pub native_rate: u32,
    /// The device's channel count (downmixed for level math).
    pub channels: u16,
    /// RMS level in dBFS per ~500 ms window, in capture order.
    pub windows_dbfs: Vec<f32>,
    /// Loudest window seen.
    pub peak_dbfs: f32,
}

/// Open the same input device [`run_capture`] would and measure raw levels
/// for `seconds`. Blocking — call from a blocking task.
pub fn mic_probe(input_device: Option<&str>, seconds: u32) -> Result<MicProbeReport, String> {
    let host = cpal::default_host();
    let device = pick_input(&host, input_device)?;
    let name = device.name().unwrap_or_else(|_| "<unknown>".into());
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let native_rate = supported.sample_rate().0;
    let channels = supported.channels().max(1);
    let cfg: cpal::StreamConfig = supported.clone().into();

    let q: SampleQ = Arc::new(Mutex::new(VecDeque::new()));
    let n_ch = channels as usize;
    let err_fn = |e| tracing::warn!("[mic-probe] input stream error: {e}");
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let q = q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _| {
                    let mut g = q.lock().unwrap();
                    g.extend(
                        data.chunks(n_ch)
                            .map(|fr| fr.iter().copied().sum::<f32>() / n_ch as f32),
                    );
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let q = q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _| {
                    let mut g = q.lock().unwrap();
                    for fr in data.chunks(n_ch) {
                        let s: f32 = fr.iter().map(|&x| f32::from(x) / 32_768.0).sum::<f32>()
                            / n_ch as f32;
                        g.push_back(s);
                    }
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("unsupported input sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    let window = (native_rate as usize / 2).max(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds.into());
    let mut cur: Vec<f32> = Vec::new();
    let mut windows_dbfs: Vec<f32> = Vec::new();
    let mut peak_dbfs = f32::NEG_INFINITY;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
        {
            let mut g = q.lock().unwrap();
            cur.extend(g.drain(..));
        }
        while cur.len() >= window {
            let w: Vec<f32> = cur.drain(..window).collect();
            let rms = (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt();
            let db = 20.0 * rms.max(1e-9).log10();
            peak_dbfs = peak_dbfs.max(db);
            windows_dbfs.push(db);
        }
    }
    drop(stream);
    Ok(MicProbeReport {
        device: name,
        native_rate,
        channels,
        windows_dbfs,
        peak_dbfs,
    })
}

fn pick_input(host: &cpal::Host, want: Option<&str>) -> Result<cpal::Device, String> {
    if let Some(sub) = want {
        let sub_l = sub.to_lowercase();
        for d in host.input_devices().map_err(|e| e.to_string())? {
            if let Ok(name) = d.name()
                && name.to_lowercase().contains(&sub_l)
            {
                return Ok(d);
            }
        }
        return Err(format!("no input device matching {sub:?}"));
    }
    host.default_input_device()
        .ok_or_else(|| "no default input device".into())
}

/// Run the live capture path: cpal mic → AEC → [`CaptureProcessor`].
///
/// Blocks until `cancel` is set. Resamples the device stream to 16 kHz,
/// routes it through the shared [`AecProcessor`], and pushes each cleaned
/// frame into `processor` (which emits impulses + forwards frames). Intended
/// to be called from a blocking task — the cpal stream is kept alive on this
/// thread.
pub fn run_capture(
    input_device: Option<&str>,
    aec: Arc<Mutex<AecProcessor>>,
    mut processor: CaptureProcessor,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = pick_input(&host, input_device)?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let in_rate = supported.sample_rate().0;
    let channels = supported.channels().max(1) as usize;
    let cfg: cpal::StreamConfig = supported.clone().into();
    tracing::info!(
        device = %device.name().unwrap_or_else(|_| "<unknown>".into()),
        rate = in_rate,
        channels,
        "voice capture stream opening"
    );

    let cap_q: SampleQ = Arc::new(Mutex::new(VecDeque::new()));
    let err_fn = |e| tracing::warn!("[aec-device] input stream error: {e}");

    let downmix =
        move |frame: &[f32]| -> f32 { frame.iter().copied().sum::<f32>() / channels as f32 };

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let q = cap_q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _| {
                    let mut g = q.lock().unwrap();
                    g.extend(data.chunks(channels).map(&downmix));
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let q = cap_q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _| {
                    let mut g = q.lock().unwrap();
                    for fr in data.chunks(channels) {
                        let s: f32 = fr.iter().map(|&x| f32::from(x) / 32_768.0).sum::<f32>()
                            / channels as f32;
                        g.push_back(s);
                    }
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("unsupported input sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    let mut down_16k = LinResampler::new(in_rate, TARGET_SR);
    let mut scratch: Vec<f32> = Vec::new();
    let mut wire: Vec<f32> = Vec::new();
    // Pre/post-AEC level accounting, logged ~every 2 s. If post is materially
    // below pre, the AEC is attenuating the mic (the round-8 listen-vs-test-mic
    // delta); on the passthrough (listen-only) path they should track ±~1 dB.
    let mut pre_ss = 0.0f64;
    let mut pre_n = 0u64;
    let mut post_ss = 0.0f64;
    let mut post_n = 0u64;
    let mut last_rms_log = std::time::Instant::now();
    while !cancel.load(Ordering::Relaxed) {
        scratch.clear();
        {
            let mut g = cap_q.lock().unwrap();
            scratch.extend(g.drain(..));
        }
        if scratch.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }
        wire.clear();
        down_16k.process(&scratch, &mut wire);
        let mic: Vec<i16> = wire
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32_767.0) as i16)
            .collect();
        pre_ss += mic.iter().map(|&s| f64::from(s) * f64::from(s)).sum::<f64>();
        pre_n += mic.len() as u64;
        let cleaned = {
            let mut guard = aec.lock().unwrap();
            if guard.is_active() {
                guard.process_capture(&mic)
            } else {
                // No real echo canceller (passthrough / listen-only bypass):
                // skip the 16k→48k→16k APM round-trip entirely and forward the
                // mic straight through. Removes a pointless resample stage and a
                // suspect from the listen-vs-test-mic delta.
                mic
            }
        };
        post_ss += cleaned
            .iter()
            .map(|&s| f64::from(s) * f64::from(s))
            .sum::<f64>();
        post_n += cleaned.len() as u64;
        if last_rms_log.elapsed() >= std::time::Duration::from_secs(2) && pre_n > 0 && post_n > 0 {
            let pre_db = 20.0 * ((pre_ss / pre_n as f64).sqrt() / 32_768.0).log10();
            let post_db = 20.0 * ((post_ss / post_n as f64).sqrt() / 32_768.0).log10();
            tracing::info!(
                pre_aec_dbfs = pre_db,
                post_aec_dbfs = post_db,
                delta_db = post_db - pre_db,
                "capture level (pre vs post AEC)"
            );
            pre_ss = 0.0;
            pre_n = 0;
            post_ss = 0.0;
            post_n = 0;
            last_rms_log = std::time::Instant::now();
        }
        if !cleaned.is_empty() {
            processor.push(cleaned);
        }
    }
    drop(stream);
    Ok(())
}

/// Spawn the live cpal output stream that drains [`AecTtsSink::play_queue`],
/// resampling 16 kHz → the device rate. The returned stream must be held
/// alive for playback to continue.
pub fn spawn_output(sink: &AecTtsSink) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no default output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let out_rate = supported.sample_rate().0;
    let channels = supported.channels().max(1) as usize;
    let cfg: cpal::StreamConfig = supported.clone().into();
    let play_q = sink.play_queue();
    let mut up = LinResampler::new(TARGET_SR, out_rate);
    let mut buf: VecDeque<f32> = VecDeque::new();
    let mut pulled: Vec<f32> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();
    let err_fn = |e| tracing::warn!("[aec-device] output stream error: {e}");

    let mut fill = move |buf: &mut VecDeque<f32>| {
        pulled.clear();
        {
            let mut g = play_q.lock().unwrap();
            pulled.extend(g.drain(..).map(|s| f32::from(s) / 32_768.0));
        }
        if !pulled.is_empty() {
            resampled.clear();
            up.process(&pulled, &mut resampled);
            buf.extend(resampled.iter().copied());
        }
    };

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &cfg,
            move |out: &mut [f32], _| {
                fill(&mut buf);
                for fr in out.chunks_mut(channels) {
                    let s = buf.pop_front().unwrap_or(0.0);
                    fr.iter_mut().for_each(|slot| *slot = s);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &cfg,
            move |out: &mut [i16], _| {
                fill(&mut buf);
                for fr in out.chunks_mut(channels) {
                    let s = buf.pop_front().unwrap_or(0.0);
                    let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                    fr.iter_mut().for_each(|slot| *slot = v);
                }
            },
            err_fn,
            None,
        ),
        other => return Err(format!("unsupported output sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(v: i16, n: usize) -> TtsChunk {
        TtsChunk {
            samples: vec![v; n],
            sample_rate: TARGET_SR,
        }
    }

    #[tokio::test]
    async fn enqueue_preserves_order_and_feeds_render_ref() {
        let aec = Arc::new(Mutex::new(AecProcessor::new()));
        let sink = AecTtsSink::new(aec.clone());
        let q = sink.play_queue();

        sink.play_chunk(&chunk(1, 160)).await.unwrap();
        sink.play_chunk(&chunk(2, 160)).await.unwrap();

        {
            let g = q.lock().unwrap();
            assert_eq!(g.len(), 320, "both chunks enqueued");
            assert_eq!(g.front().copied(), Some(1), "first chunk plays first");
            assert_eq!(g.back().copied(), Some(2), "second chunk plays after");
        }
        assert!(
            aec.lock().unwrap().render_ref_len() > 0,
            "played audio must feed the AEC render reference"
        );
    }

    #[tokio::test]
    async fn flush_drops_playback_and_render_ref() {
        let aec = Arc::new(Mutex::new(AecProcessor::new()));
        let sink = AecTtsSink::new(aec.clone());
        let q = sink.play_queue();

        sink.play_chunk(&chunk(7, 320)).await.unwrap();
        assert!(!q.lock().unwrap().is_empty());
        assert!(aec.lock().unwrap().render_ref_len() > 0);

        sink.flush().await;
        assert!(q.lock().unwrap().is_empty(), "barge-in clears playback");
        assert_eq!(
            aec.lock().unwrap().render_ref_len(),
            0,
            "barge-in clears the stale render reference"
        );
    }

    #[tokio::test]
    async fn resamples_non_16k_chunks() {
        let aec = Arc::new(Mutex::new(AecProcessor::new()));
        let sink = AecTtsSink::new(aec);
        let q = sink.play_queue();
        // 24 kHz (SNAC) chunk → ~2/3 the samples at 16 kHz.
        sink.play_chunk(&TtsChunk {
            samples: vec![1_000i16; 2_400],
            sample_rate: 24_000,
        })
        .await
        .unwrap();
        let n = q.lock().unwrap().len();
        assert!(
            (1_500..=1_700).contains(&n),
            "24k→16k should yield ~1600 samples, got {n}"
        );
    }

    // Live device paths — require a real mic/speaker, so they are ignored in
    // CI/agent runtimes. Run with: `cargo test -p clawft-voice-aec --features
    // device,webrtc-aec -- --ignored`.
    #[test]
    #[ignore = "requires a live audio output device"]
    fn live_output_stream_starts() {
        let aec = Arc::new(Mutex::new(AecProcessor::new()));
        let sink = AecTtsSink::new(aec);
        let stream = spawn_output(&sink).expect("output stream");
        std::thread::sleep(std::time::Duration::from_millis(50));
        drop(stream);
    }

    #[test]
    #[ignore = "requires a live audio input device"]
    fn live_capture_starts_and_cancels() {
        use clawft_channels::voice::{EnergyVad, ImpulseSink, VoiceImpulse};
        struct Discard;
        impl ImpulseSink for Discard {
            fn emit(&self, _impulse: VoiceImpulse, _hlc: u64) {}
        }
        let aec = Arc::new(Mutex::new(AecProcessor::new()));
        let (tx, _rx) = tokio::sync::mpsc::channel::<Vec<i16>>(64);
        let vad = EnergyVad::new(TARGET_SR, -45.0, 300, 100, 10_000);
        let processor = CaptureProcessor::new(vad, Arc::new(Discard), tx);
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        let h = std::thread::spawn(move || run_capture(None, aec, processor, c2));
        std::thread::sleep(std::time::Duration::from_millis(100));
        cancel.store(true, Ordering::Relaxed);
        h.join().unwrap().expect("capture run");
    }
}
