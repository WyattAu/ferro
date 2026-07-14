use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::focus_trap::FocusTrap;
use crate::components::toast::ToastContext;
use crate::t;

/// Dialog for creating a new folder in the current directory.
#[component]
pub fn NewFolderDialog(
    /// Whether the dialog is visible.
    open: ReadSignal<bool>,
    /// Setter for dialog visibility.
    set_open: WriteSignal<bool>,
    /// Current directory path (where the folder will be created).
    current_path: ReadSignal<String>,
    /// Callback invoked after successful creation.
    on_created: Callback<()>,
) -> impl IntoView {
    let (folder_name, set_folder_name) = signal(String::new());

    let do_create = move |_: ev::MouseEvent| {
        let name = folder_name.get();
        if name.is_empty() {
            return;
        }
        let path = current_path.get();
        let folder_path = if path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", path, name)
        };
        let fp = folder_path.clone();
        spawn_local(async move {
            match api::create_directory(&fp).await {
                Ok(()) => {
                    set_open.set(false);
                    set_folder_name.set(String::new());
                    ToastContext::success(t!("toast.folder_created"));
                    on_created.run(());
                }
                Err(e) => {
                    ToastContext::error(format!("Failed to create folder: {}", e));
                }
            }
        });
    };

    view! {
        {move || open.get().then(|| view! {
            <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
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
                    aria-labelledby="new-folder-title"
                    tabindex="-1"
                >
                    <h3 id="new-folder-title" class="text-section font-mono text-[var(--text-primary)] mb-4">{t!("dialog.new_folder.title")}</h3>
                    <label for="new-folder-name" class="block mb-4">
                        <span class="block text-xs font-bold uppercase font-mono text-[var(--text-secondary)] mb-1">{t!("dialog.new_folder.name_label")}</span>
                        <input id="new-folder-name"
                            type="text"
                            placeholder=t!("dialog.new_folder.name_placeholder")
                            class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
                            prop:value=folder_name
                            on:input=move |ev| set_folder_name.set(event_target_value(&ev))
                        />
                    </label>
                    <div class="flex justify-end gap-2">
                        <button
                            class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 rounded min-h-[44px]"
                            on:click=move |_| set_open.set(false)
                        >
                            {t!("common.cancel")}
                        </button>
                        <button
                            class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                            on:click=do_create
                        >
                            {t!("common.create")}
                        </button>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
