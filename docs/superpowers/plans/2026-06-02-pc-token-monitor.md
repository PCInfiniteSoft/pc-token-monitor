# PC Token Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Windows desktop overlay that shows Claude Code 5hr and 7-day utilization % (from Anthropic OAuth API) in an MSI Afterburner-style always-on-top window with system tray % indicator.

**Architecture:** Rust backend (Tauri 2) reads OAuth credentials and fetches utilization data from `api.anthropic.com/api/oauth/claude_cli/client_data`, watches `~/.claude/projects` for changes, and emits events to a React frontend. Frontend renders a frameless dark overlay with progress bars. JSONL parsing used as offline fallback only.

**Tech Stack:** Tauri 2, React 19, TypeScript, Tailwind CSS, Zustand, reqwest, notify, image + ab_glyph (tray icon generation), Vitest + @testing-library/react

---

## Prerequisites (verify before Task 1)

The engineer must have installed:
- Rust toolchain via [rustup.rs](https://rustup.rs) — run `rustup update stable`
- Node.js 20+ — run `node --version`
- Microsoft C++ Build Tools (or Visual Studio with "Desktop development with C++")
- WebView2 runtime (pre-installed on Windows 11)

---

## File Map

| File | Responsibility |
|---|---|
| `src-tauri/src/types.rs` | Shared Rust structs (UsageData, WindowUsage, AppConfig) |
| `src-tauri/src/oauth_fetcher.rs` | Fetch utilization from Anthropic OAuth API |
| `src-tauri/src/jsonl_parser.rs` | Fallback: parse ~/.claude/projects JSONL for raw token counts |
| `src-tauri/src/config.rs` | Read/write app_config.json (plan selection) |
| `src-tauri/src/file_watcher.rs` | Watch ~/.claude/projects for JSONL changes |
| `src-tauri/src/tray.rs` | System tray icon (dynamic % PNG) + menu |
| `src-tauri/src/lib.rs` | Tauri app setup, command registration, event loop |
| `src-tauri/src/main.rs` | Entry point (calls lib.rs) |
| `src/types.ts` | TypeScript mirror of Rust types |
| `src/stores/usageStore.ts` | Zustand store — single source of truth for frontend |
| `src/hooks/useTauriEvents.ts` | Listen to `usage-updated` Tauri event, update store |
| `src/components/UsageBar.tsx` | Progress bar + label + reset countdown |
| `src/components/PlanBadge.tsx` | Plan chip ([PRO] / [MAX 50] / [MAX 200] / [OFFLINE]) |
| `src/components/FirstRunDialog.tsx` | Plan selection dialog on first launch |
| `src/components/OverlayWindow.tsx` | Main overlay layout (composes all components) |
| `src/App.tsx` | Root — mounts overlay, handles first-run, always-on-top toggle |

---

## Task 1: Project Scaffold

**Files:**
- Create: entire project via `npm create tauri-app@latest`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json` (add dev dependencies)
- Modify: `src/index.css` (Tailwind directives)
- Create: `tailwind.config.js`

- [ ] **Step 1: Scaffold the project**

In `C:\Users\PC-Laptop\Documents\Dev Project\PC Token Monitor`, run:
```powershell
npm create tauri-app@latest . -- --template react-ts --manager npm
```
When prompted:
- Package manager: `npm`
- UI template: `React` → `TypeScript`

- [ ] **Step 2: Install frontend dependencies**

```powershell
npm install zustand
npm install -D vitest @testing-library/react @testing-library/jest-dom @vitejs/plugin-react jsdom
```

- [ ] **Step 3: Install Tailwind**

```powershell
npm install -D tailwindcss @tailwindcss/vite
```

Create `tailwind.config.js`:
```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
}
```

Replace contents of `src/index.css`:
```css
@import "tailwindcss";
```

- [ ] **Step 4: Add Rust dependencies**

Replace the `[dependencies]` section in `src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tokio = { version = "1", features = ["full"] }
notify = "6"
chrono = { version = "0.4", features = ["serde"] }
dirs = "5"
image = { version = "0.25", default-features = false, features = ["png"] }
ab_glyph = "0.2"
imageproc = "0.25"
```

- [ ] **Step 5: Configure Tauri window**

In `src-tauri/tauri.conf.json`, replace the `windows` array:
```json
{
  "productName": "PC Token Monitor",
  "version": "0.1.0",
  "identifier": "com.pcmonitor.tokenmonitor",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "PC Token Monitor",
        "width": 280,
        "height": 165,
        "decorations": false,
        "alwaysOnTop": true,
        "resizable": false,
        "transparent": false,
        "visible": true,
        "center": false,
        "x": 20,
        "y": 20
      }
    ],
    "trayIcon": {
      "iconPath": "icons/32x32.png",
      "iconAsTemplate": false
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/icon.ico"]
  }
}
```

- [ ] **Step 6: Configure Vitest**

Add to `vite.config.ts`:
```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
  },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
