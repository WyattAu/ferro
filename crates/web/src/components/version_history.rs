use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::focus_trap::FocusTrap;
use crate::components::icons::{Icon, IconName};
use crate::components::toast::ToastContext;
use crate::t;
use ferro_common::format::format_size;

/// Version history panel component.
///
/// Shows a list of file versions with restore and compare functionality.
/// Opens as a modal dialog with a focused panel layout.
#[component]
pub fn VersionHistory(
    /// Whether the panel is visible.
    open: ReadSignal<bool>,
    /// Setter for panel visibility.
    set_open: WriteSignal<bool>,
    /// File path to show versions for.
    file_path: ReadSignal<String>,
) -> impl IntoView {
    let (versions, set_versions) = signal(Vec::<api::FileVersion>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (restoring_id, set_restoring_id) = signal(None::<u64>);
    let (compare_mode, set_compare_mode) = signal(false);
    let (selected_for_compare, set_selected_for_compare) = signal(Vec::<u64>::new());
    let (diff_result, set_diff_result) = signal(None::<api::DiffResponse>);

    let load_versions = move |path: String| {
        set_loading.set(true);
        set_error.set(None);
        set_diff_result.set(None);
        set_selected_for_compare.set(vec![]);
        set_compare_mode.set(false);
        let p = path.clone();
        spawn_local(async move {
            match api::list_versions(&p).await {
                Ok(resp) => {
                    let mut sorted = resp.versions;
                    sorted.sort_by_key(|v| std::cmp::Reverse(v.id));
                    set_versions.set(sorted);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    // Reload versions whenever the panel opens with a non-empty path.
    Effect::new(move |_| {
        if open.get() {
            let path = file_path.get();
            if !path.is_empty() {
                load_versions(path);
            }
        }
    });

    let do_restore = move |version_id: u64| {
        let path = file_path.get();
        set_restoring_id.set(Some(version_id));
        spawn_local(async move {
            match api::restore_version(&path, version_id).await {
                Ok(()) => {
                    ToastContext::success(t!("toast.version_restored"));
                    set_restoring_id.set(None);
                    load_versions(path);
                }
                Err(e) => {
                    ToastContext::error(format!("Restore failed: {}", e));
                    set_restoring_id.set(None);
                }
            }
        });
    };

    let toggle_compare_select = move |version_id: u64| {
        set_selected_for_compare.update(|sel| {
            if sel.contains(&version_id) {
                sel.retain(|&id| id != version_id);
            } else if sel.len() < 2 {
                sel.push(version_id);
            }
        });
    };

    let do_compare = move |_: ev::MouseEvent| {
        let sel = selected_for_compare.get();
        if sel.len() != 2 {
            return;
        }
        let from = sel[0].min(sel[1]);
        let to = sel[0].max(sel[1]);
        let path = file_path.get();
        spawn_local(async move {
            match api::diff_versions(&path, from, to).await {
                Ok(diff) => set_diff_result.set(Some(diff)),
                Err(e) => ToastContext::error(format!("Diff failed: {}", e)),
            }
        });
    };

    let close_panel = move |_: ev::MouseEvent| {
        set_open.set(false);
        set_compare_mode.set(false);
        set_selected_for_compare.set(vec![]);
        set_diff_result.set(None);
    };

    let on_backdrop_click = move |ev: ev::MouseEvent| {
        if ev.target() == ev.current_target() {
            set_open.set(false);
        }
    };

    let on_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            set_open.set(false);
        }
    };

    view! {
        {move || open.get().then(|| view! {
            <div
                class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center p-4 transition-opacity duration-200"
                on:click=on_backdrop_click
                on:keydown=on_keydown
            >
                <FocusTrap>
                    <div
                        class="brutal-block rounded shadow-xl w-full max-w-lg mx-auto transition-all duration-200 flex flex-col max-h-[80vh]"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="version-history-title"
                        tabindex="-1"
                    >
                        // Header
                        <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-default)]">
                            <div class="flex items-center gap-2">
                                <Icon name=IconName::Clock class="w-5 h-5 text-[var(--text-tertiary)]".to_string() />
                                <h3 id="version-history-title" class="text-lg font-semibold font-mono text-[var(--text-primary)]">
                                    {t!("dialog.version_history.title")}
                                </h3>
                            </div>
                            <div class="flex items-center gap-2">
                                <button
                                    class=move || {
                                        let base = "px-3 py-1.5 text-xs font-bold uppercase rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[36px]";
                                        if compare_mode.get() {
                                            format!("{} bg-[var(--accent)] text-[var(--text-on-accent)] hover:bg-[var(--accent-hover)]", base)
                                        } else {
                                            format!("{} text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)]", base)
                                        }
                                    }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        let new_mode = !compare_mode.get();
                                        set_compare_mode.set(new_mode);
                                        if !new_mode {
                                            set_selected_for_compare.set(vec![]);
                                            set_diff_result.set(None);
                                        }
                                    }
                                    aria-label=t!("dialog.version_history.compare_toggle")
                                >
                                    {t!("dialog.version_history.compare")}
                                </button>
                                <button
                                    class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded min-w-[44px] min-h-[44px] flex items-center justify-center"
                                    aria-label=t!("aria.close_dialog")
                                    on:click=close_panel
                                >
                                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                        </div>

                        // Compare bar
                        {move || compare_mode.get().then(|| view! {
                            <div class="px-4 py-2 bg-[var(--accent-subtle)] border-b border-[var(--border-default)] flex items-center justify-between">
                                <span class="text-sm text-[var(--accent)] font-mono">
                                    {move || {
                                        let count = selected_for_compare.with(|s| s.len());
                                        match count {
                                            0 => t!("dialog.version_history.select_two"),
                                            1 => t!("dialog.version_history.selected_one"),
                                            _ => t!("dialog.version_history.selected_two"),
                                        }
                                    }}
                                </span>
                                <button
                                    class="px-3 py-1.5 text-xs font-bold uppercase bg-[var(--accent)] text-[var(--text-on-accent)] rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
                                    disabled=move || selected_for_compare.with(|s| s.len()) != 2
                                    on:click=do_compare
                                >
                                    {t!("dialog.version_history.diff")}
                                </button>
                            </div>
                        })}

                        // Version list
                        <div class="overflow-y-auto flex-1 p-4 space-y-2" role="list" aria-label=t!("dialog.version_history.version_list")>
                            {move || loading.get().then(|| view! {
                                <div class="flex items-center justify-center py-8">
                                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                                </div>
                            })}

                            {move || error.get().map(|e| view! {
                                <div class="p-3 bg-[var(--danger-subtle)] border-l-4 border-l-[var(--danger)] rounded text-sm text-[var(--danger)]" role="alert">
                                    {e}
                                </div>
                            })}

                            {move || (!loading.get() && error.get().is_none() && versions.with(Vec::is_empty)).then(|| view! {
                                <div class="text-center py-8 text-[var(--text-tertiary)]" role="status">
                                    <Icon name=IconName::Clock class="w-12 h-12 mx-auto mb-3 text-[var(--text-tertiary)]".to_string() />
                                    <div class="text-sm font-medium">{t!("dialog.version_history.empty")}</div>
                                    <div class="text-xs text-[var(--text-tertiary)] mt-1">{t!("dialog.version_history.empty_hint")}</div>
                                </div>
                            })}

                            <For
                                each=move || versions.get()
                                key=|v| v.id
                                let:version
                            >
                                {
                                    let vid = version.id;
                                    let vauthor = version.author.clone();
                                    let vsize = format_size(version.size);
                                    let vmodified = version.modified_at.clone();
                                    let vnote = version.note.clone();
                                    let _vpath = file_path.get();
                                    let is_restoring = move || restoring_id.get() == Some(vid);
                                    let is_selected = move || compare_mode.get() && selected_for_compare.with(|s| s.contains(&vid));
                                    let is_latest = versions.with(|vs| vs.first().map(|v| v.id) == Some(vid));

                                    let ts_display = if vmodified.len() >= 19 {
                                        vmodified[..19].to_string()
                                    } else {
                                        vmodified
                                    };

                                    view! {
                                        <div
                                            class=move || {
                                                let base = "flex items-center gap-3 p-3 rounded-lg border transition-colors";
                                                if is_selected() {
                                                    format!("{} bg-[var(--accent-subtle)] border-[var(--border-default)]", base)
                                                } else {
                                                    format!("{} bg-[var(--bg-surface)] border-[var(--border-default)] hover:bg-[var(--interactive-hover)]", base)
                                                }
                                            }
                                            role="listitem"
                                        >
                                            {compare_mode.get().then(|| view! {
                                                <input
                                                    type="checkbox"
                                                    class="rounded border text-[var(--accent)] focus:ring-[var(--border-focus)] shrink-0"
                                                    prop:checked=is_selected
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        toggle_compare_select(vid);
                                                    }
                                                    aria-label=format!("Select version {} for comparison", vid)
                                                />
                                            })}

                                            <div class="flex-1 min-w-0">
                                                <div class="flex items-center gap-2">
                                                    <span class="font-mono text-sm font-semibold text-[var(--text-primary)]">
                                                        {format!("v{}", vid)}
                                                    </span>
                                                    {is_latest.then(|| view! {
                                                        <span class="px-1.5 py-0.5 text-[10px] font-bold uppercase bg-[var(--success-subtle)] text-[var(--success)] rounded">
                                                            {t!("dialog.version_history.current")}
                                                        </span>
                                                    })}
                                                </div>
                                                <div class="text-xs text-[var(--text-tertiary)] font-mono mt-0.5">
                                                    <span>{ts_display}</span>
                                                    <span class="mx-1">"·"</span>
                                                    <span>{vsize}</span>
                                                    <span class="mx-1">"·"</span>
                                                    <span>{vauthor}</span>
                                                </div>
                                                {vnote.as_ref().map(|n| view! {
                                                    <div class="text-xs text-[var(--text-tertiary)] mt-0.5 italic">{n.clone()}</div>
                                                })}
                                            </div>

                                            {(!compare_mode.get()).then(|| view! {
                                                <button
                                                    class="px-3 py-1.5 text-xs font-bold uppercase bg-[var(--accent)] text-[var(--text-on-accent)] rounded hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[36px] whitespace-nowrap"
                                                    disabled=is_restoring
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        do_restore(vid);
                                                    }
                                                    aria-label=format!("Restore version {}", vid)
                                                >
                                                    {move || if is_restoring() {
                                                        t!("dialog.version_history.restoring")
                                                    } else {
                                                        t!("dialog.version_history.restore")
                                                    }}
                                                </button>
                                            })}
                                        </div>
                                    }
                                }
                            </For>
                        </div>

                        // Diff result
                        {move || diff_result.get().map(|diff| view! {
                            <div class="border-t border-[var(--border-default)] px-4 py-3 max-h-[40vh] overflow-y-auto">
                                <div class="flex items-center justify-between mb-2">
                                    <h4 class="text-sm font-semibold font-mono text-[var(--text-primary)]">
                                        {format!("Diff: v{} → v{}", diff.from_version, diff.to_version)}
                                    </h4>
                                    <button
                                        class="text-xs text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded px-2 py-1"
                                        on:click=move |_| set_diff_result.set(None)
                                        aria-label="Close diff"
                                    >
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                        </svg>
                                    </button>
                                </div>
                                <div class="text-xs text-[var(--text-tertiary)] font-mono mb-2">
                                    <span class="text-[var(--success)]">+{diff.stats.additions}</span>
                                    <span class="mx-1">/</span>
                                    <span class="text-[var(--danger)]">-{diff.stats.deletions}</span>
                                    <span class="mx-1">/</span>
                                    <span>{diff.stats.unchanged} unchanged</span>
                                </div>
                                {if diff.is_binary {
                                    view! {
                                        <div class="text-sm text-[var(--text-tertiary)] italic py-2">
                                            {t!("dialog.version_history.binary_diff")}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <pre class="text-xs font-mono bg-[var(--bg-inset)] rounded p-2 overflow-x-auto"><code>
                                            {diff.lines.iter().map(|line| {
                                                let class = match line.type_.as_str() {
                                                    "added" => "text-[var(--success)] bg-[var(--success-subtle)]",
                                                    "removed" => "text-[var(--danger)] bg-[var(--danger-subtle)]",
                                                    _ => "text-[var(--text-secondary)]",
                                                };
                                                let prefix = match line.type_.as_str() {
                                                    "added" => "+",
                                                    "removed" => "-",
                                                    _ => " ",
                                                };
                                                view! {
                                                    <div class=class>
                                                        <span class="inline-block w-6 text-[var(--text-tertiary)] text-right mr-2">{prefix}</span>
                                                        {line.content.clone()}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </code></pre>
                                    }.into_any()
                                }}
                            </div>
                        })}
                    </div>
                </FocusTrap>
            </div>
        })}
    }
}
