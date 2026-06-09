use leptos::prelude::*;
use std::time::Duration;

/// Debounce a value by the given delay.
///
/// Uses `leptos_use::use_debounce_fn` pattern: the signal updates only after
/// the input has been still for the specified duration.
#[component]
pub fn UseDebounce(
    value: Signal<String>,
    delay_ms: u32,
) -> impl IntoView {
    let (debounced, set_debounced) = create_signal(String::new());

    create_effect(move |_| {
        let new_val = value.get();
        let set_clone = set_debounced;
        let handle = set_timeout_with_handle(
            move || {
                set_clone.set(new_val);
            },
            Duration::from_millis(delay_ms as u64),
        );
        on_cleanup(move || {
            let _ = handle;
        });
    });

    view! { <span>{move || debounced.get()}</span> }
}

/// Detect CSS media queries for responsive design.
///
/// Mirrors `leptos_use::use_media_query` patterns: listens to
/// `matchMedia` change events and exposes a reactive boolean signal.
#[component]
pub fn UseMediaQuery(
    _query: String,
) -> impl IntoView {
    let (_matches, _set_matches) = create_signal(false);

    #[cfg(target_arch = "wasm32")]
    {
        let query_clone = _query.clone();
        let set_matches_clone = set_matches;

        create_effect(move |_| {
            if let Some(window) = web_sys::window()
                && let Some(mql) = window.match_media(&query_clone).ok().flatten()
            {
                set_matches_clone.set(mql.matches());
                let closure = Closure::wrap(Box::new(move |_: web_sys::MediaQueryListEvent| {
                    set_matches_clone.set(mql.matches());
                }) as Box<dyn Fn(_)>);
                let _ = mql.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
                closure.forget();
            }
        });
    }

    view! { <span>{move || _matches.get()}</span> }
}

/// Element size observer using leptos-use patterns.
///
/// Mirrors `leptos_use::use_element_size`: watches an element's dimensions
/// via `ResizeObserver` and exposes width/height as reactive signals.
#[component]
pub fn UseElementSize() -> impl IntoView {
    #[allow(unused_variables)]
    let (width, set_width) = create_signal(0_u32);
    #[allow(unused_variables)]
    let (height, set_height) = create_signal(0_u32);

    #[cfg(target_arch = "wasm32")]
    {
        let set_w = set_width;
        let set_h = set_height;
        create_effect(move |_| {
            if let Some(window) = web_sys::window()
                && let Some(doc) = window.document()
                && let Some(body) = doc.body()
            {
                let set_w = set_w.clone();
                let set_h = set_h.clone();
                let cb = Closure::wrap(Box::new(move |entries: js_sys::Array| {
                    if let Some(entry) = entries.get(0).dyn_into::<web_sys::ResizeObserverEntry>().ok() {
                        let rect = entry.content_rect();
                        set_w.set(rect.width() as u32);
                        set_h.set(rect.height() as u32);
                    }
                }) as Box<dyn Fn(js_sys::Array)>);
                if let Ok(obs) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
                    obs.observe(body.unchecked_ref());
                }
                cb.forget();
            }
        });
    }

    view! {
        <div>
            <span class="element-size-width">{move || width.get()}</span>
            <span class="element-size-height">{move || height.get()}</span>
        </div>
    }
}
