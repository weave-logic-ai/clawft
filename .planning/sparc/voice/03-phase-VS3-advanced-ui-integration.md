# Phase VS3: Advanced UI Integration + Cloud Fallback + Advanced Features

> **Element:** Voice Development -- Sprint 3
> **Phase:** VS3
> **Timeline:** Weeks 7-9
> **Priority:** P1 (UI Voice Integration), P2 (Cloud Fallback, Advanced Features)
> **Crates:** `ui/src/components/voice/` (TypeScript), `clawft-plugin`, `clawft-core`, `clawft-tools`
> **Dependencies IN:** VS1 (Core Pipeline), VS2 (Wake Word + Platform), UI Sprint S1 (Backend API + WebSocket), UI Sprint S2 (Dashboard Framework)
> **Blocks:** K6 (Native Shells -- Tauri voice integration point)
> **Status:** Planning

---

## 1. Overview

Phase VS3 is the final voice sprint, bridging the voice pipeline (VS1/VS2) with the web dashboard UI, adding cloud STT/TTS fallback for reliability, and delivering advanced features including per-agent voice personality, voice command shortcuts, audio file tools, and comprehensive benchmarking.

VS3 is split into three sub-phases across three weeks:

- **VS3.1 (Week 7):** UI voice components -- status bar, Talk Mode overlay, waveform visualizer, settings panel, push-to-talk, WebSocket event streaming, Tauri native mic access. All components use shadcn/ui and integrate with the existing dashboard layout from UI Sprint S1/S2.
- **VS3.2 (Week 8):** Cloud fallback providers for STT (OpenAI Whisper API) and TTS (ElevenLabs/OpenAI TTS), fallback chain logic (local-first with cloud on failure/low confidence), speaker diarization via sherpa-rs, and voice transcription logging to JSONL session files.
- **VS3.3 (Week 9):** Per-agent voice personality configuration, voice command shortcuts, `audio_transcribe` and `audio_synthesize` tools, latency/WER/CPU benchmarking suites, voice permission integration, and end-to-end Playwright tests with audio simulation.

---

## 2. Current Code

### Existing Voice Infrastructure (from VS1/VS2)

By VS3, the following exists from prior sprints:

- `clawft-plugin/src/voice/` -- `AudioCapture`, `AudioPlayback`, `VoiceActivityDetector`, `SpeechToText`, `TextToSpeech`, `WakeWordDetector`, `VoiceChannel` (ChannelAdapter), `TalkModeController`
- `clawft-tools/` -- `voice_listen` tool (on-demand STT), `voice_speak` tool (on-demand TTS)
- `clawft-services/` -- WebSocket voice event: `voice:status` (idle/listening/processing/speaking)
- `clawft-types/` -- `VoiceConfig`, silence timeout, voice selection types

### Existing UI Infrastructure (from S1/S2)

- `ui/src/hooks/use-websocket.ts` -- Reconnecting WebSocket client with topic subscription
- `ui/src/lib/ws-client.ts` -- WebSocket transport layer
- `ui/src/lib/api-client.ts` -- Fetch wrapper with auth token
- `ui/src/components/ui/` -- shadcn/ui component library (Button, Card, Badge, Slider, Select, Switch, Dialog, Sheet, Tabs)
- `ui/src/stores/ws-store.ts` -- Zustand store for WebSocket state
- `ui/src/components/layout/main-layout.tsx` -- Dashboard shell with sidebar

### No Existing UI Voice Components

There are no `ui/src/components/voice/` files yet. This phase creates them from scratch.

---

## 3. Implementation Tasks

### Task VS3.1: UI Voice Integration (Week 7)

#### VS3.1.1 VoiceStatusBar Component

Voice status indicator for the dashboard header. Shows current pipeline state with animated transitions.

**File:** `ui/src/components/voice/status-bar.tsx`

```tsx
"use client";

import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Mic, MicOff, Loader2, Volume2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWebSocket } from "@/hooks/use-websocket";

export type VoiceState = "idle" | "listening" | "processing" | "speaking";

interface VoiceStatusBarProps {
  className?: string;
  onClick?: () => void;
}

const stateConfig: Record<
  VoiceState,
  { icon: React.ElementType; label: string; color: string; pulse: boolean }
> = {
  idle: {
    icon: MicOff,
    label: "Voice Off",
    color: "bg-muted text-muted-foreground",
    pulse: false,
  },
  listening: {
    icon: Mic,
    label: "Listening",
    color: "bg-green-500/15 text-green-600 border-green-500/30",
    pulse: true,
  },
  processing: {
    icon: Loader2,
    label: "Processing",
    color: "bg-yellow-500/15 text-yellow-600 border-yellow-500/30",
    pulse: false,
  },
  speaking: {
    icon: Volume2,
    label: "Speaking",
    color: "bg-blue-500/15 text-blue-600 border-blue-500/30",
    pulse: true,
  },
};

export function VoiceStatusBar({ className, onClick }: VoiceStatusBarProps) {
  const [voiceState, setVoiceState] = useState<VoiceState>("idle");
  const { subscribe } = useWebSocket();

  useEffect(() => {
    const unsub = subscribe("voice:status", (event: { state: VoiceState }) => {
      setVoiceState(event.state);
    });
    return unsub;
  }, [subscribe]);

  const config = stateConfig[voiceState];
  const Icon = config.icon;

  return (
    <Badge
      variant="outline"
      className={cn(
        "cursor-pointer select-none gap-1.5 px-2.5 py-1 transition-all",
        config.color,
        config.pulse && "animate-pulse",
        className,
      )}
      onClick={onClick}
    >
      <Icon
        className={cn("h-3.5 w-3.5", voiceState === "processing" && "animate-spin")}
      />
      <span className="text-xs font-medium">{config.label}</span>
    </Badge>
  );
}
```

#### VS3.1.2 TalkModeOverlay Component

Floating overlay showing live conversation transcript with stop and mute controls. Appears when Talk Mode is active.

**File:** `ui/src/components/voice/talk-overlay.tsx`

```tsx
"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter, CardHeader } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Mic, MicOff, Square, GripVertical } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWebSocket } from "@/hooks/use-websocket";

interface TranscriptEntry {
  id: string;
  speaker: "user" | "agent";
  text: string;
  timestamp: number;
  partial?: boolean;
}

interface TalkModeOverlayProps {
  isOpen: boolean;
  onStop: () => void;
  className?: string;
}

export function TalkModeOverlay({ isOpen, onStop, className }: TalkModeOverlayProps) {
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [isMuted, setIsMuted] = useState(false);
  const [partialText, setPartialText] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const { subscribe, send } = useWebSocket();

  useEffect(() => {
    const unsubPartial = subscribe(
      "voice:partial_transcription",
      (event: { text: string }) => {
        setPartialText(event.text);
      },
    );

    const unsubFinal = subscribe(
      "voice:transcription",
      (event: { text: string; speaker: "user" | "agent" }) => {
        setPartialText("");
        setTranscript((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            speaker: event.speaker,
            text: event.text,
            timestamp: Date.now(),
          },
        ]);
      },
    );

    const unsubTts = subscribe(
      "voice:tts_text",
      (event: { text: string }) => {
        setTranscript((prev) => [
          ...prev,
          {
            id: crypto.randomUUID(),
            speaker: "agent",
            text: event.text,
            timestamp: Date.now(),
          },
        ]);
      },
    );

    return () => {
      unsubPartial();
      unsubFinal();
      unsubTts();
    };
  }, [subscribe]);

  useEffect(() => {
    scrollRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcript, partialText]);

  const handleMuteToggle = useCallback(() => {
    const newMuted = !isMuted;
    setIsMuted(newMuted);
    send({ type: "voice:mute", muted: newMuted });
  }, [isMuted, send]);

  if (!isOpen) return null;

  return (
    <Card
      className={cn(
        "fixed bottom-4 right-4 z-50 w-96 shadow-2xl border-2",
        "bg-background/95 backdrop-blur-sm",
        className,
      )}
    >
      <CardHeader className="flex flex-row items-center justify-between py-2 px-3">
        <div className="flex items-center gap-2">
          <GripVertical className="h-4 w-4 text-muted-foreground cursor-grab" />
          <span className="text-sm font-semibold">Talk Mode</span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleMuteToggle}
          >
            {isMuted ? (
              <MicOff className="h-4 w-4 text-destructive" />
            ) : (
              <Mic className="h-4 w-4 text-green-600" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-destructive hover:text-destructive"
            onClick={onStop}
          >
            <Square className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>

      <CardContent className="p-0">
        <ScrollArea className="h-64 px-3">
          <div className="space-y-2 py-2">
            {transcript.map((entry) => (
              <div
                key={entry.id}
                className={cn(
                  "rounded-lg px-3 py-2 text-sm max-w-[85%]",
                  entry.speaker === "user"
                    ? "ml-auto bg-primary text-primary-foreground"
                    : "bg-muted",
                )}
              >
                {entry.text}
              </div>
            ))}
            {partialText && (
              <div className="ml-auto rounded-lg px-3 py-2 text-sm max-w-[85%] bg-primary/50 text-primary-foreground italic">
                {partialText}
              </div>
            )}
            <div ref={scrollRef} />
          </div>
        </ScrollArea>
      </CardContent>

      <CardFooter className="py-2 px-3 text-xs text-muted-foreground">
        {isMuted ? "Microphone muted" : "Listening..."}
      </CardFooter>
    </Card>
  );
}
```

