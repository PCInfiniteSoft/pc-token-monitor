import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../stores/usageStore", () => ({
  useUsageStore: () => ({
    frontendState: {
      usage: {
        five_hour: { utilization: 0.73, resets_at: "2026-07-16T15:30:00Z" },
        seven_day: { utilization: 0.4, resets_at: "2026-07-23T10:00:00Z" },
      },
      config: { plan: "Max200" },
      user_name: "taksin",
    },
    isOffline: () => false,
  }),
}));

import { invoke } from "@tauri-apps/api/core";
import { MacPopover } from "./MacPopover";

describe("MacPopover", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("renders both usage windows with percentages", () => {
    render(<MacPopover />);
    expect(screen.getByText("5-Hour")).toBeInTheDocument();
    expect(screen.getByText("7-Day")).toBeInTheDocument();
    expect(screen.getByText("73%")).toBeInTheDocument();
    expect(screen.getByText("40%")).toBeInTheDocument();
  });

  it("invokes open_settings when the settings button is clicked", () => {
    render(<MacPopover />);
    fireEvent.click(screen.getByLabelText("settings"));
    expect(invoke).toHaveBeenCalledWith("open_settings");
  });

  it("invokes quit_app when the quit button is clicked", () => {
    render(<MacPopover />);
    fireEvent.click(screen.getByLabelText("quit"));
    expect(invoke).toHaveBeenCalledWith("quit_app");
  });
});
