# VK API Client - Design Document

**Version**: 0.1.0
**VK API Version**: 5.199
**Last Updated**: 2025-12-16

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Module Structure](#module-structure)
3. [API Methods Specification](#api-methods-specification)
4. [Types Specification](#types-specification)
5. [Error Handling](#error-handling)
6. [Testing Strategy](#testing-strategy)

---

## Architecture Overview

### Design Principles

1. **Modular**: Each VK API section (messages, users, friends) is a separate module
2. **Type-safe**: Strong typing for all VK API objects
3. **Async-first**: All methods are async/await
4. **Ergonomic**: Builder patterns where appropriate
5. **Testable**: Easy to mock and test

### Client Structure

```rust
pub struct VkClient {
    client: reqwest::Client,
    access_token: String,
}

impl VkClient {
    // Core request methods
    async fn request<T>(&self, method: &str, params: HashMap<&str, String>) -> Result<T>
    async fn request_with_files(&self, method: &str, params: FormData) -> Result<T>

    // Namespace methods (returns method builders)
    pub fn messages(&self) -> MessagesApi<'_>
    pub fn users(&self) -> UsersApi<'_>
    pub fn friends(&self) -> FriendsApi<'_>
    pub fn groups(&self) -> GroupsApi<'_>
    pub fn photos(&self) -> PhotosApi<'_>
    pub fn docs(&self) -> DocsApi<'_>
    pub fn account(&self) -> AccountApi<'_>
    pub fn longpoll(&self) -> LongPollApi<'_>
}
```

---

## Module Structure

```
src/
├── lib.rs                  # Public API, re-exports
├── client.rs               # VkClient core
├── auth.rs                 # OAuth & token management
│
├── types/                  # VK API type definitions
│   ├── mod.rs
│   ├── common.rs           # VkResponse, VkError, Peer
│   ├── user.rs             # User, UserFull, LastSeen
│   ├── message.rs          # Message, Conversation, ConversationItem
│   ├── attachment.rs       # Attachment, Photo, Doc, PhotoSize
│   ├── longpoll.rs         # LongPollServer, LongPollResponse
│   ├── group.rs            # Group, GroupFull
│   └── misc.rs             # City, Country, Counters, etc.
│
├── methods/                # API method implementations
│   ├── mod.rs
│   ├── messages.rs         # MessagesApi
│   ├── users.rs            # UsersApi
│   ├── friends.rs          # FriendsApi
│   ├── groups.rs           # GroupsApi
│   ├── photos.rs           # PhotosApi
│   ├── docs.rs             # DocsApi
│   ├── account.rs          # AccountApi
│   └── longpoll.rs         # LongPollApi
│
└── utils/
    ├── mod.rs
    ├── multipart.rs        # Multipart form builder
    └── pagination.rs       # Pagination helpers
```

---

## API Methods Specification

### 1. Messages API

```rust
pub struct MessagesApi<'a> {
    client: &'a VkClient,
}

impl<'a> MessagesApi<'a> {
    // ========== Conversations ==========

    /// Get list of conversations
    ///
    /// # Arguments
    /// * `offset` - Offset for pagination (default: 0)
    /// * `count` - Number of conversations to return (max: 200, default: 20)
    /// * `extended` - Return extended info with profiles (default: true)
    ///
    /// # Returns
    /// ConversationsResponse with items and profiles
    ///
    /// # VK API
    /// Method: messages.getConversations
    /// https://dev.vk.com/method/messages.getConversations
    pub async fn get_conversations(
        &self,
        offset: u32,
        count: u32,
    ) -> Result<ConversationsResponse>;

    /// Get conversation by peer_id
    pub async fn get_conversation_by_id(
        &self,
        peer_id: i64,
    ) -> Result<Conversation>;

    // ========== Messages ==========

    /// Get message history for a conversation
    ///
    /// # Arguments
    /// * `peer_id` - Peer ID (user_id for DM, 2000000000+chat_id for group chats)
    /// * `offset` - Offset for pagination
    /// * `count` - Number of messages (max: 200)
    ///
    /// # Returns
    /// MessagesHistoryResponse with messages and profiles
    ///
    /// # VK API
    /// Method: messages.getHistory
    pub async fn get_history(
        &self,
        peer_id: i64,
        offset: u32,
        count: u32,
    ) -> Result<MessagesHistoryResponse>;

    /// Get message by conversation_message_id
    pub async fn get_by_conversation_message_id(
        &self,
        peer_id: i64,
        conversation_message_ids: &[i64],
    ) -> Result<Vec<Message>>;

    // ========== Send Messages ==========

    /// Send text message
    ///
    /// # Arguments
    /// * `peer_id` - Recipient peer ID
    /// * `message` - Message text (max: 4096 chars)
    ///
    /// # Returns
    /// Message ID
    ///
    /// # VK API
    /// Method: messages.send
    pub async fn send(
        &self,
        peer_id: i64,
        message: &str,
    ) -> Result<i64>;

    /// Send message with reply
    pub async fn send_with_reply(
        &self,
        peer_id: i64,
        message: &str,
        reply_to: i64,
    ) -> Result<i64>;

    /// Send message with forward
    pub async fn send_with_forward(
        &self,
        peer_id: i64,
        message: &str,
        forward_messages: &[i64],
    ) -> Result<i64>;

    /// Send message with attachment
    pub async fn send_with_attachment(
        &self,
        peer_id: i64,
        message: &str,
        attachment: &str,
    ) -> Result<i64>;

    // ========== Edit/Delete ==========

    /// Edit message
    ///
    /// # Arguments
    /// * `peer_id` - Peer ID
    /// * `message_id` - Message ID to edit
    /// * `message` - New message text
    ///
    /// # VK API
    /// Method: messages.edit
    pub async fn edit(
        &self,
        peer_id: i64,
        message_id: i64,
        message: &str,
    ) -> Result<()>;

    /// Delete messages
    ///
    /// # Arguments
    /// * `message_ids` - IDs of messages to delete
    /// * `delete_for_all` - Delete for all participants (only for own messages)
    ///
    /// # VK API
    /// Method: messages.delete
    pub async fn delete(
        &self,
        message_ids: &[i64],
        delete_for_all: bool,
    ) -> Result<()>;

    // ========== Search ==========

    /// Search messages
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `peer_id` - Search in specific conversation (None for global search)
    /// * `count` - Number of results (max: 100)
    ///
    /// # VK API
    /// Method: messages.search
    pub async fn search(
        &self,
        query: &str,
        peer_id: Option<i64>,
        count: u32,
    ) -> Result<Vec<Message>>;

    // ========== Pin/Unpin ==========

    /// Pin message in conversation
    pub async fn pin(
        &self,
        peer_id: i64,
        message_id: i64,
    ) -> Result<()>;

    /// Unpin message in conversation
    pub async fn unpin(
        &self,
        peer_id: i64,
    ) -> Result<()>;

    // ========== Read Status ==========

    /// Mark messages as read
    pub async fn mark_as_read(
        &self,
        peer_id: i64,
    ) -> Result<i32>;

    // ========== Activity ==========

    /// Set typing/recording activity
    ///
    /// # VK API
    /// Method: messages.setActivity
    pub async fn set_activity(
        &self,
        peer_id: i64,
        activity_type: ActivityType,
    ) -> Result<()>;

    // ========== Reactions ==========

    /// Send reaction to message
    pub async fn send_reaction(
        &self,
        peer_id: i64,
        cmid: i64,
        reaction_id: i64,
    ) -> Result<()>;

    /// Get available reaction assets
    pub async fn get_reactions_assets(&self) -> Result<Vec<Reaction>>;
}

/// Activity types for setActivity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    Typing,
    AudioMessage,
}

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::Typing => "typing",
            ActivityType::AudioMessage => "audiomessage",
        }
    }
}
```

### 2. Users API

```rust
pub struct UsersApi<'a> {
    client: &'a VkClient,
}

impl<'a> UsersApi<'a> {
    /// Get user info by IDs
    ///
    /// # Arguments
    /// * `user_ids` - User IDs (max: 1000)
    ///
    /// # VK API
    /// Method: users.get
    pub async fn get(
        &self,
        user_ids: &[i64],
    ) -> Result<Vec<User>>;

    /// Search users
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `count` - Number of results (max: 1000, default: 20)
    /// * `fields` - Additional fields (photo_50, online, etc.)
    ///
    /// # VK API
    /// Method: users.search
    pub async fn search(
        &self,
        query: &str,
        count: u32,
    ) -> Result<Vec<User>>;

    /// Get user subscriptions (communities and users)
    pub async fn get_subscriptions(
        &self,
        user_id: i64,
    ) -> Result<Subscriptions>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscriptions {
    pub users: Vec<i64>,
    pub groups: Vec<i64>,
}
```

### 3. Friends API

```rust
pub struct FriendsApi<'a> {
    client: &'a VkClient,
}

impl<'a> FriendsApi<'a> {
    /// Get friends list
    ///
    /// # Arguments
    /// * `user_id` - User ID (None for current user)
    /// * `order` - Sort order ("hints", "random", "name")
    ///
    /// # VK API
    /// Method: friends.get
    pub async fn get(
        &self,
        user_id: Option<i64>,
    ) -> Result<Vec<User>>;

    /// Get online friends
    pub async fn get_online(
        &self,
    ) -> Result<Vec<i64>>;

    /// Search in friends
    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Vec<User>>;

    /// Get recently added friends
    pub async fn get_recent(
        &self,
        count: u32,
    ) -> Result<Vec<User>>;
}
```

### 4. Groups API

```rust
pub struct GroupsApi<'a> {
    client: &'a VkClient,
}

impl<'a> GroupsApi<'a> {
    /// Get user's groups
    ///
    /// # VK API
    /// Method: groups.get
    pub async fn get(
        &self,
        user_id: Option<i64>,
    ) -> Result<Vec<Group>>;

    /// Get groups by IDs
    pub async fn get_by_id(
        &self,
        group_ids: &[i64],
    ) -> Result<Vec<Group>>;

    /// Search groups
    pub async fn search(
        &self,
        query: &str,
        count: u32,
    ) -> Result<Vec<Group>>;
}
```

### 5. Photos API

```rust
pub struct PhotosApi<'a> {
    client: &'a VkClient,
}

impl<'a> PhotosApi<'a> {
    /// Get upload server for message photo
    pub async fn get_messages_upload_server(
        &self,
        peer_id: i64,
    ) -> Result<UploadServer>;

    /// Save uploaded photo
    pub async fn save_messages_photo(
        &self,
        upload_response: UploadResponse,
    ) -> Result<Vec<SavedPhoto>>;

    /// Send photo to peer (combines upload + save + send)
    pub async fn send_to_peer(
        &self,
        peer_id: i64,
        photo_path: &Path,
    ) -> Result<i64>;
}
```

### 6. Docs API

```rust
pub struct DocsApi<'a> {
    client: &'a VkClient,
}

impl<'a> DocsApi<'a> {
    /// Get upload server for message document
    pub async fn get_messages_upload_server(
        &self,
        peer_id: i64,
    ) -> Result<UploadServer>;

    /// Save uploaded document
    pub async fn save(
        &self,
        upload_response: UploadResponse,
    ) -> Result<Vec<SavedDoc>>;

    /// Send document to peer (combines upload + save + send)
    pub async fn send_to_peer(
        &self,
        peer_id: i64,
        doc_path: &Path,
    ) -> Result<i64>;
}
```

### 7. Account API

```rust
pub struct AccountApi<'a> {
    client: &'a VkClient,
}

impl<'a> AccountApi<'a> {
    /// Get counters (messages, friends, notifications, etc.)
    pub async fn get_counters(&self) -> Result<Counters>;

    /// Get profile info
    pub async fn get_profile_info(&self) -> Result<ProfileInfo>;

    /// Set online status
    pub async fn set_online(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counters {
    pub messages: Option<u32>,
    pub friends: Option<u32>,
    pub notifications: Option<u32>,
    pub groups: Option<u32>,
}
```

### 8. Long Poll API

```rust
pub struct LongPollApi<'a> {
    client: &'a VkClient,
}

impl<'a> LongPollApi<'a> {
    /// Get Long Poll server info
    pub async fn get_server(&self) -> Result<LongPollServer>;

    /// Poll for updates
    pub async fn poll(
        &self,
        server: &LongPollServer,
    ) -> Result<LongPollResponse>;

    /// Get history of missed events
    pub async fn get_history(
        &self,
        ts: &str,
        pts: Option<i64>,
    ) -> Result<LongPollHistory>;
}
```

---

## Types Specification

### Common Types

```rust
// types/common.rs

#[derive(Debug, Deserialize)]
pub struct VkResponse<T> {
    pub response: Option<T>,
    pub error: Option<VkError>,
}

#[derive(Debug, Deserialize)]
pub struct VkError {
    pub error_code: i32,
    pub error_msg: String,
    pub request_params: Option<Vec<RequestParam>>,
}

#[derive(Debug, Deserialize)]
pub struct RequestParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Peer {
    pub id: i64,
    #[serde(rename = "type")]
    pub peer_type: String,
    #[serde(default)]
    pub local_id: Option<i64>,
}
```

### User Types

```rust
// types/user.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,

    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,

    #[serde(default)]
    pub online: Option<i32>,

    #[serde(default)]
    pub last_seen: Option<LastSeen>,

    #[serde(default)]
    pub screen_name: Option<String>,

    #[serde(default)]
    pub verified: Option<bool>,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_online(&self) -> bool {
        self.online == Some(1)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LastSeen {
    pub time: i64,
    pub platform: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserFull {
    #[serde(flatten)]
    pub base: User,

    #[serde(default)]
    pub status: Option<String>,

    #[serde(default)]
    pub city: Option<City>,

    #[serde(default)]
    pub country: Option<Country>,

    #[serde(default)]
    pub home_town: Option<String>,
}
```

### Message Types

```rust
// types/message.rs

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
    pub fwd_messages: Vec<Message>,

    #[serde(default)]
    pub reply_message: Option<Box<Message>>,

    #[serde(default)]
    pub conversation_message_id: Option<i64>,

    #[serde(default)]
    pub update_time: Option<i64>,
}

impl Message {
    pub fn is_outgoing(&self) -> bool {
        self.out == Some(1)
    }

    pub fn is_read(&self) -> bool {
        self.read_state == Some(1)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationItem {
    pub conversation: Conversation,
    pub last_message: Message,
}

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

#[derive(Debug, Clone, Deserialize)]
pub struct ChatSettings {
    pub title: String,

    #[serde(default)]
    pub members_count: Option<i32>,

    #[serde(default)]
    pub photo: Option<ChatPhoto>,

    #[serde(default)]
    pub pinned_message: Option<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatPhoto {
    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,

    #[serde(default)]
    pub photo_200: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanWrite {
    pub allowed: bool,

    #[serde(default)]
    pub reason: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ConversationsResponse {
    pub count: i32,
    pub items: Vec<ConversationItem>,

    #[serde(default)]
    pub profiles: Vec<User>,

    #[serde(default)]
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct MessagesHistoryResponse {
    pub count: i32,
    pub items: Vec<Message>,

    #[serde(default)]
    pub profiles: Vec<User>,

    #[serde(default)]
    pub groups: Vec<Group>,
}
```

### Attachment Types

```rust
// types/attachment.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub attachment_type: String,

    #[serde(default)]
    pub photo: Option<Photo>,

    #[serde(default)]
    pub doc: Option<Doc>,

    #[serde(default)]
    pub audio: Option<Audio>,

    #[serde(default)]
    pub video: Option<Video>,

    #[serde(default)]
    pub link: Option<Link>,

    #[serde(default)]
    pub sticker: Option<Sticker>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Photo {
    pub id: i64,
    pub owner_id: i64,

    #[serde(default)]
    pub sizes: Vec<PhotoSize>,

    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PhotoSize {
    #[serde(rename = "type")]
    pub size_type: String,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub width: Option<u32>,

    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Doc {
    pub id: i64,
    pub owner_id: i64,

    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub size: Option<u64>,

    #[serde(default, rename = "ext")]
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Audio {
    pub id: i64,
    pub owner_id: i64,
    pub artist: String,
    pub title: String,
    pub duration: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Video {
    pub id: i64,
    pub owner_id: i64,
    pub title: String,
    pub duration: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Link {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sticker {
    pub sticker_id: i64,
    pub product_id: i64,
}
```

### Group Types

```rust
// types/group.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub screen_name: String,

    #[serde(default)]
    pub photo_50: Option<String>,

    #[serde(default)]
    pub photo_100: Option<String>,

    #[serde(default)]
    pub photo_200: Option<String>,

    #[serde(default)]
    pub is_closed: Option<i32>,

    #[serde(default)]
    pub verified: Option<bool>,
}
```

### Misc Types

```rust
// types/misc.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct City {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Country {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reaction {
    pub reaction_id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileInfo {
    pub first_name: String,
    pub last_name: String,
    pub screen_name: Option<String>,
    pub status: Option<String>,
}
```

---

## Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VkApiError {
    #[error("VK API error {code}: {message}")]
    ApiError { code: i32, message: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

pub type Result<T> = std::result::Result<T, VkApiError>;
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_full_name() {
        let user = User {
            id: 1,
            first_name: "John".into(),
            last_name: "Doe".into(),
            ..Default::default()
        };
        assert_eq!(user.full_name(), "John Doe");
    }
}
```

### Integration Tests

```rust
// tests/messages_test.rs
use vk_api::VkClient;

fn get_test_client() -> VkClient {
    let token = std::env::var("VK_TEST_TOKEN").expect("VK_TEST_TOKEN required");
    VkClient::new(token)
}

#[tokio::test]
async fn test_get_conversations() {
    let client = get_test_client();
    let result = client.messages().get_conversations(0, 10).await;
    assert!(result.is_ok());
}
```

### curl Examples

```bash
# examples/curl/messages_search.sh
#!/bin/bash
curl -X POST "https://api.vk.com/method/messages.search" \
  -d "q=${1}" \
  -d "count=20" \
  -d "access_token=${VK_TOKEN}" \
  -d "v=5.199"
```

---

## Implementation Phases

### Phase 1: Core Refactoring (Week 1)
- ✅ Split types.rs into modules
- ✅ Create methods/ directory structure
- ✅ Refactor existing methods
- ✅ Add error handling
- ✅ Unit tests

### Phase 2: Messages API (Week 2)
- ✅ Implement edit, delete, search
- ✅ Implement pin/unpin
- ✅ Implement setActivity
- ✅ Implement reactions
- ✅ Integration tests

### Phase 3: Users & Friends (Week 3)
- ✅ Users API (search, subscriptions)
- ✅ Friends API (get, search, online)
- ✅ Tests + examples

### Phase 4: Groups & Extended (Week 4)
- ✅ Groups API
- ✅ Account API
- ✅ Polish & documentation

### Phase 5: Release (Week 5)
- ✅ Full test coverage
- ✅ Examples for all methods
- ✅ README + CHANGELOG
- ✅ Publish to crates.io

---

## Usage Examples

### Basic Usage

```rust
use vk_api::VkClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = VkClient::new(token);

    // Get conversations
    let chats = client.messages().get_conversations(0, 20).await?;

    // Send message
    let msg_id = client.messages().send(12345, "Hello!").await?;

    // Edit message
    client.messages().edit(12345, msg_id, "Hello, world!").await?;

    // Search messages
    let results = client.messages().search("important", None, 20).await?;

    // Get friends
    let friends = client.friends().get(None).await?;

    Ok(())
}
```

### Advanced Usage

```rust
// Long Poll
let mut server = client.longpoll().get_server().await?;
loop {
    match client.longpoll().poll(&server).await {
        Ok(response) => {
            if let Some(ts) = response.ts {
                server.ts = ts;
            }
            // Process updates
        }
        Err(e) => {
            eprintln!("Long Poll error: {}", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
```

---

**End of Design Document**
