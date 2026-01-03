//! Commands that frontends can send to the core.
//!
//! Commands represent actions that require async processing,
//! typically VK API calls.

use std::path::PathBuf;

use crate::models::AttachmentInfo;

/// Synchronous commands (immediate state changes).
#[derive(Debug, Clone)]
pub enum Command {
    /// Select a chat by index.
    SelectChat(usize),
    /// Select a message by index.
    SelectMessage(usize),
    /// Clear current messages.
    ClearMessages,
    /// Update user cache.
    UpdateUsers(Vec<vk_api::User>),
}

/// Async commands that require API calls.
#[derive(Debug, Clone)]
pub enum AsyncCommand {
    // === Loading ===
    /// Load conversations list.
    LoadConversations { offset: u32 },

    /// Load messages for a chat.
    LoadMessages { peer_id: i64, offset: u32 },

    /// Load messages around a specific message.
    LoadMessagesAround { peer_id: i64, message_id: i64 },

    /// Load messages with offset from a specific message.
    /// Used for pagination with start_cmid.
    LoadMessagesWithOffset {
        peer_id: i64,
        start_cmid: i64,
        offset: i32,
        count: u32,
    },
    /// Load messages with offset from a specific message id.
    /// Used for pagination with start_message_id.
    LoadMessagesWithStartMessageId {
        peer_id: i64,
        start_message_id: i64,
        offset: i32,
        count: u32,
    },

    // === Messaging ===
    /// Send a text message.
    SendMessage { peer_id: i64, text: String },

    /// Send a message with reply.
    SendReply {
        peer_id: i64,
        reply_to: i64,
        text: String,
    },

    /// Forward messages.
    SendForward {
        peer_id: i64,
        message_ids: Vec<i64>,
        comment: String,
    },

    /// Edit a message.
    EditMessage {
        peer_id: i64,
        message_id: i64,
        cmid: Option<i64>,
        text: String,
    },

    /// Delete a message.
    DeleteMessage {
        peer_id: i64,
        message_id: i64,
        for_all: bool,
    },

    // === Attachments ===
    /// Send a photo.
    SendPhoto { peer_id: i64, path: PathBuf },

    /// Send a document.
    SendDoc { peer_id: i64, path: PathBuf },

    /// Download attachments.
    DownloadAttachments { attachments: Vec<AttachmentInfo> },

    // === Search ===
    /// Search messages globally.
    SearchMessages { query: String, peer_id: Option<i64> },

    // === Other ===
    /// Start LongPoll listener.
    StartLongPoll,

    /// Mark messages as read.
    MarkAsRead { peer_id: i64 },

    /// Fetch message details by ID.
    FetchMessageById { message_id: i64 },
}
