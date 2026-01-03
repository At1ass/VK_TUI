//! Message types.

use super::AttachmentInfo;

/// Delivery state for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Failed,
}

/// Preview of a reply message.
#[derive(Debug, Clone)]
pub struct ReplyPreview {
    pub from: String,
    pub text: String,
    pub attachments: Vec<AttachmentInfo>,
}

/// A forwarded message item (can be nested).
#[derive(Debug, Clone)]
pub struct ForwardItem {
    pub from: String,
    pub text: String,
    pub attachments: Vec<AttachmentInfo>,
    pub nested: Vec<ForwardItem>,
}

/// A single message in a conversation.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub cmid: Option<i64>,
    pub from_id: i64,
    pub from_name: String,
    pub text: String,
    pub timestamp: i64,
    pub is_outgoing: bool,
    pub is_read: bool,
    pub is_edited: bool,
    pub is_pinned: bool,
    pub delivery: DeliveryStatus,
    pub attachments: Vec<AttachmentInfo>,
    pub reply: Option<ReplyPreview>,
    pub fwd_count: usize,
    pub forwards: Vec<ForwardItem>,
}

impl ChatMessage {
    /// Get the peer_id from the message (for forwarded messages context).
    pub fn peer_id(&self) -> i64 {
        // For outgoing messages, peer_id is not from_id
        // This is a simplified version; actual peer_id should be passed from context
        self.from_id
    }
}
