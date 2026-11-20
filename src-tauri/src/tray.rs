use crate::commands::{emit_proxy_status, start_proxy_internal, stop_proxy_internal};
use crate::state::AppState;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "localtls-tray";
const MENU_SHOW: &str = "show";
const MENU_TOGGLE_PROXY: &str = "toggle_proxy";
const MENU_QUIT: &str = "quit";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, MENU_SHOW, "Open LocalTLS", true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(
        app,
        MENU_TOGGLE_PROXY,
        "Start / Stop Proxy Server",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quit LocalTLS", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &toggle_item, &separator, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("LocalTLS — Localhost SSL Proxy")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => {
                show_main_window(app);
            }
            MENU_TOGGLE_PROXY => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<Arc<AppState>>();
                    if state.proxy.is_running() {
                        stop_proxy_internal(&state);
                    } else {
                        let _ = start_proxy_internal(&app_handle, &state).await;
                    }
                    emit_proxy_status(&app_handle, &state);
                });
            }
            MENU_QUIT => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.proxy.stop();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
