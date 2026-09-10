// Typed API client for Ferro server
const BASE = import.meta.env.PUBLIC_FERRO_URL || "";

let token: string | null = localStorage.getItem("ferro_token");

export function setToken(t: string) { token = t; localStorage.setItem("ferro_token", t); }
export function clearToken() { token = null; localStorage.removeItem("ferro_token"); }
export function getToken() { return token; }

async function request<T>(method: string, path: string, body?: BodyInit, headers?: Record<string, string>): Promise<T> {
  const h: Record<string, string> = { ...headers };
  if (token) h["Authorization"] = `Bearer ${token}`;
  if (body && !h["Content-Type"] && body instanceof Blob) h["Content-Type"] = "application/octet-stream";

  const resp = await fetch(`${BASE}${path}`, { method, headers: h, body });
  if (resp.status === 401) { window.location.href = "/api/auth/login?redirect=" + encodeURIComponent(window.location.pathname); throw new Error("Unauthorized"); }
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
  const ct = resp.headers.get("content-type") || "";
  if (ct.includes("json")) return resp.json();
  return resp.text() as unknown as T;
}

export interface FileEntry { path: string; name: string; size: number; is_collection: boolean; mime_type: string; modified_at: string; }
export interface Task { id: string; owner: string; title: string; description: string; status: string; assignee: string; due_date: string | null; priority: string; tags: string; created_at: string; updated_at: string; }

export const api = {
  // Files (WebDAV PROPFIND)
  async listFiles(path: string): Promise<FileEntry[]> {
    const text = await request<string>("PROPFIND", `/remote.php/dav/files/wyatt${path}`, undefined, { Depth: "1" });
    const doc = new DOMParser().parseFromString(text, "text/xml");
    return Array.from(doc.querySelectorAll("D\\:response")).map(r => {
      const href = r.querySelector("D\\:href")?.textContent || "";
      const isCollection = !!r.querySelector("D\\:collection");
      const size = parseInt(r.querySelector("D\\:getcontentlength")?.textContent || "0");
      const modified = r.querySelector("D\\:getlastmodified")?.textContent || "";
      const name = decodeURIComponent(href).split("/").filter(Boolean).pop() || "/";
      return { path: decodeURIComponent(href), name, size, is_collection: isCollection, mime_type: "", modified_at: modified };
    }).filter(e => e.path !== decodeURIComponent(`/remote.php/dav/files/wyatt${path}`).replace(/\/$/, ""));
  },
  uploadFile: (path: string, file: File, onProgress?: (pct: number) => void) =>
    new Promise<void>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("PUT", `${BASE}/remote.php/dav/files/wyatt${path}`);
      if (token) xhr.setRequestHeader("Authorization", `Bearer ${token}`);
      xhr.upload.onprogress = (e) => { if (e.lengthComputable && onProgress) onProgress(Math.round(e.loaded / e.total * 100)); };
      xhr.onload = () => xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`HTTP ${xhr.status}`));
      xhr.onerror = () => reject(new Error("Network error"));
      xhr.send(file);
    }),
  deleteFile: (path: string) => request<void>("DELETE", `/remote.php/dav/files/wyatt${path}`),
  createFolder: (path: string) => request<void>("MKCOL", `/remote.php/dav/files/wyatt${path}`),
  downloadFile: (path: string) => `${BASE}/remote.php/dav/files/wyatt${path}`,

  // Tasks
  listTasks: () => request<{ tasks: Task[]; total: number }>("GET", "/api/tasks"),
  createTask: (body: Partial<Task>) => request<{ task: Task }>("POST", "/api/tasks", JSON.stringify(body), { "Content-Type": "application/json" }),
  updateTaskStatus: (id: string, status: string) => request<void>("PATCH", `/api/tasks/${id}/status`, JSON.stringify({ status }), { "Content-Type": "application/json" }),
  deleteTask: (id: string) => request<void>("DELETE", `/api/tasks/${id}`),

  // Auth
  me: () => request<{ sub: string; email: string | null; name: string | null }>("GET", "/api/auth/info"),
};
