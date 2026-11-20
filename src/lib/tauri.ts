import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AddProxyInput,
  AppConfig,
  AppSettings,
  DiagnosticsReport,
  ProxyLogEntry,
  ProxyServerStatus,
  UpdateProxyInput,
} from "../types";

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  addProxy: (input: AddProxyInput) => invoke<AppConfig>("add_proxy", { input }),
  updateProxy: (input: UpdateProxyInput) =>
    invoke<AppConfig>("update_proxy", { input }),
  deleteProxy: (id: string) => invoke<AppConfig>("delete_proxy", { id }),
  toggleProxy: (id: string, enabled: boolean) =>
    invoke<AppConfig>("toggle_proxy", { id, enabled }),

  addGroup: (name: string, color: string) =>
    invoke<AppConfig>("add_group", { name, color }),
  updateGroup: (id: string, name: string, color: string) =>
    invoke<AppConfig>("update_group", { id, name, color }),
  deleteGroup: (id: string) => invoke<AppConfig>("delete_group", { id }),

  updateSettings: (settings: AppSettings) =>
    invoke<AppConfig>("update_settings", { settings }),
  exportConfig: () => invoke<string>("export_config"),
  importConfig: (json: string) => invoke<AppConfig>("import_config", { json }),

  startProxyServer: () => invoke<ProxyServerStatus>("start_proxy_server"),
  stopProxyServer: () => invoke<ProxyServerStatus>("stop_proxy_server"),
  getProxyStatus: () => invoke<ProxyServerStatus>("get_proxy_status"),

  getLogs: () => invoke<ProxyLogEntry[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),

  ensureRootCa: () => invoke<string>("ensure_root_ca"),
  trustRootCa: () => invoke<void>("trust_root_ca"),
  untrustRootCa: () => invoke<void>("untrust_root_ca"),
  syncHosts: () => invoke<void>("sync_hosts"),
  getHostsContent: () => invoke<string>("get_hosts_content"),
  getDiagnostics: () => invoke<DiagnosticsReport>("get_diagnostics"),
  openDataDir: () => invoke<void>("open_data_dir"),

  openExternalUrl: (url: string) => openUrl(url),
};
