import { createSignal, createResource, Show, For } from "solid-js";
import { api, type FileEntry, type ShareLink } from "../lib/api";
import { X, Copy, Link2, Loader2, Trash2 } from "lucide-solid";

export function ShareDialog(props: { entry: FileEntry; onClose: () => void }) {
  const davPath = () => props.entry.href.replace(/^\/users\//, "");
  const [password, setPassword] = createSignal("");
  const [expiresHours, setExpiresHours] = createSignal("168");
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [created, setCreated] = createSignal<ShareLink | null>(null);
  const [copied, setCopied] = createSignal(false);

  const shareUrl = (token: string) => `${window.location.origin}/s/${token}`;

  const create = async () => {
    setCreating(true); setError(null);
    try {
      const link = await api.createShare({
        path: davPath(),
        password: password() || undefined,
        expires_in_hours: parseInt(expiresHours(), 10) || undefined,
        share_type: props.entry.isCollection ? "folder" : "file",
        allow_download: true,
      });
      setCreated(link);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setCreating(false); }
  };

  const copy = async (token: string) => {
    await navigator.clipboard.writeText(shareUrl(token));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4" onClick={props.onClose}>
      <div class="glass rounded-2xl w-full max-w-md p-5 bg-[var(--bg-base)]/90" onClick={(e) => e.stopPropagation()}>
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-base font-semibold flex items-center gap-2"><Link2 size={16} /> Share "{props.entry.name}"</h2>
          <button onClick={props.onClose} class="p-1.5 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Close"><X size={16} /></button>
        </div>

        <Show when={created()} fallback={
          <div class="space-y-3">
            <label class="block text-xs text-[var(--text-secondary)]">
              Password (optional)
              <input value={password()} onInput={(e) => setPassword(e.currentTarget.value)} type="password" placeholder="None"
                class="mt-1 w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
            </label>
            <label class="block text-xs text-[var(--text-secondary)]">
              Expires in (hours)
              <input value={expiresHours()} onInput={(e) => setExpiresHours(e.currentTarget.value)} inputmode="numeric"
                class="mt-1 w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
            </label>
            <Show when={error()}><p class="text-xs text-[var(--danger)]">{error()}</p></Show>
            <button onClick={create} disabled={creating()}
              class="w-full flex items-center justify-center gap-2 px-3 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-50">
              <Show when={creating()} fallback={<Link2 size={14} />}><Loader2 size={14} class="animate-spin" /></Show>
              Create share link
            </button>
          </div>
        }>
          <div class="space-y-3">
            <div class="flex items-center gap-2 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-2">
              <span class="text-xs font-mono truncate flex-1">{shareUrl(created()!.token)}</span>
              <button onClick={() => copy(created()!.token)} class="p-1.5 rounded hover:bg-[var(--bg-raised)] shrink-0" aria-label="Copy link">
                <Copy size={14} />
              </button>
            </div>
            <Show when={copied()}><p class="text-xs text-[var(--ok)]">Copied!</p></Show>
            <p class="text-xs text-[var(--text-tertiary)]">
              {created()!.password ? "Password-protected" : "No password"} · expires {new Date(created()!.expires_at).toLocaleString()}
            </p>
          </div>
        </Show>
      </div>
    </div>
  );
}

export function SharesList() {
  const [shares, { refetch }] = createResource(async () => {
    const resp = await api.listShares();
    return resp.shares ?? resp;
  });
  const remove = async (token: string) => {
    if (!confirm("Revoke this share?")) return;
    await api.deleteShare(token);
    refetch();
  };
  return (
    <div class="p-6 max-w-3xl mx-auto">
      <h1 class="text-xl font-semibold mb-4">Active shares</h1>
      <Show when={shares()} fallback={<p class="text-sm text-[var(--text-secondary)]">Loading…</p>}>
        <Show when={(shares() ?? []).length === 0} fallback={
          <For each={shares() ?? []}>
            {(s) => (
              <div class="flex items-center gap-3 px-4 py-2.5 border-b border-[var(--border-default)]/40 text-sm">
                <Link2 size={15} class="text-[var(--accent)] shrink-0" />
                <span class="flex-1 truncate">{s.path}</span>
                <span class="text-xs text-[var(--text-tertiary)]">{s.download_count} downloads</span>
                <button onClick={() => remove(s.token)} class="p-1.5 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]" aria-label="Revoke"><Trash2 size={14} /></button>
              </div>
            )}
          </For>
        }>
          <p class="text-sm text-[var(--text-tertiary)]">No active shares</p>
        </Show>
      </Show>
    </div>
  );
}
