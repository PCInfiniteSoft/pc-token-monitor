//! Watches the foreground window and pins/unpins the overlay accordingly.

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

/// The foreground window's process id and exe basename. `None` if it can't be
/// determined.
#[cfg(windows)]
fn foreground_exe() -> Option<(u32, String)> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
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
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        if ok.is_err() {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        let name = full
            .rsplit(|c| c == '\\' || c == '/')
            .next()
            .unwrap_or(&full)
            .to_string();
        Some((pid, name))
    }
}

#[cfg(not(windows))]
fn foreground_exe() -> Option<(u32, String)> {
    None
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
pub fn start_aot_watcher(app: AppHandle, config: Arc<Mutex<AppConfig>>) {
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
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;

            let Some(win) = app.get_webview_window("main") else {
                continue;
            };
            let (mode, allowlist) = {
                let c = config.lock().unwrap();
                (c.aot_mode.clone(), c.aot_allowlist.clone())
            };

            let pin = match (&mode, foreground_exe()) {
                (AotMode::Pinned, _) => true,
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
