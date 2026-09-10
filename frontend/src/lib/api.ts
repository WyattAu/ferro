const STORAGE_KEY = "ferro_access_token";
const REFRESH_KEY = "ferro_refresh_token";
const EXPIRY_KEY = "ferro_token_expires_at";
const LOGOUT_KEY = "ferro_logout_url";

export function getToken(): string | null {
  try { return localStorage.getItem(STORAGE_KEY); } catch { return null; }
}
export function getRefreshToken(): string | null {
  try { return localStorage.getItem(REFRESH_KEY); } catch { return null; }
}
export function getExpiresAt(): number {
  try { return parseInt(localStorage.getItem(EXPIRY_KEY) ?? "0", 10); } catch { return 0; }
}
export function storeTokens(access: string, refresh: string | null, expiresIn: number) {
  localStorage.setItem(STORAGE_KEY, access);
  if (refresh) localStorage.setItem(REFRESH_KEY, refresh);
  localStorage.setItem(EXPIRY_KEY, String(Date.now() + expiresIn * 1000));
}
export function storeLogoutUrl(url: string) { localStorage.setItem(LOGOUT_KEY, url); }
export function getLogoutUrl(): string | null {
  try { return localStorage.getItem(LOGOUT_KEY); } catch { return null; }
}
export function clearTokens() {
  [STORAGE_KEY, REFRESH_KEY, EXPIRY_KEY, LOGOUT_KEY].forEach((k) => { try { localStorage.removeItem(k); } catch { /* ignore */ } });
}

export class ApiError extends Error {
  constructor(public status: number, message: string) { super(message); }
}

function redirectToLogin() {
  window.location.href = `/api/auth/login?redirect=${encodeURIComponent(window.location.pathname + window.location.search)}`;
}

let refreshing: Promise<boolean> | null = null;

export async function refreshAccessToken(): Promise<boolean> {
  const rt = getRefreshToken();
  if (!rt) return false;
  refreshing ??= (async () => {
    try {
      const resp = await fetch("/api/auth/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refresh_token: rt }),
      });
      if (!resp.ok) return false;
      const data = await resp.json();
      storeTokens(data.access_token, data.refresh_token ?? rt, data.expires_in ?? 900);
      scheduleRefresh();
      return true;
    } catch { return false; }
    finally { refreshing = null; }
  })();
  return refreshing;
}

let refreshTimer: ReturnType<typeof setTimeout> | null = null;
export function scheduleRefresh() {
  if (refreshTimer) clearTimeout(refreshTimer);
  const at = getExpiresAt();
  if (!at) return;
  const delay = Math.max((at - Date.now()) - 90_000, 10_000);
  refreshTimer = setTimeout(() => { if (!refreshAccessToken()) clearTokens(); }, delay);
}

async function request<T>(method: string, path: string, body?: BodyInit, headers: Record<string, string> = {}, retried = false): Promise<T> {
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  const resp = await fetch(path, { method, headers, body });
  if (resp.status === 401 && !retried) {
    if (await refreshAccessToken()) return request<T>(method, path, body, headers, true);
    clearTokens();
    redirectToLogin();
    throw new ApiError(401, "Unauthorized");
  }
  if (!resp.ok) {
    let msg = `HTTP ${resp.status}`;
    try { const j = await resp.json(); msg = j.error ?? j.message ?? msg; } catch { /* ignore */ }
    throw new ApiError(resp.status, msg);
  }
  return resp.json() as Promise<T>;
}

export interface AuthInfo { sub: string; iss: string; aud: string; email: string | null; name: string | null; groups: string[]; auth_type: string; }
export interface CallbackResponse {
  access_token: string; token_type: string; expires_in: number;
  user: { sub: string; email?: string | null; name?: string | null };
  redirect: string; refresh_token: string | null; logout_url: string;
}

export interface FileEntry {
  href: string; name: string; isCollection: boolean;
  size: number | null; mimeType: string | null; modifiedAt: string | null;
}

function decodeHref(href: string): string {
  try { return decodeURIComponent(href); } catch { return href; }
}

