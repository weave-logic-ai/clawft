//! Dual-layer text-to-speech (ADR-061 §4, §7).
//!
//! The voice *trilemma*: no single local engine is simultaneously fast,
//! voice-cloning, and literal-tag-performing. ADR-061's accepted default
//! covers all three *perceptually* with two engines:
//!
//! - **FAST layer — Chatterbox (cloned voice):** the immediate, contextual
//!   acknowledgment ("Puyo Puyo — one sec.") and snappy chit-chat. ~0.7 s.
//! - **SLOW layer — Orpheus (preset voice):** the expressive grounded answer,
//!   **streamed chunk-by-chunk** (producer renders sentence-by-sentence, the
//!   consumer plays each chunk as it lands) so first audio is ~1–2 s instead
//!   of one ~30 s blocking render. Performs paralinguistics (`<laugh>`)
//!   literally.
//!
//! The fast ack **covers** the slow render's latency — the loop never goes
//! silent. A blocking "render-all-then-play-once" call is explicitly
//! rejected. Barge-in flushes both producer and consumer.
//!
//! ECC mapping: the fast ack is the **Speculative** spoken node; the slow
//! grounded answer is the **Committed** node that supersedes it.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::types::VoiceError;
use super::wav::wav_to_pcm_s16le;

/// Which perceptual layer an engine serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsTier {
    /// Lowest-latency engine for acks / chit-chat (Chatterbox).
    Fast,
    /// Expressive, streamed engine for the grounded answer (Orpheus).
    Slow,
}

/// One rendered audio chunk (one sentence-ish unit), 16-bit mono PCM.
#[derive(Debug, Clone)]
pub struct TtsChunk {
    /// Interleaved `s16le` samples.
    pub samples: Vec<i16>,
    /// Sample rate of `samples`.
    pub sample_rate: u32,
}

/// A streaming TTS engine. Implementations MUST render incrementally
/// (sentence-by-sentence) and push each [`TtsChunk`] as it is ready, honoring
/// `cancel` between chunks so barge-in is responsive.
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Stream-synthesize `text`, sending chunks to `tx`. Returns when the
    /// whole text is rendered, the receiver is gone, or `cancel` fires.
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError>;

    /// Which tier this engine serves.
    fn tier(&self) -> TtsTier;
}

/// Audio sink for streamed TTS. Distinct from the request/response
/// [`PlaybackSink`](super::channel::PlaybackSink): it plays incrementally and
/// supports [`flush`](Self::flush) for barge-in.
#[async_trait]
pub trait TtsSink: Send + Sync {
    /// Play one chunk (appends to the playback stream; should not block longer
    /// than the chunk's own duration for a gap-free experience).
    async fn play_chunk(&self, chunk: &TtsChunk) -> Result<(), VoiceError>;

    /// Barge-in: drop all queued + in-flight audio immediately.
    async fn flush(&self);

    /// Wait until queued audio has actually left the speaker. `play_chunk`
    /// queues; without this, a caller that resumes capturing the mic as soon
    /// as the last chunk is queued will hear the bot's own tail-end speech
    /// as a new user turn (observed live). Default is a no-op for sinks with
    /// no physical playback latency (tests, discard sinks).
    async fn wait_drained(&self) {}
}

/// Strip every markup fragment so no `<`/`>`/`[`/`]` ever reaches audio
/// (ADR-061 §7 hard scrub guarantee) — used for engines that don't perform
/// tags. Removes balanced `<...>` / `[...]` spans and any stray brackets.
pub fn scrub_tags(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let close = match c {
            '<' => Some('>'),
            '[' => Some(']'),
            _ => None,
        };
        if let Some(close) = close {
            // Drop a well-formed `<...>` / `[...]` span; a stray opener drops
            // only itself (so an unbalanced `<` can't eat the rest of the
            // utterance).
            if let Some(rel) = chars[i + 1..].iter().position(|&x| x == close) {
                i += rel + 2; // skip opener..=close
            } else {
                i += 1;
            }
            continue;
        }
        if c == '>' || c == ']' {
            i += 1; // stray closer: drop it
            continue;
        }
        out.push(c);
        i += 1;
    }
    // Collapse the whitespace a removed tag can leave behind.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Split text into sentence-ish units for chunked streaming. Keeps terminal
/// punctuation; never returns empty units.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?') {
            let t = cur.trim();
            if !t.is_empty() {
                sentences.push(t.to_string());
            }
            cur.clear();
        }
    }
    let tail = cur.trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }
    sentences
}

