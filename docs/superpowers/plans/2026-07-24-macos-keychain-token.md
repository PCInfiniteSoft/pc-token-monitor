# macOS Keychain Token Source Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On macOS, read the Claude OAuth token/plan from the login Keychain when `~/.claude/.credentials.json` is absent, so the app shows live usage instead of OFFLINE.

**Architecture:** Separate the credential *source* (file vs Keychain) from *parsing* (unchanged). Add string-based parse helpers and a file reader, then a `read_credentials_json()` source selector that on macOS falls back from file to a `security`-subprocess Keychain read. Rewire the two `oauth_fetcher` public functions and their two `lib.rs` call sites.

**Tech Stack:** Rust + Tauri 2, `serde_json`, `std::process::Command` (shelling out to `/usr/bin/security`). No new crate dependencies.

## Global Constraints

- All changes are in `src-tauri/src/oauth_fetcher.rs` and two call sites in `src-tauri/src/lib.rs`. No other files change.
- macOS-only code is gated with `#[cfg(target_os = "macos")]`; non-macOS builds must reference no Keychain code and produce **no new warnings**.
- Keychain service name, verbatim: `Claude Code-credentials`. Read via `/usr/bin/security find-generic-password -w -s "Claude Code-credentials"`, matched by service only (no `-a` account).
- Fallback order on macOS: **file first, then Keychain**. Non-macOS: file only (unchanged behavior).
- The credential source is re-read every poll (do not cache) so refreshed tokens are picked up.
- No new crate dependencies. No Settings UI. No Windows/Linux behavior change.
- Rust tests: `cd src-tauri && cargo test`. Build check: `cd src-tauri && cargo build`.
- The existing `CredentialsFile` / `OAuthCredentials` structs, `credentials_path()`, `detect_plan()`, and `fetch_usage()` are unchanged.

---

### Task 1: Extract string-based parsing and a file reader

Additive refactor: introduce `parse_access_token`, `parse_plan`, and `read_file_credentials`, and route the existing `load_access_token(path)` / `load_plan(path)` through them. Public signatures stay `path`-based in this task, so `lib.rs` and all existing tests keep compiling.

**Files:**
- Modify: `src-tauri/src/oauth_fetcher.rs`

