# Overlay Layout Redesign + Claude User Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move window controls to the header, show the Claude account display name bottom-left, and the AI name + plan badge bottom-right.

**Architecture:** Read the display name once from `~/.claude.json` (`oauthAccount`) in Rust, surface it via `FrontendState.user_name`, and restructure `OverlayWindow`'s header/footer. The provider name is a single frontend constant for future multi-provider support.

**Tech Stack:** Tauri 2 (Rust, serde_json), React 19 + TypeScript, Vitest + Rust `#[test]`.

**Spec:** `docs/superpowers/specs/2026-06-06-overlay-layout-and-user-design.md`

---

## Environment notes

- The `npm run tauri dev` server is **running** (task watches `src-tauri` + `src`); it rebuilds on save. cargo/npm commands you run will share the build lock — run them in the **foreground** with a long timeout; they may wait for an in-flight dev rebuild. Do not background them.
- Rust tests/build: `--no-default-features`. Frontend tests: `npm test -- --run`.
- **Before the final commit, run `npx tsc --noEmit`** — `npm test` (Vitest) does NOT type-check, and a missing-field type error broke the v0.2.0 release.

## File Structure

- Create `src-tauri/src/account.rs` — read/parse the Claude display name.
- Modify `src-tauri/src/lib.rs` — `mod account`; `AppState`/`FrontendState` plumbing; `start_poll_loop` param; `get_state`.
- Modify `src-tauri/src/types.rs` — `FrontendState.user_name`.
- Modify `src/types.ts` — `FrontendState.user_name`.
- Create `src/constants.ts` — `AI_NAME`.
- Modify `src/components/OverlayWindow.tsx` — header/footer restructure.
- Modify `src/stores/usageStore.test.ts` — add `user_name` to the typed mock.

---

### Task 1: Read the Claude display name (Rust, TDD)

**Files:** Create `src-tauri/src/account.rs`; Modify `src-tauri/src/lib.rs` (add `mod account;`)

- [ ] **Step 1: Create `src-tauri/src/account.rs`**

```rust
//! Reads the signed-in Claude account's display name from `~/.claude.json`.

use std::path::PathBuf;

pub fn account_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude.json")
}

/// The account display name: `oauthAccount.displayName` if present and
/// non-empty, else `oauthAccount.emailAddress`, else `None`.
pub fn parse_user_name(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let acct = v.get("oauthAccount")?;
    let display = acct
        .get("displayName")
        .and_then(|d| d.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(d) = display {
        return Some(d.to_string());
    }
    acct.get("emailAddress")
        .and_then(|e| e.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn load_user_name(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_user_name(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_display_name() {
        let json = r#"{"oauthAccount":{"displayName":"Cocoa","emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(json), Some("Cocoa".to_string()));
    }

    #[test]
    fn falls_back_to_email_when_display_missing_or_empty() {
        let missing = r#"{"oauthAccount":{"emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(missing), Some("a@b.com".to_string()));
        let empty = r#"{"oauthAccount":{"displayName":"  ","emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(empty), Some("a@b.com".to_string()));
    }

    #[test]
    fn none_when_no_account_or_malformed() {
        assert_eq!(parse_user_name(r#"{"other":1}"#), None);
        assert_eq!(parse_user_name("not json"), None);
        assert_eq!(parse_user_name(r#"{"oauthAccount":{}}"#), None);
    }
}
```

- [ ] **Step 2: Register the module in `lib.rs`**

Add alongside the other `mod` lines near the top:

```rust
mod account;
```

- [ ] **Step 3: Run the tests**