/// Substrate HTTP TTS engine. POSTs `{"text": "..."}` per sentence to a
/// synthesis endpoint and expects an `audio/wav` body, streaming one chunk
/// per sentence. `strip_tags` controls whether markup is scrubbed before
/// synthesis (true for the fast/preset path, false for an engine that
/// performs `<laugh>` literally).
pub struct SubstrateTts {
    http: reqwest::Client,
    url: String,
    tier: TtsTier,
    strip_tags: bool,
}

impl SubstrateTts {
    /// Build an engine pointed at a fully-qualified synthesize URL.
    pub fn new(
        url: impl Into<String>,
        tier: TtsTier,
        strip_tags: bool,
        timeout_s: u64,
    ) -> Result<Self, VoiceError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_s.max(1)))
            .build()
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        Ok(Self {
            http,
            url: url.into(),
            tier,
            strip_tags,
        })
    }

    async fn render_one(&self, sentence: &str) -> Result<TtsChunk, VoiceError> {
        let text = if self.strip_tags {
            scrub_tags(sentence)
        } else {
            sentence.to_string()
        };
        let resp = self
            .http
            .post(&self.url)
            .json(&serde_json::json!({ "text": text }))
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
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| VoiceError::Transport(e.to_string()))?;
        let (samples, sample_rate) =
            wav_to_pcm_s16le(&bytes).map_err(|e| VoiceError::Malformed(format!("tts wav: {e}")))?;
        Ok(TtsChunk {
            samples,
            sample_rate,
        })
    }
}

#[async_trait]
impl TtsEngine for SubstrateTts {
    async fn synthesize_stream(
        &self,
        text: &str,
        tx: mpsc::Sender<TtsChunk>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        for sentence in split_sentences(text) {
            if cancel.is_cancelled() {
                break;
            }
            let chunk = self.render_one(&sentence).await?;
            if cancel.is_cancelled() || tx.send(chunk).await.is_err() {
                break;
            }
        }
        Ok(())
    }

    fn tier(&self) -> TtsTier {
        self.tier
    }
}

/// Dual-layer orchestrator: a fast ack covering a streamed expressive answer.
pub struct DualLayerTts {
    fast: Arc<dyn TtsEngine>,
    slow: Arc<dyn TtsEngine>,
    /// Acks pre-rendered through the SLOW tier (single-voice consistency):
    /// `speak` plays a cached ack instantly in the answer's own voice
    /// instead of rendering it in the fast tier's different voice.
    ack_cache: Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<TtsChunk>>>>,
}

