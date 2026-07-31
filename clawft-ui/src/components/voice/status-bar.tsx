import { useEffect } from "react";
import { Mic, MicOff, Loader, Volume2 } from "lucide-react";
import { useVoiceStore, type VoiceState } from "../../stores/voice-store";
import { api } from "../../lib/api-client";
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

type VoiceStatusPayload = {
  state?: VoiceState;
  status?: VoiceState;
  transcript?: string | null;
  response?: string | null;
  talkModeActive?: boolean;
};

function applyVoiceStatusPayload(
  payload: VoiceStatusPayload,
  setState: (s: VoiceState) => void,
  setTranscript: (t: string) => void,
  setResponse: (t: string) => void,
  setTalkMode: (a: boolean) => void,
) {
  const next = payload.state ?? payload.status;
  if (next) setState(next);
  if (payload.transcript !== undefined) setTranscript(payload.transcript ?? "");
  if (payload.response !== undefined) setResponse(payload.response ?? "");
  if (payload.talkModeActive !== undefined) setTalkMode(payload.talkModeActive);
}

export function VoiceStatusBar() {
  const { state, talkModeActive, setTalkMode, setState, setTranscript, setResponse } =
    useVoiceStore();

  useEffect(() => {
    // Real backend path: TopicBroadcaster topic "voice:status" is enveloped
    // by the WS handler as { type: "event", topic, data } (same as canvas).
    wsClient.subscribe("voice:status");

    const off = wsClient.on("event", (raw: unknown) => {
      const msg = raw as {
        topic?: string;
        data?: VoiceStatusPayload;
      };
      if (msg.topic !== "voice:status" || !msg.data) return;
      applyVoiceStatusPayload(
        msg.data,
        setState,
        setTranscript,
        setResponse,
        setTalkMode,
      );
    });

    return () => {
      wsClient.unsubscribe("voice:status");
      off();
    };
  }, [setState, setTranscript, setResponse, setTalkMode]);

  const config = stateConfig[state];
  const Icon = config.icon;

  const toggleTalkMode = () => {
    const next = !talkModeActive;
    setTalkMode(next);
    setState(next ? "listening" : "idle");
    // Push to daemon so GET /api/voice/status and WS subscribers stay in sync
    // (WEFT-218 real broadcaster — not MSW-only).
    void api.voice
      .updatePipeline({
        talkModeActive: next,
        state: next ? "listening" : "idle",
      })
      .catch(() => {
        /* offline / MSW: local store still drives the overlay */
      });
  };

  return (
    <button
      onClick={toggleTalkMode}
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
