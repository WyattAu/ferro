use leptos::ev;
use leptos::html;
use leptos::prelude::*;

/// Reusable focus trap wrapper for modal dialogs.
///
/// When mounted, this component:
/// 1. Intercepts Tab/Shift+Tab to cycle focus within focusable children
/// 2. Auto-focuses the first focusable element on mount
/// 3. On unmount, restores focus to the previously focused element
#[component]
pub fn FocusTrap(children: Children) -> impl IntoView {
    let container_ref = NodeRef::<html::Div>::new();
    let (_prev_focus, _set_prev_focus) = signal(None::<web_sys::Element>);

    let _focusable_selector = "a[href],button:not([disabled]),textarea:not([disabled]),input:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex='-1']),[contenteditable='true']";

    /// Try to focus a DOM element by downcasting to HtmlElement.
    #[cfg(target_arch = "wasm32")]
    fn focus_element(el: &web_sys::Element) {
        use wasm_bindgen::JsCast;
        let _ = el.clone().dyn_into::<web_sys::HtmlElement>().ok().map(|h| h.focus());
    }

    // On mount: save previous focus and auto-focus first focusable element
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(el) = container_ref.get() else {
                return;
            };

            use wasm_bindgen::JsCast;
            if let Ok(el_element) = el.dyn_into::<web_sys::Element>() {
                if let Some(active) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|doc| doc.active_element())
                {
                    _set_prev_focus.set(Some(active));
                }

                if let Ok(Some(focusable)) = el_element.query_selector(_focusable_selector) {
                    focus_element(&focusable);
                }
            }
        }
    });

    // On keydown: trap Tab/Shift+Tab within focusable children
    let on_keydown = move |_ev: ev::KeyboardEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            if _ev.key() != "Tab" {
                return;
            }
            let Some(el) = container_ref.get() else {
                return;
            };

            use wasm_bindgen::JsCast;
            let Ok(el_element) = el.dyn_into::<web_sys::Element>() else {
                return;
            };

            let Ok(list) = el_element.query_selector_all(_focusable_selector) else {
                return;
            };
            let len = list.length();
            if len == 0 {
                return;
            }

            let mut elements = Vec::with_capacity(len as usize);
            for i in 0..len {
                if let Some(node) = list.item(i) {
                    if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                        elements.push(el);
                    }
                }
            }
            if elements.is_empty() {
                return;
            }

            let active = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|doc| doc.active_element());

            let idx = active
                .as_ref()
                .and_then(|active_el| elements.iter().position(|el| active_el.is_same_node(Some(el))));

            match idx {
                None => {
                    _ev.prevent_default();
                    if let Some(first) = elements.first() {
                        focus_element(first);
                    }
                }
                Some(i) if _ev.shift_key() && i == 0 => {
                    _ev.prevent_default();
                    if let Some(last) = elements.last() {
                        focus_element(last);
                    }
                }
                Some(i) if !_ev.shift_key() && i + 1 >= elements.len() => {
                    _ev.prevent_default();
                    if let Some(first) = elements.first() {
                        focus_element(first);
                    }
                }
                _ => {}
            }
        }
    };

    // On unmount: restore focus to previous element
    on_cleanup(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(prev) = _prev_focus.get() {
                focus_element(&prev);
            }
        }
    });

    view! {
        <div node_ref=container_ref on:keydown=on_keydown>
            {children()}
        </div>
    }
}