```

Create `src/test-setup.ts`:
```ts
import "@testing-library/jest-dom";
```

- [ ] **Step 7: Verify scaffold compiles**

```powershell
npm run tauri dev
```
Expected: App window opens (blank React app). Close with Ctrl+C.

- [ ] **Step 8: Commit**

```powershell
git add -A
git commit -m "feat: scaffold Tauri 2 + React + TypeScript + Tailwind project"
```

---

## Task 2: Rust Types

**Files:**
- Create: `src-tauri/src/types.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod types;`)

- [ ] **Step 1: Write the types module**

Create `src-tauri/src/types.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowUsage {
    pub utilization: f64,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub five_hour: WindowUsage,
    pub seven_day: WindowUsage,
    pub seven_day_opus_utilization: Option<f64>,
    pub extra_usage_enabled: bool,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    OAuth,
    JsonlFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Plan {
    Pro,
    Max50,
    Max200,
    Unknown,
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::Pro => write!(f, "PRO"),
            Plan::Max50 => write!(f, "MAX 50"),
            Plan::Max200 => write!(f, "MAX 200"),
            Plan::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub plan: Plan,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig { plan: Plan::Unknown }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendState {
    pub usage: Option<UsageData>,
    pub config: AppConfig,
}
```

- [ ] **Step 2: Add module to lib.rs**

In `src-tauri/src/lib.rs`, add at the top:
```rust
mod types;
```

- [ ] **Step 3: Verify it compiles**

```powershell
cd src-tauri && cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```powershell
cd .. && git add src-tauri/src/types.rs src-tauri/src/lib.rs
git commit -m "feat: add Rust types for usage data and app config"
```

---

## Task 3: OAuth Fetcher

**Files:**
- Create: `src-tauri/src/oauth_fetcher.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod oauth_fetcher;`)

- [ ] **Step 1: Write failing test**

Create `src-tauri/src/oauth_fetcher.rs` with tests only first:
```rust
use crate::types::{DataSource, UsageData, WindowUsage};
use chrono::DateTime;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct OAuthWindowRaw {
    utilization: f64,
    resets_at: String,
}

#[derive(Debug, Deserialize)]
struct OAuthUsageRaw {
    five_hour: OAuthWindowRaw,
    seven_day: OAuthWindowRaw,
    seven_day_opus: Option<OAuthWindowRaw>,
    extra_usage: Option<ExtraUsageRaw>,
}

#[derive(Debug, Deserialize)]
struct ExtraUsageRaw {
    is_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct OAuthResponseRaw {
    usage: OAuthUsageRaw,
}

#[derive(Debug, Deserialize)]
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthCredentials>,
}

#[derive(Debug, Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: String,
}

pub fn credentials_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join(".credentials.json")
}

pub fn load_access_token(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let creds: CredentialsFile = serde_json::from_str(&content).ok()?;
    creds.claude_ai_oauth.map(|o| o.access_token)
}

fn parse_oauth_response(json: &str) -> Result<UsageData, String> {
    let raw: OAuthResponseRaw =
        serde_json::from_str(json).map_err(|e| format!("parse error: {e}"))?;
    let five_hour = WindowUsage {
        utilization: raw.usage.five_hour.utilization,
        resets_at: DateTime::parse_from_rfc3339(&raw.usage.five_hour.resets_at)
            .map_err(|e| format!("bad resets_at: {e}"))?
            .with_timezone(&chrono::Utc),
    };
    let seven_day = WindowUsage {
        utilization: raw.usage.seven_day.utilization,
        resets_at: DateTime::parse_from_rfc3339(&raw.usage.seven_day.resets_at)
            .map_err(|e| format!("bad resets_at: {e}"))?
            .with_timezone(&chrono::Utc),
    };
    Ok(UsageData {
        five_hour,
        seven_day,
        seven_day_opus_utilization: raw
            .usage
            .seven_day_opus
            .map(|o| o.utilization),
        extra_usage_enabled: raw
            .usage
            .extra_usage
            .map(|e| e.is_enabled)
            .unwrap_or(false),
        source: DataSource::OAuth,
    })
}

pub async fn fetch_usage(access_token: &str) -> Result<UsageData, String> {
    let client = reqwest::Client::new();
    let endpoints = [
        "https://api.anthropic.com/api/oauth/claude_cli/client_data",
        "https://api.anthropic.com/api/oauth/usage",
    ];
    for url in &endpoints {
        let resp = client
            .get(*url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            let body = resp.text().await.map_err(|e| e.to_string())?;
            return parse_oauth_response(&body);
        }
    }
    Err("all endpoints failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RESPONSE: &str = r#"{
        "usage": {
            "five_hour": { "utilization": 0.73, "resets_at": "2026-06-02T15:30:00Z" },
            "seven_day": { "utilization": 0.91, "resets_at": "2026-06-09T10:00:00Z" },
            "seven_day_opus": { "utilization": 0.45, "resets_at": "2026-06-09T10:00:00Z" },
            "extra_usage": { "is_enabled": true, "monthly_limit": 100.0, "used_credits": 45.0, "utilization": 0.45 }
        }
    }"#;

    #[test]
    fn parses_utilization_correctly() {
        let data = parse_oauth_response(SAMPLE_RESPONSE).unwrap();
        assert!((data.five_hour.utilization - 0.73).abs() < f64::EPSILON);
        assert!((data.seven_day.utilization - 0.91).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_resets_at_correctly() {
        let data = parse_oauth_response(SAMPLE_RESPONSE).unwrap();
        assert_eq!(
            data.five_hour.resets_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-06-02T15:30:00Z"
        );
    }

    #[test]
    fn parses_extra_usage_enabled() {
        let data = parse_oauth_response(SAMPLE_RESPONSE).unwrap();
        assert!(data.extra_usage_enabled);
    }

    #[test]
    fn parses_opus_utilization() {
        let data = parse_oauth_response(SAMPLE_RESPONSE).unwrap();
        assert!((data.seven_day_opus_utilization.unwrap() - 0.45).abs() < f64::EPSILON);
    }

    #[test]
    fn handles_missing_opus_and_extra_usage() {
        let json = r#"{
            "usage": {
                "five_hour": { "utilization": 0.1, "resets_at": "2026-06-02T15:30:00Z" },
                "seven_day": { "utilization": 0.2, "resets_at": "2026-06-09T10:00:00Z" }
            }
        }"#;
        let data = parse_oauth_response(json).unwrap();
        assert!(data.seven_day_opus_utilization.is_none());
        assert!(!data.extra_usage_enabled);
    }

    #[test]
    fn returns_err_on_malformed_json() {
        assert!(parse_oauth_response("not json").is_err());
    }

    #[test]
    fn load_access_token_returns_none_for_missing_file() {
        let path = PathBuf::from("/nonexistent/.credentials.json");
        assert!(load_access_token(&path).is_none());
    }
}
```

- [ ] **Step 2: Add module declaration**

In `src-tauri/src/lib.rs` add:
```rust
mod oauth_fetcher;
```

- [ ] **Step 3: Run tests — verify they pass**

```powershell
cd src-tauri && cargo test oauth_fetcher
```
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```powershell
cd .. && git add src-tauri/src/oauth_fetcher.rs src-tauri/src/lib.rs
git commit -m "feat: OAuth fetcher with utilization parsing and tests"
```

---

## Task 4: JSONL Fallback Parser

**Files:**
- Create: `src-tauri/src/jsonl_parser.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod jsonl_parser;`)

