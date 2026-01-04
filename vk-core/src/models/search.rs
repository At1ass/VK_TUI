//! Search-related types.

use serde::{Serialize, Deserialize};

/// Search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub message_id: i64,
    pub peer_id: i64,
    pub from_id: i64,
    pub from_name: String,
    pub chat_title: String,
    pub text: String,
    pub timestamp: i64,
}
