import { Mic, MicOff, Loader, Volume2 } from "lucide-react";
import { useVoiceStore, type VoiceState } from "../../stores/voice-store";
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

function WaveformBars({ active }: { active: boolean }) {
  return (
    <div className="flex items-end gap-1 h-12">
      {Array.from({ length: 7 }).map((_, i) => (
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
                  animationDuration: `${0.6 + Math.random() * 0.4}s`,
                }
              : undefined
          }
        />
      ))}
    </div>
  );
}

export function TalkModeOverlay() {
  const { state, transcript, response, talkModeActive, setTalkMode } =
    useVoiceStore();

  if (!talkModeActive) return null;

  const Icon = overlayIcons[state];
  const isActive = state === "listening" || state === "speaking";

  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-black/70 backdrop-blur-sm">
      {/* State icon */}
      <div className="relative mb-6">
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
        {overlayLabels[state]}
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

      {/* Response */}
      {response && (
        <div className="mb-8 max-w-lg rounded-lg bg-blue-500/10 px-6 py-3">
          <p className="text-xs font-medium text-blue-300 uppercase mb-1">
            Assistant
          </p>
          <p className="text-sm text-white">{response}</p>
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
