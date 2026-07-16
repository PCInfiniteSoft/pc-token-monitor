import { describe, it, expect, afterEach, vi } from "vitest";
import { isMacOS } from "./platform";

function setUA(ua: string) {
  vi.spyOn(navigator, "userAgent", "get").mockReturnValue(ua);
}

describe("isMacOS", () => {
  afterEach(() => vi.restoreAllMocks());

  it("is true on a macOS WebKit user agent", () => {
    setUA(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    );
    expect(isMacOS()).toBe(true);
  });

  it("is false on a Windows user agent", () => {
    setUA("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");
    expect(isMacOS()).toBe(false);
  });
});
