//! Events emitted by the core to frontends.
//!
//! These events represent state changes and async operation results
//! that frontends need to react to.

use crate::models::{AttachmentInfo, Chat, ChatMessage, ForwardItem, ReplyPreview, SearchResult};
use vk_api::User;
use serde::{Serialize, Deserialize};

/// Events from VK LongPoll API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VkEvent {
    /// New message received.
    NewMessage {
        message_id: i64,
        peer_id: i64,
        timestamp: i64,
        text: String,
        from_id: i64,
        is_outgoing: bool,
    },
    /// Message read.
    MessageRead { peer_id: i64, message_id: i64 },
    /// Message edited (from Long Poll).
    MessageEditedFromLongPoll { peer_id: i64, message_id: i64 },
    /// Message deleted (from Long Poll).
    MessageDeletedFromLongPoll { peer_id: i64, message_id: i64 },
    /// User typing.
    UserTyping { peer_id: i64, user_id: i64 },
    /// Connection status changed.
    ConnectionStatus(bool),
}

/// Events from core to frontends.
///
/// These events notify frontends about state changes and
/// results of async operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEvent {
    // === Data Loaded ===
    /// Conversations loaded from API.
    ConversationsLoaded {
        chats: Vec<Chat>,
        profiles: Vec<User>,
        total_count: u32,
        has_more: bool,
    },

    /// Messages loaded from API.
    MessagesLoaded {
        peer_id: i64,
        messages: Vec<ChatMessage>,
        profiles: Vec<User>,
        total_count: u32,
        has_more: bool,
    },

    /// Search results loaded.
    SearchResultsLoaded {
        results: Vec<SearchResult>,
        total_count: i32,
    },

    // === Message Actions ===
    /// Message sent successfully.
    MessageSent { message_id: i64, cmid: i64 },

    /// Message edited successfully.
    MessageEdited { message_id: i64 },

    /// Message deleted successfully.
    MessageDeleted { message_id: i64 },

    /// Message details fetched (for updating cmid, attachments, etc).
    MessageDetailsFetched {
        message_id: i64,
        cmid: Option<i64>,
        text: Option<String>,
        is_edited: bool,
        attachments: Option<Vec<AttachmentInfo>>,
        reply: Option<ReplyPreview>,
        fwd_count: Option<usize>,
        forwards: Option<Vec<ForwardItem>>,
    },

    // === Real-time Events ===
    /// VK LongPoll event.
    VkEvent(VkEvent),

    // === Errors ===
    /// Error occurred.
    Error(String),

    /// Send operation failed.
    SendFailed(String),
}
