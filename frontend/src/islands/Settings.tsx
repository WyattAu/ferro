import { createSignal, createResource, For, Show, Switch, Match } from "solid-js";
import { api } from "../lib/api";
import { Loader2, UserCog, KeyRound, ShieldCheck, SlidersHorizontal, Check } from "lucide-solid";

function Profile() {
  const [me, { refetch }] = createResource(async () => api.getMe());
  const [quota] = createResource(async () => api.getQuota().catch(() => null));
  const [name, setName] = createSignal("");
  const [msg, setMsg] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const save = async () => {
    setBusy(true); setMsg(null);
    try { await api.updateMe({ display_name: name().trim() || undefined }); setMsg("Saved"); refetch(); }
    catch (e) { setMsg(e instanceof Error ? e.message : String(e)); }
    finally { setBusy(false); }
  };
  return (
    <div class="max-w-md space-y-4">
      <Show when={me()} fallback={<Loader2 size={20} class="animate-spin text-[var(--text-tertiary)]" />}>
        {(m) => (
          <>
            <div class="glass rounded-xl p-4 space-y-3">
              <label class="block text-xs text-[var(--text-secondary)]">Display name
                <input value={name() || m().display_name || ""} onInput={(e) => setName(e.currentTarget.value)}
                  class="mt-1 w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
              </label>
              <div class="text-xs text-[var(--text-tertiary)]">Username: {m().username} · Email: {m().email ?? "—"}</div>
              <Show when={quota()}>
                {(q) => (
                  <div class="text-xs text-[var(--text-tertiary)]">
                    Storage: {q().unlimited
                      ? `${(q().used_bytes / 1048576).toFixed(1)} MB used · unlimited`
                      : `${(q().used_bytes / 1048576).toFixed(1)} / ${q().quota_bytes ? (q().quota_bytes / 1048576).toFixed(0) + " MB" : "—"}`}
                    <div class="w-full h-1.5 mt-1 bg-[var(--bg-raised)] rounded-full overflow-hidden">
                      <div class="h-full bg-[var(--accent)]" style={{ width: `${Math.min(q().used_percent, 100)}%` }} />
                    </div>
                    <div class="mt-1">{q().file_count} files</div>
                  </div>
                )}
              </Show>
              <button onClick={save} disabled={busy()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">Save</button>
              <Show when={msg()}><p class="text-xs text-[var(--ok)]">{msg()}</p></Show>
            </div>
          </>
        )}
      </Show>
    </div>
  );
}

function Security() {
  const [status] = createResource(async () => api.totpStatus().catch(() => null));
  const [me] = createResource(async () => api.authInfo().catch(() => null));
  const isOidc = () => (me()?.auth_type ?? "oidc") === "oidc";

  // Passkeys: register a credential for local-account passkey login.
  const b64ToBuffer = (b64: string): ArrayBuffer => {
    const s = b64.replace(/-/g, "+").replace(/_/g, "/");
    const raw = atob(s + "=".repeat((4 - (s.length % 4)) % 4));
    return Uint8Array.from(raw, (c) => c.charCodeAt(0)).buffer;
  };
  const [pkMsg, setPkMsg] = createSignal<string | null>(null);
  const registerPasskey = async () => {
    setPkMsg(null);
    try {
      const username = "local"; // server ties credentials to the local account
      const begin = await fetch("/api/auth/webauthn/register/begin", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username }),
      }).then((r) => r.json());
      const options = begin.publicKey ?? begin;
      options.challenge = b64ToBuffer(options.challenge);
      options.user.id = b64ToBuffer(options.user.id);
      if (options.excludeCredentials) {
        options.excludeCredentials = options.excludeCredentials.map((c: { id: string }) => ({ ...c, id: b64ToBuffer(c.id) }));
      }
      const credential = (await navigator.credentials.create({ publicKey: options })) as PublicKeyCredential;
      if (!credential) throw new Error("no credential returned");
      if (!credential) throw new Error("no credential returned");
      const resp = credential.response as AuthenticatorAttestationResponse;
      const finishBody = {
        username,
        challenge_id: begin.challenge_id ?? begin.challengeId,
        credential_id: btoa(String.fromCharCode(...new Uint8Array(credential.rawId)))
          .replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""),
        client_data_json: btoa(String.fromCharCode(...new Uint8Array(resp.clientDataJSON)))
          .replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""),
        attestation_object: btoa(String.fromCharCode(...new Uint8Array(resp.attestationObject)))
          .replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""),
      };
      const finish = await fetch("/api/auth/webauthn/register/finish", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(finishBody),
      }).then((r) => r.json());
      setPkMsg(finish.success ? "Passkey registered" : finish.error ?? "Registration failed");
    } catch (e) {
      setPkMsg(e instanceof Error ? e.message : String(e));
    }
  };
  const [cur, setCur] = createSignal(""); const [nw, setNw] = createSignal("");
  const [pwMsg, setPwMsg] = createSignal<string | null>(null);
  const [secret, setSecret] = createSignal<{ secret: string; otpauth_uri: string } | null>(null);
  const [code, setCode] = createSignal("");
  const [totpPw, setTotpPw] = createSignal("");
  const [totpMsg, setTotpMsg] = createSignal<string | null>(null);

  const changePw = async () => {
    setPwMsg(null);
    try { await api.changePassword(cur(), nw()); setPwMsg("Password changed"); setCur(""); setNw(""); }
    catch (e) { setPwMsg(e instanceof Error ? e.message : String(e)); }
  };
  const startTotp = async () => {
    setTotpMsg(null);
    try { setSecret(await api.totpSetup(totpPw())); }
    catch (e) { setTotpMsg(e instanceof Error ? e.message : String(e)); }
  };
  const enableTotp = async () => {
    setTotpMsg(null);
    try {
      const r = await api.totpEnable(totpPw(), code());
      setTotpMsg(r.verified ? "2FA enabled" : r.error ?? "Invalid code");
      if (r.verified) { setSecret(null); setCode(""); }
    } catch (e) { setTotpMsg(e instanceof Error ? e.message : String(e)); }
  };

  return (
    <div class="max-w-md space-y-4">
      <Show when={isOidc()}>
        <div class="glass rounded-xl p-4 space-y-2">
          <h3 class="text-sm font-medium">Managed by your identity provider</h3>
          <p class="text-xs text-[var(--text-secondary)]">
            Password and two-factor authentication for this account live in Keycloak.
          </p>
          <a href="https://auth.wyattau.com/realms/company-realm/account" target="_blank" rel="noreferrer"
            class="inline-block px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">
            Open Keycloak account
          </a>
        </div>
      </Show>
      <Show when={!isOidc()}>
      <div class="glass rounded-xl p-4 space-y-3">
        <h3 class="text-sm font-medium flex items-center gap-2"><KeyRound size={14} /> Change password</h3>
        <input type="password" value={cur()} onInput={(e) => setCur(e.currentTarget.value)} placeholder="Current password"
          class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
        <input type="password" value={nw()} onInput={(e) => setNw(e.currentTarget.value)} placeholder="New password"
          class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
        <button onClick={changePw} disabled={!cur() || !nw()}
          class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">Update</button>
        <Show when={pwMsg()}><p class="text-xs text-[var(--ok)]">{pwMsg()}</p></Show>
      </div>

      <div class="glass rounded-xl p-4 space-y-3">
        <h3 class="text-sm font-medium flex items-center gap-2"><ShieldCheck size={14} /> Two-factor (TOTP)</h3>
        <p class="text-xs text-[var(--text-tertiary)]">Status: {status()?.enabled ? "enabled" : "disabled"}</p>
        <input type="password" value={totpPw()} onInput={(e) => setTotpPw(e.currentTarget.value)} placeholder="Confirm password"
          class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
        <Show when={!secret()}>
          <button onClick={startTotp} disabled={!totpPw()} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white disabled:opacity-40">Begin setup</button>
        </Show>
        <Show when={secret()}>
          <p class="text-xs text-[var(--text-secondary)]">Add this secret to your authenticator app:</p>
          <code class="block text-xs font-mono bg-[var(--bg-surface)] rounded-lg px-3 py-2 break-all">{secret()!.secret}</code>
          <input value={code()} onInput={(e) => setCode(e.currentTarget.value)} placeholder="6-digit code" inputmode="numeric"
            class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-1.5 text-sm outline-none focus:border-[var(--accent)]" />
          <button onClick={enableTotp} disabled={code().length < 6} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--ok)] text-white disabled:opacity-40">Verify & enable</button>
        </Show>
        <Show when={totpMsg()}><p class="text-xs text-[var(--ok)]">{totpMsg()}</p></Show>

      <div class="glass rounded-xl p-4 space-y-3">
        <h3 class="text-sm font-medium">Passkeys (local accounts)</h3>
        <button onClick={registerPasskey}
          class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Register passkey</button>
        <Show when={pkMsg()}><p class="text-xs text-[var(--text-secondary)]">{pkMsg()}</p></Show>
      </div>
      </div>
      </Show>
    </div>
  );
}

