use leptos::*;
use std::time::Duration;

/// Debounce a value by the given delay.
///
/// Returns a signal that updates only after the input has been still
/// for the specified duration.
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
