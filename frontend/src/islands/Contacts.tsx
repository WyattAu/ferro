import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type Contact } from "../lib/api";
import { Plus, Trash2, Loader2, User, Download, X } from "lucide-solid";

function parseVCard(v: string): { name: string; email: string; phone: string } {
  let name = "", email = "", phone = "";
  for (const line of v.split(/\r?\n/)) {
    if (line.startsWith("FN:")) name = line.slice(3).trim();
    else if (/^item?\d*\.?EMAIL[^:]*:/.test(line) && !email) email = line.split(":").slice(1).join(":").trim();
    else if (/^item?\d*\.?TEL[^:]*:/.test(line) && !phone) phone = line.split(":").slice(1).join(":").trim();
  }
  return { name: name || "Unknown", email, phone };
}

function buildVCard(name: string, email: string, phone: string): string {
  let v = "BEGIN:VCARD\r\nVERSION:3.0\r\n";
  const [first, ...rest] = name.split(" ");
  v += `N:${rest.join(" ") || first};${first};;;\r\n`;
  v += `FN:${name}\r\n`;
  if (email) v += `EMAIL;TYPE=INTERNET:${email}\r\n`;
  if (phone) v += `TEL;TYPE=CELL:${phone}\r\n`;
  v += "END:VCARD";
  return v;
}

export default function ContactsPage() {
  const [contacts, { refetch }] = createResource(async () => (await api.listContacts()).contacts as Contact[]);
  const [error, setError] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [showAdd, setShowAdd] = createSignal(false);
  const [form, setForm] = createSignal({ name: "", email: "", phone: "" });
  const [query, setQuery] = createSignal("");

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setError(null);
    try { await fn(); refetch(); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  const add = async () => {
    const f = form();
    if (!f.name.trim()) return;
    await run(async () => {
      await api.createContact({ address_book_id: "", vcard_data: buildVCard(f.name.trim(), f.email.trim(), f.phone.trim()) });
      setForm({ name: "", email: "", phone: "" });
      setShowAdd(false);
    });
  };

  const filtered = () => {
    const q = query().toLowerCase();
    return (contacts() ?? [])
      .map((c) => ({ uid: c.uid, ...parseVCard(c.vcard_data) }))
      .filter((c) => !q || c.name.toLowerCase().includes(q) || c.email.toLowerCase().includes(q));
  };

  const exportAll = () => {
    const all = (contacts() ?? []).map((c) => c.vcard_data).join("\r\n");
    const blob = new Blob([all], { type: "text/vcard" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "contacts.vcf";
    a.click();
  };

  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <div class="flex items-center justify-between mb-4 shrink-0">
        <h1 class="text-xl font-semibold">Contacts</h1>
        <div class="flex gap-2">
          <Show when={(contacts() ?? []).length > 0}>
            <button onClick={exportAll} class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]"><Download size={14} /> Export</button>
          </Show>
          <button onClick={() => setShowAdd(!showAdd())} class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white"><Plus size={14} /> Add</button>
        </div>
      </div>

      <Show when={showAdd()}>
        <div class="glass rounded-xl p-4 mb-4 space-y-2 shrink-0">
          <input value={form().name} onInput={(e) => setForm({ ...form(), name: e.currentTarget.value })} placeholder="Full name"
            class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <div class="flex gap-2">
            <input value={form().email} onInput={(e) => setForm({ ...form(), email: e.currentTarget.value })} placeholder="Email"
              class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
            <input value={form().phone} onInput={(e) => setForm({ ...form(), phone: e.currentTarget.value })} placeholder="Phone"
              class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          </div>
          <div class="flex gap-2 justify-end">
            <button onClick={() => setShowAdd(false)} class="px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]">Cancel</button>
            <button onClick={add} disabled={busy()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Save</button>
          </div>
        </div>
      </Show>

      <input value={query()} onInput={(e) => setQuery(e.currentTarget.value)} placeholder="Search…"
        class="mb-3 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)] shrink-0" />

      <Show when={error()}><p class="text-sm text-[var(--danger)] mb-3">{error()}</p></Show>

      <div class="flex-1 overflow-y-auto min-h-0">
        <Switch>
          <Match when={contacts.loading}><div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
          <Match when={filtered().length === 0}><p class="text-sm text-[var(--text-tertiary)] text-center pt-16">No contacts</p></Match>
          <Match when={true}>
            <For each={filtered()}>
              {(c) => (
                <div class="group flex items-center gap-3 px-4 py-2.5 hover:bg-[var(--bg-raised)] border-b border-[var(--border-default)]/40">
                  <div class="w-8 h-8 rounded-full bg-[var(--accent-subtle)] flex items-center justify-center shrink-0">
                    <User size={15} class="text-[var(--accent)]" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="text-sm truncate">{c.name}</div>
                    <div class="text-xs text-[var(--text-tertiary)] truncate">{[c.email, c.phone].filter(Boolean).join(" · ")}</div>
                  </div>
                  <a href={`mailto:${c.email}`} class="text-xs text-[var(--text-secondary)] hover:text-[var(--accent)] hidden sm:block">{c.email}</a>
                </div>
              )}
            </For>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