function Preferences() {
  const [prefs, setPrefs] = createSignal<Record<string, unknown> | null>(null);
  const [raw, setRaw] = createSignal("{}");
  const [msg, setMsg] = createSignal<string | null>(null);
  createResource(async () => {
    const p = await api.getPreferences();
    setPrefs(p); setRaw(JSON.stringify(p, null, 2));
    return p;
  });
  const save = async () => {
    setMsg(null);
    try {
      const parsed = JSON.parse(raw()) as Record<string, unknown>;
      await api.updatePreferences(parsed); setPrefs(parsed); setMsg("Saved");
    } catch (e) { setMsg(e instanceof Error ? e.message : String(e)); }
  };
  return (
    <div class="max-w-lg space-y-3">
      <div class="text-xs text-[var(--text-secondary)] flex items-center gap-2"><SlidersHorizontal size={13} /> Raw preferences JSON</div>
      <textarea value={raw()} onInput={(e) => setRaw(e.currentTarget.value)} rows={10} spellcheck={false}
        class="w-full bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-lg px-3 py-2 text-xs font-mono outline-none focus:border-[var(--accent)]" />
      <button onClick={save} class="px-3 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white">Save</button>
      <Show when={msg()}><p class="text-xs text-[var(--ok)]">{msg()}</p></Show>
    </div>
  );
}

const TABS = [
  { id: "profile", label: "Profile", icon: UserCog },
  { id: "security", label: "Security", icon: ShieldCheck },
  { id: "prefs", label: "Preferences", icon: SlidersHorizontal },
] as const;

export default function SettingsPage() {
  const [tab, setTab] = createSignal<(typeof TABS)[number]["id"]>("profile");
  return (
    <div class="p-6 max-w-3xl mx-auto h-full flex flex-col">
      <h1 class="text-xl font-semibold mb-4 shrink-0">Settings</h1>
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
          <Match when={tab() === "profile"}><Profile /></Match>
          <Match when={tab() === "security"}><Security /></Match>
          <Match when={tab() === "prefs"}><Preferences /></Match>
        </Switch>
      </div>
    </div>
  );
}
