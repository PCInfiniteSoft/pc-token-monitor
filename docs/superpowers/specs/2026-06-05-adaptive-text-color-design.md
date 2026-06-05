# Adaptive Text Color for the Transparent Overlay — Design

Date: 2026-06-05
Status: Approved (pending spec review)

## Problem

The overlay is a transparent, always-on-top window. Its text and bar labels
are light-colored with a dark `text-shadow` so they stay legible over most
backgrounds. Over a *light* desktop background, light text plus a dark shadow
still reads poorly. We want the text color to adapt to whatever is actually
behind the window.

CSS `mix-blend-mode` / `backdrop-filter` cannot see the OS desktop behind a
transparent OS window — they only blend within the webview document. The only
way to know the real background is to sample the screen pixels behind the
window natively.

## Goal

Flip the overlay's text palette between a "dark background" scheme (light text)
and a "light background" scheme (dark text) based on the luminance of the
desktop content directly behind the window. Whole-window, single decision (not
per-element).

## Approach (chosen): sample transparent gaps — no capture-exclusion

The overlay draws only text, thin bars, and borders; everything else is
transparent, so the composited screen pixels at those transparent spots are the
desktop behind the window. We sample a handful of layout-guaranteed-transparent
points and read them straight off the screen with GDI `GetPixel`. Because we
only read transparent spots, the sample never picks up the overlay's own
text/bars, so no `WDA_EXCLUDEFROMCAPTURE` is needed and the overlay stays
visible to screenshots / screen-share.

Tradeoff: a strongly multi-colored or gradient background behind the window may
be misjudged (one global decision). If that proves unacceptable in practice,
fall back to the capture+`WDA_EXCLUDEFROMCAPTURE` approach (sample the whole
region averaged) — recorded as the fallback, not built now.

## Components

### 1. Background sampler (Rust, `src-tauri`)

New module `bg_sampler.rs`.

- A timer task (interval ~800ms) spawned in `setup`, holding the `AppHandle`.
- Each tick:
  - Get the main window's outer position + size (physical pixels).
  - `GetDC(NULL)` (screen DC).
  - `GetPixel` at ~8 sample points chosen to land in transparent zones of the
    current layout:
    - four corners inset ~4px,
    - midpoints of the top, bottom, left, right edges (inset ~4px).
    These avoid the centered title, badge, bars, and footer buttons.
  - Discard `CLR_INVALID` results; if all invalid, skip this tick.
  - Average the valid RGB samples.
  - `luminance = 0.299*R + 0.587*G + 0.114*B`.
- `ReleaseDC` the screen DC every tick.
- Uses the `windows` crate (already a transitive Tauri dependency): GDI
  `GetDC`, `GetPixel`, `ReleaseDC`. No new crate.

### 2. Decision + anti-flicker (Rust)

- Hysteresis around the threshold to avoid flicker when luminance hovers near
  the midpoint:
  - go `light` (background is light) only when `luminance > 140`,
  - go `dark` only when `luminance < 115`,
  - otherwise keep the previous decision.
- Track the last emitted theme; only `app.emit("bg-theme", theme)` when it
  changes. `theme` is the string `"light"` or `"dark"` ("light" = light
  background → use dark text).
- Initial state: `dark` (matches today's default light-on-dark look).

### 3. Frontend palette switch (`src/components/OverlayWindow.tsx`)

- Listen for `bg-theme` (via the existing Tauri events hook or a local
  `listen`), store `bgTheme: "light" | "dark"` (default `"dark"`).
- Apply one of two palettes to the whole overlay:

  | token            | dark bg (default)         | light bg                  |
  |------------------|---------------------------|---------------------------|
  | primary text     | `#e8e8e8`                 | `#1a1a1a`                 |
  | 5HR label        | `#00d4ff`                 | `#0077aa`                 |
  | 7DAY label       | `#ffd700`                 | `#9a7d0a`                 |
  | text-shadow      | dark (current)            | light/white               |

- The bar fill colors (usage gradient) are unchanged; only text/label colors
  and the shadow flip. `UsageBar`'s `labelColor` is passed from
  `OverlayWindow`, so the label colors switch there.
- The system tray icon is independent (it has its own solid background) and is
  not affected.

## Data flow

```
[800ms timer] → GetPixel x8 → average → luminance
   → hysteresis → theme changed? → emit "bg-theme"
      → OverlayWindow listener → setBgTheme → palette classes/inline styles
```

## Error handling

- `GetDC` null / `GetPixel` returns `CLR_INVALID`: skip the offending sample;
  skip the whole tick if no valid samples. Never panic; the sampler must never
  take down the app.
- Window minimized/hidden (the `[×]` hides to tray): if the window is not
  visible, skip sampling (no point) — check `is_visible()`.
- Multi-monitor: `GetPixel` uses virtual-screen coordinates, which match the
  window's physical position, so it works across monitors without special
  handling.

## Testing

- Rust unit test for the pure decision function
  `decide_theme(prev, luminance) -> theme` covering: `luminance < 115` →
  `"dark"` (dark background → light text), `luminance > 140` → `"light"`
  (light background → dark text), and the hysteresis band `115..=140` keeping
  `prev`. (The GDI sampling itself is not unit-tested — it needs a live
  screen; verified manually.)
- Manual verification: move the overlay over a white window and a dark window;
  confirm the text flips and is readable in both, with no rapid flicker at
  the boundary.
- Existing frontend tests must stay green; add a small test that the overlay
  renders with the light-bg palette when `bgTheme` is `"light"` if it fits the
  existing test setup.

## Out of scope

- Per-element / per-region adaptation.
- The `WDA_EXCLUDEFROMCAPTURE` capture approach (fallback only).
- Adapting the tray icon.
