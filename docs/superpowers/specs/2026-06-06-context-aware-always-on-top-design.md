# Context-Aware Always-on-Top + Settings Page — Design

Date: 2026-06-06
Status: Approved (pending spec review)

## Problem

The overlay is always-on-top, so it floats over *every* app — annoying when the
user is in a browser, editor, etc. They only want it pinned while they're in a
terminal (where Claude Code runs) or the Claude Desktop app. When they switch to
any other app, the overlay should drop behind; when they switch back, it should
re-pin automatically.

## Goal

- **AUTO mode (default):** poll the foreground window; if its process is in an
  allowlist (or is the overlay/settings window itself), keep the overlay
  always-on-top; otherwise turn always-on-top off.
- **PINNED mode:** always-on-top at all times (the old behavior).
- A **Settings window** to switch mode and edit the allowlist; both persist.

Default allowlist (lowercased exe names):
`windowsterminal.exe`, `powershell.exe`, `pwsh.exe`, `claude.exe`, `cmd.exe`,
`conhost.exe`.

Windows-only (Win32 foreground APIs).

## Architecture

A Rust background task polls the foreground window every ~400ms, decides whether
to pin, and calls `window.set_always_on_top(bool)` only when the value changes.
Mode + allowlist live in the persisted `AppConfig` (shared `Arc<Mutex<>>`), so
the Settings UI and the footer toggle mutate the same config the watcher reads.

## Components

### 1. Config (`src-tauri/src/types.rs`, `config.rs`)

Add an enum and two fields to `AppConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AotMode {
    Auto,
    Pinned,
}

pub struct AppConfig {
    pub plan: Plan,
    #[serde(default = "default_aot_mode")]
    pub aot_mode: AotMode,
    #[serde(default = "default_aot_allowlist")]
    pub aot_allowlist: Vec<String>,
}
```

- `default_aot_mode() -> AotMode::Auto`.
- `default_aot_allowlist() -> Vec<String>` = the 6 names above.
- Implement `Default for AppConfig` manually (plan `Unknown`, mode `Auto`,
  allowlist = the 6) instead of deriving, so a missing config file and the
  `#[serde(default = ...)]` fallbacks all yield the intended defaults.
- `#[serde(default)]` on the new fields keeps old `{ "plan": ... }` config files
  loading (they backfill mode/allowlist).
- The existing `config.rs` test literal `AppConfig { plan: Plan::Max50 }` must
  become `AppConfig { plan: Plan::Max50, ..Default::default() }`.

### 2. Always-on-top watcher (`src-tauri/src/aot_watcher.rs`, new)

```rust
pub fn should_pin(mode: &AotMode, allowlist: &[String], fg_name: &str, fg_is_self: bool) -> bool
```
Pure, testable:
- `Pinned` → `true`.
- `Auto` → `fg_is_self || allowlist.iter().any(|a| a.eq_ignore_ascii_case(fg_name))`.

```rust
#[cfg(windows)]
fn foreground_exe(): Option<(u32 /*pid*/, String /*exe basename*/)>
```
Win32: `GetForegroundWindow` → `GetWindowThreadProcessId` → `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW` → take the basename after the last `\`/`/`. Returns `None` if any step fails. A `#[cfg(not(windows))]` stub returns `None`.

```rust
pub fn start_aot_watcher(app: AppHandle, config: Arc<Mutex<AppConfig>>)
```
Spawns a 400ms loop:
- get the `"main"` window (skip tick if absent);
- snapshot `(aot_mode, aot_allowlist)` from `config` (clone, release lock);
- compute `pin`:
  - `Pinned` → `true`;
  - `Auto` + `foreground_exe() == Some((pid, name))` → `should_pin(mode, allowlist, &name, pid == std::process::id())`;
  - `Auto` + `foreground_exe() == None` → skip this tick (keep previous);
- if `pin` differs from the last applied value, call `win.set_always_on_top(pin)` and remember it.

Never panics; all Win32 calls are best-effort.

### 3. Backend wiring (`src-tauri/src/lib.rs`)

