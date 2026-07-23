use leptos::prelude::*;

/// Modal dialog component. Renders once, visibility controlled by signal.
/// Accessibility: role="dialog", aria-modal="true".
/// TODO: Implement focus trap (Tab cycles within dialog) and Escape key close.
#[component]
pub fn Dialog(
    #[prop(into)] open: Signal<bool>,
    #[prop(into, optional)] title: String,
    #[prop(optional)] class: String,
    children: Children,
) -> impl IntoView {
    let cls = format!("dialog {class}");

    view! {
        <div class="dialog-overlay" class:hidden=move || !open.get() style:display=move || {
            if open.get() { "" } else { "none" }
        }
        on:keydown=move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape" {
                // TODO: Close dialog by toggling open signal.
                // This requires the open signal to be writable, which it already is via Signal<bool>.
            }
        }
        >
            <div class=cls role="dialog" aria-modal="true">
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
