//! Tauri commands callable from frontend.

use tauri::{AppHandle, State};
use vk_api::auth::AuthManager;
use vk_core::AsyncCommand;

use crate::state::AppState;

/// Get VK OAuth URL.
#[tauri::command]
pub fn get_auth_url() -> String {
    AuthManager::get_auth_url()
}

/// Login with OAuth redirect URL.
#[tauri::command]
pub async fn login(
    app: AppHandle,
    state: State<'_, AppState>,
    redirect_url: String,
) -> Result<(), String> {
    state.login_from_redirect(app, &redirect_url).await
}

/// Check if authenticated.
#[tauri::command]
pub async fn is_authenticated(state: State<'_, AppState>) -> Result<bool, String> {
    let auth = state.auth.lock().await;
    Ok(auth.is_authenticated() && !auth.is_token_expired())
}

/// Validate existing session on startup.
#[tauri::command]
pub async fn validate_session(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let auth = state.auth.lock().await;

    if !auth.is_authenticated() {
        return Err("Not authenticated".to_string());
    }

    if auth.is_token_expired() {
        return Err("Token expired".to_string());
    }

    let token = auth.access_token().ok_or("No token")?.to_string();
    drop(auth);

    state.initialize_session(app, token).await
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

/// Load messages around a specific message.
#[tauri::command]
pub async fn load_messages_around(
    state: State<'_, AppState>,
    peer_id: i64,
    message_id: i64,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::LoadMessagesAround { peer_id, message_id })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Load messages with offset from a conversation message id.
#[tauri::command]
pub async fn load_messages_with_offset(
    state: State<'_, AppState>,
    peer_id: i64,
    start_cmid: i64,
    offset: i32,
    count: u32,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::LoadMessagesWithOffset {
            peer_id,
            start_cmid,
            offset,
            count,
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Load messages with offset from a message id.
#[tauri::command]
pub async fn load_messages_with_start_message_id(
    state: State<'_, AppState>,
    peer_id: i64,
    start_message_id: i64,
    offset: i32,
    count: u32,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::LoadMessagesWithStartMessageId {
            peer_id,
            start_message_id,
            offset,
            count,
        })
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

/// Forward messages with a comment.
#[tauri::command]
pub async fn send_forward(
    state: State<'_, AppState>,
    peer_id: i64,
    message_ids: Vec<i64>,
    comment: String,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::SendForward {
            peer_id,
            message_ids,
            comment,
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Edit a message.
#[tauri::command]
pub async fn edit_message(
    state: State<'_, AppState>,
    peer_id: i64,
    message_id: i64,
    cmid: Option<i64>,
    text: String,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::EditMessage {
            peer_id,
            message_id,
            cmid,
            text,
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Delete a message.
#[tauri::command]
pub async fn delete_message(
    state: State<'_, AppState>,
    peer_id: i64,
    message_id: i64,
    for_all: bool,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::DeleteMessage {
            peer_id,
            message_id,
            for_all,
        })
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Fetch message details (cmid, attachments, reply, forwards).
#[tauri::command]
pub async fn fetch_message_by_id(
    state: State<'_, AppState>,
    message_id: i64,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::FetchMessageById { message_id })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Search messages (global or within a chat).
#[tauri::command]
pub async fn search_messages(
    state: State<'_, AppState>,
    query: String,
    peer_id: Option<i64>,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::SearchMessages { query, peer_id })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Mark messages as read in a chat.
#[tauri::command]
pub async fn mark_as_read(
    state: State<'_, AppState>,
    peer_id: i64,
) -> Result<(), String> {
    let tx = state.command_tx.lock().await;
    if let Some(tx) = tx.as_ref() {
        tx.send(AsyncCommand::MarkAsRead { peer_id })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Logout.
#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let mut auth = state.auth.lock().await;
    auth.logout().map_err(|e| e.to_string())?;

    *state.vk_client.lock().await = None;
    *state.command_tx.lock().await = None;

    Ok(())
}
