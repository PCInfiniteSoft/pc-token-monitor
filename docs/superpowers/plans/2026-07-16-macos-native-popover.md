# macOS Native Popover UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the macOS click-popover contents with a native-feeling, appearance-aware panel (rounded, translucent, SF Pro), leaving the Windows overlay HUD unchanged.

**Architecture:** A macOS-only `MacPopover` React component is swapped in for `OverlayWindow` in `App.tsx` via a runtime `isMacOS()` check. It reuses the same `useUsageStore` data and `PlanBadge`. The reset-countdown formatter is extracted to a shared module so `UsageBar` and `MacPopover` share one implementation. On macOS the `main` window is resized to the roomier popover dimensions.

**Tech Stack:** React 19 + TypeScript, Tailwind CSS v4 (`dark:` = `prefers-color-scheme`), Vitest + Testing Library; Rust + Tauri 2.11.2.

## Global Constraints

- macOS-only: the new UI is gated behind `isMacOS()` (frontend) and `#[cfg(target_os = "macos")]` (Rust). Windows renders the unchanged `OverlayWindow` and keeps `tauri.conf.json`'s 200×140 window.
- Band thresholds (verbatim), matching the Rust tray: `>= 90` red `#ff453a`; `>= 70` amber `#ff9f0a`; else azure `#0a84ff`.
- `isMacOS()` = `/Mac/i.test(navigator.userAgent)`.
- Percent shown = `Math.min(100, Math.round(utilization * 100))` (utilization is a 0..1 fraction).
- Reuse existing commands: Settings → `invoke("open_settings")`, Quit → `invoke("quit_app")`.
- Frontend tests: `cd "/Users/pcinfinity/Dev/PC Token Monitor" && npm test`. Rust: `cd src-tauri && cargo build`.
- Test mock convention: `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` and set `vi.mocked(invoke).mockResolvedValue(undefined)` in `beforeEach` (see `src/hooks/useTauriEvents.test.ts`).

---

### Task 1: Extract the reset-countdown formatter to a shared module

**Files:**
- Create: `src/usageFormat.ts`, `src/usageFormat.test.ts`
- Modify: `src/components/UsageBar.tsx` (remove local `formatCountdown`, import shared)

**Interfaces:**
- Produces: `export function formatCountdown(resetsAt: string): string`
- Consumes: nothing.

The current `UsageBar.tsx` contains this exact function (lines ~16-26):

```ts
function formatCountdown(resetsAt: string): string {
  const diff = new Date(resetsAt).getTime() - Date.now();
  if (diff <= 0) return "resetting...";
  const totalSecs = Math.floor(diff / 1000);
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}
```

- [ ] **Step 1: Write the failing test**

Create `src/usageFormat.test.ts`:

```ts
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { formatCountdown } from "./usageFormat";

describe("formatCountdown", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-16T00:00:00Z"));
  });
  afterEach(() => vi.useRealTimers());

  it("shows days and hours when more than a day away", () => {
    expect(formatCountdown("2026-07-18T05:00:00Z")).toBe("2d 5h");
  });

  it("shows hours and minutes when less than a day away", () => {
    expect(formatCountdown("2026-07-16T02:30:00Z")).toBe("2h 30m");
  });

  it("shows minutes only when less than an hour away", () => {
    expect(formatCountdown("2026-07-16T00:45:00Z")).toBe("45m");
  });

  it("shows resetting when already elapsed", () => {
    expect(formatCountdown("2026-07-15T23:00:00Z")).toBe("resetting...");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- usageFormat`
Expected: FAIL — cannot resolve `./usageFormat`.

- [ ] **Step 3: Create the shared module**

Create `src/usageFormat.ts`:

```ts
export function formatCountdown(resetsAt: string): string {
  const diff = new Date(resetsAt).getTime() - Date.now();
  if (diff <= 0) return "resetting...";
  const totalSecs = Math.floor(diff / 1000);
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}
```

- [ ] **Step 4: Point `UsageBar.tsx` at the shared module**

In `src/components/UsageBar.tsx`, delete the local `function formatCountdown(...) { ... }` block and add an import at the top (with the other imports):

```ts
import { formatCountdown } from "../usageFormat";
```

