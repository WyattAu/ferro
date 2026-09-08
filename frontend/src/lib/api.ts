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

export async function propfind(path: string, depth = "1"): Promise<FileEntry[]> {
  const resp = await fetch(`/users/${path}`, { method: "PROPFIND", headers: { Depth: depth, ...authHeaders() } });
  if (resp.status === 401) {
    if (await refreshAccessToken()) return propfind(path, depth);
    clearTokens(); redirectToLogin();
    throw new ApiError(401, "Unauthorized");
  }
  if (!resp.ok) throw new ApiError(resp.status, `PROPFIND failed: ${resp.status}`);
  const xml = await resp.text();
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  const responses = Array.from(doc.getElementsByTagNameNS("DAV:", "response"));
  const current = decodeHref(`/users/${path}`).replace(/\/$/, "");
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

function davUrl(path: string) { return `/users/${path}`; }

export async function davRequest(method: string, path: string, extraHeaders: Record<string, string> = {}, body?: BodyInit): Promise<Response> {
  const doFetch = () => fetch(davUrl(path), { method, headers: { ...authHeaders(), ...extraHeaders }, body });
  let resp = await doFetch();
  if (resp.status === 401 && (await refreshAccessToken())) resp = await doFetch();
  return resp;
}

function destinationHeader(toPath: string): Record<string, string> {
  return { Destination: `${window.location.origin}/users/${toPath}` };
}

export async function uploadFile(path: string, file: File, onProgress?: (pct: number) => void): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("PUT", davUrl(path));
    const token = getToken();
    if (token) xhr.setRequestHeader("Authorization", `Bearer ${token}`);
    xhr.upload.onprogress = (e) => { if (e.lengthComputable && onProgress) onProgress(Math.round((e.loaded / e.total) * 100)); };
    xhr.onload = () => xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new ApiError(xhr.status, `Upload failed: ${xhr.status}`));
    xhr.onerror = () => reject(new ApiError(0, "Network error"));
    xhr.send(file);
  });
}

export function downloadUrl(path: string): string {
  const token = getToken();
  return token ? `${davUrl(path)}?Authorization=${encodeURIComponent(`Bearer ${token}`)}` : davUrl(path);
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
  delete: (path: string) => davRequest("DELETE", path).then(ensureOk),
  mkdir: (path: string) => davRequest("MKCOL", path).then(ensureOk),
  move: (from: string, to: string) => davRequest("MOVE", from, destinationHeader(to)).then(ensureOk),
  copy: (from: string, to: string) => davRequest("COPY", from, destinationHeader(to)).then(ensureOk),
  downloadUrl,
  fetchText: (path: string) => fetch(davUrl(path), { headers: authHeaders() }).then((r) => r.ok ? r.text() : Promise.reject(new ApiError(r.status, "Fetch failed"))),
  fetchBlob: (path: string) => fetch(davUrl(path), { headers: authHeaders() }).then((r) => r.ok ? r.blob() : Promise.reject(new ApiError(r.status, "Fetch failed"))),
  listTasks: () => request<{ tasks: unknown[]; total: number }>("GET", "/api/tasks"),

  // Trash
  listTrash: () => request<{ entries: TrashedEntry[] }>("GET", "/api/trash"),
  trashPath: (path: string) => davJson("DELETE", `/api/trash/${encodePath(path)}`),
  restoreTrash: (originalPath: string) => davJson("POST", "/api/trash/restore", JSON.stringify({ original_path: originalPath })),
  purgeTrash: (originalPath: string) => davJson("DELETE", "/api/trash/purge", JSON.stringify({ original_path: originalPath })),
  emptyTrash: () => davJson("DELETE", "/api/trash/empty"),

  // Bulk
  bulkTrash: (paths: string[]) => davJson("POST", "/api/bulk/delete", JSON.stringify({ paths })),

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

function encodePath(p: string): string {
  return p.split("/").map(encodeURIComponent).join("/");
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
