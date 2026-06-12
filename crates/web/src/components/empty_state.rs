use leptos::prelude::*;

use crate::t;

/// Empty state shown when a folder has no files.
#[component]
pub fn EmptyState(
    /// Whether data is currently loading.
    loading: ReadSignal<bool>,
    /// Whether the Files tab is active.
    files_tab_active: Signal<bool>,
    /// Whether there is an error (suppress empty state if so).
    has_error: Signal<bool>,
    /// Whether entries list is empty.
    is_empty: Signal<bool>,
    /// Called when user clicks "Upload your first file".
    on_upload: Callback<()>,
) -> impl IntoView {
    view! {
            {move || (!loading.get() && files_tab_active.get() && is_empty.get() && !has_error.get()).then(|| view! {
                <div class="px-6 py-16 text-center" role="status">
                    <svg class="w-20 h-20 mx-auto mb-4 text-gray-300" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                    </svg>
                    <div class="text-lg font-medium text-gray-500">{t!("empty.folder")}</div>
                    <div class="text-sm text-gray-400 mt-1 mb-4">{t!("empty.folder_hint")}</div>
                    <button
                        class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                        on:click=move |_| on_upload.run(())
                        aria-label=t!("empty.folder_upload_btn")
                    >
    {t!("empty.folder_upload_btn")}
                    </button>
                </div>
            })}
        }
}
