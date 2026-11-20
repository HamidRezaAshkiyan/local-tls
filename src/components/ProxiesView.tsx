import {
  AlertTriangle,
  Check,
  Copy,
  ExternalLink,
  FolderPlus,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  ShieldAlert,
  Trash2,
} from "lucide-react";
import { useMemo, useState } from "react";
import { api } from "../lib/tauri";
import { useAppStore } from "../store/useAppStore";
import type { ProxyEntry } from "../types";
import { GroupModal } from "./GroupModal";
import { ProxyModal } from "./ProxyModal";

export function ProxiesView() {
  const {
    config,
    status,
    diagnostics,
    selectedGroupId,
    setSelectedGroupId,
    toggleProxy,
    deleteProxy,
    syncHosts,
    trustRootCa,
    busyAction,
    addToast,
  } = useAppStore();

  const [search, setSearch] = useState("");
  const [proxyModalOpen, setProxyModalOpen] = useState(false);
  const [editingProxy, setEditingProxy] = useState<ProxyEntry | null>(null);
  const [groupModalOpen, setGroupModalOpen] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const groups = config?.groups ?? [];
  const proxies = config?.proxies ?? [];
  const httpsPort = status?.httpsPort ?? config?.settings.httpsPort ?? 443;

  const groupMap = useMemo(() => {
    const map = new Map<string, { name: string; color: string }>();
    for (const g of groups) {
      map.set(g.id, { name: g.name, color: g.color });
    }
    return map;
  }, [groups]);

  const unsyncedSet = useMemo(
    () => new Set(diagnostics?.unsyncedHosts ?? []),
    [diagnostics],
  );

  const filteredProxies = useMemo(() => {
    const q = search.trim().toLowerCase();
    return proxies.filter((p) => {
      if (selectedGroupId !== "all" && p.groupId !== selectedGroupId) {
        return false;
      }
      if (!q) return true;
      return (
        p.hostname.toLowerCase().includes(q) ||
        p.targetHost.toLowerCase().includes(q) ||
        String(p.targetPort).includes(q)
      );
    });
  }, [proxies, selectedGroupId, search]);

  const formatUrl = (hostname: string) =>
    httpsPort === 443
      ? `https://${hostname}`
      : `https://${hostname}:${httpsPort}`;

  const handleCopy = async (proxy: ProxyEntry) => {
    const url = formatUrl(proxy.hostname);
    try {
      await navigator.clipboard.writeText(url);
      setCopiedId(proxy.id);
      addToast("info", `Copied ${url}`);
      window.setTimeout(() => setCopiedId(null), 1500);
    } catch {
      addToast("error", "Could not access clipboard");
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <header className="flex flex-wrap items-center justify-between gap-4 border-b border-zinc-200 px-6 py-5 dark:border-zinc-800/80">
        <div>
          <h2 className="text-lg font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
            SSL Proxies
          </h2>
          <p className="text-xs text-zinc-500 dark:text-zinc-400">
            Map local domains over trusted HTTPS directly to your local dev
            servers.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setGroupModalOpen(true)}
            className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <FolderPlus className="h-3.5 w-3.5" />
            Groups
          </button>
          <button
            type="button"
            onClick={() => {
              setEditingProxy(null);
              setProxyModalOpen(true);
            }}
            className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-emerald-600 px-3.5 py-2 text-xs font-semibold text-white shadow-sm shadow-emerald-600/20 hover:bg-emerald-500"
          >
            <Plus className="h-3.5 w-3.5" />
            Add Proxy
          </button>
        </div>
      </header>

      {/* Actionable Onboarding / Sync Banners */}
      <div className="space-y-2 px-6 pt-4">
        {diagnostics && !diagnostics.caTrusted && (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3">
            <div className="flex items-center gap-2.5 text-xs text-amber-800 dark:text-amber-300">
              <ShieldAlert className="h-4 w-4 shrink-0" />
              <span>
                <strong>LocalTLS Root CA is not trusted yet.</strong> Trust it
                once so every local domain gets a green padlock automatically.
              </span>
            </div>
            <button
              type="button"
              disabled={busyAction === "ca"}
              onClick={() => void trustRootCa()}
              className="cursor-pointer rounded-lg bg-amber-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-amber-500 disabled:opacity-50"
            >
              {busyAction === "ca" ? "Trusting..." : "Trust Root CA"}
            </button>
          </div>
        )}

        {diagnostics && !diagnostics.hostsSynced && (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-sky-500/30 bg-sky-500/10 px-4 py-3">
            <div className="flex items-center gap-2.5 text-xs text-sky-800 dark:text-sky-300">
              <AlertTriangle className="h-4 w-4 shrink-0" />
              <span>
                <strong>
                  {diagnostics.unsyncedHosts.length} active domain
                  {diagnostics.unsyncedHosts.length === 1 ? "" : "s"}
                </strong>{" "}
                not yet synced to your OS hosts file (
                <code>{diagnostics.unsyncedHosts.join(", ")}</code>).
              </span>
            </div>
            <button
              type="button"
              disabled={busyAction === "hosts"}
              onClick={() => void syncHosts()}
              className="flex cursor-pointer items-center gap-1.5 rounded-lg bg-sky-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-sky-500 disabled:opacity-50"
            >
              <RefreshCw
                className={`h-3.5 w-3.5 ${busyAction === "hosts" ? "animate-spin" : ""}`}
              />
              {busyAction === "hosts" ? "Syncing..." : "Sync Hosts File"}
            </button>
          </div>
        )}
      </div>

      {/* Group Tabs & Search */}
      <div className="flex flex-wrap items-center justify-between gap-3 px-6 py-4">
        <div className="flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            onClick={() => setSelectedGroupId("all")}
            className={`cursor-pointer rounded-lg px-3 py-1.5 text-xs font-medium transition ${
              selectedGroupId === "all"
                ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-950"
                : "bg-zinc-100 text-zinc-600 hover:bg-zinc-200 dark:bg-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800"
            }`}
          >
            All ({proxies.length})
          </button>
          {groups.map((g) => {
            const count = proxies.filter((p) => p.groupId === g.id).length;
            const active = selectedGroupId === g.id;
            return (
              <button
                key={g.id}
                type="button"
                onClick={() => setSelectedGroupId(g.id)}
                className={`flex cursor-pointer items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition ${
                  active
                    ? "bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-950"
                    : "bg-zinc-100 text-zinc-600 hover:bg-zinc-200 dark:bg-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800"
                }`}
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: g.color }}
                />
                {g.name} ({count})
              </button>
            );
          })}
        </div>

        <div className="relative w-60">
          <Search className="pointer-events-none absolute top-1/2 left-3 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400" />
          <input
            type="text"
            placeholder="Filter domain or port..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full rounded-lg border border-zinc-200 bg-white py-1.5 pr-3 pl-8 text-xs text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-100"
          />
        </div>
      </div>

      {/* Proxy Table */}
      <div className="flex-1 overflow-y-auto px-6 pb-6">
        {filteredProxies.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-zinc-300 py-16 text-center dark:border-zinc-800">
            <p className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
              {proxies.length === 0
                ? "No SSL proxies configured yet"
                : "No proxies match your filter"}
            </p>
            <p className="mt-1 max-w-sm text-xs text-zinc-500">
              Add a local domain like <code>app.myapp.test</code> pointing to{" "}
              <code>127.0.0.1:3000</code> and get instant HTTPS with WebSocket
              support.
            </p>
            {proxies.length === 0 && (
              <button
                type="button"
                onClick={() => {
                  setEditingProxy(null);
                  setProxyModalOpen(true);
                }}
                className="mt-4 flex cursor-pointer items-center gap-1.5 rounded-lg bg-emerald-600 px-4 py-2 text-xs font-semibold text-white hover:bg-emerald-500"
              >
                <Plus className="h-3.5 w-3.5" />
                Add Your First Proxy
              </button>
            )}
          </div>
        ) : (
          <div className="overflow-hidden rounded-xl border border-zinc-200 bg-white dark:border-zinc-800/80 dark:bg-zinc-900/50">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-zinc-200 bg-zinc-50/80 text-[11px] font-semibold tracking-wider text-zinc-500 uppercase dark:border-zinc-800 dark:bg-zinc-900/80 dark:text-zinc-400">
                  <th className="w-14 px-4 py-3">On</th>
                  <th className="px-4 py-3">HTTPS Domain</th>
                  <th className="px-4 py-3">Target Upstream</th>
                  <th className="px-4 py-3">Group</th>
                  <th className="px-4 py-3">Hosts</th>
                  <th className="px-4 py-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-200/80 dark:divide-zinc-800/70">
                {filteredProxies.map((proxy) => {
                  const group = groupMap.get(proxy.groupId);
                  const url = formatUrl(proxy.hostname);
                  const isSynced =
                    !proxy.enabled ||
                    !unsyncedSet.has(proxy.hostname.toLowerCase());

                  return (
                    <tr
                      key={proxy.id}
                      className={`transition hover:bg-zinc-50 dark:hover:bg-zinc-800/40 ${
                        !proxy.enabled ? "opacity-55" : ""
                      }`}
                    >
                      <td className="px-4 py-3">
                        <button
                          type="button"
                          role="switch"
                          aria-checked={proxy.enabled}
                          onClick={() =>
                            void toggleProxy(proxy.id, !proxy.enabled)
                          }
                          className={`relative inline-flex h-5 w-9 cursor-pointer items-center rounded-full transition ${
                            proxy.enabled
                              ? "bg-emerald-600"
                              : "bg-zinc-300 dark:bg-zinc-700"
                          }`}
                        >
                          <span
                            className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white transition ${
                              proxy.enabled
                                ? "translate-x-4.5"
                                : "translate-x-1"
                            }`}
                          />
                        </button>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={() => void api.openExternalUrl(url)}
                            className="flex cursor-pointer items-center gap-1.5 font-mono text-xs font-medium text-emerald-600 hover:underline dark:text-emerald-400"
                          >
                            {url}
                            <ExternalLink className="h-3 w-3 opacity-60" />
                          </button>
                          <button
                            type="button"
                            onClick={() => void handleCopy(proxy)}
                            title="Copy URL"
                            className="cursor-pointer rounded p-1 text-zinc-400 hover:bg-zinc-200/70 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
                          >
                            {copiedId === proxy.id ? (
                              <Check className="h-3 w-3 text-emerald-500" />
                            ) : (
                              <Copy className="h-3 w-3" />
                            )}
                          </button>
                        </div>
                      </td>
                      <td className="px-4 py-3 font-mono text-xs text-zinc-600 dark:text-zinc-300">
                        http://{proxy.targetHost}:{proxy.targetPort}
                      </td>
                      <td className="px-4 py-3">
                        {group && (
                          <span className="inline-flex items-center gap-1.5 rounded-full bg-zinc-100 px-2.5 py-0.5 text-[11px] font-medium text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                            <span
                              className="h-2 w-2 rounded-full"
                              style={{ backgroundColor: group.color }}
                            />
                            {group.name}
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3">
                        {!proxy.enabled ? (
                          <span className="text-[11px] text-zinc-400">
                            Disabled
                          </span>
                        ) : isSynced ? (
                          <span className="inline-flex items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400">
                            Synced
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 rounded-full bg-amber-500/15 px-2 py-0.5 text-[11px] font-medium text-amber-700 dark:text-amber-400">
                            Pending
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="inline-flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => {
                              setEditingProxy(proxy);
                              setProxyModalOpen(true);
                            }}
                            title="Edit proxy"
                            className="cursor-pointer rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
                          >
                            <Pencil className="h-3.5 w-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => void deleteProxy(proxy.id)}
                            title="Delete proxy"
                            className="cursor-pointer rounded-lg p-1.5 text-zinc-500 hover:bg-rose-500/15 hover:text-rose-500 dark:text-zinc-400"
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {proxyModalOpen && (
        <ProxyModal
          existing={editingProxy}
          defaultGroupId={selectedGroupId}
          onClose={() => {
            setProxyModalOpen(false);
            setEditingProxy(null);
          }}
        />
      )}

      {groupModalOpen && (
        <GroupModal onClose={() => setGroupModalOpen(false)} />
      )}
    </div>
  );
}
