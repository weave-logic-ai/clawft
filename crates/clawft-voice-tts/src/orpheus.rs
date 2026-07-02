//! Orpheus slow-layer TTS (ADR-062 Phase 4.2) — token-gen over **Ollama** HTTP,
//! decoded through SNAC to streamed PCM. **No Python.**
//!
//! Orpheus is a token LM: prompted with `<|audio|>{voice}: {text}` it emits a
//! stream of `<custom_token_N>` SNAC codes. This engine drives that generation
//! over Ollama's streaming generate API
//! (`POST http://127.0.0.1:11434/api/generate`, `model:"orpheus-tts"`,
//! `raw:true`, `stream:true` — Ollama is a *native* runtime, not a Python
//! wrapper), parses the newline-delimited JSON token stream incrementally,
//! decodes whole SNAC frames through a [`SnacDecode`], and pushes each batch as
//! a [`TtsChunk`] so first audio lands early and playback is gap-free. Barge-in
//! ([`CancellationToken`]) stops generation between chunks.
//!
//! As the **expressive** layer Orpheus performs paralinguistic tags (`<laugh>`)
//! literally, so the answer text is sent **verbatim** — never scrubbed (the fast
//! ack layer is where preset/scrubbed rendering lives).
//!
//! The engine is generic over the [`SnacDecode`] seam, so the token→chunk path
//! is driven by a deterministic mock (a streaming Ollama mock + a fake decoder)
//! without weights or a live model; the real path is `#[ignore]`-gated.

use std::sync::Arc;

use async_trait::async_trait;
use clawft_channels::voice::tts::{TtsChunk, TtsEngine, TtsTier};
use clawft_channels::voice::types::VoiceError;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::snac::{FRAME_TOKENS, SNAC_CODEBOOK_SIZE, SnacDecode};

/// Default Ollama streaming-generate endpoint.
pub const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434/api/generate";
/// Default Ollama model tag serving Orpheus.
pub const DEFAULT_MODEL: &str = "orpheus-tts";
/// Default Orpheus preset voice.
pub const DEFAULT_VOICE: &str = "dan";
/// Default frames per emitted chunk (≈ the reference 28-token streaming
/// cadence: 4 frames). Smaller ⇒ lower TTFA but more decode calls.
pub const DEFAULT_BATCH_FRAMES: usize = 4;

/// Orpheus custom-token marker prefix.
const TOKEN_PREFIX: &str = "<custom_token_";
/// Orpheus vocab offset removed from every custom-token id.
const TOKEN_OFFSET: i64 = 10;

/// Parse one `<custom_token_N>` into its SNAC code, applying Orpheus' per-slot
/// offset: `N - 10 - ((index % 7) * 4096)`, where `index` is the running
/// custom-token count (so the 7 slots of a frame map back into the codebook
/// range). Returns `None` for anything that is not a well-formed custom token.
pub fn parse_custom_token(token: &str, index: usize) -> Option<i64> {
    let inner = token.trim().strip_prefix(TOKEN_PREFIX)?.strip_suffix('>')?;
    let n: i64 = inner.parse().ok()?;
    Some(n - TOKEN_OFFSET - ((index % FRAME_TOKENS) as i64) * SNAC_CODEBOOK_SIZE)
}

/// Incremental parser turning a streamed token text into SNAC codes. Tolerates
/// custom tokens split across stream fragments and discards interleaved
/// non-token text (Orpheus emits only tokens under the `<|audio|>` prompt, but
/// this stays robust).
#[derive(Default)]
struct TokenAccum {
    /// Text not yet fully consumed (may hold a partial trailing token).
    pending: String,
    /// Cumulative custom-token count, feeding the `% 7` slot offset.
    index: usize,
    /// Offset-removed SNAC codes awaiting whole-frame decode.
    codes: Vec<i64>,
}

