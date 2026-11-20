import { CheckCircle2, Info, X, XCircle } from "lucide-react";
import { useAppStore } from "../store/useAppStore";

export function ToastContainer() {
  const { toasts, dismissToast } = useAppStore();

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed right-5 bottom-5 z-50 flex flex-col gap-2">
      {toasts.map((t) => {
        const Icon =
          t.kind === "success"
            ? CheckCircle2
            : t.kind === "error"
              ? XCircle
              : Info;
        const accent =
          t.kind === "success"
            ? "border-emerald-500/30 bg-zinc-900 text-emerald-300"
            : t.kind === "error"
              ? "border-rose-500/30 bg-zinc-900 text-rose-300"
              : "border-zinc-700 bg-zinc-900 text-zinc-200";

        return (
          <div
            key={t.id}
            className={`pointer-events-auto flex max-w-sm items-center gap-2.5 rounded-xl border px-3.5 py-2.5 text-xs shadow-xl ${accent}`}
          >
            <Icon className="h-4 w-4 shrink-0" />
            <span className="flex-1 leading-snug">{t.text}</span>
            <button
              type="button"
              onClick={() => dismissToast(t.id)}
              className="cursor-pointer rounded p-0.5 opacity-60 hover:opacity-100"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
