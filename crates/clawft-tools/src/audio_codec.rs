//! Multi-format audio codecs for agent tools (WEFT-233).
//!
//! Decodes `.wav` / `.mp3` / `.ogg` / `.webm` into mono `s16le` PCM suitable
//! for the live STT path, and encodes PCM back to WAV for `audio_synthesize`.
//!
//! ## Formats
//!
//! | Format | Decode | Encode | Engine |
//! |--------|--------|--------|--------|
//! | WAV (PCM s16le) | yes | yes | `clawft_channels::voice::wav` (fast path) + Symphonia |
//! | MP3 | yes | no | Symphonia |
//! | OGG (Vorbis) | yes | no | Symphonia |
//! | WebM (Vorbis/PCM in Matroska) | yes when codec supported | no | Symphonia `mkv` |
//!
//! Opus-in-WebM is not supported by the enabled Symphonia features; callers
//! get a clear error. Prefer WAV for synthesis output.

use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

use clawft_channels::voice::wav::{pcm_s16le_to_wav, wav_to_pcm_s16le};
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::voice_backend::PcmAudio;

/// Supported container / codec families for tool I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// RIFF/WAV (PCM).
    Wav,
    /// MPEG-1/2 Layer III.
    Mp3,
    /// Ogg container (typically Vorbis).
    Ogg,
    /// WebM / Matroska container.
    Webm,
}

impl AudioFormat {
    /// MIME type advertised by the tools.
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Mp3 => "audio/mpeg",
            Self::Ogg => "audio/ogg",
            Self::Webm => "audio/webm",
        }
    }

    /// File extension without the leading dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
            Self::Webm => "webm",
        }
    }

    /// Resolve format from a file extension (case-insensitive).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "wav" | "wave" => Some(Self::Wav),
            "mp3" => Some(Self::Mp3),
            "ogg" | "oga" => Some(Self::Ogg),
            "webm" | "mkv" => Some(Self::Webm),
            _ => None,
        }
    }
}

/// Detect format from a path's extension.
pub fn format_from_path(path: &Path) -> Result<AudioFormat, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "File has no extension".to_string())?;
    AudioFormat::from_extension(ext).ok_or_else(|| {
        format!(
            "Unsupported audio format: .{ext} (supported: wav, mp3, ogg, webm)"
        )
    })
}

/// Decode an audio file to mono PCM.
///
/// WAV uses the fast channels helper when the buffer is standard s16le PCM;
/// other formats (and non-canonical WAV) go through Symphonia.
pub fn decode_file(path: &Path) -> Result<(PcmAudio, AudioFormat), String> {
    let format = format_from_path(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    let pcm = decode_bytes(&bytes, format)?;
    Ok((pcm, format))
}

/// Decode audio bytes for a known format into mono `s16le` PCM.
pub fn decode_bytes(bytes: &[u8], format: AudioFormat) -> Result<PcmAudio, String> {
    if bytes.is_empty() {
        return Err("audio file is empty".into());
    }

    // Fast path: canonical mono s16le WAV.
    if format == AudioFormat::Wav
        && let Ok((samples, sample_rate)) = wav_to_pcm_s16le(bytes)
    {
        return Ok(PcmAudio {
            samples,
            sample_rate,
        });
    }

    decode_with_symphonia(bytes, format)
}

/// Encode mono PCM to a WAV byte stream (RIFF s16le).
pub fn encode_wav(pcm: &PcmAudio) -> Vec<u8> {
    pcm_s16le_to_wav(&pcm.samples, pcm.sample_rate)
}

/// Write mono PCM to a `.wav` file.
pub fn write_wav_file(path: &Path, pcm: &PcmAudio) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(format!(
            "Output directory does not exist: {}",
            parent.display()
        ));
    }
    let bytes = encode_wav(pcm);
    let mut file = File::create(path).map_err(|e| format!("Failed to create WAV: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write WAV: {e}"))?;
    Ok(())
}

fn decode_with_symphonia(bytes: &[u8], format: AudioFormat) -> Result<PcmAudio, String> {
    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(format.extension());

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Failed to probe {} audio: {e}", format.extension()))?;

    let mut reader = probed.format;
    let track = reader
        .default_track()
        .ok_or_else(|| format!("No audio track in {} file", format.extension()))?
        .clone();

    if track.codec_params.codec == CODEC_TYPE_NULL {
        return Err(format!(
            "Unsupported or unknown codec in {} file",
            format.extension()
        ));
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| {
            format!(
                "Cannot decode {} track (codec may be unsupported, e.g. Opus-in-WebM): {e}",
                format.extension()
            )
        })?;

    let track_id = track.id;
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(16_000);
    let mut mono_f32: Vec<f32> = Vec::new();

    loop {
        let packet = match reader.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(e) => {
                // End-of-stream is reported variously; treat hard errors as fatal
                // only when we have no samples yet.
                if mono_f32.is_empty() {
                    return Err(format!("Decode error ({}): {e}", format.extension()));
                }
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => {
                if mono_f32.is_empty() {
                    return Err(format!("Decode error ({}): {e}", format.extension()));
                }
                break;
            }
        };

        sample_rate = decoded.spec().rate;
        append_mono_f32(decoded, &mut mono_f32);
    }

    if mono_f32.is_empty() {
        return Err(format!(
            "No PCM samples decoded from {} file",
            format.extension()
        ));
    }

    let samples: Vec<i16> = mono_f32
        .into_iter()
        .map(|s| {
            let c = s.clamp(-1.0, 1.0);
            (c * f32::from(i16::MAX)) as i16
        })
        .collect();

    Ok(PcmAudio {
        samples,
        sample_rate,
    })
}

