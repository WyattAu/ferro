use leptos::*;

use crate::api;
use crate::components::theme_toggle::{ThemeToggle, provide_theme_state};
use ferro_common::format::format_size;

#[component]
pub fn AdminPage() -> impl IntoView {
    provide_theme_state();
    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900">
            <div class="max-w-7xl mx-auto py-8">
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-2xl font-bold font-mono text-gray-900 tracking-tight">"Admin Dashboard"</h1>
                    <ThemeToggle />
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    <div class="surface brutal-border rounded-lg shadow-concrete p-6">
                        <h2 class="text-label font-mono text-gray-900 mb-4">"Storage"</h2>
                        <StorageStatsCard />
                    </div>

                    <div class="surface brutal-border rounded-lg shadow-concrete p-6">
                        <h2 class="text-label font-mono text-gray-900 mb-4">"Share Links"</h2>
                        <ShareLinksCard />
                    </div>

                    <div class="surface brutal-border rounded-lg shadow-concrete p-6">
                        <h2 class="text-label font-mono text-gray-900 mb-4">"Recent Activity"</h2>
                        <AuditLogCard />
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn StorageStatsCard() -> impl IntoView {
    let (stats, set_stats) = create_signal(None::<serde_json::Value>);
    let (loading, set_loading) = create_signal(true);

    create_effect(move |_| {
        spawn_local(async move {
            match api::fetch_json("/api/storage/stats").await {
                Ok(data) => {
                    set_stats.set(Some(data));
                    set_loading.set(false);
                }
                Err(_) => set_loading.set(false),
            }
        });
    });

    view! {
        <div>
            {move || loading.get().then(|| view! {
                <div class="text-sm text-gray-500" role="status" aria-live="polite">"Loading..."</div>
            })}
            {move || stats.get().map(|s| view! {
                <div class="space-y-3">
                    <div class="flex justify-between">
                        <span class="text-gray-600">"Files"</span>
                        <span class="font-bold font-mono text-gray-900">{s.get("files").and_then(|v| v.as_u64()).unwrap_or(0)}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">"Collections"</span>
                        <span class="font-bold font-mono text-gray-900">{s.get("collections").and_then(|v| v.as_u64()).unwrap_or(0)}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">"Total Size"</span>
                        <span class="font-bold font-mono text-gray-900">{format_size(s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0))}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">"CAS Dedup"</span>
                        <span class={if s.get("cas").and_then(|c| c.get("enabled")).and_then(|e| e.as_bool()).unwrap_or(false) { "text-green-600" } else { "text-gray-500" }}>
                            {if s.get("cas").and_then(|c| c.get("enabled")).and_then(|e| e.as_bool()).unwrap_or(false) { "Enabled" } else { "Disabled" }}
                        </span>
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
fn ShareLinksCard() -> impl IntoView {
    let (shares, set_shares) = create_signal(vec![]);
    let (loading, set_loading) = create_signal(true);

    create_effect(move |_| {
        spawn_local(async move {
            match api::fetch_json("/api/shares").await {
                Ok(data) => {
                    let list = data
                        .get("shares")
                        .and_then(|s| s.as_array())
                        .cloned()
                        .unwrap_or_default();
                    set_shares.set(list);
                    set_loading.set(false);
                }
                Err(_) => set_loading.set(false),
            }
        });
    });

    view! {
        <div>
            {move || loading.get().then(|| view! {
                <div class="text-sm text-gray-500" role="status" aria-live="polite">"Loading..."</div>
            })}
            {move || (!loading.get() && shares.with(Vec::is_empty)).then(|| view! {
                <div class="text-sm text-gray-500">"No active share links"</div>
            })}
            <For
                each=move || shares.get()
                key=|s| s.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string()
                let:share
            >
                {move || {
                    let path = share.get("path").and_then(|p| p.as_str()).unwrap_or("?").to_string();
                    let expires = share.get("expires_at").and_then(|e| e.as_str()).unwrap_or("?").to_string();
                    view! {
                        <div class="py-2 border-b border-gray-100 last:border-0">
                            <div class="text-sm font-medium text-gray-900">{path}</div>
                            <div class="text-xs text-gray-500 mt-0.5">"Expires: " {expires}</div>
                        </div>
                    }
                }}
            </For>
        </div>
    }
}

#[component]
fn AuditLogCard() -> impl IntoView {
    let (entries, set_entries) = create_signal(vec![]);
    let (loading, set_loading) = create_signal(true);

    create_effect(move |_| {
        spawn_local(async move {
            match api::fetch_json("/api/audit").await {
                Ok(data) => {
                    let list = data
                        .get("entries")
                        .and_then(|e| e.as_array())
                        .cloned()
                        .unwrap_or_default();
                    set_entries.set(list);
                    set_loading.set(false);
                }
                Err(_) => set_loading.set(false),
            }
        });
    });

    view! {
        <div>
            {move || loading.get().then(|| view! {
                <div class="text-sm text-gray-500" role="status" aria-live="polite">"Loading..."</div>
            })}
            {move || (!loading.get() && entries.with(Vec::is_empty)).then(|| view! {
                <div class="text-sm text-gray-500">"No recent activity"</div>
            })}
            <For
                each=move || entries.get()
                key=|e| e.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string()
                let:entry
            >
                {move || {
                    let status = entry.get("status").and_then(|s| s.as_u64()).unwrap_or(0);
                    let method = entry.get("method").and_then(|m| m.as_str()).unwrap_or("?").to_string();
                    let path = entry.get("path").and_then(|p| p.as_str()).unwrap_or("?").to_string();
                    let user = entry.get("user").and_then(|u| u.as_str()).unwrap_or("?").to_string();
                    let color = match status {
                        200..=299 => "text-green-600",
                        400..=499 => "text-yellow-600",
                        _ => "text-red-600",
                    };
                    view! {
                        <div class="py-1.5 border-b border-gray-100 last:border-0 text-xs">
                            <div class="flex items-center gap-2">
                                <span class={color}>{method}</span>
                                <span class="text-gray-600 truncate">{path}</span>
                                <span class="text-gray-500 ml-auto">{user}</span>
                            </div>
                        </div>
                    }
                }}
            </For>
        </div>
    }
}
