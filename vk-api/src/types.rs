use serde::{Deserialize, Serialize};

/// VK API response wrapper
#[derive(Debug, Deserialize)]
pub struct VkResponse<T> {
    pub response: Option<T>,
    pub error: Option<VkError>,
}

/// VK API error
#[derive(Debug, Deserialize)]
pub struct VkError {
    pub error_code: i32,
    pub error_msg: String,
}

/// User info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub photo_50: Option<String>,
    #[serde(default)]
    pub online: Option<i32>,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_online(&self) -> bool {
        self.online == Some(1)
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
}

/// Peer info
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Peer {
    pub id: i64,
    #[serde(rename = "type")]
    pub peer_type: String,
    #[serde(default)]
    pub local_id: Option<i64>,
}

/// Chat settings for group chats
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChatSettings {
    pub title: String,
    #[serde(default)]
    pub members_count: Option<i32>,
    #[serde(default)]
    pub photo: Option<ChatPhoto>,
}

/// Chat photo
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChatPhoto {
    #[serde(default)]
    pub photo_50: Option<String>,
    #[serde(default)]
    pub photo_100: Option<String>,
}

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
}

impl Message {
    pub fn is_outgoing(&self) -> bool {
        self.out == Some(1)
    }

    pub fn is_read(&self) -> bool {
        self.read_state == Some(1)
    }
}

/// Message attachment
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub attachment_type: String,
    #[serde(default)]
    pub photo: Option<Photo>,
    #[serde(default)]
    pub doc: Option<Doc>,
}

/// Photo attachment
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Photo {
    #[serde(default)]
    pub sizes: Vec<PhotoSize>,
}

/// Photo size info
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PhotoSize {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// Document attachment
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Doc {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default, rename = "ext")]
    pub extension: Option<String>,
}

/// Messages history response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessagesHistoryResponse {
    pub count: i32,
    pub items: Vec<Message>,
    #[serde(default)]
    pub profiles: Vec<User>,
}

/// Conversations list response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ConversationsResponse {
    pub count: i32,
    pub items: Vec<ConversationItem>,
    #[serde(default)]
    pub profiles: Vec<User>,
}

/// Long Poll server info
#[derive(Debug, Clone, Deserialize)]
pub struct LongPollServer {
    pub key: String,
    pub server: String,
    #[serde(deserialize_with = "deserialize_ts")]
    pub ts: String,
}

/// Long Poll response
#[derive(Debug, Deserialize)]
pub struct LongPollResponse {
    #[serde(default, deserialize_with = "deserialize_ts_option")]
    pub ts: Option<String>,
    pub updates: Option<Vec<serde_json::Value>>,
    pub failed: Option<i32>,
}

/// Upload server info for photos
#[derive(Debug, Deserialize)]
pub struct UploadServer {
    pub upload_url: String,
}

/// Photo upload response
#[derive(Debug, Deserialize)]
pub struct PhotoUploadResponse {
    pub server: i64,
    pub photo: String,
    pub hash: String,
}

/// Saved photo info
#[derive(Debug, Deserialize)]
pub struct SavedPhoto {
    pub id: i64,
    pub owner_id: i64,
}

/// Document upload response
#[derive(Debug, Deserialize)]
pub struct DocUploadResponse {
    pub file: String,
}

/// Saved document info
#[derive(Debug, Deserialize)]
pub struct SavedDoc {
    pub id: i64,
    pub owner_id: i64,
}

/// Deserialize ts which can be either string or number
fn deserialize_ts<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(D::Error::custom("expected string or number for ts")),
    }
}

/// Deserialize optional ts
fn deserialize_ts_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = serde::Deserialize::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}
