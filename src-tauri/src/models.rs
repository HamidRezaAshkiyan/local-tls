use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyEntry {
    pub id: String,
    pub hostname: String,
    pub target_host: String,
    pub target_port: u16,
    pub enabled: bool,
    pub group_id: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub tunnel_enabled: bool,
    #[serde(default)]
    pub tunnel_route_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub https_port: u16,
    pub http_port: u16,
    pub enable_http_redirect: bool,
    pub minimize_to_tray: bool,
    pub theme: String,
    #[serde(default)]
    pub tunnel_api_url: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            https_port: 443,
            http_port: 80,
            enable_http_redirect: true,
            minimize_to_tray: true,
            theme: "dark".to_string(),
            tunnel_api_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub groups: Vec<ProxyGroup>,
    pub proxies: Vec<ProxyEntry>,
    pub settings: AppSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            groups: vec![ProxyGroup {
                id: "default".to_string(),
                name: "Default".to_string(),
                color: "#10b981".to_string(),
                created_at: Utc::now(),
            }],
            proxies: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub host: String,
    pub path: String,
    pub upstream: String,
    pub status: u16,
    pub duration_ms: u64,
    pub is_websocket: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyServerStatus {
    pub running: bool,
    pub https_port: u16,
    pub http_port: Option<u16>,
    pub active_routes: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditionInfo {
    pub edition: String,
    pub tunnel_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatusDto {
    pub supported: bool,
    pub running: bool,
    pub state: String,
    pub hostname: Option<String>,
    /// Maps proxy ID -> live public HTTPS URL (e.g. `https://api.<id>.opentunnel.xyz`)
    pub public_urls: HashMap<String, String>,
    pub connections: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    pub os: String,
    pub data_dir: String,
    pub ca_generated: bool,
    pub ca_trusted: bool,
    pub ca_cert_path: String,
    pub hosts_path: String,
    pub hosts_synced: bool,
    pub unsynced_hosts: Vec<String>,
    pub https_port: u16,
    pub http_port: u16,
    pub https_port_available: bool,
    pub http_port_available: bool,
    pub proxy_running: bool,
    pub active_route_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProxyInput {
    pub hostname: String,
    pub target_host: Option<String>,
    pub target_port: u16,
    pub group_id: String,
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tunnel_enabled: Option<bool>,
    #[serde(default)]
    pub tunnel_route_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProxyInput {
    pub id: String,
    pub hostname: String,
    pub target_host: String,
    pub target_port: u16,
    pub group_id: String,
    pub enabled: bool,
    #[serde(default)]
    pub tunnel_enabled: Option<bool>,
    #[serde(default)]
    pub tunnel_route_name: Option<String>,
}
