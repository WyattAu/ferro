use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::*;

use crate::auth;
use crate::components::theme_toggle::{ThemeToggle, provide_theme_state};
use crate::t;

#[component]
pub fn LoginPage() -> impl IntoView {
    provide_theme_state();

    // The server reports how it authenticates: "oidc", "basic", or "none".
    // Basic mode gets a local username/password form; OIDC delegates to the
    // configured identity provider.
    #[cfg(target_arch = "wasm32")]
    {
        leptos::task::spawn_local(async move {
            if let Ok(cfg) = crate::api::get_auth_config().await {
                auth::set_login_page_mode(&cfg.auth_mode);
            }
        });
    }

    let (auth_mode, _) = auth::login_page_mode_signal();
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(String::new());
    let (busy, set_busy) = signal(false);

    let on_login = move |_: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        auth::start_login();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = ();
        }
    };

    let on_basic_login = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        set_error.set(String::new());
        if busy.get() {
            return;
        }
        let header = match auth::basic_auth_header(username.get().trim(), &password.get()) {
            Ok(h) => h,
            Err(e) => {
                set_error.set(e);
                return;
            }
        };
        set_busy.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_error = set_error.clone();
            let set_busy = set_busy.clone();
            leptos::task::spawn_local(async move {
                // Verify the credentials against an authenticated endpoint
                // before storing them (/api/auth/info is public and useless
                // as a probe). Only persist on success so a reload can never
                // loop back to the login page with dead credentials.
                if crate::api::verify_basic_auth(&header).await {
                    auth::store_basic_auth(&header);
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/ui/");
                    }
                } else {
                    set_error.set("Invalid username or password.".to_string());
                    set_busy.set(false);
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (set_error, set_busy, header);
        }
    };

    view! {
        <div class="min-h-screen bg-[var(--bg-base)] flex items-center justify-center relative">
            <div class="absolute top-4 right-4">
                <ThemeToggle />
            </div>
            <div class="brutal-block rounded-lg shadow-2xl p-8 max-w-md w-full text-center">
                <div class="flex items-center justify-center gap-2 mb-8">
                    <div class="w-10 h-10 bg-transparent brutal-border rounded flex items-center justify-center font-display text-display text-accent">{t!("brand.name")}</div>
                    <div class="text-left">
                        <h1 class="text-xl font-bold font-mono text-[var(--text-primary)] leading-none">{t!("brand.name")}</h1>
                        <span class="text-label text-muted">{t!("brand.tagline")}</span>
                    </div>
                </div>

                <h2 class="text-section font-mono text-[var(--text-primary)] mb-2">{t!("login.welcome")}</h2>
                <p class="text-muted font-mono mb-6">{t!("login.description")}</p>

                <Show
                    when=move || auth_mode.get() == "basic"
                    fallback=move || {
                        view! {
                            <button
                                class="w-full bg-[var(--accent)] text-[var(--text-on-accent)] px-4 py-3 rounded-sm hover:bg-[var(--accent-hover)] transition-colors font-bold uppercase tracking-widest brutal-border shadow-iron focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                                on:click=on_login
                            >
                                {t!("common.sign_in")}
                            </button>
                        }
                    }
                >
                    <form class="flex flex-col gap-3 text-left" on:submit=on_basic_login>
                        <label class="text-label font-mono text-[var(--text-secondary)]">
                            "Username"
                            <input
                                type="text"
                                autocomplete="username"
                                class="mt-1 w-full px-3 py-2 bg-[var(--bg-inset)] text-[var(--text-primary)] brutal-border rounded-sm font-mono text-sm focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                                placeholder="admin"
                                prop:value=move || username.get()
                                on:input=move |e| set_username.set(event_target_value(&e))
                            />
                        </label>
                        <label class="text-label font-mono text-[var(--text-secondary)]">
                            "Password"
                            <input
                                type="password"
                                autocomplete="current-password"
                                class="mt-1 w-full px-3 py-2 bg-[var(--bg-inset)] text-[var(--text-primary)] brutal-border rounded-sm font-mono text-sm focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                                prop:value=move || password.get()
                                on:input=move |e| set_password.set(event_target_value(&e))
                            />
                        </label>
                        <Show when=move || !error.get().is_empty()>
                            <p class="text-sm font-mono text-red-500">{move || error.get()}</p>
                        </Show>
                        <button
                            type="submit"
                            disabled=move || busy.get()
                            class="w-full mt-2 bg-[var(--accent)] text-[var(--text-on-accent)] px-4 py-3 rounded-sm hover:bg-[var(--accent-hover)] disabled:opacity-50 transition-colors font-bold uppercase tracking-widest brutal-border shadow-iron focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                        >
                            {if busy.get() { "Signing in…".to_string() } else { t!("common.sign_in").to_string() }}
                        </button>
                    </form>
                </Show>

                <A
                    href="/"
                    attr:class="block mt-4 text-sm text-muted font-mono hover:text-[var(--text-secondary)] no-underline focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] rounded text-label"
                >
                    {t!("common.skip_for_now")}
                </A>
            </div>
        </div>
    }
}
