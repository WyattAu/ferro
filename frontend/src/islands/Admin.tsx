import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api, type AdminStats, type AuditEntry, type AdminUser } from "../lib/api";
import { UserPlus, Users as UsersIcon, KeyRound, Trash2 } from "lucide-solid";
import { Loader2, ShieldAlert, Database, ScrollText, HardDrive, RefreshCw, Play } from "lucide-solid";
import { fmtSize } from "./FileBrowser";

function fmtUptime(secs: number): string {
  const d = Math.floor(secs / 86400), h = Math.floor((secs % 86400) / 3600), m = Math.floor((secs % 3600) / 60);
  return d > 0 ? `${d}d ${h}h` : h > 0 ? `${h}h ${m}m` : `${m}m`;
}

function Overview() {
  const [stats] = createResource(async () => api.adminStats());
  return (
    <Switch>
      <Match when={stats.loading}><div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
      <Match when={stats.error}><p class="text-sm text-[var(--danger)]">Failed to load stats: {String(stats.error)}</p></Match>
      <Match when={stats()}>
        {(s) => (
          <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
            <Stat label="Version" value={s().version} />
            <Stat label="Uptime" value={fmtUptime(s().uptime_seconds)} />
            <Stat label="Files" value={s().total_files.toLocaleString()} />
            <Stat label="Directories" value={s().total_directories.toLocaleString()} />
            <Stat label="Stored" value={fmtSize(s().total_bytes)} />
            <Stat label="Backend" value={`${s().storage_backend} · ${s().auth_type}`} />
          </div>
        )}
      </Match>
    </Switch>
  );
}

function Stat(props: { label: string; value: string }) {
  return (
    <div class="glass rounded-xl px-4 py-3">
      <div class="text-xs text-[var(--text-tertiary)] uppercase tracking-wider">{props.label}</div>
      <div class="text-lg font-semibold mt-1 truncate">{props.value}</div>
    </div>
  );
}