Leave the `useMemo(() => formatCountdown(resetsAt), [resetsAt])` call and everything else unchanged.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `npm test -- usageFormat UsageBar`
Expected: PASS — new `formatCountdown` tests pass and the existing `UsageBar` tests still pass (behavior unchanged).

- [ ] **Step 6: Commit**

```bash
git add src/usageFormat.ts src/usageFormat.test.ts src/components/UsageBar.tsx
git commit -m "refactor: extract formatCountdown into shared usageFormat module"
```

---

### Task 2: `MacPopover` component

**Files:**
- Create: `src/components/MacPopover.tsx`, `src/components/MacPopover.test.tsx`

**Interfaces:**
- Consumes: `formatCountdown` (Task 1), `useUsageStore`, `PlanBadge`, `invoke`.
- Produces: `export function MacPopover(): JSX.Element`

- [ ] **Step 1: Write the failing test**

Create `src/components/MacPopover.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../stores/usageStore", () => ({
  useUsageStore: () => ({
    frontendState: {
      usage: {
        five_hour: { utilization: 0.73, resets_at: "2026-07-16T15:30:00Z" },
        seven_day: { utilization: 0.4, resets_at: "2026-07-23T10:00:00Z" },
      },
      config: { plan: "Max200" },
      user_name: "taksin",
    },
    isOffline: () => false,
  }),
}));

import { invoke } from "@tauri-apps/api/core";
import { MacPopover } from "./MacPopover";

describe("MacPopover", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("renders both usage windows with percentages", () => {
    render(<MacPopover />);
    expect(screen.getByText("5-Hour")).toBeInTheDocument();
    expect(screen.getByText("7-Day")).toBeInTheDocument();
    expect(screen.getByText("73%")).toBeInTheDocument();
    expect(screen.getByText("40%")).toBeInTheDocument();
  });

  it("invokes open_settings when the settings button is clicked", () => {
    render(<MacPopover />);
    fireEvent.click(screen.getByLabelText("settings"));
    expect(invoke).toHaveBeenCalledWith("open_settings");
  });

  it("invokes quit_app when the quit button is clicked", () => {
    render(<MacPopover />);
    fireEvent.click(screen.getByLabelText("quit"));
    expect(invoke).toHaveBeenCalledWith("quit_app");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- MacPopover`
Expected: FAIL — cannot resolve `./MacPopover`.

- [ ] **Step 3: Implement the component**

