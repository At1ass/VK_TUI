use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::event::VkEvent;
use crate::message::Message;
use vk_api::auth::AuthManager;
use vk_api::{User, VkClient};

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
    /// Normal mode - navigation and commands
    #[default]
    Normal,
    /// Insert mode - text input
    Insert,
    /// Command mode - : commands
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

/// A single message
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

/// Application state (Model in TEA)
pub struct App {
    /// Current running state
    pub running_state: RunningState,
    /// Current screen
    pub screen: Screen,
    /// Currently focused panel
    pub focus: Focus,
    /// Current vi-like mode
    pub mode: Mode,

    // Auth state
    /// Auth manager
    pub auth: AuthManager,
    /// Token input field (for pasting OAuth redirect URL)
    pub token_input: String,
    /// Token input cursor position
    pub token_cursor: usize,

    // VK state
    /// VK API client
    pub vk_client: Option<Arc<VkClient>>,
    /// User cache (id -> User)
    pub users: HashMap<i64, User>,
    /// Current user info
    #[allow(dead_code)]
    pub current_user: Option<User>,

    // Chat state
    /// List of chats
    pub chats: Vec<Chat>,
    /// Currently selected chat index
    pub selected_chat: usize,
    /// Currently open chat peer_id
    pub current_peer_id: Option<i64>,
    /// Messages in current chat
    pub messages: Vec<ChatMessage>,
    /// Currently selected message index (for scrolling)
    pub messages_scroll: usize,

    // Input state
    /// Current input text
    pub input: String,
    /// Cursor position in input
    pub input_cursor: usize,

    // Command mode state
    /// Command input (for : commands)
    pub command_input: String,
    /// Command cursor position
    pub command_cursor: usize,

    // UI state
    /// Status message (errors, info)
    pub status: Option<String>,
    /// Is loading data
    pub is_loading: bool,
    /// Currently editing message index
    pub editing_message: Option<usize>,
    /// Show help popup
    pub show_help: bool,

    // Async action sender
    pub action_tx: Option<mpsc::UnboundedSender<AsyncAction>>,
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
}

