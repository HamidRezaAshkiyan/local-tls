use crate::certs::CA_COMMON_NAME;
use crate::hosts;
use crate::models::{
    AddProxyInput, AppConfig, AppSettings, DiagnosticsReport, EditionInfo, ProxyEntry, ProxyGroup,
    ProxyLogEntry, ProxyServerStatus, TunnelStatusDto, UpdateProxyInput,
};
use crate::platform;
use crate::proxy::is_port_available;
use crate::state::AppState;
use crate::storage::{normalize_and_validate_hostname, validate_port, validate_target_host};
use crate::tunnel;
use chrono::Utc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

pub fn emit_proxy_status(app: &AppHandle, state: &Arc<AppState>) {
    let default_port = state.config.read().settings.https_port;
    let status = state.proxy.status(default_port);
    let _ = app.emit("proxy-status-changed", &status);
}

pub fn emit_tunnel_status(app: &AppHandle, status: &TunnelStatusDto) {
    let _ = app.emit("tunnel-status-changed", status);
}

async fn sync_tunnel_routes(app: &AppHandle, state: &Arc<AppState>, proxies: &[ProxyEntry]) {
    if let Ok(status) = state.tunnel.update_routes(proxies).await {
        emit_tunnel_status(app, &status);
    }
}

pub async fn start_proxy_internal(
    app: &AppHandle,
    state: &Arc<AppState>,
) -> Result<ProxyServerStatus, String> {
    let (https_port, http_port, proxies) = {
        let cfg = state.config.read();
        let hp = if cfg.settings.enable_http_redirect {
            Some(cfg.settings.http_port)
        } else {
            None
        };
        (cfg.settings.https_port, hp, cfg.proxies.clone())
    };

    state.proxy.update_routes(&proxies);

    let app_clone = app.clone();
    let state_clone = Arc::clone(state);
    let log_cb = Arc::new(move |entry: ProxyLogEntry| {
        state_clone.push_log(entry.clone());
        let _ = app_clone.emit("proxy-log", &entry);
    });

    state
        .proxy
        .start(
            Arc::clone(&state.cert_manager),
            https_port,
            http_port,
            log_cb,
        )
        .await?;

    let status = state.proxy.status(https_port);
    let _ = app.emit("proxy-status-changed", &status);
    Ok(status)
}

pub fn stop_proxy_internal(state: &Arc<AppState>) -> ProxyServerStatus {
    state.proxy.stop();
    let default_port = state.config.read().settings.https_port;
    state.proxy.status(default_port)
}

#[tauri::command]
pub fn get_edition_info() -> EditionInfo {
    tunnel::edition_info()
}

#[tauri::command]
pub fn get_config(state: State<'_, Arc<AppState>>) -> AppConfig {
    state.config.read().clone()
}

#[tauri::command]
pub async fn add_proxy(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: AddProxyInput,
) -> Result<AppConfig, String> {
    let hostname = normalize_and_validate_hostname(&input.hostname)?;
    let target_host = validate_target_host(input.target_host.as_deref().unwrap_or("127.0.0.1"))?;
    let target_port = validate_port(input.target_port)?;

    {
        let mut cfg = state.config.write();
        if cfg
            .proxies
            .iter()
            .any(|p| p.hostname.eq_ignore_ascii_case(&hostname))
        {
            return Err(format!("A proxy for '{hostname}' already exists."));
        }

        let group_id = if cfg.groups.iter().any(|g| g.id == input.group_id) {
            input.group_id
        } else {
            cfg.groups
                .first()
                .map(|g| g.id.clone())
                .unwrap_or_else(|| "default".to_string())
        };

        cfg.proxies.push(ProxyEntry {
            id: Uuid::new_v4().to_string(),
            hostname: hostname.clone(),
            target_host,
            target_port,
            enabled: input.enabled.unwrap_or(true),
            group_id,
            created_at: Utc::now(),
            tunnel_enabled: input.tunnel_enabled.unwrap_or(false),
            tunnel_route_name: input.tunnel_route_name,
        });
    }

    // Pre-generate leaf certificate for this hostname
    let _ = state.cert_manager.get_or_create_leaf(&hostname);

    state.save_current_config()?;
    let cfg = state.config.read().clone();
    state.proxy.update_routes(&cfg.proxies);
    emit_proxy_status(&app, &state);
    sync_tunnel_routes(&app, &state, &cfg.proxies).await;
    Ok(cfg)
}

