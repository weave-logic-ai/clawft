//! Discord voice-channel bridge (WEFT-232).
//!
//! Wires Discord VC audio into the canonical clawft-channels voice path:
//!
//! ```text
//! Discord VC PCM in → STT → VoiceLlm (agent) → TTS → Discord VC PCM out
//! ```
//!
//! # Scaffold vs real transport
//!
//! Discord Voice Gateway + Opus/UDP is historically served by **serenity +
//! songbird**. Those crates pull native crypto / audio deps and need a live
//! bot token — unsuitable for default CI. This module therefore:
//!
//! 1. Defines a transport seam ([`DiscordVcTransport`]) for join/leave/recv/play.
//! 2. Implements the full STT → agent → TTS pipeline over that seam.
//! 3. Ships a [`MockDiscordVcTransport`] so tests need no token or songbird.
//! 4. Marks real songbird wiring as scaffold-only
//!    ([`SONGBIRD_TRANSPORT_IMPLEMENTED`] = `false`) until a dedicated
//!    feature lands.
//!
//! # Feature
//!
//! Gated behind `discord-voice` (which enables `voice`). Enable with:
//!
//! ```bash
//! cargo test -p clawft-channels --features discord-voice
//! ```

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::voice::policy::{VoiceAnswerPolicy, VoiceLlm};
use crate::voice::stt::{SttBackend, Utterance};
use crate::voice::tts::{TtsChunk, TtsEngine};
use crate::voice::types::VoiceError;
use crate::voice::vad::{EnergyVad, VadEvent};

/// Real serenity/songbird transport is not compiled in (WEFT-232 scaffold).
///
/// Flip only when a concrete `DiscordVcTransport` impl over songbird is wired
/// and CI can exercise it behind an optional feature without a default token.
pub const SONGBIRD_TRANSPORT_IMPLEMENTED: bool = false;

/// Human-readable status for operators / readiness probes.
pub fn real_transport_status() -> &'static str {
    if SONGBIRD_TRANSPORT_IMPLEMENTED {
        "songbird transport available"
    } else {
        "scaffold-only: implement DiscordVcTransport over serenity/songbird; \
         use MockDiscordVcTransport for tests"
    }
}

/// Configuration for the Discord VC bridge session.
#[derive(Debug, Clone)]
pub struct DiscordVoiceBridgeConfig {
    /// Guild (server) snowflake.
    pub guild_id: String,
    /// Voice channel snowflake to join.
    pub channel_id: String,
    /// Capture / playback sample rate (Hz). Discord native is 48 kHz;
    /// the canonical STT path expects 16 kHz mono — transport implementations
    /// should resample at the boundary. Tests use 16 kHz end-to-end.
    pub sample_rate: u32,
    /// VAD energy threshold (dBFS).
    pub vad_threshold_dbfs: f32,
    /// Silence tail (ms) that ends an utterance.
    pub silence_ms: u32,
    /// Minimum utterance length (ms); shorter segments are dropped.
    pub min_utterance_ms: u32,
    /// Maximum utterance length (ms); force-flush past this.
    pub max_utterance_ms: u32,
    /// Base agent system persona (spoken-answer policy is appended).
    pub base_system: String,
    /// Optional Discord user ids allowed to drive the bot. Empty = all.
    pub allowed_user_ids: Vec<String>,
}

impl Default for DiscordVoiceBridgeConfig {
    fn default() -> Self {
        Self {
            guild_id: String::new(),
            channel_id: String::new(),
            sample_rate: 16_000,
            vad_threshold_dbfs: -45.0,
            silence_ms: 700,
            min_utterance_ms: 300,
            max_utterance_ms: 20_000,
            base_system: String::new(),
            allowed_user_ids: Vec::new(),
        }
    }
}

impl DiscordVoiceBridgeConfig {
    /// Reject incomplete join targets.
    pub fn validate(&self) -> Result<(), VoiceError> {
        if self.guild_id.is_empty() {
            return Err(VoiceError::Config(
                "discord voice: guild_id is required".into(),
            ));
        }
        if self.channel_id.is_empty() {
            return Err(VoiceError::Config(
                "discord voice: channel_id is required".into(),
            ));
        }
        if self.sample_rate == 0 {
            return Err(VoiceError::Config(
                "discord voice: sample_rate must be > 0".into(),
            ));
        }
        Ok(())
    }

    /// Whether `user_id` may drive the bridge (empty allow-list = everyone).
    pub fn is_user_allowed(&self, user_id: &str) -> bool {
        self.allowed_user_ids.is_empty()
            || self.allowed_user_ids.iter().any(|id| id == user_id)
    }
}

