use serde::{Deserialize, Serialize};

use super::attachment::Attachment;
use super::common::Peer;
use super::group::Group;
use super::misc::CanWrite;
use super::user::User;

/// Message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub id: i64,
    pub from_id: i64,
    pub peer_id: i64,
    pub date: i64,
    pub text: String,

    #[serde(default)]
    pub out: Option<i32>,

    #[serde(default)]
    pub read_state: Option<i32>,

    #[serde(default)]
    pub attachments: Vec<Attachment>,

    #[serde(default)]
    pub conversation_message_id: Option<i64>,

    #[serde(default)]
    pub fwd_messages: Vec<Message>,

    #[serde(default)]
    pub reply_message: Option<Box<Message>>,
}

impl Message {
    pub fn is_outgoing(&self) -> bool {
        self.out == Some(1)
    }

    pub fn is_read(&self) -> bool {
        self.read_state == Some(1)
    }
}

/// Conversation item from getConversations
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationItem {
    pub conversation: Conversation,
    pub last_message: Message,
}

/// Conversation info
#[derive(Debug, Clone, Deserialize)]
pub struct Conversation {
    pub peer: Peer,

    #[serde(default)]
    pub unread_count: Option<u32>,

    #[serde(default)]
    pub chat_settings: Option<ChatSettings>,

    #[serde(default)]
    pub can_write: Option<CanWrite>,
}

/// Chat settings for group chats
#[derive(Debug, Clone, Deserialize)]
pub struct ChatSettings {
    pub title: String,

    #[serde(default)]
    pub members_count: Option<i32>,

    #[serde(default)]
    pub photo: Option<ChatPhoto>,
}

/// Chat photo
#[derive(Debug, Clone, Deserialize)]
pub struct ChatPhoto {
    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,
}

/// Messages history response
#[derive(Debug, Deserialize)]
pub struct MessagesHistoryResponse {
    pub count: i32,
    pub items: Vec<Message>,

    #[serde(default)]
    pub profiles: Vec<User>,

    #[serde(default)]
    pub groups: Vec<Group>,
}

/// Conversations list response
#[derive(Debug, Deserialize)]
pub struct ConversationsResponse {
    pub count: i32,
    pub items: Vec<ConversationItem>,

    #[serde(default)]
    pub profiles: Vec<User>,

    #[serde(default)]
    pub groups: Vec<Group>,
}
