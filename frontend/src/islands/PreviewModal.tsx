import { createSignal, createResource, Show, Switch, Match } from "solid-js";
import { api, type FileEntry } from "../lib/api";
import { X, Download, Loader2, Pencil } from "lucide-solid";

const IMAGE_EXT = /\.(png|jpe?g|gif|webp|svg|bmp|ico|avif)$/i;
const TEXT_EXT = /\.(txt|md|markdown|json|ya?ml|toml|xml|html?|css|js|jsx|ts|tsx|py|rs|go|java|c|cpp|h|sh|env|csv|log|ini|conf|cfg)$/i;
const OFFICE_EXT = /\.(docx|xlsx|pptx|odt|ods|odp)$/i;

export function PreviewModal(props: { entry: FileEntry; onClose: () => void }) {
  const davPath = () => props.entry.href.replace(/^\/users\//, "");
  const [editUrl, setEditUrl] = createSignal<string | null>(null);
  const [editError, setEditError] = createSignal<string | null>(null);

  const openEditor = async () => {
    setEditError(null);
    try { setEditUrl(await api.buildEditorUrl(davPath())); }
    catch (e) { setEditError(e instanceof Error ? e.message : String(e)); }
  };

  const kind = (): "image" | "pdf" | "text" | "office" | "none" => {
    if (IMAGE_EXT.test(props.entry.name)) return "image";
    if (/pdf$/i.test(props.entry.name)) return "pdf";
    if (TEXT_EXT.test(props.entry.name)) return "text";
    if (OFFICE_EXT.test(props.entry.name)) return "office";
    return "none";
  };

  const [content] = createResource(() => (kind() === "none" ? null : davPath()), async (p) => {
    if (!p) return null;
    const k = kind();
    if (k === "text") return api.fetchText(p);
    if (k === "image") return URL.createObjectURL(await api.fetchBlob(p));
    if (k === "pdf") return URL.createObjectURL(await api.fetchBlob(p));
    return null;
  });

  return (
    <div class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4" onClick={props.onClose}>
      <Show when={editUrl()} fallback={
      <div class="glass rounded-2xl w-full max-w-4xl max-h-[85vh] flex flex-col bg-[var(--bg-base)]/90" onClick={(e) => e.stopPropagation()}>
        <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-default)] shrink-0">
          <span class="text-sm font-medium truncate">{props.entry.name}</span>
          <div class="flex items-center gap-1">
            <a href={api.downloadUrl(davPath())} download="" class="p-1.5 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Download"><Download size={15} /></a>
            <button onClick={props.onClose} class="p-1.5 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Close"><X size={15} /></button>
          </div>
        </div>
        <div class="flex-1 overflow-auto p-4 flex items-center justify-center min-h-0">
          <Switch>
            <Match when={kind() === "office"}>
              <div class="text-center">
                <p class="text-sm text-[var(--text-secondary)] mb-3">Office document</p>
                <Show when={editError()}><p class="text-xs text-[var(--danger)] mb-2">{editError()}</p></Show>
                <div class="flex gap-2 justify-center">
                  <button onClick={openEditor} class="px-4 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white inline-flex items-center gap-2">
                    <Pencil size={14} /> Edit online
                  </button>
                  <a href={api.downloadUrl(davPath())} download="" class="px-4 py-2 text-sm rounded-lg hover:bg-[var(--bg-raised)] inline-flex items-center gap-2">
                    <Download size={14} /> Download
                  </a>
                </div>
              </div>
            </Match>
            <Match when={kind() === "none"}>
              <div class="text-center">
                <p class="text-sm text-[var(--text-secondary)] mb-3">No preview available</p>
                <a href={api.downloadUrl(davPath())} download="" class="px-4 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white inline-flex items-center gap-2">
                  <Download size={14} /> Download
                </a>
              </div>
            </Match>
            <Match when={content.loading}>
              <Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" />
            </Match>
            <Match when={kind() === "image"}>
              <img src={content() ?? undefined} alt={props.entry.name} class="max-w-full max-h-[70vh] object-contain rounded" />
            </Match>
            <Match when={kind() === "pdf"}>
              <iframe src={content() ?? undefined} class="w-full h-[70vh] rounded border-0" title={props.entry.name} />
            </Match>
            <Match when={kind() === "text"}>
              <pre class="w-full h-full overflow-auto text-xs font-mono whitespace-pre-wrap text-[var(--text-primary)]">{content()}</pre>
            </Match>
          </Switch>
        </div>
      </div>
      }>
        <div class="glass rounded-2xl w-full max-w-[95vw] h-[92vh] flex flex-col bg-[var(--bg-base)]/90">
          <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-default)] shrink-0">
            <span class="text-sm font-medium truncate">{props.entry.name} — editing</span>
            <button onClick={() => { setEditUrl(null); }} class="p-1.5 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Close editor"><X size={15} /></button>
          </div>
          <iframe src={editUrl() ?? undefined} class="flex-1 w-full border-0" title="Office editor" allow="clipboard-read; clipboard-write" />
        </div>
      </Show>
    </div>
  );
}
