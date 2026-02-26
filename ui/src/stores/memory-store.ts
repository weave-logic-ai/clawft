import { create } from "zustand";
import { api } from "../lib/api-client";
import type { MemoryEntry } from "../lib/types";

interface MemoryStore {
  entries: MemoryEntry[];
  namespaces: string[];
  searchQuery: string;
  threshold: number;
  selectedNamespace: string;
  selectedTags: string[];
  loading: boolean;
  setSearchQuery: (query: string) => void;
  setThreshold: (threshold: number) => void;
  setSelectedNamespace: (ns: string) => void;
  setSelectedTags: (tags: string[]) => void;
  fetchEntries: () => Promise<void>;
  search: (query: string, threshold: number) => Promise<void>;
  createEntry: (data: {
    key: string;
    value: string;
    namespace: string;
    tags: string[];
  }) => Promise<void>;
  deleteEntry: (key: string) => Promise<void>;
}

export const useMemoryStore = create<MemoryStore>((set, get) => ({
  entries: [],
  namespaces: [],
  searchQuery: "",
  threshold: 0.8,
  selectedNamespace: "",
  selectedTags: [],
  loading: false,
  setSearchQuery: (query) => set({ searchQuery: query }),
  setThreshold: (threshold) => set({ threshold }),
  setSelectedNamespace: (ns) => set({ selectedNamespace: ns }),
  setSelectedTags: (tags) => set({ selectedTags: tags }),
  fetchEntries: async () => {
    set({ loading: true });
    try {
      const entries = await api.memory.list();
      const namespaces = [...new Set(entries.map((e) => e.namespace))];
      set({ entries, namespaces, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  search: async (query, threshold) => {
    set({ loading: true });
    try {
      const entries = await api.memory.search(query, threshold);
      set({ entries, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  createEntry: async (data) => {
    await api.memory.create(data);
    await get().fetchEntries();
  },
  deleteEntry: async (key) => {
    await api.memory.delete(key);
    await get().fetchEntries();
  },
}));