#[derive(Debug, Clone)]
pub enum AttachmentKind {
    Photo,
    File,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct ReplyPreview {
    pub from: String,
    pub text: String,
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

impl App {
    /// Create new application state
    pub fn new() -> Self {
        let mut app = Self::default();

        // Check if we have a saved token
        if app.auth.is_authenticated()
            && let Some(token) = app.auth.access_token()
        {
            app.vk_client = Some(Arc::new(VkClient::new(token.to_string())));
            app.screen = Screen::Main;
            app.status = Some("Restoring session...".into());
        }

        app
    }

    /// Get auth URL for display
    pub fn auth_url(&self) -> String {
        AuthManager::get_auth_url()
    }

    /// Check if app should quit
    pub fn is_running(&self) -> bool {
        self.running_state == RunningState::Running
    }

    /// Set action sender
    pub fn set_action_tx(&mut self, tx: mpsc::UnboundedSender<AsyncAction>) {
        self.action_tx = Some(tx);
    }

    /// Send async action
    pub fn send_action(&self, action: AsyncAction) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Get current chat peer_id
    pub fn current_chat(&self) -> Option<&Chat> {
        self.chats.get(self.selected_chat)
    }

    /// Get user name by id
    pub fn get_user_name(&self, user_id: i64) -> String {
        if let Some(user) = self.users.get(&user_id) {
            user.full_name()
        } else if user_id < 0 {
            format!("Group {}", -user_id)
        } else {
            format!("User {}", user_id)
        }
    }

    /// Get currently highlighted message
    pub fn current_message(&self) -> Option<&ChatMessage> {
        self.messages.get(self.messages_scroll)
    }
}

/// Update function - handles messages and updates state
pub fn update(app: &mut App, msg: Message) -> Option<Message> {
    match msg {
        Message::Noop => {}

        Message::Quit => {
            app.running_state = RunningState::Done;
        }

        Message::OpenAuthUrl => {
            if app.screen == Screen::Auth {
                let url = app.auth_url();
                match open::that(&url) {
                    Ok(()) => {
                        app.status =
                            Some("Opened in browser. Authorize and paste the redirect URL.".into());
                    }
                    Err(e) => {
                        app.status = Some(format!("Failed to open browser: {}", e));
                    }
                }
            }
        }

        Message::FocusNext => {
            if app.screen == Screen::Main {
                app.focus = app.focus.next();
            }
        }

        Message::FocusPrev => {
            if app.screen == Screen::Main {
                app.focus = app.focus.prev();
            }
        }

        Message::NavigateUp => {
            if app.screen == Screen::Main {
                match app.focus {
                    Focus::ChatList => {
                        if app.selected_chat > 0 {
                            app.selected_chat -= 1;
                        }
                    }
                    Focus::Messages => {
                        if app.messages_scroll > 0 {
                            app.messages_scroll -= 1;
                        }
                    }
                    Focus::Input => {}
                }
            }
        }

        Message::NavigateDown => {
            if app.screen == Screen::Main {
                match app.focus {
                    Focus::ChatList => {
                        if app.selected_chat < app.chats.len().saturating_sub(1) {
                            app.selected_chat += 1;
                        }
                    }
                    Focus::Messages => {
                        if app.messages_scroll < app.messages.len().saturating_sub(1) {
                            app.messages_scroll += 1;
                        }
                    }
                    Focus::Input => {}
                }
            }
        }

        Message::GoToTop => {
            if app.screen == Screen::Main {
                match app.focus {
                    Focus::ChatList => app.selected_chat = 0,
                    Focus::Messages => app.messages_scroll = 0,
                    Focus::Input => {}
                }
            }
        }

        Message::GoToBottom => {
            if app.screen == Screen::Main {
                match app.focus {
                    Focus::ChatList => {
                        app.selected_chat = app.chats.len().saturating_sub(1);
                    }
                    Focus::Messages => {
                        app.messages_scroll = app.messages.len().saturating_sub(1);
                    }
                    Focus::Input => {}
                }
            }
        }

        Message::Select => {
            match app.screen {
                Screen::Auth => {
                    // Try to authenticate with entered token URL
                    if !app.token_input.is_empty() {
                        match app.auth.save_token_from_url(&app.token_input) {
                            Ok(()) => {
                                if let Some(token) = app.auth.access_token() {
                                    app.vk_client =
                                        Some(Arc::new(VkClient::new(token.to_string())));
                                    app.screen = Screen::Main;
                                    app.token_input.clear();
                                    app.token_cursor = 0;
                                    app.status = Some("Authenticated! Loading chats...".into());
                                    app.is_loading = true;
                                    app.send_action(AsyncAction::LoadConversations);
                                    app.send_action(AsyncAction::StartLongPoll);
                                }
                            }
                            Err(e) => {
                                app.status = Some(format!("Auth error: {}", e));
                            }
                        }
                    }
                }
                Screen::Main => {
                    match app.focus {
                        Focus::ChatList => {
                            if let Some(chat) = app.chats.get(app.selected_chat) {
                                let peer_id = chat.id;
                                app.current_peer_id = Some(peer_id);
                                app.messages.clear();
                                app.messages_scroll = 0;
                                app.focus = Focus::Messages;
                                app.is_loading = true;
                                app.send_action(AsyncAction::LoadMessages(peer_id));
                                app.send_action(AsyncAction::MarkAsRead(peer_id));
                                if let Some(chat) = app.chats.get_mut(app.selected_chat) {
                                    chat.unread_count = 0;
                                }
                            }
                        }
                        Focus::Input => {
                            // Send message on Enter
                            return Some(Message::InputSubmit);
                        }
                        Focus::Messages => {
                            // Switch to input on Enter in messages
                            app.focus = Focus::Input;
                        }
                    }
                }
            }
        }

        Message::Back => match app.screen {
            Screen::Auth => {}
            Screen::Main => match app.focus {
                Focus::Messages | Focus::Input => {
                    app.focus = Focus::ChatList;
                    app.current_peer_id = None;
                }
                Focus::ChatList => {}
            },
        },

        Message::OpenLink => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if let Some(url) = first_url(msg) {
                    match open::that(&url) {
                        Ok(()) => app.status = Some(format!("Opened {}", url)),
                        Err(e) => app.status = Some(format!("Failed to open link: {}", e)),
                    }
                } else {
                    app.status = Some("No link in message".into());
                }
            }
        }

        Message::DownloadAttachment => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                let downloadable: Vec<AttachmentInfo> = msg
                    .attachments
                    .iter()
                    .filter(|a| a.url.is_some())
                    .cloned()
                    .collect();
                if downloadable.is_empty() {
                    app.status = Some("No downloadable attachments".into());
                } else {
                    app.send_action(AsyncAction::DownloadAttachments(downloadable));
                    app.status = Some("Downloading attachments...".into());
                }
            }
        }

        Message::InputChar(c) => match app.screen {
            Screen::Auth => {
                insert_char_at(&mut app.token_input, app.token_cursor, c);
                app.token_cursor += 1;
            }
            Screen::Main if app.focus == Focus::Input => {
                insert_char_at(&mut app.input, app.input_cursor, c);
                app.input_cursor += 1;
            }
            _ => {}
        },

        Message::InputBackspace => match app.screen {
            Screen::Auth => {
                if app.token_cursor > 0 {
                    app.token_cursor -= 1;
                    remove_char_at(&mut app.token_input, app.token_cursor);
                }
            }
            Screen::Main if app.focus == Focus::Input => {
                if app.input_cursor > 0 {
                    app.input_cursor -= 1;
                    remove_char_at(&mut app.input, app.input_cursor);
                }
            }
            _ => {}
        },

        Message::InputDeleteWord => {
            let (input, cursor) = match app.screen {
                Screen::Auth => (&mut app.token_input, &mut app.token_cursor),
                Screen::Main if app.focus == Focus::Input => {
                    (&mut app.input, &mut app.input_cursor)
                }
                _ => return None,
            };

            // Delete whitespace
            while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| c.is_whitespace()) {
                *cursor -= 1;
                remove_char_at(input, *cursor);
            }
            // Delete word
            while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| !c.is_whitespace()) {
                *cursor -= 1;
                remove_char_at(input, *cursor);
            }
        }

        Message::InputSubmit => {
            match app.screen {
                Screen::Auth => {
                    return Some(Message::Select);
                }
                Screen::Main if app.focus == Focus::Input => {
                    if !app.input.is_empty()
                        && let Some(peer_id) = app.current_peer_id
                    {
                        // Editing existing message
                        if let Some(edit_idx) = app.editing_message {
                            let (message_id, cmid) = if let Some(msg) = app.messages.get(edit_idx) {
                                if msg.id == 0 {
                                    app.status =
                                        Some("Cannot edit message that is not sent yet".into());
                                    app.editing_message = None;
                                    return None;
                                }
                                (msg.id, msg.cmid)
                            } else {
                                app.editing_message = None;
                                return None;
                            };

                            let text = std::mem::take(&mut app.input);
                            app.input_cursor = 0;
                            app.mode = Mode::Normal;
                            app.editing_message = None;
                            app.status = Some("Editing...".into());

                            if let Some(m) = app.messages.get_mut(edit_idx) {
                                m.text = text.clone();
                            }

                            app.status = Some("Editing...".into());

                            app.send_action(AsyncAction::EditMessage(
                                peer_id, message_id, cmid, text,
                            ));
                            return None;
                        }

                        if let Some(cmd) = parse_send_command(&app.input) {
                            match cmd {
                                SendCommand::File(path) => {
                                    let title = std::path::Path::new(&path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("file")
                                        .to_string();

                                    app.messages.push(ChatMessage {
                                        id: 0, // Will be updated from server
                                        cmid: None,
                                        from_id: app.auth.user_id().unwrap_or(0),
                                        from_name: "You".into(),
                                        text: format!("[file] {}", title),
                                        timestamp: chrono_timestamp(),
                                        is_outgoing: true,
                                        is_read: false,
                                        is_edited: false,
                                        delivery: DeliveryStatus::Pending,
                                        attachments: vec![AttachmentInfo {
                                            kind: AttachmentKind::File,
                                            title: title.clone(),
                                            url: None,
                                            size: None,
                                        }],
                                        reply: None,
                                        fwd_count: 0,
                                    });
                                    app.messages_scroll = app.messages.len().saturating_sub(1);
                                    app.input.clear();
                                    app.input_cursor = 0;
                                    app.send_action(AsyncAction::SendDoc(peer_id, path));
                                    return None;
                                }
                                SendCommand::Image(path) => {
                                    let title = std::path::Path::new(&path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("image")
                                        .to_string();

                                    app.messages.push(ChatMessage {
                                        id: 0, // Will be updated from server
                                        cmid: None,
                                        from_id: app.auth.user_id().unwrap_or(0),
                                        from_name: "You".into(),
                                        text: format!("[image] {}", title),
                                        timestamp: chrono_timestamp(),
                                        is_outgoing: true,
                                        is_read: false,
                                        is_edited: false,
                                        delivery: DeliveryStatus::Pending,
                                        attachments: vec![AttachmentInfo {
                                            kind: AttachmentKind::Photo,
                                            title: title.clone(),
                                            url: None,
                                            size: None,
                                        }],
                                        reply: None,
                                        fwd_count: 0,
                                    });
                                    app.messages_scroll = app.messages.len().saturating_sub(1);
                                    app.input.clear();
                                    app.input_cursor = 0;
                                    app.send_action(AsyncAction::SendPhoto(peer_id, path));
                                    return None;
                                }
                                SendCommand::ImageClipboard => match read_clipboard_image() {
                                    Ok(path) => {
                                        let title = path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("clipboard.png")
                                            .to_string();
                                        app.messages.push(ChatMessage {
                                            id: 0,
                                            cmid: None,
                                            from_id: app.auth.user_id().unwrap_or(0),
                                            from_name: "You".into(),
                                            text: format!("[image] {}", title),
                                            timestamp: chrono_timestamp(),
                                            is_outgoing: true,
                                            is_read: false,
                                            is_edited: false,
                                            delivery: DeliveryStatus::Pending,
                                            attachments: vec![AttachmentInfo {
                                                kind: AttachmentKind::Photo,
                                                title: title.clone(),
                                                url: None,
                                                size: None,
                                            }],
                                            reply: None,
                                            fwd_count: 0,
                                        });
                                        app.messages_scroll = app.messages.len().saturating_sub(1);
                                        app.input.clear();
                                        app.input_cursor = 0;
                                        if let Some(path_str) = path.to_str() {
                                            app.send_action(AsyncAction::SendPhoto(
                                                peer_id,
                                                path_str.to_string(),
                                            ));
                                        }
                                        return None;
                                    }
                                    Err(e) => {
                                        app.status = Some(format!("Clipboard image error: {}", e));
                                        return None;
                                    }
                                },
                            }
                        }

                        let text = std::mem::take(&mut app.input);
                        app.input_cursor = 0;

                        // Add message locally (optimistic update)
                        app.messages.push(ChatMessage {
                            id: 0, // Will be updated from server
                            cmid: None,
                            from_id: app.auth.user_id().unwrap_or(0),
                            from_name: "You".into(),
                            text: text.clone(),
                            timestamp: chrono_timestamp(),
                            is_outgoing: true,
                            is_read: false,
                            is_edited: false,
                            delivery: DeliveryStatus::Pending,
                            attachments: Vec::new(),
                            reply: None,
                            fwd_count: 0,
                        });
                        app.messages_scroll = app.messages.len().saturating_sub(1);

                        // Send to server
                        app.send_action(AsyncAction::SendMessage(peer_id, text));
                    }
                }
                _ => {}
            }
        }

        Message::VkEvent(vk_event) => {
            match vk_event {
                VkEvent::NewMessage {
                    peer_id,
                    text,
                    from_id,
                } => {
                    // Update chat list
                    if let Some(chat) = app.chats.iter_mut().find(|c| c.id == peer_id) {
                        chat.last_message = text.clone();
                        chat.last_message_time = chrono_timestamp();
                        if app.current_peer_id == Some(peer_id) {
                            chat.unread_count = 0;
                        } else {
                            chat.unread_count += 1;
                        }
                    }

                    // Add to current chat if open
                    if app.current_peer_id == Some(peer_id) {
                        app.messages.push(ChatMessage {
                            id: 0,
                            cmid: None,
                            from_id,
                            from_name: app.get_user_name(from_id),
                            text,
                            timestamp: chrono_timestamp(),
                            is_outgoing: from_id == app.auth.user_id().unwrap_or(0),
                            is_read: true,
                            is_edited: false,
                            delivery: DeliveryStatus::Sent,
                            attachments: Vec::new(),
                            reply: None,
                            fwd_count: 0,
                        });
                        app.messages_scroll = app.messages.len().saturating_sub(1);
                        app.send_action(AsyncAction::MarkAsRead(peer_id));
                    }
                }
                VkEvent::MessageRead {
                    peer_id,
                    message_id,
                } => {
                    if let Some(chat) = app.chats.iter_mut().find(|c| c.id == peer_id) {
                        chat.unread_count = 0;
                    }
                    if app.current_peer_id == Some(peer_id) {
                        if message_id > 0 {
                            // Mark all outgoing messages up to message_id as read
                            // This handles Long Poll event 7 (outgoing messages read by opponent)
                            for msg in app.messages.iter_mut() {
                                if msg.is_outgoing && msg.id <= message_id {
                                    msg.is_read = true;
                                    msg.delivery = DeliveryStatus::Sent;
                                }
                            }
                        } else {
                            // message_id == 0: mark all outgoing messages as read
                            for msg in app.messages.iter_mut().filter(|m| m.is_outgoing) {
                                msg.is_read = true;
                                msg.delivery = DeliveryStatus::Sent;
                            }
                        }
                    }
                }
                VkEvent::MessageEditedFromLongPoll {
                    peer_id,
                    message_id,
                } => {
                    // Fetch fresh data from API to reflect authoritative text/cmid
                    if app.current_peer_id == Some(peer_id) {
                        app.send_action(AsyncAction::FetchMessageById(message_id));
                        app.status = Some("Message updated from web".into());
                    }
                }
                VkEvent::MessageDeletedFromLongPoll {
                    peer_id,
                    message_id,
                } => {
                    // Remove message from current chat
                    if app.current_peer_id == Some(peer_id) {
                        if let Some(pos) = app.messages.iter().position(|m| m.id == message_id) {
                            app.messages.remove(pos);
                            // Adjust scroll position if needed
                            if app.messages_scroll >= app.messages.len() && app.messages_scroll > 0
                            {
                                app.messages_scroll -= 1;
                            }
                            app.status = Some("Message deleted from web".into());
                        }
                    }
                }
                VkEvent::UserTyping { peer_id, user_id } => {
                    if app.current_peer_id == Some(peer_id) {
                        let name = app.get_user_name(user_id);
                        app.status = Some(format!("{} is typing...", name));
                    }
                }
                VkEvent::ConnectionStatus(connected) => {
                    app.status = Some(if connected {
                        "Connected to VK".into()
                    } else {
                        "Disconnected from VK".into()
                    });
                }
            }
        }

        Message::ConversationsLoaded(chats, users) => {
            app.is_loading = false;
            app.chats = chats;
            for user in users {
                app.users.insert(user.id, user);
            }
            app.status = Some(format!("Loaded {} conversations", app.chats.len()));
        }

        Message::MessagesLoaded(messages, users) => {
            app.is_loading = false;
            app.messages = messages;
            app.messages_scroll = app.messages.len().saturating_sub(1);
            if let Some(peer_id) = app.current_peer_id {
                if let Some(chat) = app.chats.iter_mut().find(|c| c.id == peer_id) {
                    chat.unread_count = 0;
                }
                for msg in app.messages.iter_mut() {
                    if !msg.is_outgoing {
                        msg.is_read = true;
                    }
                }
            }
            for user in users {
                app.users.insert(user.id, user);
            }
        }

        Message::MessageSent(msg_id, cmid) => {
            // Update the last message ID and cmid
            if let Some(msg) = app.messages.last_mut()
                && msg.id == 0
            {
                msg.id = msg_id;
                msg.cmid = Some(cmid);
                msg.delivery = DeliveryStatus::Sent;
            }

            // If cmid is missing (can happen in some responses), fetch full message to fill it
            if cmid == 0 {
                app.send_action(AsyncAction::FetchMessageById(msg_id));
            }
        }

        Message::MessageEdited(msg_id) => {
            app.status = Some("Message edited".into());
            app.editing_message = None;
            if let Some(msg) = app.messages.iter_mut().find(|m| m.id == msg_id) {
                msg.delivery = DeliveryStatus::Sent;
                msg.is_edited = true;
            }
            // Refresh message from API to get authoritative text / cmid
            app.send_action(AsyncAction::FetchMessageById(msg_id));
        }

        Message::MessageDeleted(msg_id) => {
            app.status = Some("Message deleted".into());
            // Remove the message from the list
            if let Some(pos) = app.messages.iter().position(|m| m.id == msg_id) {
                app.messages.remove(pos);
                // Adjust scroll position if needed
                if app.messages_scroll >= app.messages.len() && app.messages_scroll > 0 {
                    app.messages_scroll -= 1;
                }
            }
        }

        Message::MessageDetailsFetched {
            message_id,
            cmid,
            text,
            is_edited,
        } => {
            // Update details for the message
            if let Some(msg) = app.messages.iter_mut().find(|m| m.id == message_id) {
                if let Some(cmid) = cmid {
                    msg.cmid = Some(cmid);
                }
                if let Some(text) = text {
                    msg.text = text;
                }
                if is_edited {
                    msg.is_edited = true;
                }
            }
        }

        Message::Error(err) => {
            app.is_loading = false;
            if is_auth_error(&err) {
                // Token likely invalid/expired: clear session and return to Auth
                let _ = app.auth.logout();
                app.vk_client = None;
                app.screen = Screen::Auth;
                app.focus = Focus::ChatList;
                app.mode = Mode::Insert;
                app.chats.clear();
                app.messages.clear();
                app.current_peer_id = None;
                app.status = Some("Authorization failed. Please re-authenticate.".into());
            } else {
                app.status = Some(format!("Error: {}", err));
            }
        }

        Message::SendFailed(err) => {
            app.is_loading = false;
            if let Some(last) = app.messages.last_mut()
                && last.delivery == DeliveryStatus::Pending
                && last.is_outgoing
            {
                last.delivery = DeliveryStatus::Failed;
            }
            app.status = Some(err);
        }

        // === Mode transitions ===
        Message::EnterNormalMode => {
            app.mode = Mode::Normal;
            // When leaving Insert mode, move focus to Messages
            if app.focus == Focus::Input {
                app.focus = Focus::Messages;
            }
            // Clear command input when leaving Command mode
            app.command_input.clear();
            app.command_cursor = 0;
        }

        Message::EnterInsertMode => {
            app.mode = Mode::Insert;
            // Move focus to Input panel
            app.focus = Focus::Input;
        }

        Message::EnterCommandMode => {
            app.mode = Mode::Command;
            app.command_input.clear();
            app.command_cursor = 0;
        }

        // === Command mode input ===
        Message::CommandChar(c) => {
            if app.mode == Mode::Command {
                insert_char_at(&mut app.command_input, app.command_cursor, c);
                app.command_cursor += 1;
            }
        }

        Message::CommandBackspace => {
            if app.mode == Mode::Command && app.command_cursor > 0 {
                app.command_cursor -= 1;
                remove_char_at(&mut app.command_input, app.command_cursor);
            }
        }

        Message::CommandDeleteWord => {
            if app.mode == Mode::Command {
                // Delete whitespace
                while app.command_cursor > 0
                    && char_at(&app.command_input, app.command_cursor - 1)
                        .is_some_and(|c| c.is_whitespace())
                {
                    app.command_cursor -= 1;
                    remove_char_at(&mut app.command_input, app.command_cursor);
                }
                // Delete word
                while app.command_cursor > 0
                    && char_at(&app.command_input, app.command_cursor - 1)
                        .is_some_and(|c| !c.is_whitespace())
                {
                    app.command_cursor -= 1;
                    remove_char_at(&mut app.command_input, app.command_cursor);
                }
            }
        }

        Message::CommandSubmit => {
            if app.mode == Mode::Command {
                let cmd = app.command_input.trim().to_string();
                app.mode = Mode::Normal;
                app.command_input.clear();
                app.command_cursor = 0;
                return handle_command(app, &cmd);
            }
        }

        // === Page navigation ===
        Message::PageUp => {
            if app.screen == Screen::Main && app.focus == Focus::Messages {
                let page_size = 10; // TODO: calculate from terminal height
                app.messages_scroll = app.messages_scroll.saturating_sub(page_size);
            }
        }

        Message::PageDown => {
            if app.screen == Screen::Main && app.focus == Focus::Messages {
                let page_size = 10; // TODO: calculate from terminal height
                app.messages_scroll =
                    (app.messages_scroll + page_size).min(app.messages.len().saturating_sub(1));
            }
        }

        // === Vi-like message actions ===
        Message::ReplyToMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(_msg) = app.current_message()
            {
                // TODO: Implement reply functionality
                app.status = Some("Reply not yet implemented".into());
            }
        }

        Message::ForwardMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(_msg) = app.current_message()
            {
                // TODO: Implement forward functionality
                app.status = Some("Forward not yet implemented".into());
            }
        }

        Message::DeleteMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if msg.is_outgoing {
                    let msg_id = msg.id;
                    if msg_id == 0 {
                        app.status = Some("Cannot delete message that is not sent yet".into());
                        return None;
                    }
                    if let Some(peer_id) = app.current_peer_id {
                        app.status = Some("Deleting message...".into());
                        app.send_action(AsyncAction::DeleteMessage(peer_id, msg_id, false));
                    }
                } else {
                    app.status = Some("Can only delete your own messages".into());
                }
            }
        }

        Message::EditMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if msg.is_outgoing {
                    // Pre-fill input with message text and switch to Insert mode
                    app.input = msg.text.clone();
                    app.input_cursor = app.input.chars().count();
                    app.editing_message = Some(app.messages_scroll);
                    app.mode = Mode::Insert;
                    app.focus = Focus::Input;
                    app.status = Some("Editing message (not yet saved)".into());
                } else {
                    app.status = Some("Can only edit your own messages".into());
                }
            }
        }

        Message::YankMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                // TODO: Copy to system clipboard
                app.status = Some(format!("Copied: {}", truncate_str(&msg.text, 50)));
            }
        }

        Message::PinMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if let Some(peer_id) = app.current_peer_id {
                    // TODO: Add AsyncAction::PinMessage
                    app.status = Some(format!("Pin message {} in {}", msg.id, peer_id));
                }
            }
        }

        // === Search ===
        Message::StartSearch => {
            if app.screen == Screen::Main {
                // TODO: Implement search UI
                app.status = Some("Search not yet implemented".into());
            }
        }

        // === Help popup ===
        Message::ToggleHelp => {
            app.show_help = !app.show_help;
        }

        Message::ClosePopup => {
            app.show_help = false;
        }
    }

    None
}

