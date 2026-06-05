# Adaptive Overlay Text Color Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Flip the transparent overlay's text/label palette between light-on-dark and dark-on-light based on the luminance of the desktop directly behind the window.

**Architecture:** A native Rust timer samples the screen at transparent points inside the window rect (GDI `GetPixel`), averages luminance, applies hysteresis, and emits a `bg-theme` event (`"light"`/`"dark"`) on change. The React overlay listens and swaps a palette (text/label colors + text-shadow) applied via CSS variables.

**Tech Stack:** Tauri 2 (Rust), `windows` crate (GDI), React 19 + TypeScript, Vitest, Rust `#[test]`.

**Spec:** `docs/superpowers/specs/2026-06-05-adaptive-text-color-design.md`

---

## File Structure

- Create: `src-tauri/src/bg_sampler.rs` — luminance sampling + `decide_theme` + sampler task.
- Modify: `src-tauri/src/lib.rs` — `mod bg_sampler;` and start the sampler in `setup`.
- Modify: `src-tauri/Cargo.toml` — add `windows` as a direct Windows-only dependency.
- Create: `src/overlayPalette.ts` — pure palette map (testable).
- Create: `src/overlayPalette.test.ts` — palette unit test.
- Modify: `src/components/OverlayWindow.tsx` — listen for `bg-theme`, apply palette + CSS vars.
- Modify: `src/components/UsageBar.tsx` — use CSS vars for the `%` value and reset text.

Note: the dev server (`npm run tauri dev`) is running and rebuilds on file save. After Rust edits expect a ~40–70s incremental rebuild; after frontend-only edits Vite HMR applies instantly.

---

### Task 1: `decide_theme` pure decision function (Rust, TDD)

**Files:**
- Create: `src-tauri/src/bg_sampler.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod bg_sampler;`)

- [ ] **Step 1: Create `bg_sampler.rs` with the function and failing tests**

```rust
//! Samples the desktop behind the transparent overlay and decides whether the
//! background is light or dark, so the frontend can flip its text palette.

/// Decide the theme from average background luminance (0..=255), with
/// hysteresis to avoid flicker near the threshold.
///
/// Returns `"light"` when the background is light (frontend should use dark
/// text) and `"dark"` when the background is dark (light text). Within the
/// 115..=140 band the previous theme is kept.
pub fn decide_theme(prev: &str, luminance: f64) -> &'static str {
    if luminance > 140.0 {
        "light"
    } else if luminance < 115.0 {
        "dark"
    } else if prev == "light" {
        "light"
    } else {
        "dark"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_background_flips_to_light() {
        assert_eq!(decide_theme("dark", 200.0), "light");
        assert_eq!(decide_theme("dark", 145.0), "light");
    }

    #[test]
    fn dark_background_flips_to_dark() {
        assert_eq!(decide_theme("light", 50.0), "dark");
        assert_eq!(decide_theme("light", 110.0), "dark");
    }

    #[test]
    fn hysteresis_band_keeps_previous() {
        assert_eq!(decide_theme("dark", 130.0), "dark");
        assert_eq!(decide_theme("light", 130.0), "light");
        assert_eq!(decide_theme("dark", 115.0), "dark");
        assert_eq!(decide_theme("light", 140.0), "light");
    }
}
```

- [ ] **Step 2: Register the module**

In `src-tauri/src/lib.rs`, add the module declaration alongside the other `mod` lines near the top (e.g. after `mod tray;`):

```rust
mod bg_sampler;
```

- [ ] **Step 3: Run the tests, expect them to pass**

Run: `cd src-tauri && cargo test --no-default-features bg_sampler`
Expected: the 3 `bg_sampler::tests::*` tests PASS. (A `dead_code` warning for `decide_theme` is expected until Task 2 uses it — ignore it.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bg_sampler.rs src-tauri/src/lib.rs
git commit -m "feat: add bg luminance -> theme decision with hysteresis

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: GDI sampling + sampler task wired into setup (Rust)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/bg_sampler.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the `windows` dependency (Windows-only, version matched to Tauri)**

In `src-tauri/Cargo.toml`, after the `[dev-dependencies]` block add a target-specific dependencies section:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
] }
```

- [ ] **Step 2: Add the sampler and task to `bg_sampler.rs`**

Add these imports at the very top of `src-tauri/src/bg_sampler.rs` (above the `decide_theme` doc comment is fine):

```rust
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
```

Then append the sampling helper and the task launcher to the file (below `decide_theme`, above the `#[cfg(test)]` module):

