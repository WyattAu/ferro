use crate::components::clipboard::{ClipboardAction, ClipboardState};
use crate::t;
use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;

use super::{BrowserTab, ViewMode};

#[component]
pub fn Toolbar(
    current_path: ReadSignal<String>,
    go_up: impl FnMut(ev::MouseEvent) + 'static,
    active_tab: ReadSignal<BrowserTab>,
    switch_tab: Callback<BrowserTab>,
    clipboard_state: ClipboardState,
    clipboard_paste: Callback<()>,
    set_show_upload: WriteSignal<bool>,
    set_show_new_folder: WriteSignal<bool>,
    view_mode: ReadSignal<ViewMode>,
    toggle_view_mode: impl FnMut(ev::MouseEvent) + 'static,
    select_mode: ReadSignal<bool>,
    toggle_select_mode: impl FnMut(ev::MouseEvent) + 'static,
    show_activity: ReadSignal<bool>,
    toggle_activity: impl FnMut(ev::MouseEvent) + 'static,
    #[allow(unused)] children: Children,
) -> impl IntoView {
    view! {
        <div class="brutal-border border-b px-2 sm:px-6 py-1.5 sm:py-3 surface shadow-concrete sticky top-0 z-20 bg-[var(--bg-surface)]">
           <div class="flex items-center justify-between gap-2">
               <div class="flex items-center gap-2 min-w-0 flex-1">
                   <button
                       class="p-2 text-[var(--text-tertiary)] hover:text-gray-700 hover:bg-gray-100 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center shrink-0"
                        aria-label=t!("breadcrumb.parent")
                       on:click=go_up
                       disabled=move || current_path.get() == "/"
                   >
                       <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                           <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 17l-5-5m0 0l5-5m-5 5h12" />
                       </svg>
                   </button>

                    {children()}

                   <div class="flex items-center gap-1 sm:gap-2 flex-wrap justify-end shrink-0">
                       <div class="flex items-center bg-gray-100 dark:bg-gray-700 rounded p-0.5">
                           <button
                               class=move || {
                                   let base = "px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                   let active = if active_tab.get() == BrowserTab::Files { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-[var(--text-tertiary)] hover:text-gray-700" };
                                   format!("{} {}", base, active)
                               }
                                on:click=move |_| switch_tab.run(BrowserTab::Files)
                                aria-label=move || t!("nav.files")
                           >
                               {t!("nav.files")}
                          </button>
                          <button
                              class=move || {
                                  let base = "px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                  let active = if active_tab.get() == BrowserTab::Favorites { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-[var(--text-tertiary)] hover:text-gray-700" };
                                  format!("{} {}", base, active)
                              }
                              on:click=move |_| switch_tab.run(BrowserTab::Favorites)
                              aria-label=move || t!("nav.favorites")
                         >
                             <span class="hidden sm:inline">{t!("nav.favorites")}</span>
                               <svg class="w-4 h-4 sm:hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" /></svg>
                         </button>
                         <button
                             class=move || {
                                 let base = "px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                 let active = if active_tab.get() == BrowserTab::Recent { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-[var(--text-tertiary)] hover:text-gray-700" };
                                 format!("{} {}", base, active)
                             }
                             on:click=move |_| switch_tab.run(BrowserTab::Recent)
                             aria-label=move || t!("nav.recent")
                        >
                             <span class="hidden sm:inline">{t!("nav.recent")}</span>
                               <svg class="w-4 h-4 sm:hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                         </button>
                     </div>

                     {move || clipboard_state.has_files().then(|| {
                         let count = clipboard_state.file_count();
                         let al = clipboard_state.action().map(|a| match a {
                             ClipboardAction::Copy => t!("clipboard.copy"),
                             ClipboardAction::Cut => t!("clipboard.cut"),
                         }).unwrap_or_default();
                          view! {
                              <button
                                  class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-green-600 text-white rounded-sm brutal-border font-bold uppercase hover:bg-green-700 transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                  on:click=move |_| clipboard_paste.run(())
                                  title=move || format!("{} file(s) on clipboard ({})", count, al)
                                  aria-label=move || format!("Paste {} file(s)", count)
                              >
                                 <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
                                 </svg>
                                 <span class="hidden sm:inline">{move || format!("{} ({})", count, al)}</span>
                                 <span class="sm:hidden">{count}</span>
                             </button>
                         }.into_any()
                     })}

                     <button
                         class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-[var(--accent)] text-[var(--text-on-accent)] rounded-sm hover:bg-blue-700 brutal-border shadow-iron transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px] uppercase font-bold tracking-wider"
                         aria-label=t!("toolbar.aria_upload")
                         on:click=move |_| set_show_upload.set(true)
                     >
                         <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                         </svg>
                         <span class="hidden sm:inline">{t!("common.upload")}</span>
                     </button>
                     <button
                         class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 rounded-sm brutal-border font-bold uppercase hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px] tracking-wider"
                          aria-label=t!("toolbar.aria_new_folder")
                         on:click=move |_| set_show_new_folder.set(true)
                     >
                         <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                         </svg>
                         <span class="hidden sm:inline">{t!("dialog.new_folder.title")}</span>
                     </button>
                     <A
                         href="/ui/trash"
                         attr:class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm text-gray-600 hover:text-gray-800 rounded hover:bg-gray-100 transition-colors no-underline flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                          attr:aria-label=t!("toolbar.aria_trash")
                     >
                         <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                         </svg>
                         <span class="hidden sm:inline">{t!("common.trash")}</span>
                     </A>

                     // View mode toggle
                     <button
                         class="p-2 text-[var(--text-tertiary)] hover:text-gray-700 hover:bg-gray-100 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center"
                          aria-label=move || if view_mode.get() == ViewMode::Grid { t!("toolbar.aria_toggle_view") } else { t!("toolbar.aria_toggle_grid") }
                          title=move || if view_mode.get() == ViewMode::Grid { t!("toolbar.list_view") } else { t!("toolbar.grid_view") }
                         on:click=toggle_view_mode
                     >
                         {move || match view_mode.get() {
                             ViewMode::List => view! {
                                 <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                                 </svg>
                             }.into_any(),
                             ViewMode::Grid => view! {
                                 <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                                 </svg>
                             }.into_any(),
                         }}
                     </button>

                     <button
                         class=move || format!(
                             "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center {}",
                             if select_mode.get() { "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300" } else { "text-[var(--text-tertiary)] hover:text-gray-700 hover:bg-gray-100" }
                         )
                          aria-label=t!("toolbar.aria_select_mode")
                         aria_pressed=move || select_mode.get()
                         on:click=toggle_select_mode
                     >
                         <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
                         </svg>
                     </button>
                     <button
                         class=move || format!(
                             "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center transition-all duration-200 {}",
                             if show_activity.get() { "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300" } else { "text-[var(--text-tertiary)] hover:text-gray-700 hover:bg-gray-100" }
                         )
                          aria-label=t!("toolbar.aria_activity")
                         on:click=toggle_activity
                     >
                         <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                         </svg>
                     </button>
                 </div>
             </div>
         </div>
         </div>
    }
}