/// Handle command mode commands
fn handle_command(app: &mut App, cmd: &str) -> Option<Message> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        // Quit commands
        "q" | "quit" => {
            app.running_state = RunningState::Done;
        }
        "qa" | "quitall" => {
            app.running_state = RunningState::Done;
        }

        // Navigation
        "b" | "back" => {
            app.focus = Focus::ChatList;
            app.current_peer_id = None;
        }

        // Search
        "s" | "search" => {
            if parts.len() > 1 {
                let query = parts[1..].join(" ");
                app.status = Some(format!("Search: {} (not yet implemented)", query));
            } else {
                app.status = Some("Usage: :search <query>".into());
            }
        }

        // Quick message
        "m" | "msg" => {
            if parts.len() > 1 {
                let text = parts[1..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendMessage(peer_id, text));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else {
                app.status = Some("Usage: :msg <text>".into());
            }
        }

        // Attachments
        "ap" | "attach" => {
            if parts.len() > 2 && parts[1] == "photo" {
                let path = parts[2..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendPhoto(peer_id, path));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else if parts.len() > 2 && parts[1] == "doc" {
                let path = parts[2..].join(" ");
                if let Some(peer_id) = app.current_peer_id {
                    app.send_action(AsyncAction::SendDoc(peer_id, path));
                } else {
                    app.status = Some("No chat selected".into());
                }
            } else {
                app.status = Some("Usage: :attach photo <path> | :attach doc <path>".into());
            }
        }

        // Help
        "h" | "help" => {
            app.show_help = true;
        }

        // Close popup
        "close" => {
            app.show_help = false;
        }

        _ => {
            app.status = Some(format!("Unknown command: {}", parts[0]));
        }
    }

    None
}

