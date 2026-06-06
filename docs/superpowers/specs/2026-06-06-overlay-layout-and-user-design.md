# Overlay Layout Redesign + Claude User — Design

Date: 2026-06-06
Status: Approved (pending spec review)

## Problem / Goal

Rework the overlay's header/footer and surface the Claude account identity:

1. Move the window controls (`⚙` settings, `[×]` close-to-tray) to the **top
   right** (where the plan badge currently sits).
2. Remove the footer AUTO/PINNED button (mode is still controlled from the
   Settings window); put the **Claude user's display name** at the **bottom
   left**.
3. Put the **AI name ("Claude") + subscription plan badge** at the **bottom
   right**.

Forward-looking: the app should eventually support multiple AI agents. For now
the provider name lives in a single constant; real multi-provider support is a
separate future spec (YAGNI here).

## Target layout (200×140, transparent)

```
┌────────────────────────────────────┐
│ ⬡ PC TOKEN MONITOR            ⚙  × │  header: title (left) · ⚙ + × (right)
│ 5HR  ▓▓▓▓░░░  73%   reset 2h        │
│ 7DAY ▓▓▓▓▓▓░  91%   reset 5d        │
│ Cocoa               Claude [MAX 200]│  footer: user (left) · AI + plan (right)
└────────────────────────────────────┘
```

## Data: Claude user name

The display name is in `~/.claude.json` → `oauthAccount`:
`{ "displayName": "Cocoa", "emailAddress": "...", ... }` (confirmed present).

### Rust (`src-tauri/src/account.rs`, new)

- `account_path() -> PathBuf` = `dirs::home_dir()/.claude.json`.
- `parse_user_name(json: &str) -> Option<String>` (pure, tested): parse JSON,
  return `oauthAccount.displayName` if present and non-empty, else
  `oauthAccount.emailAddress`, else `None`. Malformed JSON → `None`.
- `load_user_name(path: &PathBuf) -> Option<String>`: read file, call
  `parse_user_name`; `None` on read error.

Use `serde_json::Value` for parsing (the file is large and we only need two
fields — don't model the whole schema).

### Wiring

- `AppState` (`lib.rs`) gains `user_name: Option<String>`, populated once in
  `setup` via `account::load_user_name(&account::account_path())`.
- `FrontendState` (`types.rs`) gains `user_name: Option<String>`.
- Both `FrontendState` construction sites include it:
  - `get_state` command → `user_name: state.user_name.clone()`.
  - poll loop emit → `user_name: <captured at setup>`. The poll loop runs in a
    spawned task without `State`; capture the resolved name once at setup and
    move a clone into the loop (it doesn't change at runtime).

## Frontend

### Types (`src/types.ts`)

`FrontendState` gains `user_name: string | null`.

### AI name constant (`src/constants.ts`, new)

```ts
// Single source for the provider name until multi-provider support lands.
export const AI_NAME = "Claude";
```

### OverlayWindow (`src/components/OverlayWindow.tsx`)

- **Header right:** replace `<PlanBadge>` with the controls group `⚙` + `[×]`
  (moved from the footer). Keep the title on the left, keep the
  `pointer-events-none` on the title.
- **Footer:** becomes two ends:
  - left: the user name from `frontendState.user_name` (fallback `"—"` when
    null), with a small `👤` prefix.
  - right: `{AI_NAME}` text followed by `<PlanBadge plan={plan} offline={offline} />`.
- Remove the AUTO/PINNED mode button and its `toggleMode`/`aotMode` state
  (mode is managed in the Settings window now). Keep `openSettings()` (used by
  the header `⚙`) and `minimize()` (header `[×]`).
- Palette: user name + AI name use `var(--ov-muted)`; keep the adaptive theme
  wiring intact.

## Out of scope

- Real multi-provider support (provider abstraction for endpoint / credentials
  path / plan mapping) — future spec.
- Showing the email (tooltip) — display name only.
- Reacting live to `~/.claude.json` changes — read once at startup.

## Testing

- Rust unit tests for `parse_user_name`: displayName present → it; displayName
  missing/empty → emailAddress; neither → `None`; malformed JSON → `None`.
- Update the typed `FrontendState` mock in `src/stores/usageStore.test.ts` to
  include `user_name` (the untyped mock in `useTauriEvents.test.ts` needs no
  change but add it for realism).
- **Run `npx tsc --noEmit` (or `npm run build`) — not just `npm test`** — so a
  missing-field type error can't slip into a release (this bit v0.2.0).
- Existing Rust + frontend suites stay green.
- Manual: overlay shows `Cocoa` bottom-left and `Claude [MAX 200]` bottom-right;
  `⚙`/`[×]` work from the header; Settings still toggles AUTO/PINNED.
