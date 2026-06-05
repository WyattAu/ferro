use leptos::*;

use crate::auth;

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

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[component]
pub fn AuthCallbackPage() -> impl IntoView {
    let (error, set_error) = create_signal(None::<String>);
    let (processing, set_processing) = create_signal(true);

    let state = auth::use_auth_state();

    create_effect(move |_| {
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
            set_error.set(Some("Missing authorization code".to_string()));
            set_processing.set(false);
            return;
        }

        auth::handle_callback(&state, &code, &cb_state);
    });

    view! {
        <div class="min-h-screen bg-gray-100 flex items-center justify-center">
            <div class="bg-white rounded-xl shadow-sm p-8 max-w-md w-full text-center">
                {move || processing.get().then(|| view! {
                    <div>
                        <div class="w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
                        <p class="text-gray-600">"Completing sign in..."</p>
                    </div>
                })}

                {move || error.get().map(|e| view! {
                    <div>
                        <div class="w-12 h-12 bg-red-100 rounded-full flex items-center justify-center mx-auto mb-4">
                            <svg class="w-6 h-6 text-red-600" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </div>
                        <h2 class="text-lg font-semibold text-gray-900 mb-2">"Sign in failed"</h2>
                        <p class="text-gray-500 mb-4">{e}</p>
                        <a href="/ui/auth/login" class="inline-block bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 no-underline">
                            "Try again"
                        </a>
                    </div>
                })}
            </div>
        </div>
    }
}
