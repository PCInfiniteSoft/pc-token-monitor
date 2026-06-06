use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowUsage {
    pub utilization: f64,
    pub resets_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub five_hour: WindowUsage,
    pub seven_day: WindowUsage,
    pub seven_day_opus_utilization: Option<f64>,
    pub extra_usage_enabled: bool,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    // snake_case would turn `OAuth` into `o_auth`; the frontend expects `oauth`.
    #[serde(rename = "oauth")]
    OAuth,
    JsonlFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Plan {
    Pro,
    Max50,
    Max200,
    Unknown,
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::Pro => write!(f, "PRO"),
            Plan::Max50 => write!(f, "MAX 50"),
            Plan::Max200 => write!(f, "MAX 200"),
            Plan::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AotMode {
    Auto,
    Pinned,
}

fn default_aot_mode() -> AotMode {
    AotMode::Auto
}

fn default_aot_allowlist() -> Vec<String> {
    vec![
        "windowsterminal.exe".to_string(),
        "powershell.exe".to_string(),
        "pwsh.exe".to_string(),
        "claude.exe".to_string(),
        "cmd.exe".to_string(),
        "conhost.exe".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub plan: Plan,
    #[serde(default = "default_aot_mode")]
    pub aot_mode: AotMode,
    #[serde(default = "default_aot_allowlist")]
    pub aot_allowlist: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            plan: Plan::Unknown,
            aot_mode: default_aot_mode(),
            aot_allowlist: default_aot_allowlist(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendState {
    pub usage: Option<UsageData>,
    pub config: AppConfig,
    pub user_name: Option<String>,
}
