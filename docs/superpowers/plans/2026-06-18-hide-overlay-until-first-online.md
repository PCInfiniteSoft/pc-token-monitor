# Hide Overlay Until First Online (one-shot) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a cold boot, keep the overlay hidden in the system tray while it is OFFLINE; show it once live usage data first arrives, then resume normal behavior for the session.

**Architecture:** A single setup-local `Arc<AtomicBool>` (`started_online`, starts `false`) gates visibility. The poll loop flips it to `true` the first time it produces a live (non-fallback) data source. The AOT watcher — already the sole owner of the main window's visibility — forces the window hidden at the top of each loop iteration until the flag is set, then runs its existing pin logic unchanged. The main window starts with `visible: false` to avoid a boot flash.

**Tech Stack:** Rust, Tauri 2, tokio async runtime, `std::sync::atomic`.

## Global Constraints

- Target platform: Windows (this is a Windows desktop app). Non-Windows paths are stubbed and out of scope.
- One-shot semantics: once online is achieved, never re-hide for the rest of the session — even if the network later drops.
- The AOT watcher remains the only code that calls `show()`/`hide()` on the `main` window. The poll loop only sets the flag; it must not touch window visibility.
- Do not add the flag to `AppState` — no Tauri command needs it. Keep it a `setup`-local `Arc` cloned into the poll loop and the watcher.

---

### Task 1: Online detection + `started_online` flag in the poll loop

