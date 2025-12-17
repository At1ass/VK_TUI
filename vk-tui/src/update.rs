use std::process::Command;
use std::sync::Arc;

use crate::event::VkEvent;
use crate::message::Message;
use crate::state::{
    App, AsyncAction, AttachmentInfo, AttachmentKind, ChatMessage, DeliveryStatus, Focus, Mode,
    RunningState, Screen,
};
use vk_api::VkClient;

pub fn update(app: &mut App, msg: Message) -> Option<Message> {
    match msg {
        Message::Noop => {}
        Message::Quit => {
            app.running_state = RunningState::Done;
        }
        Message::OpenAuthUrl => {
            if app.screen == Screen::Auth {
                let url = app.auth_url();
                if let Err(e) = open::that(&url) {
                    app.status = Some(format!("Failed to open browser: {}", e));
                } else {
                    app.status =
                        Some("Opened in browser. Authorize and paste the redirect URL.".into());
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
                    Focus::ChatList => app.selected_chat = app.selected_chat.saturating_sub(1),
                    Focus::Messages => app.messages_scroll = app.messages_scroll.saturating_sub(1),
                    Focus::Input => {}
                }
            }
        }
        Message::NavigateDown => {
            if app.screen == Screen::Main {
                match app.focus {
                    Focus::ChatList => {
                        if app.selected_chat + 1 < app.chats.len() {
                            app.selected_chat += 1;
                        }
                    }
                    Focus::Messages => {
                        if app.messages_scroll + 1 < app.messages.len() {
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
                    Focus::ChatList => app.selected_chat = app.chats.len().saturating_sub(1),
                    Focus::Messages => app.messages_scroll = app.messages.len().saturating_sub(1),
                    Focus::Input => {}
                }
            }
        }
        Message::Select => {
            if app.screen == Screen::Auth {
                if app.auth.save_token_from_url(&app.token_input).is_ok()
                    && let Some(token) = app.auth.access_token()
                {
                    app.vk_client = Some(Arc::new(VkClient::new(token.to_string())));
                    app.screen = Screen::Main;
                    app.status = Some("Authenticated successfully".into());
                    app.send_action(AsyncAction::LoadConversations);
                    app.send_action(AsyncAction::StartLongPoll);
                } else {
                    app.status = Some("Failed to parse token from URL".into());
                }
            } else if app.screen == Screen::Main && app.focus == Focus::ChatList {
                if let Some((peer_id, title)) =
                    app.current_chat().map(|chat| (chat.id, chat.title.clone()))
                {
                    app.current_peer_id = Some(peer_id);
                    app.messages.clear();
                    app.is_loading = true;
                    app.send_action(AsyncAction::LoadMessages(peer_id));
                    app.send_action(AsyncAction::MarkAsRead(peer_id));
                    app.status = Some(format!("Loading chat: {}", title));
                    app.focus = Focus::Messages;
                }
            }
        }
        Message::Back => {
            if app.screen == Screen::Main {
                app.focus = Focus::ChatList;
                app.current_peer_id = None;
            }
        }
        Message::PageUp => {
            if app.screen == Screen::Main && app.focus == Focus::Messages {
                app.messages_scroll = app.messages_scroll.saturating_sub(10);
            }
        }
        Message::PageDown => {
            if app.screen == Screen::Main && app.focus == Focus::Messages {
                app.messages_scroll =
                    (app.messages_scroll + 10).min(app.messages.len().saturating_sub(1));
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
            delete_word(input, cursor);
        }
        Message::InputSubmit => match app.screen {
            Screen::Auth => return Some(Message::Select),
            Screen::Main if app.focus == Focus::Input => {
                if app.input.is_empty() {
                    return None;
                }
                let peer_id = match app.current_peer_id {
                    Some(id) => id,
                    None => {
                        app.status = Some("No chat selected".into());
                        return None;
                    }
                };
                if let Some(edit_idx) = app.editing_message {
                    let (message_id, cmid) = if let Some(msg) = app.messages.get(edit_idx) {
                        if msg.id == 0 {
                            app.status = Some("Cannot edit message that is not sent yet".into());
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
                    app.send_action(AsyncAction::EditMessage(peer_id, message_id, cmid, text));
                    return None;
                }

                if let Some(cmd) = parse_send_command(&app.input) {
                    return handle_send_command(app, peer_id, cmd);
                }

                let text = std::mem::take(&mut app.input);
                app.input_cursor = 0;
                app.mode = Mode::Normal;
                app.status = Some("Sending...".into());
                app.send_action(AsyncAction::SendMessage(peer_id, text));
            }
            _ => {}
        },

        // Command mode input
        Message::CommandChar(c) => {
            insert_char_at(&mut app.command_input, app.command_cursor, c);
            app.command_cursor += 1;
        }
        Message::CommandBackspace => {
            if app.command_cursor > 0 {
                app.command_cursor -= 1;
                remove_char_at(&mut app.command_input, app.command_cursor);
            }
        }
        Message::CommandDeleteWord => {
            delete_word(&mut app.command_input, &mut app.command_cursor);
        }
        Message::CommandSubmit => {
            let cmd = app.command_input.clone();
            if let Some(res) = handle_command(app, &cmd) {
                return Some(res);
            }
            app.command_input.clear();
            app.command_cursor = 0;
            app.mode = Mode::Normal;
        }

        // Mode switches
        Message::EnterNormalMode => {
            app.mode = Mode::Normal;
            app.status = Some("Normal mode".into());
        }
        Message::EnterInsertMode => {
            app.mode = Mode::Insert;
            app.focus = Focus::Input;
            app.status = Some("Insert mode".into());
        }
        Message::EnterCommandMode => {
            app.mode = Mode::Command;
            app.focus = Focus::Input;
            app.status = Some("Command mode".into());
        }

        // Downloads
        Message::DownloadAttachment => {
            if app.screen == Screen::Main && app.focus == Focus::Messages {
                if let Some(msg) = app.current_message() {
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
        }

        // Messages from VK events and async actions
        Message::VkEvent(event) => return handle_vk_event(app, event),
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
            if let Some(msg) = app.messages.last_mut()
                && msg.id == 0
            {
                msg.id = msg_id;
                msg.cmid = Some(cmid);
                msg.delivery = DeliveryStatus::Sent;
            }
            app.send_action(AsyncAction::FetchMessageById(msg_id));
        }
        Message::MessageEdited(msg_id) => {
            app.status = Some("Message edited".into());
            app.editing_message = None;
            if let Some(msg) = app.messages.iter_mut().find(|m| m.id == msg_id) {
                msg.delivery = DeliveryStatus::Sent;
                msg.is_edited = true;
            }
            app.send_action(AsyncAction::FetchMessageById(msg_id));
        }
        Message::MessageDeleted(msg_id) => {
            app.status = Some("Message deleted".into());
            if let Some(pos) = app.messages.iter().position(|m| m.id == msg_id) {
                app.messages.remove(pos);
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
            attachments,
            reply,
            fwd_count,
        } => {
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
                if let Some(atts) = attachments {
                    msg.attachments = atts;
                }
                if let Some(r) = reply {
                    msg.reply = Some(r);
                }
                if let Some(fwd) = fwd_count {
                    msg.fwd_count = fwd;
                }
            }
        }
        Message::Error(err) => {
            app.is_loading = false;
            if is_auth_error(&err) {
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
            {
                last.delivery = DeliveryStatus::Failed;
            }
            app.status = Some(format!("Failed to send: {}", err));
        }
        _ => {}
    }

    None
}

fn handle_send_command(app: &mut App, peer_id: i64, cmd: SendCommand) -> Option<Message> {
    match cmd {
        SendCommand::File(path) => {
            let title = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();

            app.messages.push(ChatMessage {
                id: 0,
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
                    kind: AttachmentKind::Doc,
                    title: title.clone(),
                    url: None,
                    size: None,
                    subtitle: None,
                }],
                reply: None,
                fwd_count: 0,
            });
            app.messages_scroll = app.messages.len().saturating_sub(1);
            app.input.clear();
            app.input_cursor = 0;
            app.send_action(AsyncAction::SendDoc(peer_id, path));
            None
        }
        SendCommand::Image(path) => {
            let title = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image")
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
                    subtitle: None,
                }],
                reply: None,
                fwd_count: 0,
            });
            app.messages_scroll = app.messages.len().saturating_sub(1);
            app.input.clear();
            app.input_cursor = 0;
            app.send_action(AsyncAction::SendPhoto(peer_id, path));
            None
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
                        subtitle: None,
                    }],
                    reply: None,
                    fwd_count: 0,
                });
                app.messages_scroll = app.messages.len().saturating_sub(1);
                app.input.clear();
                app.input_cursor = 0;
                if let Some(path_str) = path.to_str() {
                    app.send_action(AsyncAction::SendPhoto(peer_id, path_str.to_string()));
                }
                None
            }
            Err(e) => {
                app.status = Some(format!("Clipboard image error: {}", e));
                None
            }
        },
    }
}

