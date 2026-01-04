//! Application state types for TUI.
//!
//! This module re-exports core types from vk-core and defines
//! TUI-specific state types.

use std::collections::HashMap;
use tokio::sync::mpsc;

use vk_api::User;
use vk_api::auth::AuthManager;

// Re-export core types
pub use vk_core::{
    AttachmentInfo, AttachmentKind, Chat, ChatMessage, ChatsPagination, DeliveryStatus,
    ForwardItem, MessagesPagination, ReplyPreview, SearchResult,
};

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

/// Async actions to be performed in background
pub enum AsyncAction {
    ValidateSession,
    LoadConversations(u32),                     // offset
    LoadMessages(i64, u32),                     // peer_id, offset
    LoadMessagesAround(i64, i64),               // peer_id, message_id
    LoadMessagesWithOffset(i64, i64, i32, u32), // peer_id, start_message_id, offset, count
    SendMessage(i64, String),                   // peer_id, text
    SendForward(i64, Vec<i64>, String),         // peer_id, message_ids, comment
    SendReply(i64, i64, String),                // peer_id, reply_to_msg_id, text
    StartLongPoll,
    MarkAsRead(i64),
    SendPhoto(i64, String), // peer_id, path
    SendDoc(i64, String),   // peer_id, path
    DownloadAttachments(Vec<AttachmentInfo>),
    EditMessage(i64, i64, Option<i64>, String), // peer_id, message_id, cmid, text
    #[allow(dead_code)]
    DeleteMessage(i64, i64, bool), // peer_id, message_id, delete_for_all
    FetchMessageById(i64),                      // message_id - to get cmid after sending
    SearchMessages(String),                     // query
}

/// Chat filter state for local fuzzy search
#[derive(Debug, Clone)]
pub struct ChatFilter {
    pub query: String,
    pub cursor: usize,
    pub filtered_indices: Vec<usize>,
}

impl ChatFilter {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
            filtered_indices: Vec::new(),
        }
    }
}

/// Global search state
#[derive(Debug, Clone)]
pub struct GlobalSearch {
    pub query: String,
    pub cursor: usize,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub is_loading: bool,
    pub total_count: i32,
}

impl GlobalSearch {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
            results: Vec::new(),
            selected: 0,
            is_loading: false,
            total_count: 0,
        }
    }
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
    pub target_message_id: Option<i64>,
    pub reply_to: Option<(i64, ReplyPreview)>,

    // Search and filter state
    pub chat_filter: Option<ChatFilter>,
    pub global_search: Option<GlobalSearch>,

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
            target_message_id: None,
            reply_to: None,
            chat_filter: None,
            global_search: None,
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
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
}

/// Completion state machine
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub enum CompletionState {
    /// No completion active
    #[default]
    Inactive,

    /// Completing command names
    Commands {
        suggestions: Vec<CommandSuggestion>,
        selected: usize,
    },

    /// Completing subcommands
    Subcommands {
        command: String,
        options: Vec<SubcommandOption>,
        selected: usize,
    },

    /// Completing file paths
    FilePaths {
        context: String,
        base_path: String,
        entries: Vec<PathEntry>,
        selected: usize,
    },
}
