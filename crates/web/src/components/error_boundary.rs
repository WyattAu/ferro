use leptos::*;

#[component]
pub fn ErrorBoundary(children: Children) -> impl IntoView {
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let (error_msg, set_error_msg) = create_signal(None::<String>);

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let set_err = set_error_msg;
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |msg: String,
                          _source: String,
                          _lineno: u32,
                          _colno: u32,
                          _error: wasm_bindgen::JsValue| {
                        set_err.set(Some(msg));
                    },
                )
                    as Box<dyn Fn(String, String, u32, u32, wasm_bindgen::JsValue)>);
                let _ = window.set_onerror(Some(cb.as_ref().unchecked_ref()));
                cb.forget();

                let set_err2 = set_err;
                let cb2 = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |ev: wasm_bindgen::JsValue| {
                        let msg = js_sys::JSON::stringify(&ev)
                            .map(|s| s.as_string().unwrap_or_default())
                            .unwrap_or_else(|_| "Unknown promise rejection".to_string());
                        set_err2.set(Some(msg));
                    },
                )
                    as Box<dyn Fn(wasm_bindgen::JsValue)>);
                let _ = window.add_event_listener_with_callback(
                    "unhandledrejection",
                    cb2.as_ref().unchecked_ref(),
                );
                cb2.forget();
            }
        });
    }

    view! {
        {move || error_msg.get().map(|msg| view! {
            <div class="fixed inset-0 z-[9999] bg-gray-100 dark:bg-gray-900 flex items-center justify-center p-4">
                <div class="max-w-md w-full text-center">
                    <svg class="w-16 h-16 text-red-500 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.832c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
                    </svg>
                    <h1 class="text-xl font-bold text-gray-900 dark:text-gray-100 mb-2">"Something went wrong"</h1>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-6">
                        "An unexpected error occurred. Please try reloading the page."
                    </p>
                    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mb-6 text-left">
                        <p class="text-xs font-mono text-red-600 dark:text-red-400 break-all max-h-32 overflow-y-auto">{msg}</p>
                    </div>
                    <button
                        class="px-6 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900"
                        on:click=move |_| {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().reload();
                            }
                        }
                    >
                        "Reload"
                    </button>
                </div>
            </div>
        })}
        {children()}
    }
}
