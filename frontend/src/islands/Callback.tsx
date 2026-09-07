import { onMount, Show, createSignal } from "solid-js";
import { handleOAuthCallback } from "../lib/auth";

export default function Callback() {
  const [err, setErr] = createSignal<string | null>(null);
  onMount(async () => {
    const res = await handleOAuthCallback();
    if (!res.ok) setErr(res.error ?? "Login failed");
  });
  return (
    <div class="flex items-center justify-center h-screen">
      <div class="text-center">
        <Show when={!err()} fallback={<p class="text-sm text-[var(--danger)]">{err()} — <a href="/ui/files/" class="underline">retry</a></p>}>
          <div class="w-9 h-9 rounded-full border-2 border-[var(--border-strong)] border-t-[var(--accent)] animate-spin mx-auto mb-4" />
          <p class="text-sm text-[var(--text-secondary)]">Completing sign-in…</p>
        </Show>
      </div>
    </div>
  );
}
