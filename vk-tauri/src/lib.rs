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

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state::AppState::new())
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
