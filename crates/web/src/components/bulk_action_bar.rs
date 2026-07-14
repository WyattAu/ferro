use leptos::prelude::*;

use crate::t;

/// Floating action bar shown when files are selected in select mode.
#[component]
pub fn BulkActionBar(
    /// Whether select mode is active.
    select_mode: ReadSignal<bool>,
    /// Number of currently selected items.
    selected_count: Signal<usize>,
    /// Called when user clicks Delete.
    on_delete: Callback<()>,
    /// Called when user clicks Download.
    on_download: Callback<()>,
    /// Called when user clicks Clear.
    on_clear: Callback<()>,
) -> impl IntoView {
    view! {
        {move || (select_mode.get() && selected_count.get() > 0).then(|| view! {
            <div class="fixed bottom-0 sm:bottom-4 left-0 sm:left-1/2 sm:-translate-x-1/2 w-full sm:w-auto surface-dark text-white dark:text-gray-900 rounded-none sm:rounded shadow-2xl brutal-border border-t-3 border-t-accent px-4 sm:px-6 py-3 flex items-center gap-2 sm:gap-4 z-50 justify-between sm:justify-center transition-all duration-200">
                <span class="text-sm font-bold font-mono uppercase tracking-wider">
                    {move || selected_count.get()} {t!("common.selected")}
                </span>
                 <div class="flex items-center gap-2">
                     <button
                          class="px-3 py-1.5 text-sm bg-[var(--danger)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-red-700 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                         on:click=move |_| on_delete.run(())
                         aria-label=t!("bulk.aria_delete")
                     >
                         {t!("common.delete")}
                     </button>
                     <button
                          class="px-3 py-1.5 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                         on:click=move |_| on_download.run(())
                         aria-label=t!("bulk.aria_download")
                     >
                         <span class="hidden sm:inline">{t!("common.download")}</span>
                         <svg class="w-4 h-4 sm:hidden" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
                     </button>
                     <button
                         class="px-3 py-1.5 text-sm bg-gray-600 dark:bg-gray-300 text-white dark:text-gray-900 brutal-border rounded-sm font-bold uppercase hover:bg-gray-500 dark:hover:bg-gray-200 transition-colors focus:outline-none focus:ring-2 focus:ring-gray-400 min-h-[44px]"
                         on:click=move |_| on_clear.run(())
                         aria-label=t!("bulk.aria_clear")
                     >
                         {t!("common.clear")}
                     </button>
                 </div>
            </div>
        })}
    }
}
