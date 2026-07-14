use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::modal::Modal;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Available,
    Installed,
    Enabled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SortBy {
    Name,
    Rating,
    Downloads,
    Recent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ViewTab {
    Browse,
    Installed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: String,
    pub rating: f64,
    pub downloads: u64,
    pub status: PluginStatus,
    pub changelog: String,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplaceResponse {
    pub plugins: Vec<MarketplacePlugin>,
}

fn mock_plugins() -> Vec<MarketplacePlugin> {
    vec![
        MarketplacePlugin {
            id: "pdf-preview".into(),
            name: "PDF Preview".into(),
            version: "1.2.0".into(),
            author: "Ferro Team".into(),
            description: "Render PDF files inline in the browser with zoom and page navigation.".into(),
            category: "Productivity".into(),
            rating: 4.8,
            downloads: 12_340,
            status: PluginStatus::Available,
            changelog: "## 1.2.0\n- Added text selection in preview\n- Fixed rendering on Safari\n\n## 1.1.0\n- Added thumbnail sidebar".into(),
            permissions: vec!["read_files".into()],
        },
        MarketplacePlugin {
            id: "image-compress".into(),
            name: "Image Compressor".into(),
            version: "2.0.1".into(),
            author: "Community".into(),
            description: "Automatically compress uploaded images using WASM-based WebP conversion.".into(),
            category: "Media".into(),
            rating: 4.5,
            downloads: 8_920,
            status: PluginStatus::Installed,
            changelog: "## 2.0.1\n- Fixed EXIF orientation handling\n\n## 2.0.0\n- Rewritten in WASM for performance".into(),
            permissions: vec!["read_files".into(), "write_files".into()],
        },
        MarketplacePlugin {
            id: "antivirus-scan".into(),
            name: "Antivirus Scanner".into(),
            version: "3.1.0".into(),
            author: "Security Labs".into(),
            description: "Scan uploaded files for malware using ClamAV integration.".into(),
            category: "Security".into(),
            rating: 4.9,
            downloads: 22_100,
            status: PluginStatus::Enabled,
            changelog: "## 3.1.0\n- Incremental scan for large files\n- Updated signature database\n\n## 3.0.0\n- Async scanning pipeline".into(),
            permissions: vec!["read_files".into(), "network".into()],
        },
        MarketplacePlugin {
            id: "markdown-editor".into(),
            name: "Markdown Editor".into(),
            version: "1.0.0".into(),
            author: "Ferro Team".into(),
            description: "WYSIWYG Markdown editor with live preview, syntax highlighting, and export.".into(),
            category: "Productivity".into(),
            rating: 4.3,
            downloads: 5_670,
            status: PluginStatus::Available,
            changelog: "## 1.0.0\n- Initial release\n- Full CommonMark support".into(),
            permissions: vec!["read_files".into(), "write_files".into()],
        },
        MarketplacePlugin {
            id: "video-transcode".into(),
            name: "Video Transcoder".into(),
            version: "0.9.2".into(),
            author: "MediaForge".into(),
            description: "Transcode video files to web-friendly formats using server-side FFmpeg.".into(),
            category: "Media".into(),
            rating: 3.8,
            downloads: 2_340,
            status: PluginStatus::Available,
            changelog: "## 0.9.2\n- Fixed audio sync issues\n\n## 0.9.0\n- Added HLS output support".into(),
            permissions: vec!["read_files".into(), "write_files".into(), "network".into()],
        },
        MarketplacePlugin {
            id: "audit-report".into(),
            name: "Audit Report Generator".into(),
            version: "1.4.0".into(),
            author: "Compliance.io".into(),
            description: "Generate PDF compliance reports from audit logs for SOC2 and GDPR.".into(),
            category: "Compliance".into(),
            rating: 4.6,
            downloads: 6_780,
            status: PluginStatus::Installed,
            changelog: "## 1.4.0\n- GDPR article mapping\n\n## 1.3.0\n- SOC2 Type II template".into(),
            permissions: vec!["read_files".into(), "admin_api".into()],
        },
    ]
}

fn all_categories(plugins: &[MarketplacePlugin]) -> Vec<String> {
    let mut cats: Vec<String> = plugins.iter().map(|p| p.category.clone()).collect();
    cats.sort();
    cats.dedup();
    cats
}

fn status_badge_variant(status: &PluginStatus) -> BadgeVariant {
    match status {
        PluginStatus::Enabled => BadgeVariant::Success,
        PluginStatus::Installed => BadgeVariant::Warning,
        PluginStatus::Available => BadgeVariant::Neutral,
    }
}

fn status_text(status: &PluginStatus) -> String {
    match status {
        PluginStatus::Enabled => "Enabled".into(),
        PluginStatus::Installed => "Installed".into(),
        PluginStatus::Available => "Available".into(),
    }
}

fn sort_plugins(plugins: &mut [MarketplacePlugin], sort_by: &SortBy) {
    match sort_by {
        SortBy::Name => plugins.sort_by(|a, b| a.name.cmp(&b.name)),
        SortBy::Rating => plugins.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal)),
        SortBy::Downloads => plugins.sort_by_key(|b| std::cmp::Reverse(b.downloads)),
        SortBy::Recent => plugins.reverse(),
    }
}

