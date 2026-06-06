# Context-Aware Always-on-Top + Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-manage the overlay's always-on-top based on the focused app (AUTO mode, default) with a PINNED override, plus a Settings window to switch mode and edit the app allowlist.

**Architecture:** A 400ms Rust task polls the foreground window's exe name; a pure `should_pin` decides whether to pin; the watcher calls `window.set_always_on_top` only on change. Mode + allowlist live in the persisted `AppConfig` (shared `Arc<Mutex<>>`) mutated by Tauri commands from a second "settings" webview and the overlay footer.

**Tech Stack:** Tauri 2 (Rust), `windows` crate (Win32 foreground APIs), React 19 + TypeScript, Tailwind, Vitest + Rust `#[test]`.

**Spec:** `docs/superpowers/specs/2026-06-06-context-aware-always-on-top-design.md`

---

## Environment notes (for the executor)

- The `npm run tauri dev` server is **stopped**, and no repo build is running, so cargo has no lock contention. Run all `cargo`/`npm` commands in the **foreground** with a long timeout (a cold build can take a few minutes). Do not background them.
- Rust tests/builds use `--no-default-features`.
- Frontend tests: `npm test -- --run` (forces a single Vitest run).
- For the manual-verification steps, start the app with `npm run tauri dev` (the currently *installed* app is an older build without this feature).

## File Structure

- Modify `src-tauri/src/types.rs` — `AotMode` enum; extend `AppConfig`; defaults.
- Modify `src-tauri/src/config.rs` — fix test literal; add config tests.
- Create `src-tauri/src/aot_watcher.rs` — `should_pin`, `foreground_exe`, `start_aot_watcher`.
- Modify `src-tauri/src/lib.rs` — `mod aot_watcher`; start watcher; helper `open_settings_window`; new commands; `invoke_handler`.
- Modify `src-tauri/src/tray.rs` — replace "Always on Top" with "Settings".
- Modify `src-tauri/Cargo.toml` — add Win32 feature flags.
- Modify `src-tauri/capabilities/default.json` — allow the `settings` window.
- Modify `src/types.ts` — `AotMode`, extend `AppConfig`.
- Modify `src/App.tsx` — route by window label.
- Create `src/components/Settings.tsx` — settings UI.
- Modify `src/components/OverlayWindow.tsx` — footer mode toggle + ⚙, drop old toggle.

---

### Task 1: Config — AotMode + AppConfig fields (Rust, TDD)

**Files:** Modify `src-tauri/src/types.rs`, `src-tauri/src/config.rs`

- [ ] **Step 1: Add `AotMode` + extend `AppConfig` in `types.rs`**

Replace the existing `AppConfig` struct and its `Default` impl (currently `pub struct AppConfig { pub plan: Plan }` + `impl Default`) with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AotMode {
    Auto,
    Pinned,
}

fn default_aot_mode() -> AotMode {
    AotMode::Auto
}

fn default_aot_allowlist() -> Vec<String> {
    vec![
        "windowsterminal.exe".to_string(),
        "powershell.exe".to_string(),
        "pwsh.exe".to_string(),
        "claude.exe".to_string(),
        "cmd.exe".to_string(),
        "conhost.exe".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub plan: Plan,
    #[serde(default = "default_aot_mode")]
    pub aot_mode: AotMode,
    #[serde(default = "default_aot_allowlist")]
    pub aot_allowlist: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            plan: Plan::Unknown,
            aot_mode: default_aot_mode(),
            aot_allowlist: default_aot_allowlist(),
        }
    }
}
```

- [ ] **Step 2: Fix the existing config test + add new tests in `config.rs`**

Change the import line `use crate::types::{AppConfig, Plan};` to:

```rust
use crate::types::{AppConfig, AotMode, Plan};
```

In `save_and_load_roundtrip`, change the struct literal
`let config = AppConfig { plan: Plan::Max50 };` to:

```rust
        let config = AppConfig { plan: Plan::Max50, ..Default::default() };