#### VS3.1.3 AudioWaveform Component

Real-time microphone input visualization using the Web Audio API. Renders an animated waveform of the mic signal.

**File:** `ui/src/components/voice/waveform.tsx`

```tsx
"use client";

import { useCallback, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

interface AudioWaveformProps {
  stream: MediaStream | null;
  isActive: boolean;
  barCount?: number;
  className?: string;
}

export function AudioWaveform({
  stream,
  isActive,
  barCount = 32,
  className,
}: AudioWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>(0);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const contextRef = useRef<AudioContext | null>(null);

  const cleanup = useCallback(() => {
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
      animationRef.current = 0;
    }
    if (contextRef.current && contextRef.current.state !== "closed") {
      contextRef.current.close();
      contextRef.current = null;
    }
    analyserRef.current = null;
  }, []);

  useEffect(() => {
    if (!stream || !isActive || !canvasRef.current) {
      cleanup();
      return;
    }

    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const audioCtx = new AudioContext();
    contextRef.current = audioCtx;

    const analyser = audioCtx.createAnalyser();
    analyser.fftSize = 256;
    analyser.smoothingTimeConstant = 0.8;
    analyserRef.current = analyser;

    const source = audioCtx.createMediaStreamSource(stream);
    source.connect(analyser);

    const bufferLength = analyser.frequencyBinCount;
    const dataArray = new Uint8Array(bufferLength);

    const draw = () => {
      if (!analyserRef.current) return;
      animationRef.current = requestAnimationFrame(draw);

      analyserRef.current.getByteFrequencyData(dataArray);

      const { width, height } = canvas;
      ctx.clearRect(0, 0, width, height);

      const barWidth = width / barCount;
      const step = Math.floor(bufferLength / barCount);

      for (let i = 0; i < barCount; i++) {
        const value = dataArray[i * step] ?? 0;
        const barHeight = (value / 255) * height;
        const x = i * barWidth;
        const y = height - barHeight;

        const hue = 142 + (value / 255) * 60;
        ctx.fillStyle = `hsl(${hue}, 70%, 50%)`;
        ctx.fillRect(x + 1, y, barWidth - 2, barHeight);
      }
    };

    draw();

    return cleanup;
  }, [stream, isActive, barCount, cleanup]);

  return (
    <canvas
      ref={canvasRef}
      width={256}
      height={48}
      className={cn("rounded-md bg-muted/50", className)}
    />
  );
}
```

#### VS3.1.4 VoiceSettings Panel

Settings panel for voice configuration: microphone selection, voice selection, language, wake word toggle.

**File:** `ui/src/components/voice/settings.tsx`

```tsx
"use client";

import { useCallback, useEffect, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Button } from "@/components/ui/button";
import { useApiClient } from "@/hooks/use-api";

interface AudioDevice {
  deviceId: string;
  label: string;
  kind: "audioinput" | "audiooutput";
}

interface VoiceSettingsData {
  inputDevice: string;
  outputDevice: string;
  voice: string;
  language: string;
  wakeWordEnabled: boolean;
  wakeWordSensitivity: number;
  silenceTimeout: number;
  cloudFallbackEnabled: boolean;
}

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
  { value: "ko", label: "Korean" },
  { value: "pt", label: "Portuguese" },
  { value: "ru", label: "Russian" },
  { value: "ar", label: "Arabic" },
  { value: "hi", label: "Hindi" },
  { value: "it", label: "Italian" },
];

const VOICES = [
  { value: "default", label: "Default" },
  { value: "alloy", label: "Alloy" },
  { value: "echo", label: "Echo" },
  { value: "fable", label: "Fable" },
  { value: "onyx", label: "Onyx" },
  { value: "nova", label: "Nova" },
  { value: "shimmer", label: "Shimmer" },
];

interface VoiceSettingsProps {
  className?: string;
}

export function VoiceSettings({ className }: VoiceSettingsProps) {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [settings, setSettings] = useState<VoiceSettingsData>({
    inputDevice: "default",
    outputDevice: "default",
    voice: "default",
    language: "en",
    wakeWordEnabled: false,
    wakeWordSensitivity: 0.5,
    silenceTimeout: 1500,
    cloudFallbackEnabled: false,
  });
  const api = useApiClient();

  useEffect(() => {
    navigator.mediaDevices.enumerateDevices().then((allDevices) => {
      setDevices(
        allDevices
          .filter((d) => d.kind === "audioinput" || d.kind === "audiooutput")
          .map((d) => ({
            deviceId: d.deviceId,
            label: d.label || `${d.kind} (${d.deviceId.slice(0, 8)})`,
            kind: d.kind as "audioinput" | "audiooutput",
          })),
      );
    });

    api.get<VoiceSettingsData>("/api/voice/config").then((data) => {
      if (data) setSettings(data);
    });
  }, [api]);

  const updateSetting = useCallback(
    <K extends keyof VoiceSettingsData>(key: K, value: VoiceSettingsData[K]) => {
      setSettings((prev) => {
        const next = { ...prev, [key]: value };
        api.patch("/api/voice/config", next);
        return next;
      });
    },
    [api],
  );

  const inputDevices = devices.filter((d) => d.kind === "audioinput");
  const outputDevices = devices.filter((d) => d.kind === "audiooutput");

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle>Voice Settings</CardTitle>
        <CardDescription>Configure microphone, voice output, and wake word.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Microphone Selection */}
        <div className="space-y-2">
          <Label>Microphone</Label>
          <Select
            value={settings.inputDevice}
            onValueChange={(v) => updateSetting("inputDevice", v)}
          >
            <SelectTrigger><SelectValue placeholder="Select microphone" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="default">System Default</SelectItem>
              {inputDevices.map((d) => (
                <SelectItem key={d.deviceId} value={d.deviceId}>{d.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Speaker Selection */}
        <div className="space-y-2">
          <Label>Speaker</Label>
          <Select
            value={settings.outputDevice}
            onValueChange={(v) => updateSetting("outputDevice", v)}
          >
            <SelectTrigger><SelectValue placeholder="Select speaker" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="default">System Default</SelectItem>
              {outputDevices.map((d) => (
                <SelectItem key={d.deviceId} value={d.deviceId}>{d.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Voice Selection */}
        <div className="space-y-2">
          <Label>Voice</Label>
          <Select
            value={settings.voice}
            onValueChange={(v) => updateSetting("voice", v)}
          >
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {VOICES.map((v) => (
                <SelectItem key={v.value} value={v.value}>{v.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Language Selection */}
        <div className="space-y-2">
          <Label>Language</Label>
          <Select
            value={settings.language}
            onValueChange={(v) => updateSetting("language", v)}
          >
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {LANGUAGES.map((l) => (
                <SelectItem key={l.value} value={l.value}>{l.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Wake Word Toggle */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label>Wake Word ("Hey Weft")</Label>
            <p className="text-xs text-muted-foreground">Always-on listening for activation</p>
          </div>
          <Switch
            checked={settings.wakeWordEnabled}
            onCheckedChange={(v) => updateSetting("wakeWordEnabled", v)}
          />
        </div>

        {/* Wake Word Sensitivity */}
        {settings.wakeWordEnabled && (
          <div className="space-y-2">
            <Label>Wake Word Sensitivity ({Math.round(settings.wakeWordSensitivity * 100)}%)</Label>
            <Slider
              value={[settings.wakeWordSensitivity]}
              onValueChange={([v]) => updateSetting("wakeWordSensitivity", v)}
              min={0.1}
              max={1.0}
              step={0.05}
            />
          </div>
        )}

        {/* Silence Timeout */}
        <div className="space-y-2">
          <Label>Silence Timeout ({settings.silenceTimeout}ms)</Label>
          <Slider
            value={[settings.silenceTimeout]}
            onValueChange={([v]) => updateSetting("silenceTimeout", v)}
            min={500}
            max={5000}
            step={100}
          />
        </div>

        {/* Cloud Fallback */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label>Cloud Fallback</Label>
            <p className="text-xs text-muted-foreground">Use cloud STT/TTS when local fails</p>
          </div>
          <Switch
            checked={settings.cloudFallbackEnabled}
            onCheckedChange={(v) => updateSetting("cloudFallbackEnabled", v)}
          />
        </div>

        {/* Test Buttons */}
        <div className="flex gap-2">
          <Button variant="outline" size="sm" onClick={() => api.post("/api/voice/test-mic", {})}>
            Test Mic
          </Button>
          <Button variant="outline" size="sm" onClick={() => api.post("/api/voice/test-speak", {})}>
            Test TTS
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
```

#### VS3.1.5 PushToTalkButton Component

Hold-to-speak button. Captures audio while held, processes on release.

