import { create } from "zustand";

export type VoiceState = "idle" | "listening" | "processing" | "speaking";

interface VoiceSettings {
  enabled: boolean;
  wakeWordEnabled: boolean;
  language: string;
  echoCancel: boolean;
  noiseSuppression: boolean;
  pushToTalk: boolean;
}

interface VoiceStore {
  state: VoiceState;
  transcript: string;
  response: string;
  talkModeActive: boolean;
  settings: VoiceSettings;
  setState: (state: VoiceState) => void;
  setTranscript: (text: string) => void;
  setResponse: (text: string) => void;
  setTalkMode: (active: boolean) => void;
  updateSettings: (settings: Partial<VoiceSettings>) => void;
}

export const useVoiceStore = create<VoiceStore>((set) => ({
  state: "idle",
  transcript: "",
  response: "",
  talkModeActive: false,
  settings: {
    enabled: false,
    wakeWordEnabled: false,
    language: "en",
    echoCancel: true,
    noiseSuppression: true,
    pushToTalk: false,
  },
  setState: (state) => set({ state }),
  setTranscript: (text) => set({ transcript: text }),
  setResponse: (text) => set({ response: text }),
  setTalkMode: (active) => set({ talkModeActive: active }),
  updateSettings: (partial) =>
    set((prev) => ({ settings: { ...prev.settings, ...partial } })),
}));
