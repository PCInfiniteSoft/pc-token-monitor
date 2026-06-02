import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { UsageBar } from "./UsageBar";

describe("UsageBar", () => {
  const baseProps = {
    label: "5HR",
    utilization: 0.73,
    resetsAt: "2026-06-02T15:30:00Z",
    labelColor: "#00d4ff",
  };

  it("renders label", () => {
    render(<UsageBar {...baseProps} />);
    expect(screen.getByText("5HR")).toBeInTheDocument();
  });

  it("renders percent text", () => {
    render(<UsageBar {...baseProps} />);
    expect(screen.getByText("73%")).toBeInTheDocument();
  });

  it("clamps utilization at 100%", () => {
    render(<UsageBar {...baseProps} utilization={1.5} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("shows Limit reached at 100%", () => {
    render(<UsageBar {...baseProps} utilization={1.0} />);
    expect(screen.getByText(/limit reached/i)).toBeInTheDocument();
  });

  it("shows progress bar with correct aria-valuenow", () => {
    render(<UsageBar {...baseProps} />);
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "73");
  });
});
