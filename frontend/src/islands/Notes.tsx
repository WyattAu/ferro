import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type Note } from "../lib/api";
import { Plus, Trash2, Loader2, FileText, Save, X } from "lucide-solid";

export default function NotesPage() {
  const [notes, { refetch }] = createResource(async () => (await api.listNotes()).notes as Note[]);
  const [selected, setSelected] = createSignal<Note | null>(null);
  const [draft, setDraft] = createSignal<{ title: string; content: string } | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [query, setQuery] = createSignal("");

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setError(null);
    try { await fn(); refetch(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const open = (n: Note) => { setSelected(n); setDraft({ title: n.title, content: n.content }); };
  const close = () => { setSelected(null); setDraft(null); };

  const create = async () => {
    let n = 1;
    let title = "Untitled";
    const existing = new Set((notes() ?? []).map((x) => x.title));
    while (existing.has(title)) { n++; title = `Untitled ${n}`; }
    await run(async () => {
      const created = await api.createNote({ title });
      if (created) open(created);
    });
  };

  const save = async () => {
    const n = selected(); const d = draft();
    if (!n || !d) return;
    await run(() => api.updateNote(n.id, { title: d.title, content: d.content }));
    close();
  };

  const filtered = () => {
    const list = notes() ?? [];
    const q = query().toLowerCase();
    return q ? list.filter((n) => n.title.toLowerCase().includes(q) || n.content.toLowerCase().includes(q)) : list;
  };

  return (
    <div class="h-full flex">
      {/* List */}
      <div class="w-72 border-r border-[var(--border-default)] flex flex-col shrink-0">
        <div class="p-3 flex gap-2 shrink-0">
          <input value={query()} onInput={(e) => setQuery(e.currentTarget.value)} placeholder="Search…"
            class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <button onClick={create} disabled={busy()} class="p-2 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white" aria-label="New note"><Plus size={15} /></button>
        </div>
        <div class="flex-1 overflow-y-auto">
          <Switch>
            <Match when={notes.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
            <Match when={filtered().length === 0}><p class="text-xs text-[var(--text-tertiary)] text-center pt-10">No notes</p></Match>
            <Match when={true}>
              <For each={filtered()}>
                {(n) => (
                  <button
                    onClick={() => open(n)}
                    class={`w-full text-left px-4 py-2.5 border-b border-[var(--border-default)]/40 hover:bg-[var(--bg-raised)] ${selected()?.id === n.id ? "bg-[var(--accent-subtle)]" : ""}`}
                  >
                    <div class="text-sm truncate flex items-center gap-2"><FileText size={13} class="text-[var(--accent)] shrink-0" />{n.title}</div>
                    <div class="text-xs text-[var(--text-tertiary)] truncate">{n.content.slice(0, 60) || "Empty"}</div>
                  </button>
                )}
              </For>
            </Match>
          </Switch>
        </div>
      </div>

      {/* Editor */}
      <div class="flex-1 flex flex-col min-w-0">
        <Switch>
          <Match when={selected() && draft()}>
            <div class="flex items-center gap-2 px-4 py-2 border-b border-[var(--border-default)] shrink-0">
              <input value={draft()!.title} onInput={(e) => setDraft({ ...draft()!, title: e.currentTarget.value })}
                class="flex-1 bg-transparent text-sm font-medium outline-none" />
              <button onClick={save} disabled={busy()} class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">
                <Save size={14} /> Save
              </button>
              <button onClick={close} class="p-2 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Close"><X size={15} /></button>
              <button onClick={async () => { if (confirm("Delete note?")) { await run(() => api.deleteNote(selected()!.id)); close(); } }}
                class="p-2 rounded-lg text-[var(--danger)] hover:bg-[var(--danger-subtle)]" aria-label="Delete"><Trash2 size={15} /></button>
            </div>
            <textarea
              value={draft()!.content} onInput={(e) => setDraft({ ...draft()!, content: e.currentTarget.value })}
              placeholder="Write…"
              class="flex-1 bg-transparent p-4 text-sm font-mono outline-none resize-none"
            />
          </Match>
          <Match when={true}>
            <div class="flex-1 flex items-center justify-center text-sm text-[var(--text-tertiary)]">
              <Show when={error()} fallback={<span>Select or create a note</span>}>{error()}</Show>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
