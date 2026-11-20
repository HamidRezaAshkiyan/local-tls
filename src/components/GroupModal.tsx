import { Trash2, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useAppStore } from "../store/useAppStore";

const PRESET_COLORS = [
  "#10b981",
  "#3b82f6",
  "#8b5cf6",
  "#f59e0b",
  "#ec4899",
  "#06b6d4",
  "#ef4444",
];

interface GroupModalProps {
  onClose: () => void;
}

export function GroupModal({ onClose }: GroupModalProps) {
  const { config, addGroup, deleteGroup } = useAppStore();
  const groups = config?.groups ?? [];

  const [name, setName] = useState("");
  const [color, setColor] = useState(PRESET_COLORS[0]);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleCreate = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await addGroup(name, color);
      setName("");
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to create group.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-xs">
      <div className="w-full max-w-md rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xl dark:border-zinc-800 dark:bg-zinc-900">
        <div className="mb-5 flex items-center justify-between">
          <h2 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">
            Manage Proxy Groups
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded-lg p-1 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleCreate} className="mb-5 space-y-3">
          <div className="flex gap-2">
            <input
              type="text"
              required
              placeholder="New group name (e.g. Microservices)"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="flex-1 rounded-lg border border-zinc-300 bg-zinc-50 px-3 py-2 text-sm text-zinc-900 outline-none focus:border-emerald-500 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100"
            />
            <button
              type="submit"
              disabled={submitting}
              className="cursor-pointer rounded-lg bg-emerald-600 px-3.5 py-2 text-xs font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
            >
              Add
            </button>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-zinc-500">Color:</span>
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                type="button"
                onClick={() => setColor(c)}
                className={`h-5 w-5 cursor-pointer rounded-full transition ${
                  color === c ? "scale-110 ring-2 ring-zinc-400 ring-offset-2 dark:ring-offset-zinc-900" : ""
                }`}
                style={{ backgroundColor: c }}
              />
            ))}
          </div>
          {error && (
            <div className="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-600 dark:text-rose-400">
              {error}
            </div>
          )}
        </form>

        <div className="max-h-56 space-y-1.5 overflow-y-auto">
          {groups.map((g) => {
            const count =
              config?.proxies.filter((p) => p.groupId === g.id).length ?? 0;
            return (
              <div
                key={g.id}
                className="flex items-center justify-between rounded-lg border border-zinc-200 bg-zinc-50 px-3 py-2 dark:border-zinc-800 dark:bg-zinc-950/60"
              >
                <div className="flex items-center gap-2.5">
                  <span
                    className="h-2.5 w-2.5 rounded-full"
                    style={{ backgroundColor: g.color }}
                  />
                  <span className="text-sm font-medium text-zinc-800 dark:text-zinc-200">
                    {g.name}
                  </span>
                  <span className="text-xs text-zinc-500">({count})</span>
                </div>
                {g.id !== "default" && (
                  <button
                    type="button"
                    onClick={() => void deleteGroup(g.id)}
                    title="Delete group"
                    className="cursor-pointer rounded p-1 text-zinc-400 hover:bg-rose-500/15 hover:text-rose-500"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
