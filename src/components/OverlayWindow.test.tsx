import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide: vi.fn().mockResolvedValue(undefined) }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../stores/usageStore", () => ({
  useUsageStore: () => ({
    frontendState: {
      usage: {
        five_hour: { utilization: 0.73, resets_at: "2026-07-16T15:30:00Z" },
        seven_day: { utilization: 0.4, resets_at: "2026-07-23T10:00:00Z" },
      },
      config: { plan: "Max200" },
      user_name: "tester",
    },
    isOffline: () => false,
  }),
}));

import { invoke } from "@tauri-apps/api/core";
import { OverlayWindow } from "./OverlayWindow";

describe("OverlayWindow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("invokes quit_app when the quit button is clicked", () => {
    render(<OverlayWindow />);
    fireEvent.click(screen.getByLabelText("quit"));
    expect(invoke).toHaveBeenCalledWith("quit_app");
  });
});
