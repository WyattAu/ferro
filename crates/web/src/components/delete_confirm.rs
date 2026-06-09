use leptos::ev;
use leptos::prelude::*;

use crate::components::focus_trap::FocusTrap;
use crate::t;

/// Confirmation dialog for bulk delete operations.
#[component]
pub fn DeleteConfirmDialog(
    /// Whether the dialog is visible.
    open: ReadSignal<bool>,
    /// Setter for dialog visibility.
    set_open: WriteSignal<bool>,
    /// Number of items to be deleted (displayed in the message).
    count: Signal<usize>,
    /// Called when the user confirms deletion.
    on_confirm: Callback<ev::MouseEvent>,
) -> impl IntoView {
    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4 transition-opacity duration-200"
                on:keydown=move |ev: ev::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        set_open.set(false);
                    }
                }
            >
                <FocusTrap>
                <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="delete-confirm-title"
                    tabindex="-1"
                >
                    <h3 id="delete-confirm-title" class="text-lg font-semibold text-gray-900 mb-2">{t!("dialog.delete_confirm.title")}</h3>
                    <p class="text-sm text-gray-600 mb-6">
                        {move || format!("Are you sure you want to delete {} file(s)? This action cannot be undone.", count.get())}
                    </p>
                    <div class="flex justify-end gap-2">
                        <button
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded min-h-[44px]"
                            on:click=move |_| set_open.set(false)
                        >
                            {t!("common.cancel")}
                        </button>
                        <button
                            class="px-4 py-2 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                            on:click=move |ev| {
                                set_open.set(false);
                                on_confirm.run(ev);
                            }
                        >
                            {t!("common.delete")}
                        </button>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
