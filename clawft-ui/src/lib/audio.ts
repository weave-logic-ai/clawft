/**
 * Audio utilities for voice features.
 *
 * ## Capture backends (WEFT-228)
 *
 * - **Tauri shell:** native **cpal** mic via Tauri commands (`mic_*`).
 *   Prefer this when `isTauriShell()` is true — avoids browser-only
 *   `getUserMedia` and uses CoreAudio / ALSA / WASAPI.
 * - **Browser / Vite dev:** Web Audio + `getUserMedia` (unchanged).
 *
 * Playback and Web Speech remain browser-side; the axum backend still
 * handles configuration and message processing (text → agent → text).
 */

import { api } from "./api-client";
import {
  tokenizeWords,
  wordIndexAtChar,
  wordIndexAtElapsed,
  type WordBoundaryInfo,
  type WordTiming,
} from "./voice-highlight";

export type { WordBoundaryInfo, WordTiming };

// ---------------------------------------------------------------------------
// Tauri native bridge (WEFT-228)
// ---------------------------------------------------------------------------

type TauriInvoke = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

/** Soft-detect Tauri 2 invoke without requiring `@tauri-apps/api` at build. */
function getTauriInvoke(): TauriInvoke | null {
  if (typeof window === "undefined") return null;
  const w = window as unknown as {
    __TAURI_INTERNALS__?: { invoke?: TauriInvoke };
    __TAURI__?: { core?: { invoke?: TauriInvoke } };
  };
  return w.__TAURI_INTERNALS__?.invoke ?? w.__TAURI__?.core?.invoke ?? null;
}

/** True when the dashboard is running inside the clawft-ui Tauri shell. */
export function isTauriShell(): boolean {
  return getTauriInvoke() !== null;
}

interface CmdEnvelope<T> {
  ok: boolean;
  data?: T | null;
  error?: string | null;
}

async function tauriCmd<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const invoke = getTauriInvoke();
  if (!invoke) {
    throw new Error("not running in Tauri shell");
  }
  // Tauri 2 passes a single args object; unwrap our CmdResponse envelope.
  const raw = (await invoke(cmd, args ?? {})) as CmdEnvelope<T> | T;
  if (
    raw &&
    typeof raw === "object" &&
    "ok" in (raw as object) &&
    typeof (raw as CmdEnvelope<T>).ok === "boolean"
  ) {
    const env = raw as CmdEnvelope<T>;
    if (!env.ok) {
      throw new Error(env.error || `${cmd} failed`);
    }
    return env.data as T;
  }
  return raw as T;
}

export interface NativeMicBackendInfo {
  available: boolean;
  backend: string;
  hostBackend: string;
  targetOs: string;
  notes: string;
  capturing: boolean;
}

export interface NativeMicDevice {
  name: string;
  isDefault: boolean;
}

export interface NativeMicSession {
  device: string;
  sampleRate: number;
  channels: number;
  backend: string;
  targetOs: string;
}

export interface NativeMicPoll {
  pcmI16: number[];
  sampleRate: number;
  peak: number;
  capturing: boolean;
}

export interface NativeMicLevelReport {
  level: number;
  device: string;
  sampleRate: number;
  backend: string;
  targetOs: string;
}

/** Diagnostics for the native cpal path (no-op error if not in Tauri). */
export async function nativeMicBackendInfo(): Promise<NativeMicBackendInfo | null> {
  if (!isTauriShell()) return null;
  try {
    return await tauriCmd<NativeMicBackendInfo>("mic_backend_info");
  } catch {
    return null;
  }
}

/** List native input devices (Tauri only). */
export async function nativeMicListDevices(): Promise<NativeMicDevice[]> {
  return tauriCmd<NativeMicDevice[]>("mic_list_devices");
}

/** Start continuous native capture (Tauri only). */
export async function nativeMicStart(device?: string): Promise<NativeMicSession> {
  return tauriCmd<NativeMicSession>("mic_start", {
    args: device ? { device } : {},
  });
}

/** Stop native capture (Tauri only). */
export async function nativeMicStop(): Promise<boolean> {
  return tauriCmd<boolean>("mic_stop");
}

/** Poll buffered PCM + peak from the active native session (Tauri only). */
export async function nativeMicPoll(): Promise<NativeMicPoll> {
  return tauriCmd<NativeMicPoll>("mic_poll");
}

// ---------------------------------------------------------------------------
// Microphone access & level metering
// ---------------------------------------------------------------------------

let _audioCtx: AudioContext | null = null;

function getAudioContext(): AudioContext {
  if (!_audioCtx || _audioCtx.state === "closed") {
    _audioCtx = new AudioContext();
  }
  return _audioCtx;
}

