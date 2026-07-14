use leptos::prelude::*;

use crate::api::{AdminConnectionConfig, ApiState};

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

pub fn save_connection(config: &AdminConnectionConfig) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let _ = storage.set_item("ferro_admin_url", &config.url);
        let _ = storage.set_item("ferro_admin_token", &config.token);
    }
}

pub fn load_connection() -> Option<AdminConnectionConfig> {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
    {
        let url = storage.get_item("ferro_admin_url").ok()??;
        let token = storage.get_item("ferro_admin_token").ok()??;
        if !url.is_empty() && !token.is_empty() {
            return Some(AdminConnectionConfig { url, token });
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
        Some(config) => ApiState { config: Some(config) },
        None => ApiState::new(),
    };
    let state = RwSignal::new(initial);
    provide_context(state);
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_zero() {
        assert_eq!(format_uptime(0), "0m 0s");
    }

    #[test]
    fn test_format_uptime_seconds() {
        assert_eq!(format_uptime(45), "0m 45s");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(90), "1m 30s");
        assert_eq!(format_uptime(3600 - 1), "59m 59s");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1h 0m 0s");
        assert_eq!(format_uptime(3661), "1h 1m 1s");
        assert_eq!(format_uptime(86399), "23h 59m 59s");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(86400), "1d 0h 0m");
        assert_eq!(format_uptime(90061), "1d 1h 1m");
        assert_eq!(format_uptime(172800), "2d 0h 0m");
    }

    #[test]
    fn test_format_timestamp_empty() {
        assert_eq!(format_timestamp(""), "-");
    }

    #[test]
    fn test_format_timestamp_valid_rfc3339() {
        let ts = "2025-01-15T14:30:00Z";
        let formatted = format_timestamp(ts);
        assert_eq!(formatted, "2025-01-15 14:30");
    }

    #[test]
    fn test_format_timestamp_with_timezone() {
        let ts = "2025-06-01T08:00:00+05:30";
        let formatted = format_timestamp(ts);
        assert!(formatted.contains("2025-06-01"));
    }

    #[test]
    fn test_format_timestamp_invalid() {
        let ts = "not a valid timestamp";
        assert_eq!(format_timestamp(ts), ts);
    }

    #[test]
    fn test_format_timestamp_already_short() {
        let ts = "2025-01-01 00:00";
        assert_eq!(format_timestamp(ts), ts);
    }
}