/// Truncate string for display
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars()
                .take(max_len.saturating_sub(3))
                .collect::<String>()
        )
    }
}

fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Helper to get byte index from char position
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Insert char at character position (not byte position)
fn insert_char_at(s: &mut String, char_pos: usize, c: char) {
    let byte_idx = char_to_byte_index(s, char_pos);
    s.insert(byte_idx, c);
}

/// Remove char at character position (not byte position)
fn remove_char_at(s: &mut String, char_pos: usize) -> Option<char> {
    let byte_idx = char_to_byte_index(s, char_pos);
    if byte_idx < s.len() {
        Some(s.remove(byte_idx))
    } else {
        None
    }
}

/// Get character at position
fn char_at(s: &str, char_pos: usize) -> Option<char> {
    s.chars().nth(char_pos)
}

/// Extract first URL from message text or attachments
fn first_url(msg: &ChatMessage) -> Option<String> {
    extract_first_url(&msg.text).or_else(|| msg.attachments.iter().find_map(|a| a.url.clone()))
}

fn extract_first_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|token| token.starts_with("http://") || token.starts_with("https://"))
        .map(|s| {
            s.trim_matches(|c: char| c.is_ascii_punctuation())
                .to_string()
        })
}

/// Parse slash commands from input
fn parse_send_command(input: &str) -> Option<SendCommand> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("/sendfile ") {
        let path = rest.trim().to_string();
        if !path.is_empty() {
            return Some(SendCommand::File(path));
        }
    }
    if let Some(rest) = trimmed.strip_prefix("/sendimg ") {
        let arg = rest.trim();
        if arg == "--clipboard" {
            return Some(SendCommand::ImageClipboard);
        }
        if !arg.is_empty() {
            return Some(SendCommand::Image(arg.to_string()));
        }
    }
    None
}