/**
 * Measure peak mic level over ~1 second.
 *
 * Prefers native cpal via Tauri (`mic_test_level`) when available (WEFT-228);
 * falls back to browser `getUserMedia` + Web Audio analyser.
 */
export async function testMicrophone(): Promise<{ level: number }> {
  if (isTauriShell()) {
    try {
      const report = await tauriCmd<NativeMicLevelReport>("mic_test_level", {
        args: { durationMs: 1000 },
      });
      return { level: Math.min(Math.max(report.level, 0), 1) };
    } catch {
      // Fall through to browser path (e.g. permission denied / no device).
    }
  }

  const stream = await navigator.mediaDevices.getUserMedia({
    audio: {
      echoCancellation: true,
      noiseSuppression: true,
    },
  });

  const ctx = getAudioContext();
  if (ctx.state === "suspended") await ctx.resume();
  const source = ctx.createMediaStreamSource(stream);
  const analyser = ctx.createAnalyser();
  analyser.fftSize = 256;
  source.connect(analyser);

  const data = new Uint8Array(analyser.frequencyBinCount);
  let peak = 0;

  // Sample for ~1 second (10 frames at ~100ms each).
  await new Promise<void>((resolve) => {
    let frames = 0;
    const interval = setInterval(() => {
      analyser.getByteTimeDomainData(data);
      for (let i = 0; i < data.length; i++) {
        const v = Math.abs(data[i] - 128) / 128;
        if (v > peak) peak = v;
      }
      frames++;
      if (frames >= 10) {
        clearInterval(interval);
        resolve();
      }
    }, 100);
  });

  // Cleanup
  source.disconnect();
  stream.getTracks().forEach((t) => t.stop());

  return { level: Math.min(peak, 1) };
}

/** Play a short test tone (~0.3s, 440Hz sine wave). */
export async function testSpeaker(): Promise<void> {
  const ctx = getAudioContext();
  if (ctx.state === "suspended") await ctx.resume();

  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = "sine";
  osc.frequency.value = 440;
  gain.gain.value = 0.3;

  osc.connect(gain);
  gain.connect(ctx.destination);

  osc.start();
  // Fade out to avoid click
  gain.gain.setValueAtTime(0.3, ctx.currentTime + 0.25);
  gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.35);
  osc.stop(ctx.currentTime + 0.35);

  await new Promise((r) => setTimeout(r, 400));
}

// ---------------------------------------------------------------------------
// Speech Recognition (Web Speech API)
// ---------------------------------------------------------------------------

type SpeechRecognitionEvent = Event & {
  results: SpeechRecognitionResultList;
  resultIndex: number;
};

type SpeechRecognitionErrorEvent = Event & {
  error: string;
  message: string;
};

/** Check if SpeechRecognition is available in this browser. */
export function hasSpeechRecognition(): boolean {
  return !!(
    (window as unknown as Record<string, unknown>).SpeechRecognition ||
    (window as unknown as Record<string, unknown>).webkitSpeechRecognition
  );
}

/**
 * Create a SpeechRecognition instance. Returns null if unsupported.
 *
 * The returned object follows the Web Speech API interface.
 */
export function createSpeechRecognition(opts?: {
  lang?: string;
  continuous?: boolean;
  interimResults?: boolean;
}): SpeechRecognition | null {
  const Ctor =
    (window as unknown as Record<string, unknown>).SpeechRecognition ||
    (window as unknown as Record<string, unknown>).webkitSpeechRecognition;
  if (!Ctor) return null;

  const recognition = new (Ctor as new () => SpeechRecognition)();
  recognition.lang = opts?.lang || "en-US";
  recognition.continuous = opts?.continuous ?? false;
  recognition.interimResults = opts?.interimResults ?? true;
  return recognition;
}

// Type augmentation for SpeechRecognition (not in all TS libs)
interface SpeechRecognition extends EventTarget {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  start(): void;
  stop(): void;
  abort(): void;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: SpeechRecognitionErrorEvent) => void) | null;
  onend: (() => void) | null;
  onstart: (() => void) | null;
}

interface SpeechRecognitionResultList {
  readonly length: number;
  item(index: number): SpeechRecognitionResult;
  [index: number]: SpeechRecognitionResult;
}

interface SpeechRecognitionResult {
  readonly isFinal: boolean;
  readonly length: number;
  item(index: number): SpeechRecognitionAlternative;
  [index: number]: SpeechRecognitionAlternative;
}

interface SpeechRecognitionAlternative {
  readonly transcript: string;
  readonly confidence: number;
}

export type { SpeechRecognition, SpeechRecognitionEvent, SpeechRecognitionErrorEvent };

// ---------------------------------------------------------------------------
// Speech Synthesis (TTS)
// ---------------------------------------------------------------------------

