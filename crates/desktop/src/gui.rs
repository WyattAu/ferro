use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use ferro_desktop::commands::DesktopState;
use ferro_desktop::config::DesktopConfig;
use ferro_desktop::tauri_commands::{
    cmd_default_mount_point, cmd_get_config, cmd_get_mount_progress,
    cmd_get_mount_status, cmd_mount, cmd_open_path, cmd_save_config,
    cmd_show_notification, cmd_unmount,
};

#[tauri::command]
fn get_server_url() -> String {
    "http://localhost:8080".to_string()
}

pub fn run() {
    let config = DesktopConfig::default();
    let state = DesktopState::new(config);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_server_url,
            cmd_mount,
            cmd_unmount,
            cmd_get_mount_status,
            cmd_get_config,
            cmd_save_config,
            cmd_get_mount_progress,
            cmd_open_path,
            cmd_show_notification,
            cmd_default_mount_point,
        ])
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Ferro — File Storage")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                window.set_title("Ferro")?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::get_server_url;

    #[test]
    fn test_server_url_command() {
        let url = get_server_url();
        assert!(url.starts_with("http"));
    }
}
