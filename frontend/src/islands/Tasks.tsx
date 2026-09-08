import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type Task } from "../lib/api";
import { Plus, Trash2, Loader2, Circle, CircleDot, CheckCircle2, AlertCircle } from "lucide-solid";

const STATUS_ORDER = ["todo", "doing", "done"] as const;
const STATUS_ICON: Record<string, typeof Circle> = { todo: Circle, doing: CircleDot, done: CheckCircle2 };

export default function TasksPage() {
  const [tasks, { refetch }] = createResource(async () => (await api.listTasks()).tasks as Task[]);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [title, setTitle] = createSignal("");
  const [priority, setPriority] = createSignal("medium");
  const [filter, setFilter] = createSignal<string | null>(null);

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setError(null);
    try { await fn(); refetch(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const create = async () => {
    const t = title().trim();
    if (!t) return;
    await run(() => api.createTaskRaw({ title: t, priority: priority() }));
    setTitle("");
  };

  const move = (task: Task, dir: 1 | -1) => {
    const idx = STATUS_ORDER.indexOf((task.status || "todo") as (typeof STATUS_ORDER)[number]);
    const next = STATUS_ORDER[Math.min(Math.max(idx + dir, 0), STATUS_ORDER.length - 1)];
    if (next !== task.status) return run(() => api.patchTaskStatus(task.id, next));
  };

  const filtered = () => {
    const list = tasks() ?? [];
    return filter() ? list.filter((t) => t.status === filter()) : list;
  };
  const count = (s: string) => (tasks() ?? []).filter((t) => (t.status || "todo") === s).length;

  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <div class="flex items-center justify-between mb-4 shrink-0">
        <h1 class="text-xl font-semibold">Tasks</h1>
        <div class="flex gap-1 text-sm">
          <For each={[null, ...STATUS_ORDER]}>
            {(s) => (
              <button
                onClick={() => setFilter(s)}
                class={`px-2.5 py-1 rounded-lg ${filter() === s ? "bg-[var(--accent)] text-white" : "hover:bg-[var(--bg-raised)] text-[var(--text-secondary)]"}`}
              >{s ?? "all"}{s ? ` ${count(s)}` : ""}</button>
            )}
          </For>
        </div>
      </div>

      <div class="flex gap-2 mb-4 shrink-0">
        <input
          value={title()} onInput={(e) => setTitle(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && create()}
          placeholder="New task…"
          class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-2 text-sm outline-none focus:border-[var(--accent)]"
        />
        <select value={priority()} onChange={(e) => setPriority(e.currentTarget.value)}
          class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-2 py-2 text-sm outline-none">
          <option value="low">low</option><option value="medium">medium</option><option value="high">high</option>
        </select>
        <button onClick={create} disabled={busy() || !title().trim()}
          class="flex items-center gap-1.5 px-3 py-2 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">
          <Plus size={15} /> Add
        </button>
      </div>

      <Show when={error()}><p class="text-sm text-[var(--danger)] mb-3">{error()}</p></Show>

      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={tasks.loading}>
            <div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div>
          </Match>
          <Match when={filtered().length === 0}>
            <p class="text-sm text-[var(--text-tertiary)] text-center pt-16">No tasks</p>
          </Match>
          <Match when={true}>
            <For each={filtered()}>
              {(task) => {
                const Icon = STATUS_ICON[task.status || "todo"] ?? Circle;
                const isDone = (task.status || "todo") === "done";
                return (
                  <div class={`group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] border-b border-[var(--border-default)]/40 ${isDone ? "opacity-50" : ""}`}>
                    <Icon size={17} class={isDone ? "text-[var(--ok)]" : task.status === "doing" ? "text-[var(--accent)]" : "text-[var(--text-tertiary)]"} />
                    <span class={`flex-1 text-sm ${isDone ? "line-through" : ""}`}>{task.title}</span>
                    <Show when={task.priority === "high" && !isDone}>
                      <AlertCircle size={14} class="text-[var(--danger)]" />
                    </Show>
                    <Show when={task.due_date}>
                      <span class="text-xs text-[var(--text-tertiary)]">{new Date(task.due_date!).toLocaleDateString()}</span>
                    </Show>
                    <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <Show when={!isDone}>
                        <button title="Start/advance" onClick={() => move(task, 1)} class="px-1.5 py-0.5 rounded text-xs hover:bg-[var(--bg-raised)]">→</button>
                      </Show>
                      <Show when={(task.status || "todo") !== "todo"}>
                        <button title="Move back" onClick={() => move(task, -1)} class="px-1.5 py-0.5 rounded text-xs hover:bg-[var(--bg-raised)]">←</button>
                      </Show>
                      <button title="Delete" disabled={busy()} onClick={() => run(() => api.deleteTask(task.id))}
                        class="p-1 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"><Trash2 size={13} /></button>
                    </div>
                  </div>
                );
              }}
            </For>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