/**
 * Strip markdown formatting so text reads naturally via TTS.
 *
 * Removes headers, bold/italic markers, code fences, link URLs,
 * horizontal rules, list bullets, and other symbols that sound
 * wrong when read aloud.
 */
export function stripMarkdownForSpeech(md: string): string {
  let text = md;

  // Remove code fences (``` ... ```)
  text = text.replace(/```[\s\S]*?```/g, "");
  // Remove inline code backticks
  text = text.replace(/`([^`]*)`/g, "$1");

  // Convert links [text](url) → text
  text = text.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");

  // Remove bare URLs
  text = text.replace(/https?:\/\/[^\s)>]+/g, "");

  // Remove images ![alt](url)
  text = text.replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1");

  // Remove horizontal rules (---, ***, ___)
  text = text.replace(/^[-*_]{3,}\s*$/gm, "");

  // Remove headers (## Header → Header)
  text = text.replace(/^#{1,6}\s+/gm, "");

  // Remove bold/italic markers
  text = text.replace(/\*{1,3}([^*]+)\*{1,3}/g, "$1");
  text = text.replace(/_{1,3}([^_]+)_{1,3}/g, "$1");
  text = text.replace(/~~([^~]+)~~/g, "$1");

  // Convert list bullets to natural pauses
  text = text.replace(/^\s*[-*+]\s+/gm, ". ");
  // Numbered lists: "1. " → keep as-is (reads fine)

  // Remove blockquote markers
  text = text.replace(/^\s*>\s?/gm, "");

  // Remove HTML tags
  text = text.replace(/<[^>]+>/g, "");

  // Collapse multiple newlines to single period+space
  text = text.replace(/\n{2,}/g, ". ");
  // Single newlines → space
  text = text.replace(/\n/g, " ");

  // Collapse multiple spaces
  text = text.replace(/\s{2,}/g, " ");

  // Remove leading/trailing periods and spaces
  text = text.replace(/^[\s.]+/, "").replace(/[\s.]+$/, "");

  // Clean up double periods
  text = text.replace(/\.{2,}/g, ".");
  text = text.replace(/\.\s*\./g, ".");

  return text.trim();
}

// ── Cloud TTS playback state ──

/** Currently playing audio source (for cancellation). */
let _currentAudioSource: AudioBufferSourceNode | null = null;

/** rAF id for word-timing clock during cloud playback. */
let _wordClockRaf = 0;

function stopWordClock(): void {
  if (_wordClockRaf) {
    cancelAnimationFrame(_wordClockRaf);
    _wordClockRaf = 0;
  }
}

/** Cancel any in-progress speech — both cloud audio and browser TTS. */
export function cancelSpeech(): void {
  stopWordClock();
  // Stop cloud audio playback
  if (_currentAudioSource) {
    try {
      _currentAudioSource.stop();
    } catch {
      // already stopped
    }
    _currentAudioSource = null;
  }
  // Stop browser TTS
  if (window.speechSynthesis) {
    window.speechSynthesis.cancel();
  }
}

export interface SpeakOptions {
  lang?: string;
  rate?: number;
  /**
   * Optional absolute word timings (ms from audio start).
   * Used for cloud/blob playback highlight. Absent → graceful no-op
   * (no word callbacks from the audio clock).
   */
  wordTimings?: WordTiming[];
  /**
   * Fired on each spoken-word boundary when available
   * (Web Speech `boundary` events, or word-timing clock).
   */
  onWord?: (info: WordBoundaryInfo) => void;
}

/**
 * Play an audio blob (MP3) through the Web Audio API.
 *
 * Returns a promise that resolves when playback completes.
 * The playing source is stored so `cancelSpeech()` can stop it.
 *
 * When `wordTimings` + `onWord` are provided, drives highlight via elapsed time.
 * Without timings the surface is a graceful no-op (audio still plays).
 */
async function playAudioBlob(
  blob: Blob,
  opts?: { wordTimings?: WordTiming[]; onWord?: (info: WordBoundaryInfo) => void },
): Promise<void> {
  const ctx = getAudioContext();
  if (ctx.state === "suspended") await ctx.resume();

  const arrayBuffer = await blob.arrayBuffer();
  const audioBuffer = await ctx.decodeAudioData(arrayBuffer);

  return new Promise<void>((resolve) => {
    const source = ctx.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(ctx.destination);
    _currentAudioSource = source;

    const timings = opts?.wordTimings;
    const onWord = opts?.onWord;
    let lastIndex = -2;
    const startAt = performance.now();

    const tick = () => {
      if (!_currentAudioSource) return;
      if (timings && timings.length && onWord) {
        const elapsed = performance.now() - startAt;
        const idx = wordIndexAtElapsed(timings, elapsed);
        if (idx !== null && idx !== lastIndex && idx >= 0) {
          lastIndex = idx;
          const t = timings[idx];
          onWord({
            wordIndex: idx,
            charIndex: -1,
            name: "word",
            word: t?.word ?? "",
          });
        }
      }
      _wordClockRaf = requestAnimationFrame(tick);
    };

    // Only start the clock when timings are available — graceful no-op otherwise.
    if (timings && timings.length && onWord) {
      _wordClockRaf = requestAnimationFrame(tick);
    }

    source.onended = () => {
      stopWordClock();
      _currentAudioSource = null;
      resolve();
    };
    source.start(0);
  });
}

// ── TTS provider cache ──

let _ttsProvider: string | null = null;
let _ttsProviderChecked = false;

/** Providers that the server `/api/voice/tts` cloud proxy can synthesize. */
const CLOUD_TTS_PROVIDERS = new Set(["openai", "elevenlabs"]);

/** Fetch the TTS provider setting from the backend (cached). */
async function getTtsProvider(): Promise<string> {
  // Default matches VoiceConfig.tts.provider (WEFT-238: "local").
  if (_ttsProviderChecked) return _ttsProvider || "local";
  try {
    const cfg = await api.voice.ttsConfig();
    _ttsProvider = cfg.provider;
    _ttsProviderChecked = true;
    return cfg.provider;
  } catch {
    _ttsProviderChecked = true;
    return "local";
  }
}

/** Reset the TTS provider cache (call after settings change). */
export function resetTtsProviderCache(): void {
  _ttsProvider = null;
  _ttsProviderChecked = false;
}

/**
 * Speak text using the best available TTS engine.
 *
 * Cloud providers (`openai`, `elevenlabs`) go through `/api/voice/tts`.
 * Default `local` / `local-stub` / `browser` use the browser Web Speech
 * API (WEFT-238 — no server browser dispatch; UI path is intentional).
 *
 * ## Word highlighting (WEFT-231)
 *
 * - Browser TTS: fires `onWord` from SpeechSynthesis `boundary` events.
 * - Cloud blob + `wordTimings`: fires `onWord` from an elapsed-time clock.
 * - Cloud blob without timings: graceful no-op (audio plays; no highlight events).
 */
export async function speak(
  text: string,
  opts?: SpeakOptions,
): Promise<void> {
  const clean = stripMarkdownForSpeech(text);
  if (!clean) return;

  const provider = await getTtsProvider();

  // Only hit the cloud proxy for providers it can synthesize.
  if (CLOUD_TTS_PROVIDERS.has(provider)) {
    try {
      const blob = await api.voice.tts(clean, { speed: opts?.rate });
      await playAudioBlob(blob, {
        wordTimings: opts?.wordTimings,
        onWord: opts?.onWord,
      });
      return;
    } catch {
      // Cloud TTS failed — fall through to browser Web Speech
    }
  }

  // local / browser / unknown: Web Speech API (always available in Chromium UI)
  return speakBrowser(clean, opts);
}

/** Speak text using the browser's built-in speech synthesis. */
function speakBrowser(
  cleanText: string,
  opts?: SpeakOptions,
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!window.speechSynthesis) {
      reject(new Error("Speech synthesis not available"));
      return;
    }
    const utterance = new SpeechSynthesisUtterance(cleanText);
    utterance.lang = opts?.lang || "en-US";
    utterance.rate = opts?.rate || 1.0;

    // WEFT-231: word boundary → highlight. No-op when onWord is omitted.
    if (opts?.onWord) {
      const spans = tokenizeWords(cleanText);
      // Prefer absolute timings if the caller supplied them even for browser path.
      const timings = opts.wordTimings;
      utterance.onboundary = (ev: SpeechSynthesisEvent) => {
        if (ev.name && ev.name !== "word" && ev.name !== "sentence") return;
        if (ev.name === "sentence") return;
        let wordIndex = wordIndexAtChar(spans, ev.charIndex);
        // If timings provided, prefer elapsed (charIndex can be unreliable on some engines).
        if (timings && timings.length && typeof ev.elapsedTime === "number") {
          const fromTime = wordIndexAtElapsed(timings, ev.elapsedTime);
          if (fromTime !== null && fromTime >= 0) wordIndex = fromTime;
        }
        const word =
          wordIndex >= 0 && wordIndex < spans.length
            ? spans[wordIndex].word
            : "";
        opts.onWord?.({
          wordIndex,
          charIndex: ev.charIndex,
          name: ev.name || "word",
          word,
        });
      };
    }

    utterance.onend = () => resolve();
    utterance.onerror = (e) => reject(e);
    window.speechSynthesis.speak(utterance);
  });
}
