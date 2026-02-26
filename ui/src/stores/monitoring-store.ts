import { create } from "zustand";
import { api } from "../lib/api-client";
import type {
  TokenUsageSummary,
  CostBreakdown,
  PipelineRun,
} from "../lib/types";

interface MonitoringStore {
  tokenUsage: TokenUsageSummary | null;
  costs: CostBreakdown | null;
  pipelineRuns: PipelineRun[];
  loading: boolean;
  fetchTokenUsage: () => Promise<void>;
  fetchCosts: () => Promise<void>;
  fetchPipelineRuns: () => Promise<void>;
  fetchAll: () => Promise<void>;
}

export const useMonitoringStore = create<MonitoringStore>((set) => ({
  tokenUsage: null,
  costs: null,
  pipelineRuns: [],
  loading: false,
  fetchTokenUsage: async () => {
    set({ loading: true });
    try {
      const data = await api.monitoring.tokenUsage();
      set({ tokenUsage: data, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  fetchCosts: async () => {
    set({ loading: true });
    try {
      const data = await api.monitoring.costs();
      set({ costs: data, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  fetchPipelineRuns: async () => {
    set({ loading: true });
    try {
      const data = await api.monitoring.pipelineRuns();
      set({ pipelineRuns: data, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  fetchAll: async () => {
    set({ loading: true });
    try {
      const [tokenUsage, costs, pipelineRuns] = await Promise.all([
        api.monitoring.tokenUsage(),
        api.monitoring.costs(),
        api.monitoring.pipelineRuns(),
      ]);
      set({ tokenUsage, costs, pipelineRuns, loading: false });
    } catch {
      set({ loading: false });
    }
  },
}));
