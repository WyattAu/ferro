pub mod commands;
pub mod config;
#[cfg(target_os = "macos")]
pub mod macos_integration;
pub mod mount;
pub mod overlay;
pub mod rclone;
pub mod shell_integration;
pub mod tauri_commands;
pub mod tray;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "mobile")]
pub mod mobile;

#[cfg(feature = "mobile")]
pub mod mobile_commands;

#[cfg(all(feature = "mobile", not(feature = "tauri")))]
mod mobile_app {
    use crate::commands::DesktopState;
    use crate::config::DesktopConfig;
    use tauri::State;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CliConnection {
        pub server_url: Option<String>,
        pub auth_token: Option<String>,
    }

    #[tauri::command]
    pub fn get_cli_connection(state: State<'_, CliConnection>) -> CliConnection {
        state.inner().clone()
    }

    #[tauri::command]
    pub fn get_server_url() -> String {
        "http://localhost:8080".to_string()
    }

    #[tauri::command]
    pub async fn cmd_get_config(
        state: State<'_, DesktopState>,
    ) -> Result<crate::commands::ConfigResponse, String> {
        Ok(state.get_config().await)
    }

    #[tauri::command]
    pub async fn cmd_save_config(
        state: State<'_, DesktopState>,
        request: crate::commands::SaveConfigRequest,
    ) -> Result<(), String> {
        state.save_config(request).await
    }

    #[tauri::command]
    pub async fn cmd_show_notification(
        title: String,
        body: String,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        use tauri_plugin_notification::NotificationExt;
        app_handle
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
            .map_err(|e| format!("Notification failed: {}", e))
    }

    pub fn build_app() -> Result<(), Box<dyn std::error::Error>> {
        let config = crate::config::load_config_from_disk().unwrap_or_default();
        let state = DesktopState::new(config);
        let cli_conn = CliConnection {
            server_url: None,
            auth_token: None,
        };

        tauri::Builder::default()
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_http::init())
            .manage(state)
            .manage(cli_conn)
            .invoke_handler(tauri::generate_handler![
                get_cli_connection,
                get_server_url,
                cmd_get_config,
                cmd_save_config,
                cmd_show_notification,
                crate::mobile_commands::mobile_get_file_thumbnail,
                crate::mobile_commands::mobile_get_storage_stats,
                crate::mobile_commands::mobile_start_background_sync,
                crate::mobile_commands::mobile_stop_background_sync,
                crate::mobile_commands::mobile_get_offline_files,
                crate::mobile_commands::mobile_pin_file_offline,
                crate::mobile_commands::mobile_unpin_file_offline,
                crate::mobile_commands::mobile_get_sync_status,
                crate::mobile_commands::mobile_resolve_conflict,
                crate::mobile_commands::mobile_share_file,
                crate::mobile_commands::mobile_monitor_connectivity,
                crate::mobile_commands::mobile_register_push_notifications,
            ])
            .setup(|app| {
                #[cfg(target_os = "android")]
                {
                    tracing::info!("Android platform initialized");
                }

                #[cfg(target_os = "ios")]
                {
                    tracing::info!("iOS platform initialized");
                }

                let _ = app;
                Ok(())
            })
            .run(tauri::generate_context!())?;

        Ok(())
    }
}

#[cfg(all(feature = "mobile", not(feature = "tauri")))]
pub fn run_mobile() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("Ferro Mobile starting");

    if let Err(e) = mobile_app::build_app() {
        tracing::error!("Fatal error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
