import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type Photo } from "../lib/api";
import { Loader2, Image as ImgIcon, X, Download, ChevronLeft, ChevronRight } from "lucide-solid";

export default function PhotosPage() {
  const [photos] = createResource(async () => (await api.listPhotos()).photos as Photo[]);
  const [active, setActive] = createSignal<number | null>(null);

  const current = () => (active() != null ? (photos() ?? [])[active()!] : null);

  const step = (d: 1 | -1) => {
    const list = photos() ?? [];
    if (!list.length || active() == null) return;
    setActive((active()! + d + list.length) % list.length);
  };

  return (
    <div class="p-6 max-w-6xl mx-auto h-full flex flex-col">
      <h1 class="text-xl font-semibold mb-4 shrink-0">Photos</h1>
      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={photos.loading}><div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
          <Match when={(photos() ?? []).length === 0}><p class="text-sm text-[var(--text-tertiary)] text-center pt-16">No photos indexed</p></Match>
          <Match when={true}>
            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-2">
              <For each={photos() ?? []}>
                {(p, i) => (
                  <button onClick={() => setActive(i())} class="group relative aspect-square rounded-xl overflow-hidden bg-[var(--bg-surface)] border border-[var(--border-default)]">
                    <img src={api.thumbUrl(p.path)} alt={p.name} loading="lazy"
                      class="w-full h-full object-cover group-hover:scale-105 transition-transform" />
                    <span class="absolute bottom-0 inset-x-0 px-2 py-1 text-[11px] truncate text-left bg-black/50 text-white opacity-0 group-hover:opacity-100 transition-opacity">{p.name}</span>
                  </button>
                )}
              </For>
            </div>
          </Match>
        </Switch>
      </div>

      <Show when={current()}>
        <div class="fixed inset-0 z-50 bg-black/85 flex items-center justify-center" onClick={() => setActive(null)}>
          <button class="absolute top-4 right-4 p-2 rounded-lg hover:bg-white/10 text-white" aria-label="Close"><X size={20} /></button>
          <button class="absolute left-4 p-2 rounded-lg hover:bg-white/10 text-white" aria-label="Previous"
            onClick={(e) => { e.stopPropagation(); step(-1); }}><ChevronLeft size={24} /></button>
          <img src={api.downloadUrlForPhoto(current()!.path)} alt={current()!.name}
            class="max-w-[90vw] max-h-[85vh] object-contain rounded" onClick={(e) => e.stopPropagation()} />
          <button class="absolute right-4 p-2 rounded-lg hover:bg-white/10 text-white" aria-label="Next"
            onClick={(e) => { e.stopPropagation(); step(1); }}><ChevronRight size={24} /></button>
          <div class="absolute bottom-4 left-0 right-0 text-center text-sm text-white/80">
            {current()!.name} · <a href={api.downloadUrlForPhoto(current()!.path)} download="" class="underline" onClick={(e) => e.stopPropagation()}>download</a>
          </div>
        </div>
      </Show>
    </div>
  );
}