Create `src/components/MacPopover.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";
import { PlanBadge } from "./PlanBadge";
import { formatCountdown } from "../usageFormat";

const SYSTEM_FONT =
  "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif";

function bandColor(pct: number): string {
  if (pct >= 90) return "#ff453a";
  if (pct >= 70) return "#ff9f0a";
  return "#0a84ff";
}

function barGradient(pct: number): string {
  if (pct >= 90) return "linear-gradient(90deg,#ff6961,#ff453a)";
  if (pct >= 70) return "linear-gradient(90deg,#ffb340,#ff9f0a)";
  return "linear-gradient(90deg,#4aa3ff,#0a84ff)";
}

function UsageRow({
  label,
  utilization,
  resetsAt,
}: {
  label: string;
  utilization: number;
  resetsAt: string;
}) {
  const pct = Math.min(100, Math.round(utilization * 100));
  return (
    <div className="mb-3.5 last:mb-3">
      <div className="flex items-baseline justify-between mb-1.5">
        <span className="text-[11px] font-semibold tracking-wide uppercase text-black/55 dark:text-white/60">
          {label}
        </span>
        <span
          className="text-[17px] font-bold tabular-nums tracking-tight"
          style={{ color: bandColor(pct) }}
        >
          {pct}%
        </span>
      </div>
      <div className="h-[7px] rounded-full overflow-hidden bg-black/10 dark:bg-white/15">
        <div
          className="h-full rounded-full"
          style={{ width: `${pct}%`, background: barGradient(pct) }}
        />
      </div>
      <div className="text-[11px] mt-1.5 tabular-nums text-black/45 dark:text-white/45">
        reset in {formatCountdown(resetsAt)}
      </div>
    </div>
  );
}

export function MacPopover() {
  const { frontendState, isOffline } = useUsageStore();
  const usage = frontendState?.usage;
  const plan = frontendState?.config.plan ?? "Unknown";
  const userName = frontendState?.user_name ?? "—";
  const offline = isOffline();
  const initial = userName.trim().charAt(0).toUpperCase() || "?";

  return (
    <div
      className="w-full h-full p-1.5 bg-transparent select-none"
      style={{ fontFamily: SYSTEM_FONT }}
    >
      <div className="w-full h-full rounded-xl px-4 pt-3.5 pb-3 flex flex-col bg-white/80 dark:bg-[#282828]/80 backdrop-blur-2xl border-[0.5px] border-black/10 dark:border-white/15 shadow-2xl text-[#1c1c1e] dark:text-[#f2f2f7]">
        {/* header */}
        <div className="flex items-center justify-between mb-3.5">
          <span className="text-[14px] font-semibold tracking-tight">
            Claude Usage
          </span>
          <PlanBadge plan={plan} offline={offline} />
        </div>

        {/* usage windows */}
        <div className="flex-1">
          {usage ? (
            <>
              <UsageRow
                label="5-Hour"
                utilization={usage.five_hour.utilization}
                resetsAt={usage.five_hour.resets_at}
              />
              <UsageRow
                label="7-Day"
                utilization={usage.seven_day.utilization}
                resetsAt={usage.seven_day.resets_at}
              />
            </>
          ) : (
            <div className="text-[12px] text-center py-4 text-black/45 dark:text-white/45">
              connecting…
            </div>
          )}
        </div>

        {/* footer */}
        <div className="flex items-center justify-between pt-3 border-t border-black/10 dark:border-white/15">
          <span className="flex items-center gap-1.5 text-[11.5px] font-medium text-black/55 dark:text-white/60">
            <span
              className="w-[17px] h-[17px] rounded-full inline-flex items-center justify-center text-white text-[9px] font-bold"
              style={{ background: "linear-gradient(135deg,#0a84ff,#7c5aa8)" }}
            >
              {initial}
            </span>
            {userName}
          </span>
          <div className="flex gap-1">
            <button
              onClick={() => invoke("open_settings").catch(() => {})}
              aria-label="settings"
              className="w-[26px] h-[26px] rounded-[7px] inline-flex items-center justify-center text-[12px] bg-black/5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/20 text-black/70 dark:text-white/85 transition-colors"
            >
              ⚙
            </button>
            <button
              onClick={() => invoke("quit_app").catch(() => {})}
              aria-label="quit"
              className="w-[26px] h-[26px] rounded-[7px] inline-flex items-center justify-center text-[12px] bg-black/5 hover:bg-black/10 dark:bg-white/10 dark:hover:bg-white/20 text-black/70 dark:text-white/85 transition-colors"
            >
              ⏻
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npm test -- MacPopover`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add src/components/MacPopover.tsx src/components/MacPopover.test.tsx
git commit -m "feat(macos): native menu-bar popover component"
```

---

### Task 3: Platform detection + `App.tsx` swap

**Files:**
- Create: `src/platform.ts`, `src/platform.test.ts`
- Modify: `src/App.tsx`

**Interfaces:**
- Produces: `export function isMacOS(): boolean`
- Consumes: `MacPopover` (Task 2), `OverlayWindow`.

- [ ] **Step 1: Write the failing test**

Create `src/platform.test.ts`:

```ts
import { describe, it, expect, afterEach, vi } from "vitest";
import { isMacOS } from "./platform";

function setUA(ua: string) {
  vi.spyOn(navigator, "userAgent", "get").mockReturnValue(ua);
}

