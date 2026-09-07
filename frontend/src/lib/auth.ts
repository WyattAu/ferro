import { api, clearToken, storeToken } from "./api";

export async function login(redirect = window.location.pathname) {
  window.location.href = `/api/auth/login?redirect=${encodeURIComponent(redirect)}`;
}

export async function handleOAuthCallback(): Promise<{ ok: boolean; error?: string }> {
  const params = new URLSearchParams(window.location.search);
  const code = params.get("code");
  const state = params.get("state");
  if (!code || !state) return { ok: false, error: "Missing code or state" };
  try {
    const resp = await api.callback(code, state);
    storeToken(resp.access_token);
    const target = resp.redirect && resp.redirect !== "/" ? resp.redirect : "/ui/files/";
    window.location.href = target;
    return { ok: true };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

export function logout(logoutUrl?: string | null) {
  clearToken();
  if (logoutUrl) { window.location.href = logoutUrl; return; }
  window.location.href = "/api/auth/login?redirect=%2Fui%2Ffiles%2F";
}