```

Add these two tests inside the `tests` module:

```rust
    #[test]
    fn default_config_is_auto_with_default_allowlist() {
        let c = AppConfig::default();
        assert_eq!(c.aot_mode, AotMode::Auto);
        assert!(c.aot_allowlist.iter().any(|a| a == "claude.exe"));
    }

    #[test]
    fn save_and_load_preserves_aot_settings() {
        let (_dir, path) = temp_config_path();
        let config = AppConfig {
            plan: Plan::Pro,
            aot_mode: AotMode::Pinned,
            aot_allowlist: vec!["foo.exe".to_string()],
        };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(loaded.aot_mode, AotMode::Pinned);
        assert_eq!(loaded.aot_allowlist, vec!["foo.exe".to_string()]);
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --no-default-features config`
Expected: all `config::tests::*` pass (the existing 4 + 2 new).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/config.rs
git commit -m "feat: add aot_mode + aot_allowlist to AppConfig

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `should_pin` pure decision (Rust, TDD)

**Files:** Create `src-tauri/src/aot_watcher.rs`; Modify `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `aot_watcher.rs` with `should_pin` + tests**

```rust
//! Watches the foreground window and pins/unpins the overlay accordingly.

use crate::types::AotMode;

/// Whether the overlay should be always-on-top given the current mode, the
/// allowlist, and the foreground app's exe name. `fg_is_self` is true when the
/// foreground window belongs to our own process (so dragging the overlay or
/// using Settings keeps it pinned).
pub fn should_pin(mode: &AotMode, allowlist: &[String], fg_name: &str, fg_is_self: bool) -> bool {
    match mode {
        AotMode::Pinned => true,
        AotMode::Auto => {
            fg_is_self || allowlist.iter().any(|a| a.eq_ignore_ascii_case(fg_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Vec<String> {
        vec!["claude.exe".to_string(), "powershell.exe".to_string()]
    }

    #[test]
    fn pinned_is_always_true() {
        assert!(should_pin(&AotMode::Pinned, &[], "chrome.exe", false));
    }

    #[test]
    fn auto_allowlisted_is_true_case_insensitive() {
        assert!(should_pin(&AotMode::Auto, &list(), "Claude.exe", false));
        assert!(should_pin(&AotMode::Auto, &list(), "POWERSHELL.EXE", false));
    }

    #[test]
    fn auto_not_listed_is_false() {
        assert!(!should_pin(&AotMode::Auto, &list(), "chrome.exe", false));
    }

    #[test]
    fn auto_self_is_true() {
        assert!(should_pin(&AotMode::Auto, &list(), "chrome.exe", true));
    }
}
```

- [ ] **Step 2: Register the module in `lib.rs`**

Add alongside the other `mod` lines near the top:

```rust
mod aot_watcher;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --no-default-features aot_watcher`
Expected: 4 `aot_watcher::tests::*` pass. (A `dead_code` warning for `should_pin` is expected until Task 3 — ignore.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/aot_watcher.rs src-tauri/src/lib.rs
git commit -m "feat: should_pin decision for context-aware always-on-top

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Foreground sampling + watcher task (Rust)

**Files:** Modify `src-tauri/Cargo.toml`, `src-tauri/src/aot_watcher.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add Win32 feature flags to the windows dep**

In `src-tauri/Cargo.toml`, the existing block is:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
] }
```

Add two features so it becomes:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Threading",
] }
```

- [ ] **Step 2: Add imports + foreground sampler + watcher to `aot_watcher.rs`**

Add these imports at the top (below the `//!` doc comment, above `use crate::types::AotMode;`):

```rust
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::types::AppConfig;
```

Append below `should_pin` (above the `#[cfg(test)]` module):

