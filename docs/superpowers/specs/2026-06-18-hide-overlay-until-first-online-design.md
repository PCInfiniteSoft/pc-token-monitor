# Hide overlay until first online (one-shot) — Design

**Date:** 2026-06-18
**Status:** Approved (pending implementation)

## Problem

On a cold boot, the overlay window appears immediately (`tauri.conf.json` has
`"visible": true`). If the machine can't reach the Claude usage API yet — no
network, or no logged-in token — the first poll falls back to local JSONL data
and the plan badge reads `[OFFLINE]` with 0% across the board. The user sees a
useless 0% overlay flash onto the screen before any real data exists.

Desired behavior: at first boot, if the status is OFFLINE, keep the overlay
hidden in the system tray. As soon as it goes online for the first time, show it
normally (per the existing Auto/Pinned always-on-top behavior).

## What "OFFLINE" means here

The `[OFFLINE]` badge is driven by `isOffline()` in `src/stores/usageStore.ts`:

```ts
isOffline: () => get().frontendState?.usage?.source === "jsonl_fallback",
```

`source` is set in the Rust poll loop (`src-tauri/src/lib.rs`). It is
`DataSource::JsonlFallback` only when **the very first poll fails to fetch live
data and there is no previously cached value** (no token, or the OAuth fetch
errored). Once any live fetch succeeds, `source` is a live value, and a later
fetch failure keeps the last known live value rather than reverting to fallback.

Consequence: `OFFLINE` can only appear at cold start, before the API is first
reachable. This is exactly the window of time we want to keep the overlay
hidden. After the first successful fetch, the overlay follows normal behavior
and never re-hides for transient offline — which matches the agreed one-shot
scope.

## Scope (agreed)

- **One-shot.** Hide the overlay only until the first time live data arrives.
  After that, normal Auto/Pinned behavior for the rest of the session.
- If the user never logs in / never gets online, the overlay stays in the tray
  for the whole session. It remains reachable via the tray icon (subject to the
  trade-off below).

## Design

A single shared atomic flag gates visibility. The AOT watcher remains the *sole*
owner of the main window's visibility — the poll loop only flips the flag, it
never calls `show()`/`hide()` itself. This avoids two code paths fighting over
window visibility.

### Components

1. **`started_online: Arc<AtomicBool>`** — initialized `false`.
   - Stored in `AppState` (`src-tauri/src/lib.rs`).
   - Cloned into both the poll loop and `aot_watcher::start_aot_watcher`.

2. **Poll loop (`start_poll_loop`, `src-tauri/src/lib.rs`)**
   - After computing `usage` for a tick, if `usage.source` is a live source
     (i.e. **not** `DataSource::JsonlFallback`), set `started_online` to `true`.
   - Use `store(true, Ordering::SeqCst)` unconditionally — idempotent, no need to
     compare-and-swap. Once true it stays true for the process lifetime.

3. **AOT watcher (`start_aot_watcher`, `src-tauri/src/aot_watcher.rs`)**
   - At the top of each loop iteration, before the pin computation: if
     `!started_online.load(Ordering::SeqCst)`, force the window hidden
     (`if win.is_visible() { win.hide() }`) and `continue` — skip all pin logic.
   - Once `started_online` is true, the existing pin/show/hide logic runs
     unchanged.

4. **`tauri.conf.json`** — change the `main` window `"visible": true` →
   `"visible": false`.
   - Prevents the boot flash: previously the window appeared then got hidden.
   - When online and the pin conditions are met, the watcher shows it within one
     poll tick (~200ms–1s), so there is no lasting visible delay for the normal
     case.

### Data flow

```
boot
 └─ window starts hidden (visible:false)
 └─ poll loop tick:
      fetch live? ── yes ──► usage.source = live ──► started_online = true
                  └─ no  ──► (first time) source = JsonlFallback, flag stays false
 └─ aot_watcher tick (every 200ms):
      started_online == false ──► force hide, continue
      started_online == true  ──► normal Auto/Pinned show/hide
```

## Trade-off (accepted)

While the gate is active (offline at startup), clicking the tray icon to show
the overlay will be overridden: the next watcher tick (~200ms) hides it again,
because the gate is authoritative until the first online. The user accepted
this — "can't manually show until online" is consistent with the intent. If this
becomes annoying later, a separate `manual_show` flag could let a tray-show beat
the gate, but that is out of scope (YAGNI).

## Edge cases

- **Never online (no login):** overlay stays in tray all session. Correct.
- **Transient offline after first online:** poll loop keeps last known live
  value, `source` stays live, flag stays true → no re-hide. Correct (one-shot).
- **Pinned mode at boot while offline:** still hidden until online (gate wins
  over Pinned). After online, Pinned shows it. Matches intent — don't show a 0%
  overlay before real data.
- **Non-Windows:** out of scope; this is a Windows app. `any_monitored_visible`
  already stubs to `true` off-Windows and the AOT loop holds state, so behavior
  there is unchanged aside from the initial hidden start.

## Testing

- Existing `should_pin` unit tests are unaffected (pure function, untouched).
- The gate logic is loop/OS-state dependent and not unit-testable in isolation;
  verify by manual test:
  1. **Offline boot:** disconnect network (or sign out), launch → overlay stays
     in tray, tray icon present, no flash.
  2. **Goes online:** reconnect → within ~1–2 poll cycles the overlay appears
     per the current mode (Auto: when a monitored app is foreground/visible;
     Pinned: immediately).
  3. **Online boot (regression):** normal network, launch → overlay appears as
     before, no regression, no extra delay beyond one tick.
  4. **Stays online then drops:** after data loads, kill network → overlay does
     NOT hide (one-shot honored).

## Files touched

- `src-tauri/src/lib.rs` — add flag to `AppState`, set it in the poll loop, pass
  it to the watcher.
- `src-tauri/src/aot_watcher.rs` — accept the flag, gate visibility at the top of
  the loop.
- `src-tauri/tauri.conf.json` — `main` window `visible: false`.