**Files:**
- Modify: `src-tauri/src/lib.rs` (add `source_is_live` helper, extend `start_poll_loop` signature, set flag in loop, create the Arc in `setup` and pass it to the poll loop, add a `#[cfg(test)]` module)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `fn source_is_live(source: &DataSource) -> bool` — `true` when `*source != DataSource::JsonlFallback`.
  - `start_poll_loop(app: AppHandle, state: Arc<Mutex<Option<UsageData>>>, config: Arc<Mutex<AppConfig>>, user_name: Option<String>, started_online: Arc<AtomicBool>)` — new 5th parameter.
  - A `setup`-local binding `let started_online = Arc::new(AtomicBool::new(false));` available to clone for Task 2.

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src-tauri/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_source_is_online() {
        assert!(source_is_live(&DataSource::OAuth));
    }

    #[test]
    fn jsonl_fallback_is_offline() {
        assert!(!source_is_live(&DataSource::JsonlFallback));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test source` 
Expected: FAIL to compile — `cannot find function `source_is_live` in this scope`.

- [ ] **Step 3: Add the `source_is_live` helper**

Add near `dominant_percent` (after line 86 in `src-tauri/src/lib.rs`):

```rust
/// Whether a data source represents live (online) data. Fallback JSONL data is
/// what the OFFLINE badge reflects, so anything else counts as online.
fn source_is_live(source: &DataSource) -> bool {
    *source != DataSource::JsonlFallback
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test source`
Expected: PASS (`live_source_is_online`, `jsonl_fallback_is_offline`).

- [ ] **Step 5: Add the atomic import**

In `src-tauri/src/lib.rs`, update the `std::sync` import (line 11):

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
```

- [ ] **Step 6: Extend `start_poll_loop` signature and set the flag**

Change the signature (lib.rs line 88):

```rust
fn start_poll_loop(
    app: AppHandle,
    state: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
    user_name: Option<String>,
    started_online: Arc<AtomicBool>,
) {
```

Inside the loop, in the existing `if let Some(ref u) = usage {` block (currently starting at line 147), add the flag set as the first statement of the block:

```rust
            if let Some(ref u) = usage {
                if source_is_live(&u.source) {
                    started_online.store(true, Ordering::SeqCst);
                }
                let pct = dominant_percent(u);
                tray::update_tray_icon(&app, pct);

                let cfg = config.lock().unwrap().clone();
                let frontend = FrontendState {
                    usage: Some(u.clone()),
                    config: cfg,
                    user_name: user_name.clone(),
                };
                let _ = app.emit("usage-updated", frontend);
            }
```

- [ ] **Step 7: Create the Arc in `setup` and pass it to the poll loop**

In the `setup` closure, just before the `start_poll_loop(...)` call (currently around line 214-222), add the binding and pass a clone:

```rust
            let started_online = Arc::new(AtomicBool::new(false));

            let app_handle = app.handle().clone();
            let usage_for_poll = usage_arc.clone();
            let config_for_poll = config_arc.clone();
            start_poll_loop(
                app_handle.clone(),
                usage_for_poll,
                config_for_poll,
                user_name.clone(),
                started_online.clone(),
            );
```

- [ ] **Step 8: Verify it compiles and tests pass**

Run: `cd src-tauri && cargo test source`
Expected: compiles clean; both tests PASS. (`started_online` is created and used by the poll loop; the watcher wiring comes in Task 2.)

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: track first-online state in the poll loop"
```

---

### Task 2: Gate overlay visibility in the AOT watcher + hidden boot

**Files:**
- Modify: `src-tauri/src/aot_watcher.rs` (extend `start_aot_watcher` signature, gate visibility at top of the loop, add atomic import)
- Modify: `src-tauri/src/lib.rs` (pass `started_online` clone into `start_aot_watcher`)
- Modify: `src-tauri/tauri.conf.json` (`main` window `visible: true` → `false`)

**Interfaces:**
- Consumes: `started_online: Arc<AtomicBool>` created in Task 1's `setup` block.
- Produces: `start_aot_watcher(app: AppHandle, config: Arc<Mutex<AppConfig>>, started_online: Arc<AtomicBool>)` — new 3rd parameter.

- [ ] **Step 1: Add the atomic import to the watcher**

In `src-tauri/src/aot_watcher.rs`, update the imports (line 3):

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
```

- [ ] **Step 2: Extend `start_aot_watcher` signature**

Change the signature (aot_watcher.rs line 157):

```rust
pub fn start_aot_watcher(
    app: AppHandle,
    config: Arc<Mutex<AppConfig>>,
    started_online: Arc<AtomicBool>,
) {
```

- [ ] **Step 3: Gate visibility at the top of the loop**

Inside the loop, immediately after the existing `let Some(win) = app.get_webview_window("main") else { continue; };` block (currently lines 177-179), insert the gate before the mode/allowlist read:

```rust
            let Some(win) = app.get_webview_window("main") else {
                continue;
            };

            // Until live data first arrives, keep the overlay hidden in the tray
            // (one-shot). Once online, this gate is permanently open and the
            // normal pin logic below runs unchanged.
            if !started_online.load(Ordering::SeqCst) {
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                }
                continue;
            }

            let (mode, allowlist) = {
                let c = config.lock().unwrap();
                (c.aot_mode.clone(), c.aot_allowlist.clone())
            };
```

- [ ] **Step 4: Pass the flag from `setup`**

In `src-tauri/src/lib.rs`, update the `start_aot_watcher` call (currently line 223):

```rust
            aot_watcher::start_aot_watcher(
                app_handle.clone(),
                config_arc.clone(),
                started_online.clone(),
            );
```

- [ ] **Step 5: Start the main window hidden**

In `src-tauri/tauri.conf.json`, change the `main` window's visibility (line 21):

```json
        "visible": false,
```

- [ ] **Step 6: Verify it compiles and all unit tests pass**

Run: `cd src-tauri && cargo test`
Expected: compiles clean; the 4 existing `should_pin` tests plus the 2 `source_is_live` tests PASS (6 total).

- [ ] **Step 7: Manual verification**

Per the spec's test plan:
1. **Offline boot:** disconnect network (or sign out of Claude), launch via `npm run tauri dev` (or the built exe) → overlay stays hidden, tray icon present, NO flash of the overlay window.
2. **Goes online:** reconnect → within ~1–2 poll cycles (poll loop sleeps 30s, so allow up to ~30s) the overlay appears per the current mode (Auto: when a monitored app is foreground/visible; Pinned: immediately).
3. **Online boot (regression):** with normal network, launch → overlay appears as before (Auto/Pinned), no regression.
4. **One-shot:** after data has loaded, kill the network → overlay does NOT hide.

> Note: the poll loop sleeps 30s between ticks, so "goes online" can take up to ~30s to reflect. This matches existing poll cadence; no change needed.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/aot_watcher.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "feat: hide overlay in tray until first online (one-shot)"
```

---

## Self-Review

**Spec coverage:**
- "Hide on offline boot" → Task 2 Step 3 gate + Step 5 hidden boot. ✓
- "Show on first online" → Task 1 sets flag on live source; Task 2 gate opens. ✓
- One-shot (no re-hide after online) → flag only ever set to `true`, never cleared. ✓
- No boot flash → `visible: false` (Task 2 Step 5). ✓
- Watcher remains sole visibility owner → poll loop only stores the flag, never shows/hides. ✓
- Never-online stays in tray → flag stays `false`, gate stays closed. ✓
- Existing `should_pin` tests unaffected → untouched. ✓

**Placeholder scan:** No TBD/TODO/"handle edge cases"; every code step shows complete code. ✓

**Type consistency:** `source_is_live(&DataSource)` defined in Task 1 and consumed only in Task 1. `started_online: Arc<AtomicBool>` parameter name and type identical across `start_poll_loop` (Task 1) and `start_aot_watcher` (Task 2). `Ordering::SeqCst` used for both store and load. ✓
