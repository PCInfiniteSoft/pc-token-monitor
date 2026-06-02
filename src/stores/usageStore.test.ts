import { describe, it, expect, beforeEach } from "vitest";
import { useUsageStore } from "./usageStore";
import type { FrontendState } from "../types";

const mockState: FrontendState = {
  usage: {
    five_hour: { utilization: 0.73, resets_at: "2026-06-02T15:30:00Z" },
    seven_day: { utilization: 0.91, resets_at: "2026-06-09T10:00:00Z" },
    seven_day_opus_utilization: null,
    extra_usage_enabled: false,
    source: "oauth",
  },
  config: { plan: "Max50" },
};

describe("usageStore", () => {
  beforeEach(() => {
    useUsageStore.getState().setFrontendState(null);
  });

  it("starts with null state", () => {
    expect(useUsageStore.getState().frontendState).toBeNull();
  });

  it("setFrontendState updates state", () => {
    useUsageStore.getState().setFrontendState(mockState);
    expect(useUsageStore.getState().frontendState?.usage?.five_hour.utilization).toBe(0.73);
  });

  it("dominantPercent returns highest utilization as integer %", () => {
    useUsageStore.getState().setFrontendState(mockState);
    expect(useUsageStore.getState().dominantPercent()).toBe(91);
  });

  it("dominantPercent returns 0 when no usage", () => {
    expect(useUsageStore.getState().dominantPercent()).toBe(0);
  });

  it("isOffline returns true when source is jsonl_fallback", () => {
    useUsageStore.getState().setFrontendState({
      ...mockState,
      usage: { ...mockState.usage!, source: "jsonl_fallback" },
    });
    expect(useUsageStore.getState().isOffline()).toBe(true);
  });
});
