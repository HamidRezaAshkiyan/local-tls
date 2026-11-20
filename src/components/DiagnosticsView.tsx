import {
  CheckCircle2,
  FileText,
  FolderOpen,
  RefreshCw,
  ShieldCheck,
  ShieldOff,
  XCircle,
} from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/tauri";
import { useAppStore } from "../store/useAppStore";

export function DiagnosticsView() {
  const {
    diagnostics,
    busyAction,
    refreshDiagnostics,
    trustRootCa,
    untrustRootCa,
    syncHosts,
    addToast,
  } = useAppStore();

  const [hostsContent, setHostsContent] = useState<string>("");
  const [showHosts, setShowHosts] = useState(false);

  useEffect(() => {
    void refreshDiagnostics();
  }, [refreshDiagnostics]);

  const loadHostsPreview = async () => {
    try {
      const content = await api.getHostsContent();
      setHostsContent(content);
      setShowHosts(true);
    } catch (err) {
      addToast("error", typeof err === "string" ? err : "Cannot read hosts");
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <header className="flex items-center justify-between border-b border-zinc-200 px-6 py-5 dark:border-zinc-800/80">
        <div>
          <h2 className="text-lg font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
            System Diagnostics
          </h2>
          <p className="text-xs text-zinc-500 dark:text-zinc-400">
            Inspect Local Root CA trust, hosts file synchronization, and port
            availability.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void api.openDataDir()}
            className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <FolderOpen className="h-3.5 w-3.5" />
            Open Data Folder
          </button>
          <button
            type="button"
            onClick={() => void refreshDiagnostics()}
            className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 py-2 text-xs font-semibold text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-950 dark:hover:bg-zinc-200"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Re-check
          </button>
        </div>
      </header>

      <div className="flex-1 space-y-5 overflow-y-auto p-6">
        {diagnostics && (
          <>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {/* Root CA Card */}
              <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-zinc-800 dark:bg-zinc-900/60">
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                      Local Root CA ({diagnostics.os})
                    </h3>
                    <p className="mt-1 text-xs text-zinc-500">
                      Trusted once in{" "}
                      {diagnostics.os === "macOS"
                        ? "macOS System Keychain"
                        : "Windows User Root Cert Store"}
                      .
                    </p>
                  </div>
                  {diagnostics.caTrusted ? (
                    <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                  ) : (
                    <XCircle className="h-5 w-5 text-amber-500" />
                  )}
                </div>

                <div className="mt-3 rounded-lg bg-zinc-100 px-3 py-2 font-mono text-[11px] break-all text-zinc-600 dark:bg-zinc-950 dark:text-zinc-400">
                  {diagnostics.caCertPath}
                </div>

                <div className="mt-4 flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={busyAction === "ca"}
                    onClick={() => void trustRootCa()}
                    className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
                  >
                    <ShieldCheck className="h-3.5 w-3.5" />
                    {diagnostics.caTrusted
                      ? "Re-trust Root CA"
                      : "Install & Trust Root CA"}
                  </button>
                  {diagnostics.caTrusted && (
                    <button
                      type="button"
                      disabled={busyAction === "ca"}
                      onClick={() => void untrustRootCa()}
                      className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs font-medium text-rose-600 hover:bg-rose-500/20 disabled:opacity-50 dark:text-rose-400"
                    >
                      <ShieldOff className="h-3.5 w-3.5" />
                      Remove Trust
                    </button>
                  )}
                </div>
              </div>

              {/* Hosts File Card */}
              <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-zinc-800 dark:bg-zinc-900/60">
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                      OS Hosts File Sync
                    </h3>
                    <p className="mt-1 text-xs text-zinc-500">
                      Managed block preserves your custom entries and creates
                      backups before writes.
                    </p>
                  </div>
                  {diagnostics.hostsSynced ? (
                    <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                  ) : (
                    <XCircle className="h-5 w-5 text-amber-500" />
                  )}
                </div>

                <div className="mt-3 rounded-lg bg-zinc-100 px-3 py-2 font-mono text-[11px] break-all text-zinc-600 dark:bg-zinc-950 dark:text-zinc-400">
                  {diagnostics.hostsPath}
                </div>

                <div className="mt-4 flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={busyAction === "hosts"}
                    onClick={() => void syncHosts()}
                    className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-sky-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-500 disabled:opacity-50"
                  >
                    <RefreshCw
                      className={`h-3.5 w-3.5 ${busyAction === "hosts" ? "animate-spin" : ""}`}
                    />
                    Sync Hosts Now
                  </button>
                  <button
                    type="button"
                    onClick={() => void loadHostsPreview()}
                    className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
                  >
                    <FileText className="h-3.5 w-3.5" />
                    Inspect Hosts File
                  </button>
                </div>
              </div>

              {/* Port Availability Card */}
              <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-zinc-800 dark:bg-zinc-900/60 md:col-span-2">
                <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                  Network Ports &amp; Runtime
                </h3>
                <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
                  <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-950/60">
                    <div className="text-[11px] text-zinc-500">
                      HTTPS Listen Port
                    </div>
                    <div className="mt-1 flex items-center justify-between">
                      <span className="font-mono text-sm font-semibold">
                        :{diagnostics.httpsPort}
                      </span>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${
                          diagnostics.httpsPortAvailable
                            ? "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400"
                            : "bg-rose-500/15 text-rose-600 dark:text-rose-400"
                        }`}
                      >
                        {diagnostics.proxyRunning
                          ? "Active"
                          : diagnostics.httpsPortAvailable
                            ? "Available"
                            : "In Use"}
                      </span>
                    </div>
                  </div>

                  <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-950/60">
                    <div className="text-[11px] text-zinc-500">
                      HTTP Redirect Port
                    </div>
                    <div className="mt-1 flex items-center justify-between">
                      <span className="font-mono text-sm font-semibold">
                        :{diagnostics.httpPort}
                      </span>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${
                          diagnostics.httpPortAvailable
                            ? "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400"
                            : "bg-amber-500/15 text-amber-600 dark:text-amber-400"
                        }`}
                      >
                        {diagnostics.proxyRunning
                          ? "Active"
                          : diagnostics.httpPortAvailable
                            ? "Available"
                            : "In Use"}
                      </span>
                    </div>
                  </div>

                  <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-3 dark:border-zinc-800 dark:bg-zinc-950/60">
                    <div className="text-[11px] text-zinc-500">
                      Active Routes
                    </div>
                    <div className="mt-1 font-mono text-sm font-semibold">
                      {diagnostics.activeRouteCount}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {showHosts && (
              <div className="rounded-xl border border-zinc-200 bg-white p-4 dark:border-zinc-800 dark:bg-zinc-900/60">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                    {diagnostics.hostsPath}
                  </span>
                  <button
                    type="button"
                    onClick={() => setShowHosts(false)}
                    className="cursor-pointer text-xs text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100"
                  >
                    Hide
                  </button>
                </div>
                <pre className="max-h-72 overflow-auto rounded-lg bg-zinc-950 p-3 font-mono text-[11px] leading-relaxed text-zinc-300">
                  {hostsContent}
                </pre>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
