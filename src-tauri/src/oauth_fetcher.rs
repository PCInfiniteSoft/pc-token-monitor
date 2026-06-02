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
        seven_day_opus_utilization: raw.usage.seven_day_opus.map(|o| o.utilization),
        extra_usage_enabled: raw.usage.extra_usage.map(|e| e.is_enabled).unwrap_or(false),
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