describe("isMacOS", () => {
  afterEach(() => vi.restoreAllMocks());

  it("is true on a macOS WebKit user agent", () => {
    setUA(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15",
    );
    expect(isMacOS()).toBe(true);
  });

  it("is false on a Windows user agent", () => {
    setUA("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");
    expect(isMacOS()).toBe(false);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm test -- platform`
Expected: FAIL — cannot resolve `./platform`.

- [ ] **Step 3: Create the platform helper**

Create `src/platform.ts`:

```ts
/** True when running in the macOS WebKit webview. */
export function isMacOS(): boolean {
  return /Mac/i.test(navigator.userAgent);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npm test -- platform`
Expected: PASS.

- [ ] **Step 5: Swap the component in `App.tsx`**

In `src/App.tsx`, add imports:

```tsx
import { MacPopover } from "./components/MacPopover";
import { isMacOS } from "./platform";
```

Then in `OverlayApp`, replace the final `return <OverlayWindow />;` with:

```tsx
  return isMacOS() ? <MacPopover /> : <OverlayWindow />;
```

Leave the `settings` window branch and `FirstRunDialog` handling unchanged.

- [ ] **Step 6: Verify the whole suite passes and the build compiles**

Run: `npm test`
Expected: PASS (all files).
Run: `npm run build`
Expected: `tsc` + `vite build` succeed with no type errors.

- [ ] **Step 7: Commit**

```bash
git add src/platform.ts src/platform.test.ts src/App.tsx
git commit -m "feat(macos): render MacPopover on macOS, keep overlay on Windows"
```

---

### Task 4: Resize the macOS popover window

**Files:**
- Modify: `src-tauri/src/tray.rs` (make `POPOVER_WIDTH` `pub`, add `POPOVER_HEIGHT`, set dimensions)
- Modify: `src-tauri/src/lib.rs` (resize `main` on macOS in `setup`)

**Interfaces:**
- Produces: `pub const POPOVER_WIDTH: f64`, `pub const POPOVER_HEIGHT: f64` (macOS only).
- Consumes: these constants in `lib.rs`.

- [ ] **Step 1: Widen and export the popover dimensions in `tray.rs`**

In `src-tauri/src/tray.rs`, replace the existing const:

```rust
/// Popover width fallback — must match the `main` window `width` in tauri.conf.json.
#[cfg(target_os = "macos")]
const POPOVER_WIDTH: f64 = 200.0;
```

with:

```rust
/// macOS popover panel dimensions (logical points). The window is resized to
/// these at startup (see lib.rs); the Windows overlay keeps its tauri.conf.json size.
#[cfg(target_os = "macos")]
pub const POPOVER_WIDTH: f64 = 270.0;
#[cfg(target_os = "macos")]
pub const POPOVER_HEIGHT: f64 = 204.0;
```

(`position_popover` already references `POPOVER_WIDTH`; no other change needed there.)

- [ ] **Step 2: Resize the window on macOS in `lib.rs` setup**

In `src-tauri/src/lib.rs` `setup`, immediately after the existing block that positions the `main` window (the `if let Some(win) = app.get_webview_window("main") { ... set_position ... }` block), add:

```rust
            // macOS: the popover panel is roomier than the Windows overlay.
            #[cfg(target_os = "macos")]
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_size(tauri::LogicalSize::new(
                    tray::POPOVER_WIDTH,
                    tray::POPOVER_HEIGHT,
                ));
            }
```

- [ ] **Step 3: Verify it builds with no new warnings**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "never used|unused|warning: unresolved"`
Expected: no new warnings for `POPOVER_WIDTH`/`POPOVER_HEIGHT` (the 4 pre-existing dead-code warnings in `jsonl_parser.rs`/`plan_from_extra_usage` may remain — those are unrelated).

- [ ] **Step 4: Verify Rust tests still pass**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result:"`
Expected: `42 passed; 0 failed` (unchanged).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tray.rs src-tauri/src/lib.rs
git commit -m "feat(macos): size the main window to the popover panel"
```

---

## Manual verification (macOS)

Run `npm run tauri dev` and confirm:

- [ ] Clicking the menu-bar item opens a rounded, translucent panel (not the dark HUD rectangle) anchored under the item.
- [ ] The panel follows the system appearance — switch System Settings → Appearance between Light and Dark and reopen; text/material invert appropriately.
- [ ] 5-Hour and 7-Day rows show the correct percentages, band colors (azure/amber/red), gradient bars, and reset countdowns.
- [ ] Settings opens the settings window; Quit exits the app.
- [ ] Clicking away dismisses the panel.

## Manual verification (Windows regression)

- [ ] The overlay renders as the existing HUD (dark, monospace, `⬡` title, draggable) — `MacPopover` never appears.
