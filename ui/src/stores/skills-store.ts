import { create } from "zustand";
import { api } from "../lib/api-client";
import type { SkillData, RegistrySkill } from "../lib/types";

interface SkillsStore {
  skills: SkillData[];
  registryResults: RegistrySkill[];
  loading: boolean;
  searchQuery: string;
  registryOpen: boolean;
  setSearchQuery: (query: string) => void;
  setRegistryOpen: (open: boolean) => void;
  fetchSkills: () => Promise<void>;
  searchRegistry: (query: string) => Promise<void>;
  installSkill: (id: string) => Promise<void>;
  uninstallSkill: (name: string) => Promise<void>;
}

export const useSkillsStore = create<SkillsStore>((set, get) => ({
  skills: [],
  registryResults: [],
  loading: false,
  searchQuery: "",
  registryOpen: false,
  setSearchQuery: (query) => set({ searchQuery: query }),
  setRegistryOpen: (open) => set({ registryOpen: open }),
  fetchSkills: async () => {
    set({ loading: true });
    try {
      const skills = await api.skills.list();
      set({ skills, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  searchRegistry: async (query) => {
    set({ loading: true });
    try {
      const registryResults = await api.skills.searchRegistry(query);
      set({ registryResults, loading: false });
    } catch {
      set({ loading: false });
    }
  },
  installSkill: async (id) => {
    await api.skills.install(id);
    await get().fetchSkills();
  },
  uninstallSkill: async (name) => {
    await api.skills.uninstall(name);
    await get().fetchSkills();
  },
}));