**File:** `ui/src/components/voice/ptt-button.tsx`

```tsx
"use client";

import { useCallback, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Mic } from "lucide-react";
import { cn } from "@/lib/utils";
import { useWebSocket } from "@/hooks/use-websocket";

interface PushToTalkButtonProps {
  className?: string;
  size?: "default" | "sm" | "lg" | "icon";
}

export function PushToTalkButton({ className, size = "lg" }: PushToTalkButtonProps) {
  const [isHeld, setIsHeld] = useState(false);
  const streamRef = useRef<MediaStream | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const { send } = useWebSocket();

  const startCapture = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true },
      });
      streamRef.current = stream;

      const recorder = new MediaRecorder(stream, { mimeType: "audio/webm;codecs=opus" });
      recorderRef.current = recorder;
      chunksRef.current = [];

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data);
      };

      recorder.start(100);
      setIsHeld(true);
      send({ type: "voice:ptt_start" });
    } catch {
      // Mic permission denied or unavailable
    }
  }, [send]);

  const stopCapture = useCallback(async () => {
    setIsHeld(false);

    const recorder = recorderRef.current;
    if (!recorder || recorder.state === "inactive") return;

    return new Promise<void>((resolve) => {
      recorder.onstop = async () => {
        const blob = new Blob(chunksRef.current, { type: "audio/webm;codecs=opus" });
        const buffer = await blob.arrayBuffer();
        const base64 = btoa(
          new Uint8Array(buffer).reduce((s, b) => s + String.fromCharCode(b), ""),
        );
        send({ type: "voice:ptt_audio", audio: base64, mime: "audio/webm;codecs=opus" });
        resolve();
      };
      recorder.stop();
    }).finally(() => {
      streamRef.current?.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
      recorderRef.current = null;
      send({ type: "voice:ptt_stop" });
    });
  }, [send]);

  return (
    <Button
      variant={isHeld ? "default" : "outline"}
      size={size}
      className={cn(
        "select-none touch-none transition-all",
        isHeld && "scale-110 ring-2 ring-green-500 ring-offset-2 bg-green-600 hover:bg-green-600",
        className,
      )}
      onPointerDown={startCapture}
      onPointerUp={stopCapture}
      onPointerLeave={isHeld ? stopCapture : undefined}
    >
      <Mic className={cn("h-5 w-5", isHeld && "animate-pulse text-white")} />
      <span className="ml-2">{isHeld ? "Release to send" : "Hold to talk"}</span>
    </Button>
  );
}
```

#### VS3.1.6 WebSocket Voice Events -- Partial Transcription Streaming

Extend the existing WebSocket event bus to support real-time partial transcription and TTS progress events.

**File (backend extension):** `clawft-services/src/api/ws_voice.rs`

```rust
use serde::Serialize;

/// Voice-specific WebSocket events streamed to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum VoiceWsEvent {
    /// Pipeline state change (idle/listening/processing/speaking).
    #[serde(rename = "voice:status")]
    Status { state: VoiceState },

    /// Partial (interim) STT transcription for real-time display.
    #[serde(rename = "voice:partial_transcription")]
    PartialTranscription { text: String, confidence: f32 },

    /// Final STT transcription after silence detected.
    #[serde(rename = "voice:transcription")]
    Transcription {
        text: String,
        speaker: String,
        language: String,
        duration_ms: u64,
    },

    /// TTS text being spoken (for word highlighting).
    #[serde(rename = "voice:tts_text")]
    TtsText { text: String },

    /// TTS progress: word index currently being spoken.
    #[serde(rename = "voice:tts_progress")]
    TtsProgress {
        word_index: usize,
        total_words: usize,
    },

    /// TTS playback completed.
    #[serde(rename = "voice:tts_complete")]
    TtsComplete,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceState {
    Idle,
    Listening,
    Processing,
    Speaking,
}

/// Emit a voice event to all subscribed WebSocket clients.
pub async fn broadcast_voice_event(
    ws_broadcaster: &crate::api::WsBroadcaster,
    event: VoiceWsEvent,
) {
    let payload = serde_json::to_value(&event).unwrap_or_default();
    ws_broadcaster.broadcast("voice", payload).await;
}
```

#### VS3.1.7 WebSocket Voice Events -- TTS Progress

TTS word-level progress tracking. The TTS engine emits progress events as each word is spoken, enabling the UI to highlight the current word in the transcript overlay.

Integrated into `broadcast_voice_event` above via `VoiceWsEvent::TtsProgress`. The `TextToSpeech` engine in `clawft-plugin` emits these events through the WebSocket broadcaster as synthesis proceeds.

#### VS3.1.8 Tauri Voice Integration

Native microphone access from the Tauri desktop shell. Tauri provides direct access to system audio devices without browser permission prompts.

**File:** `ui/src-tauri/src/voice.rs`

```rust
use tauri::command;

/// Check if microphone permission is granted on this platform.
#[command]
pub async fn check_mic_permission() -> Result<bool, String> {
    // On macOS, check AVCaptureDevice authorization status.
    // On Windows/Linux, mic access is generally available without explicit permission.
    #[cfg(target_os = "macos")]
    {
        // Tauri's native dialog or platform API to check mic permission.
        // Returns true if already granted, false if denied or undetermined.
        Ok(true) // Placeholder -- actual implementation uses objc bridge
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

/// Request microphone permission from the operating system.
#[command]
pub async fn request_mic_permission() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        // Trigger macOS mic permission dialog via AVCaptureDevice.
        Ok(true) // Placeholder
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

/// List available audio input devices via cpal (native, not WebAudio).
#[command]
pub async fn list_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let devices: Vec<AudioDeviceInfo> = host
        .input_devices()
        .map_err(|e| format!("Failed to enumerate devices: {e}"))?
        .filter_map(|d| {
            Some(AudioDeviceInfo {
                name: d.name().ok()?,
                is_default: false, // Set below
            })
        })
        .collect();

    Ok(devices)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}
```

**Tauri plugin registration** in `ui/src-tauri/src/main.rs`:

```rust
mod voice;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            voice::check_mic_permission,
            voice::request_mic_permission,
            voice::list_audio_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
```

---

### Task VS3.2: Cloud Fallback + Quality (Week 8)

#### VS3.2.1 CloudSttProvider Trait + OpenAI Whisper API

Cloud STT provider trait and OpenAI Whisper API implementation for fallback when local STT fails or has low confidence.

**File:** `clawft-plugin/src/voice/cloud_stt.rs`

```rust
use async_trait::async_trait;
use crate::PluginError;

/// Trait for cloud-based speech-to-text providers.
#[async_trait]
pub trait CloudSttProvider: Send + Sync {
    /// Provider name (e.g., "openai-whisper").
    fn name(&self) -> &str;

    /// Transcribe audio bytes to text.
    ///
    /// `audio_data` is raw audio in the format specified by `mime_type`.
    /// `language` is an optional BCP-47 language hint (e.g., "en").
    /// Returns transcribed text and a confidence score (0.0-1.0).
    async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<CloudSttResult, PluginError>;
}

#[derive(Debug, Clone)]
pub struct CloudSttResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub duration_ms: u64,
}

/// OpenAI Whisper API implementation of CloudSttProvider.
pub struct WhisperSttProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl WhisperSttProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "whisper-1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[async_trait]
impl CloudSttProvider for WhisperSttProvider {
    fn name(&self) -> &str {
        "openai-whisper"
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<CloudSttResult, PluginError> {
        let extension = match mime_type {
            "audio/wav" => "wav",
            "audio/webm" | "audio/webm;codecs=opus" => "webm",
            "audio/mp3" | "audio/mpeg" => "mp3",
            "audio/ogg" | "audio/ogg;codecs=opus" => "ogg",
            _ => "wav",
        };

        let file_part = reqwest::multipart::Part::bytes(audio_data.to_vec())
            .file_name(format!("audio.{extension}"))
            .mime_str(mime_type)
            .map_err(|e| PluginError::ExecutionFailed(format!("MIME error: {e}")))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let resp = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| PluginError::ExecutionFailed(format!("Whisper API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(
                format!("Whisper API returned {status}: {body}"),
            ));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| PluginError::ExecutionFailed(format!("Whisper response parse error: {e}")))?;

        Ok(CloudSttResult {
            text: body["text"].as_str().unwrap_or("").to_string(),
            confidence: 0.95, // Whisper does not return confidence; assume high
            language: body["language"].as_str().unwrap_or("en").to_string(),
            duration_ms: (body["duration"].as_f64().unwrap_or(0.0) * 1000.0) as u64,
        })
    }
}
```

#### VS3.2.2 CloudTtsProvider Trait + ElevenLabs/OpenAI TTS

Cloud TTS provider trait with two implementations.

**File:** `clawft-plugin/src/voice/cloud_tts.rs`

