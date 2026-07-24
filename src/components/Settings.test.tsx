import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../platform", () => ({ isMacOS: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { isMacOS } from "../platform";
import { Settings } from "./Settings";

describe("Settings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // get_state shape the component reads
    vi.mocked(invoke).mockResolvedValue({
      config: { aot_mode: "auto", aot_allowlist: ["claude.exe"] },
    });
  });

  it("shows the Windows overlay controls on non-macOS", () => {
    vi.mocked(isMacOS).mockReturnValue(false);
    render(<Settings />);
    expect(screen.getByText("Always on top")).toBeInTheDocument();
    expect(screen.getByText(/Allowed apps/)).toBeInTheDocument();
  });

  it("hides the Windows overlay controls on macOS", () => {
    vi.mocked(isMacOS).mockReturnValue(true);
    render(<Settings />);
    expect(screen.getByText("Settings")).toBeInTheDocument();
    expect(screen.queryByText("Always on top")).not.toBeInTheDocument();
    expect(screen.queryByText(/Allowed apps/)).not.toBeInTheDocument();
  });
});
