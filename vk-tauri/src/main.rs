//! VK Tauri - main entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vk_tauri_lib::{AppState, commands};

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("vk_tauri=debug,vk_core=debug,vk_api=debug")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_auth_url,
            commands::login,
            commands::is_authenticated,
            commands::validate_session,
            commands::load_conversations,
            commands::load_messages,
            commands::send_message,
            commands::send_reply,
            commands::poll_events,
            commands::logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
