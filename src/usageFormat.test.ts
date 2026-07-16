import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { formatCountdown } from "./usageFormat";

describe("formatCountdown", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-16T00:00:00Z"));
  });
  afterEach(() => vi.useRealTimers());

  it("shows days and hours when more than a day away", () => {
    expect(formatCountdown("2026-07-18T05:00:00Z")).toBe("2d 5h");
  });

  it("shows hours and minutes when less than a day away", () => {
    expect(formatCountdown("2026-07-16T02:30:00Z")).toBe("2h 30m");
  });

  it("shows minutes only when less than an hour away", () => {
    expect(formatCountdown("2026-07-16T00:45:00Z")).toBe("45m");
  });

  it("shows resetting when already elapsed", () => {
    expect(formatCountdown("2026-07-15T23:00:00Z")).toBe("resetting...");
  });
});
