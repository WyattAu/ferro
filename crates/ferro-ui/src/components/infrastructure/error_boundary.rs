use leptos::prelude::*;

/// Top-level error boundary that catches render panics and shows recovery UI.
#[component]
pub fn ErrorBoundary(children: Children) -> impl IntoView {
    let (error_msg, set_error_msg) = signal(None::<String>);

    // Catch JS errors
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        let set_err = set_error_msg;
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |msg: String, _src: String, _l: u32, _c: u32, _e: JsValue| {
                        set_err.set(Some(msg));
                    },
                )
                    as Box<dyn Fn(String, String, u32, u32, JsValue)>);
                let _ = window.set_onerror(Some(cb.as_ref().unchecked_ref()));
                cb.forget();
            }
        });
    }

    view! {
        {move || error_msg.get().map(|msg| view! {
            <div class="error-overlay" role="alertdialog" aria-modal="true">
                <div class="error-card">
                    <h2 class="error-title">"Something went wrong"</h2>
                    <p class="error-message">{msg}</p>
                    <button
                        class="btn btn-primary"
                        on:click=move |_| {
                            if let Some(w) = web_sys::window() {
                                let _ = w.location().reload();
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
