import { useEffect, useRef, useCallback } from "react";
import { Mic, MicOff, Loader, Volume2 } from "lucide-react";
import { useVoiceStore, type VoiceState } from "../../stores/voice-store";
import { api } from "../../lib/api-client";
import { sendVoiceMessage } from "../../lib/voice-chat";
import {
  cancelSpeech,
  createSpeechRecognition,
  hasSpeechRecognition,
  speak,
  stripMarkdownForSpeech,
  type SpeechRecognition,
  type SpeechRecognitionEvent,
} from "../../lib/audio";
import {
  mergeTranscriptDisplay,
  parsePartialTranscriptPayload,
  VOICE_PARTIAL_TOPIC,
} from "../../lib/voice-highlight";
import { wsClient } from "../../lib/ws-client";
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";
import { PartialTranscriptView } from "./partial-transcript";
import { HighlightedSpeech } from "./highlighted-speech";

/** Best-effort push of talk-mode pipeline state to the daemon (WEFT-218). */
function pushPipeline(update: {
  state?: VoiceState;
  talkModeActive?: boolean;
  transcript?: string | null;
  response?: string | null;
}) {
  void api.voice.updatePipeline(update).catch(() => {
    /* local-only / offline — UI store remains authoritative for this tab */
  });
}

const overlayIcons: Record<VoiceState, typeof Mic> = {
  idle: MicOff,
  listening: Mic,
  processing: Loader,
  speaking: Volume2,
};

const overlayLabels: Record<VoiceState, string> = {
  idle: "Microphone Off",
  listening: "Listening...",
  processing: "Processing...",
  speaking: "Speaking...",
};

const overlayColors: Record<VoiceState, string> = {
  idle: "text-gray-400",
  listening: "text-green-400",
  processing: "text-yellow-400",
  speaking: "text-blue-400",
};

// Pre-computed waveform durations — stable across renders.
const WAVEFORM_DURATIONS = [0.72, 0.85, 0.63, 0.91, 0.78, 0.67, 0.88];

function WaveformBars({ active }: { active: boolean }) {
  return (
    <div className="flex items-end gap-1 h-12">
      {WAVEFORM_DURATIONS.map((dur, i) => (
        <div
          key={i}
          className={cn(
            "w-1.5 rounded-full bg-current transition-all",
            active ? "animate-waveform" : "h-1",
          )}
          style={
            active
              ? {
                  animationDelay: `${i * 0.1}s`,
                  animationDuration: `${dur}s`,
                }
              : undefined
          }
        />
      ))}
    </div>
  );
}

