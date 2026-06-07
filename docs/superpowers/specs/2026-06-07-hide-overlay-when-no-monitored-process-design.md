# Hide overlay when no monitored process is running

**Date:** 2026-06-07
**Status:** Approved, ready for implementation

## Problem

In Auto mode the overlay shows when an allowlisted app (e.g. `claude.exe`,
`powershell.exe`) is foreground and hides otherwise. But when the user closes
all monitored apps, the foreground becomes a Windows shell surface
(`explorer.exe`). `start_aot_watcher` treats shell surfaces as neutral and
`continue`s, holding the last pinned state. If the overlay was visible when the
last app closed, it stays floating on top with nothing left to monitor.

## Goal

When no monitored process is running at all, hide the overlay (to the tray —
the tray icon already persists). Restore normal behavior once a monitored app
launches again.

## Scope

- **Auto mode only.** Pinned mode means "always on top by user choice" and is
  left untouched.
- "Monitored" = the `aot_allowlist` process names (the same list Auto mode
  already uses for the foreground check).
- End state is the existing `win.hide()` path. No new tray/taskbar work — the
  user wants it to disappear to the tray, which `hide()` already achieves.

## Design

### New helper: `any_monitored_running(allowlist) -> bool`

Enumerate running processes via the Win32 Toolhelp snapshot
(`CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` +
`Process32FirstW` / `Process32NextW`). For each `szExeFile` basename, compare
case-insensitively against `allowlist`. Return `true` on first match.

- Add Cargo feature `Win32_System_Diagnostics_ToolHelp` to the windows crate.
- Non-Windows stub returns `true` (no behavior change off Windows).
- Snapshot handle is closed via `CloseHandle` before returning.

### Throttle

Enumerating every process list each 200ms tick is wasteful. Check at most once
per ~1s and cache the result between checks:

- Keep `last_check: Instant` and `cached_running: bool` in the loop.
- Re-run `any_monitored_running` only when `>= 1s` since last check; otherwise
  reuse the cached value.
- Worst-case latency from closing the last app to the overlay hiding is ~1s,
  accepted.

### Watcher logic change (`start_aot_watcher`, Auto branch)

Before the foreground-based decision, in Auto mode:

```
if !monitored_running {        // cached, throttled
    pin = false;               // force hide regardless of foreground
} else {
    // existing foreground logic (self/shell -> continue/hold, else should_pin)
}
```

Pinned mode is unchanged (`pin = true`).

The existing show/hide tail is reused:

```
if pin && !visible { win.show() }
else if !pin && visible { win.hide() }
```

## Testing

- Unit-test the allowlist match logic where it can be isolated from the live
  snapshot (case-insensitive basename comparison). The raw Toolhelp enumeration
  is a thin Win32 wrapper, exercised manually.
- Manual: in Auto mode with the overlay visible, close all monitored apps ->
  overlay hides within ~1s; relaunch a monitored app -> overlay returns to
  normal Auto behavior.

## Non-goals

- No change to Pinned mode.
- No taskbar button / window-state work.
- No config/UI surface for this; it is implicit in Auto mode.
