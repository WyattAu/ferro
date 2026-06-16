use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::components::navigation::NavigationSidebar;
use crate::t;
use ferro_common::format::format_size;

#[derive(Debug, Clone, PartialEq)]
enum AdminTab {
    Overview,
    Users,
    DlpPolicies,
    DlpAlerts,
    Antivirus,
    Watermarks,
    Notifications,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminDevice {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub last_seen: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DlpPolicy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub action: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DlpAlert {
    pub id: String,
    pub policy_name: String,
    pub user: String,
    pub filename: String,
    pub detected_at: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntivirusScan {
    pub id: String,
    pub status: String,
    pub scanned_at: String,
    pub files_scanned: u64,
    pub threats_found: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WatermarkPolicy {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub opacity: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationPreference {
    pub event_type: String,
    pub email_enabled: bool,
    pub push_enabled: bool,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (tab, set_tab) = signal(AdminTab::Overview);
    let (loading, set_loading) = signal(true);
    let (error_msg, set_error) = signal(String::new());

    // Overview
    let (storage_stats, set_storage_stats) = signal(None::<serde_json::Value>);
    let (share_links, set_share_links) = signal(vec![]);
    let (audit_entries, set_audit_entries) = signal(vec![]);

    // Users
    let (users, set_users) = signal(Vec::<AdminUser>::new());
    let (show_create_user, set_show_create_user) = signal(false);
    let (new_user_username, set_new_user_username) = signal(String::new());
    let (new_user_email, set_new_user_email) = signal(String::new());
    let (new_user_role, set_new_user_role) = signal("user".to_string());
    let (editing_user, set_editing_user) = signal(None::<AdminUser>);
    let (selected_user_devices, set_selected_user_devices) = signal(Vec::<AdminDevice>::new());
    let (show_devices, set_show_devices) = signal(false);
    let (transfer_source, set_transfer_source) = signal(String::new());
    let (transfer_target, set_transfer_target) = signal(String::new());

    // DLP
    let (dlp_policies, set_dlp_policies) = signal(Vec::<DlpPolicy>::new());
    let (dlp_alerts, set_dlp_alerts) = signal(Vec::<DlpAlert>::new());
    let (show_create_policy, set_show_create_policy) = signal(false);
    let (new_policy_name, set_new_policy_name) = signal(String::new());
    let (new_policy_desc, set_new_policy_desc) = signal(String::new());
    let (new_policy_type, set_new_policy_type) = signal("content".to_string());
    let (new_policy_action, set_new_policy_action) = signal("block".to_string());

    // Antivirus
    let (av_scans, set_av_scans) = signal(Vec::<AntivirusScan>::new());
    let (av_scan_running, set_av_scan_running) = signal(false);

    // Watermarks
    let (watermark_policies, set_watermark_policies) = signal(Vec::<WatermarkPolicy>::new());
    let (show_create_watermark, set_show_create_watermark) = signal(false);
    let (new_wm_name, set_new_wm_name) = signal(String::new());
    let (new_wm_pattern, set_new_wm_pattern) = signal("{{user}} - {{date}}".to_string());
    let (new_wm_opacity, set_new_wm_opacity) = signal(0.3_f64);

    // Notifications
    let (notification_prefs, set_notification_prefs) = signal(Vec::<NotificationPreference>::new());

    Effect::new(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(String::new());

            if let Ok(data) = api::fetch_json("/api/storage/stats").await {
                set_storage_stats.set(Some(data));
            }
            if let Ok(data) = api::fetch_json("/api/shares").await {
                let list = data.get("shares").and_then(|s| s.as_array()).cloned().unwrap_or_default();
                set_share_links.set(list);
            }
            if let Ok(data) = api::fetch_json("/api/audit").await {
                let list = data.get("entries").and_then(|e| e.as_array()).cloned().unwrap_or_default();
                set_audit_entries.set(list);
            }

            set_loading.set(false);
        });
    });

    let fetch_users = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/admin/users").await {
                let list = val.get("users").and_then(|u| u.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(AdminUser {
                            id: v.get("id")?.as_str()?.to_string(),
                            username: v.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                            email: v.get("email").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                            role: v.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string(),
                            created_at: v.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                            is_active: v.get("is_active").and_then(|a| a.as_bool()).unwrap_or(true),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_users.set(list);
            }
        });
    };

    let fetch_devices = move |user_id: &str| {
        let uid = user_id.to_string();
        spawn_local(async move {
            let url = format!("/api/admin/users/{}/devices", uid);
            if let Ok(val) = api::fetch_json(&url).await {
                let list = val.get("devices").and_then(|d| d.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(AdminDevice {
                            id: v.get("id")?.as_str()?.to_string(),
                            name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                            device_type: v.get("device_type").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                            last_seen: v.get("last_seen").and_then(|l| l.as_str()).unwrap_or("").to_string(),
                            is_active: v.get("is_active").and_then(|a| a.as_bool()).unwrap_or(true),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_selected_user_devices.set(list);
            }
        });
    };

    let fetch_dlp = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/admin/dlp/policies").await {
                let list = val.get("policies").and_then(|p| p.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(DlpPolicy {
                            id: v.get("id")?.as_str()?.to_string(),
                            name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                            description: v.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                            rule_type: v.get("rule_type").and_then(|r| r.as_str()).unwrap_or("").to_string(),
                            action: v.get("action").and_then(|a| a.as_str()).unwrap_or("").to_string(),
                            enabled: v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_dlp_policies.set(list);
            }
            if let Ok(val) = api::fetch_json("/api/admin/dlp/alerts").await {
                let list = val.get("alerts").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(DlpAlert {
                            id: v.get("id")?.as_str()?.to_string(),
                            policy_name: v.get("policy_name").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                            user: v.get("user").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                            filename: v.get("filename").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                            detected_at: v.get("detected_at").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                            status: v.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_dlp_alerts.set(list);
            }
        });
    };

    let fetch_av = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/admin/antivirus/scans").await {
                let list = val.get("scans").and_then(|s| s.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(AntivirusScan {
                            id: v.get("id")?.as_str()?.to_string(),
                            status: v.get("status").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                            scanned_at: v.get("scanned_at").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                            files_scanned: v.get("files_scanned").and_then(|f| f.as_u64()).unwrap_or(0),
                            threats_found: v.get("threats_found").and_then(|t| t.as_u64()).unwrap_or(0),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_av_scans.set(list);
            }
        });
    };

    let fetch_watermarks = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/admin/watermarks").await {
                let list = val.get("policies").and_then(|p| p.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(WatermarkPolicy {
                            id: v.get("id")?.as_str()?.to_string(),
                            name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                            pattern: v.get("pattern").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                            opacity: v.get("opacity").and_then(|o| o.as_f64()).unwrap_or(0.3),
                            enabled: v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_watermark_policies.set(list);
            }
        });
    };

    let fetch_notifications = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/admin/notifications/preferences").await {
                let list = val.get("preferences").and_then(|p| p.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| {
                        Some(NotificationPreference {
                            event_type: v.get("event_type").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                            email_enabled: v.get("email_enabled").and_then(|e| e.as_bool()).unwrap_or(false),
                            push_enabled: v.get("push_enabled").and_then(|p| p.as_bool()).unwrap_or(false),
                        })
                    }).collect()
                }).unwrap_or_default();
                set_notification_prefs.set(list);
            }
        });
    };

    let on_tab_change = move |new_tab: AdminTab| {
        set_tab.set(new_tab.clone());
        match new_tab {
            AdminTab::Users => fetch_users(),
            AdminTab::DlpPolicies | AdminTab::DlpAlerts => fetch_dlp(),
            AdminTab::Antivirus => fetch_av(),
            AdminTab::Watermarks => fetch_watermarks(),
            AdminTab::Notifications => fetch_notifications(),
            _ => {}
        }
    };

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 flex overflow-hidden pt-16">
                <NavigationSidebar />
                <main id="main-content" class="flex-1 overflow-auto p-6">
                    <h1 class="text-2xl font-bold font-mono text-gray-900 dark:text-white mb-6">{t!("admin.title")}</h1>

                    {/* Tab Navigation */}
                    <div class="flex flex-wrap items-center gap-1 mb-6">
                        <button on:click=move |_| on_tab_change(AdminTab::Overview) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::Overview { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_overview")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::Users) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::Users { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_users")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::DlpPolicies) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::DlpPolicies { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_dlp")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::DlpAlerts) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::DlpAlerts { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_dlp_alerts")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::Antivirus) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::Antivirus { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_antivirus")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::Watermarks) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::Watermarks { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_watermarks")}</button>
                        <button on:click=move |_| on_tab_change(AdminTab::Notifications) class=move || format!("px-3 py-1.5 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == AdminTab::Notifications { "bg-blue-600 text-white" } else { "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700" })>{t!("admin.tab_notifications")}</button>
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

                    {/* Overview Tab */}
                    {move || (tab.get() == AdminTab::Overview && !loading.get()).then(|| view! {
                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                            <div class="surface brutal-border rounded-lg shadow-sm p-6">
                                <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("admin.storage")}</h2>
                                <StorageStatsCard stats=storage_stats.into() />
                            </div>
                            <div class="surface brutal-border rounded-lg shadow-sm p-6">
                                <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("admin.share_links")}</h2>
                                <ShareLinksCard links=share_links.into() />
                            </div>
                            <div class="surface brutal-border rounded-lg shadow-sm p-6">
                                <h2 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("admin.recent_activity")}</h2>
                                <AuditLogCard entries=audit_entries.into() />
                            </div>
                        </div>
                    })}

                    {/* Users Tab */}
                    {move || (tab.get() == AdminTab::Users).then(|| view! {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.user_management")}</h2>
                                <div class="flex items-center gap-2">
                                    <button
                                        on:click=move |_| {
                                            set_transfer_source.set(String::new());
                                            set_transfer_target.set(String::new());
                                        }
                                        class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                    >
                                        {t!("admin.account_transfer")}
                                    </button>
                                    <button
                                        on:click=move |_| set_show_create_user.set(true)
                                        class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors"
                                    >
                                        {t!("admin.create_user")}
                                    </button>
                                </div>
                            </div>

                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.username")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.email")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.role")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.status")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.created")}</th>
                                            <th class="px-4 py-3"></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || users.get()
                                            key=|u| u.id.clone()
                                            let:user
                                        >
                                            {
                                                let uid = user.id.clone();
                                                let uid2 = uid.clone();
                                                let uid3 = uid.clone();
                                                let username = user.username.clone();
                                                let email = user.email.clone();
                                                let role = user.role.clone();
                                                let role2 = role.clone();
                                                let is_active = user.is_active;
                                                let created = user.created_at.clone();
                                                let created_display = if created.len() >= 10 { created[..10].to_string() } else { created.clone() };
                                                let user_clone = user.clone();
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white">{username}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300">{email}</td>
                                                        <td class="px-4 py-3 text-sm font-mono">
                                                            <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if role2 == "admin" { "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400" } else { "bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300" })>{role}</span>
                                                        </td>
                                                        <td class="px-4 py-3 text-sm font-mono">
                                                            <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if is_active { "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" } else { "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400" })>{if is_active { t!("common.enabled") } else { t!("common.disabled") }}</span>
                                                        </td>
                                                        <td class="px-4 py-3 text-xs font-mono text-gray-500">{created_display}</td>
                                                        <td class="px-4 py-3">
                                                            <div class="flex items-center gap-1">
                                                                <button on:click=move |_| { set_editing_user.set(Some(user_clone.clone())); set_show_create_user.set(true); } class="text-xs text-blue-600 hover:text-blue-800 font-medium transition-colors">{t!("common.edit")}</button>
                                                                <button on:click=move |_| { fetch_devices(&uid2); set_show_devices.set(true); } class="text-xs text-gray-600 hover:text-gray-800 font-medium transition-colors">{t!("admin.devices")}</button>
                                                                <button on:click=move |_| { let u = uid3.clone(); spawn_local(async move { let _ = api::fetch_json_with_method(&format!("/api/admin/users/{}", u), "DELETE", None).await; }); fetch_users(); } class="text-xs text-red-600 hover:text-red-800 font-medium transition-colors">{t!("common.delete")}</button>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        </For>
                                    </tbody>
                                </table>
                                {move || users.with(Vec::is_empty).then(|| view! {
                                    <div class="p-8 text-center text-gray-500 text-sm">{t!("admin.no_users")}</div>
                                })}
                            </div>

                            {/* Account Transfer */}
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-6">
                                <h3 class="text-sm font-bold uppercase font-mono text-gray-500 mb-4">{t!("admin.account_transfer")}</h3>
                                <div class="flex items-end gap-4">
                                    <div class="flex-1">
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.source_user")}</label>
                                        <select prop:value=move || transfer_source.get() on:change=move |ev| set_transfer_source.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500">
                                            <option value="">{t!("admin.select_user")}</option>
                                            {move || users.get().iter().map(|u| { let uid = u.id.clone(); let un = u.username.clone(); view! { <option value=uid.clone() selected=move || transfer_source.get() == uid>{un}</option> } }).collect::<Vec<_>>()}
                                        </select>
                                    </div>
                                    <div class="flex-1">
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.target_user")}</label>
                                        <select prop:value=move || transfer_target.get() on:change=move |ev| set_transfer_target.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500">
                                            <option value="">{t!("admin.select_user")}</option>
                                            {move || users.get().iter().map(|u| { let uid = u.id.clone(); let un = u.username.clone(); view! { <option value=uid.clone() selected=move || transfer_target.get() == uid>{un}</option> } }).collect::<Vec<_>>()}
                                        </select>
                                    </div>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let src = transfer_source.get();
                                            let tgt = transfer_target.get();
                                            if !src.is_empty() && !tgt.is_empty() {
                                                spawn_local(async move {
                                                    let body = serde_json::json!({ "source_user": src, "target_user": tgt });
                                                    let _ = api::fetch_json_with_method("/api/admin/transfer", "POST", Some(&body.to_string())).await;
                                                });
                                            }
                                        }
                                        class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors"
                                    >
                                        {t!("admin.transfer")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    })}

                    {/* DLP Policies Tab */}
                    {move || (tab.get() == AdminTab::DlpPolicies).then(|| view! {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.dlp_policies")}</h2>
                                <button on:click=move |_| set_show_create_policy.set(true) class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors">{t!("admin.create_policy")}</button>
                            </div>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.policy_name")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.description")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.rule_type")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.action")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.status")}</th>
                                            <th class="px-4 py-3"></th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || dlp_policies.get()
                                            key=|p| p.id.clone()
                                            let:policy
                                        >
                                            {
                                                let pid = policy.id.clone();
                                                let name = policy.name.clone();
                                                let desc = policy.description.clone();
                                                let rtype = policy.rule_type.clone();
                                                let action = policy.action.clone();
                                                let enabled = policy.enabled;
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white">{name}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 truncate max-w-xs">{desc}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300">{rtype}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300">{action}</td>
                                                        <td class="px-4 py-3"><span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if enabled { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-500" })>{if enabled { t!("common.enabled") } else { t!("common.disabled") }}</span></td>
                                                        <td class="px-4 py-3">
                                                            <button on:click=move |_| { let p = pid.clone(); spawn_local(async move { let _ = api::fetch_json_with_method(&format!("/api/admin/dlp/policies/{}", p), "DELETE", None).await; }); fetch_dlp(); } class="text-xs text-red-600 hover:text-red-800 font-medium transition-colors">{t!("common.delete")}</button>
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        </For>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    })}

                    {/* DLP Alerts Tab */}
                    {move || (tab.get() == AdminTab::DlpAlerts).then(|| view! {
                        <div class="space-y-4">
                            <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.dlp_alerts")}</h2>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.policy")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.user")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.filename")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.detected_at")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.status")}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || dlp_alerts.get()
                                            key=|a| a.id.clone()
                                            let:alert
                                        >
                                            {
                                                let policy = alert.policy_name.clone();
                                                let user = alert.user.clone();
                                                let filename = alert.filename.clone();
                                                let detected = alert.detected_at.clone();
                                                let detected_display = if detected.len() >= 10 { detected[..10].to_string() } else { detected.clone() };
                                                let status = alert.status.clone();
                                                let status2 = status.clone();
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white">{policy}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300">{user}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 truncate max-w-xs">{filename}</td>
                                                        <td class="px-4 py-3 text-xs font-mono text-gray-500">{detected_display}</td>
                                                        <td class="px-4 py-3"><span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", match status2.as_str() { "resolved" => "bg-green-100 text-green-700", "pending" => "bg-yellow-100 text-yellow-700", _ => "bg-red-100 text-red-700" })>{status}</span></td>
                                                    </tr>
                                                }
                                            }
                                        </For>
                                    </tbody>
                                </table>
                                {move || dlp_alerts.with(Vec::is_empty).then(|| view! {
                                    <div class="p-8 text-center text-gray-500 text-sm">{t!("admin.no_alerts")}</div>
                                })}
                            </div>
                        </div>
                    })}

                    {/* Antivirus Tab */}
                    {move || (tab.get() == AdminTab::Antivirus).then(|| view! {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.antivirus")}</h2>
                                <button
                                    on:click=move |_| {
                                        set_av_scan_running.set(true);
                                        spawn_local(async move {
                                            let _ = api::fetch_json_with_method("/api/admin/antivirus/scan", "POST", None).await;
                                            set_av_scan_running.set(false);
                                            fetch_av();
                                        });
                                    }
                                    disabled=move || av_scan_running.get()
                                    class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50"
                                >
                                    {move || if av_scan_running.get() { t!("admin.scanning") } else { t!("admin.trigger_scan") }}
                                </button>
                            </div>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.scan_id")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.status")}</th>
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.scanned_at")}</th>
                                            <th class="px-4 py-3 text-right text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.files_scanned")}</th>
                                            <th class="px-4 py-3 text-right text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.threats_found")}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || av_scans.get()
                                            key=|s| s.id.clone()
                                            let:scan
                                        >
                                            {
                                                let sid = scan.id.clone();
                                                let status = scan.status.clone();
                                                let status2 = status.clone();
                                                let scanned_at = scan.scanned_at.clone();
                                                let scanned_display = if scanned_at.len() >= 10 { scanned_at[..10].to_string() } else { scanned_at };
                                                let files = scan.files_scanned;
                                                let threats = scan.threats_found;
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white">{sid}</td>
                                                        <td class="px-4 py-3"><span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", match status2.as_str() { "clean" => "bg-green-100 text-green-700", "threats" => "bg-red-100 text-red-700", "running" => "bg-yellow-100 text-yellow-700", _ => "bg-gray-100 text-gray-500" })>{status}</span></td>
                                                        <td class="px-4 py-3 text-xs font-mono text-gray-500">{scanned_display}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-700 dark:text-gray-300 text-right">{files}</td>
                                                        <td class="px-4 py-3 text-sm font-mono text-right"><span class=move || format!("{}", if threats > 0 { "text-red-600 font-bold" } else { "text-gray-500" })>{threats}</span></td>
                                                    </tr>
                                                }
                                            }
                                        </For>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    })}

                    {/* Watermarks Tab */}
                    {move || (tab.get() == AdminTab::Watermarks).then(|| view! {
                        <div class="space-y-4">
                            <div class="flex items-center justify-between">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.watermark_policies")}</h2>
                                <button on:click=move |_| set_show_create_watermark.set(true) class="px-4 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 transition-colors">{t!("admin.create_watermark")}</button>
                            </div>
                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                <For
                                    each=move || watermark_policies.get()
                                    key=|w| w.id.clone()
                                    let:wm
                                >
                                    {
                                        let name = wm.name.clone();
                                        let pattern = wm.pattern.clone();
                                        let opacity = wm.opacity;
                                        let enabled = wm.enabled;
                                        let wid = wm.id.clone();
                                        view! {
                                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border p-4">
                                                <div class="flex items-center justify-between mb-2">
                                                    <span class="text-sm font-bold font-mono text-gray-900 dark:text-white">{name}</span>
                                                    <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if enabled { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-500" })>{if enabled { t!("common.enabled") } else { t!("common.disabled") }}</span>
                                                </div>
                                                <div class="text-xs font-mono text-gray-500 mb-1">{t!("admin.pattern")}: {pattern}</div>
                                                <div class="text-xs font-mono text-gray-500 mb-3">{t!("admin.opacity")}: {opacity}</div>
                                                <button on:click=move |_| { let w = wid.clone(); spawn_local(async move { let _ = api::fetch_json_with_method(&format!("/api/admin/watermarks/{}", w), "DELETE", None).await; }); fetch_watermarks(); } class="text-xs text-red-600 hover:text-red-800 font-medium transition-colors">{t!("common.delete")}</button>
                                            </div>
                                        }
                                    }
                                </For>
                            </div>
                        </div>
                    })}

                    {/* Notifications Tab */}
                    {move || (tab.get() == AdminTab::Notifications).then(|| view! {
                        <div class="space-y-4">
                            <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("admin.notification_preferences")}</h2>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm brutal-border overflow-hidden">
                                <table class="w-full">
                                    <thead>
                                        <tr class="border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                                            <th class="px-4 py-3 text-left text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.event_type")}</th>
                                            <th class="px-4 py-3 text-center text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.email")}</th>
                                            <th class="px-4 py-3 text-center text-xs font-bold uppercase font-mono text-gray-500">{t!("admin.push")}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || notification_prefs.get()
                                            key=|p| p.event_type.clone()
                                            let:pref
                                        >
                                            {
                                                let event = pref.event_type.clone();
                                                let email = pref.email_enabled;
                                                let push = pref.push_enabled;
                                                view! {
                                                    <tr class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                                        <td class="px-4 py-3 text-sm font-mono text-gray-900 dark:text-white">{event}</td>
                                                        <td class="px-4 py-3 text-center">
                                                            <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if email { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-500" })>{if email { t!("common.enabled") } else { t!("common.disabled") }}</span>
                                                        </td>
                                                        <td class="px-4 py-3 text-center">
                                                            <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if push { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-500" })>{if push { t!("common.enabled") } else { t!("common.disabled") }}</span>
                                                        </td>
                                                    </tr>
                                                }
                                            }
                                        </For>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    })}

                    {/* Create User Dialog */}
                    {move || show_create_user.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| { set_show_create_user.set(false); set_editing_user.set(None); }>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{move || if editing_user.get().is_some() { t!("admin.edit_user") } else { t!("admin.create_user") }}</h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.username")}</label>
                                        <input type="text" prop:value=move || new_user_username.get() on:input=move |ev| set_new_user_username.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500" />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.email")}</label>
                                        <input type="email" prop:value=move || new_user_email.get() on:input=move |ev| set_new_user_email.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500" />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.role")}</label>
                                        <select prop:value=move || new_user_role.get() on:change=move |ev| set_new_user_role.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500">
                                            <option value="user">{t!("admin.role_user")}</option>
                                            <option value="admin">{t!("admin.role_admin")}</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="flex items-center justify-end gap-3 mt-6">
                                    <button on:click=move |_: ev::MouseEvent| { set_show_create_user.set(false); set_editing_user.set(None); } class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">{t!("common.cancel")}</button>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let username = new_user_username.get();
                                            let email = new_user_email.get();
                                            let role = new_user_role.get();
                                            let editing = editing_user.get();
                                            spawn_local(async move {
                                                let body = serde_json::json!({ "username": username, "email": email, "role": role });
                                                if let Some(user) = editing {
                                                    let _ = api::fetch_json_with_method(&format!("/api/admin/users/{}", user.id), "PUT", Some(&body.to_string())).await;
                                                } else {
                                                    let _ = api::fetch_json_with_method("/api/admin/users", "POST", Some(&body.to_string())).await;
                                                }
                                            });
                                            set_show_create_user.set(false);
                                            set_editing_user.set(None);
                                            fetch_users();
                                        }
                                        class="px-4 py-2 text-sm font-bold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                                    >
                                        {t!("common.save")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    })}

                    {/* Create DLP Policy Dialog */}
                    {move || show_create_policy.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_policy.set(false)>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{t!("admin.create_policy")}</h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.policy_name")}</label>
                                        <input type="text" prop:value=move || new_policy_name.get() on:input=move |ev| set_new_policy_name.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500" />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.description")}</label>
                                        <textarea prop:value=move || new_policy_desc.get() on:input=move |ev| set_new_policy_desc.set(event_target_value(&ev)) rows="2" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500"></textarea>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.rule_type")}</label>
                                        <select prop:value=move || new_policy_type.get() on:change=move |ev| set_new_policy_type.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500">
                                            <option value="content">{t!("admin.dlp_content")}</option>
                                            <option value="metadata">{t!("admin.dlp_metadata")}</option>
                                            <option value="filename">{t!("admin.dlp_filename")}</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.action")}</label>
                                        <select prop:value=move || new_policy_action.get() on:change=move |ev| set_new_policy_action.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500">
                                            <option value="block">{t!("admin.dlp_block")}</option>
                                            <option value="warn">{t!("admin.dlp_warn")}</option>
                                            <option value="log">{t!("admin.dlp_log")}</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="flex items-center justify-end gap-3 mt-6">
                                    <button on:click=move |_| set_show_create_policy.set(false) class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">{t!("common.cancel")}</button>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let name = new_policy_name.get();
                                            let desc = new_policy_desc.get();
                                            let rtype = new_policy_type.get();
                                            let action = new_policy_action.get();
                                            spawn_local(async move {
                                                let body = serde_json::json!({ "name": name, "description": desc, "rule_type": rtype, "action": action, "enabled": true });
                                                let _ = api::fetch_json_with_method("/api/admin/dlp/policies", "POST", Some(&body.to_string())).await;
                                            });
                                            set_show_create_policy.set(false);
                                            fetch_dlp();
                                        }
                                        class="px-4 py-2 text-sm font-bold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                                    >
                                        {t!("common.create")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    })}

                    {/* Create Watermark Dialog */}
                    {move || show_create_watermark.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_watermark.set(false)>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{t!("admin.create_watermark")}</h3>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.policy_name")}</label>
                                        <input type="text" prop:value=move || new_wm_name.get() on:input=move |ev| set_new_wm_name.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500" />
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.pattern")}</label>
                                        <input type="text" prop:value=move || new_wm_pattern.get() on:input=move |ev| set_new_wm_pattern.set(event_target_value(&ev)) class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm focus:ring-2 focus:ring-blue-500" />
                                        <p class="text-xs text-gray-500 mt-1">{t!("admin.pattern_hint")}</p>
                                    </div>
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("admin.opacity")}</label>
                                        <input type="range" min="0" max="1" step="0.1" prop:value=move || new_wm_opacity.get().to_string() on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<f64>() { set_new_wm_opacity.set(v); } } class="w-full" />
                                        <span class="text-xs font-mono text-gray-500">{move || format!("{:.1}", new_wm_opacity.get())}</span>
                                    </div>
                                </div>
                                <div class="flex items-center justify-end gap-3 mt-6">
                                    <button on:click=move |_| set_show_create_watermark.set(false) class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">{t!("common.cancel")}</button>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let name = new_wm_name.get();
                                            let pattern = new_wm_pattern.get();
                                            let opacity = new_wm_opacity.get();
                                            spawn_local(async move {
                                                let body = serde_json::json!({ "name": name, "pattern": pattern, "opacity": opacity, "enabled": true });
                                                let _ = api::fetch_json_with_method("/api/admin/watermarks", "POST", Some(&body.to_string())).await;
                                            });
                                            set_show_create_watermark.set(false);
                                            fetch_watermarks();
                                        }
                                        class="px-4 py-2 text-sm font-bold text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                                    >
                                        {t!("common.create")}
                                    </button>
                                </div>
                            </div>
                        </div>
                    })}

                    {/* Devices Dialog */}
                    {move || show_devices.get().then(|| view! {
                        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_devices.set(false)>
                            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{t!("admin.user_devices")}</h3>
                                <div class="space-y-2">
                                    <For
                                        each=move || selected_user_devices.get()
                                        key=|d| d.id.clone()
                                        let:device
                                    >
                                        {
                                            let name = device.name.clone();
                                            let dtype = device.device_type.clone();
                                            let last_seen = device.last_seen.clone();
                                            let last_seen_display = if last_seen.len() >= 10 { last_seen[..10].to_string() } else { last_seen };
                                            let is_active = device.is_active;
                                            let did = device.id.clone();
                                            view! {
                                                <div class="flex items-center justify-between py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                                                    <div>
                                                        <div class="text-sm font-mono text-gray-900 dark:text-white">{name} <span class="text-xs text-gray-500">({dtype})</span></div>
                                                        <div class="text-xs text-gray-500">{t!("admin.last_seen")}: {last_seen_display}</div>
                                                    </div>
                                                    <div class="flex items-center gap-2">
                                                        <span class=move || format!("px-2 py-0.5 rounded text-xs font-bold {}", if is_active { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-500" })>{if is_active { t!("common.enabled") } else { t!("common.disabled") }}</span>
                                                        <button on:click=move |_| { let d = did.clone(); spawn_local(async move { let _ = api::fetch_json_with_method(&format!("/api/admin/devices/{}/revoke", d), "POST", None).await; }); } class="text-xs text-red-600 hover:text-red-800 font-medium transition-colors">{t!("admin.revoke")}</button>
                                                    </div>
                                                </div>
                                            }
                                        }
                                    </For>
                                </div>
                                <div class="mt-4 flex justify-end">
                                    <button on:click=move |_| set_show_devices.set(false) class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">{t!("common.close")}</button>
                                </div>
                            </div>
                        </div>
                    })}
                </main>
            </div>
        </div>
    }
}

