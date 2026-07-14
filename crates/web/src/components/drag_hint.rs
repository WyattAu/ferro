use leptos::prelude::*;

use crate::t;

/// Enhanced drop zone indicator and drag hint at the bottom of the file list.
///
/// Shows contextual hints based on whether dragging is active,
/// entries exist, and the files tab is active.
#[component]
pub fn DragHint(
    /// Whether data is currently loading.
    loading: ReadSignal<bool>,
    /// Whether there are entries to show.
    has_entries: Signal<bool>,
    /// Whether the Files tab is active.
    files_tab_active: Signal<bool>,
    /// Whether a drag operation is in progress over the container.
    #[prop(default = Signal::derive(|| false))]
    is_dragging: Signal<bool>,
) -> impl IntoView {
    view! {
        {move || {
            let show = !loading.get() && files_tab_active.get();
            if !show {
                return view! { <div class="hidden"></div> }.into_any();
            }

            if is_dragging.get() {
                view! {
                    <div class="border-t-2 border-dashed border-blue-400 bg-[var(--accent-subtle)] px-6 py-4 text-center transition-colors duration-200">
                        <div class="flex items-center justify-center gap-2 text-[var(--accent)]">
                            <svg class="w-5 h-5 animate-bounce" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                            </svg>
                            <span class="text-sm font-medium">"Drop files here to upload"</span>
                        </div>
                    </div>
                }.into_any()
            } else if has_entries.get() {
                view! {
                    <div class="border-t border-[var(--border-subtle)] dark:border-[var(--border-strong)] px-6 py-2 text-center">
                        <span class="text-xs text-[var(--text-tertiary)] dark:text-[var(--text-tertiary)]">{t!("drop.hint")}</span>
                    </div>
                }.into_any()
            } else {
                view! { <div class="hidden"></div> }.into_any()
            }
        }}
    }
}
