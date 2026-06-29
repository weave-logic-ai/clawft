//! Core bridge: cpal device I/O, streaming resampling, AEC, stdio glue.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::consts::TARGET_SR;
use crate::dsp::{APM_FRAME, APM_SR, Aec, LinResampler, make_aec};

/// Set from the SIGUSR1 handler; polled by the flush monitor thread.
static FLUSH_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigusr1(_sig: libc::c_int) {
    // Async-signal-safe: only touch an atomic.
    FLUSH_REQUESTED.store(true, Ordering::SeqCst);
}

/// Bridge configuration (from CLI args / env).
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Input (mic) device name substring; `None` = system default input.
    pub input_device: Option<String>,
    /// Output (speaker) device name substring; `None` = system default.
    pub output_device: Option<String>,
}

fn elog(s: &str) {
    let _ = writeln!(std::io::stderr(), "{s}");
}

/// Shared FIFO of `f32` samples guarded by a mutex (small + simple; the
/// audio thread does at most a lock + extend/drain per callback).
type SampleQ = Arc<Mutex<VecDeque<f32>>>;

fn newq() -> SampleQ {
    Arc::new(Mutex::new(VecDeque::new()))
}

/// Pick a cpal device by name substring (case-insensitive) within the
/// given direction, or the system default if `want` is `None`.
fn pick_device(
    host: &cpal::Host,
    want: &Option<String>,
    input: bool,
) -> Result<cpal::Device, String> {
    if let Some(sub) = want {
        let sub_l = sub.to_lowercase();
        let devs = if input {
            host.input_devices()
        } else {
            host.output_devices()
        }
        .map_err(|e| e.to_string())?;
        for d in devs {
            if let Ok(name) = d.name()
                && name.to_lowercase().contains(&sub_l)
            {
                return Ok(d);
            }
        }
        return Err(format!("no {} device matching {sub:?}", dir(input)));
    }
    if input {
        host.default_input_device()
    } else {
        host.default_output_device()
    }
    .ok_or_else(|| format!("no default {} device", dir(input)))
}

fn dir(input: bool) -> &'static str {
    if input { "input" } else { "output" }
}

