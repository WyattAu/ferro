import { createSignal, For, Show, onMount } from "solid-js";
import { api, type FileEntry } from "../lib/api";
import { Folder, File as FileIcon, Upload, Trash2, Plus, ArrowLeft, Loader2 } from "lucide-solid";

export default function FileBrowser() {
  const [path, setPath] = createSignal("/");
  const [entries, setEntries] = createSignal<FileEntry[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [uploading, setUploading] = createSignal(false);
  const [progress, setProgress] = createSignal(0);
  const fileInputRef = document.createElement("input");

  async function load(p: string) {
    setLoading(true); setError(null);
    try { setEntries(await api.listFiles(p)); } 
    catch (e) { setError(String(e)); } 
    finally { setLoading(false); }
  }

  onMount(() => load(path()));

  const navigate = (entry: FileEntry) => {
    if (entry.is_collection) { 
      setPath(entry.path.replace("/remote.php/dav/files/wyatt", "") || "/");
      load(path());
    }
  };

  const goUp = () => {
    const p = path().replace(/\/[^/]*\/?$/, "") || "/";
    setPath(p); load(p);
  };

  const handleUpload = async (e: Event) => {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    setUploading(true);
    for (const file of Array.from(input.files)) {
      const rel = path() === "/" ? `/${file.name}` : `${path()}/${file.name}`;
      try { await api.uploadFile(rel, file, setProgress); await load(path()); }
      catch (err) { console.error("Upload failed:", err); }
    }
    setUploading(false);
    input.value = "";
  };

  const handleDelete = async (entry: FileEntry) => {
    await api.deleteFile(entry.path.replace("/remote.php/dav/files/wyatt", ""));
    await load(path());
  };

  const formatSize = (b: number) => {
    if (b < 1024) return `${b} B`;
    if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
    if (b < 1073741824) return `${(b / 1048576).toFixed(1)} MB`;
    return `${(b / 1073741824).toFixed(1)} GB`;
  };

  return (
    <div class="flex flex-col h-full">
      {/* Toolbar */}
      <div class="flex items-center gap-2 px-4 py-2 border-b border-[var(--border-default)] bg-[var(--bg-surface)]">
        <Show when={path() !== "/"} keyed>
          <button class="p-2 rounded hover:bg-[var(--interactive-hover)] min-w-[36px] min-h-[36px] flex items-center justify-center" onClick={goUp} aria-label="Go up">
            <ArrowLeft size={18} />
          </button>
        </Show>
        <span class="text-sm text-[var(--text-secondary)] font-mono flex-1 truncate">{path()}</span>
        <label class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded bg-[var(--accent)] text-white cursor-pointer hover:bg-[var(--accent-hover)] transition-colors min-h-[36px]">
          <Upload size={16} />
          <span>Upload</span>
          <input type="file" multiple class="hidden" onChange={handleUpload} />
        </label>
        <button class="p-2 rounded hover:bg-[var(--interactive-hover)] min-w-[36px] min-h-[36px] flex items-center justify-center" aria-label="New folder">
          <Plus size={18} />
        </button>
      </div>

      {/* Upload progress */}
      <Show when={uploading()}>
        <div class="px-4 py-2 bg-[var(--accent-subtle)]">
          <div class="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
            <Loader2 size={16} class="animate-spin" />
            <span>Uploading… {progress()}%</span>
          </div>
          <div class="w-full h-1 mt-1 bg-[var(--bg-inset)] rounded-full">
            <div class="h-1 bg-[var(--accent)] rounded-full transition-all" style={{ width: `${progress()}%` }} />
          </div>
        </div>
      </Show>

      {/* Error */}
      <Show when={error()}>
        <div class="px-4 py-2 bg-[var(--danger-subtle)] text-sm text-[var(--danger)]">{error()}</div>
      </Show>

      {/* Loading */}
      <Show when={loading()}>
        <div class="flex items-center justify-center py-16">
          <Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" />
        </div>
      </Show>

      {/* File list */}
      <Show when={!loading()}>
        <div class="flex-1 overflow-y-auto">
          <Show when={entries().length === 0}>
            <div class="text-center py-16 text-[var(--text-tertiary)]">This folder is empty</div>
          </Show>
          <For each={entries()}>
            {(entry) => (
              <div
                class="group flex items-center gap-3 px-4 py-2 hover:bg-[var(--interactive-hover)] border-b border-[var(--border-default)]/50 cursor-pointer"
                onDblClick={() => navigate(entry)}
                onClick={() => entry.is_collection && navigate(entry)}
              >
                {entry.is_collection
                  ? <Folder size={18} class="text-[var(--accent)] shrink-0" />
                  : <FileIcon size={18} class="text-[var(--text-tertiary)] shrink-0" />}
                <span class="flex-1 text-sm truncate">{entry.name}</span>
                <span class="text-xs text-[var(--text-tertiary)] shrink-0 hidden sm:block">
                  {entry.is_collection ? "—" : formatSize(entry.size)}
                </span>
                <Show when={!entry.is_collection}>
                  <button
                    class="p-1.5 opacity-0 group-hover:opacity-100 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)] transition-all"
                    aria-label="Delete"
                    onClick={(e) => { e.stopPropagation(); handleDelete(entry); }}
                  >
                    <Trash2 size={14} />
                  </button>
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
