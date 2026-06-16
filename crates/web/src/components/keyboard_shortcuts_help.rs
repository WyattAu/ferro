use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::components::command_palette::use_command_palette_state;
use crate::components::clipboard::use_clipboard_state;
use crate::components::focus_trap::FocusTrap;
use crate::components::theme_toggle::use_theme_state;
use crate::t;

fn all_shortcuts_flat() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("Navigation", "Ctrl+K", "Command Palette"),
        ("Navigation", "Ctrl+F", "Search Files"),
        ("Navigation", "/", "Focus Search"),
        ("Navigation", "Esc", "Close / Dismiss"),
        ("File Operations", "Ctrl+C", "Copy Selected"),
        ("File Operations", "Ctrl+X", "Cut Selected"),
        ("File Operations", "Ctrl+V", "Paste Files"),
        ("File Operations", "Ctrl+A", "Select All"),
        ("File Operations", "Del", "Delete Selected"),
        ("File Operations", "F2", "Rename Selected"),
        ("File Operations", "Enter", "Open Selected"),
        ("Create", "Ctrl+N", "New Folder"),
        ("Create", "Ctrl+U", "Upload Files"),
        ("Create", "Ctrl+Shift+N", "New Note"),
        ("View", "Ctrl+D", "Toggle Dark Mode"),
        ("View", "Ctrl+E", "Toggle Grid/List View"),
        ("View", "?", "Show Keyboard Shortcuts"),
    ]
}

#[component]
pub fn KeyboardShortcutsHelp() -> impl IntoView {
    let (show_help, set_show_help) = signal(false);
    let (search_query, set_search_query) = signal(String::new());

    let _cmd_palette = use_command_palette_state();
    let _clipboard = use_clipboard_state();
    let _theme_state = use_theme_state();

    let shortcuts = all_shortcuts_flat();

    let filtered = move || {
        let query = search_query.get().to_lowercase();
        if query.is_empty() {
            return shortcuts.clone();
        }
        shortcuts.iter()
            .filter(|(_, keys, label)| {
                label.to_lowercase().contains(&query) || keys.to_lowercase().contains(&query)
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    // Global ? key handler
    #[cfg(target_arch = "wasm32")]
    {
        let set_show = set_show_help;
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |ev: web_sys::KeyboardEvent| {
                        let key = ev.key();
                        let ctrl = ev.ctrl_key() || ev.meta_key();
                        let shift = ev.shift_key();
                        if key == "?" && !ctrl && !shift {
                            set_show.update(|v| *v = !*v);
                        }
                    },
                ) as Box<dyn Fn(web_sys::KeyboardEvent)>);
                let _ = document
                    .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                std::mem::forget(cb);
            }
        }
    }

    let close_help = set_show_help;

    view! {
        {move || show_help.get().then(|| {
            let items = filtered();
            view! {
                <div
                    class="fixed inset-0 bg-black bg-opacity-50 z-[70] flex items-center justify-center backdrop-blur-sm"
                    on:click=move |_| close_help.set(false)
                >
                    <FocusTrap>
                    <div
                        class="brutal-block rounded shadow-2xl w-[calc(100%-2rem)] sm:w-full sm:max-w-lg mx-auto overflow-hidden surface"
                        role="dialog"
                        aria-modal="true"
                        aria-label="Keyboard Shortcuts"
                        tabindex="-1"
                        on:click=move |ev| ev.stop_propagation()
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Escape" {
                                close_help.set(false);
                            }
                        }
                    >
                        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                            <h2 class="text-section font-semibold text-gray-900 dark:text-gray-100">{t!("shortcuts.title")}</h2>
                            <button
                                class="p-1 rounded-sm opacity-60 hover:opacity-100 transition-opacity font-mono min-w-[44px] min-h-[44px] flex items-center justify-center focus:outline-none focus:ring-2 focus:ring-blue-500"
                                aria-label=t!("aria.close_dialog")
                                on:click=move |_| close_help.set(false)
                            >
                                <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>

                        <div class="px-4 py-2 border-b border-gray-100 dark:border-gray-700">
                            <div class="relative">
                                <svg class="absolute left-3 top-2.5 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                </svg>
                                <input
                                    type="text"
                                    placeholder="Search shortcuts..."
                                    class="w-full pl-10 pr-4 py-2 text-sm border border-gray-200 dark:border-gray-600 rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                                    prop:value=search_query
                                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="px-4 py-3 max-h-[60vh] overflow-y-auto">
                            {if items.is_empty() {
                                view! {
                                    <div class="py-8 text-center text-sm text-gray-500 font-mono">
                                        "No shortcuts match your search"
                                    </div>
                                }.into_any()
                            } else {
                                let mut grouped: std::collections::HashMap<&str, Vec<(&str, &str)>> = std::collections::HashMap::new();
                                for (cat, keys, label) in &items {
                                    grouped.entry(*cat).or_default().push((keys, label));
                                }
                                let mut sections = Vec::new();
                                for (cat, shortcuts) in grouped {
                                    let cat_str = cat.to_string();
                                    let mut shortcut_rows = Vec::new();
                                    for (keys, label) in shortcuts {
                                        let keys_owned = keys.to_string();
                                        let label_owned = label.to_string();
                                        shortcut_rows.push(view! {
                                            <div class="flex items-center justify-between py-1.5 border-b border-gray-50 dark:border-gray-800 last:border-0">
                                                <span class="text-sm text-gray-700 dark:text-gray-300">{label_owned}</span>
                                                <kbd class="px-2 py-0.5 text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 rounded-sm brutal-border">{keys_owned}</kbd>
                                            </div>
                                        }.into_any());
                                    }
                                    sections.push(view! {
                                        <div class="mb-4">
                                            <h3 class="text-xs font-bold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-2 font-mono">
                                                {cat_str}
                                            </h3>
                                            <div class="space-y-1">
                                                {shortcut_rows}
                                            </div>
                                        </div>
                                    }.into_any());
                                }
                                view! { <div>{sections}</div> }.into_any()
                            }}
                        </div>

                        <div class="px-4 py-2 border-t border-gray-100 dark:border-gray-700 text-xs text-gray-400 font-mono text-center">
                            "Press ? to toggle this panel"
                        </div>
                    </div>
                    </FocusTrap>
                </div>
            }
        })}
    }
}
