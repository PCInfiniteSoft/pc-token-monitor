import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { PlanBadge } from "./PlanBadge";

describe("PlanBadge", () => {
  it("shows PRO for Pro plan", () => {
    render(<PlanBadge plan="Pro" offline={false} />);
    expect(screen.getByText("[PRO]")).toBeInTheDocument();
  });

  it("shows MAX 50 for Max50 plan", () => {
    render(<PlanBadge plan="Max50" offline={false} />);
    expect(screen.getByText("[MAX 50]")).toBeInTheDocument();
  });

  it("shows MAX 200 for Max200 plan", () => {
    render(<PlanBadge plan="Max200" offline={false} />);
    expect(screen.getByText("[MAX 200]")).toBeInTheDocument();
  });

  it("shows OFFLINE badge when offline", () => {
    render(<PlanBadge plan="Pro" offline={true} />);
    expect(screen.getByText("[OFFLINE]")).toBeInTheDocument();
  });
});
