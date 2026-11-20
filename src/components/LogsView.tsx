import { Pause, Play, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { useAppStore } from "../store/useAppStore";

function statusClass(code: number): string {
  if (code === 101)
    return "bg-purple-500/15 text-purple-600 dark:text-purple-400";
  if (code >= 200 && code < 300)
    return "bg-emerald-500/15 text-emerald-700 dark:text-emerald-400";
  if (code >= 300 && code < 400)
    return "bg-sky-500/15 text-sky-700 dark:text-sky-400";
  if (code >= 400 && code < 500)
    return "bg-amber-500/15 text-amber-700 dark:text-amber-400";
  return "bg-rose-500/15 text-rose-700 dark:text-rose-400";
}

export function LogsView() {
  const { logs, logsPaused, setLogsPaused, clearLogs } = useAppStore();
  const [filter, setFilter] = useState("");

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return logs;
    return logs.filter(
      (l) =>
        l.host.toLowerCase().includes(q) ||
        l.path.toLowerCase().includes(q) ||
        l.method.toLowerCase().includes(q) ||
        String(l.status).includes(q) ||
        l.upstream.toLowerCase().includes(q),
    );
  }, [logs, filter]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <header className="flex flex-wrap items-center justify-between gap-4 border-b border-zinc-200 px-6 py-5 dark:border-zinc-800/80">
        <div>
          <h2 className="text-lg font-semibold tracking-tight text-zinc-900 dark:text-zinc-100">
            Live Request Logs
          </h2>
          <p className="text-xs text-zinc-500 dark:text-zinc-400">
            Real-time HTTPS and WebSocket traffic routed through LocalTLS.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="relative w-56">
            <Search className="pointer-events-none absolute top-1/2 left-3 h-3.5 w-3.5 -translate-y-1/2 text-zinc-400" />
            <input
              type="text"
              placeholder="Filter host, path, status..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="w-full rounded-lg border border-zinc-200 bg-white py-1.5 pr-3 pl-8 text-xs text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-100"
            />
          </div>

          <button
            type="button"
            onClick={() => setLogsPaused(!logsPaused)}
            className={`flex cursor-pointer items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs font-medium transition ${
              logsPaused
                ? "border-amber-500/40 bg-amber-500/15 text-amber-700 dark:text-amber-300"
                : "border-zinc-300 bg-white text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
            }`}
          >
            {logsPaused ? (
              <>
                <Play className="h-3.5 w-3.5" /> Resume
              </>
            ) : (
              <>
                <Pause className="h-3.5 w-3.5" /> Pause
              </>
            )}
          </button>

          <button
            type="button"
            onClick={() => void clearLogs()}
            className="flex cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-300 bg-white px-3 py-1.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <Trash2 className="h-3.5 w-3.5" />
            Clear
          </button>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto p-6">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-zinc-300 py-16 text-center dark:border-zinc-800">
            <p className="text-sm font-medium text-zinc-700 dark:text-zinc-300">
              No requests logged yet
            </p>
            <p className="mt-1 text-xs text-zinc-500">
              Start the proxy server and visit one of your local HTTPS domains
              to see live traffic here.
            </p>
          </div>
        ) : (
          <div className="overflow-hidden rounded-xl border border-zinc-200 bg-white dark:border-zinc-800/80 dark:bg-zinc-900/50">
            <table className="w-full text-left font-mono text-xs">
              <thead>
                <tr className="border-b border-zinc-200 bg-zinc-50/80 font-sans text-[11px] font-semibold tracking-wider text-zinc-500 uppercase dark:border-zinc-800 dark:bg-zinc-900/80 dark:text-zinc-400">
                  <th className="px-3.5 py-2.5">Time</th>
                  <th className="px-3.5 py-2.5">Status</th>
                  <th className="px-3.5 py-2.5">Method</th>
                  <th className="px-3.5 py-2.5">Host &amp; Path</th>
                  <th className="px-3.5 py-2.5">Upstream</th>
                  <th className="px-3.5 py-2.5 text-right">Latency</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-200/70 dark:divide-zinc-800/60">
                {filtered.map((entry) => {
                  const time = new Date(entry.timestamp).toLocaleTimeString();
                  return (
                    <tr
                      key={entry.id}
                      className="hover:bg-zinc-50 dark:hover:bg-zinc-800/40"
                    >
                      <td className="whitespace-nowrap px-3.5 py-2 text-zinc-500">
                        {time}
                      </td>
                      <td className="whitespace-nowrap px-3.5 py-2">
                        <span
                          className={`inline-block rounded px-1.5 py-0.5 text-[11px] font-semibold ${statusClass(entry.status)}`}
                        >
                          {entry.status}
                        </span>
                      </td>
                      <td className="whitespace-nowrap px-3.5 py-2 font-semibold text-zinc-800 dark:text-zinc-200">
                        {entry.method}
                      </td>
                      <td className="max-w-md truncate px-3.5 py-2 text-zinc-700 dark:text-zinc-300">
                        <span className="text-emerald-600 dark:text-emerald-400">
                          {entry.host}
                        </span>
                        <span className="text-zinc-500">{entry.path}</span>
                        {entry.error && (
                          <div className="mt-0.5 font-sans text-[11px] text-rose-500">
                            {entry.error}
                          </div>
                        )}
                      </td>
                      <td className="whitespace-nowrap px-3.5 py-2 text-zinc-500">
                        {entry.upstream}
                      </td>
                      <td className="whitespace-nowrap px-3.5 py-2 text-right text-zinc-500">
                        {entry.durationMs} ms
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
