use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::navigation::NavigationSidebar;
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalyticsOverview {
    pub total_views: u64,
    pub total_downloads: u64,
    pub total_links: u64,
    pub storage_used: u64,
    pub active_users: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareLinkAnalytics {
    pub token: String,
    pub path: String,
    pub views: u64,
    pub downloads: u64,
    pub unique_visitors: u64,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub views: u64,
    pub downloads: u64,
    pub unique_visitors: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReferrerStat {
    pub referrer: String,
    pub count: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewsOverTime {
    pub date: String,
    pub views: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TopLink {
    pub path: String,
    pub views: u64,
    pub downloads: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageUsage {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub by_type: Vec<(String, u64)>,
}

#[derive(Debug, Clone, PartialEq)]
enum AnalyticsTab {
    Overview,
    Links,
    LinkDetail(String),
}

fn format_bytes(bytes: u64) -> String {
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
}

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (tab, set_tab) = signal(AnalyticsTab::Overview);
    let (error_msg, set_error) = signal(String::new());

    let (overview, set_overview) = signal(None::<AnalyticsOverview>);
    let (views_over_time, set_views_over_time) = signal(Vec::<ViewsOverTime>::new());
    let (top_links, set_top_links) = signal(Vec::<TopLink>::new());
    let (storage, set_storage) = signal(None::<StorageUsage>);
    let (share_links, set_share_links) = signal(Vec::<ShareLinkAnalytics>::new());

    let (link_detail_daily, set_link_detail_daily) = signal(Vec::<DailyStats>::new());
    let (link_detail_referrers, set_link_detail_referrers) = signal(Vec::<ReferrerStat>::new());

    let (date_from, set_date_from) = signal(String::new());
    let (date_to, set_date_to) = signal(String::new());

    Effect::new(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(String::new());

            let mut overview_val = None;
            let mut views_val = None;
            let mut top_links_val = None;
            let mut storage_val = None;
            let mut shares_val = None;

            if let Ok(v) = api::fetch_json("/api/analytics/overview").await {
                overview_val = Some(v);
            }
            if let Ok(v) = api::fetch_json("/api/analytics/views-over-time").await {
                views_val = Some(v);
            }
            if let Ok(v) = api::fetch_json("/api/analytics/top-links").await {
                top_links_val = Some(v);
            }
            if let Ok(v) = api::fetch_json("/api/analytics/storage").await {
                storage_val = Some(v);
            }
            if let Ok(v) = api::fetch_json("/api/analytics/share-links").await {
                shares_val = Some(v);
            }

            if let Some(v) = overview_val {
                set_overview.set(Some(AnalyticsOverview {
                    total_views: v.get("total_views").and_then(|v| v.as_u64()).unwrap_or(0),
                    total_downloads: v.get("total_downloads").and_then(|v| v.as_u64()).unwrap_or(0),
                    total_links: v.get("total_links").and_then(|v| v.as_u64()).unwrap_or(0),
                    storage_used: v.get("storage_used").and_then(|v| v.as_u64()).unwrap_or(0),
                    active_users: v.get("active_users").and_then(|v| v.as_u64()).unwrap_or(0),
                }));
            }

            if let Some(v) = views_val {
                let list = v
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|item| ViewsOverTime {
                                date: item.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                                views: item.get("views").and_then(|v| v.as_u64()).unwrap_or(0),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_views_over_time.set(list);
            }

            if let Some(v) = top_links_val {
                let list = v
                    .get("links")
                    .and_then(|l| l.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|item| TopLink {
                                path: item.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                                views: item.get("views").and_then(|v| v.as_u64()).unwrap_or(0),
                                downloads: item.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_top_links.set(list);
            }

            if let Some(v) = storage_val {
                set_storage.set(Some(StorageUsage {
                    used_bytes: v.get("used_bytes").and_then(|u| u.as_u64()).unwrap_or(0),
                    total_bytes: v.get("total_bytes").and_then(|t| t.as_u64()).unwrap_or(0),
                    by_type: v
                        .get("by_type")
                        .and_then(|b| b.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|item| {
                                    let key = item
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    let size = item.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                                    (key, size)
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }));
            }

            if let Some(v) = shares_val {
                let list = v
                    .get("shares")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|item| ShareLinkAnalytics {
                                token: item.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                path: item.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                                views: item.get("views").and_then(|v| v.as_u64()).unwrap_or(0),
                                downloads: item.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
                                unique_visitors: item.get("unique_visitors").and_then(|u| u.as_u64()).unwrap_or(0),
                                created_at: item
                                    .get("created_at")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                expires_at: item
                                    .get("expires_at")
                                    .and_then(|e| e.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_share_links.set(list);
            }

            set_loading.set(false);
        });
    });

    let fetch_link_detail = move |token: &str| {
        let t = token.to_string();
        spawn_local(async move {
            let url = format!("/api/analytics/share-links/{}", t);
            if let Ok(v) = api::fetch_json(&url).await {
                let daily = v
                    .get("daily")
                    .and_then(|d| d.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|item| DailyStats {
                                date: item.get("date").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                                views: item.get("views").and_then(|v| v.as_u64()).unwrap_or(0),
                                downloads: item.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
                                unique_visitors: item.get("unique_visitors").and_then(|u| u.as_u64()).unwrap_or(0),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_link_detail_daily.set(daily);

                let referrers = v
                    .get("referrers")
                    .and_then(|r| r.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|item| ReferrerStat {
                                referrer: item
                                    .get("referrer")
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("Direct")
                                    .to_string(),
                                count: item.get("count").and_then(|c| c.as_u64()).unwrap_or(0),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_link_detail_referrers.set(referrers);
            }
        });
    };

    let max_views = move || views_over_time.get().iter().map(|v| v.views).max().unwrap_or(1).max(1);

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 flex overflow-hidden pt-16">
                <NavigationSidebar />
                <main id="main-content" class="flex-1 overflow-auto p-6">
                    <div class="flex items-center justify-between mb-6">
                        <h1 class="text-2xl font-bold font-mono text-gray-900 dark:text-white">{t!("analytics.title")}</h1>
                        <div class="flex items-center gap-2">
                            <label class="text-sm font-mono text-gray-600 dark:text-gray-400">{t!("analytics.from")}</label>
                            <input
                                type="date"
                                prop:value=move || date_from.get()
                                on:input=move |ev| set_date_from.set(event_target_value(&ev))
                                class="px-3 py-1.5 text-sm font-mono border rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            />
                            <label class="text-sm font-mono text-gray-600 dark:text-gray-400">{t!("analytics.to")}</label>
                            <input
                                type="date"
                                prop:value=move || date_to.get()
                                on:input=move |ev| set_date_to.set(event_target_value(&ev))
                                class="px-3 py-1.5 text-sm font-mono border rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            />
                        </div>
                    </div>

                    {/* Tab Navigation */}
                    <div class="flex items-center gap-1 mb-6">
                        <button
                            on:click=move |_| set_tab.set(AnalyticsTab::Overview)
                            class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}",
                                if tab.get() == AnalyticsTab::Overview { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" }
                            )
                        >
                            {t!("analytics.overview")}
                        </button>
                        <button
                            on:click=move |_| set_tab.set(AnalyticsTab::Links)
                            class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}",
                                if tab.get() == AnalyticsTab::Links { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" }
                            )
                        >
                            {t!("analytics.links")}
                        </button>
                    </div>

                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12" role="status" aria-busy="true">
                            <div class="text-sm text-gray-500 font-mono">{t!("common.loading")}</div>
                        </div>
                    })}

                    {move || (!error_msg.get().is_empty() && !loading.get()).then(|| view! {
                        <div class="p-4 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">
                            <span class="font-bold">{t!("error.prefix")}</span> {error_msg}
                        </div>
                    })}

                    {move || if !loading.get() && tab.get() == AnalyticsTab::Overview {
                        let ov = overview.get();
                        let vot = views_over_time.get();
                        let tl = top_links.get();
                        let st = storage.get();
                        let mv = max_views();

                        view! {
                            <div class="space-y-6">
                                <div class="grid grid-cols-2 lg:grid-cols-5 gap-4">
                                    {move || ov.as_ref().map(|o| view! { <><StatCard label=t!("analytics.total_views") value=o.total_views.to_string() /><StatCard label=t!("analytics.total_downloads") value=o.total_downloads.to_string() /><StatCard label=t!("analytics.total_links") value=o.total_links.to_string() /><StatCard label=t!("analytics.storage_used") value=format_bytes(o.storage_used) /><StatCard label=t!("analytics.active_users") value=o.active_users.to_string() /></> })}
                                </div>
                                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                    <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("analytics.views_over_time")}</h2>
                                    <div class="flex items-end gap-1 h-48">
                                            {vot.iter().map(|v| {
                                                let height_pct = if mv > 0 { v.views as f64 / mv as f64 * 100.0 } else { 0.0 };
                                                let date = v.date.clone();
                                                let date_short = if date.len() >= 5 { date[date.len()-5..].to_string() } else { date.clone() };
                                                let views = v.views;
                                                view! {
                                                    <div class="flex-1 flex flex-col items-center gap-1" title=format!("{}: {} views", date, views)>
                                                        <div class="w-full bg-blue-500 rounded-t transition-all" style=format!("height: {}%", height_pct.max(2.0))></div>
                                                        <span class="text-[10px] font-mono text-gray-500 truncate w-full text-center">{date_short}</span>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                        <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("analytics.top_links")}</h2>
                                        <div class="space-y-2">
                                            {tl.iter().take(5).map(|link| {
                                                let path = link.path.clone();
                                                let views = link.views;
                                                let downloads = link.downloads;
                                                view! {
                                                    <div class="flex items-center justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                                                        <div class="text-sm font-mono text-gray-900 dark:text-white truncate">{path}</div>
                                                        <div class="flex items-center gap-3 text-xs text-gray-500 shrink-0">
                                                            <span>{views} " views"</span>
                                                            <span>{downloads} " downloads"</span>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                        <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("analytics.storage_breakdown")}</h2>
                                        {move || st.as_ref().map(|s| view! {
                                            <div class="space-y-3">
                                                <div class="flex justify-between text-sm font-mono">
                                                    <span class="text-gray-600 dark:text-gray-400">{t!("analytics.used")}</span>
                                                    <span class="font-bold text-gray-900 dark:text-white">{format_bytes(s.used_bytes)}</span>
                                                </div>
                                                <div class="flex justify-between text-sm font-mono">
                                                    <span class="text-gray-600 dark:text-gray-400">{t!("analytics.total")}</span>
                                                    <span class="font-bold text-gray-900 dark:text-white">{format_bytes(s.total_bytes)}</span>
                                                </div>
                                                {s.by_type.iter().map(|(type_name, size)| {
                                                    let tn = type_name.clone();
                                                    let sz = *size;
                                                    view! {
                                                        <div class="flex justify-between text-xs font-mono">
                                                            <span class="text-gray-500">{tn}</span>
                                                            <span class="text-gray-700 dark:text-gray-300">{format_bytes(sz)}</span>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        })}
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="hidden"></div> }.into_any()
                    }}

                    {move || if !loading.get() && tab.get() == AnalyticsTab::Links {
                        let links = share_links.get();
                        view! {
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("analytics.path")}</th>
                                            <th class="px-4 py-3 text-right text-xs font-bold uppercase font-mono text-gray-500">{t!("analytics.views")}</th>
                                            <th class="px-4 py-3 text-right text-xs font-bold uppercase font-mono text-gray-500">{t!("analytics.downloads")}</th>
                                            <th class="px-4 py-3 text-right text-xs font-bold uppercase font-mono text-gray-500">{t!("analytics.unique_visitors")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("analytics.created")}</th>
                                            <th class="px-4 py-3"></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                            {links.iter().map(|link| {
                                                let token = link.token.clone();
                                                let path = link.path.clone();
                                                let views = link.views;
                                                let downloads = link.downloads;
                                                let unique = link.unique_visitors;
                                                let created = link.created_at.clone();
                                                let created_display = if created.len() >= 10 { created[..10].to_string() } else { created };
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white truncate max-w-xs">{path}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 text-right">{views}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 text-right">{downloads}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 text-right">{unique}</td>
                                                        <td class="px-4 py-3 text-xs font-mono text-gray-500">{created_display}</td>
                                                    <td class="px-4 py-3">
                                                        <button
                                                            on:click=move |_: ev::MouseEvent| {
                                                                set_tab.set(AnalyticsTab::LinkDetail(token.clone()));
                                                                fetch_link_detail(&token);
                                                            }
                                                            class="text-xs text-blue-600 hover:text-blue-800 font-medium transition-colors"
                                                        >
                                                            {t!("analytics.details")}
                                                        </button>
                                                    </td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </tbody>
                                </table>
                                {move || links.is_empty().then(|| view! {
                                    <div class="p-8 text-center text-gray-500 text-sm">{t!("analytics.no_links")}</div>
                                })}
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="hidden"></div> }.into_any()
                    }}

                    {move || if !loading.get() && matches!(tab.get(), AnalyticsTab::LinkDetail(_)) {
                        let daily = link_detail_daily.get();
                        let referrers = link_detail_referrers.get();
                        let max_daily = daily.iter().map(|d| d.views).max().unwrap_or(1).max(1);
                        let tk = match tab.get() { AnalyticsTab::LinkDetail(ref t) => t.clone(), _ => String::new() };

                        view! {
                            <div class="space-y-6">
                                <div class="flex items-center gap-3">
                                    <button
                                        on:click=move |_| set_tab.set(AnalyticsTab::Links)
                                        class="p-1 text-gray-500 hover:text-gray-700 rounded transition-colors"
                                        aria-label=t!("common.back")
                                    >
                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>
                                    </button>
                                    <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("analytics.link_detail")} ": " {tk}</h2>
                                </div>
                                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                    <h3 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("analytics.daily_breakdown")}</h3>
                                    <div class="flex items-end gap-1 h-48">
                                            {daily.iter().map(|d| {
                                                let height_pct = if max_daily > 0 { d.views as f64 / max_daily as f64 * 100.0 } else { 0.0 };
                                                let date = d.date.clone();
                                                let date_short = if date.len() >= 5 { date[date.len()-5..].to_string() } else { date.clone() };
                                                let views = d.views;
                                                view! {
                                                    <div class="flex-1 flex flex-col items-center gap-1" title=format!("{}: {} views", date, views)>
                                                        <div class="w-full bg-blue-500 rounded-t transition-all" style=format!("height: {}%", height_pct.max(2.0))></div>
                                                        <span class="text-[10px] font-mono text-gray-500 truncate w-full text-center">{date_short}</span>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                    <h3 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("analytics.referrers")}</h3>
                                    <div class="space-y-2">
                                        {referrers.iter().map(|r| {
                                            let referrer = r.referrer.clone();
                                            let count = r.count;
                                            view! {
                                                <div class="flex items-center justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                                                    <span class="text-sm font-mono text-gray-900 dark:text-white">{referrer}</span>
                                                    <span class="text-xs font-mono text-gray-500">{count}</span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                        {move || referrers.is_empty().then(|| view! {
                                            <div class="text-sm text-gray-500 text-center py-4">{t!("analytics.no_referrers")}</div>
                                        })}
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="hidden"></div> }.into_any()
                    }}
                </main>
            </div>
        </div>
    }
}

#[component]
fn StatCard(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-4">
            <div class="text-xs font-bold uppercase font-mono text-gray-500 mb-1">{label}</div>
            <div class="text-2xl font-bold font-mono text-gray-900 dark:text-white">{value}</div>
        </div>
    }
}
