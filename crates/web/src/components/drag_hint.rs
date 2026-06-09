use leptos::prelude::*;

use crate::t;

/// Subtle hint bar at the bottom of the file list reminding users they can drag-and-drop.
#[component]
pub fn DragHint(
    /// Whether data is currently loading.
    loading: ReadSignal<bool>,
    /// Whether there are entries to show.
    has_entries: Signal<bool>,
    /// Whether the Files tab is active.
    files_tab_active: Signal<bool>,
) -> impl IntoView {
    view! {
        {move || (!loading.get() && has_entries.get() && files_tab_active.get()).then(|| view! {
            <div class="border-t border-gray-100 px-6 py-2 text-center">
                <span class="text-xs text-gray-500">{t!("drop.hint")}</span>
            </div>
        })}
    }
}
