use leptos::prelude::*;

/// Badge component for labels and counts.
#[component]
pub fn Badge(
    #[prop(into, optional)] variant: String,
    #[prop(optional)] class: String,
    children: Children,
) -> impl IntoView {
    let v = if variant.is_empty() { "accent" } else { &variant };
    let class_str = format!("badge badge-{} {}", v, class);
    view! { <span class=class_str>{children()}</span> }
}

/// Avatar component for user images/initials.
#[component]
pub fn Avatar(
    #[prop(into, optional)] src: String,
    #[prop(into, optional)] alt: String,
    #[prop(into, optional)] initials: String,
    #[prop(optional)] size: u32,
) -> impl IntoView {
    let sz = if size == 0 { 36 } else { size };
    let size_style = format!("width:{}px;height:{}px;font-size:{}px", sz, sz, sz / 3);

    if !src.is_empty() {
        view! {
            <img src=src alt=alt style=size_style.clone() class="avatar" />
        }.into_any()
    } else {
        view! {
            <div class="avatar avatar-initials" style=size_style.clone() aria-label=alt>
                {initials}
            </div>
        }.into_any()
    }
}

/// Spinner/loading indicator.
#[component]
pub fn Spinner(#[prop(optional)] size: u32) -> impl IntoView {
    let sz = if size == 0 { 24 } else { size };
    let style = format!("width:{}px;height:{}px", sz, sz);
    view! { <div class="spinner" style=style role="status" aria-label="Loading" /> }
}

/// Divider component.
#[component]
pub fn Divider(#[prop(optional)] class: String) -> impl IntoView {
    let class_str = format!("border-t border-[var(--color-border)] {}", class);
    view! { <hr class=class_str /> }
}