The JSONL fallback reads raw token sums from conversation logs. Since we can't compute % without server-side limits, we return raw counts only. The frontend shows `[OFFLINE]` badge.

- [ ] **Step 1: Write failing tests**

Create `src-tauri/src/jsonl_parser.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct JMessage {
    usage: Option<JUsage>,
}

#[derive(Debug, Deserialize)]
struct JUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<JMessage>,
    timestamp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenEvent {
    pub tokens: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct FallbackUsage {
    pub five_hour_tokens: u64,
    pub seven_day_tokens: u64,
}

pub fn claude_projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("projects")
}

pub fn parse_line(line: &str) -> Option<TokenEvent> {
    let entry: JEntry = serde_json::from_str(line).ok()?;
    if entry.entry_type.as_deref() != Some("assistant") {
        return None;
    }
    let msg = entry.message?;
    let usage = msg.usage?;
    let input = usage.input_tokens.unwrap_or(0);
    let output = usage.output_tokens.unwrap_or(0);
    if input == 0 && output == 0 {
        return None;
    }
    let ts_str = entry.timestamp?;
    let at = DateTime::parse_from_rfc3339(&ts_str).ok()?.with_timezone(&Utc);
    Some(TokenEvent { tokens: input + output, at })
}

pub fn compute_fallback(events: &[TokenEvent], now: DateTime<Utc>) -> FallbackUsage {
    let five_hour_cutoff = now - chrono::Duration::hours(5);
    let seven_day_cutoff = now - chrono::Duration::days(7);
    FallbackUsage {
        five_hour_tokens: events
            .iter()
            .filter(|e| e.at > five_hour_cutoff)
            .map(|e| e.tokens)
            .sum(),
        seven_day_tokens: events
            .iter()
            .filter(|e| e.at > seven_day_cutoff)
            .map(|e| e.tokens)
            .sum(),
    }
}

pub fn scan_projects_dir(dir: &PathBuf) -> Vec<TokenEvent> {
    let mut events = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return events;
    };
    for project in entries.flatten() {
        collect_jsonl_events(&project.path(), &mut events);
    }
    events.sort_by_key(|e| e.at);
    events
}

fn collect_jsonl_events(dir: &PathBuf, events: &mut Vec<TokenEvent>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "jsonl") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    if let Some(ev) = parse_line(line) {
                        events.push(ev);
                    }
                }
            }
        } else if path.is_dir() {
            collect_jsonl_events(&path, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_line_extracts_assistant_tokens() {
        let line = r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50}},"timestamp":"2026-06-02T10:00:00Z"}"#;
        let ev = parse_line(line).unwrap();
        assert_eq!(ev.tokens, 150);
    }

    #[test]
    fn parse_line_ignores_non_assistant_entries() {
        let line = r#"{"type":"permission-mode","permissionMode":"default"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parse_line_ignores_zero_token_entries() {
        let line = r#"{"type":"assistant","message":{"usage":{"input_tokens":0,"output_tokens":0}},"timestamp":"2026-06-02T10:00:00Z"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn compute_fallback_sums_5hr_window() {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let events = vec![
            TokenEvent { tokens: 1000, at: now - chrono::Duration::hours(4) },
            TokenEvent { tokens: 2000, at: now - chrono::Duration::hours(6) },
            TokenEvent { tokens: 500,  at: now - chrono::Duration::hours(1) },
        ];
        let result = compute_fallback(&events, now);
        assert_eq!(result.five_hour_tokens, 1500);
        assert_eq!(result.seven_day_tokens, 3500);
    }

    #[test]
    fn compute_fallback_excludes_old_events() {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let events = vec![
            TokenEvent { tokens: 999, at: now - chrono::Duration::days(8) },
        ];
        let result = compute_fallback(&events, now);
        assert_eq!(result.seven_day_tokens, 0);
    }
}
```

- [ ] **Step 2: Add module**

In `src-tauri/src/lib.rs`:
```rust
mod jsonl_parser;
```

- [ ] **Step 3: Run tests**

```powershell
cd src-tauri && cargo test jsonl_parser
```
Expected: 5 tests pass.

- [ ] **Step 4: Commit**

```powershell
cd .. && git add src-tauri/src/jsonl_parser.rs src-tauri/src/lib.rs
git commit -m "feat: JSONL fallback parser with token window calculations"
```

---

## Task 5: App Config

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `src-tauri/src/config.rs`:
```rust
use crate::types::{AppConfig, Plan};
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PCTokenMonitor")
        .join("app_config.json")
}

pub fn load_config(path: &PathBuf) -> AppConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn plan_from_extra_usage(extra_usage_enabled: bool, saved: &Plan) -> Plan {
    if extra_usage_enabled && *saved == Plan::Unknown {
        Plan::Max50
    } else {
        saved.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_config_path() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app_config.json");
        (dir, path)
    }

    #[test]
    fn load_config_returns_default_when_file_missing() {
        let path = PathBuf::from("/nonexistent/app_config.json");
        let config = load_config(&path);
        assert_eq!(config.plan, Plan::Unknown);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (_dir, path) = temp_config_path();
        let config = AppConfig { plan: Plan::Max50 };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(loaded.plan, Plan::Max50);
    }

    #[test]
    fn plan_from_extra_usage_upgrades_unknown_to_max50() {
        let result = plan_from_extra_usage(true, &Plan::Unknown);
        assert_eq!(result, Plan::Max50);
    }

    #[test]
    fn plan_from_extra_usage_preserves_user_selection() {
        let result = plan_from_extra_usage(true, &Plan::Max200);
        assert_eq!(result, Plan::Max200);
    }
}
```