```rust
use async_trait::async_trait;
use crate::PluginError;

/// Trait for cloud-based text-to-speech providers.
#[async_trait]
pub trait CloudTtsProvider: Send + Sync {
    /// Provider name (e.g., "elevenlabs", "openai-tts").
    fn name(&self) -> &str;

    /// Available voice IDs for this provider.
    fn available_voices(&self) -> Vec<VoiceInfo>;

    /// Synthesize text to audio bytes.
    ///
    /// Returns audio data and its MIME type (e.g., "audio/mp3").
    async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
    ) -> Result<CloudTtsResult, PluginError>;
}

#[derive(Debug, Clone)]
pub struct CloudTtsResult {
    pub audio_data: Vec<u8>,
    pub mime_type: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
}

/// OpenAI TTS API implementation.
pub struct OpenAiTtsProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiTtsProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "tts-1".to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CloudTtsProvider for OpenAiTtsProvider {
    fn name(&self) -> &str { "openai-tts" }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        ["alloy", "echo", "fable", "onyx", "nova", "shimmer"]
            .iter()
            .map(|v| VoiceInfo {
                id: v.to_string(),
                name: v.to_string(),
                language: "en".to_string(),
            })
            .collect()
    }

    async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
    ) -> Result<CloudTtsResult, PluginError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice_id,
            "response_format": "mp3",
        });

        let resp = self.client
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| PluginError::ExecutionFailed(format!("OpenAI TTS request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(
                format!("OpenAI TTS returned {status}: {err_body}"),
            ));
        }

        let audio_data = resp.bytes().await
            .map_err(|e| PluginError::ExecutionFailed(format!("TTS response read error: {e}")))?
            .to_vec();

        Ok(CloudTtsResult {
            audio_data,
            mime_type: "audio/mp3".to_string(),
            duration_ms: None,
        })
    }
}

/// ElevenLabs TTS API implementation.
pub struct ElevenLabsTtsProvider {
    api_key: String,
    client: reqwest::Client,
}

impl ElevenLabsTtsProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CloudTtsProvider for ElevenLabsTtsProvider {
    fn name(&self) -> &str { "elevenlabs" }

    fn available_voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo { id: "21m00Tcm4TlvDq8ikWAM".into(), name: "Rachel".into(), language: "en".into() },
            VoiceInfo { id: "AZnzlk1XvdvUeBnXmlld".into(), name: "Domi".into(), language: "en".into() },
            VoiceInfo { id: "EXAVITQu4vr4xnSDxMaL".into(), name: "Bella".into(), language: "en".into() },
            VoiceInfo { id: "ErXwobaYiN019PkySvjV".into(), name: "Antoni".into(), language: "en".into() },
        ]
    }

    async fn synthesize(
        &self,
        text: &str,
        voice_id: &str,
    ) -> Result<CloudTtsResult, PluginError> {
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.75,
            },
        });

        let resp = self.client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("Accept", "audio/mpeg")
            .json(&body)
            .send()
            .await
            .map_err(|e| PluginError::ExecutionFailed(format!("ElevenLabs request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(
                format!("ElevenLabs returned {status}: {err_body}"),
            ));
        }

        let audio_data = resp.bytes().await
            .map_err(|e| PluginError::ExecutionFailed(format!("ElevenLabs read error: {e}")))?
            .to_vec();

        Ok(CloudTtsResult {
            audio_data,
            mime_type: "audio/mpeg".to_string(),
            duration_ms: None,
        })
    }
}
```

#### VS3.2.3 FallbackChain

Orchestrates local-first with cloud fallback on failure or low confidence.

**File:** `clawft-plugin/src/voice/fallback.rs`

```rust
use crate::PluginError;
use super::cloud_stt::{CloudSttProvider, CloudSttResult};
use super::cloud_tts::{CloudTtsProvider, CloudTtsResult};

/// Minimum confidence score to accept a local STT result.
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.60;

/// Fallback chain for STT: local -> cloud on failure or low confidence.
pub struct SttFallbackChain {
    local: Box<dyn LocalSttEngine>,
    cloud: Option<Box<dyn CloudSttProvider>>,
    confidence_threshold: f32,
}

/// Trait for the local STT engine (sherpa-rs based).
#[async_trait::async_trait]
pub trait LocalSttEngine: Send + Sync {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<LocalSttResult, PluginError>;
}

#[derive(Debug, Clone)]
pub struct LocalSttResult {
    pub text: String,
    pub confidence: f32,
}

impl SttFallbackChain {
    pub fn new(local: Box<dyn LocalSttEngine>) -> Self {
        Self {
            local,
            cloud: None,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
        }
    }

    pub fn with_cloud(mut self, provider: Box<dyn CloudSttProvider>) -> Self {
        self.cloud = Some(provider);
        self
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Transcribe audio, falling back to cloud if local fails or confidence is low.
    pub async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        language: Option<&str>,
    ) -> Result<SttFallbackResult, PluginError> {
        // Try local first
        match self.local.transcribe(audio_data, language).await {
            Ok(local_result) if local_result.confidence >= self.confidence_threshold => {
                return Ok(SttFallbackResult {
                    text: local_result.text,
                    confidence: local_result.confidence,
                    source: SttSource::Local,
                    language: language.unwrap_or("en").to_string(),
                });
            }
            Ok(low_confidence) => {
                // Local succeeded but confidence too low -- try cloud
                if let Some(cloud) = &self.cloud {
                    match cloud.transcribe(audio_data, mime_type, language).await {
                        Ok(cloud_result) => {
                            return Ok(SttFallbackResult {
                                text: cloud_result.text,
                                confidence: cloud_result.confidence,
                                source: SttSource::Cloud(cloud.name().to_string()),
                                language: cloud_result.language,
                            });
                        }
                        Err(_) => {
                            // Cloud also failed -- return low-confidence local result
                            return Ok(SttFallbackResult {
                                text: low_confidence.text,
                                confidence: low_confidence.confidence,
                                source: SttSource::Local,
                                language: language.unwrap_or("en").to_string(),
                            });
                        }
                    }
                }
                // No cloud provider -- return low-confidence result
                return Ok(SttFallbackResult {
                    text: low_confidence.text,
                    confidence: low_confidence.confidence,
                    source: SttSource::Local,
                    language: language.unwrap_or("en").to_string(),
                });
            }
            Err(local_err) => {
                // Local failed -- try cloud
                if let Some(cloud) = &self.cloud {
                    let cloud_result = cloud.transcribe(audio_data, mime_type, language).await?;
                    return Ok(SttFallbackResult {
                        text: cloud_result.text,
                        confidence: cloud_result.confidence,
                        source: SttSource::Cloud(cloud.name().to_string()),
                        language: cloud_result.language,
                    });
                }
                return Err(local_err);
            }
        }
    }
}

/// TTS fallback chain: local -> cloud on failure.
pub struct TtsFallbackChain {
    local: Box<dyn LocalTtsEngine>,
    cloud: Option<Box<dyn CloudTtsProvider>>,
}

#[async_trait::async_trait]
pub trait LocalTtsEngine: Send + Sync {
    async fn synthesize(&self, text: &str) -> Result<(Vec<u8>, String), PluginError>;
}

impl TtsFallbackChain {
    pub fn new(local: Box<dyn LocalTtsEngine>) -> Self {
        Self { local, cloud: None }
    }

    pub fn with_cloud(mut self, provider: Box<dyn CloudTtsProvider>) -> Self {
        self.cloud = Some(provider);
        self
    }

    pub async fn synthesize(
        &self,
        text: &str,
        voice_id: Option<&str>,
    ) -> Result<TtsFallbackResult, PluginError> {
        match self.local.synthesize(text).await {
            Ok((audio, mime)) => Ok(TtsFallbackResult {
                audio_data: audio,
                mime_type: mime,
                source: TtsSource::Local,
            }),
            Err(_local_err) => {
                if let Some(cloud) = &self.cloud {
                    let voice = voice_id.unwrap_or("alloy");
                    let result = cloud.synthesize(text, voice).await?;
                    Ok(TtsFallbackResult {
                        audio_data: result.audio_data,
                        mime_type: result.mime_type,
                        source: TtsSource::Cloud(cloud.name().to_string()),
                    })
                } else {
                    Err(_local_err)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SttFallbackResult {
    pub text: String,
    pub confidence: f32,
    pub source: SttSource,
    pub language: String,
}

#[derive(Debug, Clone)]
pub enum SttSource {
    Local,
    Cloud(String),
}

#[derive(Debug, Clone)]
pub struct TtsFallbackResult {
    pub audio_data: Vec<u8>,
    pub mime_type: String,
    pub source: TtsSource,
}

#[derive(Debug, Clone)]
pub enum TtsSource {
    Local,
    Cloud(String),
}
```

#### VS3.2.4 SpeakerDiarizer

Multi-speaker identification using sherpa-rs speaker diarization.

**File:** `clawft-plugin/src/voice/diarizer.rs`