function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export async function propfind(path: string, depth = "1", namespace: "users" | "_spaces" = "users"): Promise<FileEntry[]> {
  const resp = await fetch(`/${namespace}/${path}`, { method: "PROPFIND", headers: { Depth: depth, ...authHeaders() } });
  if (resp.status === 401) {
    if (await refreshAccessToken()) return propfind(path, depth);
    clearTokens(); redirectToLogin();
    throw new ApiError(401, "Unauthorized");
  }
  if (!resp.ok) throw new ApiError(resp.status, `PROPFIND failed: ${resp.status}`);
  const xml = await resp.text();
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  const responses = Array.from(doc.getElementsByTagNameNS("DAV:", "response"));
  const current = decodeHref(`/${namespace}/${path}`).replace(/\/$/, "");
  const entries: FileEntry[] = [];
  for (const r of responses) {
    const hrefEl = r.getElementsByTagNameNS("DAV:", "href")[0];
    if (!hrefEl?.textContent) continue;
    const href = decodeHref(hrefEl.textContent);
    if (href.replace(/\/$/, "") === current) continue;
    const isCollection = !!r.getElementsByTagNameNS("DAV:", "collection")[0];
    const sizeEl = r.getElementsByTagNameNS("DAV:", "getcontentlength")[0];
    const typeEl = r.getElementsByTagNameNS("DAV:", "getcontenttype")[0];
    const modEl = r.getElementsByTagNameNS("DAV:", "getlastmodified")[0];
    const name = href.replace(/\/$/, "").split("/").pop() || "/";
    entries.push({
      href, name, isCollection,
      size: sizeEl ? parseInt(sizeEl.textContent || "0", 10) : null,
      mimeType: typeEl?.textContent ?? null,
      modifiedAt: modEl?.textContent ?? null,
    });
  }
  entries.sort((a, b) => (a.isCollection === b.isCollection) ? a.name.localeCompare(b.name) : a.isCollection ? -1 : 1);
  return entries;
}

function davUrl(path: string, namespace: "users" | "_spaces" = "users") {
  return `/${namespace}/${path}`;
}

export async function davRequest(
  method: string,
  path: string,
  extraHeaders: Record<string, string> = {},
  body?: BodyInit,
  namespace: "users" | "_spaces" = "users",
): Promise<Response> {
  const doFetch = () => fetch(davUrl(path, namespace), { method, headers: { ...authHeaders(), ...extraHeaders }, body });
  let resp = await doFetch();
  if (resp.status === 401 && (await refreshAccessToken())) resp = await doFetch();
  return resp;
}

function destinationHeader(toPath: string, namespace: "users" | "_spaces" = "users"): Record<string, string> {
  return { Destination: `${window.location.origin}/${namespace}/${toPath}` };
}

export async function uploadFile(
  path: string,
  file: File,
  onProgress?: (pct: number) => void,
  namespace: "users" | "_spaces" = "users",
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("PUT", davUrl(path, namespace));
    const token = getToken();
    if (token) xhr.setRequestHeader("Authorization", `Bearer ${token}`);
    xhr.upload.onprogress = (e) => { if (e.lengthComputable && onProgress) onProgress(Math.round((e.loaded / e.total) * 100)); };
    xhr.onload = () => xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new ApiError(xhr.status, `Upload failed: ${xhr.status}`));
    xhr.onerror = () => reject(new ApiError(0, "Network error"));
    xhr.send(file);
  });
}

export function downloadUrl(path: string, namespace: "users" | "_spaces" = "users"): string {
  const token = getToken();
  return token
    ? `${davUrl(path, namespace)}?Authorization=${encodeURIComponent(`Bearer ${token}`)}`
    : davUrl(path, namespace);
}