impl TokenAccum {
    fn push_text(&mut self, frag: &str) {
        self.pending.push_str(frag);
        loop {
            let Some(start) = self.pending.find(TOKEN_PREFIX) else {
                // No complete marker; retain only a trailing `<…` that could be
                // the start of a token split across the next fragment.
                keep_trailing_lt(&mut self.pending);
                break;
            };
            let rest = &self.pending[start..];
            let Some(gt) = rest.find('>') else {
                // Incomplete token — keep from the marker for the next fragment.
                self.pending = rest.to_string();
                break;
            };
            let token = rest[..=gt].to_string();
            let after = rest[gt + 1..].to_string();
            if let Some(code) = parse_custom_token(&token, self.index) {
                self.codes.push(code);
                self.index += 1;
            }
            self.pending = after;
        }
    }

    /// Drain one whole `batch_frames`-frame batch, if available.
    fn take_batch(&mut self, batch_frames: usize) -> Option<Vec<i64>> {
        let avail = self.codes.len() / FRAME_TOKENS;
        if batch_frames > 0 && avail >= batch_frames {
            Some(self.codes.drain(..batch_frames * FRAME_TOKENS).collect())
        } else {
            None
        }
    }

    /// Drain all remaining whole frames (end-of-stream flush); a sub-frame tail
    /// is dropped.
    fn take_remaining(&mut self) -> Option<Vec<i64>> {
        let frames = self.codes.len() / FRAME_TOKENS;
        (frames > 0).then(|| self.codes.drain(..frames * FRAME_TOKENS).collect())
    }
}

/// Retain only a trailing `<…` (a possible split token start); else clear.
fn keep_trailing_lt(s: &mut String) {
    match s.rfind('<') {
        Some(pos) if pos > 0 => *s = s[pos..].to_string(),
        Some(_) => {} // already starts at '<'
        None => s.clear(),
    }
}

/// Parse one NDJSON line from Ollama's stream into `(response_fragment, done)`.
fn parse_ndjson_line(line: &[u8]) -> Option<(String, bool)> {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let frag = v
        .get("response")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
    let done = v.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
    Some((frag, done))
}

/// Orpheus slow-layer engine. Build with [`OrpheusTts::new`] (default Ollama
/// endpoint) or [`OrpheusTts::with_endpoint`]. Generic only over the
/// [`SnacDecode`] seam; the HTTP/token plumbing is concrete.
pub struct OrpheusTts {
    http: reqwest::Client,
    url: String,
    model: String,
    voice: String,
    decoder: Arc<dyn SnacDecode>,
    batch_frames: usize,
}

impl OrpheusTts {
    /// Build against the default local Ollama endpoint + `orpheus-tts` model.
    pub fn new(decoder: Arc<dyn SnacDecode>) -> Result<Self, VoiceError> {
        Self::with_endpoint(DEFAULT_OLLAMA_URL, DEFAULT_MODEL, DEFAULT_VOICE, decoder)
    }

    /// Build against an explicit Ollama generate URL, model tag, and voice.
    pub fn with_endpoint(
        url: impl Into<String>,
        model: impl Into<String>,
        voice: impl Into<String>,
        decoder: Arc<dyn SnacDecode>,
    ) -> Result<Self, VoiceError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        Ok(Self {
            http,
            url: url.into(),
            model: model.into(),
            voice: voice.into(),
            decoder,
            batch_frames: DEFAULT_BATCH_FRAMES,
        })
    }

    /// Override how many SNAC frames are decoded per emitted chunk (≥ 1).
    pub fn with_batch_frames(mut self, frames: usize) -> Self {
        self.batch_frames = frames.max(1);
        self
    }

    /// The Orpheus prompt for `text`: `<|audio|>{voice}: {text}` (tags kept
    /// verbatim — Orpheus performs them).
    fn prompt(&self, text: &str) -> String {
        // lex-au Orpheus raw prompt format. The trailing `<|eot_id|>` is
        // load-bearing: without it the model never enters audio-token
        // generation and streams zero `<custom_token_N>` pieces (verified
        // against the working voicelab `tts_orpheus_ollama` impl).
        format!("<|audio|>{}: {}<|eot_id|>", self.voice, text)
    }
}

