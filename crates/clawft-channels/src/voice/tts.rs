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

use clawft_types::config::ResolvedTtsVoice;

use super::types::VoiceError;
use super::wav::wav_to_pcm_s16le;

/// Optional per-request voice overrides for substrate/cloud TTS (WEFT-222).
///
/// When set, POST bodies include `voice_id` / `speed` / `pitch` so a multi-agent
/// deployment can route each agent to a distinct voice.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TtsVoiceParams {
    pub voice_id: Option<String>,
    pub speed: Option<f32>,
    pub pitch: Option<f32>,
    pub provider: Option<String>,
    pub language: Option<String>,
}

impl TtsVoiceParams {
    /// Build from a resolved personality (always fills voice_id/speed/pitch).
    pub fn from_resolved(r: &ResolvedTtsVoice) -> Self {
        Self {
            voice_id: Some(r.voice_id.clone()).filter(|s| !s.is_empty()),
            speed: Some(r.speed),
            pitch: Some(r.pitch),
            provider: Some(r.provider.clone()).filter(|s| !s.is_empty()),
            language: Some(r.language.clone()).filter(|s| !s.is_empty()),
        }
    }

    /// Merge personality fields into a `{"text": …}` substrate body.
    pub fn apply_to_json(&self, text: &str) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("text".into(), serde_json::Value::String(text.to_string()));
        if let Some(ref v) = self.voice_id {
            map.insert("voice_id".into(), serde_json::Value::String(v.clone()));
        }
        if let Some(s) = self.speed {
            map.insert("speed".into(), serde_json::json!(s));
        }
        if let Some(p) = self.pitch {
            map.insert("pitch".into(), serde_json::json!(p));
        }
        if let Some(ref p) = self.provider {
            map.insert("provider".into(), serde_json::Value::String(p.clone()));
        }
        if let Some(ref l) = self.language {
            map.insert("language".into(), serde_json::Value::String(l.clone()));
        }
        serde_json::Value::Object(map)
    }

    /// True when any override field is set.
    pub fn is_empty(&self) -> bool {
        self.voice_id.is_none()
            && self.speed.is_none()
            && self.pitch.is_none()
            && self.provider.is_none()
            && self.language.is_none()
    }
}

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
    /// Per-agent voice overrides (WEFT-222). Empty = text-only body.
    voice_params: TtsVoiceParams,
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
            voice_params: TtsVoiceParams::default(),
        })
    }

    /// Attach per-agent voice parameters (from [`ResolvedTtsVoice`]).
    pub fn with_voice_params(mut self, params: TtsVoiceParams) -> Self {
        self.voice_params = params;
        self
    }

    /// Replace voice parameters after construction (agent switch).
    pub fn set_voice_params(&mut self, params: TtsVoiceParams) {
        self.voice_params = params;
    }

    /// Current voice parameters.
    pub fn voice_params(&self) -> &TtsVoiceParams {
        &self.voice_params
    }

    /// Build the JSON body for one synthesis request (testable without HTTP).
    pub fn request_body(&self, sentence: &str) -> serde_json::Value {
        let text = if self.strip_tags {
            scrub_tags(sentence)
        } else {
            sentence.to_string()
        };
        if self.voice_params.is_empty() {
            serde_json::json!({ "text": text })
        } else {
            self.voice_params.apply_to_json(&text)
        }
    }

    async fn render_one(&self, sentence: &str) -> Result<TtsChunk, VoiceError> {
        let body = self.request_body(sentence);
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
    /// in) the producer stops and the sink is flushed. Thin composition of
    /// [`speak_ack`](Self::speak_ack) + [`speak_answer`](Self::speak_answer) —
    /// kept as a single call for callers that don't need to interleave
    /// anything (e.g. a filler) between the two.
    pub async fn speak(
        &self,
        ack: Option<&str>,
        answer: &str,
        sink: Arc<dyn TtsSink>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        if let Some(ack) = ack {
            self.speak_ack(ack, sink.clone(), cancel.clone()).await?;
        }
        self.speak_answer(answer, sink, cancel).await
    }

    /// Speak `ack` alone (fast layer, covers latency): prefer the
    /// pre-rendered slow-voice cache (single-voice consistency — the ack
    /// speaks in the answer's own voice, instantly); fall back to the FAST
    /// layer for uncached acks so the loop never goes silent while the slow
    /// answer renders. Split out from [`speak`](Self::speak) so a caller can
    /// speak the ack immediately (before the grounded answer even starts
    /// generating) instead of only once both are ready.
    pub async fn speak_ack(
        &self,
        ack: &str,
        sink: Arc<dyn TtsSink>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
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
        Ok(())
    }

    /// Speak dynamic filler text (e.g. "Checking on {subject} now.") through
    /// the FAST tier only. Unlike [`speak_ack`](Self::speak_ack), fillers are
    /// never cached — the subject is spliced in per-turn, so there is no
    /// fixed closed set to pre-render.
    pub async fn speak_filler(
        &self,
        filler: &str,
        sink: Arc<dyn TtsSink>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(4);
        let fast = self.fast.clone();
        let text = filler.to_string();
        let fcancel = cancel.clone();
        let producer = tokio::spawn(async move {
            let _ = fast.synthesize_stream(&text, tx, fcancel).await;
        });
        while let Some(chunk) = rx.recv().await {
            if cancel.is_cancelled() {
                break;
            }
            sink.play_chunk(&chunk).await?;
        }
        let _ = producer.await;
        Ok(())
    }

    /// Stream the expressive `answer` alone (slow layer, gap-free,
    /// chunk-by-chunk) to `sink`. Split out from [`speak`](Self::speak) so a
    /// caller can speak the ack (and an optional filler) up front and only
    /// start this once the grounded answer text is ready.
    /// Fully render `answer` to chunks WITHOUT playing — the producer +
    /// prebuffer half of [`Self::speak_answer`] (same slow-tier render, same
    /// zero-audio fast-tier fallback). WEFT-615's windowed playback needs the
    /// render and the playback separated: rendering once up front keeps the
    /// old gap-free pipeline economics, while playback interleaves the
    /// inter-sentence listening windows (per-sentence re-rendering serialized
    /// synthesis into every gap — seconds of dead air, found live).
    pub async fn render_answer(
        &self,
        answer: &str,
        cancel: CancellationToken,
    ) -> Result<Vec<TtsChunk>, VoiceError> {
        if cancel.is_cancelled() || answer.trim().is_empty() {
            return Ok(Vec::new());
        }
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(4);
        let slow = self.slow.clone();
        let answer_text = answer.to_string();
        let prod_cancel = cancel.clone();
        let producer =
            tokio::spawn(
                async move { slow.synthesize_stream(&answer_text, tx, prod_cancel).await },
            );
        let mut buffered: Vec<TtsChunk> = Vec::new();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe = rx.recv() => match maybe {
                    Some(chunk) => buffered.push(chunk),
                    None => break,
                },
            }
        }
        let slow_result = match producer.await {
            Ok(r) => r,
            Err(_) => Ok(()),
        };
        if buffered.is_empty() && !cancel.is_cancelled() {
            tracing::warn!(
                slow_error = %slow_result.as_ref().err().map(|e| e.to_string()).unwrap_or_else(|| "produced no chunks".into()),
                "slow TTS tier produced no audio — rendering the answer through the fast tier"
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
                buffered.push(chunk);
            }
            let _ = fprod.await;
        }
        Ok(buffered)
    }

    /// Play one pre-rendered chunk (windowed playback's per-sentence step).
    pub async fn play_chunk(
        &self,
        chunk: &TtsChunk,
        sink: &Arc<dyn TtsSink>,
    ) -> Result<(), VoiceError> {
        sink.play_chunk(chunk).await
    }

    pub async fn speak_answer(
        &self,
        answer: &str,
        sink: Arc<dyn TtsSink>,
        cancel: CancellationToken,
    ) -> Result<(), VoiceError> {
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

    /// WEFT-222: SubstrateTts request body carries per-agent voice params.
    #[test]
    fn substrate_tts_request_body_two_personalities() {
        use clawft_types::config::{VoiceConfig, VoicePersonality};
        use super::super::personality::PersonalityTtsDispatch;

        let mut cfg = VoiceConfig::default();
        cfg.personalities.insert(
            "alpha".into(),
            VoicePersonality {
                voice_id: "nova".into(),
                provider: "openai".into(),
                speed: 1.1,
                pitch: 0.2,
                greeting_prefix: Some("Alpha greets.".into()),
                language: "en".into(),
            },
        );
        cfg.personalities.insert(
            "beta".into(),
            VoicePersonality {
                voice_id: "onyx".into(),
                provider: "openai".into(),
                speed: 0.95,
                pitch: -0.15,
                greeting_prefix: Some("Beta greets.".into()),
                language: "en".into(),
            },
        );

        let mut dispatch = PersonalityTtsDispatch::new(cfg, "alpha");
        let alpha_res = dispatch.resolve();
        let alpha_engine = SubstrateTts::new("http://localhost/synthesize", TtsTier::Fast, true, 5)
            .unwrap()
            .with_voice_params(TtsVoiceParams::from_resolved(&alpha_res));
        let alpha_body = alpha_engine.request_body("hello");
        assert_eq!(alpha_body["text"], "hello");
        assert_eq!(alpha_body["voice_id"], "nova");
        assert!((alpha_body["speed"].as_f64().unwrap() - 1.1).abs() < 1e-5);
        assert!((alpha_body["pitch"].as_f64().unwrap() - 0.2).abs() < 1e-5);

        dispatch.set_agent("beta");
        let beta_res = dispatch.resolve();
        let beta_engine = SubstrateTts::new("http://localhost/synthesize", TtsTier::Slow, false, 5)
            .unwrap()
            .with_voice_params(TtsVoiceParams::from_resolved(&beta_res));
        let beta_body = beta_engine.request_body("hello");
        assert_eq!(beta_body["voice_id"], "onyx");
        assert!((beta_body["speed"].as_f64().unwrap() - 0.95).abs() < 1e-5);
        assert!((beta_body["pitch"].as_f64().unwrap() - (-0.15)).abs() < 1e-5);
        assert_ne!(alpha_body["voice_id"], beta_body["voice_id"]);

        // Greeting consumption at session start via dispatch.
        let mut session = PersonalityTtsDispatch::new(
            {
                let mut c = VoiceConfig::default();
                c.personalities.insert(
                    "alpha".into(),
                    VoicePersonality {
                        voice_id: "nova".into(),
                        greeting_prefix: Some("Alpha greets.".into()),
                        ..Default::default()
                    },
                );
                c
            },
            "alpha",
        );
        let (r, spoken) = session.prepare_utterance("Standing by.", true);
        assert_eq!(r.voice_id, "nova");
        assert_eq!(spoken, "Alpha greets. Standing by.");
        let (_, spoken2) = session.prepare_utterance("Standing by.", true);
        assert_eq!(spoken2, "Standing by.");
    }

    #[test]
    fn tts_voice_params_apply_to_json() {
        let p = TtsVoiceParams {
            voice_id: Some("alloy".into()),
            speed: Some(1.0),
            pitch: Some(0.0),
            provider: Some("local".into()),
            language: Some("en".into()),
        };
        let j = p.apply_to_json("hi");
        assert_eq!(j["text"], "hi");
        assert_eq!(j["voice_id"], "alloy");
    }
}
