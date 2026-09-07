import { createSignal, createResource, Show, For, Switch, Match } from "solid-js";
import { useLocation, useNavigate } from "@solidjs/router";
import { propfind, uploadFile, api, type FileEntry } from "../lib/api";
import { Folder, File as FileIcon, Upload, Trash2, FolderPlus, ArrowLeft, Loader2, Download, Search } from "lucide-solid";

function fmtSize(b: number | null): string {
  if (b == null) return "—";
  if (b < 1024) return `${b} B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1073741824) return `${(b / 1048576).toFixed(1)} MB`;
  return `${(b / 1073741824).toFixed(2)} GB`;
}
function fmtDate(s: string | null): string {
  if (!s) return "";
  const d = new Date(s);
  return isNaN(+d) ? s : d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) + " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export default function FileBrowser() {
  const location = useLocation();
  const nav = useNavigate();
  const userRoot = location.pathname.split("/")[3] ?? ""; // /ui/files/<root>/<subpath>

  const subPath = () => {
    const parts = location.pathname.split("/").slice(4); // after /ui/files/<root>/
    const joined = parts.filter(Boolean).join("/");
    return joined ? `${userRoot}/${joined}` : userRoot;
  };

  const [entries, { refetch }] = createResource(subPath, async (p) => {
    if (!p) return [];
    const { entries } = await propfind(p);
    return entries;
  });

  const [error, setError] = createSignal<string | null>(null);
  const [uploadPct, setUploadPct] = createSignal<number | null>(null);
  const [query, setQuery] = createSignal("");
  const [showMkdir, setShowMkdir] = createSignal(false);
  const [newFolder, setNewFolder] = createSignal("");

  const go = (p: string) => {
    const rest = p.split("/").slice(1).join("/");
    nav(`/ui/files/${rest}`);
  };
  const up = () => {
    const parts = subPath().split("/").filter(Boolean);
    if (parts.length <= 1) return;
    go("/" + parts.slice(0, -1).join("/"));
  };

  const onUpload = async (e: Event) => {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    setError(null);
    for (const file of Array.from(input.files)) {
      const target = `${subPath()}/${encodeURIComponent(file.name)}`;
      try { setUploadPct(0); await uploadFile(target, file, setUploadPct); }
      catch (err) { setError(err instanceof Error ? err.message : String(err)); }
      finally { setUploadPct(null); }
    }
    input.value = "";
    refetch();
  };

  const onDelete = async (entry: FileEntry) => {
    if (!confirm(`Delete "${entry.name}"?`)) return;
    setError(null);
    try { await api.delete(entry.href.replace(/^\/users\//, "")); refetch(); }
    catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  };

  const onMkdir = async () => {
    const name = newFolder().trim();
    if (!name) return;
    setError(null);
    try {
      await api.mkdir(`${subPath()}/${encodeURIComponent(name)}`);
      setShowMkdir(false); setNewFolder(""); refetch();
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  };

  const filtered = () => {
    const q = query().toLowerCase();
    const list = entries() ?? [];
    return q ? list.filter((e) => e.name.toLowerCase().includes(q)) : list;
  };

  const rowClass = "group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] cursor-pointer border-b border-[var(--border-default)]/40";

  return (
    <div class="h-full flex flex-col">
      <div class="glass flex items-center gap-2 px-4 py-2.5 shrink-0">
        <Show when={subPath().split("/").filter(Boolean).length > 1}>
          <button onClick={up} class="p-2 rounded-lg hover:bg-[var(--bg-raised)] shrink-0" aria-label="Up">
            <ArrowLeft size={17} />
          </button>
        </Show>
        <span class="font-mono text-sm text-[var(--text-secondary)] truncate flex-1" title={subPath()}>
          /{subPath().split("/").slice(1).join("/")}
        </span>
        <div class="relative w-56 hidden md:block">
          <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-tertiary)]" />
          <input
            value={query()} onInput={(e) => setQuery(e.currentTarget.value)}
            placeholder="Filter…"
            class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg pl-8 pr-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]"
          />
        </div>
        <label class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] text-white cursor-pointer hover:bg-[var(--accent-hover)] transition-colors shrink-0">
          <Upload size={15} /> Upload
          <input type="file" multiple class="hidden" onChange={onUpload} />
        </label>
        <button onClick={() => setShowMkdir(!showMkdir())} class="p-2 rounded-lg hover:bg-[var(--bg-raised)] shrink-0" aria-label="New folder">
          <FolderPlus size={16} />
        </button>
      </div>

      <Show when={showMkdir()}>
        <div class="flex items-center gap-2 px-4 py-2 glass shrink-0">
          <input
            value={newFolder()} onInput={(e) => setNewFolder(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && onMkdir()}
            placeholder="Folder name" class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]"
            autoFocus
          />
          <button onClick={onMkdir} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Create</button>
          <button onClick={() => setShowMkdir(false)} class="px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]">Cancel</button>
        </div>
      </Show>

      <Show when={uploadPct() !== null}>
        <div class="px-4 py-2 shrink-0 bg-[var(--accent-subtle)]">
          <div class="text-xs text-[var(--text-secondary)] mb-1">Uploading… {uploadPct()}%</div>
          <div class="h-1 rounded-full bg-[var(--bg-raised)] overflow-hidden">
            <div class="h-full bg-[var(--accent)] transition-all" style={{ width: `${uploadPct()}%` }} />
          </div>
        </div>
      </Show>

      <Show when={error()}>
        <div class="px-4 py-2 text-sm text-[var(--danger)] bg-[var(--danger-subtle)] shrink-0">{error()}</div>
      </Show>

      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={entries.loading}>
            <div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div>
          </Match>
          <Match when={entries.error}>
            <div class="text-center pt-16 text-sm text-[var(--danger)]">{String(entries.error)}</div>
          </Match>
          <Match when={filtered().length === 0}>
            <div class="text-center pt-16 text-sm text-[var(--text-tertiary)]">Empty</div>
          </Match>
          <Match when={true}>
            <div class="px-4 py-1 text-[11px] uppercase tracking-wider text-[var(--text-tertiary)] flex justify-between">
              <span>{filtered().length} items</span>
            </div>
            <For each={filtered()}>
              {(entry) => (
                <div
                  class={rowClass}
                  onClick={() => entry.isCollection && go(entry.href.replace(/^\/users\//, ""))}
                >
                  {entry.isCollection
                    ? <Folder size={18} class="text-[var(--accent)] shrink-0" />
                    : <FileIcon size={17} class="text-[var(--text-tertiary)] shrink-0" />}
                  <span class="flex-1 text-sm truncate">{entry.name}</span>
                  <span class="text-xs text-[var(--text-tertiary)] tabular-nums hidden sm:block w-28 text-right shrink-0">{fmtDate(entry.modifiedAt)}</span>
                  <span class="text-xs text-[var(--text-tertiary)] tabular-nums hidden sm:block w-20 text-right shrink-0">{entry.isCollection ? "—" : fmtSize(entry.size)}</span>
                  <Show when={!entry.isCollection}>
                    <a
                      href={api.downloadUrl(entry.href.replace(/^\/users\//, ""))}
                      class="p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--bg-raised)] transition-all"
                      download aria-label="Download" onClick={(e) => e.stopPropagation()}
                    ><Download size={14} /></a>
                  </Show>
                  <button
                    class="p-1.5 rounded opacity-0 group-hover:opacity-100 text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-all shrink-0"
                    aria-label="Delete" onClick={(e) => { e.stopPropagation(); onDelete(entry); }}
                  ><Trash2 size={14} /></button>
                </div>
              )}
            </For>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
