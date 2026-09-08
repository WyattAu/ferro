import { createSignal, createResource, Show, For } from "solid-js";
import { api, type TrashedEntry } from "../lib/api";
import { Undo2, Trash2, Loader2, Flame } from "lucide-solid";

function fmtSize(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1073741824) return `${(b / 1048576).toFixed(1)} MB`;
  return `${(b / 1073741824).toFixed(2)} GB`;
}

export default function TrashPage() {
  const [entries, { refetch }] = createResource(async () => (await api.listTrash()).entries ?? []);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setError(null);
    try { await fn(); refetch(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <div class="flex items-center justify-between mb-4 shrink-0">
        <h1 class="text-xl font-semibold">Trash</h1>
        <Show when={(entries() ?? []).length > 0}>
          <button
            onClick={() => confirm("Permanently delete ALL trashed items?") && run(() => api.emptyTrash())}
            class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg text-[var(--danger)] border border-[var(--danger)]/40 hover:bg-[var(--danger-subtle)]"
          ><Flame size={14} /> Empty trash</button>
        </Show>
      </div>
      <Show when={error()}><p class="text-sm text-[var(--danger)] mb-3">{error()}</p></Show>
      <div class="flex-1 overflow-y-auto min-h-0">
        <Show when={entries.loading} fallback={
          <Show when={(entries() ?? []).length === 0}>
            <p class="text-sm text-[var(--text-tertiary)] text-center pt-16">Trash is empty</p>
          </Show>
        }>
          <div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div>
        </Show>
        <For each={entries() ?? []}>
          {(item: TrashedEntry) => (
            <div class="group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] border-b border-[var(--border-default)]/40 text-sm">
              <Trash2 size={15} class="text-[var(--text-tertiary)] shrink-0" />
              <span class="flex-1 truncate font-mono text-xs">{item.original_path}</span>
              <span class="text-xs text-[var(--text-tertiary)] hidden sm:block">{new Date(item.deleted_at).toLocaleString()}</span>
              <span class="text-xs text-[var(--text-tertiary)] w-16 text-right">{fmtSize(item.size)}</span>
              <button disabled={busy()} onClick={() => run(() => api.restoreTrash(item.original_path))}
                class="p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--bg-raised)]" aria-label="Restore"><Undo2 size={14} /></button>
              <button disabled={busy()} onClick={() => confirm(`Permanently delete ${item.original_path}?`) && run(() => api.purgeTrash(item.original_path))}
                class="p-1.5 rounded opacity-0 group-hover:opacity-100 text-[var(--danger)] hover:bg-[var(--danger-subtle)]" aria-label="Purge"><Trash2 size={14} /></button>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
