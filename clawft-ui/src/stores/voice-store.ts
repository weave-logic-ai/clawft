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
  /** Committed / final STT text for the current utterance. */
  transcript: string;
  /**
   * Live interim (non-final) STT hypothesis (WEFT-231).
   * Empty when no partial is streaming.
   */
  partialTranscript: string;
  response: string;
  /**
   * Active TTS word index for highlight surface (WEFT-231).
   * `null` when not speaking or when timings/boundaries unavailable.
   */
  speakingWordIndex: number | null;
  talkModeActive: boolean;
  settings: VoiceSettings;
  setState: (state: VoiceState) => void;
  setTranscript: (text: string) => void;
  setPartialTranscript: (text: string) => void;
  setResponse: (text: string) => void;
  setSpeakingWordIndex: (index: number | null) => void;
  setTalkMode: (active: boolean) => void;
  updateSettings: (settings: Partial<VoiceSettings>) => void;
  /** Clear STT partial + final for a fresh listen cycle. */
  clearTranscripts: () => void;
}

export const useVoiceStore = create<VoiceStore>((set) => ({
  state: "idle",
  transcript: "",
  partialTranscript: "",
  response: "",
  speakingWordIndex: null,
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
  setPartialTranscript: (text) => set({ partialTranscript: text }),
  setResponse: (text) => set({ response: text, speakingWordIndex: null }),
  setSpeakingWordIndex: (index) => set({ speakingWordIndex: index }),
  setTalkMode: (active) => set({ talkModeActive: active }),
  updateSettings: (partial) =>
    set((prev) => ({ settings: { ...prev.settings, ...partial } })),
  clearTranscripts: () =>
    set({ transcript: "", partialTranscript: "", speakingWordIndex: null }),
}));
