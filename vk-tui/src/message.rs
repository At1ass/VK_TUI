use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::event::VkEvent;
use crate::state::{AttachmentInfo, Chat, ChatMessage, Focus, Mode, ReplyPreview};
use vk_api::User;

/// Messages for the TEA update loop
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Message {
    /// No operation
    Noop,
    /// Quit the application
    Quit,
    /// Open auth URL in browser
    OpenAuthUrl,
    /// Switch focus to next panel
    FocusNext,
    /// Switch focus to previous panel
    FocusPrev,
    /// Navigate up in current list
    NavigateUp,
    /// Navigate down in current list
    NavigateDown,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Select current item
    Select,
    /// Go back / cancel
    Back,

    // Insert mode - text input
    /// Input character
    InputChar(char),
    /// Delete character (backspace)
    InputBackspace,
    /// Delete word
    InputDeleteWord,
    /// Submit input (send message or confirm auth)
    InputSubmit,

    // Command mode - command input
    /// Command character
    CommandChar(char),
    /// Command backspace
    CommandBackspace,
    /// Command delete word
    CommandDeleteWord,
    /// Execute command
    CommandSubmit,

    // Mode transitions
    /// Enter Normal mode
    EnterNormalMode,
    /// Enter Insert mode
    EnterInsertMode,
    /// Enter Command mode
    EnterCommandMode,

    // Navigation
    /// Go to top of list
    GoToTop,
    /// Go to bottom of list
    GoToBottom,

    // Message actions (vi-like)
    /// Reply to selected message
    ReplyToMessage,
    /// Forward selected message
    ForwardMessage,
    /// Delete selected message
    DeleteMessage,
    /// Edit selected message
    EditMessage,
    /// Copy message text (yank)
    YankMessage,
    /// Pin/unpin message
    PinMessage,
    /// Open link from selected message
    OpenLink,
    /// Download attachments from selected message
    DownloadAttachment,

    // Search
    /// Start search mode
    StartSearch,

    // UI
    /// Toggle help popup
    ToggleHelp,
    /// Close popup
    ClosePopup,

    // VK events
    /// Send message failed
    SendFailed(String),
    /// VK API event
    VkEvent(VkEvent),
    /// Conversations loaded from API
    ConversationsLoaded(Vec<Chat>, Vec<User>),
    /// Messages loaded from API
    MessagesLoaded(Vec<ChatMessage>, Vec<User>),
    /// Message sent successfully (message_id, cmid)
    MessageSent(i64, i64),
    /// Message edited successfully
    MessageEdited(i64),
    /// Message deleted successfully
    MessageDeleted(i64), // message_id
    /// Message details fetched (update cmid/text/attachments)
    MessageDetailsFetched {
        message_id: i64,
        cmid: Option<i64>,
        text: Option<String>,
        is_edited: bool,
        attachments: Option<Vec<AttachmentInfo>>,
        reply: Option<ReplyPreview>,
        fwd_count: Option<usize>,
    },
    /// Error occurred
    Error(String),
}