export const api = {
  authInfo: () => request<AuthInfo>("GET", "/api/auth/info"),
  loginUrl: async (redirect: string): Promise<string> => {
    const resp = await fetch(`/api/auth/login?redirect=${encodeURIComponent(redirect)}`);
    const data = await resp.json();
    return data.authorization_url as string;
  },
  callback: (code: string, state: string) =>
    request<CallbackResponse>("GET", `/api/auth/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`),
  refresh: (rt: string) => request<{ access_token: string; expires_in: number; refresh_token?: string }>("POST", "/api/auth/refresh", JSON.stringify({ refresh_token: rt }), { "Content-Type": "application/json" }),
  delete: (path: string, namespace: "users" | "_spaces" = "users") =>
    davRequest("DELETE", path, {}, undefined, namespace).then(ensureOk),
  mkdir: (path: string, namespace: "users" | "_spaces" = "users") =>
    davRequest("MKCOL", path, {}, undefined, namespace).then(ensureOk),
  move: (from: string, to: string, namespace: "users" | "_spaces" = "users") =>
    davRequest("MOVE", from, { ...destinationHeader(to, namespace) }, undefined, namespace).then(ensureOk),
  copy: (from: string, to: string, namespace: "users" | "_spaces" = "users") =>
    davRequest("COPY", from, { ...destinationHeader(to, namespace) }, undefined, namespace).then(ensureOk),
  downloadUrl,
  fetchText: (path: string) => fetch(davUrl(path), { headers: authHeaders() }).then((r) => r.ok ? r.text() : Promise.reject(new ApiError(r.status, "Fetch failed"))),
  fetchBlob: (path: string) => fetch(davUrl(path), { headers: authHeaders() }).then((r) => r.ok ? r.blob() : Promise.reject(new ApiError(r.status, "Fetch failed"))),
  listTasks: () => request<{ tasks: Task[]; total: number }>("GET", "/api/tasks"),
  createTaskRaw: (body: Partial<Task>) => request<Task>("POST", "/api/tasks", JSON.stringify(body), { "Content-Type": "application/json" }),
  patchTaskStatus: (id: string, status: string) => request<Task>("PATCH", `/api/tasks/${id}/status`, JSON.stringify({ status }), { "Content-Type": "application/json" }),
  deleteTask: (id: string) => request<void>("DELETE", `/api/tasks/${id}`),

  listNotes: () => request<{ notes: Note[]; total: number }>("GET", "/api/notes"),
  createNote: (body: { title: string; content?: string; folder?: string; tags?: string }) =>
    request<Note>("POST", "/api/notes", JSON.stringify(body), { "Content-Type": "application/json" }),
  updateNote: (id: string, body: { title?: string; content?: string }) =>
    request<Note>("PUT", `/api/notes/${id}`, JSON.stringify(body), { "Content-Type": "application/json" }),
  deleteNote: (id: string) => request<void>("DELETE", `/api/notes/${id}`),

  listContacts: () => request<{ contacts: Contact[] }> ("GET", "/api/contacts"),
  createContact: (body: { address_book_id: string; vcard_data: string }) =>
    request<Contact>("POST", "/api/contacts", JSON.stringify(body), { "Content-Type": "application/json" }),
  deleteContact: (uid: string) => request<void>("DELETE", `/api/contacts/${uid}`),

  listEvents: () => request<{ events: CalEvent[] }>("GET", "/api/calendar/events"),
  createEvent: (body: { calendar_id: string; ical_data: string }) =>
    request<CalEvent>("POST", "/api/calendar/events", JSON.stringify(body), { "Content-Type": "application/json" }),
  deleteEvent: (uid: string) => request<void>("DELETE", `/api/calendar/events/${uid}`),

  listPhotos: () => request<{ photos: Photo[] }>("GET", "/api/photos"),
  // Photo paths from /api/photos are decoded virtual paths; encode the
  // whole thing into the single :path segment (slashes ride as %2F).
  thumbUrl: (path: string) => `/api/photos/thumbnail/${encodeURIComponent(path)}`,
  // Fetch any API/DAV URL with the Bearer token and wrap it in an object
  // URL — <img>/<video>/<audio> src attributes cannot attach auth headers.
  authedObjectUrl: async (url: string): Promise<string> => {
    const doFetch = () => fetch(url, { headers: authHeaders() });
    let resp = await doFetch();
    if (resp.status === 401 && (await refreshAccessToken())) resp = await doFetch();
    if (!resp.ok) throw new ApiError(resp.status, `Fetch failed: ${resp.status}`);
    return URL.createObjectURL(await resp.blob());
  },
  downloadUrlForPhoto: (path: string) => path.startsWith("/users/") ? downloadUrl(path) : `/users/${path}`,

  listBoards: () => request<{ whiteboards: Board[]; total: number }>("GET", "/api/whiteboard"),
  createBoard: (name: string) => request<Board>("POST", "/api/whiteboard", JSON.stringify({ name }), { "Content-Type": "application/json" }),
  getBoard: (id: string) => request<unknown>("GET", `/api/whiteboard/${id}`),
  saveBoard: (id: string, content: string) =>
    fetch(`/api/whiteboard/${id}`, { method: "PUT", headers: { ...authHeaders(), "Content-Type": "application/json" }, body: JSON.stringify({ content }) }).then(ensureOk),

  listRooms: () => request<{ rooms: ChatRoom[] }>("GET", "/api/chat/rooms"),
  createRoom: (name: string) => request<ChatRoom>("POST", "/api/chat/rooms", JSON.stringify({ name }), { "Content-Type": "application/json" }),
  listMessages: (roomId: string) => request<{ messages: ChatMsg[]; total: number; has_more: boolean }>("GET", `/api/chat/rooms/${roomId}/messages`),
  sendMessage: (roomId: string, content: string) =>
    request<ChatMsg>("POST", `/api/chat/rooms/${roomId}/messages`, JSON.stringify({ content }), { "Content-Type": "application/json" }),

  adminStats: () => request<AdminStats>("GET", "/api/admin/stats"),
  adminAudit: (params: string) => request<{ entries: AuditEntry[] }>("GET", `/api/admin/audit${params}`),
  adminBackups: () => request<unknown[]>("GET", "/api/admin/backups"),
  triggerBackup: () => request<unknown>("POST", "/api/admin/backup", "{}", { "Content-Type": "application/json" }),
  adminGdpr: () => request<{ requests: { id: string; type?: string; status?: string; created_at?: string }[] }>("GET", "/api/admin/gdpr"),

  wopiDiscovery: () => request<string>("GET", "/wopi/office-discovery"),
  wopiIssueToken: (path: string) =>
    request<{ access_token: string; expires_in: number }>(
      "POST",
      `/wopi-token?path=${encodeURIComponent(path)}`,
      undefined,
    ),
  // path must be the FULL virtual path INCLUDING /users/<sub>/ — WOPI
  // handlers key storage directly on it. Pass decoded href, not davPath().
  buildEditorUrl: async (fullPath: string): Promise<string> => {
    const path = decodeURIComponent(fullPath);
    const xml = await api.wopiDiscovery();
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    const doc = new DOMParser().parseFromString(xml, "application/xml");
    let urlsrc: string | null = null;
    for (const el of Array.from(doc.getElementsByTagName("action"))) {
      if (el.getAttribute("name") === "edit" && el.getAttribute("ext") === ext) {
        urlsrc = el.getAttribute("urlsrc");
        break;
      }
    }
    if (!urlsrc) throw new ApiError(404, `No online editor for .${ext}`);
    const token = await api.wopiIssueToken(path);
    const wopiSrc = `${window.location.origin}/wopi/files/${encodeURIComponent(path)}`;
    const sep = urlsrc.includes("?") ? "&" : "?";
    return `${urlsrc}${sep}WOPISrc=${encodeURIComponent(wopiSrc)}&access_token=${encodeURIComponent(token.access_token)}`;
  },

  // --- User self-service ---
  getMe: () => request<MeProfile>("GET", "/api/users/me"),
  updateMe: (body: { display_name?: string; password?: string }) =>
    request<MeProfile>("PUT", "/api/users/me", JSON.stringify(body), { "Content-Type": "application/json" }),
  changePassword: (current_password: string, new_password: string) =>
    request<unknown>("POST", "/api/auth/change-password", JSON.stringify({ current_password, new_password }), { "Content-Type": "application/json" }),
  totpStatus: () => request<{ enabled: boolean; has_secret: boolean }>("GET", "/api/auth/totp/status"),
  totpSetup: (password: string) =>
    request<{ secret: string; otpauth_uri: string }>("POST", "/api/auth/totp/setup", JSON.stringify({ password }), { "Content-Type": "application/json" }),
  totpEnable: (password: string, code: string) =>
    request<{ verified: boolean; error?: string }>("POST", "/api/auth/totp/enable", JSON.stringify({ password, code }), { "Content-Type": "application/json" }),
  totpDisable: (password: string, code: string) =>
    request<unknown>("POST", "/api/auth/totp/disable", JSON.stringify({ password, code }), { "Content-Type": "application/json" }),
  getPreferences: () => request<Record<string, unknown>>("GET", "/api/preferences"),
  updatePreferences: (prefs: Record<string, unknown>) =>
    request<unknown>("PUT", "/api/preferences", JSON.stringify(prefs), { "Content-Type": "application/json" }),

  // --- Search ---
  search: (query: string, scope?: string) =>
    request<{ results: { path: string; name: string; score?: number }[]; total?: number; hits?: { path: string; name: string; score?: number }[] }>(
      "GET",
      `/api/search?q=${encodeURIComponent(query)}${scope ? `&scope=${encodeURIComponent(scope)}` : ""}`,
    ),

  // --- Admin: users / groups / branding / maintenance ---
  adminUsers: () => request<{ users: AdminUser[] }>("GET", "/api/admin/users"),
  adminCreateUser: (body: { username: string; display_name: string; email: string; password: string; role?: string }) =>
    request<unknown>("POST", "/api/admin/users", JSON.stringify(body), { "Content-Type": "application/json" }),
  adminDeleteUser: (id: string) => request<unknown>("DELETE", `/api/admin/users/${id}`),
  adminSetRole: (id: string, role: string) =>
    request<unknown>("PUT", `/api/admin/users/${id}/role`, JSON.stringify({ role }), { "Content-Type": "application/json" }),
  adminResetPassword: (id: string, new_password: string) =>
    request<unknown>("POST", `/api/admin/users/${id}/reset-password`, JSON.stringify({ new_password }), { "Content-Type": "application/json" }),
  adminUserDevices: (id: string) => request<{ devices: { id: string; name?: string; device_type?: string; last_seen?: string }[] }>("GET", `/api/admin/users/${id}/devices`),
  adminRevokeDevice: (id: string, deviceId: string) =>
    request<unknown>("DELETE", `/api/admin/users/${id}/devices/${deviceId}/revoke`),

  adminGroups: () => request<{ groups: { id: string; name: string; member_count?: number }[] }>("GET", "/api/groups"),
  adminCreateGroup: (name: string) =>
    request<{ id?: string }>("POST", "/api/groups", JSON.stringify({ name }), { "Content-Type": "application/json" }),
  adminDeleteGroup: (id: string) => request<unknown>("DELETE", `/api/groups/${id}`),
  adminGroupMembers: (id: string) => request<{ members: { id?: string; username: string }[] }>("GET", `/api/groups/${id}/members`),
  adminAddGroupMember: (id: string, username: string) =>
    request<unknown>("POST", `/api/groups/${id}/members/${encodeURIComponent(username)}`),
  adminRemoveGroupMember: (id: string, username: string) =>
    request<unknown>("DELETE", `/api/groups/${id}/members/${encodeURIComponent(username)}`),

  adminBranding: () => request<Record<string, unknown>>("GET", "/api/admin/branding"),
  adminUpdateBranding: (body: Record<string, unknown>) =>
    request<unknown>("PUT", "/api/admin/branding", JSON.stringify(body), { "Content-Type": "application/json" }),
  adminMaintenance: (enabled: boolean) =>
    request<unknown>("POST", "/api/admin/maintenance", JSON.stringify({ enabled }), { "Content-Type": "application/json" }),

  getSpaceMembers: () => request<SpaceMembership[]>("GET", "/api/admin/space-members"),
  putSpaceMembers: (members: SpaceMembership[]) =>
    request<{ ok: boolean; spaces: number; policies: number }>("PUT", "/api/admin/space-members", JSON.stringify(members), { "Content-Type": "application/json" }),

  // Trash
  listTrash: () => request<{ entries: TrashedEntry[] }>("GET", "/api/trash"),
  // NOTE: callers pass transport-ready (already percent-encoded) paths
  // derived from PROPFIND hrefs — do NOT re-encode here.
  // The /api/trash/:path route is a SINGLE axum segment — the whole DAV
  // path must ride inside it with slashes as %2F. Callers pass
  // transport-ready href-derived paths, so decode first to avoid %25.
  trashPath: (path: string) => davJson("DELETE", `/api/trash/${encodeURIComponent(decodeURIComponent(path))}`),
  restoreTrash: (originalPath: string) => davJson("POST", "/api/trash/restore", JSON.stringify({ original_path: originalPath })),
  purgeTrash: (originalPath: string) => davJson("DELETE", "/api/trash/purge", JSON.stringify({ original_path: originalPath })),
  emptyTrash: () => davJson("DELETE", "/api/trash/empty"),

  // Bulk
  // Hard delete (not trash). Paths are virtual decoded paths.
  bulkTrash: (paths: string[]) => davJson("POST", "/api/bulk/delete", JSON.stringify({ paths: paths.map((p) => { try { return decodeURIComponent(p); } catch { return p; } }) })),

  // Shares
  listShares: () => request<{ shares: ShareLink[] }>("GET", "/api/shares"),
  createShare: (req: { path: string; password?: string; expires_in_hours?: number; share_type?: string; allow_download?: boolean }) =>
    davJson("POST", "/api/shares", JSON.stringify(req)).then((r) => r.json() as Promise<ShareLink>),
  deleteShare: (token: string) => davJson("DELETE", `/api/shares/${encodeURIComponent(token)}`),
};

