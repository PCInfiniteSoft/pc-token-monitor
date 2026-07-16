# macOS Menu Bar Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On macOS, present Claude usage in the menu bar (colored dot + 5-hour %) and open the existing React usage UI as a click popover, dropping the floating overlay — Windows behavior unchanged.

**Architecture:** All new behavior is gated with `#[cfg(target_os = "macos")]`. The tray icon becomes a band-colored dot with a `set_title` percentage; left-click positions and shows the pre-built `main` window under the menu bar item and it auto-hides on focus loss. The Windows badge icon, draggable overlay, and `aot_watcher` are untouched.

**Tech Stack:** Rust + Tauri 2.11.2 (`tray-icon`, `image-png`), `image` 0.25, `imageproc` 0.25; React 19 + TypeScript + Vitest.

## Global Constraints

- Platform split: every new behavior gated with `#[cfg(target_os = "macos")]`; Windows paths must compile and behave exactly as today.
- Menu-bar percentage and dot band track the **5-hour** window only (`five_hour.utilization`); Windows badge keeps the **dominant** percent.
- Band thresholds (verbatim): `>= 90` → red `[211, 47, 47]`; `>= 70` → amber `[216, 110, 0]`; else azure `[0, 103, 184]`.
- `utilization` is a `0..1` fraction; convert to `0..100 u8` with `(utilization * 100.0) as u8` then `.min(100)`.
- Tauri API names (2.11.2): `TrayIconBuilder::show_menu_on_left_click(bool)`, `TrayIcon::set_title::<S: AsRef<str>>(Option<S>)`, `TrayIconEvent::Click { position: PhysicalPosition<f64>, .. }`.
- Reuse the existing `main` window (`tauri.conf.json`: `visible: false`, `transparent`, `alwaysOnTop: true`); do not create a new window.
- Rust tests: `cd src-tauri && cargo test`. Frontend tests: `npm test`.

---

### Task 1: Band-colored dot icon + `set_title` on macOS

