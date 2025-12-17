//! Application state types split out of app.rs for readability.
use std::collections::HashMap;
use tokio::sync::mpsc;

use vk_api::User;
use vk_api::auth::AuthManager;

/// Current screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Auth,
    Main,
}

/// Which panel is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    ChatList,
    Messages,
    Input,
}

/// Vi-like input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Command,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::ChatList => Focus::Messages,
            Focus::Messages => Focus::Input,
            Focus::Input => Focus::ChatList,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Focus::ChatList => Focus::Input,
            Focus::Messages => Focus::ChatList,
            Focus::Input => Focus::Messages,
        }
    }
}

/// Application running state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

/// A chat/conversation in the list
#[derive(Debug, Clone)]
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub last_message: String,
    pub last_message_time: i64,
    pub unread_count: u32,
    pub is_online: bool,
}

/// Delivery state for messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Failed,
}

/// Attachment summary for UI/commands
#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    pub kind: AttachmentKind,
    pub title: String,
    pub url: Option<String>,
    pub size: Option<u64>,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AttachmentKind {
    Photo,
    Doc,
    Link,
    Audio,
    Sticker,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct ReplyPreview {
    pub from: String,
    pub text: String,
}

/// A single message
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
    pub delivery: DeliveryStatus,
    pub attachments: Vec<AttachmentInfo>,
    pub reply: Option<ReplyPreview>,
    pub fwd_count: usize,
}

/// Async actions to be performed in background
#[derive(Debug, Clone)]
pub enum AsyncAction {
    LoadConversations,
    LoadMessages(i64),        // peer_id
    SendMessage(i64, String), // peer_id, text
    StartLongPoll,
    MarkAsRead(i64),
    SendPhoto(i64, String), // peer_id, path
    SendDoc(i64, String),   // peer_id, path
    DownloadAttachments(Vec<AttachmentInfo>),
    EditMessage(i64, i64, Option<i64>, String), // peer_id, message_id, cmid, text
    DeleteMessage(i64, i64, bool),              // peer_id, message_id, delete_for_all
    FetchMessageById(i64),                      // message_id - to get cmid after sending
}

/// Application state (Model in TEA)
pub struct App {
    pub running_state: RunningState,
    pub screen: Screen,
    pub focus: Focus,
    pub mode: Mode,

    // Auth state
    pub auth: AuthManager,
    pub token_input: String,
    pub token_cursor: usize,

    // VK state
    pub vk_client: Option<std::sync::Arc<vk_api::VkClient>>,
    pub users: HashMap<i64, User>,
    pub current_user: Option<User>,

    // Chat state
    pub chats: Vec<Chat>,
    pub selected_chat: usize,
    pub current_peer_id: Option<i64>,
    pub messages: Vec<ChatMessage>,
    pub messages_scroll: usize,

    // Input state
    pub input: String,
    pub input_cursor: usize,

    // Command mode state
    pub command_input: String,
    pub command_cursor: usize,

    // UI state
    pub status: Option<String>,
    pub is_loading: bool,
    pub editing_message: Option<usize>,
    pub show_help: bool,

    // Async action sender
    pub action_tx: Option<mpsc::UnboundedSender<AsyncAction>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running_state: RunningState::Running,
            screen: Screen::Auth,
            focus: Focus::ChatList,
            mode: Mode::Normal,
            auth: AuthManager::default(),
            token_input: String::new(),
            token_cursor: 0,
            vk_client: None,
            users: HashMap::new(),
            current_user: None,
            chats: Vec::new(),
            selected_chat: 0,
            current_peer_id: None,
            messages: Vec::new(),
            messages_scroll: 0,
            input: String::new(),
            input_cursor: 0,
            command_input: String::new(),
            command_cursor: 0,
            status: None,
            is_loading: false,
            editing_message: None,
            show_help: false,
            action_tx: None,
        }
    }
}