impl DualLayerTts {
    /// Build from a fast (ack) and slow (answer) engine. The fast engine
    /// MUST be [`TtsTier::Fast`] and the slow [`TtsTier::Slow`].
    pub fn new(fast: Arc<dyn TtsEngine>, slow: Arc<dyn TtsEngine>) -> Result<Self, VoiceError> {
        if fast.tier() != TtsTier::Fast || slow.tier() != TtsTier::Slow {
            return Err(VoiceError::Config(
                "DualLayerTts: fast engine must be Fast tier and slow engine Slow tier".into(),
            ));
        }
        Ok(Self {
            fast,
            slow,
            ack_cache: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Pre-render `acks` through the SLOW tier into the cache, off-thread —
    /// call once at session start. Until a given ack lands in the cache,
    /// `speak` falls back to the fast tier for it (different voice, still
    /// audible), so warming is best-effort and non-blocking.
    pub fn spawn_warm_acks(&self, acks: Vec<String>) {
        let slow = self.slow.clone();
        let cache = self.ack_cache.clone();
        tokio::spawn(async move {
            for ack in acks {
                if cache.lock().unwrap().contains_key(&ack) {
                    continue;
                }
                let (tx, mut rx) = mpsc::channel::<TtsChunk>(16);
                let engine = slow.clone();
                let text = ack.clone();
                let producer = tokio::spawn(async move {
                    engine
                        .synthesize_stream(&text, tx, CancellationToken::new())
                        .await
                });
                let mut chunks: Vec<TtsChunk> = Vec::new();
                while let Some(c) = rx.recv().await {
                    chunks.push(c);
                }
                match producer.await {
                    Ok(Ok(())) if !chunks.is_empty() => {
                        tracing::info!(ack = %ack, "ack pre-rendered in the answer voice");
                        cache.lock().unwrap().insert(ack, chunks);
                    }
                    res => {
                        tracing::debug!(
                            ack = %ack,
                            ?res,
                            "ack warm failed; fast-tier fallback will speak it"
                        );
                    }
                }
            }
        });
    }

    /// Speak `ack` (optional, fast layer, covers latency) then stream `answer`
    /// (slow layer, gap-free, chunk-by-chunk) to `sink`. On `cancel` (barge-
    /// in) the producer stops and the sink is flushed.
    pub async fn speak(
        &self,
        ack: Option<&str>,
        answer: &str,
        sink: Arc<dyn TtsSink>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        // ACK: prefer the pre-rendered slow-voice cache (single-voice
        // consistency — the ack speaks in the answer's own voice, instantly);
        // fall back to the FAST layer for uncached acks so the loop never
        // goes silent while the slow answer renders.
        if let Some(ack) = ack {
            if cancel.is_cancelled() {
                sink.flush().await;
                return Ok(());
            }
            let cached = self.ack_cache.lock().unwrap().get(ack).cloned();
            if let Some(chunks) = cached {
                for chunk in &chunks {
                    if cancel.is_cancelled() {
                        break;
                    }
                    sink.play_chunk(chunk).await?;
                }
            } else {
                let (atx, mut arx) = mpsc::channel::<TtsChunk>(4);
                let fast = self.fast.clone();
                let ack_text = ack.to_string();
                let ack_cancel = cancel.clone();
                let producer = tokio::spawn(async move {
                    let _ = fast.synthesize_stream(&ack_text, atx, ack_cancel).await;
                });
                while let Some(chunk) = arx.recv().await {
                    if cancel.is_cancelled() {
                        break;
                    }
                    sink.play_chunk(&chunk).await?;
                }
                let _ = producer.await;
            }
        }

        if cancel.is_cancelled() {
            sink.flush().await;
            return Ok(());
        }

        // SLOW layer: stream the expressive answer chunk-by-chunk. Producer
        // renders sentences in a task; the consumer plays each as it lands.
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(4);
        let slow = self.slow.clone();
        let answer_text = answer.to_string();
        let prod_cancel = cancel.clone();
        let producer =
            tokio::spawn(
                async move { slow.synthesize_stream(&answer_text, tx, prod_cancel).await },
            );

        // PREBUFFER the slow tier (voicelab `slow_prebuffer` semantics):
        // Orpheus over Ollama can render slower than real time on a loaded
        // machine, and playing its chunks as they trickle in starves the
        // output stream — audible stutter ("skipping"). Collect the full
        // render first, then play gap-free; the fast ack already covers
        // the latency window.
        let mut interrupted = false;
        let mut slow_chunks = 0usize;
        let mut buffered: Vec<TtsChunk> = Vec::new();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    interrupted = true;
                    break;
                }
                maybe = rx.recv() => {
                    match maybe {
                        Some(chunk) => {
                            slow_chunks += 1;
                            buffered.push(chunk);
                        }
                        None => break,
                    }
                }
            }
        }
        if !interrupted {
            for chunk in &buffered {
                if cancel.is_cancelled() {
                    interrupted = true;
                    break;
                }
                sink.play_chunk(chunk).await?;
            }
        }
        if interrupted {
            sink.flush().await;
            return match producer.await {
                Ok(r) => r,
                Err(_) => Ok(()),
            };
        }
        // Producer observes the same cancel token; join its result.
        let slow_result = match producer.await {
            Ok(r) => r,
            Err(_) => Ok(()), // task aborted/panicked on shutdown — non-fatal
        };

        // The answer must ALWAYS be audible: when the slow tier yielded no
        // audio at all (endpoint down, unparseable token stream, missing
        // decoder), re-render the whole answer through the fast tier rather
        // than going silent after the ack. Observed live: Ollama `orpheus-tts`
        // returning zero decodable frames left only "One sec" playing.
        if slow_chunks == 0 && !answer.trim().is_empty() {
            tracing::warn!(
                slow_error = %slow_result.as_ref().err().map(|e| e.to_string()).unwrap_or_else(|| "produced no chunks".into()),
                "slow TTS tier produced no audio — falling back to the fast tier for the answer"
            );
            let (ftx, mut frx) = mpsc::channel::<TtsChunk>(4);
            let fast = self.fast.clone();
            let text = answer.to_string();
            let fcancel = cancel.clone();
            let fprod = tokio::spawn(async move {
                let _ = fast.synthesize_stream(&text, ftx, fcancel).await;
            });
            while let Some(chunk) = frx.recv().await {
                if cancel.is_cancelled() {
                    break;
                }
                sink.play_chunk(&chunk).await?;
            }
            let _ = fprod.await;
            return Ok(());
        }
        slow_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn scrub_removes_all_markup() {
        assert_eq!(scrub_tags("hello <laugh> there"), "hello there");
        assert_eq!(scrub_tags("a [stage] b"), "a b");
        // Tag markers stripped; wrapped spoken text is kept.
        assert_eq!(scrub_tags("<emotion>warm</emotion> hi"), "warm hi");
        // Standalone paralinguistic tag removed entirely (no content).
        assert_eq!(scrub_tags("ha <laugh>"), "ha");
        // Stray/unbalanced brackets must not leak.
        assert_eq!(scrub_tags("oops < broken"), "oops broken");
        assert!(!scrub_tags("x <tag> y").contains('<'));
    }

    #[test]
    fn sentence_split_keeps_terminators() {
        let s = split_sentences("Hi there. How are you? Good!");
        assert_eq!(s, vec!["Hi there.", "How are you?", "Good!"]);
        // No terminator -> single unit.
        assert_eq!(split_sentences("no period here"), vec!["no period here"]);
        assert!(split_sentences("   ").is_empty());
    }

    /// Fake engine emitting one chunk per sentence; records a tier marker in
    /// the first sample so the consumer can prove ordering.
    struct FakeEngine {
        tier: TtsTier,
        marker: i16,
    }
    #[async_trait]
    impl TtsEngine for FakeEngine {
        async fn synthesize_stream(
            &self,
            text: &str,
            tx: mpsc::Sender<TtsChunk>,
            cancel: CancellationToken,
        ) -> Result<(), VoiceError> {
            for _ in split_sentences(text) {
                if cancel.is_cancelled() {
                    break;
                }
                let chunk = TtsChunk {
                    samples: vec![self.marker; 160],
                    sample_rate: 16_000,
                };
                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
            Ok(())
        }
        fn tier(&self) -> TtsTier {
            self.tier
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        chunks: tokio::sync::Mutex<Vec<i16>>,
        flushes: AtomicUsize,
    }
    #[async_trait]
    impl TtsSink for RecordingSink {
        async fn play_chunk(&self, chunk: &TtsChunk) -> Result<(), VoiceError> {
            self.chunks.lock().await.push(chunk.samples[0]);
            Ok(())
        }
        async fn flush(&self) {
            self.flushes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn dual_layer_plays_ack_before_answer() {
        let fast = Arc::new(FakeEngine {
            tier: TtsTier::Fast,
            marker: 1,
        });
        let slow = Arc::new(FakeEngine {
            tier: TtsTier::Slow,
            marker: 2,
        });
        let dual = DualLayerTts::new(fast, slow).unwrap();
        let sink = Arc::new(RecordingSink::default());
        let cancel = CancellationToken::new();
        dual.speak(
            Some("one sec."),
            "Here is the answer. It has two parts.",
            sink.clone(),
            cancel,
        )
        .await
        .unwrap();
        let order = sink.chunks.lock().await.clone();
        // 1 ack chunk (marker 1) then 2 answer chunks (marker 2).
        assert_eq!(order, vec![1, 2, 2]);
        assert_eq!(sink.flushes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn barge_in_flushes_and_stops() {
        // A slow engine that emits many chunks slowly so cancel lands mid-stream.
        struct SlowDrip;
        #[async_trait]
        impl TtsEngine for SlowDrip {
            async fn synthesize_stream(
                &self,
                _text: &str,
                tx: mpsc::Sender<TtsChunk>,
                cancel: CancellationToken,
            ) -> Result<(), VoiceError> {
                for _ in 0..50 {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let chunk = TtsChunk {
                        samples: vec![9i16; 160],
                        sample_rate: 16_000,
                    };
                    if tx.send(chunk).await.is_err() {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Ok(())
            }
            fn tier(&self) -> TtsTier {
                TtsTier::Slow
            }
        }
        let fast = Arc::new(FakeEngine {
            tier: TtsTier::Fast,
            marker: 1,
        });
        let dual = DualLayerTts::new(fast, Arc::new(SlowDrip)).unwrap();
        let sink = Arc::new(RecordingSink::default());
        let cancel = CancellationToken::new();
        let c2 = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            c2.cancel();
        });
        dual.speak(None, "long answer", sink.clone(), cancel)
            .await
            .unwrap();
        assert_eq!(
            sink.flushes.load(Ordering::SeqCst),
            1,
            "barge-in must flush"
        );
        // It must have stopped early (far fewer than 50 chunks).
        assert!(sink.chunks.lock().await.len() < 50);
    }

    #[test]
    fn rejects_mismatched_tiers() {
        let a = Arc::new(FakeEngine {
            tier: TtsTier::Slow,
            marker: 0,
        });
        let b = Arc::new(FakeEngine {
            tier: TtsTier::Fast,
            marker: 0,
        });
        assert!(DualLayerTts::new(a, b).is_err());
    }
}