#[component]
fn StorageStatsCard(stats: Signal<Option<serde_json::Value>>) -> impl IntoView {
    view! {
        <div>
            {move || stats.get().map(|s| view! {
                <div class="space-y-3">
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">{t!("admin.files")}</span>
                        <span class="font-bold font-mono text-gray-900">{s.get("files").and_then(|v| v.as_u64()).unwrap_or(0)}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">{t!("admin.collections")}</span>
                        <span class="font-bold font-mono text-gray-900">{s.get("collections").and_then(|v| v.as_u64()).unwrap_or(0)}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">{t!("admin.total_size")}</span>
                        <span class="font-bold font-mono text-gray-900">{format_size(s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0))}</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600 font-mono text-sm">{t!("admin.cas_dedup")}</span>
                        <span class={if s.get("cas").and_then(|c| c.get("enabled")).and_then(|e| e.as_bool()).unwrap_or(false) { "text-green-600" } else { "text-gray-500" }}>
                            {if s.get("cas").and_then(|c| c.get("enabled")).and_then(|e| e.as_bool()).unwrap_or(false) { t!("common.enabled") } else { t!("common.disabled") }}
                        </span>
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
fn ShareLinksCard(links: Signal<Vec<serde_json::Value>>) -> impl IntoView {
    view! {
        <div>
            {move || links.with(Vec::is_empty).then(|| view! {
                <div class="text-sm text-gray-500">{t!("empty.share_links")}</div>
            })}
            <For
                each=move || links.get()
                key=|s| s.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string()
                let:share
            >
                {move || {
                    let path = share.get("path").and_then(|p| p.as_str()).unwrap_or("?").to_string();
                    let expires = share.get("expires_at").and_then(|e| e.as_str()).unwrap_or("?").to_string();
                    view! {
                        <div class="py-2 border-b border-gray-100 dark:border-gray-700 last:border-0">
                            <div class="text-sm font-mono text-gray-900 dark:text-white">{path}</div>
                            <div class="text-xs text-gray-500 mt-0.5">{t!("admin.expires")} {expires}</div>
                        </div>
                    }
                }}
            </For>
        </div>
    }
}

#[component]
fn AuditLogCard(entries: Signal<Vec<serde_json::Value>>) -> impl IntoView {
    view! {
        <div>
            {move || entries.with(Vec::is_empty).then(|| view! {
                <div class="text-sm text-gray-500">{t!("empty.admin_activity")}</div>
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
                        <div class="py-1.5 border-b border-gray-100 dark:border-gray-700 last:border-0 text-xs">
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
