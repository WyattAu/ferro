use leptos::*;

use crate::api::ApiState;
use crate::state::format_uptime;

#[component]
pub fn MonitoringPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (stats, set_stats) = create_signal(None::<serde_json::Value>);
    let (metrics_text, set_metrics_text) = create_signal(String::new());
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);
    let (metrics_loading, set_metrics_loading) = create_signal(false);
    let (grafana_url, set_grafana_url) = create_signal(String::new());

    create_effect(move |_| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.server_stats().await {
                Ok(s) => set_stats.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
            match api_clone.prometheus_metrics().await {
                Ok(m) => set_metrics_text.set(m),
                Err(_) => set_metrics_text.set("(no prometheus metrics available)".to_string()),
            }
            set_loading.set(false);
        });
    });

    let do_refresh = move |_: leptos::ev::MouseEvent| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_metrics_loading.set(true);
            match api_clone.prometheus_metrics().await {
                Ok(m) => set_metrics_text.set(m),
                Err(e) => set_error.set(Some(e)),
            }
            set_metrics_loading.set(false);
        });
    };

    view! {
        <div class="page">
            {move || loading.get().then(|| view! { <div class="loading">"Loading monitoring data..."</div> })}
            {move || error.get().map(|e| view! { <div class="error-banner">{e}</div> })}

            {move || {
                let s = stats.get()?;
                let uptime = format_uptime(s.get("uptime_seconds").and_then(|v| v.as_u64()).unwrap_or(0));
                let version = s.get("version").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let auth_type = s.get("auth_type").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let search_on = s.get("search_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                let storage_backend = s.get("storage_backend").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let total_files = s.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0).to_string();

                let features = s.get("features").and_then(|f| f.as_object()).cloned().unwrap_or_default();
                let mut items: Vec<_> = features.iter().map(|(k, v)| (k.clone(), v.as_bool().unwrap_or(false))).collect();
                items.sort_by(|a, b| a.0.cmp(&b.0));
                let feature_rows: Vec<_> = items.iter().map(|(name, enabled)| {
                    let n = name.clone();
                    let en = *enabled;
                    view! {
                        <div class="feature-item">
                            <span class="feature-name">{n}</span>
                            <span class={if en { "feature-enabled" } else { "feature-disabled" }}>{if en { "Enabled" } else { "Disabled" }}</span>
                        </div>
                    }
                }).collect();

                Some(view! {
                    <>
                        <div class="stats-grid">
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Uptime"</span></div><div class="stats-card-value">{uptime}</div></div>
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Version"</span></div><div class="stats-card-value">{version}</div></div>
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Auth"</span></div><div class="stats-card-value">{auth_type}</div></div>
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Search"</span></div><div class="stats-card-value">{if search_on { "Enabled" } else { "Disabled" }}</div></div>
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Storage"</span></div><div class="stats-card-value">{storage_backend}</div></div>
                            <div class="stats-card"><div class="stats-card-header"><span class="stats-card-title">"Total Files"</span></div><div class="stats-card-value">{total_files}</div></div>
                        </div>
                        <div class="panel">
                            <h3 class="panel-title">"Server Features"</h3>
                            <div class="feature-grid">{feature_rows}</div>
                        </div>
                    </>
                })
            }}

            <div class="panel">
                <div class="panel-header-row">
                    <h3 class="panel-title">"Prometheus Metrics"</h3>
                    <button class="btn btn-secondary btn-sm" on:click=do_refresh disabled=metrics_loading>
                        {move || if metrics_loading.get() { "Refreshing..." } else { "Refresh Metrics" }}
                    </button>
                </div>
                <pre class="metrics-output">{move || metrics_text.get()}</pre>
            </div>

            <div class="panel">
                <h3 class="panel-title">"External Grafana"</h3>
                <div class="form-group">
                    <label class="form-label">"Grafana Dashboard URL (optional)"</label>
                    <input type="url" class="form-input" placeholder="https://grafana.example.com/d/..." prop:value=grafana_url on:input=move |ev| set_grafana_url.set(event_target_value(&ev)) />
                </div>
                {move || {
                    let url = grafana_url.get();
                    if !url.is_empty() {
                        view! { <div style="margin-top:8px"><a href=url target="_blank" class="btn btn-secondary">"Open Grafana Dashboard"</a></div> }
                    } else {
                        view! { <div style="margin-top:8px;color:var(--text-secondary);font-size:13px">"Configure a Grafana URL above to view dashboards"</div> }
                    }
                }}
            </div>
        </div>
    }
}
