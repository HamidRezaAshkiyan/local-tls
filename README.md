# LocalTLS — Cross-Platform Localhost SSL Proxy Manager

A fast, zero-dependency desktop application for **macOS** and **Windows** that gives your local dev servers trusted `https://` custom domains in seconds — inspired by [`ophiuchi-desktop`](https://github.com/cheeselemon/ophiuchi-desktop), without requiring Docker or Nginx.

## Features

- **Built-in Native Rust HTTPS Proxy**: Powered by `tokio`, `hyper`, and `tokio-rustls` with SNI, HTTP/1.1, HTTP/2, full WebSocket/HMR tunneling, and optional HTTP → HTTPS 301 redirection.
- **Trust-Once Local Root CA**: Generates a local ECDSA P-256 Root CA (`rcgen`) and trusts it once in your OS trust store (macOS System Keychain or Windows User Root Certificate Store). Every new domain gets an auto-signed SAN leaf certificate trusted immediately by Chrome, Safari, Edge, and Brave.
- **Safe Cross-Platform Hosts Sync**: Uses managed block markers (`# >>> LocalTLS Managed Hosts >>>`) with automatic timestamped backups and native privilege elevation (`osascript` on macOS, UAC PowerShell on Windows). Never requests or stores plaintext passwords.
- **Proxy Groups & Live Hot Reload**: Organize routes by project/group; adding, editing, or toggling proxies updates the running server immediately without restarting.
- **System Tray / Menubar Mode**: Keep the proxy running in the background and toggle it from the tray.
- **Live Request Logs & Diagnostics**: Real-time request inspector (status, method, host/path, upstream, latency) and system health checks.

## Development

### Prerequisites
- Node.js 20+
- Rust & Cargo (via `rustup`)
- Platform prerequisites per [Tauri v2 docs](https://v2.tauri.app/start/prerequisites/) (Xcode CLT on macOS, WebView2 + MSVC Build Tools on Windows)

### Commands

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Run Rust unit & end-to-end TLS/WebSocket proxy tests
cd src-tauri && cargo test

# Build frontend production bundle
npm run build

# Build native installer (.dmg/.app on macOS, .msi/.exe on Windows)
npm run tauri build
```

## Cross-Platform Packaging

Push a version tag (e.g. `v0.1.0`) or trigger `.github/workflows/release.yml` in GitHub Actions to build signed-ready installers for:
- **macOS**: Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) `.dmg` / `.app`
- **Windows**: `x86_64-pc-windows-msvc` `.msi` & NSIS `.exe`
