use crate::components::clipboard::{ClipboardAction, ClipboardState};
use crate::components::custom_view::{self, ViewPreset};
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
    set_view_mode: WriteSignal<ViewMode>,
    select_mode: ReadSignal<bool>,
    toggle_select_mode: impl FnMut(ev::MouseEvent) + 'static,
    show_activity: ReadSignal<bool>,
    toggle_activity: impl FnMut(ev::MouseEvent) + 'static,
    show_smart_collections: ReadSignal<bool>,
    toggle_smart_collections: impl FnMut(ev::MouseEvent) + 'static,
    #[allow(unused)] children: Children,
) -> impl IntoView {
    let (show_preset_menu, set_show_preset_menu) = signal(false);
    let (active_preset, set_active_preset) = signal("Default".to_string());
    let (saved_views, set_saved_views) = signal(custom_view::load_custom_views());
    let (show_mobile_tabs, set_show_mobile_tabs) = signal(false);

    // Close mobile tabs menu when clicking outside
    {
        Effect::new(move |_| {
            if show_mobile_tabs.get() {
                let handle = window_event_listener(ev::click, move |_: ev::MouseEvent| {
                    set_show_mobile_tabs.set(false);
                });
                on_cleanup(move || handle.remove());
            }
        });
    }

    let apply_preset = move |preset: ViewPreset, ev: ev::MouseEvent| {
        ev.stop_propagation();
        let view = preset.to_custom_view();
        custom_view::save_custom_view(&view);
        set_active_preset.set(preset.label().to_string());
        set_show_preset_menu.set(false);
        set_saved_views.set(custom_view::load_custom_views());
    };

    let delete_view = move |id: String, ev: ev::MouseEvent| {
        ev.stop_propagation();
        custom_view::delete_custom_view(&id);
        set_saved_views.set(custom_view::load_custom_views());
    };
    view! {
        <div class="brutal-border border-b px-2 sm:px-6 py-1.5 sm:py-3 surface shadow-concrete sticky top-0 z-20 bg-[var(--bg-surface)]">
           <div class="flex items-center justify-between gap-2">
               <div class="flex items-center gap-2 min-w-0 flex-1">
                   <button
                       class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center shrink-0"
                        aria-label=t!("breadcrumb.parent")
                       on:click=go_up
                       disabled=move || current_path.get() == "/"
                   >
                       <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                           <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 17l-5-5m0 0l5-5m-5 5h12" />
                       </svg>
                   </button>

                    {children()}

                    <div class="flex items-center gap-1 sm:gap-2 flex-wrap justify-end shrink-0 relative">
                        // Mobile hamburger menu for tabs (hidden on sm+)
                        <div class="sm:hidden relative">
                            <button
                                class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center"
                                aria-label="Tab navigation"
                                on:click=move |ev| { ev.stop_propagation(); set_show_mobile_tabs.update(|v| *v = !*v); }
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
                                </svg>
                            </button>
                            {move || show_mobile_tabs.get().then(|| view! {
                                <div class="absolute top-full left-0 mt-1 bg-[var(--bg-surface)] rounded-lg shadow-lg border border-[var(--border-default)] min-w-[160px] z-50">
                                    {[
                                        (BrowserTab::Files, t!("nav.files")),
                                        (BrowserTab::Favorites, t!("nav.favorites")),
                                        (BrowserTab::Recent, t!("nav.recent")),
                                    ].into_iter().map(|(tab, label)| {
                                        let tab_clone = tab;
                                        let is_active = move || active_tab.get() == tab_clone;
                                        view! {
                                            <button
                                                class=move || format!(
                                                    "block w-full text-left px-3 py-2 text-sm font-mono transition-colors {}",
                                                    if is_active() { "bg-[var(--accent-subtle)] text-[var(--accent)]" } else { "text-[var(--text-primary)] hover:bg-[var(--interactive-hover)]" }
                                                )
                                                on:click=move |ev| { ev.stop_propagation(); switch_tab.run(tab); set_show_mobile_tabs.set(false); }
                                            >
                                                {label}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            })}
                        </div>

                        // Desktop tab bar (hidden on mobile)
                        <div class="hidden sm:flex items-center bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] rounded p-0.5">
                            <button
                                class=move || {
                                    let base = "px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                    let active = if active_tab.get() == BrowserTab::Files { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] dark:text-gray-100 shadow-sm border-b-2 border-b-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]" };
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
                                   let active = if active_tab.get() == BrowserTab::Favorites { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] dark:text-gray-100 shadow-sm border-b-2 border-b-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]" };
                                   format!("{} {}", base, active)
                               }
                               on:click=move |_| switch_tab.run(BrowserTab::Favorites)
                               aria-label=move || t!("nav.favorites")
                          >
                              {t!("nav.favorites")}
                          </button>
                          <button
                              class=move || {
                                   let base = "px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                   let active = if active_tab.get() == BrowserTab::Recent { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] dark:text-gray-100 shadow-sm border-b-2 border-b-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]" };
                                   format!("{} {}", base, active)
                              }
                              on:click=move |_| switch_tab.run(BrowserTab::Recent)
                              aria-label=move || t!("nav.recent")
                         >
                              {t!("nav.recent")}
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
                                  class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-[var(--success)] text-[var(--text-on-accent)] rounded-sm brutal-border font-bold uppercase hover:bg-[var(--success-hover)] transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--success)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
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
                         class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-[var(--accent)] text-[var(--text-on-accent)] rounded-sm hover:bg-[var(--accent-hover)] brutal-border shadow-iron transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px] uppercase font-bold tracking-wider"
                         aria-label=t!("toolbar.aria_upload")
                         on:click=move |_| set_show_upload.set(true)
                     >
                         <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                         </svg>
                         <span class="hidden sm:inline">{t!("common.upload")}</span>
                     </button>
                     <button
                         class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] rounded-sm brutal-border font-bold uppercase hover:bg-[var(--border-subtle)] hover:bg-[var(--interactive-hover)] transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px] tracking-wider"
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
                         attr:class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] rounded hover:bg-[var(--bg-inset)] transition-colors no-underline flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                          attr:aria-label=t!("toolbar.aria_trash")
                     >
                         <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                             <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                         </svg>
                         <span class="hidden sm:inline">{t!("common.trash")}</span>
                     </A>

                      // View mode toggle
                      <div class="flex items-center bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] rounded p-0.5">
                          <button
                              class=move || {
                                  let base = "p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                  let active = if view_mode.get() == ViewMode::List { "bg-[var(--bg-surface)] text-[var(--text-primary)] shadow-sm" } else { "" };
                                  format!("{} {}", base, active)
                              }
                              aria-label=move || if view_mode.get() == ViewMode::List { "List view active" } else { "Switch to list view" }
                              title="List View"
                              on:click=move |_| set_view_mode.set(ViewMode::List)
                          >
                              <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                              </svg>
                          </button>
                          <button
                              class=move || {
                                  let base = "p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                  let active = if view_mode.get() == ViewMode::Grid { "bg-[var(--bg-surface)] text-[var(--text-primary)] shadow-sm" } else { "" };
                                  format!("{} {}", base, active)
                              }
                              aria-label=move || if view_mode.get() == ViewMode::Grid { "Grid view active" } else { "Switch to grid view" }
                              title="Grid View"
                              on:click=move |_| set_view_mode.set(ViewMode::Grid)
                          >
                              <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                              </svg>
                          </button>
                          <button
                              class=move || {
                                  let base = "p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                  let active = if view_mode.get() == ViewMode::Graph { "bg-[var(--bg-surface)] text-[var(--text-primary)] shadow-sm" } else { "" };
                                  format!("{} {}", base, active)
                              }
                              aria-label=move || if view_mode.get() == ViewMode::Graph { "Graph view active" } else { "Switch to graph view" }
                              title="Graph View"
                              on:click=move |_| set_view_mode.set(ViewMode::Graph)
                          >
                              <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                              </svg>
                          </button>
                          // DualPane button - hidden on mobile
                          <div class="hidden sm:block">
                              <button
                                  class=move || {
                                       let base = "p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center";
                                       let active = if view_mode.get() == ViewMode::DualPane { "bg-[var(--bg-surface)] text-[var(--text-primary)] shadow-sm" } else { "" };
                                       format!("{} {}", base, active)
                                  }
                                  aria-label=move || if view_mode.get() == ViewMode::DualPane { "Dual pane active" } else { "Switch to dual pane" }
                                  title="Dual Pane Mode"
                                  on:click=move |_| set_view_mode.set(ViewMode::DualPane)
                              >
                                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2" />
                                  </svg>
                              </button>
                          </div>
                      </div>

                      // View preset selector
                      <div class="relative">
                          <button
                              class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center"
                              on:click=move |ev| { ev.stop_propagation(); set_show_preset_menu.update(|v| *v = !*v); }
                              title="View Presets"
                              aria-label="View presets"
                          >
                              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
                              </svg>
                          </button>
                          {move || show_preset_menu.get().then(|| view! {
                              <div class="absolute top-full right-0 mt-1 bg-[var(--bg-surface)] rounded-lg shadow-lg border border-[var(--border)] min-w-[180px] z-50">
                                  <div class="px-3 py-1.5 text-xs font-bold uppercase text-[var(--text-tertiary)] border-b border-[var(--border)]">
                                      View Presets
                                  </div>
                                  {vec![
                                      ViewPreset::Default,
                                      ViewPreset::Compact,
                                      ViewPreset::Detailed,
                                      ViewPreset::Media,
                                      ViewPreset::Documents,
                                  ].into_iter().map(|preset| {
                                      let label = preset.label().to_string();
                                      let label_clone = label.clone();
                                      let preset_clone = preset.clone();
                                      let is_active = move || active_preset.get() == label_clone;
                                      view! {
                                          <button
                                              class=move || format!(
                                                  "block w-full text-left px-3 py-1.5 text-xs font-mono hover:bg-[var(--interactive-hover)] {}",
                                                  if is_active() { "text-[var(--accent)]" } else { "text-[var(--text-primary)]" }
                                              )
                                              on:click=move |ev| apply_preset(preset_clone.clone(), ev)
                                          >
                                              {label}
                                          </button>
                                      }
                                  }).collect::<Vec<_>>()}
                                  {move || {
                                      let views = saved_views.get();
                                      if !views.is_empty() {
                                          view! {
                                              <div class="px-3 py-1.5 text-xs font-bold uppercase text-[var(--text-tertiary)] border-t border-[var(--border)] mt-1">
                                                  Saved Views
                                              </div>
                                              {views.into_iter().map(|view_config| {
                                                  let view_id = view_config.id.clone();
                                                  let view_name = view_config.name.clone();
                                                  view! {
                                                      <div class="flex items-center justify-between px-3 py-1.5 hover:bg-[var(--interactive-hover)]">
                                                          <span class="text-xs font-mono text-[var(--text-primary)]">{view_name}</span>
                                                          <button
                                                              class="text-[var(--text-tertiary)] hover:text-[var(--danger)] text-xs"
                                                              on:click=move |ev| delete_view(view_id.clone(), ev)
                                                              aria-label="Delete view"
                                                          >
                                                              "×"
                                                          </button>
                                                      </div>
                                                  }
                                              }).collect::<Vec<_>>()}
                                          }.into_any()
                                      } else {
                                          view! { <div class="hidden"></div> }.into_any()
                                      }
                                  }}
                              </div>
                          })}
                      </div>

                     <button
                         class=move || format!(
                             "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center {}",
                             if select_mode.get() { "bg-[var(--accent-subtle)] text-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)]" }
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
                              if show_activity.get() { "bg-[var(--accent-subtle)] text-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)]" }
                          )
                           aria-label=t!("toolbar.aria_activity")
                          on:click=toggle_activity
                      >
                          <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                          </svg>
                      </button>
                      <button
                          class=move || format!(
                              "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center transition-all duration-200 {}",
                              if show_smart_collections.get() { "bg-[var(--accent-subtle)] text-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-inset)]" }
                          )
                           aria-label="Smart Collections"
                          on:click=toggle_smart_collections
                      >
                          <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                          </svg>
                      </button>
                  </div>
             </div>
         </div>
         </div>
    }
}
