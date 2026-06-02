# PC Token Monitor — Design Spec

**Date:** 2026-06-02  
**Status:** Approved

---

## Overview

A lightweight Windows desktop overlay app that monitors Claude Code token usage in real time. Styled like MSI Afterburner — compact, dark, always-on-top. Shows 5-hour and weekly usage limits with progress bars, plan badge, and system tray % indicator.

## Hard Constraints

- **Zero token consumption.** The app MUST NOT make any call to `/v1/messages` or any Claude inference endpoint. All data comes from local file reads and/or a usage-metadata HTTP endpoint only. The monitor must never inject prompts or context into Claude conversations.

---

## Architecture

```
┌─────────────────────────────────────┐
│  Tauri 2 App                        │
│                                     │
│  ┌─────────────┐  ┌──────────────┐  │
│  │ Rust Backend│  │ React Frontend│ │
│  │             │  │              │  │
│  │ FileWatcher │→ │ Overlay UI   │  │
│  │ JSONL Parser│  │ Progress bars│  │
│  │ Usage Calc  │  │ Plan badge   │  │
│  │ Tray Manager│  │ Countdown    │  │
│  └─────────────┘  └──────────────┘  │
└─────────────────────────────────────┘
```

**Stack:**
- Tauri 2 (Rust backend + WebView frontend)
- React 19 + TypeScript
- Tailwind CSS
- Zustand (state management)

---

## UI Design

### Overlay Window

Frameless window, always-on-top toggle, ~280×160px

```
┌────────────────────────────────┐
│ ⬡ PC TOKEN MONITOR    [MAX 50] │
├────────────────────────────────┤
│ 5HR   ████████░░░░  73%        │
│       reset in  1h 24m         │
│                                │
│ 7DAY  ██████████████  91%      │
│       resets in  2d 14h        │
├────────────────────────────────┤
│ [⊤ Always on Top]  [×]        │
└────────────────────────────────┘
```

### Visual Style

| Element | Value |
|---|---|
| Background | `#0a0a0a` |
| 5HR label | `#00d4ff` (cyan) |
| 7DAY label | `#ffd700` (yellow) |
| Plan badge | `#ffffff` on `#333` |
| Font | Consolas / JetBrains Mono (monospace) |
| Progress bar | green → orange → red (threshold >80%) |

### System Tray

- Icon displays % of the limit closest to being reached (e.g., `73%`)
- Right-click menu:
  - Show / Hide window
  - Always on Top (toggle, checkmark)
  - Quit

### Plan Badges

| Plan | Badge |
|---|---|
| Pro | `[PRO]` |
| Max 50 | `[MAX 50]` |
| Max 200 | `[MAX 200]` |
| Unknown | `[UNKNOWN]` |

---

## Data Layer

### JSONL Source

Path: `~/.claude/projects/<project-hash>/<conversation>.jsonl`

Each line format:
```json
{
  "type": "assistant",
  "message": {
    "usage": {
      "input_tokens": 1234,
      "output_tokens": 567,
      "cache_read_input_tokens": 0
    }
  },
  "timestamp": "2026-06-02T10:30:00Z"
}
```

### OAuth API (Primary Data Source)

Load OAuth token from `~/.claude/.credentials.json`. Try endpoints in order:

1. `https://api.anthropic.com/api/oauth/claude_cli/client_data` (newer)
2. `https://api.anthropic.com/api/oauth/usage` (legacy fallback)

Request headers:
```
Authorization: Bearer {token}
anthropic-beta: oauth-2025-04-20
```

Response fields used:

| Field | Used for |
|---|---|
| `five_hour.utilization` | 5hr progress bar % |
| `five_hour.resets_at` | 5hr reset countdown (RFC3339) |
| `seven_day.utilization` | 7-day progress bar % |
| `seven_day.resets_at` | 7-day reset time (RFC3339) |
| `seven_day_opus.utilization` | Opus-specific usage (display only, optional) |
| `extra_usage.is_enabled` | Indicates Max plan (extra usage credits active) |

**No token arithmetic needed** — API returns utilization percentages directly. The 7-day window is a rolling window, not a calendar week reset.

### Plan Detection

The OAuth API does not return plan type. Detection order:

1. `extra_usage.is_enabled = true` → likely Max plan (show as `[MAX]`)
2. First-run settings dialog → user selects plan manually (Pro / Max 50 / Max 200)
3. Selection persisted in app config (`app_config.json` in app data dir)

### JSONL Fallback

If OAuth fails (no credentials file, token expired, network error):

- Read `~/.claude/projects/**/*.jsonl` for raw conversation entries
- Extract token counts from `assistant` message entries
- Compute approximate utilization locally (less accurate — no server-side weighting)
- Show `[OFFLINE]` indicator in UI when in fallback mode

### Refresh Cycle

| Trigger | Action |
|---|---|
| JSONL file changed (notify) | Immediate recalculate |
| Every 30s | OAuth fetch + recalculate |
| Every 1s | Client-side countdown update only |

---

## Error Handling

### File Issues

| Condition | Behavior |
|---|---|
| `~/.claude/projects` not found | Show "No Claude data found" message |
| JSONL line corrupt | Skip line, log warning |
| File locked by Claude | Retry read in 500ms |

### OAuth Issues

| Condition | Behavior |
|---|---|
| Token expired | Silent fallback to JSONL |
| Rate limited | Backoff 5 min, retry |
| No credentials file | JSONL-only mode, no error shown |

### Display Edge Cases

| Condition | Behavior |
|---|---|
| Usage = 0% | Show `0%` normally |
| Usage ≥ 100% | Cap bar at full, text turns red, show "Limit reached" |
| Plan unknown | Show `[UNKNOWN]` badge, use Pro limits |

### Window Behavior

- Close button (×) → minimize to tray (does not quit)
- Tray → Quit → full exit
- App launched while already running → focus existing window

---

## File Structure

```
PC Token Monitor/
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── OverlayWindow.tsx
│   │   ├── UsageBar.tsx
│   │   ├── PlanBadge.tsx
│   │   └── CountdownTimer.tsx
│   ├── stores/
│   │   └── usageStore.ts
│   ├── hooks/
│   │   └── useTauriEvents.ts
│   └── types.ts
├── src-tauri/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── jsonl_parser.rs
│       ├── usage_calculator.rs
│       ├── oauth_fetcher.rs
│       ├── file_watcher.rs
│       └── tray.rs
├── docs/
│   └── superpowers/specs/
│       └── 2026-06-02-pc-token-monitor-design.md
└── package.json
```