```rust
use crate::PluginError;

/// Identifies distinct speakers in an audio stream.
pub struct SpeakerDiarizer {
    /// Minimum number of distinct speakers to detect.
    min_speakers: usize,
    /// Maximum number of distinct speakers to detect.
    max_speakers: usize,
    /// Speaker embedding model (loaded from sherpa-rs).
    _model_path: String,
}

#[derive(Debug, Clone)]
pub struct DiarizedSegment {
    /// Speaker label (e.g., "speaker_0", "speaker_1").
    pub speaker: String,
    /// Start time in milliseconds.
    pub start_ms: u64,
    /// End time in milliseconds.
    pub end_ms: u64,
    /// Transcribed text for this segment (filled after STT).
    pub text: Option<String>,
}

impl SpeakerDiarizer {
    pub fn new(model_path: impl Into<String>) -> Self {
        Self {
            min_speakers: 1,
            max_speakers: 8,
            _model_path: model_path.into(),
        }
    }

    pub fn with_speaker_range(mut self, min: usize, max: usize) -> Self {
        self.min_speakers = min;
        self.max_speakers = max;
        self
    }

    /// Diarize audio data into speaker-labeled segments.
    ///
    /// `audio_data` is 16kHz mono 16-bit PCM.
    /// Returns segments ordered by start time.
    pub async fn diarize(
        &self,
        audio_data: &[i16],
        _sample_rate: u32,
    ) -> Result<Vec<DiarizedSegment>, PluginError> {
        // sherpa-rs speaker diarization integration.
        // Uses speaker embedding model to cluster audio segments by speaker.
        //
        // Implementation outline:
        // 1. Run VAD to find speech segments
        // 2. Extract speaker embeddings for each segment
        // 3. Cluster embeddings (spectral clustering or agglomerative)
        // 4. Assign speaker labels

        // Placeholder -- actual sherpa-rs integration in implementation
        let _ = (audio_data, self.min_speakers, self.max_speakers);
        Err(PluginError::NotImplemented(
            "Speaker diarization requires sherpa-rs speaker embedding model".into(),
        ))
    }
}
```

#### VS3.2.5 Voice Transcription Logging

Persist voice conversations to JSONL session files for audit and replay.

**File:** `clawft-core/src/voice/transcript_log.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// A single entry in the voice transcript log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Speaker identifier ("user", "agent", or diarized speaker label).
    pub speaker: String,
    /// Transcribed or synthesized text.
    pub text: String,
    /// Source of transcription ("local", "cloud:openai-whisper", etc.).
    pub source: String,
    /// Confidence score (0.0-1.0) for STT entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Detected language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Duration of the audio segment in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Appends voice transcript entries to a JSONL file.
pub struct TranscriptLogger {
    path: PathBuf,
}

impl TranscriptLogger {
    /// Create a new logger for the given session.
    ///
    /// Log files are stored at: `{workspace}/voice_sessions/{session_id}.jsonl`
    pub fn new(workspace: &Path, session_id: &str) -> std::io::Result<Self> {
        let dir = workspace.join("voice_sessions");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{session_id}.jsonl"));
        Ok(Self { path })
    }

    /// Append a transcript entry to the log file.
    pub async fn log(&self, entry: &TranscriptEntry) -> std::io::Result<()> {
        let mut line = serde_json::to_string(entry)?;
        line.push('\n');

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    /// Read all entries from the log file.
    pub async fn read_all(&self) -> std::io::Result<Vec<TranscriptEntry>> {
        let content = tokio::fs::read_to_string(&self.path).await?;
        let entries: Vec<TranscriptEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(entries)
    }

    /// Path to the log file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
```

---

### Task VS3.3: Advanced Voice Features (Week 9)

#### VS3.3.1 Per-Agent Voice Personality

Configuration type for per-agent voice personality in multi-agent setups.

**File:** `clawft-types/src/voice/personality.rs`

```rust
use serde::{Deserialize, Serialize};

/// Voice personality configuration for an agent.
///
/// Each agent in a multi-agent setup can have a distinct voice,
/// allowing users to distinguish agents by sound.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePersonality {
    /// Voice model or voice ID to use for TTS.
    /// For local TTS: model name (e.g., "en_US-amy-medium").
    /// For cloud TTS: provider-specific voice ID (e.g., "nova", "EXAVITQu4vr4xnSDxMaL").
    pub voice_id: String,

    /// Preferred TTS provider ("local", "openai", "elevenlabs").
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Speech rate multiplier (0.5 = half speed, 2.0 = double speed).
    #[serde(default = "default_speed")]
    pub speed: f32,

    /// Pitch adjustment (-1.0 to 1.0, 0.0 = default).
    #[serde(default)]
    pub pitch: f32,

    /// Optional spoken name prefix (e.g., "This is Agent Alpha.").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_prefix: Option<String>,

    /// Language code for this agent's voice (BCP-47).
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_provider() -> String { "local".to_string() }
fn default_speed() -> f32 { 1.0 }
fn default_language() -> String { "en".to_string() }

impl Default for VoicePersonality {
    fn default() -> Self {
        Self {
            voice_id: "default".to_string(),
            provider: default_provider(),
            speed: default_speed(),
            pitch: 0.0,
            greeting_prefix: None,
            language: default_language(),
        }
    }
}
```

#### VS3.3.2 Voice Command Shortcuts

Maps spoken trigger phrases to direct tool invocations, bypassing the full LLM pipeline for common commands.

**File:** `clawft-plugin/src/voice/commands.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A voice command shortcut that maps a spoken phrase to a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    /// Trigger phrases (any of these activates the command).
    /// Matched after STT, case-insensitive, with fuzzy tolerance.
    pub triggers: Vec<String>,
    /// Tool name to invoke.
    pub tool: String,
    /// Static parameters to pass to the tool.
    #[serde(default)]
    pub params: serde_json::Value,
    /// Whether this command requires voice confirmation before executing.
    #[serde(default)]
    pub confirm: bool,
    /// Human-readable description (for help listing).
    pub description: String,
}

/// Registry of voice command shortcuts.
pub struct VoiceCommandRegistry {
    commands: Vec<VoiceCommand>,
    /// Precomputed lowercase triggers for fast matching.
    trigger_index: HashMap<String, usize>,
}

impl VoiceCommandRegistry {
    pub fn new(commands: Vec<VoiceCommand>) -> Self {
        let mut trigger_index = HashMap::new();
        for (idx, cmd) in commands.iter().enumerate() {
            for trigger in &cmd.triggers {
                trigger_index.insert(trigger.to_lowercase(), idx);
            }
        }
        Self {
            commands,
            trigger_index,
        }
    }

    /// Match a transcribed phrase against registered commands.
    ///
    /// Returns the matched command if the transcription starts with
    /// or closely matches a registered trigger phrase.
    pub fn match_command(&self, transcription: &str) -> Option<&VoiceCommand> {
        let lower = transcription.to_lowercase().trim().to_string();

        // Exact prefix match
        for (trigger, idx) in &self.trigger_index {
            if lower.starts_with(trigger) {
                return Some(&self.commands[*idx]);
            }
        }

        // Fuzzy match: Levenshtein distance <= 2 on the first N words
        let words: Vec<&str> = lower.split_whitespace().collect();
        for (trigger, idx) in &self.trigger_index {
            let trigger_words: Vec<&str> = trigger.split_whitespace().collect();
            if words.len() >= trigger_words.len() {
                let spoken = words[..trigger_words.len()].join(" ");
                if levenshtein_distance(&spoken, trigger) <= 2 {
                    return Some(&self.commands[*idx]);
                }
            }
        }

        None
    }

    pub fn list(&self) -> &[VoiceCommand] {
        &self.commands
    }
}

/// Simple Levenshtein distance for fuzzy matching.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}
```

#### VS3.3.3 audio_transcribe Tool

Agent tool to process audio files (.wav/.mp3) through STT.

**File:** `clawft-tools/src/audio_transcribe.rs`

```rust
use async_trait::async_trait;
use clawft_plugin::{PluginError, Tool, ToolContext};
use serde_json::{json, Value};

/// Tool for transcribing audio files to text.
///
/// Accepts a file path to a .wav or .mp3 file and returns the transcription.
pub struct AudioTranscribeTool;

#[async_trait]
impl Tool for AudioTranscribeTool {
    fn name(&self) -> &str {
        "audio_transcribe"
    }

    fn description(&self) -> &str {
        "Transcribe an audio file (.wav, .mp3, .ogg, .webm) to text using speech-to-text."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["file_path"],
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the audio file to transcribe."
                },
                "language": {
                    "type": "string",
                    "description": "Optional BCP-47 language hint (e.g., 'en', 'es', 'ja')."
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: &dyn ToolContext,
    ) -> Result<Value, PluginError> {
        let file_path = params["file_path"]
            .as_str()
            .ok_or_else(|| PluginError::ExecutionFailed("file_path is required".into()))?;

        let language = params["language"].as_str();

        // Validate file exists
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            return Err(PluginError::ExecutionFailed(
                format!("File not found: {file_path}"),
            ));
        }

        // Determine MIME type from extension
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("wav") => "audio/wav",
            Some("mp3") => "audio/mpeg",
            Some("ogg") => "audio/ogg",
            Some("webm") => "audio/webm",
            Some(ext) => {
                return Err(PluginError::ExecutionFailed(
                    format!("Unsupported audio format: .{ext}"),
                ));
            }
            None => {
                return Err(PluginError::ExecutionFailed(
                    "File has no extension".into(),
                ));
            }
        };

        // Read audio file
        let audio_data = tokio::fs::read(file_path).await
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to read file: {e}")))?;

        // Use the STT fallback chain (injected via runtime context in actual implementation).
        // For now, return a placeholder indicating the tool contract.
        let _ = (audio_data, mime_type, language);

        Ok(json!({
            "status": "transcribed",
            "file": file_path,
            "text": "",
            "language": language.unwrap_or("en"),
            "note": "STT engine integration pending -- tool contract defined"
        }))
    }
}
```

