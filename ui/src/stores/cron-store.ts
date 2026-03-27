import { create } from "zustand";
import { api } from "../lib/api-client";
import type { CronJob } from "../lib/types";

interface CronStore {
  jobs: CronJob[];
  loading: boolean;
  fetchJobs: () => Promise<void>;
  createJob: (job: Omit<CronJob, "id" | "status" | "last_run" | "next_run">) => Promise<void>;
  updateJob: (id: string, data: Partial<CronJob>) => Promise<void>;
  deleteJob: (id: string) => Promise<void>;
  runNow: (id: string) => Promise<void>;
  toggleJob: (id: string, enabled: boolean) => Promise<void>;
}

export const useCronStore = create<CronStore>((set, get) => ({
  jobs: [],
  loading: false,
  fetchJobs: async () => {
    set({ loading: true });
    try {
      const jobs = await api.cron.list();
      set({ jobs, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  createJob: async (job) => {
    await api.cron.create(job);
    await get().fetchJobs();
  },
  updateJob: async (id, data) => {
    await api.cron.update(id, data);
    await get().fetchJobs();
  },
  deleteJob: async (id) => {
    await api.cron.delete(id);
    await get().fetchJobs();
  },
  runNow: async (id) => {
    await api.cron.runNow(id);
    await get().fetchJobs();
  },
  toggleJob: async (id, enabled) => {
    await api.cron.update(id, { enabled });
    await get().fetchJobs();
  },
}));
