use leptos::ev;
use leptos::prelude::*;

use super::focus_trap::FocusTrap;

#[component]
pub fn Modal(title: String, show: bool, on_close: Callback<()>, children: Children) -> impl IntoView {
    let aria_title = title.clone();
    let close_handler = on_close;
    view! {
        <div class="modal-overlay" class:modal-visible=show aria-hidden=move || !show>
            <div class="modal-backdrop" on:click=move |_| close_handler.run(()) aria-hidden="true"></div>
            <FocusTrap>
                <div
                    class="modal"
                    role="dialog"
                    aria-modal="true"
                    aria-label=aria_title
                    tabindex="-1"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            on_close.run(());
                        }
                    }
                >
                    <div class="modal-header">
                        <h2 class="modal-title font-display">{title}</h2>
                        <button
                            class="modal-close"
                            on:click=move |_| on_close.run(())
                            aria-label="Close dialog"
                            style="min-width: 44px; min-height: 44px;"
                        >
                            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                                <line x1="4" y1="4" x2="16" y2="16"/>
                                <line x1="16" y1="4" x2="4" y2="16"/>
                            </svg>
                        </button>
                    </div>
                    <div class="modal-body">{children()}</div>
                </div>
            </FocusTrap>
        </div>
    }
}
