//! Reads the signed-in Claude account's display name and plan from
//! `~/.claude.json` (the `oauthAccount` object is the authoritative source —
//! `.credentials.json`'s `subscriptionType` can be stale/personal while the
//! effective plan comes from the org tier).

use crate::types::Plan;
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

/// The effective plan from `oauthAccount`: the org rate-limit tier decides
/// Max 20x vs 5x; otherwise fall back to the org type (max → Max50, pro → Pro).
pub fn parse_plan(json: &str) -> Plan {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Plan::Unknown;
    };
    let Some(acct) = v.get("oauthAccount") else {
        return Plan::Unknown;
    };
    let tier = acct
        .get("organizationRateLimitTier")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_lowercase();
    let org = acct
        .get("organizationType")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_lowercase();
    if tier.contains("20x") {
        Plan::Max200
    } else if tier.contains("5x") {
        Plan::Max50
    } else if org.contains("max") {
        Plan::Max50
    } else if org.contains("pro") {
        Plan::Pro
    } else {
        Plan::Unknown
    }
}

pub fn load_plan(path: &PathBuf) -> Plan {
    std::fs::read_to_string(path)
        .ok()
        .map(|c| parse_plan(&c))
        .unwrap_or(Plan::Unknown)
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

    #[test]
    fn plan_from_org_rate_limit_tier() {
        let max20 = r#"{"oauthAccount":{"organizationType":"claude_max","organizationRateLimitTier":"default_claude_max_20x"}}"#;
        assert_eq!(parse_plan(max20), Plan::Max200);
        let max5 = r#"{"oauthAccount":{"organizationType":"claude_max","organizationRateLimitTier":"default_claude_max_5x"}}"#;
        assert_eq!(parse_plan(max5), Plan::Max50);
    }

    #[test]
    fn plan_falls_back_to_org_type() {
        let max = r#"{"oauthAccount":{"organizationType":"claude_max","organizationRateLimitTier":""}}"#;
        assert_eq!(parse_plan(max), Plan::Max50);
        let pro = r#"{"oauthAccount":{"organizationType":"claude_pro"}}"#;
        assert_eq!(parse_plan(pro), Plan::Pro);
    }

    #[test]
    fn plan_unknown_when_missing_or_malformed() {
        assert_eq!(parse_plan(r#"{"other":1}"#), Plan::Unknown);
        assert_eq!(parse_plan("not json"), Plan::Unknown);
    }
}
