import { createSignal, createResource, Show, For, Switch, Match, createContext, useContext } from "solid-js";
import { useLocation, useNavigate } from "@solidjs/router";
import { propfind, uploadFile, api, type FileEntry } from "../lib/api";
import {
  Folder, File as FileIcon, Upload, Trash2, FolderPlus, ArrowLeft, Loader2,
  Download, Search, Pencil, Share2, Eye, MoreVertical, X, CheckSquare, Square,
} from "lucide-solid";
import { ShareDialog } from "./ShareDialog";
import { PreviewModal } from "./PreviewModal";

export const ToastContext = createContext<{ toast: (msg: string, err?: boolean) => void }>();
export const useToast = () => useContext(ToastContext);

export function fmtSize(b: number | null): string {
  if (b == null) return "—";
  if (b < 1024) return `${b} B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1073741824) return `${(b / 1048576).toFixed(1)} MB`;
  return `${(b / 1073741824).toFixed(2)} GB`;
}
export function fmtDate(s: string | null): string {
  if (!s) return "";
  const d = new Date(s);
  return isNaN(+d) ? s : d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) + " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export default function FileBrowser() {
  const location = useLocation();
  const nav = useNavigate();
  const parts = () => location.pathname.split("/").slice(3).filter(Boolean); // after /ui/
  const subPath = () => parts().join("/");

  const [entries, { refetch }] = createResource(subPath, async (p) => {
    if (!p) return [];
    return propfind(p);
  });

  const [error, setError] = createSignal<string | null>(null);
  const [toastMsg, setToastMsg] = createSignal<{ msg: string; err: boolean } | null>(null);
  const [uploadPct, setUploadPct] = createSignal<number | null>(null);
  const [query, setQuery] = createSignal("");
  const [showMkdir, setShowMkdir] = createSignal(false);
  const [newFolder, setNewFolder] = createSignal("");
  const [renaming, setRenaming] = createSignal<FileEntry | null>(null);
  const [renameValue, setRenameValue] = createSignal("");
  const [selected, setSelected] = createSignal<Set<string>>(new Set<string>());
  const [menuFor, setMenuFor] = createSignal<string | null>(null);
  const [shareFor, setShareFor] = createSignal<FileEntry | null>(null);
  const [previewFor, setPreviewFor] = createSignal<FileEntry | null>(null);
  const [dragOver, setDragOver] = createSignal(false);
  const [lastIndex, setLastIndex] = createSignal(-1);

  const toast = (msg: string, err = false) => {
    setToastMsg({ msg, err });
    setTimeout(() => setToastMsg(null), 3500);
  };

  const davPath = (entry: FileEntry) => entry.href.replace(/^\/users\//, "");
  const go = (p: string) => nav(`/ui/${p.replace(/^\/+/, "")}`);

  const crumbs = () => {
    const segs = parts();
    const out: { name: string; path: string }[] = [{ name: "Home", path: segs[0] ?? "" }];
    let acc = segs[0] ?? "";
    for (const seg of segs.slice(1)) {
      acc += `/${seg}`;
      out.push({ name: seg, path: acc });
    }
    return out;
  };

  const toggleSelect = (e: MouseEvent, entry: FileEntry, idx: number) => {
    e.stopPropagation();
    if (e.shiftKey && lastIndex() >= 0) {
      const list = filtered();
      const [a, b] = [lastIndex(), idx].sort((x, y) => x - y);
      const next = new Set<string>(selected());
      for (let i = a; i <= b; i++) next.add(list[i].href);
      setSelected(next);
    } else {
      const next = new Set<string>(selected());
      next.has(entry.href) ? next.delete(entry.href) : next.add(entry.href);
      setSelected(next);
    }
    setLastIndex(idx);
  };
  const clearSelection = () => setSelected(new Set<string>());

  const onUpload = async (files: FileList | File[], sub: string) => {
    setError(null);
    for (const file of Array.from(files)) {
      const target = `${sub}/${encodeURIComponent(file.name)}`;
      try { setUploadPct(0); await uploadFile(target, file, setUploadPct); }
      catch (err) { setError(err instanceof Error ? err.message : String(err)); }
      finally { setUploadPct(null); }
    }
    refetch();
  };

  const onMkdir = async () => {
    const name = newFolder().trim();
    if (!name) return;
    try {
      await api.mkdir(`${subPath()}/${encodeURIComponent(name)}`);
      setShowMkdir(false); setNewFolder(""); refetch();
    } catch (err) { setError(err instanceof Error ? err.message : String(err)); }
  };

  // Trash API takes the FULL virtual path (/users/{sub}/...), not the
  // user-root-relative form — entry.href is exactly that.
  const trashOne = async (entry: FileEntry) => {
    try { await api.trashPath(entry.href); toast(`Trashed ${entry.name}`); refetch(); clearSelection(); }
    catch (err) { toast(err instanceof Error ? err.message : String(err), true); }
  };

  const trashSelected = async () => {
    const list = entries() ?? [];
    const targets = list.filter((e) => selected().has(e.href));
    if (!targets.length) return;
    try {
      for (const t of targets) await api.trashPath(t.href);
      toast(`Trashed ${targets.length} item(s)`); refetch(); clearSelection();
    } catch (err) { toast(err instanceof Error ? err.message : String(err), true); }
  };

  const onRename = async () => {
    const entry = renaming();
    if (!entry) return;
    const newName = renameValue().trim();
    if (!newName || newName === entry.name) { setRenaming(null); return; }
    const from = davPath(entry);
    const to = from.split("/").slice(0, -1).concat(encodeURIComponent(newName)).join("/");
    try { await api.move(from, to); toast("Renamed"); setRenaming(null); refetch(); }
    catch (err) { toast(err instanceof Error ? err.message : String(err), true); }
  };

  const filtered = () => {
    const q = query().toLowerCase();
    const list = entries() ?? [];
    return q ? list.filter((e) => e.name.toLowerCase().includes(q)) : list;
  };

  const rowClass = "group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] cursor-pointer border-b border-[var(--border-default)]/40";
  const iconBtn = "p-1.5 rounded opacity-0 group-hover:opacity-100 hover:bg-[var(--bg-raised)] transition-all shrink-0";

  return (
    <div
      class="h-full flex flex-col"
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => { e.preventDefault(); setDragOver(false); if (e.dataTransfer?.files.length) onUpload(e.dataTransfer.files, subPath()); }}
      onClick={() => { setMenuFor(null); clearSelection(); }}
    >
      {/* Breadcrumb toolbar */}
      <div class="glass flex items-center gap-2 px-4 py-2.5 shrink-0 flex-wrap">
        <Show when={parts().length > 1}>
          <button onClick={(e) => { e.stopPropagation(); go(parts().slice(0, -1).join("/")); }} class="p-2 rounded-lg hover:bg-[var(--bg-raised)] shrink-0" aria-label="Up">
            <ArrowLeft size={17} />
          </button>
        </Show>
        <nav class="flex items-center gap-1 text-sm min-w-0 flex-1 overflow-x-auto">
          <For each={crumbs()}>
            {(c, i) => (
              <>
                <Show when={i() > 0}><span class="text-[var(--text-tertiary)]">/</span></Show>
                <button
                  class={i() === crumbs().length - 1 ? "font-medium truncate" : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] truncate"}
                  onClick={(e) => { e.stopPropagation(); go(c.path); }}
                >{c.name}</button>
              </>
            )}
          </For>
        </nav>
        <div class="relative w-56 hidden md:block">
          <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-tertiary)]" />
          <input
            value={query()} onInput={(e) => setQuery(e.currentTarget.value)}
            placeholder="Filter…" onClick={(e) => e.stopPropagation()}
            class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg pl-8 pr-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]"
          />
        </div>
        <label class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] text-white cursor-pointer hover:bg-[var(--accent-hover)] transition-colors shrink-0">
          <Upload size={15} /> Upload
          <input type="file" multiple class="hidden" onChange={(e) => e.currentTarget.files && onUpload(e.currentTarget.files, subPath())} />
        </label>
        <button onClick={(e) => { e.stopPropagation(); setShowMkdir(!showMkdir()); }} class="p-2 rounded-lg hover:bg-[var(--bg-raised)] shrink-0" aria-label="New folder">
          <FolderPlus size={16} />
        </button>
      </div>

      {/* New folder input */}
      <Show when={showMkdir()}>
        <div class="flex items-center gap-2 px-4 py-2 glass shrink-0">
          <input
            value={newFolder()} onInput={(e) => setNewFolder(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && onMkdir()}
            placeholder="Folder name" class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]"
            autofocus
          />
          <button onClick={onMkdir} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Create</button>
          <button onClick={(e) => { e.stopPropagation(); setShowMkdir(false); }} class="px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]">Cancel</button>
        </div>
      </Show>

      {/* Selection action bar */}
      <Show when={selected().size > 0}>
        <div class="glass flex items-center gap-3 px-4 py-2 shrink-0 bg-[var(--accent-subtle)]">
          <span class="text-sm">{selected().size} selected</span>
          <button onClick={(e) => { e.stopPropagation(); trashSelected(); }} class="flex items-center gap-1 text-sm text-[var(--danger)] hover:underline">
            <Trash2 size={14} /> Trash
          </button>
          <button onClick={(e) => { e.stopPropagation(); clearSelection(); }} class="text-sm text-[var(--text-secondary)] hover:underline">Clear</button>
        </div>
      </Show>

      {/* Upload progress */}
      <Show when={uploadPct() !== null}>
        <div class="px-4 py-2 shrink-0 bg-[var(--accent-subtle)]">
          <div class="text-xs text-[var(--text-secondary)] mb-1">Uploading… {uploadPct()}%</div>
          <div class="h-1 rounded-full bg-[var(--bg-raised)] overflow-hidden">
            <div class="h-full bg-[var(--accent)] transition-all" style={{ width: `${uploadPct()}%` }} />
          </div>
        </div>
      </Show>

      <Show when={error() || toastMsg()}>
        <div class={`px-4 py-2 text-sm shrink-0 ${toastMsg()?.err || error() ? "text-[var(--danger)] bg-[var(--danger-subtle)]" : "text-[var(--ok)] bg-[var(--accent-subtle)]"}`}>
          {error() ?? toastMsg()!.msg}
        </div>
      </Show>

      {/* Drop overlay */}
      <Show when={dragOver()}>
        <div class="fixed inset-0 z-40 bg-[var(--accent-subtle)] backdrop-blur-sm flex items-center justify-center pointer-events-none">
          <div class="glass rounded-2xl px-8 py-6 text-lg">Drop files to upload</div>
        </div>
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
            <div class="text-center pt-16 text-sm text-[var(--text-tertiary)]">Empty — drop files here or use Upload</div>
          </Match>
          <Match when={true}>
            <For each={filtered()}>
              {(entry, idx) => (
                <div
                  class={`${rowClass} ${selected().has(entry.href) ? "bg-[var(--accent-subtle)]" : ""}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    if (e.ctrlKey || e.metaKey || e.shiftKey) { toggleSelect(e, entry, idx()); return; }
                    entry.isCollection ? go(davPath(entry)) : setPreviewFor(entry);
                  }}
                >
                  <button onClick={(e) => toggleSelect(e, entry, idx())} class="shrink-0 text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" aria-label="Select">
                    <Show when={selected().has(entry.href)} fallback={<Square size={15} />}><CheckSquare size={15} class="text-[var(--accent)]" /></Show>
                  </button>
                  {entry.isCollection
                    ? <Folder size={18} class="text-[var(--accent)] shrink-0" />
                    : <FileIcon size={17} class="text-[var(--text-tertiary)] shrink-0" />}
                  <Show when={renaming()?.href === entry.href} fallback={<span class="flex-1 text-sm truncate">{entry.name}</span>}>
                    <input
                      value={renameValue()} onInput={(e) => setRenameValue(e.currentTarget.value)}
                      onKeyDown={(e) => { if (e.key === "Enter") onRename(); if (e.key === "Escape") setRenaming(null); }}
                      onClick={(e) => e.stopPropagation()}
                      class="flex-1 bg-[var(--bg-surface)] border border-[var(--accent)] rounded px-2 py-0.5 text-sm outline-none" autofocus
                    />
                  </Show>
                  <span class="text-xs text-[var(--text-tertiary)] tabular-nums hidden sm:block w-32 text-right shrink-0">{fmtDate(entry.modifiedAt)}</span>
                  <span class="text-xs text-[var(--text-tertiary)] tabular-nums hidden sm:block w-20 text-right shrink-0">{entry.isCollection ? "—" : fmtSize(entry.size)}</span>
                  <Show when={!entry.isCollection}>
                    <button class={iconBtn} aria-label="Preview" onClick={(e) => { e.stopPropagation(); setPreviewFor(entry); }}><Eye size={14} /></button>
                    <a href={api.downloadUrl(davPath(entry))} class={iconBtn} download="" aria-label="Download" onClick={(e) => e.stopPropagation()}><Download size={14} /></a>
                  </Show>
                  <button class={iconBtn} aria-label="Share" onClick={(e) => { e.stopPropagation(); setShareFor(entry); }}><Share2 size={14} /></button>
                  <button class={iconBtn} aria-label="Rename" onClick={(e) => { e.stopPropagation(); setRenaming(entry); setRenameValue(entry.name); }}><Pencil size={14} /></button>
                  <button class="p-1.5 rounded opacity-0 group-hover:opacity-100 text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-all shrink-0" aria-label="Trash" onClick={(e) => { e.stopPropagation(); trashOne(entry); }}><Trash2 size={14} /></button>
                </div>
              )}
            </For>
          </Match>
        </Switch>
      </div>

      <Show when={shareFor()}>
        <ShareDialog entry={shareFor()!} onClose={() => setShareFor(null)} />
      </Show>
      <Show when={previewFor()}>
        <PreviewModal entry={previewFor()!} onClose={() => setPreviewFor(null)} />
      </Show>
    </div>
  );
}
