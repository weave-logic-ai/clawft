import { useEffect } from "react";
import { Mic, MicOff, Loader, Volume2 } from "lucide-react";
import { useVoiceStore, type VoiceState } from "../../stores/voice-store";
import { wsClient } from "../../lib/ws-client";
import { cn } from "../../lib/utils";

const stateConfig: Record<
  VoiceState,
  { icon: typeof Mic; color: string; pulse: boolean; label: string }
> = {
  idle: { icon: MicOff, color: "text-gray-400", pulse: false, label: "Voice Off" },
  listening: { icon: Mic, color: "text-green-500", pulse: true, label: "Listening" },
  processing: { icon: Loader, color: "text-yellow-500", pulse: false, label: "Processing" },
  speaking: { icon: Volume2, color: "text-blue-500", pulse: true, label: "Speaking" },
};

export function VoiceStatusBar() {
  const { state, talkModeActive, setTalkMode, setState, setTranscript, setResponse } =
    useVoiceStore();

  useEffect(() => {
    wsClient.subscribe("voice:status");

    const off = wsClient.on("voice:status", (data) => {
      const payload = data as {
        state?: VoiceState;
        transcript?: string;
        response?: string;
        talkModeActive?: boolean;
      };
      if (payload.state) setState(payload.state);
      if (payload.transcript !== undefined) setTranscript(payload.transcript);
      if (payload.response !== undefined) setResponse(payload.response);
      if (payload.talkModeActive !== undefined) setTalkMode(payload.talkModeActive);
    });

    return () => {
      wsClient.unsubscribe("voice:status");
      off();
    };
  }, [setState, setTranscript, setResponse, setTalkMode]);

  const config = stateConfig[state];
  const Icon = config.icon;

  return (
    <button
      onClick={() => setTalkMode(!talkModeActive)}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium transition-colors",
        "border border-gray-200 dark:border-gray-600",
        "hover:bg-gray-100 dark:hover:bg-gray-700",
        talkModeActive && "bg-blue-50 border-blue-300 dark:bg-blue-900/30 dark:border-blue-600",
      )}
      title={talkModeActive ? "End Talk Mode" : "Start Talk Mode"}
    >
      <span className="relative inline-flex">
        <Icon className={cn("h-3.5 w-3.5", config.color)} />
        {config.pulse && (
          <span
            className={cn(
              "absolute inset-0 rounded-full animate-ping opacity-40",
              state === "listening" ? "bg-green-400" : "bg-blue-400",
            )}
          />
        )}
      </span>
      <span className="text-gray-600 dark:text-gray-400">{config.label}</span>
    </button>
  );
}
