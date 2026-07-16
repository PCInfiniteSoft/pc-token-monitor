# macOS native popover UI — Design

**Date:** 2026-07-16
**Status:** Approved (mockup approved; pending implementation)
**Follows:** [2026-07-16-macos-menu-bar-mode-design.md](2026-07-16-macos-menu-bar-mode-design.md)

## Problem

The macOS menu-bar mode reuses `OverlayWindow.tsx` — a UI built as a Windows
always-on-top HUD — as its click popover. On macOS it looks wrong: a hard-edged
transparent dark rectangle, 9–10px monospace text with wide tracking, an
`⬡ PC TOKEN MONITOR` title bar with a drag region and an `×` "minimize" button,
and a fixed dark palette that ignores the system appearance. A menu-bar popover
should feel native: rounded, translucent, SF Pro, and appearance-aware.

## Desired behavior (macOS only)

Replace the popover contents on macOS with a native-feeling panel (approved
mockup):

- Rounded 12px panel with a translucent material (`backdrop-filter` blur) and a
  soft shadow, on the existing transparent/frameless window.
- SF Pro system typography; large tabular-figure percentages, readable labels.
- No title bar, drag region, or close button — the popover dismisses on blur
  (already implemented in the menu-bar-mode feature).
- Header: "Claude Usage" + plan badge. Two rows (5-Hour, 7-Day): uppercase
  label, big % (band-colored), gradient bar, muted reset countdown. Footer:
  user (avatar + name) on the left, round Settings and Quit icon buttons on the
  right.
- Follows the system **light and dark** appearance via
  `prefers-color-scheme` (WKWebView honors the macOS appearance).
- Uses macOS system accent colors: azure `#0a84ff`, amber `#ff9f0a`,
  red `#ff453a` (band thresholds unchanged: `<70` azure, `70–89` amber,
  `>=90` red).

## Non-goals

- **No change to the Windows overlay.** `OverlayWindow.tsx` and its HUD look,
  drag region, adaptive text color, and wallpaper sampling stay exactly as
  today. This is a macOS-gated component swap.
- No native `NSVisualEffectView` vibrancy for v1 — CSS `backdrop-filter` on the
  transparent window is sufficient and lower-risk. (Native window effects can be
  a later enhancement.)
- No new usage data — same `five_hour`/`seven_day`/plan/user from the store.

## Approach

### 1. Platform detection (frontend)

Add `src/platform.ts` exporting `isMacOS(): boolean` = `/Mac/i.test(navigator.userAgent)`.
(WKWebView on macOS reports "Macintosh" in the UA; the Windows WebView2 reports
"Windows". No new dependency.)

### 2. New `MacPopover.tsx` component

A macOS-only component rendering the approved mockup, driven by the same
`useUsageStore` data as `OverlayWindow`:

- Consumes `frontendState.usage` (`five_hour`, `seven_day`), `config.plan`,
  `user_name`, and `isOffline()`.
- Header title + `PlanBadge` (reuse existing `PlanBadge`), two usage rows, footer
  with avatar/name and Settings/Quit buttons.
- Settings button → `invoke("open_settings")`; Quit button → `invoke("quit_app")`
  (both already exist).
- Styling with Tailwind v4 utilities plus a small amount of custom CSS for
  `backdrop-filter`; light/dark via Tailwind's `dark:` variant (media strategy)
  and/or `prefers-color-scheme`.
- Band color helper mirrors the Rust thresholds (`<70/70/90`).

### 3. Shared reset-countdown util (DRY)

Extract `formatCountdown(resetsAt: string): string` from `UsageBar.tsx` into
`src/usageFormat.ts` and import it in both `UsageBar.tsx` and `MacPopover.tsx`
(so the two never drift). Behavior identical to today's `UsageBar`
implementation.

### 4. Swap in `App.tsx`

In `OverlayApp`, render `<MacPopover />` when `isMacOS()`, else `<OverlayWindow />`.
Keep the `settings` window and `FirstRunDialog` paths unchanged.

### 5. Window sizing (Rust, macOS)

The popover panel is roomier than the 200×140 overlay. On macOS, at
`setup`, resize the `main` window to the popover size and keep the width in sync
with the tray positioning constant:

- In `tray.rs`, define `POPOVER_WIDTH` (already added) and add
  `POPOVER_HEIGHT`; set them to the popover dimensions (e.g. `270.0 × 200.0`).
- In `lib.rs` `setup`, on macOS resize `main` to those dimensions
  (`win.set_size(tauri::LogicalSize::new(POPOVER_WIDTH, POPOVER_HEIGHT))`),
  exposing the constants from `tray.rs` (make them `pub`). Windows keeps the
  `tauri.conf.json` 200×140.

The window stays `transparent: true` / `decorations: false`; the rounded panel's
corners fall on the transparent area so the desktop shows through.

## Files touched

- Create: `src/platform.ts`, `src/usageFormat.ts`, `src/components/MacPopover.tsx`,
  `src/components/MacPopover.test.tsx`, `src/usageFormat.test.ts`.
- Modify: `src/components/UsageBar.tsx` (import shared `formatCountdown`),
  `src/App.tsx` (platform swap), `src-tauri/src/tray.rs` (`POPOVER_HEIGHT`,
  make dims `pub`), `src-tauri/src/lib.rs` (macOS window resize).

## Testing

- **Unit:** `usageFormat.test.ts` covers `formatCountdown` (days/hours, hours/mins,
  `resetting...` when elapsed) — moved with the code so coverage isn't lost.
- **Component:** `MacPopover.test.tsx` — renders 5-Hour/7-Day rows and percentages
  from a mocked store; Settings button invokes `open_settings`; Quit invokes
  `quit_app`; band color follows the 70/90 thresholds.
- **Manual (macOS):** popover shows the rounded native panel; light/dark follows
  the system appearance; bars/percentages/countdowns correct; Settings/Quit work.
- **Manual (Windows):** the overlay HUD is unchanged.

## Windows regression guard

`isMacOS()` is `false` on Windows, so `App.tsx` renders the unchanged
`OverlayWindow`; the Rust resize is macOS-gated; `UsageBar.tsx` behavior is
byte-identical after the pure `formatCountdown` extraction. Windows is untouched
in behavior.