```rust
/// The foreground window's process id and exe basename (lowercased by caller as
/// needed). `None` if it can't be determined.
#[cfg(windows)]
fn foreground_exe() -> Option<(u32, String)> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        if ok.is_err() {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        let name = full
            .rsplit(|c| c == '\\' || c == '/')
            .next()
            .unwrap_or(&full)
            .to_string();
        Some((pid, name))
    }
}

#[cfg(not(windows))]
fn foreground_exe() -> Option<(u32, String)> {
    None
}

/// Poll the foreground window every 400ms and pin/unpin the overlay.
pub fn start_aot_watcher(app: AppHandle, config: Arc<Mutex<AppConfig>>) {
    tauri::async_runtime::spawn(async move {
        let self_pid = std::process::id();
        let mut applied: Option<bool> = None;
        loop {
            tokio::time::sleep(Duration::from_millis(400)).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };
            let (mode, allowlist) = {
                let c = config.lock().unwrap();
                (c.aot_mode.clone(), c.aot_allowlist.clone())
            };

            let pin = match (&mode, foreground_exe()) {
                (AotMode::Pinned, _) => true,
                (AotMode::Auto, Some((pid, name))) => {
                    should_pin(&mode, &allowlist, &name, pid == self_pid)
                }
                // Can't read the foreground app: keep the current state.
                (AotMode::Auto, None) => continue,
            };

            if applied != Some(pin) {
                let _ = win.set_always_on_top(pin);
                applied = Some(pin);
            }
        }
    });
}
```

- [ ] **Step 3: Start the watcher in `lib.rs` setup**

In the `.setup(|app| { ... })` closure, after the existing `start_poll_loop(...)` call, add:

```rust
            aot_watcher::start_aot_watcher(app_handle.clone(), config_arc.clone());
```

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds with no errors. If a `windows`-crate signature differs, fix minimally per the compiler (likely points: `GetForegroundWindow` returns `HWND` whose `.0` is a `*mut c_void` so the null check is `hwnd.0.is_null()`; `GetWindowThreadProcessId(hwnd, Some(&mut pid))`; `QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(...), &mut len)` returns `windows::core::Result<()>`). Keep the algorithm unchanged.

- [ ] **Step 5: Run all Rust tests**

