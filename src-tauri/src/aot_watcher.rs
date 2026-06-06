//! Watches the foreground window and pins/unpins the overlay accordingly.

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
