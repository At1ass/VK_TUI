use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Chat, ChatMessage};
use crate::event::VkEvent;
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
    /// Select current item
    Select,
    /// Go back / cancel
    Back,
    /// Input character
    InputChar(char),
    /// Delete character (backspace)
    InputBackspace,
    /// Delete word
    InputDeleteWord,
    /// Submit input (send message or confirm auth)
    InputSubmit,
    /// Go to top of list
    GoToTop,
    /// Go to bottom of list
    GoToBottom,
    /// Open link from selected message
    OpenLink,
    /// Download attachments from selected message
    DownloadAttachment,
    /// Send message failed
    SendFailed(String),
    /// VK API event
    VkEvent(VkEvent),
    /// Conversations loaded from API
    ConversationsLoaded(Vec<Chat>, Vec<User>),
    /// Messages loaded from API
    MessagesLoaded(Vec<ChatMessage>, Vec<User>),
    /// Message sent successfully
    MessageSent(i64),
    /// Error occurred
    Error(String),
}

impl Message {
    /// Convert key event to message based on current mode
    /// When in_input is true, most keys go to input; when false, vi-like navigation
    pub fn from_key_event(key: KeyEvent, in_input: bool) -> Self {
        // Global shortcuts (work everywhere)
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Message::Quit;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Message::Quit;
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Message::OpenAuthUrl;
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Message::OpenLink;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Message::DownloadAttachment;
            }
            KeyCode::Esc => return Message::Back,
            _ => {}
        }

        // Input mode - characters go to input
        if in_input {
            return Self::input_mode_key(key);
        }

        // Normal mode - vi-like navigation
        Self::normal_mode_key(key)
    }

    /// Handle keys in input mode
    fn input_mode_key(key: KeyEvent) -> Self {
        match key.code {
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

    /// Handle keys in normal (navigation) mode - vi-like
    fn normal_mode_key(key: KeyEvent) -> Self {
        match key.code {
            // Vi-like navigation
            KeyCode::Char('j') | KeyCode::Down => Message::NavigateDown,
            KeyCode::Char('k') | KeyCode::Up => Message::NavigateUp,
            KeyCode::Char('h') | KeyCode::Left => Message::FocusPrev,
            KeyCode::Char('l') | KeyCode::Right => Message::FocusNext,

            // Jump to top/bottom
            KeyCode::Char('g') => Message::GoToTop,
            KeyCode::Char('G') => Message::GoToBottom,

            // Select / Enter chat / Start input
            KeyCode::Enter | KeyCode::Char('i') => Message::Select,

            // Tab also switches panels
            KeyCode::Tab => Message::FocusNext,
            KeyCode::BackTab => Message::FocusPrev,

            // In auth screen, allow typing
            KeyCode::Char(c) => Message::InputChar(c),
            KeyCode::Backspace => Message::InputBackspace,

            _ => Message::Noop,
        }
    }
}
