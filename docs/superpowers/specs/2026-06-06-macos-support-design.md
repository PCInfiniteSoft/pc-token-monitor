# macOS Support — Design (deferred: implement when a Mac is available)

Date: 2026-06-06
Status: 📐 Design only — NOT implemented. Blocked on Mac access for build + verify.

## Why deferred

The maintainer currently has no Mac. macOS support requires native code
(NSWorkspace, screen capture, NSWindow) that cannot be behavior-tested without a
Mac, and unsigned `.app` bundles are hard-blocked by Gatekeeper. Writing and
shipping untested platform code is not acceptable, so this is captured as a
design to execute once a Mac (and ideally an Apple Developer account) exists.

## What already works cross-platform

Tauri + the web UI carry most of the app unchanged: the overlay window, usage
fetch (`oauth_fetcher`, HTTPS), JSONL fallback, config persistence, the Settings
window, the tray icon (renders from an in-memory image), and the hide/show
visibility logic. `account.rs` reads `~/.claude.json` which exists on macOS too.

## What is Windows-only today (the work)

Two modules gate behind `#[cfg(windows)]` with `#[cfg(not(windows))]` stubs that
return `None`, so on macOS they currently no-op (always-on-top auto + adaptive
text simply don't function):

- `aot_watcher.rs` — `foreground_exe()` (Win32 `GetForegroundWindow` +
  `QueryFullProcessImageNameW`), `set_no_activate()` (`WS_EX_NOACTIVATE`), and
  the topmost behavior.
- `bg_sampler.rs` — `sample_luminance()` (GDI `GetDC`/`GetPixel`).

## Architecture for the port

Introduce a thin platform abstraction so the watcher/sampler call
platform-neutral functions:

- A `platform` module with `#[cfg(windows)] mod windows;` and
  `#[cfg(target_os = "macos")] mod macos;`, each providing:
  - `foreground_app() -> Option<(u32 pid, String name)>`
  - `sample_luminance(x, y, w, h) -> Option<f64>`
  - `set_non_activating(&WebviewWindow)` / window-level helper
- `aot_watcher`/`bg_sampler` keep their pure logic (`should_pin`, `decide_theme`)
  and call `platform::*`. The pure functions already have unit tests that run on
  any OS.

## Sub-projects (each its own spec → plan → implement, in order)

### 1. Build readiness (do first; partly verifiable via CI)
- Add a `macos-latest` job to the release workflow producing `.app`/`.dmg`.
- Confirm the crate compiles for macOS and bundles; confirm graceful
  degradation (Windows-only features no-op, app still launches and shows usage).
- Caveat: CI proves it *compiles + bundles*, not that it *runs* — needs a Mac to
  confirm launch.

### 2. Foreground detection (macOS)
- `NSWorkspace.sharedWorkspace.frontmostApplication` →
  `bundleIdentifier` / `localizedName` (via `objc2`/`cocoa`).
- Map the existing allowlist concept to bundle ids (e.g. `com.apple.Terminal`,
  `com.googlecode.iterm2`, `com.anthropic.claude`) — note the allowlist becomes
  bundle-id-based on macOS; decide whether to keep exe-name + add bundle-id, or
  store both. The Settings allowlist editor needs a macOS-appropriate hint.

### 3. Always-on-top, non-activating (macOS)
- `NSWindow.level = .floating` (or `.statusBar`) for topmost; a non-activating
  panel (`NSWindowStyleMask.nonactivatingPanel` / `NSPanel`) so it never steals
  focus — the macOS equivalent of `WS_EX_NOACTIVATE`.
- hide/show stays as the Tauri cross-platform mechanism.

### 4. Adaptive text capture (macOS) — most complex
- `ScreenCaptureKit` (12.3+) or `CGWindowListCreateImage` to grab the region
  behind the overlay; average luminance feeds the existing `decide_theme`.
- Requires the **Screen Recording permission** (TCC prompt). Must handle the
  not-yet-granted state gracefully (fall back to the static text-shadow).

### 5. Signing / notarization / distribution
- Apple Developer account ($99/yr). Sign + notarize the `.app`/`.dmg` in CI
  (`codesign` + `notarytool`) so Gatekeeper accepts it.
- Without it, document the right-click-Open workaround in the README (like the
  Windows unsigned note).

## Constraints / risks

- **No Mac to verify** — everything above is build-blind until that changes. Do
  not ship a macOS release that hasn't run on a real Mac.
- Screen Recording permission UX is intrusive; adaptive text may be opt-in on
  macOS.
- Apple Developer cost + notarization tooling needed for a clean install.

## Out of scope (this doc)

- Implementation of any sub-project (deferred).
- Linux support.
- Multi-AI provider support (separate future spec).
