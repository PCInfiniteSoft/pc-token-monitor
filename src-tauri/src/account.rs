//! Reads the signed-in Claude account's display name from `~/.claude.json`.

use std::path::PathBuf;

pub fn account_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude.json")
}

/// The account display name: `oauthAccount.displayName` if present and
/// non-empty, else `oauthAccount.emailAddress`, else `None`.
pub fn parse_user_name(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let acct = v.get("oauthAccount")?;
    let display = acct
        .get("displayName")
        .and_then(|d| d.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(d) = display {
        return Some(d.to_string());
    }
    acct.get("emailAddress")
        .and_then(|e| e.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn load_user_name(path: &PathBuf) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_user_name(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_display_name() {
        let json = r#"{"oauthAccount":{"displayName":"Cocoa","emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(json), Some("Cocoa".to_string()));
    }

    #[test]
    fn falls_back_to_email_when_display_missing_or_empty() {
        let missing = r#"{"oauthAccount":{"emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(missing), Some("a@b.com".to_string()));
        let empty = r#"{"oauthAccount":{"displayName":"  ","emailAddress":"a@b.com"}}"#;
        assert_eq!(parse_user_name(empty), Some("a@b.com".to_string()));
    }

    #[test]
    fn none_when_no_account_or_malformed() {
        assert_eq!(parse_user_name(r#"{"other":1}"#), None);
        assert_eq!(parse_user_name("not json"), None);
        assert_eq!(parse_user_name(r#"{"oauthAccount":{}}"#), None);
    }
}
