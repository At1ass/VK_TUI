//! Tauri commands callable from frontend.

use tauri::State;
use vk_api::auth::AuthManager;
use vk_core::{AsyncCommand, CoreEvent};

use crate::state::AppState;

/// Get VK OAuth URL.
#[tauri::command]
pub fn get_auth_url() -> String {
    AuthManager::get_auth_url()
}

/// Login with OAuth redirect URL.
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    redirect_url: String,
) -> Result<(), String> {
    let mut auth = state.auth.lock().await;

    auth.save_token_from_url(&redirect_url)
        .map_err(|e| format!("Failed to parse token: {}", e))?;

    let token = auth
        .access_token()
        .ok_or("Token not found")?
        .to_string();

    drop(auth); // Release lock before async call

    state.initialize_session(token).await?;

    Ok(())
}

/// Check if authenticated.
#[tauri::command]
pub async fn is_authenticated(state: State<'_, AppState>) -> Result<bool, String> {
    let auth = state.auth.lock().await;
    Ok(auth.is_authenticated() && !auth.is_token_expired())
}

/// Validate existing session on startup.
#[tauri::command]
pub async fn validate_session(state: State<'_, AppState>) -> Result<(), String> {
    let auth = state.auth.lock().await;

    if !auth.is_authenticated() {
        return Err("Not authenticated".to_string());
    }

    if auth.is_token_expired() {
        return Err("Token expired".to_string());
    }

    let token = auth.access_token().ok_or("No token")?.to_string();
    drop(auth);

    state.initialize_session(token).await
}

/// Load conversations.
#[tauri::command]
pub async fn load_conversations(
    state: State<'_, AppState>,
    offset: u32,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::LoadConversations { offset })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Load messages for a chat.
#[tauri::command]
pub async fn load_messages(
    state: State<'_, AppState>,
    peer_id: i64,
    offset: u32,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::LoadMessages { peer_id, offset })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Send a message.
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    peer_id: i64,
    text: String,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::SendMessage { peer_id, text })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Send a reply.
#[tauri::command]
pub async fn send_reply(
    state: State<'_, AppState>,
    peer_id: i64,
    reply_to: i64,
    text: String,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::SendReply { peer_id, reply_to, text })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Poll for events from vk-core.
/// Frontend should call this periodically.
#[tauri::command]
pub async fn poll_events(state: State<'_, AppState>) -> Result<Vec<CoreEvent>, String> {
    let mut rx = state.event_rx.lock().await;
    let mut events = Vec::new();

    if let Some(rx) = rx.as_mut() {
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
    }

    Ok(events)
}

/// Logout.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut auth = state.auth.lock().await;
    auth.logout().map_err(|e| e.to_string())?;

    *state.vk_client.lock().await = None;
    *state.command_tx.lock().await = None;
    *state.event_rx.lock().await = None;

    Ok(())
}