#### VS3.3.4 audio_synthesize Tool

Agent tool to save TTS output to an audio file.

**File:** `clawft-tools/src/audio_synthesize.rs`

```rust
use async_trait::async_trait;
use clawft_plugin::{PluginError, Tool, ToolContext};
use serde_json::{json, Value};

/// Tool for synthesizing text to an audio file.
pub struct AudioSynthesizeTool;

#[async_trait]
impl Tool for AudioSynthesizeTool {
    fn name(&self) -> &str {
        "audio_synthesize"
    }

    fn description(&self) -> &str {
        "Synthesize text to speech and save as an audio file (.wav)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text", "output_path"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to synthesize into speech."
                },
                "output_path": {
                    "type": "string",
                    "description": "Absolute path for the output .wav file."
                },
                "voice": {
                    "type": "string",
                    "description": "Voice ID to use (default: system default voice)."
                },
                "speed": {
                    "type": "number",
                    "description": "Speech rate multiplier (0.5-2.0, default 1.0)."
                }
            }
        })
    }

    async fn execute(
        &self,
        params: Value,
        _ctx: &dyn ToolContext,
    ) -> Result<Value, PluginError> {
        let text = params["text"]
            .as_str()
            .ok_or_else(|| PluginError::ExecutionFailed("text is required".into()))?;

        let output_path = params["output_path"]
            .as_str()
            .ok_or_else(|| PluginError::ExecutionFailed("output_path is required".into()))?;

        let _voice = params["voice"].as_str().unwrap_or("default");
        let _speed = params["speed"].as_f64().unwrap_or(1.0) as f32;

        // Validate output directory exists
        let path = std::path::Path::new(output_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(PluginError::ExecutionFailed(
                    format!("Output directory does not exist: {}", parent.display()),
                ));
            }
        }

        // Validate extension
        match path.extension().and_then(|e| e.to_str()) {
            Some("wav") => {}
            Some(ext) => {
                return Err(PluginError::ExecutionFailed(
                    format!("Only .wav output is supported, got .{ext}"),
                ));
            }
            None => {
                return Err(PluginError::ExecutionFailed(
                    "Output path must have .wav extension".into(),
                ));
            }
        }

        // Use the TTS fallback chain (injected via runtime context in actual implementation).
        let _ = text;

        Ok(json!({
            "status": "synthesized",
            "output_path": output_path,
            "duration_ms": 0,
            "note": "TTS engine integration pending -- tool contract defined"
        }))
    }
}
```

#### VS3.3.5 LatencyBenchmark Suite

Benchmark harness measuring end-to-end voice pipeline latency.

**File:** `tests/voice/bench_latency.rs`

```rust
//! Voice pipeline latency benchmarks.
//!
//! Measures:
//! - VAD detection latency (speech-end to silence signal)
//! - STT completion latency (silence signal to transcription)
//! - TTS first-byte latency (text-ready to first audio sample)
//! - End-to-end latency (speech-end to first response audio byte)

use std::time::{Duration, Instant};

/// Latency measurement result.
#[derive(Debug, Clone)]
pub struct LatencyResult {
    pub metric: String,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub samples: usize,
}

impl LatencyResult {
    pub fn from_samples(metric: &str, mut durations: Vec<Duration>) -> Self {
        durations.sort();
        let n = durations.len();
        let sum: Duration = durations.iter().sum();

        Self {
            metric: metric.to_string(),
            p50: durations[n / 2],
            p95: durations[(n as f64 * 0.95) as usize],
            p99: durations[(n as f64 * 0.99) as usize],
            mean: sum / n as u32,
            min: durations[0],
            max: durations[n - 1],
            samples: n,
        }
    }

    pub fn passes_target(&self, target: Duration) -> bool {
        self.p95 <= target
    }
}

/// Latency targets from voice_development.md.
pub struct LatencyTargets;

impl LatencyTargets {
    pub const VAD_DETECTION: Duration = Duration::from_millis(300);
    pub const STT_COMPLETION: Duration = Duration::from_millis(500);
    pub const TTS_FIRST_BYTE: Duration = Duration::from_millis(200);
    pub const END_TO_END: Duration = Duration::from_millis(3000);
    pub const WAKE_WORD: Duration = Duration::from_millis(500);
    pub const INTERRUPTION: Duration = Duration::from_millis(50);
}

/// Run a latency benchmark with N iterations of a closure that returns a Duration.
pub async fn bench_latency<F, Fut>(
    metric: &str,
    iterations: usize,
    f: F,
) -> LatencyResult
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Duration>,
{
    let mut durations = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let d = f().await;
        durations.push(d);
    }
    LatencyResult::from_samples(metric, durations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_result_calculation() {
        let durations: Vec<Duration> = (1..=100)
            .map(|ms| Duration::from_millis(ms))
            .collect();

        let result = LatencyResult::from_samples("test", durations);
        assert_eq!(result.samples, 100);
        assert_eq!(result.min, Duration::from_millis(1));
        assert_eq!(result.max, Duration::from_millis(100));
        assert!(result.p50 <= Duration::from_millis(55));
        assert!(result.p95 <= Duration::from_millis(100));
    }

    #[test]
    fn test_latency_targets_defined() {
        assert_eq!(LatencyTargets::VAD_DETECTION, Duration::from_millis(300));
        assert_eq!(LatencyTargets::STT_COMPLETION, Duration::from_millis(500));
        assert_eq!(LatencyTargets::TTS_FIRST_BYTE, Duration::from_millis(200));
        assert_eq!(LatencyTargets::END_TO_END, Duration::from_millis(3000));
    }
}
```

#### VS3.3.6 WER (Word Error Rate) Test Harness

**File:** `tests/voice/bench_wer.rs`

```rust
//! Word Error Rate (WER) benchmark harness.
//!
//! Compares STT transcriptions against reference text to measure accuracy.

/// Calculate Word Error Rate between reference and hypothesis.
///
/// WER = (Substitutions + Deletions + Insertions) / Reference word count
pub fn word_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let ref_words: Vec<&str> = reference.split_whitespace().collect();
    let hyp_words: Vec<&str> = hypothesis.split_whitespace().collect();

    let m = ref_words.len();
    let n = hyp_words.len();

    if m == 0 {
        return if n == 0 { 0.0 } else { 1.0 };
    }

    // Levenshtein distance at word level
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if ref_words[i - 1].eq_ignore_ascii_case(hyp_words[j - 1]) {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n] as f64 / m as f64
}

/// A test case for WER benchmarking.
#[derive(Debug, Clone)]
pub struct WerTestCase {
    pub id: String,
    pub audio_path: String,
    pub reference_text: String,
    pub language: String,
}

/// WER benchmark result.
#[derive(Debug, Clone)]
pub struct WerResult {
    pub test_case_id: String,
    pub reference: String,
    pub hypothesis: String,
    pub wer: f64,
}

/// Aggregate WER across multiple test cases.
pub fn aggregate_wer(results: &[WerResult]) -> f64 {
    if results.is_empty() { return 0.0; }
    let total_wer: f64 = results.iter().map(|r| r.wer).sum();
    total_wer / results.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wer_identical() {
        assert_eq!(word_error_rate("hello world", "hello world"), 0.0);
    }

    #[test]
    fn test_wer_one_substitution() {
        let wer = word_error_rate("the cat sat", "the dog sat");
        assert!((wer - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_wer_all_wrong() {
        let wer = word_error_rate("hello world", "foo bar");
        assert!((wer - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_wer_deletion() {
        let wer = word_error_rate("a b c", "a c");
        assert!((wer - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_wer_insertion() {
        let wer = word_error_rate("a c", "a b c");
        assert!((wer - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_wer_empty_reference() {
        assert_eq!(word_error_rate("", ""), 0.0);
        assert_eq!(word_error_rate("", "hello"), 1.0);
    }
}
```

#### VS3.3.7 CPU Profiling Harness

**File:** `tests/voice/bench_cpu.rs`

