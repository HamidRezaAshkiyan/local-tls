import {
  Activity,
  Cloud,
  Globe,
  Moon,
  Power,
  Settings,
  ShieldCheck,
  Sun,
  Terminal,
} from "lucide-react";
import pkg from "../../package.json";
import { useAppStore } from "../store/useAppStore";
import type { ActiveTab } from "../types";

const NAV_ITEMS: { id: ActiveTab; label: string; icon: typeof Globe }[] = [
  { id: "proxies", label: "Proxies", icon: Globe },
  { id: "logs", label: "Live Logs", icon: Terminal },
  { id: "diagnostics", label: "Diagnostics", icon: Activity },
  { id: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const {
    edition,
    activeTab,
    setActiveTab,
    status,
    tunnelStatus,
    diagnostics,
    logs,
    config,
    busyAction,
    startProxy,
    stopProxy,
    startTunnel,
    stopTunnel,
    updateSettings,
  } = useAppStore();

  const running = status?.running ?? false;
  const tunnelRunning = tunnelStatus?.running ?? false;
  const isDark = (config?.settings.theme ?? "dark") !== "light";

  const toggleTheme = () => {
    if (!config) return;
    void updateSettings({
      ...config.settings,
      theme: isDark ? "light" : "dark",
    });
  };

  return (
    <aside className="flex h-full w-64 shrink-0 flex-col border-r border-zinc-200 bg-zinc-50 dark:border-zinc-800/80 dark:bg-zinc-950">
      {/* Brand header */}
      <div className="flex items-center gap-3 px-5 py-5">
        <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-500/15 text-emerald-500 ring-1 ring-emerald-500/30">
          <ShieldCheck className="h-5 w-5" />
        </div>
        <div>
          <div className="flex items-center gap-1.5">
            <h1 className="text-sm font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
              Ophiuchi-rs
            </h1>
            <span
              className={`rounded px-1.5 py-0.5 text-[9px] font-semibold tracking-wide uppercase ${
                edition.tunnelSupported
                  ? "bg-violet-500/15 text-violet-600 dark:text-violet-400"
                  : "bg-zinc-200 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400"
              }`}
            >
              {edition.tunnelSupported ? "Tunnel" : "Standard"}
            </span>
          </div>
          <p className="text-[11px] text-zinc-500 dark:text-zinc-400">
            {diagnostics?.os ?? "Desktop"} SSL Proxy ·{" "}
            <span className="font-mono">v{pkg.version}</span>
          </p>
        </div>
      </div>

      {/* Primary Proxy Server Control */}
      <div className="space-y-2.5 px-4 pb-4">
        <div className="rounded-xl border border-zinc-200 bg-white p-3.5 shadow-sm dark:border-zinc-800 dark:bg-zinc-900/70">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span
                className={`h-2.5 w-2.5 rounded-full ${
                  running
                    ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.75)]"
                    : "bg-zinc-400 dark:bg-zinc-600"
                }`}
              />
              <span className="text-xs font-medium text-zinc-700 dark:text-zinc-200">
                {running ? "Proxy Running" : "Proxy Stopped"}
              </span>
            </div>
            <span className="rounded-md bg-zinc-100 px-1.5 py-0.5 font-mono text-[10px] text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400">
              :{status?.httpsPort ?? config?.settings.httpsPort ?? 443}
            </span>
          </div>

          <button
            type="button"
            disabled={busyAction === "proxy"}
            onClick={() => (running ? void stopProxy() : void startProxy())}
            className={`flex w-full cursor-pointer items-center justify-center gap-2 rounded-lg px-3 py-2 text-xs font-semibold transition ${
              running
                ? "bg-rose-500/15 text-rose-600 hover:bg-rose-500/25 dark:text-rose-400"
                : "bg-emerald-600 text-white hover:bg-emerald-500 shadow-sm shadow-emerald-600/20"
            } disabled:opacity-50`}
          >
            <Power className="h-3.5 w-3.5" />
            {busyAction === "proxy"
              ? "Working..."
              : running
                ? "Stop Proxy Server"
                : "Start Proxy Server"}
          </button>
        </div>

        {/* OpenTunnel Control Card (Tunnel Edition only) */}
        {edition.tunnelSupported && (
          <div className="rounded-xl border border-violet-500/25 bg-white p-3.5 shadow-sm dark:bg-zinc-900/70">
            <div className="mb-2 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span
                  className={`h-2.5 w-2.5 rounded-full ${
                    tunnelStatus?.state === "connected"
                      ? "bg-violet-500 shadow-[0_0_8px_rgba(139,92,246,0.75)]"
                      : tunnelRunning
                        ? "animate-pulse bg-amber-500"
                        : "bg-zinc-400 dark:bg-zinc-600"
                  }`}
                />
                <span className="text-xs font-medium text-zinc-700 dark:text-zinc-200">
                  OpenTunnel
                </span>
              </div>
              <span className="rounded-md bg-violet-500/10 px-1.5 py-0.5 font-mono text-[10px] text-violet-600 dark:text-violet-400">
                {tunnelStatus?.state ?? "stopped"}
              </span>
            </div>

            {tunnelStatus?.hostname && tunnelRunning && (
              <div className="mb-2.5 truncate font-mono text-[10px] text-zinc-500 dark:text-zinc-400">
                *.{tunnelStatus.hostname}
              </div>
            )}

            <button
              type="button"
              disabled={busyAction === "tunnel"}
              onClick={() =>
                tunnelRunning ? void stopTunnel() : void startTunnel()
              }
              className={`flex w-full cursor-pointer items-center justify-center gap-2 rounded-lg px-3 py-1.5 text-xs font-semibold transition ${
                tunnelRunning
                  ? "bg-rose-500/15 text-rose-600 hover:bg-rose-500/25 dark:text-rose-400"
                  : "bg-violet-600 text-white hover:bg-violet-500 shadow-sm shadow-violet-600/20"
              } disabled:opacity-50`}
            >
              <Cloud className="h-3.5 w-3.5" />
              {busyAction === "tunnel"
                ? "Connecting..."
                : tunnelRunning
                  ? "Disconnect Tunnel"
                  : "Connect Public Tunnel"}
            </button>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 px-3">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const active = activeTab === item.id;
          const showBadge =
            (item.id === "diagnostics" &&
              diagnostics &&
              (!diagnostics.caTrusted || !diagnostics.hostsSynced)) ||
            (item.id === "logs" && logs.length > 0);

          return (
            <button
              key={item.id}
              type="button"
              onClick={() => setActiveTab(item.id)}
              className={`flex w-full cursor-pointer items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition ${
                active
                  ? "bg-zinc-200/80 text-zinc-950 dark:bg-zinc-800/90 dark:text-white"
                  : "text-zinc-600 hover:bg-zinc-200/50 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-900 dark:hover:text-zinc-200"
              }`}
            >
              <span className="flex items-center gap-2.5">
                <Icon className="h-4 w-4" />
                {item.label}
              </span>
              {showBadge && item.id === "diagnostics" && (
                <span className="h-2 w-2 rounded-full bg-amber-500" />
              )}
              {showBadge && item.id === "logs" && (
                <span className="rounded-full bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                  {logs.length}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      {/* Footer status & theme toggle */}
      <div className="flex items-center justify-between border-t border-zinc-200 px-4 py-3 dark:border-zinc-800/80">
        <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
          {status?.activeRoutes ?? 0} active route
          {(status?.activeRoutes ?? 0) === 1 ? "" : "s"}
        </div>
        <button
          type="button"
          onClick={toggleTheme}
          title={isDark ? "Switch to light mode" : "Switch to dark mode"}
          className="cursor-pointer rounded-lg p-1.5 text-zinc-500 transition hover:bg-zinc-200 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
        >
          {isDark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </button>
      </div>
    </aside>
  );
}
