import { createSignal, createResource, Show, For, onMount } from "solid-js";
import { Router, Route, useLocation, useNavigate } from "@solidjs/router";
import FileBrowser from "./FileBrowser";
import Callback from "./Callback";
import { api, getToken, type AuthInfo } from "../lib/api";
import { login, logout } from "../lib/auth";

function Shell(props: { children?: import("solid-js").JSX.Element }) {
  const [me] = createResource(async () => { try { return await api.authInfo(); } catch { return null; } });
  return (
    <div class="h-screen flex flex-col">
      <header class="glass flex items-center justify-between px-5 py-3 shrink-0 z-10">
        <a href="/ui/files/" class="text-lg font-bold font-mono tracking-tight select-none">
          FERRO<span class="text-[var(--accent)]">.</span>
        </a>
        <nav class="flex items-center gap-5 text-sm">
          <a href="/ui/files/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Files</a>
          <a href="/ui/tasks/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Tasks</a>
          <Show when={me()}>
            {(m) => <span class="text-xs text-[var(--text-tertiary)] max-w-40 truncate">{m().name ?? m().email ?? m().sub.slice(0, 8)}</span>}
          </Show>
          <button
            class="text-sm text-[var(--text-secondary)] hover:text-[var(--danger)] transition-colors"
            onClick={() => logout(localStorage.getItem("ferro_logout_url"))}
          >Sign out</button>
        </nav>
      </header>
      <main class="flex-1 min-h-0">{props.children}</main>
    </div>
  );
}

function Files() {
  return <Shell><FileBrowser /></Shell>;
}

function Tasks() {
  const [data] = createResource(async () => { try { return await api.listTasks(); } catch { return null; } });
  return (
    <Shell>
      <div class="p-6 max-w-3xl mx-auto">
        <h1 class="text-xl font-semibold mb-4">Tasks</h1>
        <Show when={data()} fallback={<p class="text-[var(--text-secondary)]">Loading…</p>}>
          <p class="text-sm text-[var(--text-secondary)]">{data()!.total} tasks</p>
        </Show>
      </div>
    </Shell>
  );
}

function Guard(props: { children?: import("solid-js").JSX.Element }) {
  const [ready, setReady] = createSignal(false);
  onMount(async () => {
    if (!getToken()) { await login(); return; }
    setReady(true);
  });
  return (
    <Show when={ready()} fallback={
      <div class="flex items-center justify-center h-screen">
        <div class="w-9 h-9 rounded-full border-2 border-[var(--border-strong)] border-t-[var(--accent)] animate-spin" />
      </div>
    }>{props.children}</Show>
  );
}

function NotFound() {
  const nav = useNavigate();
  onMount(() => nav("/ui/files/", { replace: true }));
  return null;
}

export default function App() {
  return (
    <Router>
      <Route path="/ui/auth/callback" component={Callback} />
      <Route path="/" component={Guard}>
        <Route path="/ui/files" component={Files} />
        <Route path="/ui/files/*rest" component={Files} />
        <Route path="/ui/tasks" component={Tasks} />
        <Route path="*" component={NotFound} />
      </Route>
    </Router>
  );
}
