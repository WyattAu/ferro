use leptos::*;

use crate::api::{ApiState, ServerConfig};

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else {
        format!("{}m {}s", minutes, secs)
    }
}

pub fn format_timestamp(ts: &str) -> String {
    if ts.is_empty() {
        return "-".to_string();
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        ts.to_string()
    }
}

pub fn save_connection(config: &ServerConfig) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item("ferro_admin_url", &config.url);
        let _ = storage.set_item("ferro_admin_token", &config.token);
    }
}

pub fn load_connection() -> Option<ServerConfig> {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let url = storage.get_item("ferro_admin_url").ok()??;
        let token = storage.get_item("ferro_admin_token").ok()??;
        if !url.is_empty() && !token.is_empty() {
            return Some(ServerConfig { url, token });
        }
    }
    None
}

pub fn clear_connection() {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.remove_item("ferro_admin_url");
        let _ = storage.remove_item("ferro_admin_token");
    }
}

pub fn provide_api_state() -> RwSignal<ApiState> {
    let initial = match load_connection() {
        Some(config) => ApiState {
            config: Some(config),
        },
        None => ApiState::new(),
    };
    let state = create_rw_signal(initial);
    provide_context(state);
    state
}