function Audit() {
  const [filter, setFilter] = createSignal("");
  const [entries, { refetch }] = createResource(filter, async (q) => {
    const params = q ? `?limit=100&path=${encodeURIComponent(q)}` : "?limit=100";
    return (await api.adminAudit(params)).entries as AuditEntry[];
  });
  return (
    <div>
      <div class="flex gap-2 mb-3">
        <input value={filter()} onInput={(e) => setFilter(e.currentTarget.value)} placeholder="Filter by path…"
          class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
        <button onClick={() => refetch()} class="p-2 rounded-lg hover:bg-[var(--bg-raised)]" aria-label="Refresh"><RefreshCw size={15} /></button>
      </div>
      <Switch>
        <Match when={entries.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
        <Match when={(entries() ?? []).length === 0}><p class="text-sm text-[var(--text-tertiary)] text-center pt-10">No entries</p></Match>
        <Match when={true}>
          <div class="font-mono text-xs">
            <For each={entries() ?? []}>
              {(e) => (
                <div class="flex items-center gap-2 px-3 py-1.5 border-b border-[var(--border-default)]/40 hover:bg-[var(--bg-raised)]">
                  <span class={`shrink-0 w-12 ${e.status >= 400 ? "text-[var(--danger)]" : "text-[var(--ok)]"}`}>{e.status}</span>
                  <span class="shrink-0 w-14 text-[var(--text-secondary)]">{e.method}</span>
                  <span class="flex-1 truncate">{e.path}</span>
                  <span class="hidden md:block text-[var(--text-tertiary)] shrink-0">{e.user}</span>
                  <span class="hidden sm:block text-[var(--text-tertiary)] shrink-0">{new Date(e.timestamp).toLocaleTimeString()}</span>
                </div>
              )}
            </For>
          </div>
        </Match>
      </Switch>
    </div>
  );
}

function Backups() {
  const [list, { refetch }] = createResource(async () => api.adminBackups());
  const [busy, setBusy] = createSignal(false);
  const [msg, setMsg] = createSignal<string | null>(null);

  const trigger = async () => {
    if (!confirm("Trigger a server-side backup now?")) return;
    setBusy(true); setMsg(null);
    try { await api.triggerBackup(); setMsg("Backup started"); refetch(); }
    catch (e) { setMsg(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };

  return (
    <div>
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-medium">Server backups</h2>
        <button onClick={trigger} disabled={busy()}
          class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">
          <Play size={13} /> Run now
        </button>
      </div>
      <Show when={msg()}><p class="text-sm text-[var(--text-secondary)] mb-3">{msg()}</p></Show>
      <Switch>
        <Match when={list.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
        <Match when={(list() ?? []).length === 0}><p class="text-sm text-[var(--text-tertiary)]">No server-side backups recorded (nightly Restic runs outside the app)</p></Match>
        <Match when={true}>
          <For each={(list() ?? []) as { id?: string; name?: string; created_at?: string }[]}>
            {(b) => (
              <div class="px-3 py-2 border-b border-[var(--border-default)]/40 text-sm font-mono">{b.name ?? b.id} · {b.created_at}</div>
            )}
          </For>
        </Match>
      </Switch>
    </div>
  );
}

function Gdpr() {
  const [data] = createResource(async () => api.adminGdpr());
  return (
    <Switch>
      <Match when={data.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
      <Match when={(data()?.requests ?? []).length === 0}>
        <p class="text-sm text-[var(--text-tertiary)]">No GDPR requests</p>
      </Match>
      <Match when={true}>
        <For each={data()!.requests}>
          {(r: { id: string; type?: string; status?: string; created_at?: string }) => (
            <div class="px-3 py-2 border-b border-[var(--border-default)]/40 text-sm font-mono">{r.id} · {r.type} · {r.status}</div>
          )}
        </For>
      </Match>
    </Switch>
  );
}


function Users() {
  const [users, { refetch }] = createResource(async () => (await api.adminUsers()).users as AdminUser[]);
  const [showAdd, setShowAdd] = createSignal(false);
  const [form, setForm] = createSignal({ username: "", display_name: "", email: "", password: "", role: "user" });
  const [msg, setMsg] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const { fmtSize } = { fmtSize: (b: number) => (b < 1048576 ? `${(b / 1024).toFixed(0)} KB` : b < 1073741824 ? `${(b / 1048576).toFixed(1)} MB` : `${(b / 1073741824).toFixed(2)} GB`) };

  const run = async (fn: () => Promise<unknown>) => {
    setBusy(true); setMsg(null);
    try { await fn(); refetch(); } catch (e) { setMsg(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  const add = async () => {
    const f = form();
    if (!f.username || !f.password) return;
    await run(async () => { await api.adminCreateUser(f); setForm({ username: "", display_name: "", email: "", password: "", role: "user" }); setShowAdd(false); });
  };
  const resetPw = async (u: AdminUser) => {
    const pw = prompt(`New password for ${u.username}:`);
    if (pw) await run(() => api.adminResetPassword(u.id, pw));
  };

  return (
    <div>
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-medium">Users</h2>
        <button onClick={() => setShowAdd(!showAdd())} class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white"><UserPlus size={13} /> Add user</button>
      </div>
      <Show when={msg()}><p class="text-xs text-[var(--danger)] mb-2">{msg()}</p></Show>
      <Show when={showAdd()}>
        <div class="glass rounded-xl p-4 mb-4 grid grid-cols-2 gap-2">
          <input value={form().username} onInput={(e) => setForm({ ...form(), username: e.currentTarget.value })} placeholder="username" class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <input value={form().display_name} onInput={(e) => setForm({ ...form(), display_name: e.currentTarget.value })} placeholder="display name" class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <input value={form().email} onInput={(e) => setForm({ ...form(), email: e.currentTarget.value })} placeholder="email" class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <input type="password" value={form().password} onInput={(e) => setForm({ ...form(), password: e.currentTarget.value })} placeholder="password" class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <select value={form().role} onChange={(e) => setForm({ ...form(), role: e.currentTarget.value })} class="bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-2 py-1.5 text-sm">
            <option value="user">user</option><option value="admin">admin</option>
          </select>
          <div class="flex gap-2 justify-end">
            <button onClick={() => setShowAdd(false)} class="px-3 py-1.5 text-sm rounded-lg hover:bg-[var(--bg-raised)]">Cancel</button>
            <button onClick={add} disabled={busy()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Create</button>
          </div>
        </div>
      </Show>
      <Show when={users.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Show>
      <For each={users() ?? []}>
        {(u) => (
          <div class="group flex items-center gap-3 px-3 py-2 border-b border-[var(--border-default)]/40 text-sm">
            <UsersIcon size={14} class="text-[var(--text-tertiary)] shrink-0" />
            <span class="w-32 truncate">{u.username}</span>
            <span class={`text-xs px-1.5 py-0.5 rounded ${u.role === "Admin" ? "bg-[var(--accent-subtle)] text-[var(--accent)]" : "bg-[var(--bg-raised)] text-[var(--text-secondary)]"}`}>{u.role}</span>
            <span class="flex-1 truncate text-xs text-[var(--text-tertiary)] hidden md:block">{u.email ?? ""}</span>
            <span class="text-xs text-[var(--text-tertiary)] hidden sm:block">{u.total_size ? fmtSize(u.total_size) : ""}</span>
            <div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <button title="Reset password" onClick={() => resetPw(u)} class="p-1 rounded hover:bg-[var(--bg-raised)]"><KeyRound size={13} /></button>
              <button title="Delete" onClick={() => confirm(`Delete ${u.username}?`) && run(() => api.adminDeleteUser(u.id))} class="p-1 rounded text-[var(--danger)] hover:bg-[var(--danger-subtle)]"><Trash2 size={13} /></button>
            </div>
          </div>
        )}
      </For>
    </div>
  );
}

function Groups() {
  const [groups, { refetch }] = createResource(async () => (await api.adminGroups()).groups ?? []);
  const [name, setName] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const create = async () => {
    if (!name().trim()) return;
    setBusy(true);
    try { await api.adminCreateGroup(name().trim()); setName(""); refetch(); }
    finally { setBusy(false); }
  };
  return (
    <div class="max-w-lg">
      <h2 class="text-sm font-medium mb-3">Groups</h2>
      <div class="flex gap-2 mb-4">
        <input value={name()} onInput={(e) => setName(e.currentTarget.value)} onKeyDown={(e) => e.key === "Enter" && create()} placeholder="New group name"
          class="flex-1 bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
        <button onClick={create} disabled={busy() || !name().trim()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">Create</button>
      </div>
      <Show when={groups.loading}><div class="flex justify-center pt-10"><Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" /></div></Show>
      <For each={groups() ?? []}>
        {(g) => (
          <div class="group flex items-center gap-3 px-3 py-2 border-b border-[var(--border-default)]/40 text-sm">
            <span class="flex-1">{g.name}</span>
            <Show when={g.name !== "ferro-admin"}>
              <button onClick={() => confirm(`Delete group ${g.name}?`) && api.adminDeleteGroup(g.id).then(refetch)}
                class="p-1 rounded opacity-0 group-hover:opacity-100 text-[var(--danger)] hover:bg-[var(--danger-subtle)]"><Trash2 size={13} /></button>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
}

const TABS = [
  { id: "overview", label: "Overview", icon: Database },
  { id: "audit", label: "Audit", icon: ScrollText },
  { id: "users", label: "Users", icon: UsersIcon },
  { id: "groups", label: "Groups", icon: UsersIcon },
  { id: "backups", label: "Backups", icon: HardDrive },
  { id: "gdpr", label: "GDPR", icon: ShieldAlert },
] as const;

export default function AdminPage() {
  const [tab, setTab] = createSignal<(typeof TABS)[number]["id"]>("overview");
  const [stats] = createResource(async () => {
    const me = await api.authInfo().catch(() => null);
    return !!me?.groups?.includes("ferro-admin");
  });

  return (
    <div class="p-6 max-w-5xl mx-auto h-full flex flex-col">
      <h1 class="text-xl font-semibold mb-4 shrink-0">Administration</h1>
      <Switch>
        <Match when={stats.loading}><div class="flex justify-center pt-16"><Loader2 size={24} class="animate-spin text-[var(--text-tertiary)]" /></div></Match>
        <Match when={stats() === false}>
          <div class="glass rounded-xl p-6 text-center">
            <ShieldAlert size={24} class="mx-auto mb-3 text-[var(--danger)]" />
            <p class="text-sm">Admin access required. This area is restricted to server administrators.</p>
          </div>
        </Match>
        <Match when={true}>
          <div class="flex gap-1 mb-4 shrink-0 flex-wrap">
            <For each={TABS}>
              {(t) => (
                <button onClick={() => setTab(t.id)}
                  class={`flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg ${tab() === t.id ? "bg-[var(--accent)] text-white" : "hover:bg-[var(--bg-raised)] text-[var(--text-secondary)]"}`}>
                  <t.icon size={14} /> {t.label}
                </button>
              )}
            </For>
          </div>
          <div class="flex-1 overflow-y-auto min-h-0">
            <Switch>
              <Match when={tab() === "overview"}><Overview /></Match>
              <Match when={tab() === "audit"}><Audit /></Match>
              <Match when={tab() === "users"}><Users /></Match>
              <Match when={tab() === "groups"}><Groups /></Match>
              <Match when={tab() === "backups"}><Backups /></Match>
              <Match when={tab() === "gdpr"}><Gdpr /></Match>
            </Switch>
          </div>
        </Match>
      </Switch>
    </div>
  );
}
