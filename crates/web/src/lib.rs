pub mod api;
pub mod api_cache;
pub mod app;
pub mod auth;
pub mod components;
pub mod hooks;
pub mod i18n;
pub mod pages;
pub mod styles;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(app::App);
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
        assert_eq!(env!("CARGO_PKG_NAME"), "web");
    }

    #[test]
    fn test_auth_info_default() {
        let info = crate::auth::UserInfo::default();
        assert!(info.sub.is_empty());
        assert!(info.email.is_none());
        assert!(info.name.is_none());
    }

    #[test]
    fn test_auth_info_serde_roundtrip() {
        let info = crate::auth::UserInfo {
            sub: "user123".to_string(),
            email: Some("user@example.com".to_string()),
            name: Some("Test User".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: crate::auth::UserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sub, "user123");
        assert_eq!(parsed.email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn test_auth_info_deserialize_missing_optional_fields() {
        let json = r#"{"sub":"u1"}"#;
        let parsed: crate::auth::UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.sub, "u1");
        assert!(parsed.email.is_none());
        assert!(parsed.name.is_none());
    }

    #[test]
    fn test_get_auth_header_non_wasm() {
        let header = crate::auth::get_auth_header();
        assert!(header.is_none());
    }

    #[tokio::test]
    async fn test_list_files_non_wasm() {
        let result = crate::api::list_files("/").await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.current_path, "/");
        assert!(resp.entries.is_empty());
    }

    #[tokio::test]
    async fn test_upload_file_non_wasm() {
        let result = crate::api::upload_file("/test.txt", b"hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_file_non_wasm() {
        let result = crate::api::delete_file("/test.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_directory_non_wasm() {
        let result = crate::api::create_directory("/newdir").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_auth_config_non_wasm() {
        let result = crate::api::get_auth_config().await;
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(!config.configured);
        assert!(config.login_url.is_none());
    }

    #[tokio::test]
    async fn test_search_files_non_wasm() {
        let result = crate::api::search_files("test", None).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.query, "test");
        assert!(resp.results.is_empty());
    }

    #[tokio::test]
    async fn test_download_file_non_wasm() {
        let result = crate::api::download_file("/test.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_preferences_non_wasm() {
        let result = crate::api::get_preferences().await;
        assert!(result.is_ok());
        let prefs = result.unwrap();
        assert_eq!(prefs.theme, "dark");
        assert_eq!(prefs.items_per_page, 50);
    }

    #[tokio::test]
    async fn test_get_quota_non_wasm() {
        let result = crate::api::get_quota().await;
        assert!(result.is_ok());
        let quota = result.unwrap();
        assert!(quota.unlimited);
    }

    #[tokio::test]
    async fn test_bulk_delete_non_wasm() {
        let result = crate::api::bulk_delete(&[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_favorites_non_wasm() {
        let result = crate::api::list_favorites().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_trash_non_wasm() {
        let result = crate::api::list_trash().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_recent_files_non_wasm() {
        let result = crate::api::list_recent_files().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_locks_non_wasm() {
        let result = crate::api::list_locks().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_request_notification_permission_non_wasm() {
        crate::api::request_notification_permission();
    }

    #[test]
    fn test_show_notification_non_wasm() {
        crate::api::show_notification("title", "body");
    }
}
