import { create } from "zustand";
import { api } from "../lib/tauri";
import type {
  ActiveTab,
  AddProxyInput,
  AppConfig,
  AppSettings,
  DiagnosticsReport,
  EditionInfo,
  ProxyLogEntry,
  ProxyServerStatus,
  TunnelStatus,
  UpdateProxyInput,
} from "../types";

export interface ToastMessage {
  id: string;
  kind: "success" | "error" | "info";
  text: string;
}

interface AppStore {
  edition: EditionInfo;
  config: AppConfig | null;
  status: ProxyServerStatus | null;
  tunnelStatus: TunnelStatus | null;
  diagnostics: DiagnosticsReport | null;
  logs: ProxyLogEntry[];
  logsPaused: boolean;
  activeTab: ActiveTab;
  selectedGroupId: string;
  loading: boolean;
  busyAction: string | null;
  toasts: ToastMessage[];

  setActiveTab: (tab: ActiveTab) => void;
  setSelectedGroupId: (groupId: string) => void;
  setLogsPaused: (paused: boolean) => void;
  pushLog: (entry: ProxyLogEntry) => void;
  setStatus: (status: ProxyServerStatus) => void;
  setTunnelStatus: (status: TunnelStatus) => void;
  addToast: (kind: ToastMessage["kind"], text: string) => void;
  dismissToast: (id: string) => void;

  bootstrap: () => Promise<void>;
  refreshDiagnostics: () => Promise<void>;

  addProxy: (input: AddProxyInput) => Promise<void>;
  updateProxy: (input: UpdateProxyInput) => Promise<void>;
  deleteProxy: (id: string) => Promise<void>;
  toggleProxy: (id: string, enabled: boolean) => Promise<void>;
  setProxyTunnel: (
    id: string,
    tunnelEnabled: boolean,
    tunnelRouteName?: string | null,
  ) => Promise<void>;

  startTunnel: () => Promise<void>;
  stopTunnel: () => Promise<void>;

  addGroup: (name: string, color: string) => Promise<void>;
  updateGroup: (id: string, name: string, color: string) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;

  updateSettings: (settings: AppSettings) => Promise<void>;
  importConfig: (json: string) => Promise<void>;

  startProxy: () => Promise<void>;
  stopProxy: () => Promise<void>;
  clearLogs: () => Promise<void>;
  trustRootCa: () => Promise<void>;
  untrustRootCa: () => Promise<void>;
  syncHosts: () => Promise<void>;
}

const MAX_LOGS = 500;

function formatError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "An unexpected error occurred.";
}