#[async_trait]
impl TtsEngine for OrpheusTts {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let body = serde_json::json!({
            "model": self.model,
            "prompt": self.prompt(text),
            "raw": true,
            "stream": true,
            // Match the proven voicelab request: keep the model resident
            // between turns and use the calm-delivery sampling the "dan"
            // voice was tuned with (lower temp = less word-doubling).
            "keep_alive": "10m",
            "options": {
                "temperature": 0.4,
                "top_p": 0.9,
                "repeat_penalty": 1.1,
                "num_predict": 1200,
            },
        });
        let resp = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<unreadable>".into());
            return Err(VoiceError::Server {
                status: status.as_u16(),
                body,
            });
        }

        let sr = self.decoder.sample_rate();
        let mut stream = resp.bytes_stream();
        let mut byte_buf: Vec<u8> = Vec::new();
        let mut accum = TokenAccum::default();
        let mut done = false;

        while let Some(item) = stream.next().await {
            if cancel.is_cancelled() {
                return Ok(());
            }
            let bytes = item.map_err(|e| VoiceError::Transport(e.to_string()))?;
            byte_buf.extend_from_slice(&bytes);

            // Drain every complete NDJSON line.
            while let Some(nl) = byte_buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = byte_buf.drain(..=nl).collect();
                if let Some((frag, line_done)) = parse_ndjson_line(&line[..line.len() - 1]) {
                    accum.push_text(&frag);
                    done |= line_done;
                }
            }

            // Emit every whole batch the new tokens completed.
            while let Some(batch) = accum.take_batch(self.batch_frames) {
                if cancel.is_cancelled() {
                    return Ok(());
                }
                let pcm = self.decoder.decode_frames(&batch)?;
                if pcm.is_empty() {
                    continue;
                }
                let chunk = TtsChunk {
                    samples: pcm,
                    sample_rate: sr,
                };
                if cancel.is_cancelled() || tx.send(chunk).await.is_err() {
                    return Ok(());
                }
            }
            if done {
                break;
            }
        }

        // A final line may arrive without a trailing newline.
        if !byte_buf.is_empty()
            && !cancel.is_cancelled()
            && let Some((frag, _)) = parse_ndjson_line(&byte_buf)
        {
            accum.push_text(&frag);
        }

        // Flush the trailing whole frames.
        if !cancel.is_cancelled()
            && let Some(batch) = accum.take_remaining()
        {
            let pcm = self.decoder.decode_frames(&batch)?;
            if !pcm.is_empty() {
                let _ = tx
                    .send(TtsChunk {
                        samples: pcm,
                        sample_rate: sr,
                    })
                    .await;
            }
        }
        Ok(())
    }

    fn tier(&self) -> TtsTier {
        TtsTier::Slow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake SNAC decoder: deterministic, model-free. PCM length == code count,
    /// so the token→chunk path is observable without weights.
    struct FakeSnac;
    impl SnacDecode for FakeSnac {
        fn decode_frames(&self, flat_codes: &[i64]) -> Result<Vec<i16>, VoiceError> {
            Ok(vec![7i16; flat_codes.len()])
        }
        fn sample_rate(&self) -> u32 {
            24_000
        }
    }

    #[test]
    fn parse_token_applies_slot_offset() {
        // index 0 → slot 0: N - 10.
        assert_eq!(parse_custom_token("<custom_token_10>", 0), Some(0));
        assert_eq!(parse_custom_token("<custom_token_4106>", 0), Some(4096));
        // index 1 → slot 1: N - 10 - 4096.
        assert_eq!(parse_custom_token("<custom_token_4106>", 1), Some(0));
        // index 7 wraps back to slot 0.
        assert_eq!(parse_custom_token("<custom_token_10>", 7), Some(0));
        // Not a custom token.
        assert_eq!(parse_custom_token("hello", 0), None);
        assert_eq!(parse_custom_token("<custom_token_x>", 0), None);
    }

    #[test]
    fn accum_extracts_tokens_and_batches_frames() {
        let mut a = TokenAccum::default();
        // Two full frames (14 tokens) of well-formed tokens.
        let mut s = String::new();
        for i in 0..14 {
            s.push_str(&format!("<custom_token_{}>", 100 + i));
        }
        a.push_text(&s);
        assert_eq!(a.index, 14);
        let b0 = a.take_batch(1).expect("one frame");
        assert_eq!(b0.len(), FRAME_TOKENS);
        let b1 = a.take_batch(1).expect("second frame");
        assert_eq!(b1.len(), FRAME_TOKENS);
        assert!(a.take_batch(1).is_none(), "no third frame");
    }

    #[test]
    fn accum_handles_token_split_across_fragments() {
        let mut a = TokenAccum::default();
        // A custom token arriving in two pieces must not be lost.
        a.push_text("<custom_to");
        assert_eq!(a.index, 0, "partial token not yet counted");
        a.push_text("ken_137><custom_token_200>");
        assert_eq!(a.index, 2, "both tokens recovered");
        let frames = a.take_remaining();
        assert!(frames.is_none(), "two tokens is under one 7-code frame");
    }

    #[test]
    fn ndjson_line_extracts_response_and_done() {
        let (frag, done) =
            parse_ndjson_line(br#"{"response":"<custom_token_10>","done":false}"#).unwrap();
        assert_eq!(frag, "<custom_token_10>");
        assert!(!done);
        let (_, done) = parse_ndjson_line(br#"{"response":"","done":true}"#).unwrap();
        assert!(done);
        assert!(parse_ndjson_line(b"   ").is_none());
    }

    #[tokio::test]
    async fn streaming_ollama_drives_token_to_chunk_path() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Build an NDJSON token stream: 3 full frames (21 tokens) + done.
        let mut ndjson = String::new();
        for i in 0..21 {
            ndjson.push_str(&format!(
                "{{\"response\":\"<custom_token_{}>\",\"done\":false}}\n",
                100 + i
            ));
        }
        ndjson.push_str("{\"response\":\"\",\"done\":true}\n");

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/x-ndjson")
                    .set_body_string(ndjson),
            )
            .mount(&server)
            .await;

        let url = format!("{}/api/generate", server.uri());
        let engine = OrpheusTts::with_endpoint(url, "orpheus-tts", "dan", Arc::new(FakeSnac))
            .unwrap()
            .with_batch_frames(1);
        assert_eq!(engine.tier(), TtsTier::Slow);

        let (tx, mut rx) = mpsc::channel::<TtsChunk>(16);
        let cancel = CancellationToken::new();
        engine
            .synthesize_stream("hello there", tx, cancel)
            .await
            .unwrap();

        let mut chunks = 0;
        let mut samples = 0;
        while let Ok(chunk) = rx.try_recv() {
            chunks += 1;
            samples += chunk.samples.len();
            assert_eq!(chunk.sample_rate, 24_000);
        }
        // 3 frames at batch=1 → 3 chunks, 7 codes each via the fake decoder.
        assert_eq!(chunks, 3, "one chunk per frame");
        assert_eq!(samples, 21);
    }

    #[tokio::test]
    #[ignore = "requires a live Ollama serving `orpheus-tts` on :11434 + a SNAC model"]
    async fn live_orpheus_ollama() {
        let decoder = Arc::new(crate::snac::SnacOnnxDecoder::new());
        let engine = OrpheusTts::new(decoder).unwrap();
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(64);
        let cancel = CancellationToken::new();
        let h = tokio::spawn(async move {
            engine
                .synthesize_stream("Hello — this is a live test.", tx, cancel)
                .await
        });
        let mut total = 0;
        while let Some(chunk) = rx.recv().await {
            total += chunk.samples.len();
        }
        h.await.unwrap().unwrap();
        assert!(total > 0, "live Orpheus produced no audio");
    }
}
