use leptos::*;

use crate::api::ApiState;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::chart::BarChart;
use crate::components::stats_card::StatsCard;
use crate::state::{format_bytes, format_timestamp, format_uptime};

#[component]
pub fn DashboardPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (stats, set_stats) = create_signal(None::<serde_json::Value>);
    let (storage, set_storage) = create_signal(None::<serde_json::Value>);
    let (audit_entries, set_audit_entries) = create_signal(Vec::<serde_json::Value>::new());
    let (health, set_health) = create_signal(None::<serde_json::Value>);
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);

    create_effect(move |_| {
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
            set_loading.set(false);
        });
    });

    view! {
        <div class="page">
            {move || loading.get().then(|| view! { <div class="loading" role="status" aria-live="polite">"Loading dashboard..."</div> })}
            {move || error.get().map(|e| view! { <div class="error-banner" role="alert" aria-live="assertive">{e}</div> })}

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
                let recent = s.get("recent_files").and_then(|f| f.as_array()).cloned().unwrap_or_default();
                let mut chart_data: Vec<(String, f64)> = recent.iter().take(8).map(|f| {
                    let name = f.get("path").and_then(|p| p.as_str()).unwrap_or("?").rsplit('/').next().unwrap_or("?").chars().take(12).collect();
                    let size = f.get("size").and_then(|sz| sz.as_f64()).unwrap_or(0.0);
                    (name, size)
                }).collect();
                chart_data.reverse();
                Some(view! {
                    <div class="dashboard-panels">
                        <div class="panel">
                            <h3 class="panel-title">"Storage by Recent Files"</h3>
                            <BarChart data=chart_data title="".to_string() color="#E85D04".to_string() />
                        </div>

                        <div class="panel">
                            <h3 class="panel-title">"Recent Activity"</h3>
                            <div class="activity-list">
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
                    </div>
                })
            }}
        </div>
    }
}
