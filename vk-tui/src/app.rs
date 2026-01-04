use std::sync::Arc;
use tokio::sync::mpsc;

use crate::state::{App, AsyncAction, Chat, ChatMessage, RunningState, Screen};
use vk_api::VkClient;
use vk_api::auth::AuthManager;

impl App {
    /// Create new application state
    pub fn new() -> Self {
        let mut app = Self::default();

        // Restore token if present
        if app.auth.is_authenticated()
            && let Some(token) = app.auth.access_token()
        {
            if app.auth.is_token_expired() {
                let _ = app.auth.logout();
                app.screen = Screen::Auth;
                app.status = Some("Session expired. Please authorize again.".into());
            } else {
                app.vk_client = Some(Arc::new(VkClient::new(token.to_string())));
                app.screen = Screen::Main;
                app.status = Some("Restoring session...".into());
            }
        }

        app
    }

    /// Get auth URL for display
    pub fn auth_url(&self) -> String {
        AuthManager::get_auth_url()
    }

    /// Check if app should quit
    pub fn is_running(&self) -> bool {
        self.running_state == RunningState::Running
    }

    /// Set action sender
    pub fn set_action_tx(&mut self, tx: mpsc::UnboundedSender<AsyncAction>) {
        self.action_tx = Some(tx);
    }

    /// Send async action
    pub fn send_action(&self, action: AsyncAction) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Get current chat peer_id
    pub fn current_chat(&self) -> Option<&Chat> {
        if let Some(filter) = &self.chat_filter {
            // Get the actual chat index from filtered indices
            filter
                .filtered_indices
                .get(self.selected_chat)
                .and_then(|&idx| self.chats.get(idx))
        } else {
            self.chats.get(self.selected_chat)
        }
    }

    /// Get user name by id
    pub fn get_user_name(&self, user_id: i64) -> String {
        if let Some(user) = self.users.get(&user_id) {
            user.full_name()
        } else if user_id < 0 {
            format!("Group {}", -user_id)
        } else {
            format!("User {}", user_id)
        }
    }

    /// Get currently highlighted message
    pub fn current_message(&self) -> Option<&ChatMessage> {
        self.messages.get(self.messages_scroll)
    }
}
