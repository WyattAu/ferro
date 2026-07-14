use crate::t;
use leptos::prelude::*;

#[component]
pub fn Breadcrumb(current_path: Signal<String>, navigate: Callback<String>) -> impl IntoView {
    let segments = move || {
        let path = current_path.get();
        let mut segments: Vec<(String, String)> = vec![("/".to_string(), t!("nav.home").to_string())];
        if path != "/" {
            let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
            let mut built = String::new();
            for part in parts {
                built = format!("{}/{}", built, part);
                segments.push((built.clone(), part.to_string()));
            }
        }
        segments
    };

    view! {
        <nav aria-label=t!("breadcrumb.aria") class="flex items-center gap-1 text-sm min-w-0 overflow-hidden">
            <ol class="flex items-center gap-1 list-none m-0 p-0 overflow-hidden">
                <For
                    each=segments
                    key=|(path, _)| path.clone()
                    let:segment
                >
                    {
                        let (path, label) = segment;
                        let is_root = path == "/";
                        let p = path.clone();
                        let is_current = move || path == current_path.get();
                        view! {
                            <li class="flex items-center">
                                {(!is_root).then(|| view! {
                                     <span class="text-[var(--text-tertiary)] mx-1" aria-hidden="true">{t!("breadcrumb.separator")}</span>
                                })}
                                 <button
                                     class="text-[var(--accent)] hover:text-blue-800 focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 rounded truncate max-w-[120px] sm:max-w-none min-w-[44px] min-h-[44px] flex items-center justify-center"
                                     attr:aria-current=move || if is_current() { Some("page") } else { None }
                                     on:click=move |_| navigate.run(p.clone())
                                >
                                    {label}
                                </button>
                            </li>
                        }
                    }
                </For>
            </ol>
        </nav>
    }
}
