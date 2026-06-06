export type BgTheme = "light" | "dark";

export interface OverlayPalette {
  /** primary text (title, %, footer) */
  text: string;
  /** secondary/muted text (reset countdown, inactive controls) */
  muted: string;
  /** 5HR label color */
  label5: string;
  /** 7DAY label color */
  label7: string;
  /** CSS text-shadow value */
  shadow: string;
}

const DARK: OverlayPalette = {
  text: "#e8e8e8",
  muted: "#666666",
  label5: "#00d4ff",
  label7: "#ffd700",
  shadow:
    "0 1px 2px rgba(0,0,0,0.95), 0 0 4px rgba(0,0,0,0.8), 0 0 1px rgba(0,0,0,0.9)",
};

const LIGHT: OverlayPalette = {
  text: "#1a1a1a",
  muted: "#555555",
  label5: "#0077aa",
  label7: "#9a7d0a",
  shadow: "0 1px 2px rgba(255,255,255,0.95), 0 0 4px rgba(255,255,255,0.85)",
};

/** Palette for a given background theme. "light" = light background → dark text. */
export function overlayPalette(theme: BgTheme): OverlayPalette {
  return theme === "light" ? LIGHT : DARK;
}
