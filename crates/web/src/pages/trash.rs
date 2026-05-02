use leptos::*;
use leptos_router::A;

use crate::api;
use crate::api::TrashedEntry;
use crate::components::toast::ToastContext;

#[component]
pub fn TrashPage() -> impl IntoView {
    let (entries, set_entries) = create_signal::<Vec<TrashedEntry>>(vec![]);
    let (loading, set_loading) = create_signal(false);
    let (show_confirm_empty, set_show_confirm_empty) = create_signal(false);

    let load_trash = move || {
        set_loading.set(true);
        spawn_local(async move {
            match api::list_trash().await {
                Ok(list) => set_entries.set(list),
                Err(_) => set_entries.set(vec![]),
            }
            set_loading.set(false);
        });
    };

    create_effect(move |_| {
        load_trash();
    });

    let do_restore = move |path: String| {
        spawn_local(async move {
            match api::restore_trash(&path).await {
                Ok(()) => {
                    ToastContext::success(format!("Restored: {}", path));
                    load_trash();
                }
                Err(e) => {
                    ToastContext::error(format!("Restore failed: {}", e));
                }
            }
        });
    };

    let do_purge = move |path: String| {
        spawn_local(async move {
            match api::purge_trash(&path).await {
                Ok(()) => {
                    ToastContext::success(format!("Permanently deleted: {}", path));
                    load_trash();
                }
                Err(e) => {
                    ToastContext::error(format!("Purge failed: {}", e));
                }
            }
        });
    };

    let do_empty = move |_: ev::MouseEvent| {
        set_show_confirm_empty.set(false);
        spawn_local(async move {
            match api::empty_trash().await {
                Ok(()) => {
                    ToastContext::success("Trash emptied");
                    load_trash();
                }
                Err(e) => {
                    ToastContext::error(format!("Empty trash failed: {}", e));
                }
            }
        });
    };

    let display_entries = move || {
        let ents = entries.get();
        if ents.len() > 50 {
            ents[..50].to_vec()
        } else {
            ents
        }
    };

    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900 flex flex-col">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">"Skip to main content"</a>

            // Header
            <header class="surface brutal-border border-b px-6 py-3 shadow-concrete">
                <div class="flex items-center justify-between max-w-7xl mx-auto">
                    <div class="flex items-center gap-3">
                        <A href="/ui/" class="flex items-center gap-2 no-underline">
                            <div class="w-8 h-8 bg-transparent brutal-border rounded flex items-center justify-center font-display text-accent">
                                <span class="font-bold text-sm">"F"</span>
                            </div>
                            <div>
                                <h1 class="text-lg font-bold font-mono text-gray-900 leading-none">"Ferro"</h1>
                                <span class="text-label text-muted">"Trash"</span>
                            </div>
                        </A>
                    </div>
                    <div class="flex items-center gap-2">
                        <A
                            href="/ui/"
                            class="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800 no-underline rounded hover:bg-gray-100 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                        >
                            "Back to Files"
                        </A>
                        {move || (!entries.with(Vec::is_empty)).then(|| view! {
                            <button
                                class="px-3 py-1.5 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                on:click=move |_| set_show_confirm_empty.set(true)
                            >
                                "Empty Trash"
                            </button>
                        })}
                    </div>
                </div>
            </header>

            // Confirm empty dialog
            {move || show_confirm_empty.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            set_show_confirm_empty.set(false);
                        }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-96"
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="empty-trash-title"
                        aria-describedby="empty-trash-desc"
                        tabindex="-1"
                    >
                        <h3 id="empty-trash-title" class="text-section font-mono text-gray-900 mb-2">"Empty Trash?"</h3>
                        <p id="empty-trash-desc" class="text-sm text-gray-600 mb-6">
                            "This will permanently delete all trashed files. This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_confirm_empty.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                on:click=do_empty
                            >
                                "Empty Trash"
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Main content
            <main id="main-content" class="flex-1 max-w-7xl w-full mx-auto surface brutal-border shadow-concrete my-4 rounded-lg overflow-hidden">
                {move || loading.get().then(|| view! {
                    <div class="px-6 py-12 text-center text-gray-500" role="status" aria-live="polite">
                        <div class="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
                        "Loading trash..."
                    </div>
                })}

                {move || {
                    if loading.get() {
                        return view! { <div class="hidden"></div> }.into_any();
                    }
                    let ents = display_entries();
                    if ents.is_empty() {
                        view! {
                            <div class="px-6 py-16 text-center text-gray-400" role="status">
                                <svg class="w-16 h-16 mx-auto mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                </svg>
                                <div class="text-lg font-medium">"Trash is empty"</div>
                                <div class="text-sm mt-1">"Deleted files will appear here"</div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <table class="w-full" role="grid">
                                <thead class="bg-gray-50 border-b sticky top-0">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500" scope="col">"Original Path"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-40" scope="col">"Deleted"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Size"</th>
                                        <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-gray-500 w-48" scope="col">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=display_entries
                                        key=|e| e.original_path.clone()
                                        let:entry
                                    >
                                        {
                                            let restore_path = entry.original_path.clone();
                                            let purge_path = entry.original_path.clone();
                                            let restore_name = entry.original_path.clone();
                                            let purge_name = entry.original_path.clone();
                                            view! {
                                                <tr class="hover:bg-gray-50 border-b border-gray-100 transition-colors" role="row">
                                                    <td class="px-4 py-2.5 text-gray-700 text-sm" role="rowheader">
                                                        {&entry.original_path}
                                                    </td>
                                                    <td class="px-4 py-2.5 text-gray-500 text-sm" role="gridcell">
                                                        {&entry.deleted_at}
                                                    </td>
                                                    <td class="px-4 py-2.5 text-gray-500 text-sm tabular-nums" role="gridcell">
                                                        {format_size(entry.size)}
                                                    </td>
                                                    <td class="px-4 py-2.5 text-right" role="gridcell">
                                                        <div class="flex items-center justify-end gap-2">
                                                            <button
                                                                class="px-2.5 py-1 text-xs bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                                                aria_label=format!("Restore {}", restore_name)
                                                                on:click=move |_| do_restore(restore_path.clone())
                                                            >
                                                                "Restore"
                                                            </button>
                                                            <button
                                                                class="px-2.5 py-1 text-xs bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                                                aria_label=format!("Permanently delete {}", purge_name)
                                                                on:click=move |_| do_purge(purge_path.clone())
                                                            >
                                                                "Delete Permanently"
                                                            </button>
                                                        </div>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    </For>
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            </main>
        </div>
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}
