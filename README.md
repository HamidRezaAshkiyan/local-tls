# Ophiuchi-rs — Cross-Platform Localhost SSL Proxy Manager

🌐 **Website & Downloads**: https://hamidrezaashkiyan.github.io/ophiuchi-rs/

A fast, zero-dependency Rust + Tauri v2 desktop application for **macOS** and **Windows** that gives your local dev servers trusted `https://` custom domains in seconds — inspired by [`ophiuchi-desktop`](https://github.com/cheeselemon/ophiuchi-desktop), without requiring Docker or Nginx.

## Two Editions

**Ophiuchi-rs** is built in **two editions** from the same codebase via a Cargo feature flag (`tunnel`), so you can choose the exact footprint you want:

| Edition | Product Name | What's Included |
| --- | --- | --- |
| **Standard Edition** (default) | `Ophiuchi-rs` | Local-only HTTPS/HTTP2/WebSocket reverse proxy, Local Root CA, managed hosts sync, system tray, live logs, diagnostics. Zero external tunnel dependencies compiled in. |
| **Tunnel Edition** (`--features tunnel`) | `Ophiuchi-rs Tunnel` | Everything in Standard Edition **plus** one-click blind-TLS public tunnels powered by [`anomalyco/opentunnel`](https://github.com/anomalyco/opentunnel) (`https://<route>.<id>.opentunnel.xyz`), with TLS terminated locally on your machine and optional self-hosted server support. |

## Core Features

- **Built-in Native Rust HTTPS Proxy**: Powered by `tokio`, `hyper`, and `tokio-rustls` with SNI, HTTP/1.1, HTTP/2, full WebSocket/HMR tunneling, and optional HTTP → HTTPS 301 redirection.
- **Trust-Once Local Root CA**: Generates a local ECDSA P-256 Root CA (`rcgen`) and trusts it once in your OS trust store (macOS System Keychain or Windows User Root Certificate Store).
- **Safe Cross-Platform Hosts Sync**: Uses managed block markers (`# >>> Ophiuchi-rs Managed Hosts >>>`) with automatic timestamped backups and native privilege elevation (`osascript` on macOS, UAC PowerShell on Windows).
- **System Tray / Menubar Mode**, **Live Request Logs**, and **Diagnostics**.

## Development

```bash
# Install frontend dependencies
npm install

# Run Standard Edition in dev mode
npm run tauri:dev:standard

# Run Tunnel Edition (with OpenTunnel) in dev mode
npm run tauri:dev:tunnel

# Run Rust tests for BOTH editions
cd src-tauri
cargo test
cargo test --features tunnel

# Build installers
npm run tauri:build:standard
npm run tauri:build:tunnel
```
