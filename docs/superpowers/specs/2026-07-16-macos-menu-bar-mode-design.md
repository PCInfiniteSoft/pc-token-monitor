# macOS menu bar mode — Design

**Date:** 2026-07-16
**Status:** Approved (pending implementation)

## Problem

On macOS the always-on-top overlay window is redundant: the tray icon already
lives in the menu bar, which is the native place for an at-a-glance status
indicator. The user wants macOS to drop the floating overlay entirely and
present usage in the menu bar itself — a colored dot plus a live percentage —
with details available on click via a popover, not a persistent window.

Windows keeps its current behavior unchanged (rendered badge icon + draggable
always-on-top overlay). This is a macOS-only presentation change built from the
same poll loop and the same React usage UI.

## Desired behavior (macOS)

- **Menu bar item** shows a small colored dot (band color) followed by the
  **5-hour** utilization as text, e.g. `● 73%`, via the macOS-only
  `TrayIcon::set_title`.
- **The percentage and the dot band both track the 5-hour window only**
  (`five_hour`), not the dominant/worst of 5H and 7D. (Windows' badge icon keeps
  using the dominant percent as it does today.)
- **No overlay on startup.** The `main` window stays hidden at boot on macOS.
- **Left-click the menu bar item** opens the existing React usage UI as a
  **popover** positioned directly under the item, then focuses it.
- **The popover auto-hides** when it loses focus (click-away / app switch).
- The popover contains the 5H + 7D bars with reset countdowns (as today) plus
  **Settings** and **Quit** buttons.

## Non-goals

- No change to Windows behavior, the badge-icon renderer, wallpaper sampling, or
  adaptive text color — those stay Windows-only paths.
- No new data, cost, or provider features — presentation only.
- The 7-day (and Sonnet/Opus) numbers still appear **inside** the popover; only
  the menu-bar text is trimmed to 5H.

## Approach

### 1. Menu bar rendering (`tray.rs`)

Behind `#[cfg(target_os = "macos")]`:

- Render the icon as a **small filled dot** in the band color instead of the
  digit badge. Reuse the existing band thresholds from
  `icon_rgba_for_percent` (`>=90` red `211,47,47`; `>=70` amber `216,110,0`;
  else azure `0,103,184`). Add a `dot_rgba(percent)` helper (or a band-only
  variant) so the color logic stays in one place.
- Set the label with `tray.set_title(Some(&format!("{percent}%")))`.

`update_tray_icon` gains a macOS path that takes the **5-hour** percent, sets
the dot icon, and sets the title. On Windows it keeps rendering the digit badge
from the dominant percent. The caller in the poll loop passes the 5H percent on
macOS (see §3).

### 2. Popover behavior (`tray.rs` + `lib.rs`)

Reuse the existing `main` webview window (already frameless/transparent) as the
popover on macOS:

- **Left-click** (`TrayIconEvent::Click`, left, `Up`): use the event's tray
  `rect`/`position` to place the window just below the menu bar item, then
  `show()` + `set_focus()`. If already visible, `hide()` (toggle).
- **Focus lost:** in the `main` window's `on_window_event`, on
  `WindowEvent::Focused(false)` call `hide()` (macOS only). This gives native
  click-away dismissal.
- On macOS, **do not** apply the drag-anywhere / always-visible overlay
  positioning that Windows uses; the popover is anchored on each open.

### 3. Startup + poll loop (`lib.rs`)

- macOS: leave `main` hidden at startup (do not show it in `setup`). This
  composes with the existing "hide overlay until first online" logic — the
  popover simply never auto-shows; it only appears on click.
- In the poll loop where `tray::update_tray_icon` is currently called with the
  dominant percent, pass the **5-hour** percent on macOS. Keep the dominant
  percent on Windows. The 5H value is already available in the fetched
  `UsageData` as `five_hour.utilization`, stored as a `0..1` fraction; convert
  it to a `0..100` `u8` the same way the dominant percent is already derived in
  the poll loop (`(utilization * 100).round() as u8`) before passing it to the
  tray updater.

### 4. Popover controls (frontend)

Add **Settings** and **Quit** actions to the usage UI shown in the popover
(`OverlayWindow.tsx` or a small child component), wired to the existing
`open_settings_window` command and app exit. On macOS these render inside the
panel per the approved mockup. (On Windows the same buttons are harmless but the
existing tray menu already provides Settings/Quit; gating their visibility to
macOS is acceptable if simpler.)

## Files touched

- `src-tauri/src/tray.rs` — dot icon + `set_title`; left-click opens/positions
  the popover under the item (macOS).
- `src-tauri/src/lib.rs` — keep `main` hidden at startup on macOS; hide on
  focus-lost; pass 5H percent to the tray updater on macOS.
- `src/components/OverlayWindow.tsx` (+ possible small child) — Settings/Quit
  buttons in the panel.

## Platform split

All new behavior is gated with `#[cfg(target_os = "macos")]`. Windows code paths
(badge icon from dominant percent, draggable always-on-top overlay, wallpaper
sampling, adaptive text) are untouched, so one codebase ships both.

## Testing

- **Rust unit:** `dot_rgba` (or band helper) returns the correct band color at
  the `50 / 75 / 90` boundaries, mirroring the existing `icon_rgba_*` tests.
- **Manual (macOS):** menu bar shows `● {5H}%`; color flips at 70/90; left-click
  opens the popover under the item; clicking elsewhere hides it; Settings/Quit
  work; no overlay appears at startup.
- **Manual (Windows):** unchanged — badge icon with dominant percent, draggable
  overlay, tray menu.
