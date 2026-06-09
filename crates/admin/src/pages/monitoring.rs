use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::ev;

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
            <div aria-live="polite">
                {move || loading.get().then(|| view! { <div class="loading" role="status">"Loading monitoring data..."</div> })}
            </div>
            <div aria-live="assertive">
                {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}
            </div>

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
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Uptime"</span></div><div class="stats-card-value">{uptime}</div></div>
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Version"</span></div><div class="stats-card-value">{version}</div></div>
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Auth"</span></div><div class="stats-card-value">{auth_type}</div></div>
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Search"</span></div><div class="stats-card-value">{if search_on { "Enabled" } else { "Disabled" }}</div></div>
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Storage"</span></div><div class="stats-card-value">{storage_backend}</div></div>
                            <div class="stats-card surface"><div class="stats-card-header"><span class="stats-card-title font-display">"Total Files"</span></div><div class="stats-card-value">{total_files}</div></div>
                        </div>
                        <div class="panel surface">
                            <h2 class="panel-title font-display">"Server Features"</h2>
                            <div class="feature-grid">{feature_rows}</div>
                        </div>
                    </>
                })
            }}

            <div class="panel surface brutal-border">
                <div class="panel-header-row">
                    <h2 class="panel-title font-display">"Prometheus Metrics"</h2>
                    <button class="btn btn-secondary btn-sm" on:click=do_refresh disabled=metrics_loading aria-label=move || if metrics_loading.get() { "Refreshing metrics" } else { "Refresh metrics" }>
                        {move || if metrics_loading.get() { "Refreshing..." } else { "Refresh Metrics" }}
                    </button>
                </div>
                <pre class="metrics-output" aria-label="Prometheus metrics output">{move || metrics_text.get()}</pre>
            </div>

            <div class="panel surface">
                <h2 class="panel-title font-display">"External Grafana"</h2>
                <div class="form-group">
                    <label class="form-label" for="grafana-url">"Grafana Dashboard URL (optional)"</label>
                    <input id="grafana-url" type="url" class="form-input" placeholder="https://grafana.example.com/d/..." prop:value=grafana_url on:input=move |ev| set_grafana_url.set(event_target_value(&ev)) aria-label="Grafana Dashboard URL" />
                </div>
                {move || {
                    let url = grafana_url.get();
                    if !url.is_empty() {
                        view! { <div class="mt-2"><a href=url target="_blank" class="btn btn-secondary">"Open Grafana Dashboard"</a></div> }.into_any()
                    } else {
                        view! { <div class="mt-2 text-secondary-placeholder">"Configure a Grafana URL above to view dashboards"</div> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
