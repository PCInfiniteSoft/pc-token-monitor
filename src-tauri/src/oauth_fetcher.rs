use crate::types::{DataSource, Plan, UsageData, WindowUsage};
use chrono::DateTime;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct OAuthWindowRaw {
    utilization: f64,
    resets_at: String,
}

// The /api/oauth/usage endpoint returns the usage windows at the top level
// (no wrapping "usage" object) and expresses utilization as a whole-number
// percent (e.g. 2.0 == 2%), so we divide by 100 to store a 0..1 fraction.
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
struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthCredentials>,
}

#[derive(Debug, Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier")]
    rate_limit_tier: Option<String>,
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

/// Map Claude's local credentials to a plan so the user doesn't have to pick
/// one manually. `subscriptionType` is "pro"/"max"; for max accounts the
/// 5x vs 20x tier is encoded in `rateLimitTier`.
pub fn detect_plan(subscription_type: Option<&str>, rate_limit_tier: Option<&str>) -> Plan {
    match subscription_type.map(|s| s.to_lowercase()) {
        Some(ref s) if s == "pro" => Plan::Pro,
        Some(ref s) if s == "max" => {
            let tier = rate_limit_tier.unwrap_or("").to_lowercase();
            if tier.contains("20x") {
                Plan::Max200
            } else {
                Plan::Max50
            }
        }
        _ => Plan::Unknown,
    }
}

pub fn load_plan(path: &PathBuf) -> Plan {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Plan::Unknown;
    };
    let Ok(creds) = serde_json::from_str::<CredentialsFile>(&content) else {
        return Plan::Unknown;
    };
    match creds.claude_ai_oauth {
        Some(o) => detect_plan(o.subscription_type.as_deref(), o.rate_limit_tier.as_deref()),
        None => Plan::Unknown,
    }
}

fn window_from_raw(raw: &OAuthWindowRaw) -> Result<WindowUsage, String> {
    Ok(WindowUsage {
        // API utilization is a whole-number percent; store as a 0..1 fraction.
        utilization: raw.utilization / 100.0,
        resets_at: DateTime::parse_from_rfc3339(&raw.resets_at)
            .map_err(|e| format!("bad resets_at: {e}"))?
            .with_timezone(&chrono::Utc),
    })
}

fn parse_oauth_response(json: &str) -> Result<UsageData, String> {
    let raw: OAuthUsageRaw =
        serde_json::from_str(json).map_err(|e| format!("parse error: {e}"))?;
    Ok(UsageData {
        five_hour: window_from_raw(&raw.five_hour)?,
        seven_day: window_from_raw(&raw.seven_day)?,
        seven_day_opus_utilization: raw.seven_day_opus.map(|o| o.utilization / 100.0),
        extra_usage_enabled: raw.extra_usage.map(|e| e.is_enabled).unwrap_or(false),
        source: DataSource::OAuth,
    })
}

pub async fn fetch_usage(access_token: &str) -> Result<UsageData, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("usage endpoint returned {}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| e.to_string())?;
    parse_oauth_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mirrors the real /api/oauth/usage shape: top-level windows, utilization
    // as a whole-number percent.
    const SAMPLE_RESPONSE: &str = r#"{
        "five_hour": { "utilization": 73, "resets_at": "2026-06-02T15:30:00Z" },
        "seven_day": { "utilization": 91, "resets_at": "2026-06-09T10:00:00Z" },
        "seven_day_opus": { "utilization": 45, "resets_at": "2026-06-09T10:00:00Z" },
        "extra_usage": { "is_enabled": true, "monthly_limit": 100.0, "used_credits": 45.0, "utilization": 45 }
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
            "five_hour": { "utilization": 10, "resets_at": "2026-06-02T15:30:00Z" },
            "seven_day": { "utilization": 20, "resets_at": "2026-06-09T10:00:00Z" }
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

    #[test]
    fn detect_plan_maps_pro() {
        assert_eq!(detect_plan(Some("pro"), Some("default_claude_ai")), Plan::Pro);
    }

    #[test]
    fn detect_plan_maps_max_tiers() {
        assert_eq!(detect_plan(Some("max"), Some("max_20x")), Plan::Max200);
        assert_eq!(detect_plan(Some("max"), Some("max_5x")), Plan::Max50);
        assert_eq!(detect_plan(Some("max"), None), Plan::Max50);
    }

    #[test]
    fn detect_plan_unknown_for_missing_or_free() {
        assert_eq!(detect_plan(None, None), Plan::Unknown);
        assert_eq!(detect_plan(Some("free"), None), Plan::Unknown);
    }

    #[test]
    fn utilization_normalized_to_fraction() {
        let data = parse_oauth_response(SAMPLE_RESPONSE).unwrap();
        assert!((data.five_hour.utilization - 0.73).abs() < f64::EPSILON);
    }
}
