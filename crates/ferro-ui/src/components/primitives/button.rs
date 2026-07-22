use leptos::prelude::*;

/// Button component with variants and sizes.
#[component]
pub fn Button(
    #[prop(into, optional)] variant: String,
    #[prop(into, optional)] size: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: String,
    children: Children,
) -> impl IntoView {
    let v = if variant.is_empty() { "secondary".to_string() } else { variant };
    let s = if size.is_empty() { String::new() } else { format!("btn-{}", size) };
    let d = if disabled { "disabled".to_string() } else { String::new() };
    let class_str = format!("btn btn-{v} {s} {d} {class}");

    view! {
        <button class=class_str disabled=disabled>
            {children()}
        </button>
    }
}