/// Read image from clipboard using xclip (Linux). Returns path to temp file.
fn read_clipboard_image() -> anyhow::Result<std::path::PathBuf> {
    let mut errors = Vec::new();
    let mut data: Option<Vec<u8>> = None;

    // Try Wayland first
    match Command::new("wl-paste")
        .args(["--type", "image/png"])
        .output()
    {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            data = Some(output.stdout);
        }
        Ok(output) => {
            errors.push(format!("wl-paste status {}", output.status));
        }
        Err(e) => errors.push(format!("wl-paste missing: {}", e)),
    }

    // Fallback to xclip (X11)
    if data.is_none() {
        match Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-o"])
            .output()
        {
            Ok(output) if output.status.success() && !output.stdout.is_empty() => {
                data = Some(output.stdout);
            }
            Ok(output) => {
                errors.push(format!("xclip status {}", output.status));
            }
            Err(e) => errors.push(format!("xclip missing: {}", e)),
        }
    }

    let data =
        data.ok_or_else(|| anyhow::anyhow!("Clipboard image unavailable ({})", errors.join("; ")))?;

    let path = std::env::temp_dir().join("vk_tui_clipboard.png");
    std::fs::write(&path, data)?;
    Ok(path)
}

enum SendCommand {
    File(String),
    Image(String),
    ImageClipboard,
}

fn is_auth_error(msg: &str) -> bool {
    msg.contains("VK API error 5")
        || msg.contains("VK API error 7")
        || msg.contains("VK API error 179")
        || msg.to_lowercase().contains("authorization failed")
}
