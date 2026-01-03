//! Chat/conversation types.

/// A chat/conversation in the list.
#[derive(Debug, Clone)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub last_message: String,
    pub last_message_time: i64,
    pub unread_count: u32,
    pub is_online: bool,
}
