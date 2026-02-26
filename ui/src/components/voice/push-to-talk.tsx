import { useState, useRef, useCallback, useEffect } from "react";
import { Mic } from "lucide-react";
import { useVoiceStore } from "../../stores/voice-store";
import { wsClient } from "../../lib/ws-client";
import { cn } from "../../lib/utils";

export function PushToTalk() {
  const { state, setState } = useVoiceStore();
  const [active, setActive] = useState(false);
  const [duration, setDuration] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const startTimeRef = useRef<number>(0);

  const startRecording = useCallback(() => {
    setActive(true);
    setState("listening");
    startTimeRef.current = Date.now();
    setDuration(0);

    timerRef.current = setInterval(() => {
      setDuration(Math.floor((Date.now() - startTimeRef.current) / 1000));
    }, 100);

    wsClient.send({ type: "voice:push_to_talk_start" });
  }, [setState]);

  const stopRecording = useCallback(() => {
    setActive(false);
    setState("processing");

    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    wsClient.send({ type: "voice:push_to_talk_stop" });

    // Simulate returning to idle after processing
    setTimeout(() => setState("idle"), 1500);
  }, [setState]);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
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
        disabled={state === "processing"}
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
        {state === "processing"
          ? "Processing..."
          : active
            ? "Release to send"
            : "Hold to talk"}
      </p>
    </div>
  );
}
