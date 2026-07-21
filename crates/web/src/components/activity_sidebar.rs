use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::t;

/// Sidebar panel showing recent file activity (uploads, deletes, moves, etc.).
/// Self-contained: loads its own data when opened.
#[component]
pub fn ActivitySidebar(
    /// Whether the sidebar is visible.
    open: ReadSignal<bool>,
    /// Setter for sidebar visibility.
    set_open: WriteSignal<bool>,
) -> impl IntoView {
    let (entries, set_entries) = signal(Vec::<api::ActivityEntry>::new());

    let load_activity = move || {
        spawn_local(async move {
            match api::get_activity(50, 0).await {
                Ok(resp) => set_entries.set(resp.entries),
                Err(_) => set_entries.set(vec![]),
            }
        });
    };

    // Reload activity whenever the sidebar opens.
    Effect::new(move |_| {
        if open.get() {
            load_activity();
        }
    });

    view! {
        {move || open.get().then(|| view! {
            <div class="fixed right-0 top-0 bottom-0 w-72 brutal-border border-l surface overflow-y-auto transition-all duration-200 z-30 shadow-xl">
                <div class="px-4 py-3 border-b border-[var(--border-default)] flex items-center justify-between">
                    <h3 class="text-label font-mono text-[var(--text-primary)]">{t!("aria.activity_heading")}</h3>
                    <button
                        class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] rounded focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center"
                        on:click=move |_| set_open.set(false)
                        aria-label=t!("aria.close_activity")
                    >
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                    </button>
                </div>
                <div class="p-4 space-y-3">
                    <For
                        each=move || entries.get()
                        key=|e| format!("{}{}", e.timestamp.clone(), e.path.clone())
                        let:entry
                    >
                        {
                            let action = entry.action.clone();
                            let entry_path = entry.path.clone();
                            let entry_ts = entry.timestamp.clone();
                            let file_name = entry_path.rsplit('/').next().unwrap_or(&entry_path).to_string();
                            let ts_display = if entry_ts.len() >= 19 { entry_ts[..19].to_string() } else { entry_ts };
                            view! {
                                <div class="flex items-start gap-2">
                                    <span class="text-base mt-0.5 shrink-0 font-mono" aria-hidden="true">
                                        {match action.as_str() {
                                            "upload" => "\u{2191}",
                                            "delete" => "\u{2715}",
                                            "create_folder" => "\u{2192}",
                                            "copy" => "\u{2295}",
                                            "move" => "\u{2192}",
                                            _ => "\u{2022}",
                                        }}
                                    </span>
                                    <div class="min-w-0">
                                        <div class="text-sm font-mono text-[var(--text-primary)] truncate" title=entry_path.clone()>
                                            {file_name}
                                        </div>
                                        <div class="text-xs text-[var(--text-tertiary)] font-mono">
                                            {action} " " {ts_display}
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    </For>
                    {move || entries.with(Vec::is_empty).then(|| view! {
                        <div class="text-sm text-[var(--text-tertiary)] text-center py-4">{t!("empty.recent")}</div>
                    })}
                 </div>
             </div>
         })}
    }
}