export const useAppStore = create<AppStore>((set, get) => ({
  edition: { edition: "standard", tunnelSupported: false },
  config: null,
  status: null,
  tunnelStatus: null,
  diagnostics: null,
  logs: [],
  logsPaused: false,
  activeTab: "proxies",
  selectedGroupId: "all",
  loading: true,
  busyAction: null,
  toasts: [],

  setActiveTab: (tab) => set({ activeTab: tab }),
  setSelectedGroupId: (groupId) => set({ selectedGroupId: groupId }),
  setLogsPaused: (paused) => set({ logsPaused: paused }),

  pushLog: (entry) => {
    if (get().logsPaused) return;
    set((s) => {
      const next = [entry, ...s.logs];
      if (next.length > MAX_LOGS) next.length = MAX_LOGS;
      return { logs: next };
    });
  },

  setStatus: (status) => set({ status }),
  setTunnelStatus: (tunnelStatus) => set({ tunnelStatus }),

  addToast: (kind, text) => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    set((s) => ({ toasts: [...s.toasts, { id, kind, text }] }));
    window.setTimeout(() => {
      get().dismissToast(id);
    }, 4500);
  },

  dismissToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  bootstrap: async () => {
    set({ loading: true });
    try {
      const [edition, config, status, tunnelStatus, diagnostics, logs] =
        await Promise.all([
          api.getEditionInfo(),
          api.getConfig(),
          api.getProxyStatus(),
          api.getTunnelStatus(),
          api.getDiagnostics(),
          api.getLogs(),
        ]);
      set({
        edition,
        config,
        status,
        tunnelStatus,
        diagnostics,
        logs: [...logs].reverse(),
        loading: false,
      });
    } catch (err) {
      set({ loading: false });
      get().addToast("error", formatError(err));
    }
  },

  refreshDiagnostics: async () => {
    try {
      const diagnostics = await api.getDiagnostics();
      set({ diagnostics });
    } catch (err) {
      get().addToast("error", formatError(err));
    }
  },

  addProxy: async (input) => {
    const config = await api.addProxy(input);
    set({ config });
    get().addToast("success", `Added https://${input.hostname}`);
    await get().refreshDiagnostics();
  },

  updateProxy: async (input) => {
    const config = await api.updateProxy(input);
    set({ config });
    get().addToast("success", `Updated ${input.hostname}`);
    await get().refreshDiagnostics();
  },

  deleteProxy: async (id) => {
    const config = await api.deleteProxy(id);
    set({ config });
    get().addToast("info", "Proxy removed");
    await get().refreshDiagnostics();
  },

  toggleProxy: async (id, enabled) => {
    try {
      const config = await api.toggleProxy(id, enabled);
      set({ config });
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    }
  },

  setProxyTunnel: async (id, tunnelEnabled, tunnelRouteName) => {
    try {
      const config = await api.setProxyTunnel(
        id,
        tunnelEnabled,
        tunnelRouteName,
      );
      set({ config });
      get().addToast(
        "info",
        tunnelEnabled
          ? "Proxy enabled for public tunnel"
          : "Proxy removed from public tunnel",
      );
    } catch (err) {
      get().addToast("error", formatError(err));
    }
  },

  startTunnel: async () => {
    set({ busyAction: "tunnel" });
    try {
      const tunnelStatus = await api.startTunnel();
      set({ tunnelStatus });
      get().addToast(
        "success",
        tunnelStatus.hostname
          ? `OpenTunnel active on ${tunnelStatus.hostname}`
          : "OpenTunnel connecting...",
      );
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  stopTunnel: async () => {
    set({ busyAction: "tunnel" });
    try {
      const tunnelStatus = await api.stopTunnel();
      set({ tunnelStatus });
      get().addToast("info", "OpenTunnel disconnected");
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  addGroup: async (name, color) => {
    const config = await api.addGroup(name, color);
    set({ config });
    get().addToast("success", `Group "${name}" created`);
  },

  updateGroup: async (id, name, color) => {
    const config = await api.updateGroup(id, name, color);
    set({ config });
    get().addToast("success", `Group "${name}" updated`);
  },

  deleteGroup: async (id) => {
    const config = await api.deleteGroup(id);
    set((s) => ({
      config,
      selectedGroupId: s.selectedGroupId === id ? "all" : s.selectedGroupId,
    }));
    get().addToast("info", "Group deleted");
  },

  updateSettings: async (settings) => {
    const config = await api.updateSettings(settings);
    set({ config });
    get().addToast("success", "Settings saved");
    await get().refreshDiagnostics();
  },

  importConfig: async (json) => {
    const config = await api.importConfig(json);
    set({ config });
    get().addToast("success", "Configuration imported");
    await get().refreshDiagnostics();
  },

  startProxy: async () => {
    set({ busyAction: "proxy" });
    try {
      const status = await api.startProxyServer();
      set({ status });
      get().addToast(
        "success",
        `Proxy server running on https://127.0.0.1:${status.httpsPort}`,
      );
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  stopProxy: async () => {
    set({ busyAction: "proxy" });
    try {
      const status = await api.stopProxyServer();
      set({ status });
      get().addToast("info", "Proxy server stopped");
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  clearLogs: async () => {
    await api.clearLogs();
    set({ logs: [] });
  },

  trustRootCa: async () => {
    set({ busyAction: "ca" });
    try {
      await api.trustRootCa();
      get().addToast("success", "Ophiuchi-rs Root CA is now trusted by your OS");
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  untrustRootCa: async () => {
    set({ busyAction: "ca" });
    try {
      await api.untrustRootCa();
      get().addToast("info", "Ophiuchi-rs Root CA removed from OS trust store");
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },

  syncHosts: async () => {
    set({ busyAction: "hosts" });
    try {
      await api.syncHosts();
      get().addToast("success", "Hosts file synced successfully");
      await get().refreshDiagnostics();
    } catch (err) {
      get().addToast("error", formatError(err));
    } finally {
      set({ busyAction: null });
    }
  },
}));
