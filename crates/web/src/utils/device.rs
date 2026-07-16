use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportSize {
    pub width: f64,
    pub height: f64,
}

impl Default for ViewportSize {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
        }
    }
}

pub fn use_viewport_size() -> Signal<ViewportSize> {
    let (size, set_size) = signal(ViewportSize::default());

    #[cfg(target_arch = "wasm32")]
    {

        let read_size = move || {
            if let Some(window) = web_sys::window() {
                let width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1024.0);
                let height = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(768.0);
                set_size.set(ViewportSize { width, height });
            }
        };

        read_size();

        let on_resize = Closure::<dyn FnMut()>::new(move || {
            read_size();
        });

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
            on_resize.forget();
        }
    }

    size.into()
}

pub fn use_is_mobile() -> Signal<bool> {
    let size = use_viewport_size();
    Signal::derive(move || size.get().width < 768.0)
}

pub fn use_is_tablet() -> Signal<bool> {
    let size = use_viewport_size();
    Signal::derive(move || {
        let w = size.get().width;
        (768.0..1024.0).contains(&w)
    })
}
