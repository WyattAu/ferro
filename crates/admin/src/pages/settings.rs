use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::ApiState;
use crate::state::{format_bytes, format_uptime};

#[component]
pub fn SettingsPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (stats, set_stats) = create_signal(None::<serde_json::Value>);
    let (error, set_error) = create_signal(None::<String>);
    let (loading, set_loading) = create_signal(true);
    let (msg, set_msg) = create_signal(None::<String>);
    let (cors_origins, set_cors_origins) = create_signal(String::new());
    let (session_timeout, set_session_timeout) = create_signal(String::from("3600"));
    let (rate_limit, set_rate_limit) = create_signal(String::from("10000"));
    let (max_file_size, set_max_file_size) = create_signal(String::from("1073741824"));

    create_effect(move |_| {
        let api_clone = api.get_untracked();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match api_clone.server_stats().await {
                Ok(s) => set_stats.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    });

    view! {
        <div class="page">
            <div aria-live="polite">
                {move || loading.get().then(|| view! { <div class="loading" role="status">"Loading settings..."</div> })}
            </div>
            <div aria-live="assertive">
                {move || error.get().map(|e| view! { <div class="error-banner" role="alert">{e}</div> })}
            </div>
            <div aria-live="polite">
                {move || msg.get().map(|m| view! { <div class="success-banner" role="status">{m}</div> })}
            </div>

            {move || {
                let s = stats.get()?;
                let version = s.get("version").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let uptime = format_uptime(s.get("uptime_seconds").and_then(|v| v.as_u64()).unwrap_or(0));
                let storage_backend = s.get("storage_backend").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let auth_type = s.get("auth_type").and_then(|v| v.as_str()).unwrap_or("-").to_string();
                let total_files = s.get("total_files").and_then(|v| v.as_u64()).unwrap_or(0);
                let total_bytes = s.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                Some(view! {
                    <div class="panel surface brutal-border">
                        <h2 class="panel-title font-display">"Server Information"</h2>
                        <div class="settings-grid">
                            <div class="detail-row"><span class="detail-label">"Version"</span><span class="detail-value">{version}</span></div>
                            <div class="detail-row"><span class="detail-label">"Uptime"</span><span class="detail-value">{uptime}</span></div>
                            <div class="detail-row"><span class="detail-label">"Storage Backend"</span><span class="detail-value">{storage_backend}</span></div>
                            <div class="detail-row"><span class="detail-label">"Auth Type"</span><span class="detail-value">{auth_type}</span></div>
                            <div class="detail-row"><span class="detail-label">"Total Files"</span><span class="detail-value">{total_files}</span></div>
                            <div class="detail-row"><span class="detail-label">"Total Storage"</span><span class="detail-value">{format_bytes(total_bytes)}</span></div>
                        </div>
                    </div>
                })
            }}

            <div class="panel surface">
                <h2 class="panel-title font-display">"Authentication Settings"</h2>
                <div class="form-group">
                    <label class="form-label" for="session-timeout">"Session Timeout (seconds)"</label>
                    <input id="session-timeout" type="number" class="form-input form-input-half" prop:value=session_timeout on:input=move |ev| set_session_timeout.set(event_target_value(&ev)) />
                    <span class="form-hint">"Duration in seconds before a session expires"</span>
                </div>
                <button class="btn btn-primary" on:click=move |_| set_msg.set(Some("Auth settings saved (requires server restart)".to_string())) aria-label="Save authentication settings">"Save Auth Settings"</button>
            </div>

            <div class="panel surface">
                <h2 class="panel-title font-display">"CORS Configuration"</h2>
                <div class="form-group">
                    <label class="form-label" for="cors-origins">"Allowed Origins"</label>
                    <input id="cors-origins" type="text" class="form-input" placeholder="https://example.com, https://app.example.com (or * for all)" prop:value=cors_origins on:input=move |ev| set_cors_origins.set(event_target_value(&ev)) />
                    <span class="form-hint">"Comma-separated list of allowed origins."</span>
                </div>
                <button class="btn btn-primary" on:click=move |_| set_msg.set(Some("CORS settings saved (requires server restart)".to_string())) aria-label="Save CORS settings">"Save CORS Settings"</button>
            </div>

            <div class="panel surface">
                <h2 class="panel-title font-display">"Rate Limiting"</h2>
                <div class="form-group">
                    <label class="form-label" for="rate-limit">"Max Requests Per Minute"</label>
                    <input id="rate-limit" type="number" class="form-input form-input-half" prop:value=rate_limit on:input=move |ev| set_rate_limit.set(event_target_value(&ev)) />
                    <span class="form-hint">"Maximum requests per client IP per minute"</span>
                </div>
                <div class="form-group">
                    <label class="form-label" for="max-file-size">"Max File Size (bytes)"</label>
                    <input id="max-file-size" type="number" class="form-input form-input-half" prop:value=max_file_size on:input=move |ev| set_max_file_size.set(event_target_value(&ev)) />
                    <span class="form-hint">"Default: 1073741824 (1GB)"</span>
                </div>
                <button class="btn btn-primary" on:click=move |_| set_msg.set(Some("Rate limit settings saved (requires server restart)".to_string())) aria-label="Save rate limit settings">"Save Rate Limits"</button>
            </div>

            <div class="panel surface brutal-border">
                <h2 class="panel-title font-display text-accent">"Danger Zone"</h2>
                <div class="detail-row">
                    <span class="detail-label">"Note"</span>
                    <span class="detail-value">"Settings changes require a server restart. Some settings can only be configured via the server configuration file."</span>
                </div>
            </div>
        </div>
    }
}