Run: `cd src-tauri && cargo test --no-default-features`
Expected: all pass (previous suite + the new `aot_watcher`/`config` tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/aot_watcher.rs src-tauri/src/lib.rs
git commit -m "feat: poll foreground window and auto manage always-on-top

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Settings window helper + commands + capability (Rust)

**Files:** Modify `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`

- [ ] **Step 1: Import AotMode in `lib.rs`**

Change the types import line
`use types::{AppConfig, DataSource, FrontendState, Plan, UsageData, WindowUsage};` to add `AotMode`:

```rust
use types::{AotMode, AppConfig, DataSource, FrontendState, Plan, UsageData, WindowUsage};
```

- [ ] **Step 2: Add the helper + commands in `lib.rs`**

Add near the other command fns (e.g. after `set_always_on_top`):

```rust
pub fn open_settings_window(app: &AppHandle) {
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

#[tauri::command]
fn set_aot_mode(mode: String, state: State<AppState>) -> Result<(), String> {
    let m = if mode == "pinned" {
        AotMode::Pinned
    } else {
        AotMode::Auto
    };
    let mut config = state.config.lock().unwrap();
    config.aot_mode = m;
    config::save_config(&config::config_path(), &config)
}

#[tauri::command]
fn set_aot_allowlist(list: Vec<String>, state: State<AppState>) -> Result<(), String> {
    let cleaned: Vec<String> = list
        .into_iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let mut config = state.config.lock().unwrap();
    config.aot_allowlist = cleaned;
    config::save_config(&config::config_path(), &config)
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}
```

- [ ] **Step 3: Register the new commands**

Change the `invoke_handler` line to add the three new commands (keep `set_always_on_top` for now; Task 8 removes it together with its frontend caller):

```rust
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_plan,
            set_always_on_top,
            set_aot_mode,
            set_aot_allowlist,
            open_settings
        ])
```

- [ ] **Step 4: Allow the settings window in capabilities**

In `src-tauri/capabilities/default.json`, change `"windows": ["main"]` to:

```json
  "windows": ["main", "settings"],
```

- [ ] **Step 5: Build**

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds with no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat: settings window + aot config commands

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: Tray "Settings" menu item (Rust)

**Files:** Modify `src-tauri/src/tray.rs`

- [ ] **Step 1: Drop the CheckMenuItem import**

Change `use tauri::menu::{Menu, MenuItemBuilder, CheckMenuItemBuilder};` to:

```rust
use tauri::menu::{Menu, MenuItemBuilder};
```

- [ ] **Step 2: Replace the menu items**

Replace the current menu-building lines:

```rust
    let show_hide = MenuItemBuilder::new("Show / Hide").id("show_hide").build(app)?;
    let always_on_top = CheckMenuItemBuilder::new("Always on Top")
        .id("always_on_top")
        .checked(true)
        .build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = Menu::with_items(app, &[&show_hide, &always_on_top, &quit])?;
```

with:

```rust
    let show_hide = MenuItemBuilder::new("Show / Hide").id("show_hide").build(app)?;
    let settings = MenuItemBuilder::new("Settings").id("settings").build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = Menu::with_items(app, &[&show_hide, &settings, &quit])?;
```

- [ ] **Step 3: Replace the menu-event branch**

In the `.on_menu_event(|app, event| match event.id().as_ref() { ... })`, replace the `"always_on_top"` arm:

```rust
            "always_on_top" => {
                let win = app.get_webview_window("main").unwrap();
                let current = win.is_always_on_top().unwrap_or(false);
                let _ = win.set_always_on_top(!current);
            }
```

with:

```rust
            "settings" => {
                crate::open_settings_window(app);
            }
```

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds with no errors (no unused-import warning for CheckMenuItemBuilder).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tray.rs
git commit -m "feat: tray Settings item opens the settings window

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Frontend types + window routing (TypeScript)

**Files:** Modify `src/types.ts`, `src/App.tsx`

- [ ] **Step 1: Extend `src/types.ts`**

Replace:

```ts
export type Plan = "Pro" | "Max50" | "Max200" | "Unknown";

export interface AppConfig {
  plan: Plan;
}
```

with:

```ts
export type Plan = "Pro" | "Max50" | "Max200" | "Unknown";

export type AotMode = "auto" | "pinned";

export interface AppConfig {
  plan: Plan;
  aot_mode: AotMode;
  aot_allowlist: string[];
}
```

- [ ] **Step 2: Route by window label in `src/App.tsx`**

Replace the whole file with (extracts the overlay logic into `OverlayApp` so hooks are never called conditionally, and routes the `settings` window to `<Settings />`):

```tsx
import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUsageStore } from "./stores/usageStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { OverlayWindow } from "./components/OverlayWindow";
import { FirstRunDialog } from "./components/FirstRunDialog";
import { Settings } from "./components/Settings";

export default function App() {
  if (getCurrentWindow().label === "settings") {
    return <Settings />;
  }
  return <OverlayApp />;
}

function OverlayApp() {
  useTauriEvents();
  const frontendState = useUsageStore((s) => s.frontendState);
  const [showFirstRun, setShowFirstRun] = useState(false);

  useEffect(() => {
    if (frontendState?.config.plan === "Unknown") {
      setShowFirstRun(true);
    }
  }, [frontendState?.config.plan]);

  if (showFirstRun) {
    return <FirstRunDialog onDone={() => setShowFirstRun(false)} />;
  }

  return <OverlayWindow />;
}
```

- [ ] **Step 3: Verify (will fail to compile until Task 7 creates Settings)**

`Settings` does not exist yet, so do NOT run the build here. Proceed to Task 7, which creates it; the combined result is verified at the end of Task 7. Commit this task's files together with Task 7 to avoid a broken intermediate commit — i.e. skip the commit here.

> Note: Tasks 6 and 7 are committed together at the end of Task 7.

---

### Task 7: Settings page (TypeScript)

**Files:** Create `src/components/Settings.tsx`

- [ ] **Step 1: Create `src/components/Settings.tsx`**

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AotMode, FrontendState } from "../types";

