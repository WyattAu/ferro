import { createSignal, createEffect, onCleanup, Show } from "solid-js";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { ChevronLeft, ChevronRight } from "lucide-solid";

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

export default function PdfViewer(props: { url: string }) {
  let canvasRef: HTMLCanvasElement | undefined;
  const [numPages, setNumPages] = createSignal(0);
  const [page, setPage] = createSignal(1);
  const [error, setError] = createSignal("");
  const [rendering, setRendering] = createSignal(false);
  const [docProxy, setDocProxy] = createSignal<any>(null);

  createEffect(async () => {
    try {
      const doc = await pdfjsLib.getDocument({ url: props.url }).promise;
      setDocProxy(doc);
      setNumPages(doc.numPages);
    } catch (e: any) {
      setError(e?.message ?? "Failed to load PDF");
    }
  });

  createEffect(async () => {
    const d = docProxy();
    const p = page();
    if (!d || !canvasRef) return;
    setRendering(true);
    try {
      const pdfPage = await d.getPage(p);
      const viewport = pdfPage.getViewport({ scale: 1.5 });
      const canvas = canvasRef;
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      const ctx = canvas.getContext("2d")!;
      await pdfPage.render({ canvasContext: ctx, viewport }).promise;
    } catch (e: any) {
      setError(e?.message ?? "Render failed");
    } finally {
      setRendering(false);
    }
  });

  return (
    <div class="flex flex-col items-center w-full h-full min-h-0">
      <div class="flex items-center gap-2 py-1.5 shrink-0">
        <button onClick={() => setPage(Math.max(1, page() - 1))} disabled={page() <= 1}
          class="px-2 py-1 rounded hover:bg-[var(--bg-raised)] disabled:opacity-30">
          <ChevronLeft size={14} />
        </button>
        <span class="text-xs text-[var(--text-secondary)]">{page()} / {numPages()}</span>
        <button onClick={() => setPage(Math.min(numPages(), page() + 1))} disabled={page() >= numPages()}
          class="px-2 py-1 rounded hover:bg-[var(--bg-raised)] disabled:opacity-30">
          <ChevronRight size={14} />
        </button>
      </div>
      <div class="flex-1 overflow-auto min-h-0 w-full flex justify-center p-2">
        <Show when={!error()} fallback={<p class="text-sm text-[var(--danger)]">{error()}</p>}>
          <canvas ref={canvasRef} class="max-w-full shadow-lg rounded" />
        </Show>
      </div>
    </div>
  );
}
