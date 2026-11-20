export interface ProxyGroup {
  id: string;
  name: string;
  color: string;
  createdAt: string;
}

export interface ProxyEntry {
  id: string;
  hostname: string;
  targetHost: string;
  targetPort: number;
  enabled: boolean;
  groupId: string;
  createdAt: string;
}

export interface AppSettings {
  httpsPort: number;
  httpPort: number;
  enableHttpRedirect: boolean;
  minimizeToTray: boolean;
  theme: "dark" | "light" | "system" | string;
}

export interface AppConfig {
  version: number;
  groups: ProxyGroup[];
  proxies: ProxyEntry[];
  settings: AppSettings;
}

export interface ProxyLogEntry {
  id: string;
  timestamp: string;
  method: string;
  host: string;
  path: string;
  upstream: string;
  status: number;
  durationMs: number;
  isWebsocket: boolean;
  error: string | null;
}

export interface ProxyServerStatus {
  running: boolean;
  httpsPort: number;
  httpPort: number | null;
  activeRoutes: number;
  lastError: string | null;
}

export interface DiagnosticsReport {
  os: string;
  dataDir: string;
  caGenerated: boolean;
  caTrusted: boolean;
  caCertPath: string;
  hostsPath: string;
  hostsSynced: boolean;
  unsyncedHosts: string[];
  httpsPort: number;
  httpPort: number;
  httpsPortAvailable: boolean;
  httpPortAvailable: boolean;
  proxyRunning: boolean;
  activeRouteCount: number;
}

export interface AddProxyInput {
  hostname: string;
  targetHost?: string;
  targetPort: number;
  groupId: string;
  enabled?: boolean;
}

export interface UpdateProxyInput {
  id: string;
  hostname: string;
  targetHost: string;
  targetPort: number;
  groupId: string;
  enabled: boolean;
}

export type ActiveTab = "proxies" | "logs" | "diagnostics" | "settings";
