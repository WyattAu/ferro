use leptos::*;

#[component]
pub fn Tooltip(
    text: String,
    #[prop(default = "top".to_string())] position: String,
    children: Children,
) -> impl IntoView {
    let position_class = match position.as_str() {
        "bottom" => "top-full left-1/2 -translate-x-1/2 mt-1.5",
        "left" => "right-full top-1/2 -translate-y-1/2 mr-1.5",
        "right" => "left-full top-1/2 -translate-y-1/2 ml-1.5",
        _ => "bottom-full left-1/2 -translate-x-1/2 mb-1.5",
    };

    view! {
        <div class="relative inline-flex group/tooltip">
            {children()}
            <span
                class={format!(
                    "absolute z-50 px-2 py-1 text-xs font-medium text-white bg-gray-900 dark:bg-gray-700 rounded shadow-lg whitespace-nowrap opacity-0 group-hover/tooltip:opacity-100 group-focus-within/tooltip:opacity-100 pointer-events-none transition-opacity duration-150 {}",
                    position_class
                )}
                role="tooltip"
            >
                {text}
            </span>
        </div>
    }
}