/// Build the cpal input stream: downmix any channel count to mono and
/// push raw `f32` (at the device rate) into `cap_q`.
fn build_input_stream(
    device: &cpal::Device,
    cap_q: SampleQ,
) -> Result<(cpal::Stream, u32), String> {
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let in_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let cfg: cpal::StreamConfig = supported.clone().into();
    let err_fn = |e| elog(&format!("[aec] input stream error: {e}"));

    let downmix_push = move |mono: Vec<f32>, q: &SampleQ| {
        if let Ok(mut g) = q.lock() {
            g.extend(mono);
        }
    };

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let q = cap_q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _| {
                    let mut mono = Vec::with_capacity(data.len() / channels.max(1) + 1);
                    for fr in data.chunks(channels.max(1)) {
                        let s: f32 = fr.iter().copied().sum::<f32>() / channels as f32;
                        mono.push(s);
                    }
                    downmix_push(mono, &q);
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
                    let mut mono = Vec::with_capacity(data.len() / channels.max(1) + 1);
                    for fr in data.chunks(channels.max(1)) {
                        let s: f32 = fr.iter().map(|&x| f32::from(x) / 32_768.0).sum::<f32>()
                            / channels as f32;
                        mono.push(s);
                    }
                    downmix_push(mono, &q);
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let q = cap_q.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[u16], _| {
                    let mut mono = Vec::with_capacity(data.len() / channels.max(1) + 1);
                    for fr in data.chunks(channels.max(1)) {
                        let s: f32 = fr
                            .iter()
                            .map(|&x| (f32::from(x) - 32_768.0) / 32_768.0)
                            .sum::<f32>()
                            / channels as f32;
                        mono.push(s);
                    }
                    downmix_push(mono, &q);
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("unsupported input sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;

    elog(&format!(
        "[aec] input device: {} @ {in_rate} Hz, {channels} ch -> downmix to mono",
        device.name().unwrap_or_else(|_| "?".into())
    ));
    Ok((stream, in_rate))
}

/// Build the cpal output stream: pull mono `f32` (at the device rate) from
/// `play_q` and fan it out to all output channels.
fn build_output_stream(
    device: &cpal::Device,
    play_q: SampleQ,
) -> Result<(cpal::Stream, u32), String> {
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let out_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let cfg: cpal::StreamConfig = supported.clone().into();
    let err_fn = |e| elog(&format!("[aec] output stream error: {e}"));

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let q = play_q.clone();
            device.build_output_stream(
                &cfg,
                move |out: &mut [f32], _| {
                    let mut g = q.lock().unwrap();
                    for fr in out.chunks_mut(channels.max(1)) {
                        let s = g.pop_front().unwrap_or(0.0);
                        for slot in fr.iter_mut() {
                            *slot = s;
                        }
                    }
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let q = play_q.clone();
            device.build_output_stream(
                &cfg,
                move |out: &mut [i16], _| {
                    let mut g = q.lock().unwrap();
                    for fr in out.chunks_mut(channels.max(1)) {
                        let s = g.pop_front().unwrap_or(0.0);
                        let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                        for slot in fr.iter_mut() {
                            *slot = v;
                        }
                    }
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("unsupported output sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;

    elog(&format!(
        "[aec] output device: {} @ {out_rate} Hz, {channels} ch",
        device.name().unwrap_or_else(|_| "?".into())
    ));
    Ok((stream, out_rate))
}

/// Run the bridge. Blocks forever (until killed / stdin EOF + SIGINT).
pub fn run(cfg: Config) -> Result<(), String> {
    // Install the SIGUSR1 barge-in flush handler.
    unsafe {
        libc::signal(libc::SIGUSR1, on_sigusr1 as *const () as usize);
    }

    let host = cpal::default_host();

    // Queues:
    //   cap_q    : mic mono @ in_rate   (input cb -> processing thread)
    //   play_q   : speaker mono @ out_rate (stdin thread -> output cb)
    //   render_q : far-end mono @ 16k  (stdin thread -> processing thread, AEC ref)
    let cap_q = newq();
    let play_q = newq();
    let render_q = newq();

    let in_dev = pick_device(&host, &cfg.input_device, true)?;
    let out_dev = pick_device(&host, &cfg.output_device, false)?;

    let (in_stream, in_rate) = build_input_stream(&in_dev, cap_q.clone())?;
    let (out_stream, out_rate) = build_output_stream(&out_dev, play_q.clone())?;

    in_stream.play().map_err(|e| e.to_string())?;
    out_stream.play().map_err(|e| e.to_string())?;

    let aec = make_aec();
    elog(&format!(
        "[aec] engine started — capture {in_rate} Hz -> 16k mono; playback 16k -> {out_rate} Hz; \
         wire int16 mono {TARGET_SR} Hz; {}",
        aec.label()
    ));

    // stdin reader: int16 mono 16k -> render_q (16k ref) + resample to play_q (out_rate).
    {
        let play_q = play_q.clone();
        let render_q = render_q.clone();
        thread::spawn(move || stdin_loop(out_rate, play_q, render_q));
    }

    // flush monitor: on SIGUSR1, drop queued playback + render reference.
    {
        let play_q = play_q.clone();
        let render_q = render_q.clone();
        thread::spawn(move || {
            loop {
                if FLUSH_REQUESTED.swap(false, Ordering::SeqCst) {
                    if let Ok(mut g) = play_q.lock() {
                        g.clear();
                    }
                    if let Ok(mut g) = render_q.lock() {
                        g.clear();
                    }
                    elog("[aec] flush — playback + render reference cleared.");
                }
                thread::sleep(Duration::from_millis(5));
            }
        });
    }

    // processing thread: cap_q -> resample to 16k -> 160-sample frames ->
    // AEC (render+capture) -> int16 stdout.
    let proc = thread::spawn(move || processing_loop(in_rate, cap_q, render_q, aec));

    // Keep the streams alive on this (the cpal-owning) thread.
    let _ = proc.join();
    drop(in_stream);
    drop(out_stream);
    Ok(())
}

fn stdin_loop(out_rate: u32, play_q: SampleQ, render_q: SampleQ) {
    let mut up_play = LinResampler::new(TARGET_SR, out_rate); // 16k -> device rate (speaker)
    let mut up_ref = LinResampler::new(TARGET_SR, APM_SR); // 16k -> 48k (APM render reference)
    let mut stdin = std::io::stdin().lock();
    let mut raw = vec![0u8; 8192];
    let mut leftover: Vec<u8> = Vec::new();
    let mut play_out: Vec<f32> = Vec::new();
    let mut ref_out: Vec<f32> = Vec::new();
    loop {
        let n = match stdin.read(&mut raw) {
            Ok(0) => {
                elog("[aec] stdin EOF — playback input closed.");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                elog(&format!("[aec] stdin read error: {e}"));
                break;
            }
        };
        let mut bytes = std::mem::take(&mut leftover);
        bytes.extend_from_slice(&raw[..n]);
        let usable = bytes.len() - (bytes.len() % 2);
        let samples: Vec<f32> = bytes[..usable]
            .chunks_exact(2)
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32_768.0)
            .collect();
        leftover = bytes[usable..].to_vec();
        if samples.is_empty() {
            continue;
        }
        // AEC render reference: resample 16k -> 48k (APM rate).
        ref_out.clear();
        up_ref.process(&samples, &mut ref_out);
        if let Ok(mut g) = render_q.lock() {
            g.extend(ref_out.iter().copied());
        }
        // Playback: resample 16k -> device rate, enqueue for the output cb.
        play_out.clear();
        up_play.process(&samples, &mut play_out);
        if let Ok(mut g) = play_q.lock() {
            g.extend(play_out.iter().copied());
        }
    }
}

fn processing_loop(in_rate: u32, cap_q: SampleQ, render_q: SampleQ, mut aec: Box<dyn Aec>) {
    let mut up_cap = LinResampler::new(in_rate, APM_SR); // mic device rate -> 48k (APM)
    let mut down_wire = LinResampler::new(APM_SR, TARGET_SR); // 48k cleaned -> 16k wire
    let mut stdout = std::io::stdout().lock();
    let mut pending48k: Vec<f32> = Vec::new(); // 48k mono awaiting 480-sample framing
    let mut chunk: Vec<f32> = Vec::new();
    let mut cap_up: Vec<f32> = Vec::new();
    let mut render_frame = vec![0.0f32; APM_FRAME];
    let mut wire_out: Vec<f32> = Vec::new();
    let mut out_bytes: Vec<u8> = Vec::with_capacity(APM_FRAME * 2);

    loop {
        // Drain whatever the mic callback has produced, resample to 48k.
        chunk.clear();
        {
            let mut g = cap_q.lock().unwrap();
            chunk.extend(g.drain(..));
        }
        if !chunk.is_empty() {
            cap_up.clear();
            up_cap.process(&chunk, &mut cap_up);
            pending48k.extend(cap_up.iter().copied());
        }

        // Run the APM on each 10 ms (480-sample @48k) frame.
        while pending48k.len() >= APM_FRAME {
            let mut cap_frame: Vec<f32> = pending48k.drain(..APM_FRAME).collect();

            // Pull the matching 48k render reference (zeros if none queued).
            for s in render_frame.iter_mut() {
                *s = 0.0;
            }
            {
                let mut g = render_q.lock().unwrap();
                let take = g.len().min(APM_FRAME);
                for slot in render_frame.iter_mut().take(take) {
                    *slot = g.pop_front().unwrap();
                }
            }

            aec.render(&render_frame);
            aec.capture(&mut cap_frame);

            // Resample the cleaned 48k capture down to the 16k wire format.
            wire_out.clear();
            down_wire.process(&cap_frame, &mut wire_out);
            out_bytes.clear();
            for &s in &wire_out {
                let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
                out_bytes.extend_from_slice(&v.to_le_bytes());
            }
            if stdout.write_all(&out_bytes).is_err() {
                return; // stdout closed -> consumer gone
            }
            let _ = stdout.flush();
        }

        if chunk.is_empty() {
            thread::sleep(Duration::from_millis(2));
        }
    }
}
