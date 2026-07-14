use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[component]
pub fn DashboardPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (dashboard, set_dashboard) = signal(None::<api::DashboardResponse>);
    let (error_msg, set_error) = signal(String::new());

    Effect::new(move |_| {
        set_loading.set(true);
        set_error.set(String::new());
        spawn_local(async move {
            match api::get_dashboard().await {
                Ok(resp) => {
                    set_dashboard.set(Some(resp));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    });

    let format_bytes = |bytes: u64| -> String {
        if bytes == 0 {
            return "0 B".to_string();
        }
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut val = bytes as f64;
        let mut idx = 0;
        while val >= 1024.0 && idx < units.len() - 1 {
            val /= 1024.0;
            idx += 1;
        }
        format!("{:.1} {}", val, units[idx])
    };

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-auto px-2 sm:px-4 pt-16">
                <main id="main-content" class="max-w-7xl w-full mx-auto p-6">
                    <h1 class="text-2xl font-bold font-mono text-[var(--text-primary)] mb-6">{t!("dashboard.title")}</h1>

                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12" role="status" aria-busy="true">
                            <div class="text-sm text-[var(--text-tertiary)] font-mono">{t!("common.loading")}</div>
                        </div>
                    })}

                    {move || (!error_msg.get().is_empty() && !loading.get()).then(|| view! {
                        <div class="p-4 bg-[var(--danger-subtle)] border-l-4 border-l-[var(--danger)] rounded text-sm text-[var(--danger)]" role="alert">
                            <span class="font-bold">{t!("error.prefix")}</span> {error_msg}
                        </div>
                    })}

                    {move || dashboard.get().map(|data| {
                        let used = data.storage_used;
                        let total = data.storage_total;
                        let file_count = data.file_count;
                        let recent_files = data.recent_files.clone();
                        let shared_files = data.shared_files.clone();
                        let activity = data.activity.clone();
                        let used_display = format_bytes(used);
                        let total_display = if total > 0 { format_bytes(total) } else { "Unlimited".to_string() };
                        let pct_display = if total > 0 {
                            format!("{}%", (used as f64 / total as f64 * 100.0) as u32)
                        } else {
                            String::new()
                        };
                        let bar_width = if total > 0 {
                            let pct = (used as f64 / total as f64 * 100.0).min(100.0);
                            format!("width: {}%", pct)
                        } else {
                            "width: 0%".to_string()
                        };
                        view! {
                            // Storage usage bar
                            <section class="mb-8" aria-labelledby="storage-heading">
                                <h2 id="storage-heading" class="text-sm font-bold uppercase font-mono text-[var(--text-tertiary)] mb-3">{t!("dashboard.storage")}</h2>
                                <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm p-5 brutal-border">
                                    <div class="flex items-center justify-between mb-2">
                                        <span class="text-sm font-mono text-[var(--text-secondary)]">
                                            {used_display}
                                            " / "
                                            {total_display}
                                        </span>
                                        <span class="text-xs font-mono text-[var(--text-tertiary)]">{pct_display}</span>
                                    </div>
                                    <div class="w-full h-3 bg-[var(--border-subtle)] bg-[var(--bg-surface-raised)] rounded-full overflow-hidden" role="progressbar" aria-valuenow=used aria-valuemin="0" aria-valuemax=total aria-label=t!("dashboard.storage_progress")>
                                        <div class="h-full bg-[var(--accent)] rounded-full transition-all duration-500" style=bar_width></div>
                                    </div>
                                    <div class="mt-2 text-xs text-[var(--text-tertiary)] font-mono">
                                        {file_count} " file(s)"
                                    </div>
                                </div>
                            </section>

                            // Quick actions
                            <section class="mb-8" aria-labelledby="actions-heading">
                                <h2 id="actions-heading" class="sr-only">{t!("dashboard.quick_actions")}</h2>
                                <div class="flex flex-wrap gap-3">
                                    <a href="/ui/files/"
                                       class="inline-flex items-center gap-2 px-4 py-2.5 bg-[var(--accent)] text-[var(--text-on-accent)] text-sm font-bold uppercase rounded-lg hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                                       aria-label=t!("dashboard.upload")
                                    >
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>
                                        {t!("dashboard.upload")}
                                    </a>
                                    <a href="/ui/files/"
                                       class="inline-flex items-center gap-2 px-4 py-2.5 bg-[var(--bg-surface)] text-[var(--text-secondary)] text-sm font-bold uppercase rounded-lg border border-[var(--border-default)] hover:bg-[var(--interactive-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                                       aria-label=t!("dashboard.new_folder")
                                    >
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" /></svg>
                                        {t!("dashboard.new_folder")}
                                    </a>
                                    <a href="/ui/files/"
                                       class="inline-flex items-center gap-2 px-4 py-2.5 bg-[var(--bg-surface)] text-[var(--text-secondary)] text-sm font-bold uppercase rounded-lg border border-[var(--border-default)] hover:bg-[var(--interactive-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] min-h-[44px]"
                                       aria-label=t!("dashboard.new_document")
                                    >
                                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                                        {t!("dashboard.new_document")}
                                    </a>
                                </div>
                            </section>

                            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                                // Recent files
                                <DashboardSection title=t!("dashboard.recent_files")>
                                    <For
                                        each=move || recent_files.clone()
                                        key=|f| f.path.clone()
                                        let:file
                                    >
                                        {
                                            let file_name = file.path.rsplit('/').next().unwrap_or(&file.path).to_string();
                                            let ts = if file.modified_at.len() >= 19 { file.modified_at[..19].to_string() } else { file.modified_at.clone() };
                                            view! {
                                                <li class="px-4 py-3 hover:bg-[var(--interactive-hover)] transition-colors">
                                                    <div class="flex items-center justify-between">
                                                        <div class="min-w-0">
                                                            <div class="text-sm font-mono text-[var(--text-primary)] truncate" title=file.path.clone()>
                                                                {file_name}
                                                            </div>
                                                            <div class="text-xs text-[var(--text-tertiary)] font-mono">{ts}</div>
                                                        </div>
                                                    </div>
                                                </li>
                                            }
                                        }
                                    </For>
                                </DashboardSection>

                                // Shared with me
                                <DashboardSection title=t!("dashboard.shared_files")>
                                    <For
                                        each=move || shared_files.clone()
                                        key=|s| s.token.clone()
                                        let:share
                                    >
                                        {
                                            let share_path = share.path.clone();
                                            let file_name = share_path.rsplit('/').next().unwrap_or(&share_path).to_string();
                                            let expires = if share.expires_at.len() >= 10 { share.expires_at[..10].to_string() } else { share.expires_at.clone() };
                                            view! {
                                                <li class="px-4 py-3 hover:bg-[var(--interactive-hover)] transition-colors">
                                                    <div class="flex items-center justify-between">
                                                        <div class="min-w-0">
                                                            <div class="text-sm font-mono text-[var(--text-primary)] truncate" title=share_path>
                                                                {file_name}
                                                            </div>
                                                            <div class="text-xs text-[var(--text-tertiary)] font-mono">
                                                                {share.download_count} " downloads · expires " {expires}
                                                            </div>
                                                        </div>
                                                    </div>
                                                </li>
                                            }
                                        }
                                    </For>
                                </DashboardSection>
                            </div>

                            // Activity feed
                            <DashboardSection title=t!("dashboard.activity")>
                                <For
                                    each=move || activity.clone()
                                    key=|e| format!("{}{}", e.timestamp, e.path)
                                    let:entry
                                >
                                    {
                                        let action = entry.action.clone();
                                        let entry_path = entry.path.clone();
                                        let entry_ts = entry.timestamp.clone();
                                        let file_name = entry_path.rsplit('/').next().unwrap_or(&entry_path).to_string();
                                        let ts_display = if entry_ts.len() >= 19 { entry_ts[..19].to_string() } else { entry_ts };
                                        view! {
                                            <li class="px-4 py-3 hover:bg-[var(--interactive-hover)] transition-colors">
                                                <div class="flex items-center gap-3">
                                                    <span class="text-base shrink-0 font-mono" aria-hidden="true">
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
                                                            {action} " · " {ts_display}
                                                        </div>
                                                    </div>
                                                </div>
                                            </li>
                                        }
                                    }
                                </For>
                            </DashboardSection>
                        }
                    })}
                </main>
            </div>
        </div>
    }
}

/// Reusable dashboard section with a title, empty state, and list content.
#[component]
fn DashboardSection(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <section class="mt-6">
            <h2 class="text-sm font-bold uppercase font-mono text-[var(--text-tertiary)] mb-3">{title}</h2>
            <div class="bg-[var(--bg-surface)] rounded-xl shadow-sm brutal-border overflow-hidden">
                {children()}
            </div>
        </section>
    }
}