/// One frame of decoded Discord VC audio (post-Opus, mono PCM).
#[derive(Debug, Clone)]
pub struct DiscordVcPcmFrame {
    /// Interleaved mono `s16le` samples at the bridge sample rate.
    pub samples: Vec<i16>,
    /// Discord user snowflake that produced this audio (SSRC → user map).
    pub user_id: Option<String>,
}

/// Discord voice transport seam (join / leave / recv / play).
///
/// Production will implement this over serenity voice state + songbird
/// input/output tracks. Tests inject [`MockDiscordVcTransport`].
#[async_trait]
pub trait DiscordVcTransport: Send + Sync {
    /// Join a guild voice channel.
    async fn join(&self, guild_id: &str, channel_id: &str) -> Result<(), VoiceError>;

    /// Leave the current voice channel (no-op if not connected).
    async fn leave(&self) -> Result<(), VoiceError>;

    /// Whether the transport currently holds a VC connection.
    fn is_connected(&self) -> bool;

    /// Receive the next PCM frame, or `None` when the session ends.
    ///
    /// Implementations block until a frame is available, the connection
    /// drops, or `cancel` fires.
    async fn recv_pcm(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Option<DiscordVcPcmFrame>, VoiceError>;

    /// Play mono PCM into the voice channel.
    async fn play_pcm(&self, samples: &[i16], sample_rate: u32) -> Result<(), VoiceError>;
}

/// Result of one full VC → STT → agent → TTS → VC turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordVoiceTurn {
    /// STT transcript of the user utterance.
    pub transcript: String,
    /// Policy-shaped agent reply text.
    pub reply: String,
    /// Number of TTS chunks rendered.
    pub tts_chunks: usize,
    /// Total PCM samples played out to the VC.
    pub samples_out: usize,
    /// Discord user id that drove the turn (if known).
    pub user_id: Option<String>,
}

/// Discord VC bridge: STT → agent → TTS over a [`DiscordVcTransport`].
pub struct DiscordVoiceBridge {
    transport: Arc<dyn DiscordVcTransport>,
    stt: Arc<dyn SttBackend>,
    llm: Arc<dyn VoiceLlm>,
    tts: Arc<dyn TtsEngine>,
    policy: VoiceAnswerPolicy,
    config: DiscordVoiceBridgeConfig,
}

impl DiscordVoiceBridge {
    /// Build a bridge from trait-injected adapters.
    pub fn new(
        transport: Arc<dyn DiscordVcTransport>,
        stt: Arc<dyn SttBackend>,
        llm: Arc<dyn VoiceLlm>,
        tts: Arc<dyn TtsEngine>,
        policy: VoiceAnswerPolicy,
        config: DiscordVoiceBridgeConfig,
    ) -> Result<Self, VoiceError> {
        config.validate()?;
        Ok(Self {
            transport,
            stt,
            llm,
            tts,
            policy,
            config,
        })
    }

    /// Bridge configuration.
    pub fn config(&self) -> &DiscordVoiceBridgeConfig {
        &self.config
    }

    /// Join the configured VC, warm STT, then process turns until `cancel`.
    pub async fn run(&self, cancel: CancellationToken) -> Result<(), VoiceError> {
        self.transport
            .join(&self.config.guild_id, &self.config.channel_id)
            .await?;
        self.stt.warm().await?;
        info!(
            guild = %self.config.guild_id,
            channel = %self.config.channel_id,
            transport = %real_transport_status(),
            "discord voice bridge joined VC"
        );

        let mut vad = EnergyVad::new(
            self.config.sample_rate,
            self.config.vad_threshold_dbfs,
            self.config.silence_ms,
            self.config.min_utterance_ms,
            self.config.max_utterance_ms,
        );
        let mut capture: Vec<i16> = Vec::new();
        let mut capturing = false;
        let mut speech_user: Option<String> = None;

        while !cancel.is_cancelled() {
            let frame = match self.transport.recv_pcm(&cancel).await? {
                Some(f) => f,
                None => {
                    debug!("discord voice transport closed");
                    break;
                }
            };

            if let Some(ref uid) = frame.user_id
                && !self.config.is_user_allowed(uid)
            {
                continue;
            }

            let events = vad.feed(&frame.samples);
            for event in &events {
                if matches!(event, VadEvent::SpeechStart { .. }) {
                    capturing = true;
                    capture.clear();
                    speech_user = frame.user_id.clone();
                }
            }
            if capturing {
                capture.extend_from_slice(&frame.samples);
            }
            for event in events {
                if matches!(event, VadEvent::SpeechEnd { .. }) && capturing {
                    capturing = false;
                    let samples = std::mem::take(&mut capture);
                    let user = speech_user.take();
                    if samples.is_empty() {
                        continue;
                    }
                    if let Err(e) = self.process_utterance(&samples, user).await {
                        warn!(error = %e, "discord voice turn failed");
                    }
                }
            }
        }

        let _ = self.transport.leave().await;
        Ok(())
    }