- [ ] **Step 2: Add tempfile dev dependency**

In `src-tauri/Cargo.toml` add:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Add module**

In `src-tauri/src/lib.rs`:
```rust
mod config;
```

- [ ] **Step 4: Run tests**

```powershell
cd src-tauri && cargo test config
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```powershell
cd .. && git add src-tauri/src/config.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: app config persistence with plan selection"
```

---

## Task 6: File Watcher

**Files:**
- Create: `src-tauri/src/file_watcher.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create file watcher module**

Create `src-tauri/src/file_watcher.rs`:
```rust
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

pub fn start_watcher(
    watch_dir: PathBuf,
    on_change: impl Fn() + Send + 'static,
) -> Result<RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| format!("watcher init failed: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .map_err(|e| format!("watch failed: {e}"))?;

    std::thread::spawn(move || {
        loop {
            match rx.recv_timeout(Duration::from_secs(60)) {
                Ok(Ok(event)) => {
                    let is_jsonl = event.paths.iter().any(|p| {
                        p.extension().map_or(false, |e| e == "jsonl")
                    });
                    let is_write = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    );
                    if is_jsonl && is_write {
                        on_change();
                    }
                }
                Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            }
        }
    });

    Ok(watcher)
}
```

- [ ] **Step 2: Add module**

In `src-tauri/src/lib.rs`:
```rust
mod file_watcher;
```

- [ ] **Step 3: Verify compile**

```powershell
cd src-tauri && cargo check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```powershell
cd .. && git add src-tauri/src/file_watcher.rs src-tauri/src/lib.rs
git commit -m "feat: file watcher for ~/.claude/projects JSONL changes"
```

---

## Task 7: System Tray

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`

The tray icon shows the highest utilization % as a dynamically generated 32x32 PNG. Color reflects urgency: cyan (<70%), orange (70-89%), red (≥90%).

- [ ] **Step 1: Create tray module**

Create `src-tauri/src/tray.rs`:
```rust
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::{ImageBuffer, Rgba};
use imageproc::drawing::draw_text_mut;
use tauri::{App, Manager};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItemBuilder, CheckMenuItemBuilder};

const FONT_BYTES: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");

pub fn icon_rgba_for_percent(percent: u8) -> Vec<u8> {
    let size = 32u32;
    let bg_color = if percent >= 90 {
        Rgba([220u8, 50, 50, 255])
    } else if percent >= 70 {
        Rgba([255u8, 140, 0, 255])
    } else {
        Rgba([0u8, 180, 220, 255])
    };

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(size, size, bg_color);

    let font = FontRef::try_from_slice(FONT_BYTES).expect("invalid font");
    let label = format!("{percent}");
    let scale = PxScale::from(if label.len() >= 3 { 11.0 } else { 14.0 });

    let scaled = font.as_scaled(scale);
    let text_w: f32 = label.chars().map(|c| scaled.h_advance(font.glyph_id(c))).sum();
    let x = ((size as f32 - text_w) / 2.0).max(0.0) as i32;
    let y = 8i32;

    draw_text_mut(&mut img, Rgba([255u8, 255, 255, 255]), x, y, scale, &font, &label);
    img.into_raw()
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_hide = MenuItemBuilder::new("Show / Hide").id("show_hide").build(app)?;
    let always_on_top = CheckMenuItemBuilder::new("Always on Top")
        .id("always_on_top")
        .checked(true)
        .build(app)?;
    let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;

    let menu = Menu::with_items(app, &[&show_hide, &always_on_top, &quit])?;

    let initial_icon = tauri::image::Image::new(
        &icon_rgba_for_percent(0),
        32,
        32,
    );

    TrayIconBuilder::with_id("main")
        .icon(initial_icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_hide" => {
                let win = app.get_webview_window("main").unwrap();
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "always_on_top" => {
                let win = app.get_webview_window("main").unwrap();
                let current = win.is_always_on_top().unwrap_or(false);
                let _ = win.set_always_on_top(!current);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let win = app.get_webview_window("main").unwrap();
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                } else {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_icon(app: &tauri::AppHandle, percent: u8) {
    if let Some(tray) = app.tray_by_id("main") {
        let rgba = icon_rgba_for_percent(percent);
        let icon = tauri::image::Image::new(&rgba, 32, 32);
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_tooltip(Some(&format!("PC Token Monitor — {percent}%")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_rgba_correct_size() {
        let rgba = icon_rgba_for_percent(73);
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }

    #[test]
    fn icon_rgba_red_at_90_plus() {
        let rgba = icon_rgba_for_percent(90);
        assert_eq!(rgba[0], 220);
        assert_eq!(rgba[1], 50);
    }

    #[test]
    fn icon_rgba_orange_at_70_to_89() {
        let rgba = icon_rgba_for_percent(75);
        assert_eq!(rgba[0], 255);
        assert_eq!(rgba[1], 140);
    }

    #[test]
    fn icon_rgba_cyan_below_70() {
        let rgba = icon_rgba_for_percent(50);
        assert_eq!(rgba[0], 0);
        assert_eq!(rgba[1], 180);
    }
}
```

- [ ] **Step 2: Download and add the font**

Create folder `src-tauri/fonts/`. Download JetBrains Mono Bold:
```powershell
mkdir src-tauri\fonts
Invoke-WebRequest -Uri "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/JetBrainsMono-Bold.ttf" -OutFile "src-tauri\fonts\JetBrainsMono-Bold.ttf"
```

