use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::components::clipboard::use_clipboard_state;
use crate::components::command_palette::use_command_palette_state;
use crate::t;

#[derive(Debug, Clone)]
struct ShortcutEntry {
    keys: &'static str,
    label: &'static str,
}

fn shortcuts_list() -> Vec<ShortcutEntry> {
    vec![
        ShortcutEntry {
            keys: "Ctrl+K",
            label: t!("shortcuts.command_palette"),
        },
        ShortcutEntry {
            keys: "Ctrl+N",
            label: t!("shortcuts.new_folder"),
        },
        ShortcutEntry {
            keys: "Ctrl+U",
            label: t!("shortcuts.upload"),
        },
        ShortcutEntry {
            keys: "Ctrl+X",
            label: t!("shortcuts.cut"),
        },
        ShortcutEntry {
            keys: "Ctrl+C",
            label: t!("shortcuts.copy"),
        },
        ShortcutEntry {
            keys: "Ctrl+V",
            label: t!("shortcuts.paste"),
        },
        ShortcutEntry {
            keys: "Ctrl+A",
            label: t!("shortcuts.select_all"),
        },
        ShortcutEntry {
            keys: "Ctrl+F",
            label: t!("shortcuts.search"),
        },
        ShortcutEntry {
            keys: "Del",
            label: t!("shortcuts.delete"),
        },
        ShortcutEntry {
            keys: "Esc",
            label: t!("shortcuts.close"),
        },
        ShortcutEntry {
            keys: "?",
            label: t!("shortcuts.help"),
        },
    ]
}

#[component]
pub fn KeyboardShortcuts() -> impl IntoView {
    let (show_help, set_show_help) = signal(false);
    let _cmd_palette = use_command_palette_state();
    let _clipboard = use_clipboard_state();

    #[cfg(target_arch = "wasm32")]
    {
        let cmd_palette = _cmd_palette;

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |ev: web_sys::KeyboardEvent| {
                        let key = ev.key();
                        let ctrl = ev.ctrl_key() || ev.meta_key();
                        let shift = ev.shift_key();

                        if key == "?" && !shift {
                            set_show_help.update(|v| *v = !*v);
                            return;
                        }

                        if key == "Escape" {
                            if show_help.get() {
                                set_show_help.set(false);
                            }
                            return;
                        }

                        if ctrl {
                            match key.as_str() {
                                "k" | "K" => {
                                    ev.prevent_default();
                                    cmd_palette.toggle();
                                }
                                "n" | "N" => {
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:new-folder").unwrap(),
                                        );
                                    }
                                }
                                "u" | "U" => {
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:upload").unwrap(),
                                        );
                                    }
                                }
                                "a" | "A" => {
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:select-all").unwrap(),
                                        );
                                    }
                                }
                                "f" | "F" => {
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:search").unwrap(),
                                        );
                                    }
                                }
                                "c" | "C" => {
                                    if is_input_focused() {
                                        return;
                                    }
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:copy").unwrap(),
                                        );
                                    }
                                }
                                "x" | "X" => {
                                    if is_input_focused() {
                                        return;
                                    }
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:cut").unwrap(),
                                        );
                                    }
                                }
                                "v" | "V" => {
                                    if is_input_focused() {
                                        return;
                                    }
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:paste").unwrap(),
                                        );
                                    }
                                }
                                _ => {}
                            }
                            return;
                        }

                        match key.as_str() {
                            "Delete" => {
                                if !is_input_focused() {
                                    ev.prevent_default();
                                    if let Some(window) = web_sys::window() {
                                        let _ = window.dispatch_event(
                                            &web_sys::Event::new("ferro:delete-selected").unwrap(),
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    },
                )
                    as Box<dyn Fn(web_sys::KeyboardEvent)>);
                let _ = document
                    .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                std::mem::forget(cb);
            }
        }
    }

    let close_help = set_show_help;

    view! {
        {move || show_help.get().then(|| {
            let entries = shortcuts_list();
            view! {
                <div
                    class="fixed inset-0 bg-black bg-opacity-50 z-[70] flex items-center justify-center backdrop-blur-sm"
                    role="dialog"
                    aria-label=t!("shortcuts.title")
                    on:click=move |_| close_help.set(false)
                >
                    <div
                        class="brutal-block rounded shadow-2xl w-[calc(100%-2rem)] sm:w-full sm:max-w-md mx-auto overflow-hidden surface"
                        on:click=move |ev| ev.stop_propagation()
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Escape" {
                                close_help.set(false);
                            }
                        }
                    >
                        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200">
                            <h2 class="text-section">{t!("shortcuts.title")}</h2>
                            <button
                                class="p-1 rounded-sm opacity-60 hover:opacity-100 transition-opacity font-mono"
                                aria-label=t!("aria.close_dialog")
                                on:click=move |_| close_help.set(false)
                            >
                                <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                        <div class="px-4 py-3 max-h-[60vh] overflow-y-auto">
                            {entries.into_iter().map(|entry| {
                                view! {
                                    <div class="flex items-center justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                                        <span class="text-sm text-gray-700 dark:text-gray-300">{entry.label}</span>
                                        <kbd class="px-2 py-0.5 text-xs font-mono text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-700 rounded-sm brutal-border">{entry.keys}</kbd>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>
            }
        })}
    }
}

#[cfg(target_arch = "wasm32")]
fn is_input_focused() -> bool {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(active) = doc.active_element() {
                if let Ok(el) = active.dyn_into::<web_sys::HtmlElement>() {
                    let tag = el.tag_name().to_lowercase();
                    return tag == "input"
                        || tag == "textarea"
                        || tag == "select"
                        || el.get_attribute("contenteditable").is_some();
                }
            }
        }
    }
    false
}