**Files:**
- Modify: `src-tauri/src/tray.rs` (extract band color; add `dot_rgba`; macOS branch in `update_tray_icon`)
- Test: `src-tauri/src/tray.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `fn band_color(percent: u8) -> Rgba<u8>`; `pub fn dot_rgba(percent: u8) -> Vec<u8>` (32×32 RGBA, transparent bg, filled band-colored circle); `update_tray_icon` unchanged signature `pub fn update_tray_icon(app: &tauri::AppHandle, percent: u8)`.
- Consumes: nothing from other tasks.

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `mod tests` in `src-tauri/src/tray.rs`:

```rust
    #[test]
    fn dot_rgba_correct_size() {
        assert_eq!(dot_rgba(50).len(), 32 * 32 * 4);
    }

    #[test]
    fn dot_rgba_transparent_corner() {
        let rgba = dot_rgba(50);
        // top-left pixel is outside the circle → fully transparent
        assert_eq!(rgba[3], 0);
    }

    #[test]
    fn dot_rgba_center_azure_below_70() {
        let rgba = dot_rgba(50);
        let idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(&rgba[idx..idx + 4], &[0, 103, 184, 255]);
    }

    #[test]
    fn dot_rgba_center_amber_70_to_89() {
        let rgba = dot_rgba(75);
        let idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(&rgba[idx..idx + 4], &[216, 110, 0, 255]);
    }

    #[test]
    fn dot_rgba_center_red_at_90_plus() {
        let rgba = dot_rgba(90);
        let idx = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(&rgba[idx..idx + 4], &[211, 47, 47, 255]);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test dot_rgba`
Expected: FAIL — `cannot find function 'dot_rgba' in this scope`.

- [ ] **Step 3: Add `band_color`, `dot_rgba`, and refactor `icon_rgba_for_percent`**

In `src-tauri/src/tray.rs`, add near the top (after the `FONT_BYTES` const):

```rust
/// The three usage bands, shared by the Windows badge icon and the macOS dot.
fn band_color(percent: u8) -> Rgba<u8> {
    if percent >= 90 {
        Rgba([211u8, 47, 47, 255]) // deep red
    } else if percent >= 70 {
        Rgba([216u8, 110, 0, 255]) // deep amber
    } else {
        Rgba([0u8, 103, 184, 255]) // deep azure
    }
}

/// A small filled dot in the band color on a transparent background, used as
/// the macOS menu-bar icon alongside the `set_title` percentage.
pub fn dot_rgba(percent: u8) -> Vec<u8> {
    let size = 32u32;
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    imageproc::drawing::draw_filled_circle_mut(&mut img, (16, 16), 9, band_color(percent));
    img.into_raw()
}
```

Then in `icon_rgba_for_percent`, replace the existing `let bg_color = if percent >= 90 { ... } else { ... };` block with:

```rust
    let bg_color = band_color(percent);
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test`
Expected: PASS — new `dot_rgba_*` tests pass and the existing `icon_rgba_*` tests still pass (colors unchanged).

- [ ] **Step 5: Add the macOS branch to `update_tray_icon`**

Replace the body of `update_tray_icon` in `src-tauri/src/tray.rs` with:

```rust
pub fn update_tray_icon(app: &tauri::AppHandle, percent: u8) {
    if let Some(tray) = app.tray_by_id("main") {
        #[cfg(target_os = "macos")]
        {
            let rgba = dot_rgba(percent);
            let icon = tauri::image::Image::new(&rgba, 32, 32);
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_title(Some(format!("{percent}%")));
            let _ = tray.set_tooltip(Some(&format!("PC Token Monitor — {percent}%")));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let rgba = icon_rgba_for_percent(percent);
            let icon = tauri::image::Image::new(&rgba, 32, 32);
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(&format!("PC Token Monitor — {percent}%")));
        }
    }
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly (no errors).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/tray.rs
git commit -m "feat(macos): band-colored dot icon and set_title percent"
```

---

### Task 2: Feed the 5-hour percent to the tray on macOS

**Files:**
- Modify: `src-tauri/src/lib.rs` (`dominant_percent` gating; add `five_hour_percent`; poll-loop call site ~line 159–160)

**Interfaces:**
- Consumes: `tray::update_tray_icon(app, percent)` from Task 1.
- Produces: `fn five_hour_percent(usage: &UsageData) -> u8` (macOS only).

- [ ] **Step 1: Gate `dominant_percent` off macOS and add `five_hour_percent`**

In `src-tauri/src/lib.rs`, replace the existing `dominant_percent` definition:

```rust
fn dominant_percent(usage: &UsageData) -> u8 {
    let pct = (usage.five_hour.utilization.max(usage.seven_day.utilization) * 100.0) as u8;
    pct.min(100)
}
```

with:

```rust
#[cfg(not(target_os = "macos"))]
fn dominant_percent(usage: &UsageData) -> u8 {
    let pct = (usage.five_hour.utilization.max(usage.seven_day.utilization) * 100.0) as u8;
    pct.min(100)
}

/// macOS menu bar shows the 5-hour window only.
#[cfg(target_os = "macos")]
fn five_hour_percent(usage: &UsageData) -> u8 {
    let pct = (usage.five_hour.utilization * 100.0) as u8;
    pct.min(100)
}
```

- [ ] **Step 2: Pass the platform-specific percent at the call site**

In `src-tauri/src/lib.rs`, in the poll loop, replace:

```rust
                let pct = dominant_percent(u);
                tray::update_tray_icon(&app, pct);
```

with:

```rust
                #[cfg(target_os = "macos")]
                let pct = five_hour_percent(u);
                #[cfg(not(target_os = "macos"))]
                let pct = dominant_percent(u);
                tray::update_tray_icon(&app, pct);
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly with no unused-function warnings for `dominant_percent`/`five_hour_percent`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(macos): drive menu bar percent from 5-hour window"
```

---

### Task 3: Left-click popover + auto-hide on focus loss

**Files:**
- Modify: `src-tauri/src/tray.rs` (`setup_tray`: `show_menu_on_left_click(false)`, position on click; add `position_popover`)
- Modify: `src-tauri/src/lib.rs` (`main_win.on_window_event`: hide on `Focused(false)`)

**Interfaces:**
- Consumes: the pre-built `main` window; `TrayIconEvent::Click { position, .. }`.
- Produces: `fn position_popover(win: &tauri::WebviewWindow, cursor: tauri::PhysicalPosition<f64>)` (macOS only).

- [ ] **Step 1: Add `position_popover` helper (macOS only)**

In `src-tauri/src/tray.rs`, add above `setup_tray`:

```rust
/// Place the popover just below the menu-bar item, centered on the click.
#[cfg(target_os = "macos")]
fn position_popover(win: &tauri::WebviewWindow, cursor: tauri::PhysicalPosition<f64>) {
    let width = win.outer_size().map(|s| s.width as f64).unwrap_or(200.0);
    let x = (cursor.x - width / 2.0).max(8.0);
    // The click y sits inside the menu bar; add a small gap so the panel clears it.
    let y = cursor.y + 6.0;
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}
```

- [ ] **Step 2: Suppress the left-click menu on macOS**

In `src-tauri/src/tray.rs` `setup_tray`, replace the fluent builder chain that starts `TrayIconBuilder::with_id("main")` and ends `.build(app)?;` so the builder becomes a mutable binding with a macOS-only option. Replace:

```rust
    TrayIconBuilder::with_id("main")
        .icon(initial_icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
```

with:

```rust
    let mut builder = TrayIconBuilder::with_id("main")
        .icon(initial_icon)
        .menu(&menu);
    #[cfg(target_os = "macos")]
    {
        // Left-click drives the popover; the Settings/Quit menu stays on right-click.
        builder = builder.show_menu_on_left_click(false);
    }
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
```

(The rest of the chain — `.on_menu_event(...)`, `.on_tray_icon_event(...)`, `.build(app)?` — is unchanged except for Step 3.)

- [ ] **Step 3: Position the popover under the item on left-click**

In `src-tauri/src/tray.rs`, replace the whole `.on_tray_icon_event(...)` closure with:

```rust
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position: _position,
                ..
            } = event
            {
                let app = tray.app_handle();
                let win = app.get_webview_window("main").unwrap();
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                } else {
                    #[cfg(target_os = "macos")]
                    position_popover(&win, _position);
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
```

- [ ] **Step 4: Hide the popover when it loses focus (macOS)**

In `src-tauri/src/lib.rs`, replace the `main_win.on_window_event(...)` closure:

```rust
            main_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = win_for_event.hide();
                }
            });
