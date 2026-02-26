import { create } from "zustand";

interface AgentStore {
  wsConnected: boolean;
  setWsConnected: (connected: boolean) => void;
  notifications: string[];
  addNotification: (msg: string) => void;
  clearNotifications: () => void;
}

export const useAgentStore = create<AgentStore>((set) => ({
  wsConnected: false,
  setWsConnected: (connected) => set({ wsConnected: connected }),
  notifications: [],
  addNotification: (msg) =>
    set((state) => ({ notifications: [...state.notifications, msg] })),
  clearNotifications: () => set({ notifications: [] }),
}));
