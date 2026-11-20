import { X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useAppStore } from "../store/useAppStore";
import type { ProxyEntry } from "../types";

interface ProxyModalProps {
  existing?: ProxyEntry | null;
  defaultGroupId: string;
  onClose: () => void;
}

export function ProxyModal({
  existing,
  defaultGroupId,
  onClose,
}: ProxyModalProps) {
  const { config, addProxy, updateProxy } = useAppStore();
  const groups = config?.groups ?? [];

  const [hostname, setHostname] = useState(existing?.hostname ?? "");
  const [targetHost, setTargetHost] = useState(
    existing?.targetHost ?? "127.0.0.1",
  );
  const [targetPort, setTargetPort] = useState(
    existing ? String(existing.targetPort) : "3000",
  );
  const [groupId, setGroupId] = useState(
    existing?.groupId ??
      (defaultGroupId !== "all" ? defaultGroupId : groups[0]?.id ?? "default"),
  );
  const [enabled, setEnabled] = useState(existing?.enabled ?? true);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);

    const parsedPort = Number(targetPort);
    if (!Number.isInteger(parsedPort) || parsedPort < 1 || parsedPort > 65535) {
      setError("Target port must be an integer between 1 and 65535.");
      return;
    }

    setSubmitting(true);
    try {
      if (existing) {
        await updateProxy({
          id: existing.id,
          hostname,
          targetHost,
          targetPort: parsedPort,
          groupId,
          enabled,
        });
      } else {
        await addProxy({
          hostname,
          targetHost,
          targetPort: parsedPort,
          groupId,
          enabled,
        });
      }
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to save proxy.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs">
      <div className="w-full max-w-md rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xl dark:border-zinc-800 dark:bg-zinc-900">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">
            {existing ? "Edit SSL Proxy" : "Add SSL Proxy"}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded-lg p-1 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
              Local Domain Name
            </label>
            <div className="flex items-center overflow-hidden rounded-lg border border-zinc-300 bg-zinc-50 focus-within:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950">
              <span className="border-r border-zinc-200 px-3 py-2 font-mono text-xs text-zinc-500 dark:border-zinc-800">
                https://
              </span>
              <input
                type="text"
                required
                autoFocus
                placeholder="api.myapp.test"
                value={hostname}
                onChange={(e) => setHostname(e.target.value)}
                className="w-full bg-transparent px-3 py-2 font-mono text-sm text-zinc-900 outline-none dark:text-zinc-100"
              />
            </div>
            <p className="mt-1 text-[11px] text-zinc-500">
              Tip: Use <code>.test</code> or <code>.local.test</code> (avoids
              macOS Bonjour <code>.local</code> delays).
            </p>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div className="col-span-2">
              <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                Target Host
              </label>
              <input
                type="text"
                required
                placeholder="127.0.0.1"
                value={targetHost}
                onChange={(e) => setTargetHost(e.target.value)}
                className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 font-mono text-sm text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
              />
            </div>
            <div>
              <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
                Target Port
              </label>
              <input
                type="number"
                required
                min={1}
                max={65535}
                value={targetPort}
                onChange={(e) => setTargetPort(e.target.value)}
                className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 font-mono text-sm text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
              />
            </div>
          </div>

          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-700 dark:text-zinc-300">
              Group
            </label>
            <select
              value={groupId}
              onChange={(e) => setGroupId(e.target.value)}
              className="w-full rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 text-sm text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
            >
              {groups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </div>

          <label className="flex cursor-pointer items-center gap-2.5 text-sm text-zinc-700 dark:text-zinc-300">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="h-4 w-4 rounded border-zinc-300 accent-emerald-600"
            />
            Enable this proxy immediately
          </label>

          {error && (
            <div className="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-600 dark:text-rose-400">
              {error}
            </div>
          )}

          <div className="flex justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="cursor-pointer rounded-lg border border-zinc-300 px-4 py-2 text-xs font-medium text-zinc-700 hover:bg-zinc-100 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="cursor-pointer rounded-lg bg-emerald-600 px-4 py-2 text-xs font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
            >
              {submitting
                ? "Saving..."
                : existing
                  ? "Save Changes"
                  : "Create Proxy"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