```

with:

```rust
            main_win.on_window_event(move |event| match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = win_for_event.hide();
                }
                #[cfg(target_os = "macos")]
                tauri::WindowEvent::Focused(false) => {
                    let _ = win_for_event.hide();
                }
                _ => {}
            });
```

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/tray.rs src-tauri/src/lib.rs
git commit -m "feat(macos): open usage popover on tray click, hide on blur"
```

---

### Task 4: Stop the always-on-top overlay watcher on macOS

**Files:**
- Modify: `src-tauri/src/lib.rs` (gate `aot_watcher::start_aot_watcher` call to Windows)

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing new. On macOS the `main` window is controlled solely by the tray click (Task 3), never by process-foreground logic.

- [ ] **Step 1: Gate the watcher to Windows**

In `src-tauri/src/lib.rs`, replace:

```rust
            aot_watcher::start_aot_watcher(
                app_handle.clone(),
                config_arc.clone(),
                started_online.clone(),
            );
```

with:

```rust
            #[cfg(windows)]
            aot_watcher::start_aot_watcher(
                app_handle.clone(),
                config_arc.clone(),
                started_online.clone(),
            );
```

- [ ] **Step 2: Verify it compiles on macOS**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly (no unused-import or unused-variable errors — `app_handle`, `config_arc`, and `started_online` remain used by `start_poll_loop`).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(macos): don't run the foreground overlay watcher"
```

---

### Task 5: Quit control in the popover

**Files:**
- Modify: `src-tauri/src/lib.rs` (add `quit_app` command; register in `generate_handler!`)
- Modify: `src/components/OverlayWindow.tsx` (add `quit()` + button)
- Test: `src/components/OverlayWindow.test.tsx` (new)

**Interfaces:**
- Consumes: `invoke("quit_app")` from the frontend.
- Produces: `#[tauri::command] fn quit_app(app: AppHandle)`.

