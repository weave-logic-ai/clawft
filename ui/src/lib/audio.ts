/**
 * Browser-side audio utilities for voice features.
 *
 * All audio capture/playback happens in the browser via Web Audio API
 * and Web Speech API. The backend only handles configuration and
 * message processing (transcribed text → agent → response text).
 */

import { api } from "./api-client";

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

/** Request microphone access and measure the peak level over ~1 second. */
export async function testMicrophone(): Promise<{ level: number }> {
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

/** Cancel any in-progress speech — both cloud audio and browser TTS. */
export function cancelSpeech(): void {
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

/**
 * Play an audio blob (MP3) through the Web Audio API.
 *
 * Returns a promise that resolves when playback completes.
 * The playing source is stored so `cancelSpeech()` can stop it.
 */
async function playAudioBlob(blob: Blob): Promise<void> {
  const ctx = getAudioContext();
  if (ctx.state === "suspended") await ctx.resume();

  const arrayBuffer = await blob.arrayBuffer();
  const audioBuffer = await ctx.decodeAudioData(arrayBuffer);

  return new Promise<void>((resolve) => {
    const source = ctx.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(ctx.destination);
    _currentAudioSource = source;
    source.onended = () => {
      _currentAudioSource = null;
      resolve();
    };
    source.start(0);
  });
}

// ── TTS provider cache ──

let _ttsProvider: string | null = null;
let _ttsProviderChecked = false;

/** Fetch the TTS provider setting from the backend (cached). */
async function getTtsProvider(): Promise<string> {
  if (_ttsProviderChecked) return _ttsProvider || "browser";
  try {
    const cfg = await api.voice.ttsConfig();
    _ttsProvider = cfg.provider;
    _ttsProviderChecked = true;
    return cfg.provider;
  } catch {
    _ttsProviderChecked = true;
    return "browser";
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
 * If a cloud TTS provider is configured (e.g. "openai"), the text is sent
 * to the backend `/api/voice/tts` proxy which returns high-quality audio.
 * Falls back to the browser's built-in Web Speech API if cloud is
 * unavailable or not configured.
 */
export async function speak(
  text: string,
  opts?: { lang?: string; rate?: number },
): Promise<void> {
  const clean = stripMarkdownForSpeech(text);
  if (!clean) return;

  const provider = await getTtsProvider();

  // Try cloud TTS first when configured
  if (provider !== "browser") {
    try {
      const blob = await api.voice.tts(clean, { speed: opts?.rate });
      await playAudioBlob(blob);
      return;
    } catch {
      // Cloud TTS failed — fall through to browser TTS
    }
  }

  // Fallback: browser Web Speech API
  return speakBrowser(clean, opts);
}

/** Speak text using the browser's built-in speech synthesis. */
function speakBrowser(
  cleanText: string,
  opts?: { lang?: string; rate?: number },
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!window.speechSynthesis) {
      reject(new Error("Speech synthesis not available"));
      return;
    }
    const utterance = new SpeechSynthesisUtterance(cleanText);
    utterance.lang = opts?.lang || "en-US";
    utterance.rate = opts?.rate || 1.0;
    utterance.onend = () => resolve();
    utterance.onerror = (e) => reject(e);
    window.speechSynthesis.speak(utterance);
  });
}
