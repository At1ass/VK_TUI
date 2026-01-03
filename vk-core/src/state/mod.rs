//! Core application state types.
//!
//! Contains UI-agnostic state that can be shared between frontends.

use std::collections::HashMap;
use std::sync::Arc;

use vk_api::auth::AuthManager;
use vk_api::{User, VkClient};

use crate::models::{Chat, ChatMessage, SearchResult};

/// Pagination state for messages in a specific chat.
#[derive(Debug, Clone)]
pub struct MessagesPagination {
    pub peer_id: i64,
    pub offset: u32,
    pub total_count: Option<u32>,
    pub is_loading: bool,
    pub has_more: bool,
    /// First (oldest loaded) conversation_message_id.
    pub first_cmid: Option<i64>,
    /// Last (newest loaded) conversation_message_id.
    pub last_cmid: Option<i64>,
}

impl MessagesPagination {
    pub fn new(peer_id: i64) -> Self {
        Self {
            peer_id,
            offset: 0,
            total_count: None,
            is_loading: false,
            has_more: true,
            first_cmid: None,
            last_cmid: None,
        }
    }
}

/// Pagination state for chat list.
#[derive(Debug, Clone, Default)]
pub struct ChatsPagination {
    pub offset: u32,
    pub total_count: Option<u32>,
    pub is_loading: bool,
    pub has_more: bool,
}

/// Core application state - shared between frontends.
///
/// This struct contains all business data that is independent
/// of the specific UI framework being used.
#[derive(Default)]
pub struct CoreState {
    // Auth
    pub auth: AuthManager,
    pub vk_client: Option<Arc<VkClient>>,

    // User data
    pub users: HashMap<i64, User>,
    pub current_user: Option<User>,

    // Chat data
    pub chats: Vec<Chat>,
    pub selected_chat: usize,
    pub current_peer_id: Option<i64>,

    // Messages
    pub messages: Vec<ChatMessage>,
    pub selected_message: usize,
    pub target_message_id: Option<i64>,

    // Pagination
    pub chats_pagination: ChatsPagination,
    pub messages_pagination: Option<MessagesPagination>,

    // Search
    pub search_results: Vec<SearchResult>,
    pub search_total: i32,
}

impl CoreState {
    /// Create new core state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize with authenticated client.
    pub fn with_client(client: Arc<VkClient>) -> Self {
        Self {
            vk_client: Some(client),
            ..Default::default()
        }
    }

    /// Get current chat if selected.
    pub fn current_chat(&self) -> Option<&Chat> {
        self.chats.get(self.selected_chat)
    }

    /// Get current message if selected.
    pub fn current_message(&self) -> Option<&ChatMessage> {
        self.messages.get(self.selected_message)
    }

    /// Get user name by id.
    pub fn get_user_name(&self, user_id: i64) -> String {
        if let Some(user) = self.users.get(&user_id) {
            user.full_name()
        } else if user_id < 0 {
            format!("Group {}", -user_id)
        } else {
            format!("User {}", user_id)
        }
    }

    /// Check if authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.vk_client.is_some()
    }
}
