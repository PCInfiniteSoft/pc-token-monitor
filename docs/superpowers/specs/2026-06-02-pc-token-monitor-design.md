# PC Token Monitor — Design Spec

**Date:** 2026-06-02  
**Status:** Approved

---

## Overview

A lightweight Windows desktop overlay app that monitors Claude Code token usage in real time. Styled like MSI Afterburner — compact, dark, always-on-top. Shows 5-hour and weekly usage limits with progress bars, plan badge, and system tray % indicator.

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
│ WEEK  ██████████████  91%      │
│       resets Mon 00:00         │
├────────────────────────────────┤
│ [⊤ Always on Top]  [×]        │
└────────────────────────────────┘
```

### Visual Style

| Element | Value |
|---|---|
| Background | `#0a0a0a` |
| 5HR label | `#00d4ff` (cyan) |
| WEEK label | `#ffd700` (yellow) |
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

### Usage Calculations

| Metric | Calculation |
|---|---|
| 5hr usage | Sum all tokens where `timestamp > now - 5h` |
| Weekly usage | Sum all tokens where `timestamp > now - 7d` |
| 5hr reset time | Earliest timestamp in 5hr window + 5h |
| Weekly reset time | Start of next Monday 00:00 local time |

### Plan Detection

Read `~/.claude/config.json` → `plan` field (exact field name to be verified during implementation by inspecting actual Claude Code config). If absent, detect from 5hr limit threshold:

| Plan | 5hr token limit |
|---|---|
| Pro | ~900,000 |
| Max 50 | ~4,500,000 |
| Max 200 | ~18,000,000 |

Default to Pro limits (conservative) if detection fails.

### OAuth Hybrid Fetch

1. Load OAuth token from `~/.claude/.credentials.json`
2. Fetch server-side usage from internal Anthropic endpoint (same as Claude desktop usage page). Exact endpoint URL must be discovered during implementation by inspecting Claude desktop's network traffic (DevTools / Fiddler). This endpoint is undocumented and may change — the fallback exists for this reason.
3. On HTTP 200 → use server data as primary
4. On any failure → silently fall back to local JSONL
5. Retry after 5-minute backoff on rate limit

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