export function Settings() {
  const [mode, setMode] = useState<AotMode>("auto");
  const [allowlist, setAllowlist] = useState<string[]>([]);
  const [newEntry, setNewEntry] = useState("");

  useEffect(() => {
    invoke<FrontendState>("get_state")
      .then((s) => {
        setMode(s.config.aot_mode);
        setAllowlist(s.config.aot_allowlist);
      })
      .catch(console.error);
  }, []);

  function changeMode(next: AotMode) {
    setMode(next);
    invoke("set_aot_mode", { mode: next }).catch(console.error);
  }

  function commitList(next: string[]) {
    setAllowlist(next);
    invoke("set_aot_allowlist", { list: next }).catch(console.error);
  }

  function addEntry() {
    const v = newEntry.trim().toLowerCase();
    if (!v || allowlist.includes(v)) {
      setNewEntry("");
      return;
    }
    commitList([...allowlist, v]);
    setNewEntry("");
  }

  function removeEntry(name: string) {
    commitList(allowlist.filter((a) => a !== name));
  }

  return (
    <div className="min-h-screen bg-[#0a0a0a] text-[#e8e8e8] font-mono text-sm p-4 flex flex-col gap-4">
      <h1 className="text-base font-bold">Settings</h1>

      <section className="flex flex-col gap-2">
        <h2 className="text-xs text-[#888] uppercase tracking-widest">Always on top</h2>
        <div className="flex gap-2">
          <button
            onClick={() => changeMode("auto")}
            className={`px-3 py-1 rounded border ${
              mode === "auto" ? "border-[#00d4ff] text-[#00d4ff]" : "border-[#333] text-[#888]"
            }`}
          >
            AUTO
          </button>
          <button
            onClick={() => changeMode("pinned")}
            className={`px-3 py-1 rounded border ${
              mode === "pinned" ? "border-[#00d4ff] text-[#00d4ff]" : "border-[#333] text-[#888]"
            }`}
          >
            PINNED
          </button>
        </div>
        <p className="text-[11px] text-[#666]">
          AUTO: stay on top only while one of the allowed apps is focused. PINNED: always on top.
        </p>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-xs text-[#888] uppercase tracking-widest">Allowed apps (AUTO)</h2>
        <ul className="flex flex-col gap-1">
          {allowlist.map((name) => (
            <li
              key={name}
              className="flex items-center justify-between bg-[#161616] px-2 py-1 rounded"
            >
              <span>{name}</span>
              <button
                onClick={() => removeEntry(name)}
                className="text-[#888] hover:text-[#ff5555]"
                aria-label={`remove ${name}`}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
        <div className="flex gap-2">
          <input
            value={newEntry}
            onChange={(e) => setNewEntry(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") addEntry();
            }}
            placeholder="e.g. code.exe"
            className="flex-1 bg-[#161616] border border-[#333] rounded px-2 py-1 outline-none"
          />
          <button
            onClick={addEntry}
            className="px-3 py-1 rounded border border-[#333] hover:border-[#00d4ff]"
          >
            Add
          </button>
        </div>
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Run frontend tests + typecheck**

Run: `npm test -- --run`
Expected: all existing tests still pass (18). No TypeScript/compile errors (App.tsx now resolves `Settings`).

- [ ] **Step 3: Commit (Tasks 6 + 7 together)**

```bash
git add src/types.ts src/App.tsx src/components/Settings.tsx
git commit -m "feat: settings window page + window-label routing

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: Overlay footer mode toggle + ⚙, remove old toggle (TypeScript + Rust)

**Files:** Modify `src/components/OverlayWindow.tsx`, `src-tauri/src/lib.rs`

READ `src/components/OverlayWindow.tsx` first — it already has the adaptive-theme listener and `pal` palette, the `pointer-events-none` drag setup, and a footer with an "ALWAYS ON TOP" button (`toggleAlwaysOnTop` → `invoke("set_always_on_top", ...)`) and a `[×]` button.

- [ ] **Step 1: Replace the always-on-top state with aot-mode state**

Add `AotMode` to the types import:

```tsx
import { overlayPalette, type BgTheme } from "../overlayPalette";
```
becomes
```tsx
import { overlayPalette, type BgTheme } from "../overlayPalette";
import type { AotMode } from "../types";
```

Replace:

```tsx
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
```

with:

```tsx
  const [aotMode, setAotMode] = useState<AotMode>("auto");

  useEffect(() => {
    if (frontendState?.config.aot_mode) {
      setAotMode(frontendState.config.aot_mode);
    }
  }, [frontendState?.config.aot_mode]);
```

(`frontendState` is already destructured at the top of the component.)

- [ ] **Step 2: Replace the toggle handler + add the settings opener**

Replace the `toggleAlwaysOnTop` function:

```tsx
  function toggleAlwaysOnTop() {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    invoke("set_always_on_top", { value: next });
  }
```

with:

```tsx
  function toggleMode() {
    const next: AotMode = aotMode === "auto" ? "pinned" : "auto";
    setAotMode(next);
    invoke("set_aot_mode", { mode: next }).catch(() => {});
  }

  function openSettings() {
    invoke("open_settings").catch(() => {});
  }
```

- [ ] **Step 3: Update the footer buttons**

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

with:

```tsx
        <button
          onClick={toggleMode}
          className="font-mono text-[9px] tracking-widest transition-colors"
          style={{ color: aotMode === "pinned" ? pal.label5 : "var(--ov-muted)" }}
        >
          [⊤ {aotMode === "pinned" ? "PINNED" : "AUTO"}]
        </button>
```

Replace the `[×]` button block with a settings `⚙` button followed by the `[×]` button:

```tsx
        <div className="flex items-center gap-2">
          <button
            onClick={openSettings}
            className="font-mono text-[9px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="settings"
          >
            ⚙
          </button>
          <button
            onClick={minimize}
            className="font-mono text-[9px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
          >
            [×]
          </button>
        </div>
```

- [ ] **Step 4: Remove the now-unused `set_always_on_top` command (Rust)**

In `src-tauri/src/lib.rs`, delete the command fn:

```rust
#[tauri::command]
fn set_always_on_top(value: bool, app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(value);
    }
}
```

and remove `set_always_on_top,` from the `invoke_handler!` list, leaving:

```rust
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_plan,
            set_aot_mode,
            set_aot_allowlist,
            open_settings
        ])
```

- [ ] **Step 5: Build + test**

Run: `npm test -- --run`
Expected: all frontend tests pass (18).

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds with no errors and no `set_always_on_top` dead-code warning.

- [ ] **Step 6: Commit**

```bash
git add src/components/OverlayWindow.tsx src-tauri/src/lib.rs
git commit -m "feat: footer AUTO/PINNED toggle + settings button; drop manual set_always_on_top

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final Verification

- [ ] `cd src-tauri && cargo test --no-default-features` → all Rust tests pass (previous + 2 config + 4 aot_watcher).
- [ ] `npm test -- --run` → all frontend tests pass (18).
- [ ] Manual (start `npm run tauri dev`):
  - Focus a terminal (Windows Terminal / PowerShell) or Claude Desktop → overlay is on top.
  - Focus a browser/other app → overlay drops behind.
  - Switch back → overlay re-pins (within ~0.4s).
  - Footer button toggles `AUTO`/`PINNED`; in `PINNED` the overlay stays on top over a browser.
  - `⚙` opens the Settings window; the tray "Settings" item opens/focuses it too.
  - In Settings: switch AUTO/PINNED (takes effect); add an app (e.g. `chrome.exe`) → focusing Chrome now pins; remove it → Chrome no longer pins.
  - Dragging the overlay itself doesn't make it drop (fg_is_self).
- [ ] `git status` clean (except untracked `scripts/`).

## Notes / Risks

- `windows` crate signatures: if `cargo build` errors in `foreground_exe`, adjust per the compiler (the most likely spots are noted in Task 3 Step 4). Don't change the algorithm.
- Single global decision @400ms: focus changes have ≤0.4s latency — acceptable, no event hook.
- Cross-window live sync is intentionally absent (see spec): the overlay footer's mode label refreshes on the next `usage-updated` event; the watcher behavior is always current.
