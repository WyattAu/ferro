const BASE = "";
const STORAGE_KEY = "ferro_access_token";

export function getToken(): string | null {
  try { return sessionStorage.getItem(STORAGE_KEY) ?? localStorage.getItem(STORAGE_KEY); }
  catch { return null; }
}
export function storeToken(t: string) { localStorage.setItem(STORAGE_KEY, t); }
export function clearToken() { localStorage.removeItem(STORAGE_KEY); }

export class ApiError extends Error {
  constructor(public status: number, message: string) { super(message); }
}

async function request<T>(method: string, path: string, body?: BodyInit, headers: Record<string, string> = {}): Promise<T> {
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  const resp = await fetch(path, { method, headers, body });
  if (resp.status === 401) {
    clearToken();
    window.location.href = `/api/auth/login?redirect=${encodeURIComponent(window.location.pathname)}`;
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
export interface CallbackResponse { access_token: string; token_type: string; expires_in: number; user: { sub: string; email?: string | null; name?: string | null }; redirect: string; logout_url: string; }

export interface FileEntry {
  href: string; name: string; isCollection: boolean;
  size: number | null; mimeType: string | null; modifiedAt: string | null;
}

function decodeHref(href: string): string {
  try { return decodeURIComponent(href); } catch { return href; }
}

export async function propfind(path: string, depth = "1"): Promise<{ entries: FileEntry[] }> {
  const token = getToken();
  const resp = await fetch(`/users/${path}`, {
    method: "PROPFIND",
    headers: { Depth: depth, ...(token ? { Authorization: `Bearer ${token}` } : {}) },
  });
  if (resp.status === 401) { clearToken(); window.location.href = `/api/auth/login?redirect=${encodeURIComponent("/ui/files/")}`; throw new ApiError(401, "Unauthorized"); }
  if (!resp.ok) throw new ApiError(resp.status, `PROPFIND failed: ${resp.status}`);
  const xml = await resp.text();
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  const responses = Array.from(doc.getElementsByTagNameNS("DAV:", "response"));
  const prefix = `/users/${path.split("/")[0]}`;
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
  return { entries };
}

function davUrl(path: string) { return `/users/${path}`; }

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

export const api = {
  authInfo: () => request<AuthInfo>("GET", "/api/auth/info"),
  callback: (code: string, state: string) =>
    request<CallbackResponse>("GET", `/api/auth/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`),
  delete: (path: string) => request<void>("DELETE", davUrl(path)),
  mkdir: (path: string) => request<void>("MKCOL", davUrl(path)),
  downloadUrl: (path: string) => davUrl(path),
  listTasks: () => request<{ tasks: unknown[]; total: number }>("GET", "/api/tasks"),
};