    /// Process one pre-segmented utterance (no VAD). Core path used by tests
    /// and by [`Self::run`] after speech-end.
    pub async fn process_utterance(
        &self,
        samples: &[i16],
        user_id: Option<String>,
    ) -> Result<DiscordVoiceTurn, VoiceError> {
        if samples.is_empty() {
            return Err(VoiceError::Audio(
                "discord voice: empty utterance".into(),
            ));
        }
        if let Some(ref uid) = user_id
            && !self.config.is_user_allowed(uid)
        {
            return Err(VoiceError::Config(format!(
                "discord voice: user {uid} not in allow-list"
            )));
        }

        let utt = Utterance {
            samples: samples.to_vec(),
            sample_rate: self.config.sample_rate,
        };
        let transcript = self.stt.transcribe(&utt).await?;
        let transcript = transcript.trim().to_string();
        if transcript.is_empty() {
            debug!("discord voice: empty transcript, skipping agent");
            return Ok(DiscordVoiceTurn {
                transcript,
                reply: String::new(),
                tts_chunks: 0,
                samples_out: 0,
                user_id,
            });
        }

        let req = self
            .policy
            .shape_request(&self.config.base_system, &transcript, None);
        let raw = self.llm.complete(&req).await?;
        let reply = self.policy.enforce(&raw);

        let cancel = CancellationToken::new();
        let (tx, mut rx) = mpsc::channel::<TtsChunk>(8);
        let tts = Arc::clone(&self.tts);
        let reply_for_tts = reply.clone();
        let synth = tokio::spawn(async move {
            tts.synthesize_stream(&reply_for_tts, tx, cancel).await
        });

        let mut tts_chunks = 0usize;
        let mut samples_out = 0usize;
        while let Some(chunk) = rx.recv().await {
            tts_chunks += 1;
            samples_out += chunk.samples.len();
            self.transport
                .play_pcm(&chunk.samples, chunk.sample_rate)
                .await?;
        }
        synth
            .await
            .map_err(|e| VoiceError::Transport(format!("tts join: {e}")))??;

        info!(
            transcript = %transcript,
            reply = %reply,
            tts_chunks,
            samples_out,
            "discord voice turn complete"
        );

        Ok(DiscordVoiceTurn {
            transcript,
            reply,
            tts_chunks,
            samples_out,
            user_id,
        })
    }
}

// ── Mock transport (tests / dry-run) ────────────────────────────────────

/// In-memory Discord VC transport for unit tests — no token, no songbird.
pub struct MockDiscordVcTransport {
    connected: AtomicBool,
    /// Frames to yield from `recv_pcm` (FIFO).
    inbound: Mutex<VecDeque<DiscordVcPcmFrame>>,
    /// PCM segments played via `play_pcm`.
    outbound: Mutex<Vec<Vec<i16>>>,
    /// Join targets recorded for assertions.
    joined: Mutex<Option<(String, String)>>,
    /// When true, `recv_pcm` returns `None` after the inbound queue drains
    /// instead of waiting on cancel (easier deterministic run loops).
    end_on_empty: AtomicBool,
}

impl MockDiscordVcTransport {
    /// Empty mock; push frames with [`Self::push_frame`].
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            inbound: Mutex::new(VecDeque::new()),
            outbound: Mutex::new(Vec::new()),
            joined: Mutex::new(None),
            end_on_empty: AtomicBool::new(true),
        }
    }

    /// Queue a PCM frame for later `recv_pcm`.
    pub fn push_frame(&self, frame: DiscordVcPcmFrame) {
        self.inbound
            .lock()
            .expect("mock inbound poisoned")
            .push_back(frame);
    }

    /// Queue raw samples with optional user id.
    pub fn push_pcm(&self, samples: Vec<i16>, user_id: Option<String>) {
        self.push_frame(DiscordVcPcmFrame { samples, user_id });
    }

    /// Snapshot of played PCM segments (cloned).
    pub fn played(&self) -> Vec<Vec<i16>> {
        self.outbound.lock().expect("mock outbound poisoned").clone()
    }

    /// Total samples played across all segments.
    pub fn played_sample_count(&self) -> usize {
        self.played().iter().map(|s| s.len()).sum()
    }

    /// Last join `(guild_id, channel_id)` if any.
    pub fn joined_target(&self) -> Option<(String, String)> {
        self.joined.lock().expect("mock joined poisoned").clone()
    }

    /// When true (default), empty inbound ends the recv stream.
    pub fn set_end_on_empty(&self, end: bool) {
        self.end_on_empty.store(end, Ordering::SeqCst);
    }
}