export function TalkModeOverlay() {
  const {
    state,
    transcript,
    partialTranscript,
    response,
    speakingWordIndex,
    talkModeActive,
    setTalkMode,
    setState,
    setTranscript,
    setPartialTranscript,
    setResponse,
    setSpeakingWordIndex,
    clearTranscripts,
    settings,
  } = useVoiceStore();

  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const activeRef = useRef(false);
  const speakingRef = useRef(false);
  /** Throttle partial pipeline pushes so we don't spam the daemon. */
  const lastPartialPushRef = useRef(0);

  /** Stop recognition, speak, then restart recognition. */
  const processTranscript = useCallback(
    async (text: string) => {
      if (!text.trim()) {
        setState("listening");
        return;
      }

      // ── Pause recognition to prevent feedback loop ──
      speakingRef.current = true;
      if (recognitionRef.current) {
        recognitionRef.current.abort();
        recognitionRef.current = null;
      }

      setPartialTranscript("");
      setState("processing");
      pushPipeline({
        state: "processing",
        talkModeActive: true,
        transcript: text.trim(),
      });
      try {
        const responseText = await sendVoiceMessage(text.trim());
        setResponse(responseText);
        setSpeakingWordIndex(null);
        setState("speaking");
        pushPipeline({
          state: "speaking",
          talkModeActive: true,
          response: responseText,
        });

        // Speak the response (mic is off, no feedback loop).
        // WEFT-231: wire word boundary events → highlight surface.
        try {
          await speak(responseText, {
            lang: settings.language || "en-US",
            onWord: (info) => {
              if (info.wordIndex >= 0) {
                setSpeakingWordIndex(info.wordIndex);
              }
            },
          });
        } catch {
          // TTS not available — highlight stays a no-op
        }
      } catch {
        setResponse("");
      } finally {
        setSpeakingWordIndex(null);
      }

      // ── Resume recognition if still in talk mode ──
      speakingRef.current = false;
      if (activeRef.current) {
        setState("listening");
        clearTranscripts();
        pushPipeline({
          state: "listening",
          talkModeActive: true,
          transcript: null,
          response: null,
        });
        const rec = createSpeechRecognition({
          lang: settings.language || "en-US",
          continuous: true,
          interimResults: true,
        });
        if (rec) {
          wireRecognition(rec);
          rec.start();
          recognitionRef.current = rec;
        }
      } else {
        setState("idle");
        pushPipeline({ state: "idle", talkModeActive: false });
      }
    },
    [
      setState,
      setResponse,
      setPartialTranscript,
      setSpeakingWordIndex,
      clearTranscripts,
      settings.language,
    ],
  );

  /** Wire event handlers on a SpeechRecognition instance. */
  const wireRecognition = useCallback(
    (recognition: SpeechRecognition) => {
      let finalTranscript = "";

      recognition.onresult = (event: SpeechRecognitionEvent) => {
        let interim = "";
        for (let i = event.resultIndex; i < event.results.length; i++) {
          const result = event.results[i];
          if (result.isFinal) {
            finalTranscript += result[0].transcript;
            const text = finalTranscript;
            finalTranscript = "";
            setTranscript(text);
            setPartialTranscript("");
            setTimeout(() => {
              if (activeRef.current && !speakingRef.current) {
                processTranscript(text);
              }
            }, 500);
          } else {
            interim += result[0].transcript;
          }
        }
        // WEFT-231: stream interim hypothesis into the partial surface.
        setTranscript(finalTranscript);
        setPartialTranscript(interim);

        // Throttled pipeline push so remote dashboards see partials (~4 Hz).
        if (interim) {
          const now = Date.now();
          if (now - lastPartialPushRef.current > 250) {
            lastPartialPushRef.current = now;
            const merged = mergeTranscriptDisplay(finalTranscript, interim);
            pushPipeline({
              state: "listening",
              talkModeActive: true,
              transcript: merged,
            });
          }
        }
      };

      recognition.onerror = (event) => {
        if (event.error === "not-allowed") {
          setTalkMode(false);
        }
      };

      recognition.onend = () => {
        // Only auto-restart if active AND not in the middle of speaking
        if (activeRef.current && !speakingRef.current) {
          try {
            recognition.start();
          } catch {
            // already running or stopped
          }
        }
      };
    },
    [
      setTranscript,
      setPartialTranscript,
      setTalkMode,
      processTranscript,
    ],
  );

  /** Interrupt TTS and resume listening immediately. */
  const interruptSpeech = useCallback(() => {
    if (state !== "speaking") return;
    cancelSpeech();
    speakingRef.current = false;
    setResponse("");
    setSpeakingWordIndex(null);
    if (activeRef.current) {
      setState("listening");
      clearTranscripts();
      const rec = createSpeechRecognition({
        lang: settings.language || "en-US",
        continuous: true,
        interimResults: true,
      });
      if (rec) {
        wireRecognition(rec);
        rec.start();
        recognitionRef.current = rec;
      }
    }
  }, [
    state,
    setState,
    setResponse,
    setSpeakingWordIndex,
    clearTranscripts,
    settings.language,
    wireRecognition,
  ]);

  // Start/stop recognition when talk mode toggles
  useEffect(() => {
    if (!talkModeActive) {
      activeRef.current = false;
      if (recognitionRef.current) {
        recognitionRef.current.abort();
        recognitionRef.current = null;
      }
      setState("idle");
      setSpeakingWordIndex(null);
      return;
    }

    if (!hasSpeechRecognition()) {
      setState("idle");
      return;
    }

    activeRef.current = true;
    speakingRef.current = false;
    setState("listening");
    clearTranscripts();
    setResponse("");

    const recognition = createSpeechRecognition({
      lang: settings.language || "en-US",
      continuous: true,
      interimResults: true,
    });

    if (!recognition) {
      setState("idle");
      return;
    }

    wireRecognition(recognition);
    recognition.start();
    recognitionRef.current = recognition;

    return () => {
      activeRef.current = false;
      if (recognitionRef.current) {
        recognitionRef.current.abort();
        recognitionRef.current = null;
      }
    };
  }, [
    talkModeActive,
    setState,
    setResponse,
    setSpeakingWordIndex,
    clearTranscripts,
    settings.language,
    wireRecognition,
  ]);

  // WEFT-231: subscribe to dedicated partial-transcript WS topic (remote STT).
  useEffect(() => {
    if (!talkModeActive) return;

    wsClient.subscribe(VOICE_PARTIAL_TOPIC);
    const off = wsClient.on("event", (raw: unknown) => {
      const msg = raw as { topic?: string; data?: unknown };
      if (msg.topic !== VOICE_PARTIAL_TOPIC || !msg.data) return;
      // Local mic is authoritative while recognition is running; only apply
      // remote partials when we are not the STT source (no active recognition).
      if (recognitionRef.current) return;
      const parsed = parsePartialTranscriptPayload(msg.data);
      if (!parsed) return;
      if (parsed.isFinal) {
        setTranscript(parsed.text);
        setPartialTranscript("");
      } else {
        setPartialTranscript(parsed.text);
      }
    });

    return () => {
      wsClient.unsubscribe(VOICE_PARTIAL_TOPIC);
      off();
    };
  }, [talkModeActive, setTranscript, setPartialTranscript]);

  if (!talkModeActive) return null;

  const Icon = overlayIcons[state];
  const isActive = state === "listening" || state === "speaking";
  const cleanResponse = response ? stripMarkdownForSpeech(response) : "";

  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-black/70 backdrop-blur-sm">
      {/* Browser support warning */}
      {!hasSpeechRecognition() && (
        <div className="mb-6 rounded-lg bg-yellow-500/20 px-6 py-3">
          <p className="text-sm text-yellow-300">
            Speech recognition is not supported in this browser. Try Chrome or
            Edge.
          </p>
        </div>
      )}

      {/* State icon — tap to interrupt when speaking */}
      <div
        className={cn("relative mb-6", state === "speaking" && "cursor-pointer")}
        onClick={interruptSpeech}
        role={state === "speaking" ? "button" : undefined}
        aria-label={state === "speaking" ? "Tap to interrupt" : undefined}
      >
        <div
          className={cn(
            "rounded-full p-8 transition-all",
            state === "listening" && "bg-green-500/20",
            state === "processing" && "bg-yellow-500/20",
            state === "speaking" && "bg-blue-500/20",
            state === "idle" && "bg-gray-500/20",
          )}
        >
          <Icon
            className={cn(
              "h-16 w-16 transition-colors",
              overlayColors[state],
              state === "processing" && "animate-spin",
            )}
          />
        </div>
        {isActive && (
          <span
            className={cn(
              "absolute inset-0 rounded-full animate-ping opacity-20",
              state === "listening" ? "bg-green-400" : "bg-blue-400",
            )}
          />
        )}
      </div>

      {/* State label */}
      <p className="mb-4 text-lg font-medium text-white">
        {state === "speaking" ? "Speaking... tap to interrupt" : overlayLabels[state]}
      </p>

      {/* Waveform */}
      <div className={cn("mb-8", overlayColors[state])}>
        <WaveformBars active={isActive} />
      </div>

      {/* Partial / final transcript stream (WEFT-231) */}
      <PartialTranscriptView
        finalText={transcript}
        interimText={partialTranscript}
      />

      {/* Response with TTS word highlight surface (WEFT-231) */}
      {cleanResponse && (
        <HighlightedSpeech
          text={cleanResponse}
          activeWordIndex={state === "speaking" ? speakingWordIndex : null}
        />
      )}

      {/* End button */}
      <Button
        variant="outline"
        size="lg"
        onClick={() => setTalkMode(false)}
        className="border-white/30 text-white hover:bg-white/10 dark:border-white/30 dark:text-white dark:hover:bg-white/10"
      >
        End Talk Mode
      </Button>
    </div>
  );
}