**Interfaces:**
- Consumes: existing `CredentialsFile`, `OAuthCredentials`, `detect_plan`, `Plan`, `PathBuf`.
- Produces:
  - `fn parse_access_token(json: &str) -> Option<String>`
  - `fn parse_plan(json: &str) -> Plan`
  - `fn read_file_credentials(path: &PathBuf) -> Option<String>`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests { ... }` block in `src-tauri/src/oauth_fetcher.rs`:

```rust
    const SAMPLE_CREDS: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "sk-ant-oat-abc123",
            "subscriptionType": "max",
            "rateLimitTier": "max_20x"
        }
    }"#;

    #[test]
    fn parse_access_token_reads_token() {
        assert_eq!(
            parse_access_token(SAMPLE_CREDS),
            Some("sk-ant-oat-abc123".to_string())
        );
    }

    #[test]
    fn parse_access_token_none_for_malformed() {
        assert!(parse_access_token("not json").is_none());
    }

    #[test]
    fn parse_access_token_none_when_oauth_missing() {
        assert!(parse_access_token(r#"{"other": 1}"#).is_none());
    }

    #[test]
    fn parse_plan_maps_max_20x() {
        assert_eq!(parse_plan(SAMPLE_CREDS), Plan::Max200);
    }

    #[test]
    fn parse_plan_unknown_for_malformed() {
        assert_eq!(parse_plan("not json"), Plan::Unknown);
    }

    #[test]
    fn read_file_credentials_none_for_missing_path() {
        let path = PathBuf::from("/nonexistent/.credentials.json");
        assert!(read_file_credentials(&path).is_none());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test parse_access_token parse_plan read_file_credentials 2>&1 | tail -20`
Expected: compile error — `cannot find function parse_access_token` / `parse_plan` / `read_file_credentials` in this scope.

- [ ] **Step 3: Add the three functions**

In `src-tauri/src/oauth_fetcher.rs`, add these functions (place them next to `load_access_token`):

```rust
fn read_file_credentials(path: &PathBuf) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn parse_access_token(json: &str) -> Option<String> {
    let creds: CredentialsFile = serde_json::from_str(json).ok()?;
    creds.claude_ai_oauth.map(|o| o.access_token)
}

fn parse_plan(json: &str) -> Plan {
    match serde_json::from_str::<CredentialsFile>(json) {
        Ok(creds) => match creds.claude_ai_oauth {
            Some(o) => detect_plan(o.subscription_type.as_deref(), o.rate_limit_tier.as_deref()),
            None => Plan::Unknown,
        },
        Err(_) => Plan::Unknown,
    }
}
```

Then rewrite the two existing public functions to route through the helpers (keep their `path` signatures unchanged):

```rust
pub fn load_access_token(path: &PathBuf) -> Option<String> {
    read_file_credentials(path).and_then(|j| parse_access_token(&j))
}

pub fn load_plan(path: &PathBuf) -> Plan {
    read_file_credentials(path)
        .map(|j| parse_plan(&j))
        .unwrap_or(Plan::Unknown)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result:"`
Expected: all tests pass (the new 6 tests plus the pre-existing suite; the old `load_access_token_returns_none_for_missing_file` still passes because `load_access_token` still takes a path).

- [ ] **Step 5: Verify the build has no new warnings**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "warning:.*oauth_fetcher" || echo "no oauth_fetcher warnings"`
Expected: `no oauth_fetcher warnings`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/oauth_fetcher.rs
git commit -m "refactor(oauth): extract string-based credential parsing and file reader"
```

---

### Task 2: Switch to a no-arg source selector (file-only)

Change the two public functions to take no path argument and read through a new `read_credentials_json()` that, in this task, reads only the file. Update the two `lib.rs` call sites. Behavior is identical on every platform; this is a pure refactor that sets up the Keychain fallback in Task 3.

**Files:**
- Modify: `src-tauri/src/oauth_fetcher.rs`
- Modify: `src-tauri/src/lib.rs` (call sites at ~line 119–120 and ~line 204)

**Interfaces:**
- Consumes: `read_file_credentials`, `parse_access_token`, `parse_plan`, `credentials_path` (Task 1 / existing).
- Produces:
  - `fn read_credentials_json() -> Option<String>`
  - `pub fn load_access_token() -> Option<String>` (no arg)
  - `pub fn load_plan() -> Plan` (no arg)

- [ ] **Step 1: Update the callers first (so the compiler guides the change)**

In `src-tauri/src/lib.rs`, poll loop (~line 119–120), replace:

```rust
            let creds_path = oauth_fetcher::credentials_path();
            let new_usage = if let Some(token) = oauth_fetcher::load_access_token(&creds_path) {
```

with:

```rust
            let new_usage = if let Some(token) = oauth_fetcher::load_access_token() {
```

In `src-tauri/src/lib.rs`, plan detection (~line 204), replace:

```rust
                detected = oauth_fetcher::load_plan(&oauth_fetcher::credentials_path());
```

with:

```rust
                detected = oauth_fetcher::load_plan();
```

- [ ] **Step 2: Change the signatures and add the selector**

In `src-tauri/src/oauth_fetcher.rs`, add `read_credentials_json` (file-only for now) and drop the `path` parameters:

```rust
fn read_credentials_json() -> Option<String> {
    read_file_credentials(&credentials_path())
}

pub fn load_access_token() -> Option<String> {
    read_credentials_json().and_then(|j| parse_access_token(&j))
}

pub fn load_plan() -> Plan {
    read_credentials_json()
        .map(|j| parse_plan(&j))
        .unwrap_or(Plan::Unknown)
}
```

- [ ] **Step 3: Remove the now-invalid test**

In the `#[cfg(test)] mod tests` block, delete the `load_access_token_returns_none_for_missing_file` test (it calls `load_access_token(&path)`, which no longer accepts an argument; the `read_file_credentials_none_for_missing_path` test from Task 1 covers the missing-file case).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result:"`
Expected: all tests pass.

- [ ] **Step 5: Verify the whole crate builds with no new warnings**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "warning:" | grep -vE "jsonl_parser|plan_from_extra_usage" || echo "no new warnings"`
Expected: `no new warnings` (the 4 pre-existing `jsonl_parser`/`plan_from_extra_usage` dead-code warnings are unrelated and may remain).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/oauth_fetcher.rs src-tauri/src/lib.rs
git commit -m "refactor(oauth): read credentials through a no-arg source selector"
```

---

### Task 3: Add the macOS Keychain fallback

Add `read_keychain_credentials()` (macOS only) and wire it into `read_credentials_json()` as the fallback when the file is absent. This is the task that makes real usage appear on macOS.

**Files:**
- Modify: `src-tauri/src/oauth_fetcher.rs`

**Interfaces:**
- Consumes: `read_file_credentials`, `credentials_path` (existing).
- Produces: `#[cfg(target_os = "macos")] fn read_keychain_credentials() -> Option<String>`; updated `read_credentials_json()`.

- [ ] **Step 1: Add the Keychain reader**

In `src-tauri/src/oauth_fetcher.rs`, add:

```rust
/// Read Claude Code's credential blob from the macOS login Keychain.
/// Claude Code stores the same JSON as `.credentials.json` under this service.
/// The first read triggers a one-time Keychain authorization prompt.
#[cfg(target_os = "macos")]
fn read_keychain_credentials() -> Option<String> {
    let output = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-w", "-s", "Claude Code-credentials"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
```

- [ ] **Step 2: Wire the fallback into the selector**

Replace the Task 2 `read_credentials_json` with the file-then-Keychain version:

```rust
fn read_credentials_json() -> Option<String> {
    if let Some(s) = read_file_credentials(&credentials_path()) {
        return Some(s);
    }
    #[cfg(target_os = "macos")]
    {
        read_keychain_credentials()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}
```

- [ ] **Step 3: Verify tests still pass and the crate builds cleanly**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result:"`
Expected: all tests pass (no new unit tests — `read_keychain_credentials` is a thin subprocess wrapper verified manually).

Run: `cd src-tauri && cargo build 2>&1 | grep -E "warning:" | grep -vE "jsonl_parser|plan_from_extra_usage" || echo "no new warnings"`
Expected: `no new warnings`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/oauth_fetcher.rs
git commit -m "feat(macos): read Claude token from the login Keychain when the file is absent"
```

- [ ] **Step 5: Manual verification (macOS, requires a machine logged into Claude Code with Keychain storage)**

Run: `npm run tauri dev`
1. On first poll a macOS prompt appears: "security wants to use your confidential information stored in Claude Code-credentials". Click **Always Allow**.
2. Confirm the terminal log shows `[poll] token found, fetching usage...` then `[poll] fetch OK: 5h=... 7d=...` (not `[poll] no token, using fallback`).
3. Open the popover: it shows real 5-Hour / 7-Day percentages and the correct plan badge (not OFFLINE / 0%).
4. Regression check (if a `~/.claude/.credentials.json` file is present on some machine): the file is used and no Keychain prompt appears.

---

## Self-review notes

- **Spec coverage:** file reader + string parsing (Task 1) → `read_file_credentials`, `parse_access_token`, `parse_plan`; no-arg `load_*` + source selector (Task 2); Keychain read + file→Keychain fallback + macOS gating (Task 3); manual verification (Task 3 Step 5). All spec sections mapped.
- **Non-macOS cleanliness:** `read_keychain_credentials` is `#[cfg(target_os = "macos")]` and only referenced inside the `#[cfg(target_os = "macos")]` arm of `read_credentials_json`; the `#[cfg(not(target_os = "macos"))]` arm returns `None`, so no unused-function warnings on either platform.
- **Type consistency:** `read_credentials_json() -> Option<String>`, `parse_access_token(&str) -> Option<String>`, `parse_plan(&str) -> Plan`, `read_file_credentials(&PathBuf) -> Option<String>`, and no-arg `load_access_token()/load_plan()` are used consistently across tasks and the `lib.rs` call sites.
