use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct JMessage {
    usage: Option<JUsage>,
}

#[derive(Debug, Deserialize)]
struct JUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<JMessage>,
    timestamp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenEvent {
    pub tokens: u64,
    pub at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct FallbackUsage {
    pub five_hour_tokens: u64,
    pub seven_day_tokens: u64,
}

pub fn claude_projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("projects")
}

pub fn parse_line(line: &str) -> Option<TokenEvent> {
    let entry: JEntry = serde_json::from_str(line).ok()?;
    if entry.entry_type.as_deref() != Some("assistant") {
        return None;
    }
    let msg = entry.message?;
    let usage = msg.usage?;
    let input = usage.input_tokens.unwrap_or(0);
    let output = usage.output_tokens.unwrap_or(0);
    if input == 0 && output == 0 {
        return None;
    }
    let ts_str = entry.timestamp?;
    let at = DateTime::parse_from_rfc3339(&ts_str).ok()?.with_timezone(&Utc);
    Some(TokenEvent { tokens: input + output, at })
}

pub fn compute_fallback(events: &[TokenEvent], now: DateTime<Utc>) -> FallbackUsage {
    let five_hour_cutoff = now - chrono::Duration::hours(5);
    let seven_day_cutoff = now - chrono::Duration::days(7);
    FallbackUsage {
        five_hour_tokens: events
            .iter()
            .filter(|e| e.at > five_hour_cutoff)
            .map(|e| e.tokens)
            .sum(),
        seven_day_tokens: events
            .iter()
            .filter(|e| e.at > seven_day_cutoff)
            .map(|e| e.tokens)
            .sum(),
    }
}

pub fn scan_projects_dir(dir: &PathBuf) -> Vec<TokenEvent> {
    let mut events = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return events;
    };
    for project in entries.flatten() {
        collect_jsonl_events(&project.path(), &mut events);
    }
    events.sort_by_key(|e| e.at);
    events
}

fn collect_jsonl_events(dir: &PathBuf, events: &mut Vec<TokenEvent>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "jsonl") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    if let Some(ev) = parse_line(line) {
                        events.push(ev);
                    }
                }
            }
        } else if path.is_dir() {
            collect_jsonl_events(&path, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_line_extracts_assistant_tokens() {
        let line = r#"{"type":"assistant","message":{"usage":{"input_tokens":100,"output_tokens":50}},"timestamp":"2026-06-02T10:00:00Z"}"#;
        let ev = parse_line(line).unwrap();
        assert_eq!(ev.tokens, 150);
    }

    #[test]
    fn parse_line_ignores_non_assistant_entries() {
        let line = r#"{"type":"permission-mode","permissionMode":"default"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parse_line_ignores_zero_token_entries() {
        let line = r#"{"type":"assistant","message":{"usage":{"input_tokens":0,"output_tokens":0}},"timestamp":"2026-06-02T10:00:00Z"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn compute_fallback_sums_5hr_window() {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let events = vec![
            TokenEvent { tokens: 1000, at: now - chrono::Duration::hours(4) },
            TokenEvent { tokens: 2000, at: now - chrono::Duration::hours(6) },
            TokenEvent { tokens: 500,  at: now - chrono::Duration::hours(1) },
        ];
        let result = compute_fallback(&events, now);
        assert_eq!(result.five_hour_tokens, 1500);
        assert_eq!(result.seven_day_tokens, 3500);
    }

    #[test]
    fn compute_fallback_excludes_old_events() {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let events = vec![
            TokenEvent { tokens: 999, at: now - chrono::Duration::days(8) },
        ];
        let result = compute_fallback(&events, now);
        assert_eq!(result.seven_day_tokens, 0);
    }
}
