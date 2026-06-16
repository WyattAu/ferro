#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

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
            keys: "F2",
            label: t!("shortcuts.rename"),
        },
        ShortcutEntry {
            keys: "Enter",
            label: t!("shortcuts.open"),
        },
        ShortcutEntry {
            keys: "/",
            label: t!("shortcuts.search_focus"),
        },
        ShortcutEntry {
            keys: "Ctrl+D",
            label: t!("shortcuts.toggle_dark_mode"),
        },
        ShortcutEntry {
            keys: "Ctrl+Shift+N",
            label: t!("shortcuts.new_note"),
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