- [ ] **Step 1: Add the `quit_app` command**

In `src-tauri/src/lib.rs`, add next to the other `#[tauri::command]` functions (e.g. after `open_settings`):

```rust
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}
```

Then add `quit_app` to the handler list. Replace:

```rust
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_plan,
            set_aot_mode,
            set_aot_allowlist,
            open_settings
        ])
```

with:

```rust
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_plan,
            set_aot_mode,
            set_aot_allowlist,
            open_settings,
            quit_app
        ])
```

- [ ] **Step 2: Verify Rust compiles**

Run: `cd src-tauri && cargo build`
Expected: builds cleanly.

- [ ] **Step 3: Write the failing frontend test**

Create `src/components/OverlayWindow.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide: vi.fn().mockResolvedValue(undefined) }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("../stores/usageStore", () => ({
  useUsageStore: () => ({
    frontendState: {
      usage: {
        five_hour: { utilization: 0.73, resets_at: "2026-07-16T15:30:00Z" },
        seven_day: { utilization: 0.4, resets_at: "2026-07-23T10:00:00Z" },
      },
      config: { plan: "Max200" },
      user_name: "tester",
    },
    isOffline: () => false,
  }),
}));

import { invoke } from "@tauri-apps/api/core";
import { OverlayWindow } from "./OverlayWindow";

describe("OverlayWindow", () => {
  beforeEach(() => vi.clearAllMocks());

  it("invokes quit_app when the quit button is clicked", () => {
    render(<OverlayWindow />);
    fireEvent.click(screen.getByLabelText("quit"));
    expect(invoke).toHaveBeenCalledWith("quit_app");
  });
});
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `npm test -- OverlayWindow`
Expected: FAIL — `Unable to find an element with the label text of: quit`.

- [ ] **Step 5: Add the `quit()` handler and button**

In `src/components/OverlayWindow.tsx`, add the handler after `minimize()`:

```tsx
  function quit() {
    invoke("quit_app").catch(() => {});
  }
```

Then add a Quit button between the settings (`⚙`) and close (`×`) buttons, so the control group reads:

```tsx
          <button
            onClick={openSettings}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="settings"
          >
            ⚙
          </button>
          <button
            onClick={quit}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="quit"
          >
            ⏻
          </button>
          <button
            onClick={minimize}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="close"
          >
            ×
          </button>
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `npm test -- OverlayWindow`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src/components/OverlayWindow.tsx src/components/OverlayWindow.test.tsx
git commit -m "feat: add quit control to usage popover"
```

---

## Manual verification (macOS)

After all tasks, run `npm run tauri dev` on the Mac and confirm:

- [ ] Menu bar shows a colored dot + `NN%` reflecting the **5-hour** window; the dot flips azure → amber at 70% and amber → red at 90%.
- [ ] No overlay window appears at startup.
- [ ] Left-clicking the menu bar item opens the usage panel directly under it; the 5H and 7D bars and reset countdowns render.
- [ ] Clicking elsewhere (another app/window) hides the panel.
- [ ] Left-clicking again toggles it closed; Settings and Quit both work (Quit from the panel button; Settings from the panel and from the right-click menu).

## Manual verification (Windows regression)

- [ ] Tray badge still renders the digit icon from the **dominant** percent.
- [ ] The draggable always-on-top overlay and Auto/Pinned show-hide behavior are unchanged.
