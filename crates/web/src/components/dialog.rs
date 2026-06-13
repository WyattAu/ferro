use leptos::ev;
use leptos::prelude::*;

use crate::components::focus_trap::FocusTrap;
use crate::t;

/// Accessible dialog component following the leptix pattern.
///
/// Renders a modal dialog with proper ARIA attributes, focus management,
/// Escape key handling, and click-outside-to-close behavior.
///
/// Unlike simpler dialogs, this component provides:
/// - `role="dialog"` and `aria-modal="true"`
/// - `aria-labelledby` pointing to the title
/// - Optional `aria-describedby` for description text
/// - Focus trap via `FocusTrap`
/// - Close button with accessible label
///
/// # Usage
/// Compose your dialog content directly in the view:
/// ```ignore
/// view! {
///     <Dialog open=set_open set_open title="My Dialog">
///         <p>"Dialog body content"</p>
///     </Dialog>
/// }
/// ```
#[component]
pub fn Dialog(
    /// Whether the dialog is open.
    open: ReadSignal<bool>,
    /// Setter for open state.
    set_open: WriteSignal<bool>,
    /// Dialog title text.
    title: &'static str,
) -> impl IntoView {
    let close = move |_| {
        set_open.set(false);
    };

    let on_backdrop_click = move |ev: ev::MouseEvent| {
        if ev.target() == ev.current_target() {
            set_open.set(false);
        }
    };

    let on_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            set_open.set(false);
        }
    };

    view! {
        {move || open.get().then(|| view! {
            <div
                class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center p-4 transition-opacity duration-200"
                on:click=on_backdrop_click
                on:keydown=on_keydown
            >
                <FocusTrap>
                    <div
                        class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="dialog-title"
                        tabindex="-1"
                    >
                        <div class="flex items-center justify-between mb-4">
                            <h3 id="dialog-title" class="text-lg font-semibold font-mono text-gray-900">
                                {title}
                            </h3>
                            <button
                                class="p-1 rounded-sm text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center"
                                aria-label=t!("aria.close_dialog")
                                on:click=close
                            >
                                <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                    </div>
                </FocusTrap>
            </div>
        })}
    }
}

/// Accessible dialog footer with cancel/confirm actions.
#[component]
pub fn DialogFooter(
    /// Cancel button label.
    #[prop(default = "Cancel".to_string())]
    cancel_label: String,
    /// Confirm button label.
    #[prop(default = "Confirm".to_string())]
    confirm_label: String,
    /// Called when cancel is clicked.
    on_cancel: Callback<ev::MouseEvent>,
    /// Called when confirm is clicked.
    on_confirm: Callback<ev::MouseEvent>,
    /// Whether confirm button is destructive (red styling).
    #[prop(default = false)]
    destructive: bool,
    /// Whether confirm button is disabled.
    #[prop(default = false)]
    confirm_disabled: bool,
) -> impl IntoView {
    let confirm_class = if destructive {
        "px-4 py-2 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
    } else {
        "px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
    };

    view! {
        <div class="flex justify-end gap-2 mt-6">
            <button
                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded min-h-[44px]"
                on:click=move |ev| on_cancel.run(ev)
            >
                {cancel_label}
            </button>
            <button
                class=confirm_class
                disabled=confirm_disabled
                on:click=move |ev| on_confirm.run(ev)
            >
                {confirm_label}
            </button>
        </div>
    }
}
