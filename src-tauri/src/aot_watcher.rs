//! Watches the foreground window and pins/unpins the overlay accordingly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::types::AppConfig;
use crate::types::AotMode;

/// Whether the overlay should be always-on-top given the current mode, the
/// allowlist, and the foreground app's exe name. `fg_is_self` is true when the
/// foreground window belongs to our own process (so dragging the overlay or
/// using Settings keeps it pinned).
pub fn should_pin(mode: &AotMode, allowlist: &[String], fg_name: &str, fg_is_self: bool) -> bool {
    match mode {
        AotMode::Pinned => true,
        AotMode::Auto => {
            fg_is_self || allowlist.iter().any(|a| a.eq_ignore_ascii_case(fg_name))
        }
    }
}

/// The exe basename (e.g. `claude.exe`) for a process id, or `None` if the
/// process can't be opened/queried.
#[cfg(windows)]
fn exe_basename(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        if ok.is_err() {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        Some(
            full.rsplit(|c| c == '\\' || c == '/')
                .next()
                .unwrap_or(&full)
                .to_string(),
        )
    }
}

/// The foreground window's process id and exe basename. `None` if it can't be
/// determined.
#[cfg(windows)]
fn foreground_exe() -> Option<(u32, String)> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let name = exe_basename(pid)?;
        Some((pid, name))
    }
}

#[cfg(not(windows))]
fn foreground_exe() -> Option<(u32, String)> {
    None
}

/// Whether any monitored app currently has a *visible* top-level window. Used
/// to hide the overlay in Auto mode once every monitored app is gone from the
/// screen — there is nothing left to pin to.
///
/// We check for a visible window rather than a live process because apps like
/// Claude minimize to the tray on close: the `claude.exe` processes keep
/// running with no visible window, so a process-existence check would keep the
/// overlay pinned forever. This mirrors how .NET computes `MainWindowHandle`
/// (`EnumWindows` + `IsWindowVisible` + unowned top-level), which we confirmed
/// reads 0 while Claude sits in the tray.
#[cfg(windows)]
fn any_monitored_visible(allowlist: &[String]) -> bool {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible, GW_OWNER,
    };

    if allowlist.is_empty() {
        return false;
    }

    struct Ctx<'a> {
        allowlist: &'a [String],
        found: bool,
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut Ctx);
        // Skip hidden, minimized, and owned (dialog/popup) windows — only a
        // real, on-screen main window counts.
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return true.into();
        }
        if !GetWindow(hwnd, GW_OWNER).unwrap_or_default().0.is_null() {
            return true.into();
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != 0 {
            if let Some(name) = exe_basename(pid) {
                if ctx.allowlist.iter().any(|a| a.eq_ignore_ascii_case(&name)) {
                    ctx.found = true;
                    return false.into(); // stop enumeration
                }
            }
        }
        true.into() // keep enumerating
    }

    let mut ctx = Ctx { allowlist, found: false };
    let r = unsafe { EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut _ as isize)) };
    // `found` is authoritative (the callback returns FALSE only after a match,
    // which makes EnumWindows report an error). A genuine enumeration failure
    // with no match is rare — fail safe by keeping the overlay shown.
    ctx.found || r.is_err()
}

#[cfg(not(windows))]
fn any_monitored_visible(_allowlist: &[String]) -> bool {
    true
}

/// Windows shell surfaces (taskbar, Start, search). These flash in as the
/// foreground while the user switches apps, so they are treated as neutral:
/// the overlay holds its current pinned state instead of flapping.
fn is_shell(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "explorer.exe"
            | "startmenuexperiencehost.exe"
            | "searchhost.exe"
            | "searchapp.exe"
            | "shellexperiencehost.exe"
    )
}

/// Poll the foreground window and pin/unpin the overlay.
pub fn start_aot_watcher(
    app: AppHandle,
    config: Arc<Mutex<AppConfig>>,
    started_online: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        let self_pid = std::process::id();
        // Set the overlay topmost once, and non-activating so it can never
        // steal focus. After this only its *visibility* changes — never its
        // z-order — which sidesteps the "won't restack" problem entirely.
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.set_always_on_top(true);
            #[cfg(windows)]
            set_no_activate(&win);
        }
        // Throttle the (relatively expensive) process-list scan to ~once per
        // second; reuse the cached result on the in-between 200ms ticks.
        let mut monitored_cached = true;
        let mut last_proc_check = std::time::Instant::now()
            .checked_sub(Duration::from_secs(2))
            .unwrap_or_else(std::time::Instant::now);
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };

            // Until live data first arrives, keep the overlay hidden in the tray
            // (one-shot). Once online, this gate is permanently open and the
            // normal pin logic below runs unchanged. Note: while this gate is
            // closed it overrides a tray-initiated `show()` (tray.rs) — the
            // window is re-hidden on the next tick — so the overlay is
            // intentionally unreachable until first online.
            if !started_online.load(Ordering::SeqCst) {
                if win.is_visible().unwrap_or(false) {
                    let _ = win.hide();
                }
                continue;
            }

            let (mode, allowlist) = {
                let c = config.lock().unwrap();
                (c.aot_mode.clone(), c.aot_allowlist.clone())
            };

            // Refresh the "is any monitored app visible" cache at most once a
            // second. Only Auto mode consumes it, so skip the scan in Pinned.
            if matches!(mode, AotMode::Auto)
                && last_proc_check.elapsed() >= Duration::from_secs(1)
            {
                monitored_cached = any_monitored_visible(&allowlist);
                last_proc_check = std::time::Instant::now();
            }

            let pin = match (&mode, foreground_exe()) {
                (AotMode::Pinned, _) => true,
                // Nothing left to monitor — hide regardless of foreground,
                // including over shell/self surfaces that would otherwise hold
                // the last pinned state.
                (AotMode::Auto, _) if !monitored_cached => false,
                (AotMode::Auto, Some((pid, name))) => {
                    // Hold the current state while our own window or a shell
                    // surface is foreground; only real apps drive the decision.
                    if pid == self_pid || is_shell(&name) {
                        continue;
                    }
                    should_pin(&mode, &allowlist, &name, false)
                }
                (AotMode::Auto, None) => continue,
            };

            // Show when pinned, hide otherwise. A hidden window has no z-order
            // to fight, and showing an always-topmost window puts it back on
            // top every time — no restack, no "stuck behind", no taskbar block.
            let visible = win.is_visible().unwrap_or(true);
            if pin && !visible {
                let _ = win.show();
            } else if !pin && visible {
                let _ = win.hide();
            }
        }
    });
}

/// Make the window non-activating (`WS_EX_NOACTIVATE`) so clicks/raises never
/// transfer keyboard focus to it.
#[cfg(windows)]
fn set_no_activate(win: &tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    if let Ok(hwnd) = win.hwnd() {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Vec<String> {
        vec!["claude.exe".to_string(), "powershell.exe".to_string()]
    }

    #[test]
    fn pinned_is_always_true() {
        assert!(should_pin(&AotMode::Pinned, &[], "chrome.exe", false));
    }

    #[test]
    fn auto_allowlisted_is_true_case_insensitive() {
        assert!(should_pin(&AotMode::Auto, &list(), "Claude.exe", false));
        assert!(should_pin(&AotMode::Auto, &list(), "POWERSHELL.EXE", false));
    }

    #[test]
    fn auto_not_listed_is_false() {
        assert!(!should_pin(&AotMode::Auto, &list(), "chrome.exe", false));
    }

    #[test]
    fn auto_self_is_true() {
        assert!(should_pin(&AotMode::Auto, &list(), "chrome.exe", true));
    }
}
