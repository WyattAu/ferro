use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::chart::{BarChart, LineChart, PieChart};
use crate::components::stats_card::StatsCard;
use crate::state::{format_timestamp, format_uptime};
use ferro_common::format::format_size as format_bytes;

#[component]
pub fn DashboardPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (stats, set_stats) = signal(None::<serde_json::Value>);
    let (storage, set_storage) = signal(None::<serde_json::Value>);
    let (audit_entries, set_audit_entries) = signal(Vec::<serde_json::Value>::new());
    let (health, set_health) = signal(None::<serde_json::Value>);
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(true);
    let (users, set_users) = signal(Vec::<serde_json::Value>::new());

    Effect::new(move |_| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.server_stats().await {
                Ok(s) => set_stats.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
            if let Ok(s) = api_clone.storage_info().await {
                set_storage.set(Some(s))
            }
            if let Ok(a) = api_clone.audit_log(10, 0).await
                && let Some(entries) = a.get("entries").and_then(|e| e.as_array())
            {
                set_audit_entries.set(entries.clone());
            }
            if let Ok(h) = api_clone.server_health().await {
                set_health.set(Some(h))
            }
            if let Ok(u) = api_clone.list_users().await {
                set_users.set(u);
            }
            set_loading.set(false);
        });
    });

    view! {
        <div class="page">
            <div aria-live="polite">
                {move || loading.get().then(|| view! { <div class="loading" role="status" aria-live="polite">"Loading dashboard..."</div> })}
            </div>
            <div aria-live="assertive">
                {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}
            </div>

            {move || {
                let s = stats.get()?;
                let total_files = s.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0).to_string();
                let total_dirs = s.get("total_directories").and_then(|v| v.as_u64()).unwrap_or(0).to_string();
                let total_bytes = format_bytes(s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0));
                let uptime = format_uptime(s.get("uptime_seconds").and_then(|v| v.as_u64()).unwrap_or(0));
                let version = s.get("version").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let h = health.get()?;
                let health_status = h.get("status").and_then(|st| st.as_str()).unwrap_or("unknown").to_string();
                Some(view! {
                    <div class="stats-grid">
                        <StatsCard title="Total Files".to_string() value=total_files />
                        <StatsCard title="Directories".to_string() value=total_dirs />
                        <StatsCard title="Storage Used".to_string() value=total_bytes />
                        <StatsCard title="Uptime".to_string() value=uptime />
                        <StatsCard title="Version".to_string() value=version />
                        <StatsCard title="Health".to_string() value=health_status />
                    </div>
                })
            }}

            {move || {
                let s = storage.get()?;
                let total_bytes = s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                let total_capacity = s.get("total_capacity").and_then(|v| v.as_u64()).unwrap_or(0);
                let free_bytes = total_capacity.saturating_sub(total_bytes);

                let pie_data = vec![
                    ("Used".to_string(), total_bytes as f64),
                    ("Free".to_string(), free_bytes as f64),
                ];

                let recent = s.get("recent_files").and_then(|f| f.as_array()).cloned().unwrap_or_default();

                let mut file_type_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                for f in &recent {
                    let path = f.get("path").and_then(|p| p.as_str()).unwrap_or("");
                    let size = f.get("size").and_then(|sz| sz.as_f64()).unwrap_or(0.0);
                    let ext = path.rsplit('.').next().unwrap_or("other").to_uppercase();
                    let category = match ext.as_str() {
                        "PDF" | "DOC" | "DOCX" | "TXT" | "CSV" | "XLS" | "XLSX" | "PPT" | "PPTX" => "Documents",
                        "JPG" | "JPEG" | "PNG" | "GIF" | "RAW" | "HEIC" | "SVG" => "Images",
                        "MP4" | "MOV" | "AVI" | "MKV" | "WEBM" => "Video",
                        "MP3" | "FLAC" | "WAV" | "OGG" | "AAC" => "Audio",
                        "ZIP" | "TAR" | "GZ" | "7Z" | "RAR" => "Archives",
                        _ => "Other",
                    };
                    *file_type_map.entry(category.to_string()).or_insert(0.0) += size;
                }
                let mut type_data: Vec<(String, f64)> = file_type_map.into_iter().collect();
                type_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                type_data.truncate(6);

                Some(view! {
                    <div class="dashboard-panels">
                        // Storage usage pie chart
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Storage Usage"</h2>
                            <PieChart data=pie_data title="Used vs Free".to_string() />
                        </div>

                        // File type distribution
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"File Types"</h2>
                            <BarChart data=type_data title="Storage by Type".to_string() color="#E85D04".to_string() />
                        </div>

                        // Activity line chart
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Recent Activity"</h2>
                            <div class="activity-list" aria-label="Recent activity list">
                                {audit_entries.get().iter().take(10).map(|entry| {
                                    let action = entry.get("action").and_then(|a| a.as_str()).unwrap_or("unknown").to_string();
                                    let user = entry.get("user").and_then(|u| u.as_str()).unwrap_or("system").to_string();
                                    let resource = entry.get("resource").and_then(|r| r.as_str()).unwrap_or("-").to_string();
                                    let timestamp = entry.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string();
                                    let status = entry.get("status").and_then(|st| st.as_str()).unwrap_or("success").to_string();
                                    let bv = if status == "error" { BadgeVariant::Danger } else { BadgeVariant::Success };
                                    view! {
                                        <div class="activity-item">
                                            <div class="activity-main">
                                                <span class="activity-action">{action}</span>
                                                <span class="activity-resource">{resource}</span>
                                            </div>
                                            <div class="activity-meta">
                                                <span class="activity-user">{user}</span>
                                                <Badge text=status variant=bv/>
                                                <span class="activity-time">{format_timestamp(&timestamp)}</span>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Upload/download activity line chart
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Activity Over Time"</h2>
                            {move || {
                                let entries = audit_entries.get();
                                let mut uploads: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                                let mut downloads: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

                                for entry in &entries {
                                    let action = entry.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                    let timestamp = entry.get("timestamp").and_then(|t| t.as_str()).unwrap_or("");
                                    let day = if timestamp.len() >= 10 {
                                        &timestamp[..10]
                                    } else {
                                        "unknown"
                                    };
                                    match action {
                                        "upload" | "create" => {
                                            *uploads.entry(day.to_string()).or_insert(0.0) += 1.0;
                                        }
                                        "download" | "read" => {
                                            *downloads.entry(day.to_string()).or_insert(0.0) += 1.0;
                                        }
                                        _ => {}
                                    }
                                }

                                let mut all_days: Vec<String> = uploads.keys().chain(downloads.keys()).cloned().collect();
                                all_days.sort();
                                all_days.dedup();

                                let upload_data: Vec<(String, f64)> = all_days.iter().map(|d| {
                                    (d[5..].to_string(), uploads.get(d).copied().unwrap_or(0.0))
                                }).collect();
                                let download_data: Vec<(String, f64)> = all_days.iter().map(|d| {
                                    (d[5..].to_string(), downloads.get(d).copied().unwrap_or(0.0))
                                }).collect();

                                if upload_data.is_empty() && download_data.is_empty() {
                                    view! {
                                        <div class="text-sm text-center py-8" style="color: var(--text-secondary)">"No activity data yet"</div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-2">
                                            <div>
                                                <div class="text-xs font-mono mb-1" style="color: var(--text-secondary)">"Uploads"</div>
                                                <LineChart data=upload_data title="".to_string() color="#16a34a".to_string() />
                                            </div>
                                            <div>
                                                <div class="text-xs font-mono mb-1" style="color: var(--text-secondary)">"Downloads"</div>
                                                <LineChart data=download_data title="".to_string() color="#2563eb".to_string() />
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>

                        // User growth chart
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Users"</h2>
                            {move || {
                                let user_list = users.get();
                                let total_users = user_list.len();
                                let mut role_counts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                                for u in &user_list {
                                    let role = u.get("role").and_then(|r| r.as_str()).unwrap_or("user").to_string();
                                    *role_counts.entry(role).or_insert(0.0) += 1.0;
                                }
                                let role_data: Vec<(String, f64)> = role_counts.into_iter().collect();
                                if role_data.is_empty() {
                                    view! {
                                        <div class="text-sm text-center py-8" style="color: var(--text-secondary)">
                                            {format!("{} users total", total_users)}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <BarChart data=role_data title=format!("{} Total Users", total_users) color="#E85D04".to_string() />
                                    }.into_any()
                                }
                            }}
                        </div>

                        // File size line chart from recent files
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Recent Uploads"</h2>
                            {move || {
                                let s = storage.get()?;
                                let recent = s.get("recent_files").and_then(|f| f.as_array()).cloned().unwrap_or_default();
                                let mut chart_data: Vec<(String, f64)> = recent.iter().take(12).map(|f| {
                                    let name = f.get("path").and_then(|p| p.as_str()).unwrap_or("?").rsplit('/').next().unwrap_or("?").chars().take(10).collect();
                                    let size = f.get("size").and_then(|sz| sz.as_f64()).unwrap_or(0.0);
                                    (name, size)
                                }).collect();
                                chart_data.reverse();
                                if chart_data.is_empty() {
                                    None
                                } else {
                                    Some(view! {
                                        <LineChart data=chart_data title="File Size (bytes)".to_string() color="#E85D04".to_string() />
                                    })
                                }
                            }}
                        </div>
                    </div>
                })
            }}
        </div>
    }
}
