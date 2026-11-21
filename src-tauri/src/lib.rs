pub mod certs;
pub mod commands;
pub mod hosts;
pub mod models;
pub mod platform;
pub mod proxy;
pub mod state;
pub mod storage;
pub mod tray;
pub mod tunnel;

use state::AppState;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("local_tls_lib=info")
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Could not resolve app data directory: {e}"))?;

            let app_state = Arc::new(AppState::initialize(data_dir)?);
            app.manage(app_state);

            if let Err(e) = tray::setup_tray(app.handle()) {
                tracing::warn!("Failed to initialize system tray: {e}");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let minimize_to_tray = window
                    .app_handle()
                    .try_state::<Arc<AppState>>()
                    .map(|s| s.config.read().settings.minimize_to_tray)
                    .unwrap_or(false);

                if minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                    state.proxy.stop();
                    let tunnel = Arc::clone(&state.tunnel);
                    tauri::async_runtime::spawn(async move {
                        let _ = tunnel.stop().await;
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_edition_info,
            commands::get_config,
            commands::add_proxy,
            commands::update_proxy,
            commands::delete_proxy,
            commands::toggle_proxy,
            commands::set_proxy_tunnel,
            commands::get_tunnel_status,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::add_group,
            commands::update_group,
            commands::delete_group,
            commands::update_settings,
            commands::export_config,
            commands::import_config,
            commands::start_proxy_server,
            commands::stop_proxy_server,
            commands::get_proxy_status,
            commands::get_logs,
            commands::clear_logs,
            commands::ensure_root_ca,
            commands::trust_root_ca,
            commands::untrust_root_ca,
            commands::sync_hosts,
            commands::get_hosts_content,
            commands::get_diagnostics,
            commands::open_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LocalTLS application");
}
