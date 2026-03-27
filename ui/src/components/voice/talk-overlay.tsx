import { useEffect, useRef, useCallback } from "react";
import { Mic, MicOff, Loader, Volume2 } from "lucide-react";
import { useVoiceStore, type VoiceState } from "../../stores/voice-store";
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
import { Button } from "../ui/button";
import { cn } from "../../lib/utils";

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
    response,
    talkModeActive,
    setTalkMode,
    setState,
    setTranscript,
    setResponse,
    settings,
  } = useVoiceStore();

  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const activeRef = useRef(false);
  const speakingRef = useRef(false);

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

      setState("processing");
      try {
        const responseText = await sendVoiceMessage(text.trim());
        setResponse(responseText);
        setState("speaking");

        // Speak the response (mic is off, no feedback loop)
        try {
          await speak(responseText, { lang: settings.language || "en-US" });
        } catch {
          // TTS not available
        }
      } catch {
        setResponse("");
      }

      // ── Resume recognition if still in talk mode ──
      speakingRef.current = false;
      if (activeRef.current) {
        setState("listening");
        setTranscript("");
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
      }
    },
    [setState, setResponse, setTranscript, settings.language],
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
            setTimeout(() => {
              if (activeRef.current && !speakingRef.current) {
                processTranscript(text);
              }
            }, 500);
          } else {
            interim += result[0].transcript;
          }
        }
        setTranscript(finalTranscript + interim);
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
    [setTranscript, setTalkMode, processTranscript],
  );

  /** Interrupt TTS and resume listening immediately. */
  const interruptSpeech = useCallback(() => {
    if (state !== "speaking") return;
    cancelSpeech();
    speakingRef.current = false;
    setResponse("");
    if (activeRef.current) {
      setState("listening");
      setTranscript("");
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
  }, [state, setState, setResponse, setTranscript, settings.language, wireRecognition]);

  // Start/stop recognition when talk mode toggles
  useEffect(() => {
    if (!talkModeActive) {
      activeRef.current = false;
      if (recognitionRef.current) {
        recognitionRef.current.abort();
        recognitionRef.current = null;
      }
      setState("idle");
      return;
    }

    if (!hasSpeechRecognition()) {
      setState("idle");
      return;
    }

    activeRef.current = true;
    speakingRef.current = false;
    setState("listening");
    setTranscript("");
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
    setTranscript,
    setResponse,
    setTalkMode,
    settings.language,
    wireRecognition,
  ]);

  if (!talkModeActive) return null;

  const Icon = overlayIcons[state];
  const isActive = state === "listening" || state === "speaking";

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

      {/* Transcript */}
      {transcript && (
        <div className="mb-4 max-w-lg rounded-lg bg-white/10 px-6 py-3">
          <p className="text-xs font-medium text-gray-400 uppercase mb-1">
            You said
          </p>
          <p className="text-sm text-white">{transcript}</p>
        </div>
      )}

      {/* Response (cleaned for readability in voice mode) */}
      {response && (
        <div className="mb-8 max-w-lg rounded-lg bg-blue-500/10 px-6 py-3">
          <p className="text-xs font-medium text-blue-300 uppercase mb-1">
            Assistant
          </p>
          <p className="text-sm text-white">{stripMarkdownForSpeech(response)}</p>
        </div>
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
