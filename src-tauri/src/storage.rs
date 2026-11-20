use crate::models::AppConfig;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "config.json";

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CONFIG_FILE_NAME)
}

pub fn load_config(data_dir: &Path) -> Result<AppConfig, String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data directory: {e}"))?;
    let path = config_path(data_dir);
    if !path.exists() {
        let default_cfg = AppConfig::default();
        save_config(data_dir, &default_cfg)?;
        return Ok(default_cfg);
    }

    let raw = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {e}"))?;
    let mut cfg: AppConfig =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid config JSON: {e}"))?;

    if cfg.groups.is_empty() {
        cfg.groups = AppConfig::default().groups;
    }

    Ok(cfg)
}

pub fn save_config(data_dir: &Path, config: &AppConfig) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("Failed to create data directory: {e}"))?;
    let path = config_path(data_dir);
    let tmp_path = data_dir.join("config.json.tmp");
    let serialized = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(&tmp_path, serialized)
        .map_err(|e| format!("Failed to write temporary config: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to commit config: {e}"))?;
    Ok(())
}

/// Validates a local domain name according to RFC-1123 rules (no whitespace, no injection chars).
pub fn normalize_and_validate_hostname(input: &str) -> Result<String, String> {
    let trimmed = input
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_ascii_lowercase();

    if trimmed.is_empty() || trimmed.len() > 253 {
        return Err("Hostname must be between 1 and 253 characters.".to_string());
    }

    if trimmed.contains(char::is_whitespace) || trimmed.contains(':') || trimmed.contains('/') {
        return Err("Hostname must not contain spaces, ports, or paths.".to_string());
    }

    for label in trimmed.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!("Invalid domain label in '{trimmed}'."));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(format!(
                "Domain label '{label}' cannot start or end with a hyphen."
            ));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!(
                "Domain label '{label}' contains unsupported characters."
            ));
        }
    }

    Ok(trimmed)
}

pub fn validate_target_host(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok("127.0.0.1".to_string());
    }
    if trimmed.contains(char::is_whitespace) || trimmed.contains('/') {
        return Err("Target host must be a valid hostname or IP address.".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn validate_port(port: u16) -> Result<u16, String> {
    if port == 0 {
        return Err("Port must be between 1 and 65535.".to_string());
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hostname_validation() {
        assert_eq!(
            normalize_and_validate_hostname("  https://API.MyApp.Local/ ").unwrap(),
            "api.myapp.local"
        );
        assert_eq!(
            normalize_and_validate_hostname("app-1.test").unwrap(),
            "app-1.test"
        );
        assert!(normalize_and_validate_hostname("").is_err());
        assert!(normalize_and_validate_hostname("bad host.local").is_err());
        assert!(normalize_and_validate_hostname("-bad.local").is_err());
        assert!(normalize_and_validate_hostname("bad..local").is_err());
        assert!(normalize_and_validate_hostname("evil.local;rm -rf").is_err());
    }

    #[test]
    fn test_config_roundtrip() {
        let dir = TempDir::new().unwrap();
        let cfg = load_config(dir.path()).unwrap();
        assert_eq!(cfg.groups.len(), 1);
        assert_eq!(cfg.settings.https_port, 443);

        let mut updated = cfg.clone();
        updated.settings.https_port = 8443;
        save_config(dir.path(), &updated).unwrap();

        let reloaded = load_config(dir.path()).unwrap();
        assert_eq!(reloaded.settings.https_port, 8443);
    }
}