- [ ] **Step 3: Add module**

In `src-tauri/src/lib.rs`:
```rust
mod tray;
```

- [ ] **Step 4: Run tray tests**

```powershell
cd src-tauri && cargo test tray
```
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```powershell
cd .. && git add src-tauri/src/tray.rs src-tauri/fonts/ src-tauri/src/lib.rs
git commit -m "feat: dynamic system tray icon with % and color coding"
```

---

## Task 8: Tauri Commands & Event Loop

**Files:**
- Modify: `src-tauri/src/lib.rs` (main app wiring)
- Modify: `src-tauri/src/main.rs`

This wires all Rust modules together: OAuth polling loop, file watcher, command handlers, and event emission to the frontend.

- [ ] **Step 1: Write lib.rs**

Replace `src-tauri/src/lib.rs` entirely:
```rust
mod config;
mod file_watcher;
mod jsonl_parser;
mod oauth_fetcher;
mod tray;
mod types;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use types::{AppConfig, DataSource, FrontendState, Plan, UsageData};

struct AppState {
    usage: Arc<Mutex<Option<UsageData>>>,
    config: Arc<Mutex<AppConfig>>,
}

#[tauri::command]
fn get_state(state: State<AppState>) -> FrontendState {
    FrontendState {
        usage: state.usage.lock().unwrap().clone(),
        config: state.config.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn save_plan(plan_str: String, state: State<AppState>) -> Result<(), String> {
    let plan = match plan_str.as_str() {
        "Pro" => Plan::Pro,
        "Max50" => Plan::Max50,
        "Max200" => Plan::Max200,
        _ => Plan::Unknown,
    };
    let mut config = state.config.lock().unwrap();
    config.plan = plan;
    config::save_config(&config::config_path(), &config)
}

#[tauri::command]
fn set_always_on_top(value: bool, app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(value);
    }
}

fn dominant_percent(usage: &UsageData) -> u8 {
    let pct = (usage.five_hour.utilization.max(usage.seven_day.utilization) * 100.0) as u8;
    pct.min(100)
}

fn start_poll_loop(app: AppHandle, state: Arc<Mutex<Option<UsageData>>>, config: Arc<Mutex<AppConfig>>) {
    tokio::spawn(async move {
        loop {
            let creds_path = oauth_fetcher::credentials_path();
            let new_usage = if let Some(token) = oauth_fetcher::load_access_token(&creds_path) {
                oauth_fetcher::fetch_usage(&token).await.ok()
            } else {
                None
            };

            let usage = if let Some(u) = new_usage {
                Some(u)
            } else {
                let dir = jsonl_parser::claude_projects_dir();
                let events = jsonl_parser::scan_projects_dir(&dir);
                let now = chrono::Utc::now();
                let fallback = jsonl_parser::compute_fallback(&events, now);
                let reset_5hr = now + chrono::Duration::hours(5);
                let reset_7day = now + chrono::Duration::days(7);
                Some(UsageData {
                    five_hour: types::WindowUsage {
                        utilization: 0.0,
                        resets_at: reset_5hr,
                    },
                    seven_day: types::WindowUsage {
                        utilization: 0.0,
                        resets_at: reset_7day,
                    },
                    seven_day_opus_utilization: None,
                    extra_usage_enabled: false,
                    source: DataSource::JsonlFallback,
                })
            };

            {
                let mut lock = state.lock().unwrap();
                *lock = usage.clone();
            }

            if let Some(ref u) = usage {
                let pct = dominant_percent(u);
                tray::update_tray_icon(&app, pct);

                let cfg = config.lock().unwrap().clone();
                let frontend = FrontendState {
                    usage: Some(u.clone()),
                    config: cfg,
                };
                let _ = app.emit("usage-updated", frontend);
            }

            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = config::load_config(&config::config_path());
            let usage_arc: Arc<Mutex<Option<UsageData>>> = Arc::new(Mutex::new(None));
            let config_arc = Arc::new(Mutex::new(config));

            app.manage(AppState {
                usage: usage_arc.clone(),
                config: config_arc.clone(),
            });

            tray::setup_tray(app)?;

            let app_handle = app.handle().clone();
            let usage_for_poll = usage_arc.clone();
            let config_for_poll = config_arc.clone();
            start_poll_loop(app_handle.clone(), usage_for_poll, config_for_poll);

            let watch_dir = jsonl_parser::claude_projects_dir();
            if watch_dir.exists() {
                let app_for_watch = app_handle.clone();
                let _ = file_watcher::start_watcher(watch_dir, move || {
                    let _ = app_for_watch.emit("jsonl-changed", ());
                });
            }

            let main_win = app.get_webview_window("main").unwrap();
            main_win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = main_win.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_state, save_plan, set_always_on_top])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Ensure main.rs calls run()**

`src-tauri/src/main.rs` should contain:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pc_token_monitor_lib::run();
}
```

- [ ] **Step 3: Verify compile**

```powershell
cd src-tauri && cargo check
```
Expected: no errors (warnings about unused imports are OK at this stage).

- [ ] **Step 4: Commit**

```powershell
cd .. && git add src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: Tauri command handlers and OAuth polling loop"
```

---

## Task 9: TypeScript Types & Zustand Store

**Files:**
- Create: `src/types.ts`
- Create: `src/stores/usageStore.ts`

- [ ] **Step 1: Write types**

Create `src/types.ts`:
```ts
export interface WindowUsage {
  utilization: number;   // 0.0 – 1.0
  resets_at: string;     // ISO 8601
}

export type DataSource = "oauth" | "jsonl_fallback";

export interface UsageData {
  five_hour: WindowUsage;
  seven_day: WindowUsage;
  seven_day_opus_utilization: number | null;
  extra_usage_enabled: boolean;
  source: DataSource;
}

export type Plan = "Pro" | "Max50" | "Max200" | "Unknown";

export interface AppConfig {
  plan: Plan;
}

export interface FrontendState {
  usage: UsageData | null;
  config: AppConfig;
}
```

