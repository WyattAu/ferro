use leptos::*;

use crate::components::focus_trap::FocusTrap;
use crate::components::toast::ToastContext;

/// Reusable dialog for path-based operations (Move / Copy).
#[component]
pub fn PathDialog(
    /// Dialog title (e.g. "Move File" or "Copy File").
    title: &'static str,
    /// Action button label (e.g. "Move" or "Copy").
    action_label: &'static str,
    /// Whether the dialog is visible.
    open: ReadSignal<bool>,
    /// Setter for dialog visibility.
    set_open: WriteSignal<bool>,
    /// Source path display signal.
    source: ReadSignal<String>,
    /// Destination path input signal.
    dest: ReadSignal<String>,
    /// Setter for destination path input.
    set_dest: WriteSignal<String>,
    /// Called when the user confirms the action (source, dest).
    on_confirm: Callback<(String, String)>,
) -> impl IntoView {
    let do_execute = move |_: ev::MouseEvent| {
        let source = source.get();
        let dest = dest.get();
        if dest.is_empty() {
            ToastContext::error("Destination path cannot be empty");
            return;
        }
        on_confirm.call((source, dest));
    };

    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                on:keydown=move |ev: ev::KeyboardEvent| {
                    if ev.key() == "Escape" { set_open.set(false); }
                }
            >
                <FocusTrap>
                <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                    role="dialog"
                    aria-modal="true"
                    tabindex="-1"
                >
                    <h3 class="text-section font-mono text-gray-900 mb-4">{title}</h3>
                    <div class="mb-4">
                        <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Source"</label>
                        <div class="px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 truncate">
                            {source}
                        </div>
                    </div>
                    <div class="mb-4">
                        <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Destination"</label>
                        <input
                            type="text"
                            placeholder="/new/path/file.txt"
                            class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            prop:value=dest
                            on:input=move |ev| set_dest.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="flex justify-end gap-2">
                        <button
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                            on:click=move |_| set_open.set(false)
                        >"Cancel"</button>
                        <button
                            class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                            on:click=do_execute
                        >{action_label}</button>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
