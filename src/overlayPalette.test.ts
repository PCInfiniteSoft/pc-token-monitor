import { describe, it, expect } from "vitest";
import { overlayPalette } from "./overlayPalette";

describe("overlayPalette", () => {
  it("dark theme uses light text and the original label colors", () => {
    const p = overlayPalette("dark");
    expect(p.text).toBe("#e8e8e8");
    expect(p.label5).toBe("#00d4ff");
    expect(p.label7).toBe("#ffd700");
  });

  it("light theme uses dark text and darkened label colors", () => {
    const p = overlayPalette("light");
    expect(p.text).toBe("#1a1a1a");
    expect(p.label5).toBe("#0077aa");
    expect(p.label7).toBe("#9a7d0a");
  });
});