```rust
/// Average luminance (0..=255) of the desktop at several transparent points
/// inside the given physical window rect, or `None` if no pixel could be read.
#[cfg(windows)]
fn sample_luminance(x: i32, y: i32, w: i32, h: i32) -> Option<f64> {
    use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC, CLR_INVALID};

    if w <= 8 || h <= 8 {
        return None;
    }
    let inset = 4;
    let points = [
        (x + inset, y + inset),
        (x + w - inset, y + inset),
        (x + inset, y + h - inset),
        (x + w - inset, y + h - inset),
        (x + w / 2, y + inset),
        (x + w / 2, y + h - inset),
        (x + inset, y + h / 2),
        (x + w - inset, y + h / 2),
    ];

    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let mut sum = 0f64;
        let mut count = 0u32;
        for (px, py) in points {
            let color = GetPixel(hdc, px, py);
            if color == CLR_INVALID {
                continue;
            }
            let raw = color.0; // COLORREF: 0x00BBGGRR
            let r = (raw & 0xFF) as f64;
            let g = ((raw >> 8) & 0xFF) as f64;
            let b = ((raw >> 16) & 0xFF) as f64;
            sum += 0.299 * r + 0.587 * g + 0.114 * b;
            count += 1;
        }
        let _ = ReleaseDC(None, hdc);
        if count == 0 {
            None
        } else {
            Some(sum / count as f64)
        }
    }
}

#[cfg(not(windows))]
fn sample_luminance(_x: i32, _y: i32, _w: i32, _h: i32) -> Option<f64> {
    None
}

/// Spawn a background task that samples the desktop behind the overlay every
/// ~800ms and emits `bg-theme` ("light"/"dark") whenever the decision changes.
pub fn start_bg_sampler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut theme: &str = "dark";
        loop {
            tokio::time::sleep(Duration::from_millis(800)).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };
            if !win.is_visible().unwrap_or(false) {
                continue;
            }
            let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) else {
                continue;
            };

            if let Some(lum) =
                sample_luminance(pos.x, pos.y, size.width as i32, size.height as i32)
            {
                let next = decide_theme(theme, lum);
                if next != theme {
                    theme = next;
                    let _ = app.emit("bg-theme", theme);
                }
            }
        }
    });
}
```

- [ ] **Step 3: Start the sampler in `setup`**

In `src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, right after the existing `tray::setup_tray(app)?;` line, add:

```rust
            bg_sampler::start_bg_sampler(app.handle().clone());
```

- [ ] **Step 4: Build and verify it compiles and runs**

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds successfully (no errors). If the `windows` crate exposes a slightly different signature, fix per the compiler message — `GetDC`/`ReleaseDC` take `Option<HWND>` and `GetPixel` returns a `COLORREF` newtype whose inner `u32` is `.0`.

Then confirm the running dev app picks up the rebuild: the dev server log should show `Finished` then `Running ...pc-token-monitor.exe`. No new runtime errors.

- [ ] **Step 5: Manual check (smoke)**

Move a bright white window (e.g. Notepad maximized area) behind the overlay, then a dark window. Watch the dev log is clean; the visible flip is verified in Task 4 once the frontend listens. For now just confirm the app is stable (no panics, still polling).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/bg_sampler.rs src-tauri/src/lib.rs
git commit -m "feat: sample desktop luminance behind overlay and emit bg-theme

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Frontend palette module (TypeScript, TDD)

**Files:**
- Create: `src/overlayPalette.ts`
- Create: `src/overlayPalette.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/overlayPalette.test.ts`:

```ts
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
```

- [ ] **Step 2: Run it, expect failure**

Run: `npm test -- overlayPalette`
Expected: FAIL — cannot find module `./overlayPalette`.

- [ ] **Step 3: Implement the palette module**

Create `src/overlayPalette.ts`:

```ts
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
```

- [ ] **Step 4: Run the test, expect pass**

Run: `npm test -- overlayPalette`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src/overlayPalette.ts src/overlayPalette.test.ts
git commit -m "feat: overlay palette module for adaptive theming

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: OverlayWindow listens for `bg-theme` and applies the palette

**Files:**
- Modify: `src/components/OverlayWindow.tsx`

- [ ] **Step 1: Update imports**

Replace the top imports of `src/components/OverlayWindow.tsx`:

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUsageStore } from "../stores/usageStore";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";
```