- `mod aot_watcher;`
- In `setup`, after the existing background tasks, call
  `aot_watcher::start_aot_watcher(app.handle().clone(), config_arc.clone());`
  (the same `config_arc` that's in `AppState`, so command mutations are seen).
- **Remove** the `set_always_on_top` command and its `invoke_handler`
  registration (the watcher now owns always-on-top; the footer no longer calls
  it).
- Add commands (registered in `invoke_handler`):
  - `set_aot_mode(mode: String, state) -> Result<(), String>` — `"pinned"` →
    `AotMode::Pinned`, else `Auto`; write to `state.config` and persist via
    `config::save_config`.
  - `set_aot_allowlist(list: Vec<String>, state) -> Result<(), String>` —
    normalize (`trim().to_lowercase()`, drop empties), store, persist.
  - `open_settings(app)` — thin command that calls the shared helper below.

Add a shared helper in `lib.rs` (used by both the command and the tray):

```rust
pub fn open_settings_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_focus();
        return;
    }
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("PC Token Monitor — Settings")
    .inner_size(380.0, 440.0)
    .resizable(true)
    .build();
}
```

### 4. Tray menu (`src-tauri/src/tray.rs`)

Replace the `Always on Top` check item with a `Settings` menu item
(`id = "settings"`). In `on_menu_event`, handle `"settings"` by calling
`crate::open_settings_window(app)` (the shared helper). Keep `Show / Hide` and
`Quit`. (The `always_on_top` menu branch and the `CheckMenuItemBuilder` import
go away.)

### 5. Capabilities (`src-tauri/capabilities/default.json`)

Add `"settings"` to `windows` so the settings webview can invoke commands:
`"windows": ["main", "settings"]`.

### 6. Frontend types (`src/types.ts`)

```ts
export type AotMode = "auto" | "pinned";

export interface AppConfig {
  plan: Plan;
  aot_mode: AotMode;
  aot_allowlist: string[];
}
```

### 7. App routing (`src/App.tsx`)

At the top, branch on the window label:

```tsx
import { getCurrentWindow } from "@tauri-apps/api/window";
// ...
if (getCurrentWindow().label === "settings") {
  return <Settings />;
}
```

This runs before the first-run/overlay logic so the settings window never shows
the overlay or the first-run dialog.

### 8. Settings page (`src/components/Settings.tsx`, new)

- On mount, `invoke<FrontendState>("get_state")` to read `config.aot_mode` and
  `config.aot_allowlist` into local state.
- **Mode**: two-option control (AUTO / PINNED). Changing it calls
  `invoke("set_aot_mode", { mode })` immediately (live apply).
- **Allowlist**: render each entry with a remove (×) button; a text input + Add
  button appends. Every add/remove calls
  `invoke("set_aot_allowlist", { list })` with the full updated array (live
  apply). No explicit Save button.
- Normal opaque page (the settings window has decorations); simple dark styling
  consistent with the overlay.

### 9. Overlay footer (`src/components/OverlayWindow.tsx`)

- The left footer button shows the current mode: `[⊤ AUTO]` / `[⊤ PINNED]`.
  Clicking toggles via `invoke("set_aot_mode", { mode })` and flips an
  optimistic local state for the label. Seed it from
  `frontendState.config.aot_mode`.
- Add a small `⚙` button (next to `[×]`) that calls `invoke("open_settings")`.
- Remove the old `set_always_on_top` invoke.

## Data flow

```
[400ms loop] GetForegroundWindow → pid+exe
   → snapshot mode+allowlist from shared AppConfig
   → should_pin(...) → if changed: win.set_always_on_top(pin)

Settings UI / footer → invoke set_aot_mode|set_aot_allowlist
   → mutate shared AppConfig + persist → watcher reads next tick
```

## Error handling

- Any Win32 failure in `foreground_exe` → `None` → watcher keeps the previous
  pin state. No panics.
- `open_settings` builder failure is ignored (no crash); user can retry.
- Old config files without the new fields load via `#[serde(default)]`.

## Cross-window sync (accepted limitation)

Changing mode/allowlist in the Settings window does not push a live update to the
overlay footer's mode label (and vice-versa); each surface reads config on
mount / uses optimistic local state, and the overlay's `config` refreshes on the
next `usage-updated` event (≤30s). The *behavior* (watcher) updates within one
400ms tick regardless. No event bridge is built (YAGNI).

## Testing

- Rust unit tests for `should_pin`: Pinned → true regardless; Auto + allowlisted
  name (incl. different case) → true; Auto + non-listed → false; Auto +
  `fg_is_self` → true.
- Rust: a config test asserting defaults — `AppConfig::default().aot_mode ==
  AotMode::Auto` and the allowlist contains `"claude.exe"`; and a
  save/load roundtrip preserving a `Pinned` mode + custom allowlist.
- `foreground_exe` and the live watcher are verified manually.
- Existing frontend tests must stay green.
- Manual: focus a terminal / Claude Desktop → overlay pins; focus a browser →
  overlay drops behind; switch back → re-pins. PINNED mode stays on top over a
  browser. Edit the allowlist (add a browser exe → it now pins; remove it →
  drops). Open Settings from both the tray and the `⚙` button.

## Out of scope

- macOS/Linux foreground detection.
- Event-driven focus hooks (`SetWinEventHook`) — polling is enough.
- Live config push between windows.