#[tauri::command]
pub async fn update_proxy(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: UpdateProxyInput,
) -> Result<AppConfig, String> {
    let hostname = normalize_and_validate_hostname(&input.hostname)?;
    let target_host = validate_target_host(&input.target_host)?;
    let target_port = validate_port(input.target_port)?;

    {
        let mut cfg = state.config.write();
        if cfg
            .proxies
            .iter()
            .any(|p| p.id != input.id && p.hostname.eq_ignore_ascii_case(&hostname))
        {
            return Err(format!("Another proxy already uses '{hostname}'."));
        }

        let Some(entry) = cfg.proxies.iter_mut().find(|p| p.id == input.id) else {
            return Err("Proxy entry not found.".to_string());
        };

        entry.hostname = hostname.clone();
        entry.target_host = target_host;
        entry.target_port = target_port;
        entry.group_id = input.group_id;
        entry.enabled = input.enabled;
        if let Some(te) = input.tunnel_enabled {
            entry.tunnel_enabled = te;
        }
        if input.tunnel_route_name.is_some() {
            entry.tunnel_route_name = input.tunnel_route_name;
        }
    }

    let _ = state.cert_manager.get_or_create_leaf(&hostname);
    state.save_current_config()?;
    let cfg = state.config.read().clone();
    state.proxy.update_routes(&cfg.proxies);
    emit_proxy_status(&app, &state);
    sync_tunnel_routes(&app, &state, &cfg.proxies).await;
    Ok(cfg)
}

#[tauri::command]
pub async fn delete_proxy(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<AppConfig, String> {
    let removed_host = {
        let mut cfg = state.config.write();
        let pos = cfg
            .proxies
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| "Proxy entry not found.".to_string())?;
        cfg.proxies.remove(pos).hostname
    };

    state.cert_manager.remove_host_cert(&removed_host);
    state.save_current_config()?;
    let cfg = state.config.read().clone();
    state.proxy.update_routes(&cfg.proxies);
    emit_proxy_status(&app, &state);
    sync_tunnel_routes(&app, &state, &cfg.proxies).await;
    Ok(cfg)
}

#[tauri::command]
pub async fn toggle_proxy(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Result<AppConfig, String> {
    {
        let mut cfg = state.config.write();
        let Some(entry) = cfg.proxies.iter_mut().find(|p| p.id == id) else {
            return Err("Proxy entry not found.".to_string());
        };
        entry.enabled = enabled;
    }

    state.save_current_config()?;
    let cfg = state.config.read().clone();
    state.proxy.update_routes(&cfg.proxies);
    emit_proxy_status(&app, &state);
    Ok(cfg)
}

#[tauri::command]
pub async fn set_proxy_tunnel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    tunnel_enabled: bool,
    tunnel_route_name: Option<String>,
) -> Result<AppConfig, String> {
    {
        let mut cfg = state.config.write();
        let Some(entry) = cfg.proxies.iter_mut().find(|p| p.id == id) else {
            return Err("Proxy entry not found.".to_string());
        };
        entry.tunnel_enabled = tunnel_enabled;
        if tunnel_route_name.is_some() {
            entry.tunnel_route_name = tunnel_route_name;
        }
    }

    state.save_current_config()?;
    let cfg = state.config.read().clone();
    sync_tunnel_routes(&app, &state, &cfg.proxies).await;
    Ok(cfg)
}

