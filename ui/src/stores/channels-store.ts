import { create } from "zustand";
import { api } from "../lib/api-client";
import type { ChannelStatus } from "../lib/types";

interface ChannelsStore {
  channels: ChannelStatus[];
  loading: boolean;
  fetchChannels: () => Promise<void>;
  updateChannel: (name: string, data: Partial<ChannelStatus>) => void;
}

export const useChannelsStore = create<ChannelsStore>((set) => ({
  channels: [],
  loading: false,
  fetchChannels: async () => {
    set({ loading: true });
    try {
      const channels = await api.channels.list();
      set({ channels, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  updateChannel: (name, data) =>
    set((state) => ({
      channels: state.channels.map((ch) =>
        ch.name === name ? { ...ch, ...data } : ch,
      ),
    })),
}));
