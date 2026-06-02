import { create } from "zustand";
import type { FrontendState } from "../types";

interface UsageStore {
  frontendState: FrontendState | null;
  setFrontendState: (state: FrontendState | null) => void;
  dominantPercent: () => number;
  isOffline: () => boolean;
}

export const useUsageStore = create<UsageStore>((set, get) => ({
  frontendState: null,

  setFrontendState: (state) => set({ frontendState: state }),

  dominantPercent: () => {
    const usage = get().frontendState?.usage;
    if (!usage) return 0;
    return Math.min(
      100,
      Math.round(Math.max(usage.five_hour.utilization, usage.seven_day.utilization) * 100)
    );
  },

  isOffline: () => get().frontendState?.usage?.source === "jsonl_fallback",
}));
