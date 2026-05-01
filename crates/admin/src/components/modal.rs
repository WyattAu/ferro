use leptos::*;

#[component]
pub fn Modal(
    title: String,
    show: bool,
    on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="modal-overlay" class:modal-visible=show>
            <div class="modal-backdrop" on:click=move |_| on_close.call(())></div>
            <div class="modal">
                <div class="modal-header">
                    <h3 class="modal-title">{title}</h3>
                    <button class="modal-close" on:click=move |_| on_close.call(()) aria-label="Close">
                        <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="4" y1="4" x2="16" y2="16"/>
                            <line x1="16" y1="4" x2="4" y2="16"/>
                        </svg>
                    </button>
                </div>
                <div class="modal-body">{children()}</div>
            </div>
        </div>
    }
}