impl Default for MockDiscordVcTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DiscordVcTransport for MockDiscordVcTransport {
    async fn join(&self, guild_id: &str, channel_id: &str) -> Result<(), VoiceError> {
        *self.joined.lock().expect("mock joined poisoned") =
            Some((guild_id.to_string(), channel_id.to_string()));
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn leave(&self) -> Result<(), VoiceError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn recv_pcm(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Option<DiscordVcPcmFrame>, VoiceError> {
        if cancel.is_cancelled() {
            return Ok(None);
        }
        if let Some(frame) = self
            .inbound
            .lock()
            .expect("mock inbound poisoned")
            .pop_front()
        {
            return Ok(Some(frame));
        }
        if self.end_on_empty.load(Ordering::SeqCst) {
            return Ok(None);
        }
        // Wait until cancel when holding open for live-style tests.
        cancel.cancelled().await;
        Ok(None)
    }

    async fn play_pcm(&self, samples: &[i16], _sample_rate: u32) -> Result<(), VoiceError> {
        if !self.is_connected() {
            return Err(VoiceError::Audio(
                "mock discord VC: play while not connected".into(),
            ));
        }
        self.outbound
            .lock()
            .expect("mock outbound poisoned")
            .push(samples.to_vec());
        Ok(())
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::policy::VoiceTurnRequest;
    use crate::voice::stt::SttModel;
    use crate::voice::tts::TtsTier;

    struct MockStt(&'static str);
    #[async_trait]
    impl SttBackend for MockStt {
        async fn warm(&self) -> Result<(), VoiceError> {
            Ok(())
        }
        async fn transcribe(&self, _utt: &Utterance) -> Result<String, VoiceError> {
            Ok(self.0.to_string())
        }
        fn model(&self) -> SttModel {
            SttModel::ParakeetEnglish
        }
    }

    struct MockLlm(&'static str);
    #[async_trait]
    impl VoiceLlm for MockLlm {
        async fn complete(&self, req: &VoiceTurnRequest) -> Result<String, VoiceError> {
            assert!(!req.user.is_empty());
            Ok(self.0.to_string())
        }
    }

    struct MockTts;
    #[async_trait]
    impl TtsEngine for MockTts {
        async fn synthesize_stream(
            &self,
            text: &str,
            tx: mpsc::Sender<TtsChunk>,
            cancel: CancellationToken,
        ) -> Result<(), VoiceError> {
            if cancel.is_cancelled() || text.trim().is_empty() {
                return Ok(());
            }
            let _ = tx
                .send(TtsChunk {
                    samples: vec![42i16; 320],
                    sample_rate: 16_000,
                })
                .await;
            Ok(())
        }
        fn tier(&self) -> TtsTier {
            TtsTier::Slow
        }
    }

    fn cfg() -> DiscordVoiceBridgeConfig {
        DiscordVoiceBridgeConfig {
            guild_id: "guild-1".into(),
            channel_id: "vc-1".into(),
            ..Default::default()
        }
    }

    fn bridge(
        transport: Arc<MockDiscordVcTransport>,
        transcript: &'static str,
        reply: &'static str,
    ) -> DiscordVoiceBridge {
        DiscordVoiceBridge::new(
            transport,
            Arc::new(MockStt(transcript)),
            Arc::new(MockLlm(reply)),
            Arc::new(MockTts),
            VoiceAnswerPolicy::default(),
            cfg(),
        )
        .unwrap()
    }

    #[test]
    fn scaffold_marks_songbird_unimplemented() {
        assert!(!SONGBIRD_TRANSPORT_IMPLEMENTED);
        assert!(real_transport_status().contains("scaffold-only"));
    }

    #[test]
    fn config_requires_guild_and_channel() {
        let mut c = DiscordVoiceBridgeConfig::default();
        assert!(c.validate().is_err());
        c.guild_id = "g".into();
        assert!(c.validate().is_err());
        c.channel_id = "c".into();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn allow_list_filters_users() {
        let mut c = cfg();
        assert!(c.is_user_allowed("anyone"));
        c.allowed_user_ids = vec!["u1".into()];
        assert!(c.is_user_allowed("u1"));
        assert!(!c.is_user_allowed("u2"));
    }

    #[tokio::test]
    async fn process_utterance_stt_agent_tts_to_vc() {
        let transport = Arc::new(MockDiscordVcTransport::new());
        transport.join("guild-1", "vc-1").await.unwrap();
        let b = bridge(Arc::clone(&transport), "hello bot", "Hi there.");

        let samples = vec![1000i16; 1600]; // 100 ms @ 16 kHz
        let turn = b
            .process_utterance(&samples, Some("user-42".into()))
            .await
            .unwrap();

        assert_eq!(turn.transcript, "hello bot");
        assert_eq!(turn.reply, "Hi there.");
        assert_eq!(turn.tts_chunks, 1);
        assert_eq!(turn.samples_out, 320);
        assert_eq!(turn.user_id.as_deref(), Some("user-42"));
        assert_eq!(transport.played().len(), 1);
        assert_eq!(transport.played()[0], vec![42i16; 320]);
    }

    #[tokio::test]
    async fn process_utterance_rejects_disallowed_user() {
        let transport = Arc::new(MockDiscordVcTransport::new());
        transport.join("guild-1", "vc-1").await.unwrap();
        let mut config = cfg();
        config.allowed_user_ids = vec!["allowed".into()];
        let b = DiscordVoiceBridge::new(
            transport,
            Arc::new(MockStt("hi")),
            Arc::new(MockLlm("yo")),
            Arc::new(MockTts),
            VoiceAnswerPolicy::default(),
            config,
        )
        .unwrap();

        let err = b
            .process_utterance(&[1i16; 100], Some("stranger".into()))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not in allow-list"));
    }

    #[tokio::test]
    async fn process_utterance_skips_empty_transcript() {
        let transport = Arc::new(MockDiscordVcTransport::new());
        transport.join("guild-1", "vc-1").await.unwrap();
        let b = bridge(transport.clone(), "   ", "should not speak");
        let turn = b.process_utterance(&[1i16; 100], None).await.unwrap();
        assert!(turn.transcript.is_empty());
        assert_eq!(turn.tts_chunks, 0);
        assert!(transport.played().is_empty());
    }

    #[tokio::test]
    async fn mock_transport_join_leave_and_play_requires_connection() {
        let t = MockDiscordVcTransport::new();
        assert!(!t.is_connected());
        let err = t.play_pcm(&[1], 16_000).await.unwrap_err();
        assert!(err.to_string().contains("not connected"));

        t.join("g", "c").await.unwrap();
        assert!(t.is_connected());
        assert_eq!(t.joined_target(), Some(("g".into(), "c".into())));
        t.play_pcm(&[7, 8], 16_000).await.unwrap();
        assert_eq!(t.played(), vec![vec![7, 8]]);
        t.leave().await.unwrap();
        assert!(!t.is_connected());
    }

    #[tokio::test]
    async fn run_joins_processes_and_leaves_without_live_token() {
        // Loud speech frames then silence so EnergyVad emits SpeechEnd.
        // 16 kHz, ~50 ms frames of near-full-scale sine-ish constant, then silence.
        let transport = Arc::new(MockDiscordVcTransport::new());
        let frame_len = 800; // 50 ms
        let speech = vec![12_000i16; frame_len];
        let silence = vec![0i16; frame_len];

        // ~400 ms speech + ~900 ms silence (> default 700 ms tail)
        for _ in 0..8 {
            transport.push_pcm(speech.clone(), Some("u1".into()));
        }
        for _ in 0..20 {
            transport.push_pcm(silence.clone(), Some("u1".into()));
        }

        let b = bridge(
            Arc::clone(&transport),
            "what is two plus two",
            "Four.",
        );
        let cancel = CancellationToken::new();
        b.run(cancel).await.unwrap();

        assert_eq!(
            transport.joined_target(),
            Some(("guild-1".into(), "vc-1".into()))
        );
        // Left after stream end.
        assert!(!transport.is_connected());
        // TTS played into the mock VC.
        assert!(!transport.played().is_empty());
        assert_eq!(transport.played_sample_count(), 320);
    }
}
