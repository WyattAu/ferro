use leptos::*;

use crate::api;
use crate::components::toast::ToastContext;

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
    let (folder_name, set_folder_name) = create_signal(String::new());

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
                    ToastContext::success("Folder created");
                    on_created.call(());
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
                <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="new-folder-title"
                    tabindex="-1"
                >
                    <h3 id="new-folder-title" class="text-section font-mono text-gray-900 mb-4">"New Folder"</h3>
                    <label class="block mb-4">
                        <span class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Folder name"</span>
                        <input
                            type="text"
                            placeholder="Folder name"
                            class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            prop:value=folder_name
                            on:input=move |ev| set_folder_name.set(event_target_value(&ev))
                        />
                    </label>
                    <div class="flex justify-end gap-2">
                        <button
                            class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                            on:click=move |_| set_open.set(false)
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                            on:click=do_create
                        >
                            "Create"
                        </button>
                    </div>
                </div>
            </div>
        })}
    }
}
