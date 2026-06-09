use crate::components::file_icon::{FileIcon, file_type_from_extension};
use leptos::html;
use leptos::prelude::*;

fn is_previewable_image(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.rsplit('.').next().unwrap_or(""),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg"
    )
}

#[component]
pub fn Thumbnail(
    path: String,
    name: String,
    #[prop(default = 40)] size: u32,
    #[prop(default = String::new())] class: String,
) -> impl IntoView {
    let show_image = is_previewable_image(&name);
    let file_type = file_type_from_extension(&name);
    let thumb_url = format!("/api/thumbnail{}", path);

    let (load_error, set_load_error) = signal(false);
    let (in_view, set_in_view) = signal(false);

    let wrapper_ref = NodeRef::<html::Div>::new();

    Effect::new(move |_| {
        let el = wrapper_ref.get();
        if el.is_none() {
            return;
        }
        use wasm_bindgen::JsCast;

        let callback: wasm_bindgen::closure::Closure<
            dyn Fn(js_sys::Array, web_sys::IntersectionObserver),
        > = wasm_bindgen::closure::Closure::new(
            move |entries: js_sys::Array, _: web_sys::IntersectionObserver| {
                for i in 0..entries.length() {
                    if let Ok(entry) = entries
                        .get(i)
                        .dyn_into::<web_sys::IntersectionObserverEntry>()
                        && entry.is_intersecting()
                    {
                        set_in_view.set(true);
                    }
                }
            },
        );
        let callback_fn: &js_sys::Function = callback.as_ref().unchecked_ref();
        let opts = web_sys::IntersectionObserverInit::new();
        opts.set_root_margin("100px");
        let observer = web_sys::IntersectionObserver::new_with_options(callback_fn, &opts).unwrap();
        if let Some(el) = el {
            observer.observe(el.unchecked_ref());
        }
        // Leak callback to avoid Send/Sync issues (safe in WASM single-threaded env)
        std::mem::forget(callback);
        on_cleanup(move || {
            observer.disconnect();
        });
    });

    let size_class = format!("w-{} h-{}", size, size);
    let container_class = format!("relative flex items-center justify-center {}", class);
    let size_val = size as f32;

    view! {
        <div class=container_class node_ref=wrapper_ref>
            {move || {
                if !show_image || load_error.get() {
                    view! { <span><FileIcon file_type=file_type size=size /></span> }.into_any()
                } else if !in_view.get() {
                    let c = format!("{} rounded bg-gray-100 dark:bg-gray-700 animate-pulse", size_class);
                    view! { <div class=c /> }.into_any()
                } else {
                    let cls = format!(
                        "{} rounded object-cover transition-opacity duration-300",
                        size_class
                    );
                    let st = format!("max-width:{}px; max-height:{}px;", size_val, size_val);
                    view! {
                        <img
                            class=cls
                            src=thumb_url.clone()
                            alt=name.clone()
                            loading="lazy"
                            style=st
                            on:error=move |_| set_load_error.set(true)
                        />
                    }.into_any()
                }
            }}
        </div>
    }
}
