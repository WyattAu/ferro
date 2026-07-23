use leptos::prelude::*;

/// Modal dialog component — renders once, visibility controlled by signal.
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
        }>
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
