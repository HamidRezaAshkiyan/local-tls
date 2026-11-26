# AGENTS.md — Ophiuchi-rs

## Overview

**Ophiuchi-rs** is a cross-platform (macOS & Windows) localhost SSL reverse proxy manager built with **Rust + Tauri v2 + React + TypeScript**. It gives local dev servers trusted `https://` custom domains without Docker or Nginx via a native Rust HTTPS/HTTP2/WebSocket proxy, a locally trusted ECDSA P-256 Root CA, safe managed `/etc/hosts` syncing, system tray integration, live request logs, and diagnostics. It compiles into two editions from the same codebase: **Standard Edition** (default, local-only) and **Tunnel Edition** (`--features tunnel`, adding blind-TLS public tunnels via `opentunnel`).

## Tech Stack & Requirements

- **Backend (Rust, edition 2021)**: Tauri `2` (`tray-icon`, `image-png`, `tauri-plugin-opener`, `tauri-plugin-dialog`), `tokio` (`full`), `hyper` `1` + `hyper-util`, `rustls` `0.23` (`ring`) + `tokio-rustls` `0.26`, `rcgen` `0.13`, `x509-parser`, `parking_lot`, optional `opentunnel` `0.6.2`
- **Frontend**: React `19.1`, TypeScript `5.8`, Vite `6.3` (dev server on `http://localhost:1420`), Tailwind CSS `4.1` (`@tailwindcss/vite`), Zustand `5`, `lucide-react`
- **Testing**: Rust unit/integration tests (`tempfile`, `tokio-tungstenite`, `futures-util`); TypeScript type-checking (`tsc --noEmit`)
- **Targets**: macOS `>=11.0`, Windows (NSIS `currentUser`)

## Repository Structure

```text
ophiuchi-rs/
├── package.json                       # Frontend deps & standard/tunnel Tauri scripts
├── vite.config.ts / tsconfig.json     # Vite & TypeScript build configuration
├── docs/                              # Static project landing page
├── src/                               # React + TypeScript frontend
│   ├── main.tsx                       # React DOM mount
│   ├── App.tsx                        # Tab shell & Tauri event listeners (proxy-log, proxy/tunnel status)
│   ├── lib/tauri.ts                   # Typed invoke() wrappers for all Tauri backend commands
│   ├── store/useAppStore.ts           # Zustand global state (config, logs, proxy/tunnel status, toasts)
│   ├── types/index.ts                 # Shared TS interfaces mirroring Rust models
│   └── components/                    # Sidebar, ProxiesView, ProxyModal, GroupModal, LogsView,
│                                      # DiagnosticsView, SettingsView, ToastContainer
└── src-tauri/
    ├── Cargo.toml                     # Crate ophiuchi-rs / ophiuchi_rs_lib; feature `tunnel = ["dep:opentunnel"]`
    ├── tauri.conf.json                # Standard Edition config (`com.ophiuchirs.desktop`)
    ├── tauri.tunnel.conf.json         # Tunnel Edition overlay (`com.ophiuchirs.tunnel`, `Ophiuchi-rs Tunnel`)
    └── src/
        ├── main.rs                    # Binary entrypoint calling `ophiuchi_rs_lib::run()`
        ├── lib.rs                     # Tauri Builder, AppState init, tray setup, close-to-tray, 28 invoke commands
        ├── state.rs                   # Shared `AppState` (config, proxy server, tunnel manager, data dir)
        ├── models.rs                  # Serde models for proxies, groups, settings, logs, diagnostics
        ├── commands.rs                # All `#[tauri::command]` handlers exposed to frontend
        ├── proxy.rs                   # Native hyper + tokio-rustls SNI HTTPS/HTTP2/WS proxy & HTTP→HTTPS redirect
        ├── certs.rs                   # ECDSA P-256 Local Root CA & per-domain leaf cert generation (`rcgen`)
        ├── hosts.rs                   # Managed hosts block (`# >>> Ophiuchi-rs Managed Hosts >>>`) + backups
        ├── storage.rs                 # JSON config persistence in Tauri `app_data_dir()`
        ├── tunnel.rs                  # OpenTunnel integration (feature-gated `#[cfg(feature = "tunnel")]` + no-op stub)
        ├── tray.rs                    # System tray / menubar menu
        └── platform/                  # OS-specific privilege elevation & CA trust store (`macos.rs`, `windows.rs`)
