use leptos::prelude::*;

use crate::auth;
use crate::t;
use crate::utils::urlencoding_decode;

fn parse_query_string(search: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let query = search.trim_start_matches('?');
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            map.insert(urlencoding_decode(key), urlencoding_decode(value));
        }
    }
    map
}

#[component]
pub fn AuthCallbackPage() -> impl IntoView {
    let (error, set_error) = signal(None::<String>);
    let (processing, set_processing) = signal(true);

    let state = auth::use_auth_state();

    Effect::new(move |_| {
        let search = web_sys::window()
            .and_then(|w| {
                let loc = w.location();
                loc.search().ok()
            })
            .unwrap_or_default();

        let params = parse_query_string(&search);
        let code = params.get("code").cloned().unwrap_or_default();
        let cb_state = params.get("state").cloned().unwrap_or_default();

        if code.is_empty() {
            set_error.set(Some(t!("error.minus_auth_code").to_string()));
            set_processing.set(false);
            return;
        }

        auth::handle_callback(&state, &code, &cb_state);
    });

    view! {
        <div class="min-h-screen bg-[var(--bg-inset)] flex items-center justify-center">
            <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm p-8 max-w-md w-full text-center">
                {move || processing.get().then(|| view! {
                    <div>
                        <div class="w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
                        <p class="text-[var(--text-secondary)]">{t!("error.completing_sign_in")}</p>
                    </div>
                })}

                {move || error.get().map(|e| view! {
                    <div>
                        <div class="w-12 h-12 bg-[var(--danger-subtle)] rounded-full flex items-center justify-center mx-auto mb-4">
                            <svg class="w-6 h-6 text-[var(--danger)]" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </div>
                        <h2 class="text-lg font-semibold text-[var(--text-primary)] mb-2">{t!("error.sign_in_failed")}</h2>
                        <p class="text-[var(--text-tertiary)] mb-4">{e}</p>
                        <a href="/ui/auth/login" class="inline-block bg-[var(--accent)] text-[var(--text-on-accent)] px-4 py-2 rounded-lg hover:bg-[var(--accent-hover)] no-underline">
                            {t!("common.try_again")}
                        </a>
                    </div>
                })}
            </div>
        </div>
    }
}