async function davJson(method: string, path: string, body?: string): Promise<Response> {
  const doFetch = () => fetch(path, { method, headers: { ...authHeaders(), ...(body ? { "Content-Type": "application/json" } : {}) }, body });
  let resp = await doFetch();
  if (resp.status === 401 && (await refreshAccessToken())) resp = await doFetch();
  if (!resp.ok) {
    let msg = `HTTP ${resp.status}`;
    try { const j = await resp.json(); msg = j.message ?? j.error ?? msg; } catch { /* ignore */ }
    throw new ApiError(resp.status, msg);
  }
  return resp;
}

export interface MeProfile {
  id?: string; sub?: string; username?: string; display_name?: string;
  email?: string; role?: string; totp_enabled?: boolean;
}
export interface AdminUser {
  id: string; username: string; role: string; created_at: string;
  last_login: string | null; file_count: number; total_size: number; email?: string;
}
export interface SpaceMembership {
  space: string; viewers: string[]; editors: string[]; managers: string[];
}
export interface AdminStats {
  version: string; uptime_seconds: number; total_files: number;
  total_directories: number; total_bytes: number; storage_backend: string;
  auth_type: string; wasm_workers_loaded: number; search_enabled: boolean;
}
export interface AuditEntry {
  timestamp: string; method: string; path: string; user: string;
  status: number; client_ip: string; user_agent: string;
}
export interface Photo {
  id: string; path: string; name: string; size: number; mime_type: string;
  taken_at: string | null; modified_at: string; width: number | null; height: number | null;
}
export interface Board { id: string; name: string; updated_at: string; created_at: string; }
export interface ChatRoom { id: string; name: string; room_type: string; created_at: string; }
export interface ChatMsg {
  id: string; room_id: string; user_id: string; content: string;
  timestamp: string; reply_to: string | null; attachment_path: string | null;
}
export interface Contact {
  uid: string; address_book_id: string; vcard_data: string; etag: string;
  created_at: string; updated_at: string;
}
export interface CalEvent {
  uid: string; calendar_id: string; ical_data: string; etag: string;
  created_at: string; updated_at: string;
}
export interface Note {
  id: string; title: string; content: string; folder: string; tags: string;
  created_at: string; updated_at: string;
}
export interface Task {
  id: string; title: string; description: string; status: string;
  assignee: string; due_date: string | null; priority: string; tags: string;
  created_at: string; updated_at: string;
}
export interface TrashedEntry { original_path: string; deleted_at: string; size: number; mime_type: string; }
export interface ShareLink {
  token: string; path: string; password: string | null;
  expires_at: string; max_downloads: number | null; download_count: number;
  created_by: string; allow_download: boolean | null; allow_upload: boolean | null;
}

async function ensureOk(resp: Response): Promise<Response> {
  if (!resp.ok) throw new ApiError(resp.status, `${resp.status} ${resp.statusText}`);
  return resp;
}