```rust
//! CPU profiling harness for voice pipeline.
//!
//! Ensures:
//! - Wake word detection < 2% CPU
//! - Full pipeline < 10% CPU
//! - Measures via /proc/stat sampling on Linux or platform-equivalent.

use std::time::{Duration, Instant};

/// CPU usage measurement for a code section.
#[derive(Debug, Clone)]
pub struct CpuUsageResult {
    pub label: String,
    pub wall_time: Duration,
    pub cpu_time: Duration,
    pub cpu_percent: f64,
}

/// Measure CPU usage of an async function over a sampling period.
///
/// On Linux, reads /proc/self/stat before and after.
/// On other platforms, uses wall-clock time as an approximation.
pub async fn measure_cpu_usage<F, Fut>(
    label: &str,
    duration: Duration,
    f: F,
) -> CpuUsageResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let cpu_before = process_cpu_time();
    let wall_start = Instant::now();

    // Run the function with a timeout
    let _ = tokio::time::timeout(duration, f()).await;

    let wall_time = wall_start.elapsed();
    let cpu_after = process_cpu_time();
    let cpu_time = cpu_after.saturating_sub(cpu_before);

    let cpu_percent = if wall_time.as_nanos() > 0 {
        (cpu_time.as_nanos() as f64 / wall_time.as_nanos() as f64) * 100.0
    } else {
        0.0
    };

    CpuUsageResult {
        label: label.to_string(),
        wall_time,
        cpu_time,
        cpu_percent,
    }
}

/// Read process CPU time from /proc/self/stat (Linux) or fallback.
fn process_cpu_time() -> Duration {
    #[cfg(target_os = "linux")]
    {
        if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
            let fields: Vec<&str> = stat.split_whitespace().collect();
            if fields.len() > 14 {
                let utime: u64 = fields[13].parse().unwrap_or(0);
                let stime: u64 = fields[14].parse().unwrap_or(0);
                let ticks = utime + stime;
                let clock_ticks_per_sec = 100u64; // sysconf(_SC_CLK_TCK), typically 100
                return Duration::from_millis(ticks * 1000 / clock_ticks_per_sec);
            }
        }
        Duration::ZERO
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Fallback: no precise CPU time on non-Linux
        Duration::ZERO
    }
}

/// CPU budget targets.
pub struct CpuBudgets;

impl CpuBudgets {
    /// Wake word detection should use < 2% CPU.
    pub const WAKE_WORD_MAX_PERCENT: f64 = 2.0;
    /// Full voice pipeline should use < 10% CPU.
    pub const FULL_PIPELINE_MAX_PERCENT: f64 = 10.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_budgets_defined() {
        assert!(CpuBudgets::WAKE_WORD_MAX_PERCENT < CpuBudgets::FULL_PIPELINE_MAX_PERCENT);
    }
}
```

#### VS3.3.8 Voice Permission Integration

Restrict voice-triggered tool execution by permission level. Ensures voice commands respect the same permission model as text commands.

**File:** `clawft-plugin/src/voice/permissions.rs`

```rust
use serde::{Deserialize, Serialize};

/// Permission level required for voice-triggered actions.
///
/// Maps to the existing clawft permission levels.
/// Voice commands default to the same level as text commands,
/// but destructive operations may require elevated confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoicePermissionLevel {
    /// Read-only operations (queries, searches, status checks).
    ReadOnly = 0,
    /// Standard operations (file read, web search, memory access).
    Standard = 1,
    /// Write operations (file write, config changes).
    Write = 2,
    /// Destructive operations (file delete, process kill, git push).
    /// Requires voice confirmation prompt.
    Destructive = 3,
    /// Shell execution. Blocked by default for voice.
    Shell = 4,
}

/// Check if a voice-triggered tool call is permitted at the given level.
pub fn is_voice_permitted(
    tool_level: VoicePermissionLevel,
    user_level: VoicePermissionLevel,
    voice_confirmation_given: bool,
) -> VoicePermissionResult {
    if tool_level > user_level {
        return VoicePermissionResult::Denied {
            reason: format!(
                "Tool requires permission level {:?} but user has {:?}",
                tool_level, user_level
            ),
        };
    }

    if tool_level >= VoicePermissionLevel::Destructive && !voice_confirmation_given {
        return VoicePermissionResult::RequiresConfirmation {
            prompt: "This action is destructive. Please confirm by saying 'yes' or 'confirm'.".to_string(),
        };
    }

    VoicePermissionResult::Allowed
}

#[derive(Debug, Clone)]
pub enum VoicePermissionResult {
    Allowed,
    RequiresConfirmation { prompt: String },
    Denied { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_user_can_read() {
        let result = is_voice_permitted(
            VoicePermissionLevel::ReadOnly,
            VoicePermissionLevel::Standard,
            false,
        );
        assert!(matches!(result, VoicePermissionResult::Allowed));
    }

    #[test]
    fn test_destructive_requires_confirmation() {
        let result = is_voice_permitted(
            VoicePermissionLevel::Destructive,
            VoicePermissionLevel::Destructive,
            false,
        );
        assert!(matches!(result, VoicePermissionResult::RequiresConfirmation { .. }));
    }

    #[test]
    fn test_destructive_allowed_with_confirmation() {
        let result = is_voice_permitted(
            VoicePermissionLevel::Destructive,
            VoicePermissionLevel::Destructive,
            true,
        );
        assert!(matches!(result, VoicePermissionResult::Allowed));
    }

    #[test]
    fn test_insufficient_level_denied() {
        let result = is_voice_permitted(
            VoicePermissionLevel::Shell,
            VoicePermissionLevel::Write,
            true,
        );
        assert!(matches!(result, VoicePermissionResult::Denied { .. }));
    }
}
```

#### VS3.3.9 End-to-End Voice Tests (Playwright + Audio Simulation)

**File:** `tests/voice/e2e_voice.spec.ts`

```typescript
import { test, expect } from "@playwright/test";

/**
 * End-to-end voice tests using Playwright with audio simulation.
 *
 * These tests use Playwright's browser context to:
 * 1. Mock getUserMedia with synthetic audio streams
 * 2. Verify UI voice components render and respond
 * 3. Test WebSocket voice event handling
 * 4. Validate Talk Mode overlay behavior
 */

// Helper: create a synthetic audio MediaStream using AudioContext + OscillatorNode.
async function createSyntheticAudioStream(page: any): Promise<void> {
  await page.addInitScript(() => {
    const originalGetUserMedia = navigator.mediaDevices.getUserMedia.bind(
      navigator.mediaDevices,
    );

    navigator.mediaDevices.getUserMedia = async (constraints: any) => {
      if (constraints?.audio) {
        // Create a synthetic audio stream with a 440Hz tone
        const ctx = new AudioContext();
        const oscillator = ctx.createOscillator();
        oscillator.frequency.value = 440;
        const dest = ctx.createMediaStreamDestination();
        oscillator.connect(dest);
        oscillator.start();
        return dest.stream;
      }
      return originalGetUserMedia(constraints);
    };
  });
}

test.describe("Voice Status Bar", () => {
  test("renders idle state by default", async ({ page }) => {
    await page.goto("/");
    const statusBar = page.locator("[data-testid='voice-status-bar']");
    await expect(statusBar).toBeVisible();
    await expect(statusBar).toContainText("Voice Off");
  });

  test("transitions to listening on voice:status event", async ({ page }) => {
    await page.goto("/");

    // Simulate WebSocket voice:status event
    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent("ws:voice:status", {
          detail: { state: "listening" },
        }),
      );
    });

    const statusBar = page.locator("[data-testid='voice-status-bar']");
    await expect(statusBar).toContainText("Listening");
  });
});

test.describe("Push to Talk Button", () => {
  test("activates on pointer down and deactivates on pointer up", async ({
    page,
  }) => {
    await createSyntheticAudioStream(page);
    await page.goto("/voice");

    const pttButton = page.locator("[data-testid='ptt-button']");
    await expect(pttButton).toBeVisible();
    await expect(pttButton).toContainText("Hold to talk");

    await pttButton.dispatchEvent("pointerdown");
    await expect(pttButton).toContainText("Release to send");

    await pttButton.dispatchEvent("pointerup");
    await expect(pttButton).toContainText("Hold to talk");
  });
});

test.describe("Talk Mode Overlay", () => {
  test("shows transcript entries from WebSocket events", async ({ page }) => {
    await page.goto("/");

    // Open Talk Mode
    await page.click("[data-testid='start-talk-mode']");

    const overlay = page.locator("[data-testid='talk-overlay']");
    await expect(overlay).toBeVisible();

    // Simulate transcription event
    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent("ws:voice:transcription", {
          detail: { text: "Hello Weft", speaker: "user" },
        }),
      );
    });

    await expect(overlay).toContainText("Hello Weft");
  });

  test("stop button closes overlay", async ({ page }) => {
    await page.goto("/");
    await page.click("[data-testid='start-talk-mode']");

    const overlay = page.locator("[data-testid='talk-overlay']");
    await expect(overlay).toBeVisible();

    await page.click("[data-testid='stop-talk-mode']");
    await expect(overlay).not.toBeVisible();
  });
});

test.describe("Voice Settings", () => {
  test("renders mic selection and voice options", async ({ page }) => {
    await page.goto("/settings/voice");

    await expect(page.locator("text=Voice Settings")).toBeVisible();
    await expect(page.locator("text=Microphone")).toBeVisible();
    await expect(page.locator("text=Voice")).toBeVisible();
    await expect(page.locator("text=Language")).toBeVisible();
    await expect(page.locator("text=Wake Word")).toBeVisible();
  });

  test("toggle wake word switch", async ({ page }) => {
    await page.goto("/settings/voice");

    const wakeSwitch = page.locator("[data-testid='wake-word-switch']");
    await wakeSwitch.click();

    // Sensitivity slider should appear
    await expect(page.locator("text=Wake Word Sensitivity")).toBeVisible();
  });
});

test.describe("Audio Waveform", () => {
  test("canvas element renders when active", async ({ page }) => {
    await createSyntheticAudioStream(page);
    await page.goto("/voice");

    const canvas = page.locator("[data-testid='audio-waveform'] canvas");
    await expect(canvas).toBeVisible();
  });
});
```

