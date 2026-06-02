import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { renderHook, act } from "@testing-library/react";
import { useTauriEvents } from "./useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";

const mockState = {
  usage: {
    five_hour: { utilization: 0.5, resets_at: "2026-06-02T15:30:00Z" },
    seven_day: { utilization: 0.3, resets_at: "2026-06-09T10:00:00Z" },
    seven_day_opus_utilization: null,
    extra_usage_enabled: false,
    source: "oauth",
  },
  config: { plan: "Pro" },
};

describe("useTauriEvents", () => {
  beforeEach(() => {
    useUsageStore.getState().setFrontendState(null);
    vi.clearAllMocks();
  });

  it("calls invoke get_state on mount", async () => {
    vi.mocked(invoke).mockResolvedValue(mockState);
    vi.mocked(listen).mockResolvedValue(() => {});
    const { unmount } = renderHook(() => useTauriEvents());
    await act(async () => {});
    expect(invoke).toHaveBeenCalledWith("get_state");
    unmount();
  });

  it("updates store when usage-updated event fires", async () => {
    let capturedHandler: ((e: any) => void) | null = null;
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      capturedHandler = handler as any;
      return () => {};
    });
    vi.mocked(invoke).mockResolvedValue({ usage: null, config: { plan: "Unknown" } });

    const { unmount } = renderHook(() => useTauriEvents());
    await act(async () => {});

    act(() => capturedHandler?.({ payload: mockState }));
    expect(useUsageStore.getState().frontendState?.usage?.five_hour.utilization).toBe(0.5);
    unmount();
  });
});