fn append_mono_f32(buffer: AudioBufferRef<'_>, out: &mut Vec<f32>) {
    match buffer {
        AudioBufferRef::F32(buf) => mix_planes(buf.planes().planes(), out),
        AudioBufferRef::U8(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    // u8 PCM is offset binary.
                    sum += (f32::from(plane[i]) - 128.0) / 128.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::U16(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += (f32::from(plane[i]) - 32768.0) / 32768.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::U24(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    let v = plane[i].inner() as f32;
                    sum += (v - 8_388_608.0) / 8_388_608.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::U32(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += (plane[i] as f32 - 2_147_483_648.0) / 2_147_483_648.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::S8(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += f32::from(plane[i]) / 128.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::S16(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += f32::from(plane[i]) / f32::from(i16::MAX);
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::S24(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += plane[i].inner() as f32 / 8_388_607.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::S32(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += plane[i] as f32 / 2_147_483_647.0;
                }
                out.push(sum / chans.len() as f32);
            }
        }
        AudioBufferRef::F64(buf) => {
            let planes = buf.planes();
            let chans = planes.planes();
            if chans.is_empty() {
                return;
            }
            let frames = chans[0].len();
            for i in 0..frames {
                let mut sum = 0.0f32;
                for plane in chans {
                    sum += plane[i] as f32;
                }
                out.push(sum / chans.len() as f32);
            }
        }
    }
}

fn mix_planes(chans: &[&[f32]], out: &mut Vec<f32>) {
    if chans.is_empty() {
        return;
    }
    let frames = chans[0].len();
    for i in 0..frames {
        let mut sum = 0.0f32;
        for plane in chans {
            sum += plane[i];
        }
        out.push(sum / chans.len() as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_pcm(frames: usize, sample_rate: u32) -> PcmAudio {
        let samples: Vec<i16> = (0..frames)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                ((t * 440.0 * std::f32::consts::TAU).sin() * 0.4 * f32::from(i16::MAX)) as i16
            })
            .collect();
        PcmAudio {
            samples,
            sample_rate,
        }
    }

    #[test]
    fn format_from_extension_all_supported() {
        assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::from_extension("MP3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("ogg"), Some(AudioFormat::Ogg));
        assert_eq!(AudioFormat::from_extension("webm"), Some(AudioFormat::Webm));
        assert_eq!(AudioFormat::from_extension("xyz"), None);
    }

    #[test]
    fn wav_roundtrip_encode_decode() {
        let pcm = sine_pcm(1600, 16_000);
        let bytes = encode_wav(&pcm);
        let decoded = decode_bytes(&bytes, AudioFormat::Wav).unwrap();
        assert_eq!(decoded.sample_rate, 16_000);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        // Allow tiny rounding only if path went through f32; fast path is exact.
        assert_eq!(decoded.samples, pcm.samples);
    }

    #[test]
    fn wav_file_roundtrip_on_disk() {
        let dir = std::env::temp_dir().join("weft233_codec_wav");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tone.wav");
        let pcm = sine_pcm(800, 16_000);
        write_wav_file(&path, &pcm).unwrap();
        let (decoded, fmt) = decode_file(&path).unwrap();
        assert_eq!(fmt, AudioFormat::Wav);
        assert_eq!(decoded.samples, pcm.samples);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_mp3_errors_clearly() {
        let err = decode_bytes(b"not an mp3 file at all", AudioFormat::Mp3).unwrap_err();
        assert!(
            err.to_ascii_lowercase().contains("probe")
                || err.to_ascii_lowercase().contains("mp3")
                || err.to_ascii_lowercase().contains("decode")
                || err.to_ascii_lowercase().contains("format"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn empty_bytes_error() {
        assert!(decode_bytes(&[], AudioFormat::Wav).is_err());
    }

    fn fixtures_dir() -> Option<std::path::PathBuf> {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures");
        if p.is_dir() { Some(p) } else { None }
    }

    #[test]
    fn fixture_wav_decodes() {
        let Some(dir) = fixtures_dir() else {
            return;
        };
        let path = dir.join("tone.wav");
        if !path.exists() {
            return;
        }
        let (pcm, fmt) = decode_file(&path).expect("fixture wav");
        assert_eq!(fmt, AudioFormat::Wav);
        assert!(pcm.samples.len() > 100);
        assert_eq!(pcm.sample_rate, 16_000);
    }

    #[test]
    fn fixture_mp3_decodes() {
        let Some(dir) = fixtures_dir() else {
            return;
        };
        let path = dir.join("tone.mp3");
        if !path.exists() {
            return;
        }
        let (pcm, fmt) = decode_file(&path).expect("fixture mp3");
        assert_eq!(fmt, AudioFormat::Mp3);
        assert!(
            pcm.samples.len() > 100,
            "expected decoded MP3 samples, got {}",
            pcm.samples.len()
        );
        assert!(pcm.sample_rate >= 8_000);
    }
}