---

## 4. Concurrency Plan

VS3 sub-phases can be partially parallelized:

| Week | Parallel Track A | Parallel Track B |
|------|-----------------|-----------------|
| 7 | VS3.1.1-VS3.1.5 (UI components) | VS3.1.6-VS3.1.8 (Backend WS events + Tauri) |
| 8 | VS3.2.1-VS3.2.2 (Cloud providers) | VS3.2.3-VS3.2.5 (Fallback chain + diarizer + logging) |
| 9 | VS3.3.1-VS3.3.4 (Personality + commands + tools) | VS3.3.5-VS3.3.9 (Benchmarks + permissions + E2E) |

**Track A** is frontend/TypeScript focused. **Track B** is backend/Rust focused. Both tracks can proceed independently within each week.

---

## 5. Tests Required

### UI Component Tests (Vitest)

| Test | Description |
|------|-------------|
| `test_voice_status_bar_renders_idle` | VoiceStatusBar renders "Voice Off" with MicOff icon in idle state. |
| `test_voice_status_bar_transitions` | VoiceStatusBar updates on `voice:status` WebSocket event for all 4 states. |
| `test_talk_overlay_shows_transcript` | TalkModeOverlay displays transcript entries from `voice:transcription` events. |
| `test_talk_overlay_partial_text` | TalkModeOverlay shows italic partial text from `voice:partial_transcription`. |
| `test_talk_overlay_mute_toggle` | Mute button sends `voice:mute` WebSocket command. |
| `test_ptt_button_capture_cycle` | PushToTalkButton requests getUserMedia on pointerDown, sends audio on pointerUp. |
| `test_voice_settings_device_list` | VoiceSettings enumerates audio devices and populates Select components. |
| `test_voice_settings_save` | Changing a setting triggers PATCH `/api/voice/config`. |
| `test_waveform_canvas_renders` | AudioWaveform creates AudioContext and renders to canvas when stream is active. |

### Rust Unit Tests

| Test | Description |
|------|-------------|
| `test_whisper_stt_builds_multipart` | WhisperSttProvider constructs correct multipart form for Whisper API. |
| `test_openai_tts_request_body` | OpenAiTtsProvider sends correct JSON body to TTS endpoint. |
| `test_elevenlabs_request_headers` | ElevenLabsTtsProvider sets `xi-api-key` header. |
| `test_stt_fallback_local_success` | SttFallbackChain returns local result when confidence >= threshold. |
| `test_stt_fallback_low_confidence_uses_cloud` | SttFallbackChain falls back to cloud when local confidence < threshold. |
| `test_stt_fallback_local_error_uses_cloud` | SttFallbackChain uses cloud when local engine returns error. |
| `test_stt_fallback_both_fail` | SttFallbackChain returns error when both local and cloud fail. |
| `test_tts_fallback_local_success` | TtsFallbackChain returns local result on success. |
| `test_tts_fallback_uses_cloud_on_local_error` | TtsFallbackChain falls back to cloud when local fails. |
| `test_voice_command_exact_match` | VoiceCommandRegistry matches "hey weft check my email" exactly. |
| `test_voice_command_fuzzy_match` | VoiceCommandRegistry matches with Levenshtein distance <= 2. |
| `test_voice_command_no_match` | VoiceCommandRegistry returns None for unregistered phrases. |
| `test_transcript_logger_append_and_read` | TranscriptLogger writes JSONL entries and reads them back. |
| `test_voice_personality_default` | VoicePersonality::default() returns "default" voice, "local" provider, speed 1.0. |
| `test_voice_personality_serde_roundtrip` | VoicePersonality serializes to JSON and deserializes back. |
| `test_voice_permission_allowed` | Standard user can execute ReadOnly tool. |
| `test_voice_permission_requires_confirmation` | Destructive tool without confirmation returns RequiresConfirmation. |
| `test_voice_permission_denied` | Shell tool denied for Write-level user. |
| `test_wer_calculation` | WER of "the cat sat" vs "the dog sat" equals 1/3. |
| `test_wer_identical` | WER of identical strings equals 0.0. |
| `test_latency_result_percentiles` | LatencyResult calculates p50/p95/p99 from sample data. |
| `test_audio_transcribe_tool_schema` | audio_transcribe tool returns valid JSON Schema for parameters. |
| `test_audio_synthesize_tool_schema` | audio_synthesize tool returns valid JSON Schema for parameters. |
| `test_audio_transcribe_missing_file` | audio_transcribe returns error for nonexistent file path. |
| `test_audio_synthesize_invalid_extension` | audio_synthesize rejects non-.wav output path. |

### E2E Tests (Playwright)

| Test | Description |
|------|-------------|
| `test_voice_status_bar_e2e` | Status bar visible in dashboard, transitions on WebSocket events. |
| `test_push_to_talk_e2e` | PTT button activates/deactivates mic capture. |
| `test_talk_mode_overlay_e2e` | Talk Mode opens, shows transcript, closes on stop. |
| `test_voice_settings_e2e` | Settings page renders all controls, saves changes. |
| `test_waveform_e2e` | Waveform canvas renders with simulated audio. |

---

## 6. Acceptance Criteria

- [ ] `VoiceStatusBar` component renders 4 states with correct icons and colors
- [ ] `TalkModeOverlay` shows real-time transcript with partial and final entries
- [ ] `AudioWaveform` visualizes microphone input using Web Audio API
- [ ] `VoiceSettings` panel allows mic/voice/language/wake-word configuration
- [ ] `PushToTalkButton` captures audio on hold and sends via WebSocket
- [ ] WebSocket events `voice:partial_transcription` and `voice:tts_progress` stream to UI
- [ ] Tauri voice commands (`check_mic_permission`, `list_audio_devices`) are registered
- [ ] `CloudSttProvider` trait defined with `WhisperSttProvider` implementation
- [ ] `CloudTtsProvider` trait defined with `OpenAiTtsProvider` and `ElevenLabsTtsProvider`
- [ ] `SttFallbackChain` tries local first, falls back to cloud on failure/low confidence
- [ ] `TtsFallbackChain` tries local first, falls back to cloud on failure
- [ ] `SpeakerDiarizer` struct defined with `diarize()` method signature
- [ ] `TranscriptLogger` appends JSONL entries and reads them back
- [ ] `VoicePersonality` config type supports voice_id, provider, speed, pitch, language
- [ ] `VoiceCommandRegistry` matches trigger phrases with fuzzy tolerance
- [ ] `audio_transcribe` tool accepts file path and returns transcription
- [ ] `audio_synthesize` tool accepts text and output path, produces .wav
- [ ] `LatencyResult` computes p50/p95/p99 and checks against targets
- [ ] `word_error_rate()` computes WER correctly
- [ ] CPU profiling harness measures process CPU time on Linux
- [ ] Voice permission system enforces levels and requires confirmation for destructive actions
- [ ] All Playwright E2E voice tests pass with audio simulation
- [ ] All Vitest UI component tests pass
- [ ] All Rust unit tests pass (`cargo test`)
- [ ] `cargo clippy -- -D warnings` is clean for all modified crates

---

## 7. Risk Notes

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Web Audio API unavailable in some browsers | Low | Medium | Graceful degradation: hide waveform, use fallback status text. Test in Chrome, Firefox, Safari. |
| Cloud STT/TTS API rate limits or cost overruns | Medium | Medium | Rate limit cloud calls. Log usage. Default to local-only; cloud is opt-in. |
| Speaker diarization accuracy insufficient | Medium | Low | sherpa-rs diarization is best-effort. Fall back to single-speaker mode. Mark as experimental. |
| Tauri mic permission differs across OS | Medium | Low | Platform-specific permission handling with graceful fallback to web getUserMedia. |
| Playwright audio simulation fidelity | Low | Low | Use synthetic OscillatorNode streams. Focus E2E tests on UI behavior, not STT accuracy. |
| Cloud API keys leaked in config | Low | High | API keys stored via SecretRef (from A4). Never logged. Masked in UI settings panel. |
| JSONL transcript log grows unbounded | Medium | Low | Add configurable max size / rotation. Default: 100MB per session, oldest entries pruned. |
| PTT audio encoding browser compatibility | Low | Medium | Use MediaRecorder with `audio/webm;codecs=opus` (wide support). Fallback to WAV if unavailable. |