#[component]
pub fn PluginMarketplace(api: RwSignal<ApiState>) -> impl IntoView {
    let (plugins, set_plugins) = signal(mock_plugins());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (search_query, set_search_query) = signal(String::new());
    let (category_filter, set_category_filter) = signal(None::<String>);
    let (sort_by, set_sort_by) = signal(SortBy::Downloads);
    let (active_tab, set_active_tab) = signal(ViewTab::Browse);
    let (detail_plugin, set_detail_plugin) = signal(None::<MarketplacePlugin>);
    let (action_loading, set_action_loading) = signal(None::<String>);
    let (update_checking, set_update_checking) = signal(false);
    let (update_results, set_update_results) = signal(None::<Vec<String>>);

    let load_plugins = move || {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone
                .get::<MarketplaceResponse>("/api/v1/admin/plugins/marketplace")
                .await
            {
                Ok(resp) => set_plugins.set(resp.plugins),
                Err(_) => {
                    set_plugins.set(mock_plugins());
                }
            }
            set_loading.set(false);
        });
    };

    Effect::new(move |_| load_plugins());

    let filtered_plugins = Memo::new(move |_| {
        let query = search_query.get().to_lowercase();
        let cat = category_filter.get();
        let mut items: Vec<MarketplacePlugin> = plugins
            .get()
            .iter()
            .filter(|p| {
                let matches_search = query.is_empty()
                    || p.name.to_lowercase().contains(&query)
                    || p.description.to_lowercase().contains(&query)
                    || p.author.to_lowercase().contains(&query);
                let matches_cat = cat.as_ref().is_none_or(|c| c == &p.category);
                matches_search && matches_cat
            })
            .cloned()
            .collect();
        sort_plugins(&mut items, &sort_by.get());
        items
    });

    let installed_plugins = Memo::new(move |_| {
        let mut items: Vec<MarketplacePlugin> = plugins
            .get()
            .iter()
            .filter(|p| matches!(p.status, PluginStatus::Installed | PluginStatus::Enabled))
            .cloned()
            .collect();
        sort_plugins(&mut items, &sort_by.get());
        items
    });

    let categories = Memo::new(move |_| all_categories(&plugins.get()));

    let do_action = move |plugin_id: String, action: String| {
        let api_clone = api.get_untracked();
        set_action_loading.set(Some(plugin_id.clone()));
        let id_for_msg = plugin_id.clone();
        spawn_local(async move {
            let body = serde_json::json!({});
            let result = match action.as_str() {
                "install" => {
                    api_clone
                        .post::<serde_json::Value>(&format!("/api/v1/admin/plugins/{}/install", plugin_id), &body)
                        .await
                }
                "uninstall" => {
                    api_clone
                        .post::<serde_json::Value>(&format!("/api/v1/admin/plugins/{}/uninstall", plugin_id), &body)
                        .await
                }
                "enable" => {
                    api_clone
                        .post::<serde_json::Value>(&format!("/api/v1/admin/plugins/{}/enable", plugin_id), &body)
                        .await
                }
                "disable" => {
                    api_clone
                        .post::<serde_json::Value>(&format!("/api/v1/admin/plugins/{}/disable", plugin_id), &body)
                        .await
                }
                _ => Err("Unknown action".to_string()),
            };
            match result {
                Ok(_) => {
                    set_plugins.update(|ps| {
                        if let Some(p) = ps.iter_mut().find(|p| p.id == id_for_msg) {
                            match action.as_str() {
                                "install" => p.status = PluginStatus::Installed,
                                "uninstall" => p.status = PluginStatus::Available,
                                "enable" => p.status = PluginStatus::Enabled,
                                "disable" => p.status = PluginStatus::Installed,
                                _ => {}
                            }
                        }
                    });
                    set_detail_plugin.update(|dp| {
                        if let Some(p) = dp
                            && p.id == id_for_msg
                        {
                            match action.as_str() {
                                "install" => p.status = PluginStatus::Installed,
                                "uninstall" => p.status = PluginStatus::Available,
                                "enable" => p.status = PluginStatus::Enabled,
                                "disable" => p.status = PluginStatus::Installed,
                                _ => {}
                            }
                        }
                    });
                }
                Err(e) => set_error.set(Some(format!("Action failed for {}: {}", id_for_msg, e))),
            }
            set_action_loading.set(None);
        });
    };

    let check_updates = move || {
        let api_clone = api.get_untracked();
        set_update_checking.set(true);
        set_update_results.set(None);
        spawn_local(async move {
            match api_clone
                .get::<MarketplaceResponse>("/api/v1/admin/plugins/marketplace")
                .await
            {
                Ok(resp) => {
                    let mut updates = Vec::new();
                    for plugin in &resp.plugins {
                        if matches!(plugin.status, PluginStatus::Installed | PluginStatus::Enabled)
                            && let Some(local) = plugins.get_untracked().iter().find(|p| p.id == plugin.id)
                            && plugin.version != local.version
                        {
                            updates.push(format!("{}: {} -> {}", plugin.name, local.version, plugin.version));
                        }
                    }
                    set_update_results.set(Some(updates));
                }
                Err(e) => set_error.set(Some(format!("Update check failed: {}", e))),
            }
            set_update_checking.set(false);
        });
    };

    let categories_list = categories;

    view! {
        <div class="page">
            <div class="page-header">
                <div class="page-header-left">
                    <h2 class="panel-title" style="margin-bottom:0">"Plugin Marketplace"</h2>
                </div>
            </div>

            {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}

            <div class="tab-bar" role="tablist">
                <button
                    class=move || format!("tab {}", if active_tab.get() == ViewTab::Browse { "tab-active" } else { "" })
                    role="tab"
                    aria-selected=move || active_tab.get() == ViewTab::Browse
                    on:click=move |_| set_active_tab.set(ViewTab::Browse)
                >
                    "Browse"
                </button>
                <button
                    class=move || format!("tab {}", if active_tab.get() == ViewTab::Installed { "tab-active" } else { "" })
                    role="tab"
                    aria-selected=move || active_tab.get() == ViewTab::Installed
                    on:click=move |_| set_active_tab.set(ViewTab::Installed)
                >
                    "Installed"
                </button>
            </div>

            <div class="page-header" style="margin-bottom:16px">
                <div class="page-header-left">
                    <input
                        class="search-input"
                        type="text"
                        placeholder="Search plugins..."
                        prop:value=move || search_query.get()
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                        aria-label="Search plugins"
                    />
                    <select
                        class="form-input"
                        style="width:auto;min-width:160px"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_category_filter.set(if val == "all" { None } else { Some(val) });
                        }
                        aria-label="Filter by category"
                    >
                        <option value="all">"All Categories"</option>
                        {move || categories_list.get().iter().map(|cat| {
                            let c_val = cat.clone();
                            let c_label = cat.clone();
                            view! { <option value=c_val>{c_label}</option> }
                        }).collect::<Vec<_>>()}
                    </select>
                    <select
                        class="form-input"
                        style="width:auto;min-width:140px"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            match val.as_str() {
                                "name" => set_sort_by.set(SortBy::Name),
                                "rating" => set_sort_by.set(SortBy::Rating),
                                "downloads" => set_sort_by.set(SortBy::Downloads),
                                "recent" => set_sort_by.set(SortBy::Recent),
                                _ => {}
                            }
                        }
                        aria-label="Sort by"
                    >
                        <option value="downloads">"Most Popular"</option>
                        <option value="rating">"Highest Rated"</option>
                        <option value="name">"Name A-Z"</option>
                        <option value="recent">"Recently Added"</option>
                    </select>
                </div>
                <div class="page-header-right">
                    {move || if active_tab.get() == ViewTab::Installed {
                        view! {
                            <button
                                class="btn btn-secondary btn-sm"
                                disabled=move || update_checking.get()
                                on:click=move |_| check_updates()
                            >
                                {move || if update_checking.get() { "Checking..." } else { "Check for Updates" }}
                            </button>
                        }.into_any()
                    } else {
                        view! { <span/> }.into_any()
                    }}
                </div>
            </div>

            {move || update_results.get().map(|updates| {
                if updates.is_empty() {
                    view! { <div class="success-banner">"All installed plugins are up to date."</div> }.into_any()
                } else {
                    view! {
                        <div class="panel" style="margin-bottom:16px">
                            <div class="panel-title">"Available Updates"</div>
                            <div class="activity-list">
                                {updates.iter().map(|u| view! {
                                    <div class="activity-item">
                                        <span class="activity-action">{u.clone()}</span>
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_any()
                }
            })}

            {move || if loading.get() {
                view! { <div class="loading">"Loading plugins..."</div> }.into_any()
            } else {
                let items = if active_tab.get() == ViewTab::Installed {
                    installed_plugins.get()
                } else {
                    filtered_plugins.get()
                };
                if items.is_empty() {
                    let msg = if active_tab.get() == ViewTab::Installed {
                        "No plugins installed yet."
                    } else {
                        "No plugins found matching your search."
                    };
                    view! { <div class="empty-state">{msg}</div> }.into_any()
                } else {
                    view! {
                        <div class="plugin-grid">
                            {items.iter().map(|plugin| {
                                let p_install = plugin.clone();
                                let p_enable = plugin.clone();
                                let p_uninstall = plugin.clone();
                                let p_disable = plugin.clone();
                                let p_detail = plugin.clone();
                                let bv = status_badge_variant(&plugin.status);
                                let st = status_text(&plugin.status);
                                let action_btns = match plugin.status {
                                    PluginStatus::Available => {
                                        let pid_d = plugin.id.clone();
                                        let pid_t = plugin.id.clone();
                                        view! {
                                            <button
                                                class="btn btn-primary btn-sm"
                                                disabled=move || action_loading.get() == Some(pid_d.clone())
                                                on:click=move |_| do_action(p_install.id.clone(), "install".into())
                                            >
                                                {move || if action_loading.get() == Some(pid_t.clone()) { "Installing..." } else { "Install" }}
                                            </button>
                                        }.into_any()
                                    }
                                    PluginStatus::Installed => {
                                        let pid_d1 = plugin.id.clone();
                                        let pid_t1 = plugin.id.clone();
                                        let pid_d2 = plugin.id.clone();
                                        let pid_t2 = plugin.id.clone();
                                        view! {
                                            <button
                                                class="btn btn-primary btn-sm"
                                                disabled=move || action_loading.get() == Some(pid_d1.clone())
                                                on:click=move |_| do_action(p_enable.id.clone(), "enable".into())
                                            >
                                                {move || if action_loading.get() == Some(pid_t1.clone()) { "Enabling..." } else { "Enable" }}
                                            </button>
                                            <button
                                                class="btn btn-danger btn-sm"
                                                disabled=move || action_loading.get() == Some(pid_d2.clone())
                                                on:click=move |_| do_action(p_uninstall.id.clone(), "uninstall".into())
                                            >
                                                {move || if action_loading.get() == Some(pid_t2.clone()) { "Removing..." } else { "Uninstall" }}
                                            </button>
                                        }.into_any()
                                    }
                                    PluginStatus::Enabled => {
                                        let pid_d = plugin.id.clone();
                                        let pid_t = plugin.id.clone();
                                        view! {
                                            <button
                                                class="btn btn-secondary btn-sm"
                                                disabled=move || action_loading.get() == Some(pid_d.clone())
                                                on:click=move |_| do_action(p_disable.id.clone(), "disable".into())
                                            >
                                                {move || if action_loading.get() == Some(pid_t.clone()) { "Disabling..." } else { "Disable" }}
                                            </button>
                                        }.into_any()
                                    }
                                };
                                view! {
                                    <div class="plugin-card">
                                        <div class="plugin-card-header">
                                            <div class="plugin-card-title">
                                                <span class="plugin-name font-display">{plugin.name.clone()}</span>
                                                <Badge text=st variant=bv/>
                                            </div>
                                            <span class="plugin-version mono">{format!("v{}", plugin.version)}</span>
                                        </div>
                                        <div class="plugin-author mono">{"by "}{plugin.author.clone()}</div>
                                        <p class="plugin-description">{plugin.description.clone()}</p>
                                        <div class="plugin-meta">
                                            <span class="plugin-category">
                                                <span class="form-label" style="margin:0">"Category: "</span>
                                                {plugin.category.clone()}
                                            </span>
                                            <span class="plugin-rating">
                                                <span class="form-label" style="margin:0">"Rating: "</span>
                                                {format!("{:.1}", plugin.rating)}
                                                " / 5"
                                            </span>
                                            <span class="plugin-downloads mono">
                                                {format!("{} downloads", plugin.downloads)}
                                            </span>
                                        </div>
                                        <div class="plugin-card-actions">
                                            {action_btns}
                                            <button
                                                class="btn btn-secondary btn-sm"
                                                on:click=move |_| set_detail_plugin.set(Some(p_detail.clone()))
                                            >
                                                "Details"
                                            </button>
                                        </div>
                                    </div>
                                }.into_any()
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}

            {move || {
                detail_plugin.get().map(|plugin| {
                    let bv = status_badge_variant(&plugin.status);
                    let st = status_text(&plugin.status);
                    let id_for_action = plugin.id.clone();
                    let id_for_action2 = plugin.id.clone();
                    let id_for_action3 = plugin.id.clone();
                    let id_for_action4 = plugin.id.clone();
                    let status_for_action = plugin.status.clone();
                    view! {
                        <Modal
                            title=format!("{} — v{}", plugin.name, plugin.version)
                            show=true
                            on_close=Callback::new(move |()| set_detail_plugin.set(None))
                        >
                            <div class="plugin-detail">
                                <div class="detail-row">
                                    <span class="detail-label">"Status"</span>
                                    <span class="detail-value"><Badge text=st variant=bv/></span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Author"</span>
                                    <span class="detail-value">{plugin.author.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Category"</span>
                                    <span class="detail-value">{plugin.category.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Rating"</span>
                                    <span class="detail-value">{format!("{:.1} / 5", plugin.rating)}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Downloads"</span>
                                    <span class="detail-value mono">{plugin.downloads.to_string()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Description"</span>
                                    <span class="detail-value">{plugin.description.clone()}</span>
                                </div>
                                <div class="detail-row">
                                    <span class="detail-label">"Permissions"</span>
                                    <span class="detail-value">
                                        <div class="checkbox-group">
                                            {plugin.permissions.iter().map(|perm| {
                                                view! {
                                                    <span class="badge badge-info font-display">{perm.clone()}</span>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </span>
                                </div>

                                <div class="panel-title" style="margin-top:16px">"Changelog"</div>
                                <div class="changelog-content mono">{plugin.changelog.clone()}</div>

                                <div class="modal-actions" style="margin-top:20px">
                                    {match status_for_action {
                                        PluginStatus::Available => view! {
                                            <button class="btn btn-primary" on:click=move |_| {
                                                do_action(id_for_action.clone(), "install".into());
                                                set_detail_plugin.set(None);
                                            }>"Install"</button>
                                        }.into_any(),
                                        PluginStatus::Installed => view! {
                                            <button class="btn btn-primary" on:click=move |_| {
                                                do_action(id_for_action2.clone(), "enable".into());
                                                set_detail_plugin.set(None);
                                            }>"Enable"</button>
                                            <button class="btn btn-danger" on:click=move |_| {
                                                do_action(id_for_action3.clone(), "uninstall".into());
                                                set_detail_plugin.set(None);
                                            }>"Uninstall"</button>
                                        }.into_any(),
                                        PluginStatus::Enabled => view! {
                                            <button class="btn btn-secondary" on:click=move |_| {
                                                do_action(id_for_action4.clone(), "disable".into());
                                                set_detail_plugin.set(None);
                                            }>"Disable"</button>
                                        }.into_any(),
                                    }}
                                    <button class="btn btn-secondary" on:click=move |_| set_detail_plugin.set(None)>"Close"</button>
                                </div>
                            </div>
                        </Modal>
                    }
                })
            }}

            <style>{PLUGIN_MARKETPLACE_CSS}</style>
        </div>
    }
}

const PLUGIN_MARKETPLACE_CSS: &str = r#"
.plugin-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 16px;
}

@media (max-width: 480px) {
    .plugin-grid {
        grid-template-columns: 1fr;
    }
}

.plugin-card {
    background: linear-gradient(135deg, #F5F0EB 0%, #EDE8E3 100%);
    box-shadow: var(--shadow-concrete);
    border-radius: var(--radius-lg);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

@media (prefers-color-scheme: dark) {
    .plugin-card {
        background: linear-gradient(135deg, #1A1714 0%, #221F1B 100%);
        box-shadow: var(--shadow-concrete);
    }
}

.plugin-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.plugin-card-title {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
}

.plugin-name {
    font-size: 16px;
    font-weight: 800;
    letter-spacing: -0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.plugin-version {
    font-size: 12px;
    color: var(--text-secondary);
    flex-shrink: 0;
}

.plugin-author {
    font-size: 12px;
    color: var(--text-secondary);
}

.plugin-description {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
    flex: 1;
}

.plugin-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    font-size: 13px;
    color: var(--text-secondary);
}

.plugin-category,
.plugin-rating {
    display: flex;
    align-items: center;
    gap: 4px;
}

.plugin-card-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border-color);
    margin-top: 4px;
}

.plugin-detail .changelog-content {
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    padding: 12px;
    max-height: 200px;
    overflow-y: auto;
}
"#;
