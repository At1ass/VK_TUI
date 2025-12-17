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
    #[allow(dead_code)]
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
    pub attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, Clone)]
pub struct ForwardItem {
    pub from: String,
    pub text: String,
    pub attachments: Vec<AttachmentInfo>,
    pub nested: Vec<ForwardItem>,
}

/// A single message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub cmid: Option<i64>,
    #[allow(dead_code)]
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

/// Async actions to be performed in background

/// Pagination state for messages in a specific chat
#[derive(Debug, Clone)]
pub struct MessagesPagination {
    pub peer_id: i64,
    pub offset: u32,
    pub total_count: Option<u32>,
    pub is_loading: bool,
    pub has_more: bool,
}

impl MessagesPagination {
    pub fn new(peer_id: i64) -> Self {
        Self {
            peer_id,
            offset: 0,
            total_count: None,
            is_loading: false,
            has_more: true,
        }
    }
}

/// Pagination state for chat list
#[derive(Debug, Clone)]
pub struct ChatsPagination {
    pub offset: u32,
    pub total_count: Option<u32>,
    pub is_loading: bool,
    pub has_more: bool,
}

impl Default for ChatsPagination {
    fn default() -> Self {
        Self {
            offset: 0,
            total_count: None,
            is_loading: false,
            has_more: true,
        }
    }
}

pub enum AsyncAction {
    LoadConversations(u32),             // offset
    LoadMessages(i64, u32),             // peer_id, offset
    SendMessage(i64, String),           // peer_id, text
    SendForward(i64, Vec<i64>, String), // peer_id, message_ids, comment
    SendReply(i64, i64, String),        // peer_id, reply_to_msg_id, text
    StartLongPoll,
    MarkAsRead(i64),
    SendPhoto(i64, String), // peer_id, path
    SendDoc(i64, String),   // peer_id, path
    DownloadAttachments(Vec<AttachmentInfo>),
    EditMessage(i64, i64, Option<i64>, String), // peer_id, message_id, cmid, text
    #[allow(dead_code)]
    DeleteMessage(i64, i64, bool), // peer_id, message_id, delete_for_all
    FetchMessageById(i64),                      // message_id - to get cmid after sending
}

/// Application state (Model in TEA)
pub struct App {
    pub running_state: RunningState,
    pub screen: Screen,
    pub focus: Focus,
    pub mode: Mode,

    // Forward modal state
    pub forward: Option<ForwardState>,

    // Auth state
    pub auth: AuthManager,
    pub token_input: String,
    pub token_cursor: usize,

    // VK state
    pub vk_client: Option<std::sync::Arc<vk_api::VkClient>>,
    pub users: HashMap<i64, User>,
    #[allow(dead_code)]
    pub current_user: Option<User>,

    // Chat state
    pub chats: Vec<Chat>,
    pub selected_chat: usize,
    pub current_peer_id: Option<i64>,
    pub messages: Vec<ChatMessage>,
    pub messages_scroll: usize,
    pub reply_to: Option<(i64, ReplyPreview)>,

    // Pagination state
    pub chats_pagination: ChatsPagination,
    pub messages_pagination: Option<MessagesPagination>,

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
    pub forward_view: Option<ForwardView>,
    pub completion_state: CompletionState,

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
            reply_to: None,
            chats_pagination: ChatsPagination::default(),
            messages_pagination: None,
            input: String::new(),
            input_cursor: 0,
            command_input: String::new(),
            command_cursor: 0,
            status: None,
            is_loading: false,
            editing_message: None,
            show_help: false,
            forward_view: None,
            completion_state: CompletionState::default(),
            forward: None,
            action_tx: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ForwardStage {
    SelectTarget,
    EnterComment { peer_id: i64, title: String },
}

#[derive(Debug, Clone)]
pub struct ForwardState {
    pub source_message_id: i64,
    pub query: String,
    pub filtered: Vec<Chat>,
    pub selected: usize,
    pub comment: String,
    pub stage: ForwardStage,
}

#[derive(Debug, Clone)]
pub struct ForwardView {
    pub items: Vec<ForwardItem>,
    pub selected: usize,
}

/// Command completion suggestion
#[derive(Debug, Clone)]
pub struct CommandSuggestion {
    pub command: String,
    pub description: String,
    pub usage: Option<String>,
}

/// Subcommand option (e.g., "photo" or "doc" for :attach)
#[derive(Debug, Clone)]
pub struct SubcommandOption {
    pub name: String,
    pub description: String,
}

/// File system entry for path completion
#[derive(Debug, Clone)]
pub struct PathEntry {
    pub name: String,        // "file.jpg"
    pub full_path: String,   // "/home/user/file.jpg"
    pub is_dir: bool,
}

/// Completion state machine
#[derive(Debug, Clone)]
pub enum CompletionState {
    /// No completion active
    Inactive,

    /// Completing command names
    /// Example: ":att|ach"
    Commands {
        suggestions: Vec<CommandSuggestion>,
        selected: usize,
    },

    /// Completing subcommands
    /// Example: ":attach |photo"
    Subcommands {
        command: String,
        options: Vec<SubcommandOption>,
        selected: usize,
    },

    /// Completing file paths
    /// Example: ":attach photo |/home/user/file.jpg"
    FilePaths {
        context: String,         // "attach photo"
        base_path: String,       // "/home/user"
        entries: Vec<PathEntry>,
        selected: usize,
    },
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::Inactive
    }
}
