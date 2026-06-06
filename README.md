# PC Token Monitor

A tiny always-on-top desktop overlay that shows your Claude usage at a glance —
MSI-Afterburner style. It reads your local Claude credentials and displays the
**5-hour** and **7-day** usage windows as live bars, plus a color-coded
percentage in the system tray.

> **Windows only.** The overlay uses Win32 (tray icon rendering, GDI background
> sampling). It will build on other platforms but the tray and adaptive text
> features are Windows-specific.

## Features

- Frameless, transparent, always-on-top overlay (drag anywhere on the header)
- 5HR + 7DAY usage bars with reset countdowns
- Plan badge (`PRO` / `MAX 50` / `MAX 200` / `OFFLINE`), auto-detected
- System-tray icon showing the dominant usage % (cyan / orange / red)
- **Adaptive text color** — samples the desktop behind the window and flips
  between light and dark text so it stays readable over any wallpaper
- Minimize-to-tray on close
- Zero token consumption — it only reads usage metadata, it does not call the
  model

## Install

Download the latest installer from the [Releases](https://github.com/PCInfiniteSoft/pc-token-monitor/releases) page:

- `PC Token Monitor_<version>_x64-setup.exe` (NSIS, per-user, recommended), or
- `PC Token Monitor_<version>_x64_en-US.msi`

### ⚠️ The builds are unsigned

There is no code-signing certificate, so Windows will warn:

- **SmartScreen** ("Windows protected your PC"): click **More info → Run anyway**.
- **Smart App Control (SAC)**: if SAC is *on and enforcing*, it blocks unsigned
  apps with `os error 4551` and there is no per-app allowlist. You must either
  turn SAC off (Windows Security → App & browser control → Smart App Control →
  Off — note this is irreversible without resetting Windows) **or build from
  source** (below).

## Privacy

PC Token Monitor reads your usage data locally and sends nothing anywhere except
to Anthropic's own usage endpoint:

- Reads the access token from `~/.claude/.credentials.json`
  (`claudeAiOauth.accessToken`) — it stays on your machine.
- Queries `https://api.anthropic.com/api/oauth/usage` for utilization numbers.
- Falls back to reading `~/.claude/projects/**/*.jsonl` locally when the API is
  unavailable.

No telemetry, no third-party servers.

## Build from source

Prerequisites: [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) 20+,
and the Tauri 2 Windows prerequisites (WebView2 + MSVC build tools).

```bash
npm install
npm run tauri dev      # run in development (hot reload)
npm run tauri build    # produce installers in src-tauri/target/release/bundle/
```

## Tech stack

- [Tauri 2](https://tauri.app/) (Rust backend + WebView2)
- React 19 + TypeScript + Tailwind CSS
- Zustand (state), Vitest + Rust `#[test]` (tests)

## License

[MIT](./LICENSE) © PC Infinite Soft
