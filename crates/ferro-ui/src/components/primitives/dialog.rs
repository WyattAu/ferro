use leptos::prelude::*;

/// Modal dialog component. Renders once, visibility controlled by signal.
/// Accessibility: role="dialog", aria-modal="true", focus-trapped.
#[component]
pub fn Dialog(
    #[prop(into)] open: Signal<bool>,
    #[prop(into, optional)] title: String,
    #[prop(optional)] class: String,
    #[prop(optional)] on_close: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let cls = format!("dialog {class}");

    let dialog_ref = NodeRef::<html::Div>::new();
    // Auto-focus dialog when opened
    {
        let dialog_ref = dialog_ref.clone();
        Effect::new(move |_| {
            if open.get() {
                let dialog_ref = dialog_ref.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    // Small delay for overlay transition via setTimeout(10)
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        let _ = web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10);
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    if let Some(el) = dialog_ref.get() {
                        use wasm_bindgen::JsCast;
                        let _ = el.unchecked_ref::<web_sys::HtmlElement>().focus();
                    }
                });
            }
        });
    }
    // Focus trap: cycle Tab within dialog
    let on_keydown = {
        let dialog_ref = dialog_ref.clone();
        move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape"
                && let Some(ref cb) = on_close
            {
                cb.run(());
            }
            if ev.key() == "Tab" {
                if let Some(el) = dialog_ref.get() {
                    let focusables: Vec<web_sys::Element> = {
                        let nodes = el.query_selector_all("a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])").unwrap();
                        (0..nodes.length()).filter_map(|i| nodes.get(i)).collect()
                    };
                    if focusables.is_empty() {
                        ev.prevent_default();
                        return;
                    }
                    let active = web_sys::window()
                        .and_then(|w| w.document())
                        .and_then(|d| d.active_element());
                    let active_el = active.as_ref().map(|a| a.dyn_ref::<web_sys::Element>());
                    let first = focusables.first().unwrap();
                    let last = focusables.last().unwrap();
                    if !ev.shift_key() && active_el == Some(first) && focusables.len() == 1 {
                        ev.prevent_default();
                    } else if ev.shift_key() && active_el == Some(first) {
                        ev.prevent_default();
                        let _ = last.dyn_ref::<web_sys::HtmlElement>().map(|e| e.focus());
                    } else if !ev.shift_key() && active_el == Some(last) {
                        ev.prevent_default();
                        let _ = first.dyn_ref::<web_sys::HtmlElement>().map(|e| e.focus());
                    }
                }
            }
        }
    };

    view! {
        <div class="dialog-overlay" class:hidden=move || !open.get() style:display=move || {
            if open.get() { "" } else { "none" }
        }
        on:keydown=on_keydown
        >
            <div node_ref=dialog_ref class=cls role="dialog" aria-modal="true" tabindex="-1">
                {if !title.is_empty() {
                    view! {
                        <div class="dialog-header">
                            <h2 class="dialog-title">{title}</h2>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }}
                {children()}
            </div>
        </div>
    }
}
