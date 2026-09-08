import { createSignal, createResource, For, Show, Switch, Match, onCleanup, createEffect } from "solid-js";
import { api, type ChatRoom, type ChatMsg } from "../lib/api";
import { Plus, Send, Loader2, MessageSquare, Hash } from "lucide-solid";

export default function ChatPage() {
  const [rooms, { refetch: refetchRooms }] = createResource(async () => (await api.listRooms()).rooms as ChatRoom[]);
  const [roomId, setRoomId] = createSignal<string | null>(null);
  const [messages, setMessages] = createSignal<ChatMsg[]>([]);
  const [loadingMsgs, setLoadingMsgs] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [draft, setDraft] = createSignal("");
  const [sending, setSending] = createSignal(false);
  let bottomRef: HTMLDivElement | undefined;
  let poll: ReturnType<typeof setInterval> | undefined;

  const loadMessages = async (id: string) => {
    setLoadingMsgs(true);
    try { setMessages((await api.listMessages(id)).messages as ChatMsg[]); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setLoadingMsgs(false); }
  };

  const select = (id: string) => {
    setRoomId(id);
    clearInterval(poll);
    loadMessages(id);
    poll = setInterval(() => id && loadMessages(id), 5000);
  };
  onCleanup(() => clearInterval(poll));

  createEffect(() => { if (messages()) bottomRef?.scrollIntoView({ behavior: "smooth" }); });

  const create = async () => {
    const name = prompt("Room name:");
    if (!name?.trim()) return;
    try { await api.createRoom(name.trim()); refetchRooms(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };

  const send = async () => {
    const id = roomId(); const text = draft().trim();
    if (!id || !text) return;
    setSending(true);
    try {
      await api.sendMessage(id, text);
      setDraft("");
      await loadMessages(id);
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setSending(false); }
  };

  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  return (
    <div class="h-full flex">
      <div class="w-64 border-r border-[var(--border-default)] flex flex-col shrink-0">
        <div class="p-3 shrink-0">
          <button onClick={create} class="w-full flex items-center justify-center gap-1.5 px-3 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">
            <Plus size={14} /> New room
          </button>
        </div>
        <div class="flex-1 overflow-y-auto">
          <For each={rooms() ?? []}>
            {(r) => (
              <button onClick={() => select(r.id)}
                class={`w-full text-left px-4 py-2.5 border-b border-[var(--border-default)]/40 hover:bg-[var(--bg-raised)] flex items-center gap-2 ${roomId() === r.id ? "bg-[var(--accent-subtle)]" : ""}`}>
                <Hash size={13} class="text-[var(--text-tertiary)] shrink-0" />
                <span class="text-sm truncate">{r.name}</span>
              </button>
            )}
          </For>
        </div>
      </div>

      <div class="flex-1 flex flex-col min-w-0">
        <Switch>
          <Match when={!roomId()}>
            <div class="flex-1 flex items-center justify-center text-sm text-[var(--text-tertiary)]">
              <span class="flex items-center gap-2"><MessageSquare size={16} /> Select a room</span>
            </div>
          </Match>
          <Match when={true}>
            <div class="flex-1 overflow-y-auto p-4 space-y-3 min-h-0">
              <Show when={loadingMsgs() && messages().length === 0}>
                <div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div>
              </Show>
              <For each={messages()}>
                {(m) => (
                  <div class="max-w-xl">
                    <div class="text-[11px] text-[var(--text-tertiary)] mb-0.5">{m.user_id} · {new Date(m.timestamp).toLocaleTimeString()}</div>
                    <div class="glass rounded-xl rounded-tl-sm px-3 py-2 text-sm whitespace-pre-wrap break-words" innerHTML={esc(m.content)} />
                  </div>
                )}
              </For>
              <div ref={bottomRef} />
            </div>
            <Show when={error()}><p class="text-xs text-[var(--danger)] px-4 pb-1">{error()}</p></Show>
            <div class="flex gap-2 p-3 border-t border-[var(--border-default)] shrink-0">
              <input value={draft()} onInput={(e) => setDraft(e.currentTarget.value)}
                onKeyDown={(e) => e.key === "Enter" && send()}
                placeholder="Message…"
                class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-2 text-sm outline-none focus:border-[var(--accent)]" />
              <button onClick={send} disabled={sending() || !draft().trim()}
                class="p-2.5 rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40" aria-label="Send">
                <Send size={15} />
              </button>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
