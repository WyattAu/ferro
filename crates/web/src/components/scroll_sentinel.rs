use leptos::prelude::*;
use leptos::html;

use leptos::html::Div;

use crate::t;

/// Placeholder element observed by IntersectionObserver for infinite scroll.
/// Shows a spinner when more entries exist beyond the current display window.
#[component]
pub fn ScrollSentinel(
    /// Total number of entries available.
    total: Signal<usize>,
    /// Number of entries currently displayed.
    displayed: ReadSignal<usize>,
    /// Whether data is currently loading.
    loading: ReadSignal<bool>,
    /// Whether the Files tab is active (not Favorites/Recent).
    files_tab_active: Signal<bool>,
    /// NodeRef for the IntersectionObserver target.
    sentinel_ref: NodeRef<Div>,
) -> impl IntoView {
    view! {
        {move || {
            let total_count = total.get();
            let display_count = displayed.get();
            (total_count > display_count && !loading.get() && files_tab_active.get()).then(|| view! {
                <div node_ref=sentinel_ref class="py-4 text-center text-sm text-muted font-mono" role="status" aria-live="polite">
                    <div class="animate-spin w-4 h-4 border-2 border-gray-300 dark:border-gray-600 border-t-accent rounded-full mx-auto mb-2"></div>
                    {t!("common.loading_more")}
                </div>
            })
        }}
    }
}