#[tauri::command]
pub fn get_tunnel_status(state: State<'_, Arc<AppState>>) -> TunnelStatusDto {
    state.tunnel.status()
}

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<TunnelStatusDto, String> {
    let (proxies, api_url) = {
        let cfg = state.config.read();
        (cfg.proxies.clone(), cfg.settings.tunnel_api_url.clone())
    };

    let app_clone = app.clone();
    let cb = Arc::new(move |status: TunnelStatusDto| {
        emit_tunnel_status(&app_clone, &status);
    });

    state
        .tunnel
        .start(&proxies, api_url.as_deref(), cb)
        .await
}

#[tauri::command]
pub async fn stop_tunnel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<TunnelStatusDto, String> {
    let status = state.tunnel.stop().await;
    emit_tunnel_status(&app, &status);
    Ok(status)
}

#[tauri::command]
pub fn add_group(
    state: State<'_, Arc<AppState>>,
    name: String,
    color: String,
) -> Result<AppConfig, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Group name cannot be empty.".to_string());
    }

    {
        let mut cfg = state.config.write();
        if cfg
            .groups
            .iter()
            .any(|g| g.name.eq_ignore_ascii_case(trimmed))
        {
            return Err(format!("Group '{trimmed}' already exists."));
        }
        cfg.groups.push(ProxyGroup {
            id: Uuid::new_v4().to_string(),
            name: trimmed.to_string(),
            color: if color.trim().is_empty() {
                "#10b981".to_string()
            } else {
                color
            },
            created_at: Utc::now(),
        });
    }

    state.save_current_config()?;
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn update_group(
    state: State<'_, Arc<AppState>>,
    id: String,
    name: String,
    color: String,
) -> Result<AppConfig, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Group name cannot be empty.".to_string());
    }

    {
        let mut cfg = state.config.write();
        let Some(group) = cfg.groups.iter_mut().find(|g| g.id == id) else {
            return Err("Group not found.".to_string());
        };
        group.name = trimmed.to_string();
        if !color.trim().is_empty() {
            group.color = color;
        }
    }

    state.save_current_config()?;
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn delete_group(state: State<'_, Arc<AppState>>, id: String) -> Result<AppConfig, String> {
    if id == "default" {
        return Err("The Default group cannot be deleted.".to_string());
    }

    {
        let mut cfg = state.config.write();
        if cfg.groups.len() <= 1 {
            return Err("At least one group must remain.".to_string());
        }
        cfg.groups.retain(|g| g.id != id);
        let fallback_id = cfg
            .groups
            .first()
            .map(|g| g.id.clone())
            .unwrap_or_else(|| "default".to_string());
        for proxy in &mut cfg.proxies {
            if proxy.group_id == id {
                proxy.group_id = fallback_id.clone();
            }
        }
    }

    state.save_current_config()?;
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> Result<AppConfig, String> {
    validate_port(settings.https_port)?;
    validate_port(settings.http_port)?;
    if settings.enable_http_redirect && settings.http_port == settings.https_port {
        return Err("HTTP and HTTPS ports cannot be the same.".to_string());
    }

    {
        let mut cfg = state.config.write();
        cfg.settings = settings;
    }

    state.save_current_config()?;
    Ok(state.config.read().clone())
}

#[tauri::command]
pub fn export_config(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let cfg = state.config.read().clone();
    serde_json::to_string_pretty(&cfg).map_err(|e| format!("Failed to serialize config: {e}"))
}

#[tauri::command]
pub async fn import_config(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    json: String,
) -> Result<AppConfig, String> {
    let parsed: AppConfig =
        serde_json::from_str(&json).map_err(|e| format!("Invalid config JSON: {e}"))?;
    if parsed.groups.is_empty() {
        return Err("Imported config must contain at least one group.".to_string());
    }
    for p in &parsed.proxies {
        normalize_and_validate_hostname(&p.hostname)?;
        validate_port(p.target_port)?;
    }

    {
        let mut cfg = state.config.write();
        *cfg = parsed;
    }

    state.save_current_config()?;
    let cfg = state.config.read().clone();
    state.proxy.update_routes(&cfg.proxies);
    emit_proxy_status(&app, &state);
    sync_tunnel_routes(&app, &state, &cfg.proxies).await;
    Ok(cfg)
}

#[tauri::command]
pub async fn start_proxy_server(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<ProxyServerStatus, String> {
    start_proxy_internal(&app, &state).await
}

#[tauri::command]
pub fn stop_proxy_server(app: AppHandle, state: State<'_, Arc<AppState>>) -> ProxyServerStatus {
    let status = stop_proxy_internal(&state);
    emit_proxy_status(&app, &state);
    status
}

#[tauri::command]
pub fn get_proxy_status(state: State<'_, Arc<AppState>>) -> ProxyServerStatus {
    let default_port = state.config.read().settings.https_port;
    state.proxy.status(default_port)
}

#[tauri::command]
pub fn get_logs(state: State<'_, Arc<AppState>>) -> Vec<ProxyLogEntry> {
    state.logs.read().iter().cloned().collect()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, Arc<AppState>>) {
    state.logs.write().clear();
}

#[tauri::command]
pub fn ensure_root_ca(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let path = state.cert_manager.ensure_root_ca()?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn trust_root_ca(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let path = state.cert_manager.ensure_root_ca()?;
    platform::trust_ca_certificate(&path)
}

#[tauri::command]
pub fn untrust_root_ca(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let path = state.cert_manager.ca_cert_path();
    platform::untrust_ca_certificate(CA_COMMON_NAME, &path)
}

#[tauri::command]
pub fn sync_hosts(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let active_hosts = state.active_hostnames();
    hosts::sync_hosts_file(&state.data_dir, &active_hosts)
}

#[tauri::command]
pub fn get_hosts_content() -> Result<String, String> {
    hosts::read_hosts_file()
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, Arc<AppState>>) -> Result<DiagnosticsReport, String> {
    let cfg = state.config.read().clone();
    let ca_generated = state.cert_manager.ca_exists();
    let ca_cert_path = state.cert_manager.ca_cert_path();
    let ca_trusted = if ca_generated {
        platform::is_ca_certificate_trusted(CA_COMMON_NAME, &ca_cert_path).unwrap_or(false)
    } else {
        false
    };

    let active_hosts: Vec<String> = cfg
        .proxies
        .iter()
        .filter(|p| p.enabled)
        .map(|p| p.hostname.clone())
        .collect();

    let hosts_content = hosts::read_hosts_file().unwrap_or_default();
    let unsynced_hosts = hosts::find_unsynced_hosts(&hosts_content, &active_hosts);
    let proxy_running = state.proxy.is_running();

    let https_port_available = if proxy_running {
        true
    } else {
        is_port_available(cfg.settings.https_port)
    };
    let http_port_available = if proxy_running {
        true
    } else {
        is_port_available(cfg.settings.http_port)
    };

    Ok(DiagnosticsReport {
        os: platform::os_name().to_string(),
        data_dir: state.data_dir.to_string_lossy().to_string(),
        ca_generated,
        ca_trusted,
        ca_cert_path: ca_cert_path.to_string_lossy().to_string(),
        hosts_path: platform::hosts_file_path().to_string_lossy().to_string(),
        hosts_synced: unsynced_hosts.is_empty(),
        unsynced_hosts,
        https_port: cfg.settings.https_port,
        http_port: cfg.settings.http_port,
        https_port_available,
        http_port_available,
        proxy_running,
        active_route_count: active_hosts.len(),
    })
}

#[tauri::command]
pub fn open_data_dir(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    platform::open_in_file_manager(&state.data_dir)
}