Run: `cd src-tauri && cargo test --no-default-features account`
Expected: 3 `account::tests::*` pass. (`dead_code` warnings for `account_path`/`load_user_name` are expected until Task 2 — ignore.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/account.rs src-tauri/src/lib.rs
git commit -m "feat: read Claude account display name from ~/.claude.json

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Plumb `user_name` through to the frontend state (Rust)

**Files:** Modify `src-tauri/src/types.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the field to `FrontendState` (`types.rs`)**

Change:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendState {
    pub usage: Option<UsageData>,
    pub config: AppConfig,
}
```

to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendState {
    pub usage: Option<UsageData>,
    pub config: AppConfig,
    pub user_name: Option<String>,
}
```

- [ ] **Step 2: Add `user_name` to `AppState` and `get_state` (`lib.rs`)**

Change the `AppState` struct:

```rust
struct AppState {
    usage: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
    user_name: Option<String>,
}
```

Change `get_state`:

```rust
#[tauri::command]
fn get_state(state: State<AppState>) -> FrontendState {
    FrontendState {
        usage: state.usage.lock().unwrap().clone(),
        config: state.config.lock().unwrap().clone(),
        user_name: state.user_name.clone(),
    }
}
```

- [ ] **Step 3: Add a `user_name` param to `start_poll_loop` and include it in the emit (`lib.rs`)**

Change the signature:

```rust
fn start_poll_loop(
    app: AppHandle,
    state: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
    user_name: Option<String>,
) {
```

In the loop's `usage-updated` emit, include the field. Change:

```rust
                let frontend = FrontendState {
                    usage: Some(u.clone()),
                    config: cfg,
                };
```

to:

```rust
                let frontend = FrontendState {
                    usage: Some(u.clone()),
                    config: cfg,
                    user_name: user_name.clone(),
                };
```

- [ ] **Step 4: Resolve the name at setup, store it, pass it (`lib.rs` setup)**

In `setup`, before `app.manage(...)`, resolve the name:

```rust
            let user_name = account::load_user_name(&account::account_path());
```

Change the `app.manage(AppState { ... })` call to include it:

```rust
            app.manage(AppState {
                usage: usage_arc.clone(),
                config: config_arc.clone(),
                user_name: user_name.clone(),
            });
```

Change the `start_poll_loop(...)` call to pass it:

```rust
            start_poll_loop(
                app_handle.clone(),
                usage_for_poll,
                config_for_poll,
                user_name.clone(),
            );
```

- [ ] **Step 5: Build + test**

Run: `cd src-tauri && cargo build --no-default-features`
Expected: builds, no errors (no more `dead_code` for `load_user_name`/`account_path`).

Run: `cd src-tauri && cargo test --no-default-features`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/lib.rs
git commit -m "feat: expose Claude user_name in FrontendState

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Frontend type + AI name constant (TypeScript)

**Files:** Modify `src/types.ts`; Create `src/constants.ts`

- [ ] **Step 1: Add `user_name` to `FrontendState` (`src/types.ts`)**

Change:

```ts
export interface FrontendState {
  usage: UsageData | null;
  config: AppConfig;
}
```

to:

```ts
export interface FrontendState {
  usage: UsageData | null;
  config: AppConfig;
  user_name: string | null;
}
```

- [ ] **Step 2: Create `src/constants.ts`**

```ts
// Single source for the provider name until multi-provider support lands.
export const AI_NAME = "Claude";
```

- [ ] **Step 3: Update the typed test mock (`src/stores/usageStore.test.ts`)**

The mock is typed `const mockState: FrontendState = {...}`, so it must include the new field. Change:

```ts
  config: { plan: "Max50", aot_mode: "auto", aot_allowlist: [] },
};
```

to:

```ts
  config: { plan: "Max50", aot_mode: "auto", aot_allowlist: [] },
  user_name: "Cocoa",
};
```

- [ ] **Step 4: Verify types + tests**

Run: `npx tsc --noEmit`
Expected: no errors.

Run: `npm test -- --run`
Expected: all pass (18).

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/constants.ts src/stores/usageStore.test.ts
git commit -m "feat: FrontendState.user_name + AI_NAME constant

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: OverlayWindow header/footer restructure (TypeScript)

**Files:** Modify `src/components/OverlayWindow.tsx`

- [ ] **Step 1: Replace the whole component file**

Replace the entire contents of `src/components/OverlayWindow.tsx` with:

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { useUsageStore } from "../stores/usageStore";
import { overlayPalette, type BgTheme } from "../overlayPalette";
import { AI_NAME } from "../constants";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";

export function OverlayWindow() {
  const { frontendState, isOffline } = useUsageStore();

  const [bgTheme, setBgTheme] = useState<BgTheme>("dark");

  useEffect(() => {
    const unlisten = listen<string>("bg-theme", (event) => {
      setBgTheme(event.payload === "light" ? "light" : "dark");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const pal = overlayPalette(bgTheme);

  const usage = frontendState?.usage;
  const plan = frontendState?.config.plan ?? "Unknown";
  const offline = isOffline();
  const userName = frontendState?.user_name ?? "—";

  function openSettings() {
    invoke("open_settings").catch(() => {});
  }

  function minimize() {
    getCurrentWindow().hide().catch(() => {});
  }

  return (
    <div
      data-tauri-drag-region
      className="w-full h-full flex flex-col select-none"
      style={{
        textShadow: pal.shadow,
        ["--ov-text" as string]: pal.text,
        ["--ov-muted" as string]: pal.muted,
      }}
    >
      {/* Header: title + window controls */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span
          className="font-mono text-[10px] tracking-widest pointer-events-none"
          style={{ color: "var(--ov-muted)" }}
        >
          ⬡ PC TOKEN MONITOR
        </span>
        <div className="flex items-center gap-2">
          <button
            onClick={openSettings}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="settings"
          >
            ⚙
          </button>
          <button
            onClick={minimize}
            className="font-mono text-[10px] transition-opacity hover:opacity-70"
            style={{ color: "var(--ov-muted)" }}
            aria-label="close"
          >
            ×
          </button>
        </div>
      </div>

      {/* Usage bars */}
      <div className="flex flex-col gap-1.5 px-2 py-1.5 flex-1">
        {usage ? (
          <>
            <UsageBar
              label="5HR"
              utilization={usage.five_hour.utilization}
              resetsAt={usage.five_hour.resets_at}
              labelColor={pal.label5}
            />
            <UsageBar
              label="7DAY"
              utilization={usage.seven_day.utilization}
              resetsAt={usage.seven_day.resets_at}
              labelColor={pal.label7}
            />
          </>
        ) : (
          <span
            className="font-mono text-[10px] text-center py-2"
            style={{ color: "var(--ov-muted)" }}
          >
            connecting...
          </span>
        )}
      </div>

      {/* Footer: Claude user (left) · AI name + plan (right) */}
      <div className="flex items-center justify-between px-2 py-1 border-t border-[#1e1e1e]">
        <span
          className="font-mono text-[9px] tracking-wide pointer-events-none truncate max-w-[90px]"
          style={{ color: "var(--ov-muted)" }}
        >
          👤 {userName}
        </span>
        <div className="flex items-center gap-1.5 pointer-events-none">
          <span className="font-mono text-[9px] tracking-wide" style={{ color: "var(--ov-muted)" }}>
            {AI_NAME}
          </span>
          <PlanBadge plan={plan} offline={offline} />
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify types + tests**

Run: `npx tsc --noEmit`
Expected: no errors (the removed `AotMode` import and `aotMode` state are gone, so no unused-symbol errors).

Run: `npm test -- --run`
Expected: all pass (18).

- [ ] **Step 3: Manual check**

The dev server hot-reloads. Confirm in the overlay:
- header right shows `⚙` and `×`; both work (settings opens, × hides to tray);
- bottom-left shows `👤 Cocoa`;
- bottom-right shows `Claude [MAX 200]` (or `[OFFLINE]` when offline);
- the AUTO/PINNED button is gone; AUTO/PINNED is still switchable in Settings.

- [ ] **Step 4: Commit**

```bash
git add src/components/OverlayWindow.tsx
git commit -m "feat: move controls to header; show Claude user + AI/plan in footer

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final Verification

- [ ] `cd src-tauri && cargo test --no-default-features` → all pass (existing + 3 `account`).
- [ ] `npx tsc --noEmit` → no errors.
- [ ] `npm test -- --run` → all pass (18).
- [ ] Manual: header controls work; `👤 Cocoa` bottom-left; `Claude [PLAN]` bottom-right; Settings still controls AUTO/PINNED.
- [ ] `git status` clean (except untracked `scripts/`).

## Notes

- `user_name` is read once at startup; it doesn't change at runtime.
- The footer user span is `truncate max-w-[90px]` so a long name (or the email
  fallback) can't overflow the 200px window.
- Multi-provider support (per-provider name/endpoint/credentials/plan mapping)
  is intentionally deferred; only `AI_NAME` changes hands today.
