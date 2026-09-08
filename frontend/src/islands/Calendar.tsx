import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type CalEvent } from "../lib/api";
import { Plus, Trash2, Loader2, Calendar as CalIcon, X } from "lucide-solid";

function parseICal(ical: string): { summary: string; start: string; end: string } {
  let summary = "", start = "", end = "";
  for (const line of ical.split(/\r?\n/)) {
    if (line.startsWith("SUMMARY:")) summary = line.slice(8).trim();
    else if (line.startsWith("DTSTART")) start = line.split(":").pop()?.trim() ?? "";
    else if (line.startsWith("DTEND")) end = line.split(":").pop()?.trim() ?? "";
  }
  return { summary: summary || "(untitled)", start: parseICalDate(start), end: parseICalDate(end) };
}

function parseICalDate(d: string): string {
  if (!d) return "";
  const m = d.match(/^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2}))?/);
  if (!m) return d;
  const [, y, mo, dd, h, mi] = m;
  return h ? `${y}-${mo}-${dd} ${h}:${mi}` : `${y}-${mo}-${dd}`;
}

function toICalDate(local: string): string {
  return local.replace(/[-: ]/g, "").slice(0, 15) + "00";
}

function buildICal(summary: string, startLocal: string, endLocal: string): string {
  return [
    "BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Ferro//UI//EN",
    "BEGIN:VEVENT",
    `UID:${crypto.randomUUID()}@ferro`,
    `DTSTAMP:${new Date().toISOString().replace(/[-:]/g, "").slice(0, 15)}Z`,
    `DTSTART:${toICalDate(startLocal)}`,
    ...(endLocal ? [`DTEND:${toICalDate(endLocal)}`] : []),
    `SUMMARY:${summary}`,
    "END:VEVENT", "END:VCALENDAR",
  ].join("\r\n");
}

export default function CalendarPage() {
  const [events, { refetch }] = createResource(async () => (await api.listEvents()).events as CalEvent[]);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [showAdd, setShowAdd] = createSignal(false);
  const [form, setForm] = createSignal({ title: "", start: "", end: "" });

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setError(null);
    try { await fn(); refetch(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const add = async () => {
    const f = form();
    if (!f.title.trim() || !f.start) return;
    await run(async () => {
      await api.createEvent({ calendar_id: "", ical_data: buildICal(f.title.trim(), f.start, f.end) });
      setForm({ title: "", start: "", end: "" });
      setShowAdd(false);
    });
  };

  const parsed = () => (events() ?? []).map((ev) => ({ uid: ev.uid, ...parseICal(ev.ical_data) }));

  const upcoming = () => {
    const now = new Date().toISOString().slice(0, 16).replace("T", " ");
    return parsed()
      .filter((e) => e.end >= now || e.start >= now)
      .sort((a, b) => a.start.localeCompare(b.start));
  };
  const past = () => {
    const now = new Date().toISOString().slice(0, 16).replace("T", " ");
    return parsed().filter((e) => e.end < now && e.start < now).sort((a, b) => b.start.localeCompare(a.start));
  };

  const Row = (props: { ev: { summary: string; start: string; end: string }; uid: string; onDone?: boolean }) => (
    <div class="group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] border-b border-[var(--border-default)]/40">
      <CalIcon size={15} class="text-[var(--accent)] shrink-0" />
      <span class={`flex-1 text-sm truncate ${props.onDone ? "opacity-50" : ""}`}>{props.ev.summary}</span>
      <span class="text-xs text-[var(--text-tertiary)] tabular-nums">{props.ev.start}</span>
      <button disabled={busy()} onClick={() => run(() => api.deleteEvent(props.uid))}
        class="p-1 rounded opacity-0 group-hover:opacity-100 text-[var(--danger)] hover:bg-[var(--danger-subtle)]" aria-label="Delete"><Trash2 size={13} /></button>
    </div>
  );

  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <div class="flex items-center justify-between mb-4 shrink-0">
        <h1 class="text-xl font-semibold">Calendar</h1>
        <button onClick={() => setShowAdd(!showAdd())} class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">
          <Plus size={14} /> Event
        </button>
      </div>

      <Show when={showAdd()}>
        <div class="glass rounded-xl p-4 mb-4 space-y-2 shrink-0">
          <input value={form().title} onInput={(e) => setForm({ ...form(), title: e.currentTarget.value })} placeholder="Event title"
            class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <div class="flex gap-2">
            <label class="flex-1 text-xs text-[var(--text-secondary)]">Start
              <input type="datetime-local" value={form().start} onInput={(e) => setForm({ ...form(), start: e.currentTarget.value })}
                class="mt-1 w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
            </label>
            <label class="flex-1 text-xs text-[var(--text-secondary)]">End
              <input type="datetime-local" value={form().end} onInput={(e) => setForm({ ...form(), end: e.currentTarget.value })}
                class="mt-1 w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
            </label>
          </div>
          <div class="flex gap-2 justify-end">
            <button onClick={() => setShowAdd(false)} class="px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]">Cancel</button>
            <button onClick={add} disabled={busy()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Create</button>
          </div>
        </div>
      </Show>

      <Show when={error()}><p class="text-sm text-[var(--danger)] mb-3">{error()}</p></Show>

      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={events.loading}><div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
          <Match when={(events() ?? []).length === 0}><p class="text-sm text-[var(--text-tertiary)] text-center pt-16">No events</p></Match>
          <Match when={true}>
            <For each={upcoming()}>{(ev) => <Row ev={ev} uid={ev.uid} />}</For>
            <Show when={past().length > 0}>
              <h2 class="text-xs uppercase tracking-wider text-[var(--text-tertiary)] px-4 pt-4 pb-2">Past</h2>
              <For each={past()}>{(ev) => <Row ev={ev} uid={ev.uid} onDone />}</For>
            </Show>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