impl Message {
    /// Convert key event to message based on current mode and focus
    pub fn from_key_event(key: KeyEvent, mode: Mode, focus: Focus, show_help: bool) -> Self {
        // Help popup takes precedence
        if show_help {
            return Self::help_popup_key(key);
        }

        // Global shortcuts (work in all modes)
        if let Some(global) = match key.code {
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Some(Message::Quit)
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::OpenAuthUrl)
            }
            _ => None,
        } {
            return global;
        }

        // Route to mode-specific handler
        match mode {
            Mode::Normal => Self::normal_mode_key(key, focus),
            Mode::Insert => Self::insert_mode_key(key),
            Mode::Command => Self::command_mode_key(key),
        }
    }

    /// Handle keys on the Auth screen (always acts like insert mode)
    pub fn from_auth_key_event(key: KeyEvent) -> Self {
        if let Some(global) = match key.code {
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                Some(Message::Quit)
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Message::OpenAuthUrl)
            }
            _ => None,
        } {
            return global;
        }

        match key.code {
            KeyCode::Enter => Message::InputSubmit,
            KeyCode::Backspace => Message::InputBackspace,
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::InputDeleteWord
            }
            KeyCode::Char(c) => Message::InputChar(c),
            _ => Message::Noop,
        }
    }

    /// Handle keys when help popup is open
    fn help_popup_key(key: KeyEvent) -> Self {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Message::ClosePopup,
            _ => Message::Noop,
        }
    }

    /// Handle keys in normal mode - context-aware based on focus
    fn normal_mode_key(key: KeyEvent, focus: Focus) -> Self {
        // Global Normal mode keys (work in all focuses)
        match key.code {
            KeyCode::Esc => return Message::Back,
            KeyCode::Tab => return Message::FocusNext,
            KeyCode::BackTab => return Message::FocusPrev,
            KeyCode::Char(':') => return Message::EnterCommandMode,
            KeyCode::Char('?') => return Message::ToggleHelp,
            _ => {}
        }

        // Context-specific keys based on focus
        match focus {
            Focus::ChatList => Self::chatlist_keys(key),
            Focus::Messages => Self::messages_keys(key),
            Focus::Input => {
                // Input panel in Normal mode - shouldn't happen often
                // Allow entering Insert mode
                match key.code {
                    KeyCode::Char('i') | KeyCode::Enter => Message::EnterInsertMode,
                    _ => Message::Noop,
                }
            }
        }
    }

    /// Keys for ChatList panel in Normal mode
    fn chatlist_keys(key: KeyEvent) -> Self {
        match key.code {
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => Message::NavigateDown,
            KeyCode::Char('k') | KeyCode::Up => Message::NavigateUp,
            KeyCode::Char('g') => Message::GoToTop,
            KeyCode::Char('G') => Message::GoToBottom,

            // Actions
            KeyCode::Char('l') | KeyCode::Enter => Message::Select,
            KeyCode::Char('/') => Message::StartSearch,

            _ => Message::Noop,
        }
    }

    /// Keys for Messages panel in Normal mode
    fn messages_keys(key: KeyEvent) -> Self {
        match key.code {
            // Navigation with Ctrl modifiers (must come before plain keys)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => Message::PageUp,
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::PageDown
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::OpenLink
            }

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => Message::NavigateDown,
            KeyCode::Char('k') | KeyCode::Up => Message::NavigateUp,
            KeyCode::Char('g') => Message::GoToTop,
            KeyCode::Char('G') => Message::GoToBottom,

            // Enter Insert mode
            KeyCode::Char('i') | KeyCode::Char('l') | KeyCode::Enter => Message::EnterInsertMode,

            // Message actions
            KeyCode::Char('r') => Message::ReplyToMessage,
            KeyCode::Char('f') => Message::ForwardMessage,
            KeyCode::Char('e') => Message::EditMessage,
            KeyCode::Char('p') => Message::PinMessage,

            // Double-char commands (dd, yy)
            KeyCode::Char('d') => Message::DeleteMessage, // Will need state for 'dd'
            KeyCode::Char('y') => Message::YankMessage,   // Will need state for 'yy'

            // Attachments and links
            KeyCode::Char('o') => Message::OpenLink,
            KeyCode::Char('a') => Message::DownloadAttachment,

            // Search
            KeyCode::Char('/') => Message::StartSearch,

            // Back to ChatList
            KeyCode::Char('h') => Message::FocusPrev,

            _ => Message::Noop,
        }
    }

    /// Handle keys in insert mode
    fn insert_mode_key(key: KeyEvent) -> Self {
        match key.code {
            // Exit Insert mode
            KeyCode::Esc => Message::EnterNormalMode,

            // Submit
            KeyCode::Enter => Message::InputSubmit,

            // Editing
            KeyCode::Backspace => Message::InputBackspace,
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::InputDeleteWord
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::InputDeleteWord // Clear line
            }

            // Regular character
            KeyCode::Char(c) => Message::InputChar(c),

            _ => Message::Noop,
        }
    }

    /// Handle keys in command mode
    fn command_mode_key(key: KeyEvent) -> Self {
        match key.code {
            // Exit Command mode
            KeyCode::Esc => Message::EnterNormalMode,

            // Execute command
            KeyCode::Enter => Message::CommandSubmit,

            // Editing
            KeyCode::Backspace => Message::CommandBackspace,
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::CommandDeleteWord
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Message::CommandDeleteWord // Clear command line
            }

            // Regular character
            KeyCode::Char(c) => Message::CommandChar(c),

            _ => Message::Noop,
        }
    }
}