fn handle_vk_event(app: &mut App, event: VkEvent) -> Option<Message> {
    match event {
        VkEvent::NewMessage {
            peer_id,
            text,
            from_id,
        } => {
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
            } else if let Some(chat) = app.chats.iter_mut().find(|c| c.id == peer_id) {
                chat.unread_count += 1;
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
                    for msg in app.messages.iter_mut() {
                        if msg.is_outgoing && msg.id <= message_id {
                            msg.is_read = true;
                            msg.delivery = DeliveryStatus::Sent;
                        }
                    }
                } else {
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
            if app.current_peer_id == Some(peer_id) {
                app.send_action(AsyncAction::FetchMessageById(message_id));
                app.status = Some("Message updated from web".into());
            }
        }
        VkEvent::MessageDeletedFromLongPoll {
            peer_id,
            message_id,
        } => {
            if app.current_peer_id == Some(peer_id) {
                if let Some(pos) = app.messages.iter().position(|m| m.id == message_id) {
                    app.messages.remove(pos);
                    if app.messages_scroll >= app.messages.len() && app.messages_scroll > 0 {
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
    None
}

fn handle_command(app: &mut App, cmd: &str) -> Option<Message> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "q" | "quit" | "qa" | "quitall" => {
            app.running_state = RunningState::Done;
        }
        "b" | "back" => {
            app.focus = Focus::ChatList;
            app.current_peer_id = None;
        }
        "s" | "search" => {
            if parts.len() > 1 {
                let query = parts[1..].join(" ");
                app.status = Some(format!("Search: {} (not yet implemented)", query));
            } else {
                app.status = Some("Usage: :search <query>".into());
            }
        }
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
                app.status = Some("Usage: :attach photo|doc <path>".into());
            }
        }
        "dl" | "download" => {
            if let Some(msg) = app.current_message() {
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
        "h" | "help" => {
            app.show_help = true;
        }
        _ => {
            app.status = Some(format!("Unknown command: {}", parts[0]));
        }
    }

    None
}

// Helpers moved from app.rs
#[allow(dead_code)]
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

fn is_auth_error(msg: &str) -> bool {
    msg.contains("VK API error 5")
        || msg.contains("VK API error 7")
        || msg.contains("VK API error 179")
        || msg.to_lowercase().contains("authorization failed")
}

// === Input helpers ===
fn delete_word(input: &mut String, cursor: &mut usize) {
    while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| c.is_whitespace()) {
        *cursor -= 1;
        remove_char_at(input, *cursor);
    }
    while *cursor > 0 && char_at(input, *cursor - 1).is_some_and(|c| !c.is_whitespace()) {
        *cursor -= 1;
        remove_char_at(input, *cursor);
    }
}

// Minimal copies of helpers to keep update.rs self-contained
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn insert_char_at(s: &mut String, char_pos: usize, c: char) {
    let byte_idx = char_to_byte_index(s, char_pos);
    s.insert(byte_idx, c);
}

fn remove_char_at(s: &mut String, char_pos: usize) -> Option<char> {
    let byte_idx = char_to_byte_index(s, char_pos);
    if byte_idx < s.len() {
        Some(s.remove(byte_idx))
    } else {
        None
    }
}

fn char_at(s: &str, char_pos: usize) -> Option<char> {
    s.chars().nth(char_pos)
}

// Command parsing helpers (minimal inline)
#[derive(Debug, Clone)]
enum SendCommand {
    File(String),
    Image(String),
    ImageClipboard,
}

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

fn read_clipboard_image() -> anyhow::Result<std::path::PathBuf> {
    let mut errors = Vec::new();
    let mut data: Option<Vec<u8>> = None;

    match Command::new("wl-paste")
        .args(["--type", "image/png"])
        .output()
    {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            data = Some(output.stdout);
        }
        Ok(output) => errors.push(format!("wl-paste status {}", output.status)),
        Err(e) => errors.push(format!("wl-paste missing: {}", e)),
    }

    if data.is_none() {
        match Command::new("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-o"])
            .output()
        {
            Ok(output) if output.status.success() && !output.stdout.is_empty() => {
                data = Some(output.stdout);
            }
            Ok(output) => errors.push(format!("xclip status {}", output.status)),
            Err(e) => errors.push(format!("xclip missing: {}", e)),
        }
    }

    let data =
        data.ok_or_else(|| anyhow::anyhow!("Clipboard image unavailable ({})", errors.join("; ")))?;

    let path = std::env::temp_dir().join("vk_tui_clipboard.png");
    std::fs::write(&path, data)?;
    Ok(path)
}
