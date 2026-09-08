import { createSignal, createResource, For, Show, Switch, Match, onCleanup } from "solid-js";
import { api, type Board } from "../lib/api";
import { Plus, Trash2, Loader2, Pencil, Save } from "lucide-solid";

export default function WhiteboardPage() {
  const [boards, { refetch }] = createResource(async () => (await api.listBoards()).whiteboards as Board[]);
  const [active, setActive] = createSignal<Board | null>(null);
  const [content, setContent] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [dirty, setDirty] = createSignal(false);

  const open = async (b: Board) => {
    try {
      const data = await api.getBoard(b.id);
      setActive(b); setContent(typeof data === "string" ? data : JSON.stringify(data, null, 2));
      setDirty(false); setError(null);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };

  const save = async () => {
    const b = active();
    if (!b) return;
    setBusy(true);
    try { await api.saveBoard(b.id, content()); setDirty(false); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const create = async () => {
    const name = prompt("Board name:");
    if (!name?.trim()) return;
    setBusy(true); setError(null);
    try {
      const b = await api.createBoard(name.trim());
      refetch(); if (b) open(b);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  return (
    <div class="h-full flex">
      <div class="w-64 border-r border-[var(--border-default)] flex flex-col shrink-0">
        <div class="p-3 shrink-0">
          <button onClick={create} disabled={busy()}
            class="w-full flex items-center justify-center gap-1.5 px-3 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">
            <Plus size={14} /> New board
          </button>
        </div>
        <div class="flex-1 overflow-y-auto">
          <Switch>
            <Match when={boards.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
            <Match when={(boards() ?? []).length === 0}><p class="text-xs text-[var(--text-tertiary)] text-center pt-10">No boards</p></Match>
            <Match when={true}>
              <For each={boards() ?? []}>
                {(b) => (
                  <button onClick={() => open(b)}
                    class={`w-full text-left px-4 py-2.5 border-b border-[var(--border-default)]/40 hover:bg-[var(--bg-raised)] ${active()?.id === b.id ? "bg-[var(--accent-subtle)]" : ""}`}>
                    <div class="text-sm truncate flex items-center gap-2"><Pencil size={13} class="text-[var(--accent)] shrink-0" />{b.name}</div>
                    <div class="text-xs text-[var(--text-tertiary)]">{new Date(b.updated_at).toLocaleDateString()}</div>
                  </button>
                )}
              </For>
            </Match>
          </Switch>
        </div>
      </div>

      <div class="flex-1 flex flex-col min-w-0">
        <Switch>
          <Match when={active()}>
            <div class="flex items-center gap-2 px-4 py-2 border-b border-[var(--border-default)] shrink-0">
              <span class="text-sm font-medium flex-1 truncate">{active()!.name}{dirty() ? " ·" : ""}</span>
              <button onClick={save} disabled={busy() || !dirty()}
                class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">
                <Save size={14} /> Save
              </button>
            </div>
            <Show when={error()}><p class="text-sm text-[var(--danger)] px-4 py-2">{error()}</p></Show>
            <textarea
              value={content()} onInput={(e) => { setContent(e.currentTarget.value); setDirty(true); }}
              onKeyDown={(e) => { if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); save(); } }}
              placeholder="Board content (JSON)…"
              class="flex-1 bg-transparent p-4 text-sm font-mono outline-none resize-none" spellcheck={false}
            />
          </Match>
          <Match when={true}>
            <div class="flex-1 flex items-center justify-center text-sm text-[var(--text-tertiary)]">Select or create a board</div>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
