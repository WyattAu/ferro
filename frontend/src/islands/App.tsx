import { createSignal, createResource, Show, For, onMount } from "solid-js";
import { Router, Route, useLocation, useNavigate } from "@solidjs/router";
import FileBrowser from "./FileBrowser";
import TrashPage from "./Trash";
import Callback from "./Callback";
import { SharesList } from "./ShareDialog";
import TasksPage from "./Tasks";
import NotesPage from "./Notes";
import ContactsPage from "./Contacts";
import CalendarPage from "./Calendar";
import PhotosPage from "./Photos";
import WhiteboardPage from "./Whiteboard";
import ChatPage from "./Chat";
import AdminPage from "./Admin";
import { api, getToken, getExpiresAt, scheduleRefresh, type AuthInfo } from "../lib/api";
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
          <a href="/ui/calendar/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Calendar</a>
              <a href="/ui/contacts/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Contacts</a>
              <a href="/ui/notes/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Notes</a>
              <a href="/ui/photos/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Photos</a>
              <a href="/ui/chat/" class="text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors">Chat</a>
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

function AdminRoute() {
  return <Shell><AdminPage /></Shell>;
}

function PhotosRoute() {
  return <Shell><PhotosPage /></Shell>;
}

function WhiteboardRoute() {
  return <Shell><WhiteboardPage /></Shell>;
}

function ChatRoute() {
  return <Shell><ChatPage /></Shell>;
}

function ContactsRoute() {
  return <Shell><ContactsPage /></Shell>;
}

function CalendarRoute() {
  return <Shell><CalendarPage /></Shell>;
}

function NotesRoute() {
  return <Shell><NotesPage /></Shell>;
}

function TasksRoute() {
  return <Shell><TasksPage /></Shell>;
}

function TrashRoute() {
  return <Shell><TrashPage /></Shell>;
}

function SharesRoute() {
  return <Shell><SharesList /></Shell>;
}

function Guard(props: { children?: import("solid-js").JSX.Element }) {
  const [ready, setReady] = createSignal(false);
  onMount(async () => {
    if (!getToken()) { await login(); return; }
    if (getExpiresAt()) scheduleRefresh();
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
        <Route path="/ui/tasks" component={TasksRoute} />
        <Route path="/ui/notes" component={NotesRoute} />
        <Route path="/ui/contacts" component={ContactsRoute} />
        <Route path="/ui/calendar" component={CalendarRoute} />
        <Route path="/ui/photos" component={PhotosRoute} />
        <Route path="/ui/whiteboard" component={WhiteboardRoute} />
        <Route path="/ui/chat" component={ChatRoute} />
        <Route path="/ui/admin" component={AdminRoute} />
        <Route path="/ui/trash" component={TrashRoute} />
        <Route path="/ui/shares" component={SharesRoute} />
        <Route path="*" component={NotFound} />
      </Route>
    </Router>
  );
}
