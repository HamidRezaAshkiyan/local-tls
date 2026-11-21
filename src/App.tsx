import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { DiagnosticsView } from "./components/DiagnosticsView";
import { LogsView } from "./components/LogsView";
import { ProxiesView } from "./components/ProxiesView";
import { SettingsView } from "./components/SettingsView";
import { Sidebar } from "./components/Sidebar";
import { ToastContainer } from "./components/ToastContainer";
import { useAppStore } from "./store/useAppStore";
import type { ProxyLogEntry, ProxyServerStatus, TunnelStatus } from "./types";

export default function App() {
  const {
    activeTab,
    config,
    loading,
    bootstrap,
    pushLog,
    setStatus,
    setTunnelStatus,
  } = useAppStore();

  useEffect(() => {
    void bootstrap();

    const unlistenLog = listen<ProxyLogEntry>("proxy-log", (event) => {
      pushLog(event.payload);
    });

    const unlistenStatus = listen<ProxyServerStatus>(
      "proxy-status-changed",
      (event) => {
        setStatus(event.payload);
      },
    );

    const unlistenTunnel = listen<TunnelStatus>(
      "tunnel-status-changed",
      (event) => {
        setTunnelStatus(event.payload);
      },
    );

    return () => {
      void unlistenLog.then((fn) => fn());
      void unlistenStatus.then((fn) => fn());
      void unlistenTunnel.then((fn) => fn());
    };
  }, [bootstrap, pushLog, setStatus, setTunnelStatus]);

  useEffect(() => {
    const theme = config?.settings.theme ?? "dark";
    const root = document.documentElement;
    if (theme === "light") {
      root.classList.remove("dark");
    } else {
      root.classList.add("dark");
    }
  }, [config?.settings.theme]);

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-zinc-950 text-sm text-zinc-400">
        Loading LocalTLS...
      </div>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <Sidebar />
      <main className="flex-1 overflow-hidden">
        {activeTab === "proxies" && <ProxiesView />}
        {activeTab === "logs" && <LogsView />}
        {activeTab === "diagnostics" && <DiagnosticsView />}
        {activeTab === "settings" && <SettingsView />}
      </main>
      <ToastContainer />
    </div>
  );
}
