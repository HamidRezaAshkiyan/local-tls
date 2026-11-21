import { Download, Save, Upload } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { api } from "../lib/tauri";
import { useAppStore } from "../store/useAppStore";

export function SettingsView() {
  const { edition, config, updateSettings, importConfig, addToast } =
    useAppStore();
  const settings = config?.settings;

  const [httpsPort, setHttpsPort] = useState(
    String(settings?.httpsPort ?? 443),
  );
  const [httpPort, setHttpPort] = useState(String(settings?.httpPort ?? 80));
  const [enableHttpRedirect, setEnableHttpRedirect] = useState(
    settings?.enableHttpRedirect ?? true,
  );
  const [minimizeToTray, setMinimizeToTray] = useState(
    settings?.minimizeToTray ?? true,
  );
  const [theme, setTheme] = useState(settings?.theme ?? "dark");
  const [tunnelApiUrl, setTunnelApiUrl] = useState(
    settings?.tunnelApiUrl ?? "",
  );
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!settings) return;
    setHttpsPort(String(settings.httpsPort));
    setHttpPort(String(settings.httpPort));
    setEnableHttpRedirect(settings.enableHttpRedirect);
    setMinimizeToTray(settings.minimizeToTray);
    setTheme(settings.theme);
    setTunnelApiUrl(settings.tunnelApiUrl ?? "");
  }, [settings]);

  const handleSave = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    try {
      await updateSettings({
        httpsPort: Number(httpsPort),
        httpPort: Number(httpPort),
        enableHttpRedirect,
        minimizeToTray,
        theme,
        tunnelApiUrl: tunnelApiUrl.trim() || null,
      });
    } catch (err) {
      addToast("error", typeof err === "string" ? err : "Failed to save settings");
    } finally {
      setSaving(false);
    }
  };

  const handleExport = async () => {
    try {
      const json = await api.exportConfig();
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "ophiuchi-rs-config.json";
      a.click();
      URL.revokeObjectURL(url);
      addToast("success", "Exported ophiuchi-rs-config.json");
    } catch (err) {
      addToast("error", typeof err === "string" ? err : "Export failed");
    }
  };

  const handleImportFile = async (file: File | null) => {
    if (!file) return;
    try {
      const text = await file.text();
      await importConfig(text);
    } catch (err) {
      addToast("error", typeof err === "string" ? err : "Import failed");
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <header className="border-b border-zinc-200 px-6 py-5 dark:border-zinc-800/80">
        <h2 className="text-lg font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
          Settings
        </h2>
        <p className="text-xs text-zinc-500 dark:text-zinc-400">
          Configure proxy ports, background tray behavior, appearance, and
          backups.
        </p>
      </header>

      <div className="flex-1 overflow-y-auto p-6">
        <form onSubmit={handleSave} className="max-w-xl space-y-6">
          <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-zinc-800 dark:bg-zinc-900/60">
            <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
              Proxy Server Ports
            </h3>
            <p className="mt-1 text-xs text-zinc-500">
              Restart the proxy server after changing listen ports.
            </p>

            <div className="mt-4 grid grid-cols-2 gap-4">
              <div>
                <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                  HTTPS Port
                </label>
                <input
                  type="number"
                  min={1}
                  max={65535}
                  value={httpsPort}
                  onChange={(e) => setHttpsPort(e.target.value)}
                  className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 font-mono text-sm text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
                />
              </div>
              <div>
                <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                  HTTP Redirect Port
                </label>
                <input
                  type="number"
                  min={1}
                  max={65535}
                  disabled={!enableHttpRedirect}
                  value={httpPort}
                  onChange={(e) => setHttpPort(e.target.value)}
                  className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 font-mono text-sm text-zinc-900 outline-none focus:border-emerald-500 disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
                />
              </div>
            </div>

            <label className="mt-4 flex cursor-pointer items-center gap-2.5 text-xs text-zinc-700 dark:text-zinc-300">
              <input
                type="checkbox"
                checked={enableHttpRedirect}
                onChange={(e) => setEnableHttpRedirect(e.target.checked)}
                className="h-4 w-4 accent-emerald-600"
              />
              Redirect HTTP requests on port {httpPort} to HTTPS ({httpsPort})
            </label>
          </div>

          <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-zinc-800 dark:bg-zinc-900/60">
            <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
              Desktop &amp; Appearance
            </h3>

            <label className="mt-4 flex cursor-pointer items-center gap-2.5 text-xs text-zinc-700 dark:text-zinc-300">
              <input
                type="checkbox"
                checked={minimizeToTray}
                onChange={(e) => setMinimizeToTray(e.target.checked)}
                className="h-4 w-4 accent-emerald-600"
              />
              Keep proxy running in system tray / menubar when closing the
              window
            </label>

            <div className="mt-4">
              <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                Theme
              </label>
              <select
                value={theme}
                onChange={(e) => setTheme(e.target.value)}
                className="w-48 rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 text-xs text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </div>
          </div>

          {edition.tunnelSupported && (
            <div className="rounded-xl border border-violet-500/25 bg-white p-5 dark:bg-zinc-900/60">
              <h3 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
                OpenTunnel (Tunnel Edition)
              </h3>
              <p className="mt-1 text-xs text-zinc-500">
                Leave blank to use the default hosted blind-TLS service (
                <code>https://opentunnel.xyz</code>), or enter your self-hosted
                OpenTunnel server URL.
              </p>
              <div className="mt-4">
                <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                  Custom OpenTunnel API URL (optional)
                </label>
                <input
                  type="url"
                  placeholder="https://opentunnel.xyz"
                  value={tunnelApiUrl}
                  onChange={(e) => setTunnelApiUrl(e.target.value)}
                  className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 font-mono text-xs text-zinc-900 outline-none focus:border-violet-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
                />
              </div>
            </div>
          )}

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => void handleExport()}
                className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
              >
                <Download className="h-3.5 w-3.5" />
                Export JSON
              </button>
              <label className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800">
                <Upload className="h-3.5 w-3.5" />
                Import JSON
                <input
                  type="file"
                  accept="application/json"
                  className="hidden"
                  onChange={(e) =>
                    void handleImportFile(e.target.files?.[0] ?? null)
                  }
                />
              </label>
            </div>

            <button
              type="submit"
              disabled={saving}
              className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-emerald-600 px-4 py-2 text-xs font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
            >
              <Save className="h-3.5 w-3.5" />
              {saving ? "Saving..." : "Save Settings"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
