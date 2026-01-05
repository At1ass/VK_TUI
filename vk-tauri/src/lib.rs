//! VK Tauri - Tauri GUI client library.

pub mod commands;
pub mod state;

pub use commands::*;
pub use state::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("vk_tauri=debug,vk_core=debug,vk_api=debug")
        .init();

    let app_state = state::AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .setup(|app| {
            use tauri::{
                menu::{Menu, MenuItem},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                Manager, State,
            };

            // Create tray menu
            let show_item = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Create tray icon
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("VK Messenger")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // Store tray icon in app state
            let state: State<state::AppState> = app.state();
            let tray_clone = tray.clone();
            tauri::async_runtime::block_on(async move {
                *state.tray_icon.lock().await = Some(tray_clone);
            });

            // Handle window close event - minimize to tray instead of exit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        // Hide window instead of closing
                        #[cfg(target_os = "linux")]
                        {
                            let _ = window_clone.hide();
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            let _ = window_clone.minimize();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_auth_url,
            commands::login,
            commands::is_authenticated,
            commands::validate_session,
            commands::load_conversations,
            commands::load_messages,
            commands::load_messages_around,
            commands::load_messages_with_offset,
            commands::load_messages_with_start_message_id,
            commands::send_message,
            commands::send_reply,
            commands::send_forward,
            commands::edit_message,
            commands::delete_message,
            commands::fetch_message_by_id,
            commands::search_messages,
            commands::mark_as_read,
            commands::send_photo,
            commands::send_doc,
            commands::download_attachment,
            commands::logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
