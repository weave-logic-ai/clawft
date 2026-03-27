import { useState, useRef, useCallback, useEffect } from "react";
import { Mic } from "lucide-react";
import { useVoiceStore } from "../../stores/voice-store";
import { sendVoiceMessage } from "../../lib/voice-chat";
import {
  createSpeechRecognition,
  hasSpeechRecognition,
  speak,
  type SpeechRecognition,
  type SpeechRecognitionEvent,
} from "../../lib/audio";
import { cn } from "../../lib/utils";

export function PushToTalk() {
  const { state, setState, settings, setTranscript, setResponse } =
    useVoiceStore();
  const [active, setActive] = useState(false);
  const [duration, setDuration] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const startTimeRef = useRef<number>(0);
  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const transcriptRef = useRef<string>("");

  const supported = hasSpeechRecognition();

  const startRecording = useCallback(() => {
    if (!supported) {
      setError("Speech recognition not supported in this browser");
      return;
    }

    setError(null);
    setActive(true);
    setState("listening");
    setTranscript("");
    transcriptRef.current = "";
    startTimeRef.current = Date.now();
    setDuration(0);

    timerRef.current = setInterval(() => {
      setDuration(Math.floor((Date.now() - startTimeRef.current) / 1000));
    }, 100);

    const recognition = createSpeechRecognition({
      lang: settings.language || "en-US",
      continuous: true,
      interimResults: true,
    });

    if (!recognition) {
      setError("Failed to create speech recognition");
      setActive(false);
      setState("idle");
      return;
    }

    recognition.onresult = (event: SpeechRecognitionEvent) => {
      let interim = "";
      let final = "";
      for (let i = event.resultIndex; i < event.results.length; i++) {
        const result = event.results[i];
        if (result.isFinal) {
          final += result[0].transcript;
        } else {
          interim += result[0].transcript;
        }
      }
      if (final) {
        transcriptRef.current += final;
      }
      setTranscript(transcriptRef.current + interim);
    };

    recognition.onerror = (event) => {
      if (event.error !== "aborted") {
        setError(`Speech error: ${event.error}`);
      }
    };

    recognition.onend = () => {
      // Recognition can end on its own (e.g. silence timeout).
      // If we're still active, the user hasn't released yet — restart.
      if (recognitionRef.current === recognition && active) {
        try {
          recognition.start();
        } catch {
          // already stopped
        }
      }
    };

    recognition.start();
    recognitionRef.current = recognition;
  }, [supported, setState, setTranscript, settings.language, active]);

  const stopRecording = useCallback(async () => {
    setActive(false);
    setState("processing");

    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    // Stop speech recognition
    if (recognitionRef.current) {
      const rec = recognitionRef.current;
      recognitionRef.current = null;
      rec.abort();
    }

    const transcript = transcriptRef.current.trim();
    if (!transcript) {
      setState("idle");
      return;
    }

    // Send transcript and wait for the agent's real response via WebSocket.
    try {
      const text = await sendVoiceMessage(transcript);
      setResponse(text);
      setState("speaking");

      // Use browser TTS to speak the response
      try {
        await speak(text, { lang: settings.language || "en-US" });
      } catch {
        // TTS not available or failed, that's ok
      }
    } catch {
      setResponse("");
    }
    setState("idle");
  }, [setState, setResponse, settings.language]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
      if (recognitionRef.current) {
        recognitionRef.current.abort();
        recognitionRef.current = null;
      }
    };
  }, []);

  const formatDuration = (seconds: number): string => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  return (
    <div className="flex flex-col items-center gap-3">
      {/* Duration indicator */}
      <div
        className={cn(
          "text-sm font-mono tabular-nums transition-opacity",
          active ? "opacity-100" : "opacity-0",
          active ? "text-green-600 dark:text-green-400" : "text-gray-400",
        )}
      >
        {formatDuration(duration)}
      </div>

      {/* Push-to-talk button */}
      <button
        onMouseDown={startRecording}
        onMouseUp={stopRecording}
        onMouseLeave={() => {
          if (active) stopRecording();
        }}
        onTouchStart={(e) => {
          e.preventDefault();
          startRecording();
        }}
        onTouchEnd={(e) => {
          e.preventDefault();
          stopRecording();
        }}
        disabled={state === "processing" || !supported}
        className={cn(
          "relative flex items-center justify-center rounded-full transition-all",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2",
          "disabled:pointer-events-none disabled:opacity-50",
          active
            ? "h-20 w-20 bg-green-500 shadow-lg shadow-green-500/30"
            : "h-16 w-16 bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600",
        )}
        aria-label={active ? "Release to stop recording" : "Hold to talk"}
      >
        <Mic
          className={cn(
            "transition-all",
            active
              ? "h-8 w-8 text-white"
              : "h-6 w-6 text-gray-600 dark:text-gray-300",
          )}
        />

        {/* Pulse ring when active */}
        {active && (
          <span className="absolute inset-0 rounded-full animate-ping bg-green-400 opacity-30" />
        )}
      </button>

      {/* Label */}
      <p
        className={cn(
          "text-xs font-medium",
          active
            ? "text-green-600 dark:text-green-400"
            : "text-gray-500 dark:text-gray-400",
        )}
      >
        {!supported
          ? "Not supported in this browser"
          : state === "processing"
            ? "Processing..."
            : active
              ? "Release to send"
              : "Hold to talk"}
      </p>

      {/* Error message */}
      {error && (
        <p className="text-xs text-red-500 dark:text-red-400 text-center max-w-48">
          {error}
        </p>
      )}
    </div>
  );
}