- [ ] **Step 2: Write failing store tests**

Create `src/stores/usageStore.test.ts`:
```ts
import { describe, it, expect, beforeEach } from "vitest";
import { useUsageStore } from "./usageStore";
import type { FrontendState } from "../types";

const mockState: FrontendState = {
  usage: {
    five_hour: { utilization: 0.73, resets_at: "2026-06-02T15:30:00Z" },
    seven_day: { utilization: 0.91, resets_at: "2026-06-09T10:00:00Z" },
    seven_day_opus_utilization: null,
    extra_usage_enabled: false,
    source: "oauth",
  },
  config: { plan: "Max50" },
};

describe("usageStore", () => {
  beforeEach(() => {
    useUsageStore.getState().setFrontendState(null);
  });

  it("starts with null state", () => {
    expect(useUsageStore.getState().frontendState).toBeNull();
  });

  it("setFrontendState updates state", () => {
    useUsageStore.getState().setFrontendState(mockState);
    expect(useUsageStore.getState().frontendState?.usage?.five_hour.utilization).toBe(0.73);
  });

  it("dominantPercent returns highest utilization as integer %", () => {
    useUsageStore.getState().setFrontendState(mockState);
    expect(useUsageStore.getState().dominantPercent()).toBe(91);
  });

  it("dominantPercent returns 0 when no usage", () => {
    expect(useUsageStore.getState().dominantPercent()).toBe(0);
  });

  it("isOffline returns true when source is jsonl_fallback", () => {
    useUsageStore.getState().setFrontendState({
      ...mockState,
      usage: { ...mockState.usage!, source: "jsonl_fallback" },
    });
    expect(useUsageStore.getState().isOffline()).toBe(true);
  });
});
```

- [ ] **Step 3: Run test — verify it fails**

```powershell
npx vitest run src/stores/usageStore.test.ts
```
Expected: FAIL — `usageStore` not found.

- [ ] **Step 4: Write store implementation**

Create `src/stores/usageStore.ts`:
```ts
import { create } from "zustand";
import type { FrontendState } from "../types";

interface UsageStore {
  frontendState: FrontendState | null;
  setFrontendState: (state: FrontendState | null) => void;
  dominantPercent: () => number;
  isOffline: () => boolean;
}

export const useUsageStore = create<UsageStore>((set, get) => ({
  frontendState: null,

  setFrontendState: (state) => set({ frontendState: state }),

  dominantPercent: () => {
    const usage = get().frontendState?.usage;
    if (!usage) return 0;
    return Math.min(
      100,
      Math.round(Math.max(usage.five_hour.utilization, usage.seven_day.utilization) * 100)
    );
  },

  isOffline: () => get().frontendState?.usage?.source === "jsonl_fallback",
}));
```

- [ ] **Step 5: Run tests — verify pass**

```powershell
npx vitest run src/stores/usageStore.test.ts
```
Expected: 5 tests pass.

- [ ] **Step 6: Commit**

```powershell
git add src/types.ts src/stores/
git commit -m "feat: TypeScript types and Zustand usage store"
```

---

## Task 10: Tauri Events Hook

**Files:**
- Create: `src/hooks/useTauriEvents.ts`

- [ ] **Step 1: Write failing test**

Create `src/hooks/useTauriEvents.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

import { renderHook, act } from "@testing-library/react";
import { useTauriEvents } from "./useTauriEvents";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";

const mockState = {
  usage: {
    five_hour: { utilization: 0.5, resets_at: "2026-06-02T15:30:00Z" },
    seven_day: { utilization: 0.3, resets_at: "2026-06-09T10:00:00Z" },
    seven_day_opus_utilization: null,
    extra_usage_enabled: false,
    source: "oauth",
  },
  config: { plan: "Pro" },
};

describe("useTauriEvents", () => {
  beforeEach(() => {
    useUsageStore.getState().setFrontendState(null);
    vi.clearAllMocks();
  });

  it("calls invoke get_state on mount", async () => {
    vi.mocked(invoke).mockResolvedValue(mockState);
    vi.mocked(listen).mockResolvedValue(() => {});
    const { unmount } = renderHook(() => useTauriEvents());
    await act(async () => {});
    expect(invoke).toHaveBeenCalledWith("get_state");
    unmount();
  });

  it("updates store when usage-updated event fires", async () => {
    let capturedHandler: ((e: any) => void) | null = null;
    vi.mocked(listen).mockImplementation(async (_event, handler) => {
      capturedHandler = handler as any;
      return () => {};
    });
    vi.mocked(invoke).mockResolvedValue({ usage: null, config: { plan: "Unknown" } });

    const { unmount } = renderHook(() => useTauriEvents());
    await act(async () => {});

    act(() => capturedHandler?.({ payload: mockState }));
    expect(useUsageStore.getState().frontendState?.usage?.five_hour.utilization).toBe(0.5);
    unmount();
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```powershell
npx vitest run src/hooks/useTauriEvents.test.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Write hook implementation**

Create `src/hooks/useTauriEvents.ts`:
```ts
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";
import type { FrontendState } from "../types";

export function useTauriEvents() {
  const setFrontendState = useUsageStore((s) => s.setFrontendState);

  useEffect(() => {
    invoke<FrontendState>("get_state").then(setFrontendState).catch(console.error);

    const unlisten = listen<FrontendState>("usage-updated", (event) => {
      setFrontendState(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setFrontendState]);
}
```

- [ ] **Step 4: Run tests — verify pass**