```

## Architecture & Key Flows

- **Entry Points**: `src-tauri/src/main.rs` → `ophiuchi_rs_lib::run()` initializes `AppState` inside `app.path().app_data_dir()`, registers system tray, and exposes 28 IPC commands. Frontend `src/main.tsx` → `src/App.tsx` bootstraps state via `src/lib/tauri.ts` and subscribes to backend Tauri events: `proxy-log`, `proxy-status-changed`, and `tunnel-status-changed`.
- **HTTPS Reverse Proxy (`proxy.rs`)**: Listens on configured HTTPS/HTTP ports using `tokio` + `hyper` + `tokio-rustls` with dynamic SNI certificate resolution from `certs.rs`, forwarding HTTP/1.1, HTTP/2, and WebSocket/HMR upgrades to local targets and emitting live `proxy-log` events.
- **Root CA & Hosts Sync (`certs.rs`, `hosts.rs`, `platform/`)**: Generates a local Root CA once and installs/removes it via OS-native elevation (`osascript` on macOS System Keychain; UAC PowerShell on Windows User Root Store). Hosts sync edits only the delimited Ophiuchi-rs managed block with timestamped backups.
- **Dual-Edition Feature Gating**: `tunnel` Cargo feature enables `opentunnel` in `src-tauri/src/tunnel.rs`; `commands::get_edition_info` informs the UI whether tunnel controls are enabled.
- **Window Lifecycle**: Closing the window hides to tray when `settings.minimize_to_tray` is enabled; otherwise it cleanly stops both the proxy server and any active tunnel.

## Development & Commands

```bash
# Install frontend dependencies
npm install

# Run Standard Edition or Tunnel Edition in dev mode
npm run tauri:dev:standard
npm run tauri:dev:tunnel

# Type-check and build frontend only
npm run build

# Run Rust tests for BOTH editions (always verify both)
cd src-tauri && cargo test
cd src-tauri && cargo test --features tunnel

# Build desktop installers
npm run tauri:build:standard
npm run tauri:build:tunnel
```

## Configuration & Environment

- **Runtime Data Directory**: Persisted under Tauri's `app_data_dir()` (config JSON, generated Root CA/leaf certs, hosts backups). Accessible via the `open_data_dir` command.
- **Logging**: Controlled by `tracing_subscriber` (defaults to `ophiuchi_rs_lib=info`).
- **No `.env` secrets required** for local development.

## Conventions & Gotchas

- **Test Both Editions**: Any change touching `tunnel.rs`, `commands.rs`, `models.rs`, or `state.rs` must compile and pass `cargo test` both *without* and *with* `--features tunnel` so the Standard Edition never leaks `opentunnel` dependencies.
- **IPC Contract Sync**: All Tauri commands in `src-tauri/src/commands.rs` must stay registered in `src-tauri/src/lib.rs` (`tauri::generate_handler![...]`) and match the TypeScript wrappers/types in `src/lib/tauri.ts` and `src/types/index.ts`.
- **Platform Elevation**: Changes to `platform/macos.rs` or `platform/windows.rs` (CA trust or hosts write) involve privileged OS scripts (`osascript` / PowerShell) — quote/escape paths carefully and never overwrite unmanaged hosts entries outside the marker block.
- **Ports 80/443**: Binding privileged ports on macOS/Windows may require elevated permissions or alternate ports in settings; diagnostics (`get_diagnostics`) reports port/CA/hosts health.
