use crate::types::{AppConfig, AotMode, Plan};
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PCTokenMonitor")
        .join("app_config.json")
}

pub fn load_config(path: &PathBuf) -> AppConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn plan_from_extra_usage(extra_usage_enabled: bool, saved: &Plan) -> Plan {
    if extra_usage_enabled && *saved == Plan::Unknown {
        Plan::Max50
    } else {
        saved.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_config_path() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app_config.json");
        (dir, path)
    }

    #[test]
    fn load_config_returns_default_when_file_missing() {
        let path = PathBuf::from("/nonexistent/app_config.json");
        let config = load_config(&path);
        assert_eq!(config.plan, Plan::Unknown);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (_dir, path) = temp_config_path();
        let config = AppConfig { plan: Plan::Max50, ..Default::default() };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(loaded.plan, Plan::Max50);
    }

    #[test]
    fn plan_from_extra_usage_upgrades_unknown_to_max50() {
        let result = plan_from_extra_usage(true, &Plan::Unknown);
        assert_eq!(result, Plan::Max50);
    }

    #[test]
    fn plan_from_extra_usage_preserves_user_selection() {
        let result = plan_from_extra_usage(true, &Plan::Max200);
        assert_eq!(result, Plan::Max200);
    }

    #[test]
    fn default_config_is_auto_with_default_allowlist() {
        let c = AppConfig::default();
        assert_eq!(c.aot_mode, AotMode::Auto);
        assert!(c.aot_allowlist.iter().any(|a| a == "claude.exe"));
    }

    #[test]
    fn save_and_load_preserves_aot_settings() {
        let (_dir, path) = temp_config_path();
        let config = AppConfig {
            plan: Plan::Pro,
            aot_mode: AotMode::Pinned,
            aot_allowlist: vec!["foo.exe".to_string()],
        };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path);
        assert_eq!(loaded.aot_mode, AotMode::Pinned);
        assert_eq!(loaded.aot_allowlist, vec!["foo.exe".to_string()]);
    }
}