```powershell
npx vitest run src/hooks/useTauriEvents.test.ts
```
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/hooks/
git commit -m "feat: Tauri events hook wiring backend to Zustand store"
```

---

## Task 11: React Components

**Files:**
- Create: `src/components/PlanBadge.tsx` + `.test.tsx`
- Create: `src/components/UsageBar.tsx` + `.test.tsx`
- Create: `src/components/FirstRunDialog.tsx`

### 11a: PlanBadge

- [ ] **Step 1: Write failing test**

Create `src/components/PlanBadge.test.tsx`:
```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { PlanBadge } from "./PlanBadge";

describe("PlanBadge", () => {
  it("shows PRO for Pro plan", () => {
    render(<PlanBadge plan="Pro" offline={false} />);
    expect(screen.getByText("[PRO]")).toBeInTheDocument();
  });

  it("shows MAX 50 for Max50 plan", () => {
    render(<PlanBadge plan="Max50" offline={false} />);
    expect(screen.getByText("[MAX 50]")).toBeInTheDocument();
  });

  it("shows MAX 200 for Max200 plan", () => {
    render(<PlanBadge plan="Max200" offline={false} />);
    expect(screen.getByText("[MAX 200]")).toBeInTheDocument();
  });

  it("shows OFFLINE badge when offline", () => {
    render(<PlanBadge plan="Pro" offline={true} />);
    expect(screen.getByText("[OFFLINE]")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement PlanBadge**

Create `src/components/PlanBadge.tsx`:
```tsx
import type { Plan } from "../types";

const LABELS: Record<Plan, string> = {
  Pro: "PRO",
  Max50: "MAX 50",
  Max200: "MAX 200",
  Unknown: "UNKNOWN",
};

interface Props {
  plan: Plan;
  offline: boolean;
}

export function PlanBadge({ plan, offline }: Props) {
  const label = offline ? "OFFLINE" : LABELS[plan];
  return (
    <span className="font-mono text-[10px] px-1 py-0.5 rounded bg-[#333] text-white tracking-widest">
      [{label}]
    </span>
  );
}
```

- [ ] **Step 3: Run tests**

```powershell
npx vitest run src/components/PlanBadge.test.tsx
```
Expected: 4 tests pass.

### 11b: UsageBar

- [ ] **Step 4: Write failing test**

Create `src/components/UsageBar.test.tsx`:
```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { UsageBar } from "./UsageBar";

describe("UsageBar", () => {
  const baseProps = {
    label: "5HR",
    utilization: 0.73,
    resetsAt: "2026-06-02T15:30:00Z",
    labelColor: "#00d4ff",
  };

  it("renders label", () => {
    render(<UsageBar {...baseProps} />);
    expect(screen.getByText("5HR")).toBeInTheDocument();
  });

  it("renders percent text", () => {
    render(<UsageBar {...baseProps} />);
    expect(screen.getByText("73%")).toBeInTheDocument();
  });

  it("clamps utilization at 100%", () => {
    render(<UsageBar {...baseProps} utilization={1.5} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("shows Limit reached at 100%", () => {
    render(<UsageBar {...baseProps} utilization={1.0} />);
    expect(screen.getByText(/limit reached/i)).toBeInTheDocument();
  });

  it("shows progress bar with correct aria-valuenow", () => {
    render(<UsageBar {...baseProps} />);
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "73");
  });
});
```

- [ ] **Step 5: Implement UsageBar**

Create `src/components/UsageBar.tsx`:
```tsx
import { useMemo } from "react";

interface Props {
  label: string;
  utilization: number;
  resetsAt: string;
  labelColor: string;
}

function barColor(pct: number): string {
  if (pct >= 90) return "#ff3232";
  if (pct >= 70) return "#ff8c00";
  return "#00c853";
}

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

export function UsageBar({ label, utilization, resetsAt, labelColor }: Props) {
  const pct = Math.min(100, Math.round(utilization * 100));
  const color = barColor(pct);
  const countdown = useMemo(() => formatCountdown(resetsAt), [resetsAt]);
  const atLimit = pct >= 100;

  return (
    <div className="flex flex-col gap-0.5">
      <div className="flex items-center gap-2">
        <span className="font-mono text-xs w-8 shrink-0" style={{ color: labelColor }}>
          {label}
        </span>
        <div
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
          className="flex-1 h-2 bg-[#222] rounded-sm overflow-hidden"
        >
          <div
            className="h-full rounded-sm transition-all duration-500"
            style={{ width: `${pct}%`, backgroundColor: color }}
          />
        </div>
        <span
          className="font-mono text-xs w-8 text-right shrink-0"
          style={{ color: atLimit ? "#ff3232" : "#ffffff" }}
        >
          {pct}%
        </span>
      </div>
      <div className="font-mono text-[9px] text-[#666] pl-10">
        {atLimit ? (
          <span className="text-[#ff3232]">Limit reached</span>
        ) : (
          <>reset in {countdown}</>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 6: Run tests**

```powershell
npx vitest run src/components/UsageBar.test.tsx
```
Expected: 5 tests pass.

### 11c: FirstRunDialog

- [ ] **Step 7: Create FirstRunDialog**

Create `src/components/FirstRunDialog.tsx`:
```tsx
import { invoke } from "@tauri-apps/api/core";
import type { Plan } from "../types";

const PLANS: { value: Plan; label: string; desc: string }[] = [
  { value: "Pro", label: "Pro", desc: "Standard Claude plan" },
  { value: "Max50", label: "Max 50", desc: "5× usage multiplier" },
  { value: "Max200", label: "Max 200", desc: "20× usage multiplier" },
];

interface Props {
  onDone: () => void;
}

export function FirstRunDialog({ onDone }: Props) {
  async function selectPlan(plan: Plan) {
    await invoke("save_plan", { planStr: plan });
    onDone();
  }

  return (
    <div className="fixed inset-0 bg-[#0a0a0a] flex flex-col items-center justify-center gap-4 p-4">
      <p className="font-mono text-[#00d4ff] text-sm tracking-widest">SELECT PLAN</p>
      {PLANS.map((p) => (
        <button
          key={p.value}
          onClick={() => selectPlan(p.value)}
          className="w-full font-mono text-xs text-white bg-[#1a1a1a] hover:bg-[#333] border border-[#333] rounded px-3 py-2 text-left transition-colors"
        >
          <span className="text-[#ffd700]">[{p.label}]</span>{" "}
          <span className="text-[#888]">{p.desc}</span>
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 8: Commit**

```powershell
git add src/components/
git commit -m "feat: PlanBadge, UsageBar, and FirstRunDialog components with tests"
```

---

## Task 12: OverlayWindow & App Root

**Files:**
- Create: `src/components/OverlayWindow.tsx`
- Modify: `src/App.tsx`
- Modify: `src/index.css`

- [ ] **Step 1: Create OverlayWindow**

Create `src/components/OverlayWindow.tsx`:
```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useUsageStore } from "../stores/usageStore";
import { UsageBar } from "./UsageBar";
import { PlanBadge } from "./PlanBadge";

export function OverlayWindow() {
  const { frontendState, isOffline } = useUsageStore();
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);

  const usage = frontendState?.usage;
  const plan = frontendState?.config.plan ?? "Unknown";
  const offline = isOffline();

  function toggleAlwaysOnTop() {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    invoke("set_always_on_top", { value: next });
  }

  function minimize() {
    invoke("plugin:window|hide").catch(() => {});
  }

  return (
    <div
      data-tauri-drag-region
      className="w-full h-full bg-[#0a0a0a] border border-[#1e1e1e] flex flex-col select-none"
    >
      {/* Header */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between px-2 py-1 border-b border-[#1e1e1e]"
      >
        <span className="font-mono text-[10px] text-[#555] tracking-widest">
          ⬡ PC TOKEN MONITOR
        </span>
        <PlanBadge plan={plan} offline={offline} />
      </div>

      {/* Usage bars */}
      <div className="flex flex-col gap-2 px-2 py-2 flex-1">
        {usage ? (
          <>
            <UsageBar
              label="5HR"
              utilization={usage.five_hour.utilization}
              resetsAt={usage.five_hour.resets_at}
              labelColor="#00d4ff"
            />
            <UsageBar
              label="7DAY"
              utilization={usage.seven_day.utilization}
              resetsAt={usage.seven_day.resets_at}
              labelColor="#ffd700"
            />
          </>
        ) : (
          <span className="font-mono text-[10px] text-[#444] text-center py-2">
            connecting...
          </span>
        )}
      </div>

      {/* Footer controls */}
      <div className="flex items-center justify-between px-2 py-1 border-t border-[#1e1e1e]">
        <button
          onClick={toggleAlwaysOnTop}
          className={`font-mono text-[9px] tracking-widest transition-colors ${
            alwaysOnTop ? "text-[#00d4ff]" : "text-[#444]"
          }`}
        >
          [⊤ ALWAYS ON TOP]
        </button>
        <button
          onClick={minimize}
          className="font-mono text-[9px] text-[#444] hover:text-white transition-colors"
        >
          [×]
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Write App.tsx**

Replace `src/App.tsx`:
```tsx
import { useEffect, useState } from "react";
import { useUsageStore } from "./stores/usageStore";
import { useTauriEvents } from "./hooks/useTauriEvents";
import { OverlayWindow } from "./components/OverlayWindow";
import { FirstRunDialog } from "./components/FirstRunDialog";

export default function App() {
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

- [ ] **Step 3: Update global CSS**

Replace `src/index.css`:
```css
@import "tailwindcss";

* {
  box-sizing: border-box;
}

html, body, #root {
  width: 100%;
  height: 100%;
  margin: 0;
  padding: 0;
  overflow: hidden;
  font-family: 'JetBrains Mono', Consolas, monospace;
  background: #0a0a0a;
}
```

- [ ] **Step 4: Run the app**

```powershell
npm run tauri dev
```
Expected: overlay window appears — dark, 280×165px, shows "connecting..." while OAuth fetches, then renders progress bars. System tray icon shows % and updates.

- [ ] **Step 5: Verify always-on-top toggle works**

Click `[⊤ ALWAYS ON TOP]` — verify window stays on top when active (color changes to dim when disabled). Test with another window.

- [ ] **Step 6: Verify close → minimize to tray**

Click `[×]` — window hides. Tray icon still present. Left-click tray icon → window reappears. Right-click → Quit → app exits fully.

- [ ] **Step 7: Commit**

```powershell
git add src/
git commit -m "feat: OverlayWindow and App root — overlay UI complete"
```

---

## Task 13: Build & Package

- [ ] **Step 1: Build release**

```powershell
npm run tauri build
```
Expected: `src-tauri/target/release/bundle/` contains an `.msi` installer and `.exe`.

- [ ] **Step 2: Run the bundled exe**

Open `src-tauri/target/release/pc-token-monitor.exe`. Verify:
- Window appears at saved position
- Tray icon visible in system tray
- Usage data loads within 30 seconds
- Always-on-top works after relaunch

- [ ] **Step 3: Final commit**

```powershell
git add -A
git commit -m "feat: production build verified"
```

---

## Self-Review Notes

- All spec requirements covered: 5hr bar ✓, 7-day bar ✓, reset countdown ✓, plan badge ✓, always-on-top toggle ✓, tray % icon ✓, minimize to tray on close ✓, first-run plan selection ✓, zero token consumption ✓ (no `/v1/messages` calls anywhere), offline fallback ✓
- Types consistent across all tasks: `WindowUsage`, `UsageData`, `FrontendState`, `Plan` used identically in Rust and TypeScript
- `save_plan` Tauri command uses `planStr` parameter — matches `invoke("save_plan", { planStr: plan })` in FirstRunDialog ✓
- `set_always_on_top` uses `value` parameter — matches `invoke("set_always_on_top", { value: next })` in OverlayWindow ✓
