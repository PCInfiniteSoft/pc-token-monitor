# macOS Keychain Token Source — Design

**Date:** 2026-07-24
**Status:** Approved (ready for implementation plan)

## Problem

On macOS, PC Token Monitor never shows real usage — it always renders OFFLINE / 0% (JSONL fallback). The app reads the Claude OAuth access token from the file `~/.claude/.credentials.json`, but on macOS **Claude Code stores its credentials in the login Keychain, not that file**. The file does not exist, so `load_access_token` returns `None` every poll and the app falls back to offline data.

Evidence gathered:
- `~/.claude/.credentials.json` is absent on the target Mac.
- A generic-password Keychain item with service `Claude Code-credentials` exists in the login keychain.
- The Keychain item's secret is the **same JSON blob** the file would hold: `{"claudeAiOauth": {"accessToken": ..., "subscriptionType": ..., "rateLimitTier": ...}}`.

## Goal

On macOS, fetch the real token and plan by reading the Keychain when the file is absent, so the popover shows live 5-hour / 7-day usage. Windows and Linux behavior is unchanged.

Non-goals (YAGNI):
- No token caching across polls (re-read each poll, as the file path already does, so refreshed tokens are picked up).
- No Settings UI to paste a token manually (OAuth tokens expire and refresh; a pasted token would go stale).
- No changes to Windows/Linux, to the usage-fetch HTTP path, or to the JSONL fallback.

## Constraints

- OAuth access tokens expire and are refreshed by Claude Code. The source must be re-read each poll (already true for the file path; keep it true for Keychain).
- Reading another app's Keychain item triggers a one-time macOS authorization prompt. This is acceptable; the user clicks **Always Allow** once.
- macOS-only code is gated with `#[cfg(target_os = "macos")]`; non-macOS builds must not reference the Keychain path and must produce no new warnings.

## Architecture

Separate the **credential source** (file vs Keychain) from **parsing** (unchanged). All changes live in `src-tauri/src/oauth_fetcher.rs`, plus two call-site updates in `src-tauri/src/lib.rs`.

### New / changed functions in `oauth_fetcher.rs`

```
fn read_file_credentials(path: &PathBuf) -> Option<String>
    Read the raw JSON string from a file. Returns None if unreadable.

#[cfg(target_os = "macos")]
fn read_keychain_credentials() -> Option<String>
    Spawn `/usr/bin/security find-generic-password -w -s "Claude Code-credentials"`,
    capture stdout. On success (exit 0), return the trimmed stdout (the JSON blob).
    On non-zero exit, spawn failure, empty output, or user denial: return None.

fn read_credentials_json() -> Option<String>
    Source selection:
    - macOS:  read_file_credentials(credentials_path())  ...then, if None,
              read_keychain_credentials()
    - others: read_file_credentials(credentials_path())

fn parse_access_token(json: &str) -> Option<String>
    Parse CredentialsFile from a JSON string; return claudeAiOauth.accessToken.

fn parse_plan(json: &str) -> Plan
    Parse CredentialsFile from a JSON string; map to Plan via detect_plan().
    Returns Plan::Unknown on parse failure or missing oauth.

pub fn load_access_token() -> Option<String>          // signature change: no path arg
    read_credentials_json().and_then(|j| parse_access_token(&j))

pub fn load_plan() -> Plan                             // signature change: no path arg
    read_credentials_json().map(|j| parse_plan(&j)).unwrap_or(Plan::Unknown)
```

`credentials_path()`, `detect_plan()`, `fetch_usage()`, and the `CredentialsFile` / `OAuthCredentials` structs are unchanged.

### Call-site updates in `lib.rs`

- Line ~119–120 (poll loop): drop `let creds_path = ...;` and call `oauth_fetcher::load_access_token()` with no argument.
- Line ~204 (plan detection at init): call `oauth_fetcher::load_plan()` with no argument.

The poll loop reads the source every 30 s (token only); plan detection reads it once at init. These are separate reads at different times, so no per-poll deduplication is required. After **Always Allow**, each `security` spawn is silent and cheap.

## Data flow (macOS, happy path)

1. Poll tick → `load_access_token()` → `read_credentials_json()`.
2. File absent → `read_keychain_credentials()` spawns `security -w -s "Claude Code-credentials"`.
3. First time: macOS prompts; user clicks **Always Allow**. `security` prints the JSON blob to stdout.
4. `parse_access_token()` extracts `accessToken` → `fetch_usage()` → live `UsageData` (source `OAuth`).
5. Popover shows real 5-hour / 7-day percentages; OFFLINE badge clears.

## Error handling

- File unreadable/absent → `None` (fall through to Keychain on macOS, or offline elsewhere).
- Keychain item missing, `security` non-zero exit, or user denies → `read_keychain_credentials()` returns `None` → `load_access_token()` returns `None` → existing offline/fallback behavior (no crash, no change from today).
- Malformed JSON from either source → `parse_access_token` / `parse_plan` return `None` / `Plan::Unknown`.

## Testing

Unit tests (in `oauth_fetcher.rs`, extending the existing `mod tests`):
- `parse_access_token` — returns the token for valid JSON; `None` for malformed JSON and for JSON missing `claudeAiOauth`.
- `parse_plan` — maps a valid `max`/`20x` blob to `Plan::Max200`; returns `Plan::Unknown` for malformed JSON. (Reuses `detect_plan` coverage already present.)
- `read_file_credentials` — returns the contents for a temp file; `None` for a nonexistent path. (Replaces the old `load_access_token_returns_none_for_missing_file` test, whose signature no longer applies.)

Not unit-tested (thin subprocess wrapper, requires a real Keychain):
- `read_keychain_credentials` — verified manually.

Manual verification (macOS):
- Launch the app; the log shows `[poll] token found, fetching usage...` and `[poll] fetch OK`.
- Popover shows real percentages and the correct plan badge (not OFFLINE / 0%).
- Regression: on a machine where `~/.claude/.credentials.json` exists, the file is still used and no Keychain prompt appears.

## Rollout note

This change is what unblocks a real `tauri build` release being useful on macOS. Producing the release build is a follow-up, out of scope for this spec.
