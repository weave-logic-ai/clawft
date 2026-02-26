import { create } from "zustand";
import { api } from "../lib/api-client";
import type {
  ActiveDelegation,
  DelegationRule,
  DelegationHistoryEntry,
} from "../lib/types";

interface DelegationStore {
  activeDelegations: ActiveDelegation[];
  rules: DelegationRule[];
  history: DelegationHistoryEntry[];
  historyTotal: number;
  loading: boolean;
  activeTab: "active" | "rules" | "history";
  historyFilter: { session?: string; target?: string };
  setActiveTab: (tab: "active" | "rules" | "history") => void;
  setHistoryFilter: (filter: { session?: string; target?: string }) => void;
  fetchActive: () => Promise<void>;
  fetchRules: () => Promise<void>;
  upsertRule: (rule: DelegationRule) => Promise<void>;
  deleteRule: (name: string) => Promise<void>;
  fetchHistory: (offset?: number, limit?: number) => Promise<void>;
}

export const useDelegationStore = create<DelegationStore>((set, get) => ({
  activeDelegations: [],
  rules: [],
  history: [],
  historyTotal: 0,
  loading: false,
  activeTab: "active",
  historyFilter: {},
  setActiveTab: (tab) => set({ activeTab: tab }),
  setHistoryFilter: (filter) => set({ historyFilter: filter }),
  fetchActive: async () => {
    set({ loading: true });
    try {
      const delegations = await api.delegation.listActive();
      set({ activeDelegations: delegations, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  fetchRules: async () => {
    set({ loading: true });
    try {
      const rules = await api.delegation.listRules();
      set({ rules, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  upsertRule: async (rule) => {
    const updated = await api.delegation.upsertRule(rule);
    set((state) => {
      const idx = state.rules.findIndex((r) => r.name === updated.name);
      if (idx >= 0) {
        const newRules = [...state.rules];
        newRules[idx] = updated;
        return { rules: newRules };
      }
      return { rules: [...state.rules, updated] };
    });
  },
  deleteRule: async (name) => {
    await api.delegation.deleteRule(name);
    set((state) => ({
      rules: state.rules.filter((r) => r.name !== name),
    }));
  },
  fetchHistory: async (offset = 0, limit = 50) => {
    set({ loading: true });
    try {
      const { historyFilter } = get();
      const result = await api.delegation.history({
        ...historyFilter,
        offset,
        limit,
      });
      set({
        history: result.items,
        historyTotal: result.total,
        loading: false,
      });
    } catch {
      set({ loading: false });
    }
  },
}));
