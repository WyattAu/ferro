use wasm_bindgen::prelude::*;

pub mod api;
pub mod app;
pub mod components;
pub mod pages;
pub mod state;

pub use app::App;

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_crate_version() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_crate_name() {
        assert_eq!(env!("CARGO_PKG_NAME"), "ferro-admin");
    }

    #[test]
    fn test_api_state_default_not_connected() {
        let state = crate::api::ApiState::default();
        assert!(!state.is_connected());
    }

    #[test]
    fn test_api_state_connect_disconnect_cycle() {
        let mut state = crate::api::ApiState::new();
        assert!(!state.is_connected());
        state.connect("http://example.com".to_string(), "tok".to_string());
        assert!(state.is_connected());
        state.disconnect();
        assert!(!state.is_connected());
        state.connect("http://other.com".to_string(), "tok2".to_string());
        assert!(state.is_connected());
    }

    #[test]
    fn test_format_bytes_various_sizes() {
        assert_eq!(crate::state::format_bytes(0), "0 B");
        assert_eq!(crate::state::format_bytes(1), "1 B");
        assert!(crate::state::format_bytes(1024).contains("KB"));
        assert!(crate::state::format_bytes(1024 * 1024).contains("MB"));
        assert!(crate::state::format_bytes(1024 * 1024 * 1024).contains("GB"));
        assert!(crate::state::format_bytes(1024 * 1024 * 1024 * 1024).contains("TB"));
    }

    #[test]
    fn test_format_uptime_boundary_values() {
        assert_eq!(crate::state::format_uptime(0), "0m 0s");
        assert!(crate::state::format_uptime(86400).contains("1d"));
        assert!(crate::state::format_uptime(172800).contains("2d"));
    }

    #[test]
    fn test_format_timestamp_cases() {
        assert_eq!(crate::state::format_timestamp(""), "-");
        let formatted = crate::state::format_timestamp("2025-07-01T12:00:00Z");
        assert_eq!(formatted, "2025-07-01 12:00");
    }
}