with:

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useUsageStore } from "../stores/usageStore";
import { overlayPalette, type BgTheme } from "../overlayPalette";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";
```

- [ ] **Step 2: Add the theme state + listener and compute the palette**

In `OverlayWindow`, just after `const [alwaysOnTop, setAlwaysOnTop] = useState(true);` add:

```tsx
  const [bgTheme, setBgTheme] = useState<BgTheme>("dark");

  useEffect(() => {
    const unlisten = listen<string>("bg-theme", (event) => {
      setBgTheme(event.payload === "light" ? "light" : "dark");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const pal = overlayPalette(bgTheme);
```

- [ ] **Step 3: Apply palette to the root, title, bars, footer**

Replace the root `<div ...>` opening tag (the one with `data-tauri-drag-region` and the `textShadow` style) with:

```tsx
    <div
      data-tauri-drag-region
      className="w-full h-full flex flex-col select-none"
      style={{
        textShadow: pal.shadow,
        // CSS vars consumed by UsageBar and the elements below.
        ["--ov-text" as string]: pal.text,
        ["--ov-muted" as string]: pal.muted,
      }}
    >
```

Replace the title span:

```tsx
        <span className="font-mono text-[10px] tracking-widest pointer-events-none" style={{ color: "var(--ov-muted)" }}>
          ⬡ PC TOKEN MONITOR
        </span>
```

Replace the two `<UsageBar .../>` `labelColor` props to use the palette:

```tsx
            <UsageBar
              label="5HR"
              utilization={usage.five_hour.utilization}
              resetsAt={usage.five_hour.resets_at}
              labelColor={pal.label5}
            />
            <UsageBar
              label="7DAY"
              utilization={usage.seven_day.utilization}
              resetsAt={usage.seven_day.resets_at}
              labelColor={pal.label7}
            />
```

Replace the "connecting..." span:

```tsx
          <span className="font-mono text-[10px] text-center py-2" style={{ color: "var(--ov-muted)" }}>
            connecting...
          </span>
```

Replace the footer "ALWAYS ON TOP" button:

```tsx
        <button
          onClick={toggleAlwaysOnTop}
          className="font-mono text-[9px] tracking-widest transition-colors"
          style={{ color: alwaysOnTop ? pal.label5 : "var(--ov-muted)" }}
        >
          [⊤ ALWAYS ON TOP]
        </button>
```

Replace the footer "[×]" button:

```tsx
        <button
          onClick={minimize}
          className="font-mono text-[9px] transition-opacity hover:opacity-70"
          style={{ color: "var(--ov-muted)" }}
        >
          [×]
        </button>
```

- [ ] **Step 4: Verify HMR + existing tests**

Run: `npm test`
Expected: all existing tests PASS (16 + the 2 new palette tests = 18), no TypeScript errors.

The dev app applies this via Vite HMR (no Rust rebuild). Leave the visible flip check for Step 5 of Task 5 (after UsageBar adapts too).

- [ ] **Step 5: Commit**

```bash
git add src/components/OverlayWindow.tsx
git commit -m "feat: OverlayWindow listens for bg-theme and swaps palette

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: UsageBar reads palette CSS vars

**Files:**
- Modify: `src/components/UsageBar.tsx`

- [ ] **Step 1: Adapt the `%` value color**

In `src/components/UsageBar.tsx`, replace the `%` value span:

```tsx
        <span
          className="font-mono text-xs w-8 text-right shrink-0"
          style={{ color: atLimit ? "#ff3232" : "#ffffff" }}
        >
          {pct}%
        </span>
```

with:

```tsx
        <span
          className="font-mono text-xs w-8 text-right shrink-0"
          style={{ color: atLimit ? "#ff3232" : "var(--ov-text)" }}
        >
          {pct}%
        </span>
```

- [ ] **Step 2: Adapt the reset/countdown muted text**

Replace:

```tsx
      <div className="font-mono text-[9px] text-[#666] pl-10">
```

with:

```tsx
      <div className="font-mono text-[9px] pl-10" style={{ color: "var(--ov-muted)" }}>
```

- [ ] **Step 3: Run tests**

Run: `npm test`
Expected: all tests PASS (18). The existing `UsageBar`-related tests still pass (color is now via CSS var; the var falls back to inherited/unset in jsdom, which does not break assertions on text/aria).

- [ ] **Step 4: Manual verification (the real check)**

With the dev app running:
1. Drag the overlay over a **white** area (white window / white webpage). The text + `%` + labels must turn dark and stay readable.
2. Drag it over a **dark** area. Text returns to light.
3. Hover near a 50/50 boundary — confirm no rapid flicker (hysteresis holds).
4. Hide to tray with `[×]` and reopen — no errors; sampling resumes.

- [ ] **Step 5: Commit**

```bash
git add src/components/UsageBar.tsx
git commit -m "feat: UsageBar uses overlay palette vars for value and reset text

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final Verification

- [ ] `cd src-tauri && cargo test --no-default-features` → all Rust tests pass (existing 24 + 3 new = 27).
- [ ] `npm test` → all frontend tests pass (existing 16 + 2 new = 18).
- [ ] Manual: text flips correctly over white and dark backgrounds, no flicker, stable after hide/show.
- [ ] `git status` clean (except untracked `scripts/`).

## Notes / Risks

- The `windows` crate version must match Tauri's (0.61) to avoid compiling a second copy. If `cargo build` pulls a different version, pin to the one shown by `cargo tree -p windows` for the existing build.
- Single global decision: a strongly two-tone background behind the window can be misjudged. If unacceptable in use, switch to the fallback in the spec (capture the region with `WDA_EXCLUDEFROMCAPTURE`); not built here.
- `outer_position`/`GetPixel` both use physical pixels, so high-DPI and multi-monitor work without scaling math.
