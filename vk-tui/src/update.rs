use std::process::Command;
use std::sync::Arc;

use crate::commands::{determine_completion_state, handle_command};
use crate::event::VkEvent;
use crate::input::{delete_word, insert_char_at, remove_char_at};
use crate::message::Message;
use crate::state::{
    App, AsyncAction, AttachmentInfo, AttachmentKind, Chat, ChatMessage, ChatsPagination,
    CompletionState, DeliveryStatus, Focus, ForwardStage, MessagesPagination, Mode, ReplyPreview,
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
                    Focus::ChatList => {
                        app.selected_chat = app.selected_chat.saturating_sub(1);
                    }
                    Focus::Messages => {
                        // Check if we're at the top and can load more older messages
                        if app.messages_scroll == 0 {
                            let (has_more, is_loading, first_cmid) = app
                                .messages_pagination
                                .as_ref()
                                .map(|p| (p.has_more, p.is_loading, p.first_cmid))
                                .unwrap_or((false, false, None));

                            tracing::debug!(
                                "NavigateUp: has_more={}, is_loading={}, first_cmid={:?}",
                                has_more,
                                is_loading,
                                first_cmid
                            );

                            let should_load = has_more && !is_loading && first_cmid.is_some();

                            if should_load && let Some(peer_id) = app.current_peer_id {
                                let first_cmid = first_cmid.unwrap();

                                if let Some(pagination) = &mut app.messages_pagination {
                                    pagination.is_loading = true;
                                }
                                app.status = Some("Loading older messages...".into());
                                tracing::debug!(
                                    "Sending LoadMessagesWithOffset: peer_id={}, first_cmid={}, offset=-1, count=50",
                                    peer_id,
                                    first_cmid
                                );
                                // Use offset=-1 to skip first_cmid itself and load older messages
                                app.send_action(AsyncAction::LoadMessagesWithOffset(
                                    peer_id, first_cmid, -1, 50,
                                ));
                            }
                        } else {
                            app.messages_scroll = app.messages_scroll.saturating_sub(1);
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
                        // Determine the visible chat count (filtered or all)
                        let visible_count = if let Some(filter) = &app.chat_filter {
                            filter.filtered_indices.len()
                        } else {
                            app.chats.len()
                        };

                        if app.selected_chat + 1 < visible_count {
                            app.selected_chat += 1;
                        } else if app.chat_filter.is_none() {
                            // At the end of chat list (not filtered) - try to load more
                            if app.chats_pagination.has_more && !app.chats_pagination.is_loading {
                                app.chats_pagination.is_loading = true;
                                app.status = Some(format!(
                                    "Loading more chats ({}/{})",
                                    app.chats.len(),
                                    app.chats_pagination
                                        .total_count
                                        .map(|t| t.to_string())
                                        .unwrap_or_else(|| "?".to_string())
                                ));
                                app.send_action(AsyncAction::LoadConversations(
                                    app.chats_pagination.offset,
                                ));
                            }
                        }
                    }
                    Focus::Messages => {
                        // Check if we're at the bottom and can load newer messages
                        if app.messages_scroll + 1 >= app.messages.len() {
                            // At the bottom - check if we can load newer messages
                            let should_load = app
                                .messages_pagination
                                .as_ref()
                                .map(|p| !p.is_loading)
                                .unwrap_or(false);

                            if should_load && let Some(peer_id) = app.current_peer_id {
                                let last_cmid =
                                    app.messages_pagination.as_ref().and_then(|p| p.last_cmid);

                                if let Some(last_cmid) = last_cmid {
                                    if let Some(pagination) = &mut app.messages_pagination {
                                        pagination.is_loading = true;
                                    }

                                    const COUNT: u32 = 50;
                                    const OFFSET: i32 = -49; // -(COUNT - 1) for overlap

                                    app.status = Some("Loading newer messages...".into());
                                    tracing::debug!(
                                        "Sending LoadMessagesWithOffset: peer_id={}, last_cmid={}, offset={}, count={}",
                                        peer_id,
                                        last_cmid,
                                        OFFSET,
                                        COUNT
                                    );

                                    // Load with fixed offset=-49 and count=50 to get overlap + new messages
                                    // Deduplication will filter out already loaded messages
                                    app.send_action(AsyncAction::LoadMessagesWithOffset(
                                        peer_id, last_cmid, OFFSET, COUNT,
                                    ));
                                }
                            }
                        } else {
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
                    // Initialize chats pagination and load first page
                    app.chats_pagination = ChatsPagination::default();
                    app.chats_pagination.is_loading = true;
                    app.send_action(AsyncAction::LoadConversations(0));
                    app.send_action(AsyncAction::StartLongPoll);
                } else {
                    app.status = Some("Failed to parse token from URL".into());
                }
            } else if app.screen == Screen::Main
                && app.focus == Focus::ChatList
                && let Some((peer_id, title)) =
                    app.current_chat().map(|chat| (chat.id, chat.title.clone()))
            {
                // Clear chat filter if active
                app.chat_filter = None;

                app.current_peer_id = Some(peer_id);
                app.messages.clear();
                app.is_loading = true;
                // Initialize messages pagination and load first page
                app.messages_pagination = Some(MessagesPagination::new(peer_id));
                if let Some(pagination) = &mut app.messages_pagination {
                    pagination.is_loading = true;
                }
                app.send_action(AsyncAction::LoadMessages(peer_id, 0));
                app.send_action(AsyncAction::MarkAsRead(peer_id));
                app.status = Some(format!("Loading chat: {}", title));
                app.focus = Focus::Messages;
            }
        }
        Message::Back => {
            if app.screen == Screen::Main {
                app.focus = Focus::ChatList;
                app.current_peer_id = None;
            }
        }
        Message::OpenLink => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if let Some(url) = first_url(msg) {
                    if let Err(e) = open::that(&url) {
                        app.status = Some(format!("Failed to open link: {}", e));
                    } else {
                        app.status = Some(format!("Opened {}", url));
                    }
                } else {
                    app.status = Some("No link in message".into());
                }
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

                if let Some((reply_id, preview)) = app.reply_to.take() {
                    app.messages.push(ChatMessage {
                        id: 0,
                        cmid: None,
                        from_id: app.auth.user_id().unwrap_or(0),
                        from_name: "You".into(),
                        text: text.clone(),
                        timestamp: chrono_timestamp(),
                        is_outgoing: true,
                        is_read: false,
                        is_edited: false,
                        is_pinned: false,
                        delivery: DeliveryStatus::Pending,
                        attachments: Vec::new(),
                        reply: Some(preview),
                        fwd_count: 0,
                        forwards: Vec::new(),
                    });
                    app.messages_scroll = app.messages.len().saturating_sub(1);
                    app.send_action(AsyncAction::SendReply(peer_id, reply_id, text));
                } else {
                    app.send_action(AsyncAction::SendMessage(peer_id, text));
                }
            }
            _ => {}
        },

        // Command mode input
        Message::CommandChar(c) => {
            insert_char_at(&mut app.command_input, app.command_cursor, c);
            app.command_cursor += 1;

            // FSM state transition based on new input
            app.completion_state = determine_completion_state(&app.command_input);
        }
        Message::CommandBackspace => {
            if app.command_cursor > 0 {
                app.command_cursor -= 1;
                remove_char_at(&mut app.command_input, app.command_cursor);

                // FSM state transition based on new input
                app.completion_state = determine_completion_state(&app.command_input);
            }
        }
        Message::CommandDeleteWord => {
            delete_word(&mut app.command_input, &mut app.command_cursor);
        }
        Message::CommandSubmit => {
            // FSM state transition on submit (Tab key behavior)
            match std::mem::take(&mut app.completion_state) {
                CompletionState::Commands {
                    suggestions,
                    selected,
                } => {
                    // Stage 1: Select command from suggestions and add space
                    let selected_cmd = &suggestions[selected];
                    // Keep the leading ':' if present
                    let prefix = if app.command_input.starts_with(':') {
                        ":"
                    } else {
                        ""
                    };
                    app.command_input = format!("{}{} ", prefix, selected_cmd.command);
                    app.command_cursor = app.command_input.len();

                    // Re-evaluate completion state for next stage
                    app.completion_state = determine_completion_state(&app.command_input);
                    return None;
                }
                CompletionState::Subcommands {
                    command,
                    options,
                    selected,
                } => {
                    // Stage 2: Select subcommand and add space
                    let selected_opt = &options[selected];
                    // Keep the leading ':' if present
                    let prefix = if app.command_input.starts_with(':') {
                        ":"
                    } else {
                        ""
                    };
                    app.command_input = format!("{}{} {} ", prefix, command, selected_opt.name);
                    app.command_cursor = app.command_input.len();

                    // Re-evaluate completion state for next stage
                    app.completion_state = determine_completion_state(&app.command_input);
                    return None;
                }
                CompletionState::FilePaths {
                    entries, selected, ..
                } => {
                    // Stage 3: Select file/directory
                    let entry = &entries[selected];

                    // Extract command part before the path
                    let parts: Vec<_> = app.command_input.split_whitespace().collect();
                    let cmd_part = if parts.len() > 2 {
                        parts[..parts.len() - 1].join(" ")
                    } else {
                        app.command_input.trim().to_string()
                    };

                    if entry.is_dir {
                        // Directory: insert with trailing slash for further completion
                        app.command_input = format!("{} {}/", cmd_part, entry.full_path);
                        app.command_cursor = app.command_input.len();

                        // Re-evaluate completion state to show directory contents
                        app.completion_state = determine_completion_state(&app.command_input);
                    } else {
                        // File: insert with space and close completion
                        app.command_input = format!("{} {} ", cmd_part, entry.full_path);
                        app.command_cursor = app.command_input.len();
                        app.completion_state = CompletionState::Inactive;
                    }
                    return None;
                }
                CompletionState::Inactive => {
                    // No completion active - execute the command
                    let cmd = app.command_input.clone();
                    if let Some(res) = handle_command(app, &cmd) {
                        return Some(res);
                    }
                    app.command_input.clear();
                    app.command_cursor = 0;
                    app.mode = Mode::Normal;
                }
            }
        }
        Message::CompletionUp => {
            // FSM state navigation
            match &mut app.completion_state {
                CompletionState::Commands {
                    selected,
                    suggestions: _,
                } => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                CompletionState::Subcommands {
                    selected,
                    options: _,
                    ..
                } => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                CompletionState::FilePaths {
                    selected,
                    entries: _,
                    ..
                } => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                CompletionState::Inactive => {}
            }
        }
        Message::CompletionDown => {
            // FSM state navigation
            match &mut app.completion_state {
                CompletionState::Commands {
                    selected,
                    suggestions,
                } => {
                    if *selected + 1 < suggestions.len() {
                        *selected += 1;
                    }
                }
                CompletionState::Subcommands {
                    selected, options, ..
                } => {
                    if *selected + 1 < options.len() {
                        *selected += 1;
                    }
                }
                CompletionState::FilePaths {
                    selected, entries, ..
                } => {
                    if *selected + 1 < entries.len() {
                        *selected += 1;
                    }
                }
                CompletionState::Inactive => {}
            }
        }
        Message::CompletionSelect => {
            // Same as Enter - handled by CommandSubmit
        }

        // Mode switches
        Message::EnterNormalMode => {
            app.mode = Mode::Normal;
            if app.focus == Focus::Input {
                app.focus = Focus::Messages;
            }
            app.command_input.clear();
            app.command_cursor = 0;
            app.completion_state = CompletionState::Inactive; // FSM reset
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
            app.command_input.clear();
            app.command_cursor = 0;

            // FSM initial state - show all commands
            app.completion_state = determine_completion_state("");

            app.status = Some("Command mode".into());
        }

        // Downloads
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

        // Message actions
        Message::ReplyToMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message().cloned()
            {
                if msg.id == 0 {
                    app.status = Some("Cannot reply to unsent message".into());
                } else {
                    let preview = ReplyPreview {
                        from: msg.from_name.clone(),
                        text: truncate_str(&msg.text, 120),
                        attachments: msg.attachments.clone(),
                    };
                    app.reply_to = Some((msg.id, preview));
                    app.mode = Mode::Insert;
                    app.focus = Focus::Input;
                    app.status = Some(format!("Replying to {} (Esc to cancel)", msg.from_name));
                }
            }
        }
        Message::DeleteMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message().cloned()
            {
                if !msg.is_outgoing {
                    app.status = Some("Can only delete your own messages".into());
                    return None;
                }
                if msg.id == 0 {
                    app.status = Some("Cannot delete message that is not sent yet".into());
                    return None;
                }
                if let Some(peer_id) = app.current_peer_id {
                    app.status = Some("Deleting message...".into());
                    app.send_action(AsyncAction::DeleteMessage(peer_id, msg.id, false));
                }
            }
        }
        Message::EditMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if !msg.is_outgoing {
                    app.status = Some("Can only edit your own messages".into());
                    return None;
                }
                app.input = msg.text.clone();
                app.input_cursor = app.input.chars().count();
                app.editing_message = Some(app.messages_scroll);
                app.mode = Mode::Insert;
                app.focus = Focus::Input;
                app.status = Some("Editing message (not yet saved)".into());
            }
        }
        Message::YankMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                app.status = Some(format!("Copied: {}", truncate_str(&msg.text, 50)));
            }
        }
        Message::PinMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
                && let Some(peer_id) = app.current_peer_id
            {
                app.status = Some(format!("Pin message {} in {}", msg.id, peer_id));
            }
        }
        Message::CancelReply => {
            app.reply_to = None;
            app.status = Some("Reply cancelled".into());
        }
        Message::ViewForwarded => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if msg.forwards.is_empty() {
                    app.status = Some("No forwarded content to view".into());
                } else {
                    app.forward_view = Some(crate::state::ForwardView {
                        items: msg.forwards.clone(),
                        selected: 0,
                    });
                }
            }
        }
        Message::ForwardViewClose => {
            app.forward_view = None;
        }
        Message::ForwardViewUp => {
            if let Some(view) = app.forward_view.as_mut() {
                view.selected = view.selected.saturating_sub(1);
            }
        }
        Message::ForwardViewDown => {
            if let Some(view) = app.forward_view.as_mut() {
                let total = forwards_len(&view.items);
                if total > 0 && view.selected + 1 < total {
                    view.selected += 1;
                }
            }
        }
        Message::ForwardMessage => {
            if app.screen == Screen::Main
                && app.focus == Focus::Messages
                && let Some(msg) = app.current_message()
            {
                if msg.id == 0 {
                    app.status = Some("Cannot forward message that is not sent yet".into());
                } else {
                    let filtered = forward_filter(&app.chats, "");
                    app.forward = Some(crate::state::ForwardState {
                        source_message_id: msg.id,
                        query: String::new(),
                        filtered,
                        selected: 0,
                        comment: String::new(),
                        stage: ForwardStage::SelectTarget,
                    });
                    app.status = Some("Select chat to forward (j/k, type to search)".into());
                }
            }
        }
        Message::ForwardCancel => {
            app.forward = None;
            app.status = Some("Forward cancelled".into());
        }
        Message::ForwardMoveUp => {
            if let Some(fwd) = app.forward.as_mut()
                && fwd.selected > 0
            {
                fwd.selected -= 1;
            }
        }
        Message::ForwardMoveDown => {
            if let Some(fwd) = app.forward.as_mut()
                && !fwd.filtered.is_empty()
                && fwd.selected + 1 < fwd.filtered.len()
            {
                fwd.selected += 1;
            }
        }
        Message::ForwardQueryChar(c) => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::SelectTarget)
            {
                fwd.query.push(c);
                fwd.filtered = forward_filter(&app.chats, &fwd.query);
                if fwd.selected >= fwd.filtered.len() {
                    fwd.selected = fwd.filtered.len().saturating_sub(1);
                }
            }
        }
        Message::ForwardQueryBackspace => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::SelectTarget)
            {
                fwd.query.pop();
                fwd.filtered = forward_filter(&app.chats, &fwd.query);
                if fwd.selected >= fwd.filtered.len() {
                    fwd.selected = fwd.filtered.len().saturating_sub(1);
                }
            }
        }
        Message::ForwardQueryDeleteWord => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::SelectTarget)
            {
                let mut cursor = fwd.query.chars().count();
                delete_word(&mut fwd.query, &mut cursor);
                fwd.filtered = forward_filter(&app.chats, &fwd.query);
                if fwd.selected >= fwd.filtered.len() {
                    fwd.selected = fwd.filtered.len().saturating_sub(1);
                }
            }
        }
        Message::ForwardSubmit => {
            if let Some(fwd) = app.forward.clone() {
                match fwd.stage {
                    ForwardStage::SelectTarget => {
                        if fwd.filtered.is_empty() {
                            app.status = Some("No chat matched".into());
                        } else if let Some(chat) = fwd.filtered.get(fwd.selected)
                            && let Some(state) = app.forward.as_mut()
                        {
                            state.stage = ForwardStage::EnterComment {
                                peer_id: chat.id,
                                title: chat.title.clone(),
                            };
                            state.comment.clear();
                            app.status =
                                Some(format!("Forward to {}: add comment or Enter", chat.title));
                        }
                    }
                    ForwardStage::EnterComment { peer_id, .. } => {
                        let (comment, source_id) = if let Some(state) = app.forward.take() {
                            (state.comment, state.source_message_id)
                        } else {
                            (String::new(), fwd.source_message_id)
                        };

                        // Optimistic placeholder
                        let text = if comment.is_empty() {
                            "[forwarded]".to_string()
                        } else {
                            comment.clone()
                        };
                        app.messages.push(ChatMessage {
                            id: 0,
                            cmid: None,
                            from_id: app.auth.user_id().unwrap_or(0),
                            from_name: "You".into(),
                            text,
                            timestamp: chrono_timestamp(),
                            is_outgoing: true,
                            is_read: false,
                            is_edited: false,
                            is_pinned: false,
                            delivery: DeliveryStatus::Pending,
                            attachments: Vec::new(),
                            reply: None,
                            fwd_count: 1,
                            forwards: Vec::new(),
                        });
                        app.messages_scroll = app.messages.len().saturating_sub(1);

                        app.status = Some("Forwarding...".into());
                        app.send_action(AsyncAction::SendForward(
                            peer_id,
                            vec![source_id],
                            comment,
                        ));
                    }
                }
            }
        }
        Message::ForwardCommentChar(c) => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::EnterComment { .. })
            {
                fwd.comment.push(c);
            }
        }
        Message::ForwardCommentBackspace => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::EnterComment { .. })
            {
                fwd.comment.pop();
            }
        }
        Message::ForwardCommentDeleteWord => {
            if let Some(fwd) = app.forward.as_mut()
                && matches!(fwd.stage, ForwardStage::EnterComment { .. })
            {
                let mut cursor = fwd.comment.chars().count();
                delete_word(&mut fwd.comment, &mut cursor);
            }
        }

        // Messages from VK events and async actions
        Message::VkEvent(event) => return handle_vk_event(app, event),
        Message::SessionValidated { valid, error } => {
            if valid {
                app.status = Some("Session validated".into());
                app.is_loading = true;
                app.chats_pagination.is_loading = true;
                app.send_action(AsyncAction::LoadConversations(0));
                app.send_action(AsyncAction::StartLongPoll);
            } else if let Some(err) = error {
                if is_auth_error(&err) {
                    let _ = app.auth.logout();
                    app.vk_client = None;
                    app.screen = Screen::Auth;
                    app.status = Some("Session expired. Please authorize again.".into());
                } else {
                    app.status = Some(err);
                }
                app.is_loading = false;
            }
        }
        Message::ConversationsLoaded {
            chats,
            profiles,
            total_count,
            has_more,
        } => {
            app.is_loading = false;

            // Append or replace chats based on offset
            if app.chats_pagination.offset == 0 {
                // First load - replace
                app.chats = chats;
            } else {
                // Pagination - append
                app.chats.extend(chats);
            }

            // Update pagination state
            app.chats_pagination.offset = app.chats.len() as u32;
            app.chats_pagination.total_count = Some(total_count);
            app.chats_pagination.has_more = has_more;
            app.chats_pagination.is_loading = false;

            // Update users cache
            for user in profiles {
                app.users.insert(user.id, user);
            }

            app.status = Some(format!(
                "Loaded {} of {} conversations",
                app.chats.len(),
                total_count
            ));
        }
        Message::MessagesLoaded {
            peer_id,
            messages,
            profiles,
            total_count,
            has_more,
        } => {
            app.is_loading = false;

            // Append or replace messages based on offset and overlap
            if let Some(pagination) = &app.messages_pagination {
                // Always check for overlap first if we have existing messages
                if !app.messages.is_empty() {
                    let existing_ids: std::collections::HashSet<i64> =
                        app.messages.iter().map(|m| m.id).collect();
                    let has_overlap = messages.iter().any(|m| existing_ids.contains(&m.id));

                    if has_overlap {
                        // There's overlap - determine if we're loading newer or older messages
                        // by comparing message IDs
                        let max_existing_id = app.messages.iter().map(|m| m.id).max().unwrap_or(0);
                        let max_new_id = messages.iter().map(|m| m.id).max().unwrap_or(0);

                        if max_new_id > max_existing_id {
                            // Loading newer messages - append only new ones
                            let current_last_scroll = app.messages_scroll;
                            for msg in messages {
                                if !existing_ids.contains(&msg.id) {
                                    app.messages.push(msg);
                                }
                            }
                            app.messages_scroll = current_last_scroll;
                        } else {
                            // Loading older messages - prepend only new ones
                            let _loaded_count = messages.len();
                            let mut new_messages: Vec<_> = messages
                                .into_iter()
                                .filter(|m| !existing_ids.contains(&m.id))
                                .collect();
                            let prepend_count = new_messages.len();
                            new_messages.append(&mut app.messages);
                            app.messages = new_messages;
                            app.messages_scroll = app.messages_scroll.saturating_add(prepend_count);
                        }
                    } else if pagination.offset == 0 {
                        // No overlap and offset=0 - replace all (first load)
                        app.messages = messages;
                        app.messages_scroll = app.messages.len().saturating_sub(1);
                    } else {
                        // No overlap - prepend older messages
                        let loaded_count = messages.len();
                        let mut new_messages = messages;
                        new_messages.append(&mut app.messages);
                        app.messages = new_messages;
                        app.messages_scroll = app.messages_scroll.saturating_add(loaded_count);
                    }
                } else {
                    // Empty - first load
                    app.messages = messages;
                    app.messages_scroll = app.messages.len().saturating_sub(1);
                }

                // Update pagination state
                if let Some(pagination) = &mut app.messages_pagination {
                    pagination.offset = app.messages.len() as u32;
                    pagination.total_count = Some(total_count);
                    pagination.has_more = has_more;
                    pagination.is_loading = false;

                    // Update first and last cmid from current messages
                    if !app.messages.is_empty() {
                        // First message is oldest (prepended at start)
                        pagination.first_cmid = app.messages.first().and_then(|m| m.cmid);
                        // Last message is newest (appended at end)
                        pagination.last_cmid = app.messages.last().and_then(|m| m.cmid);

                        tracing::debug!(
                            "Updated pagination: first_cmid={:?}, last_cmid={:?}, has_more={}, offset={}, messages_count={}",
                            pagination.first_cmid,
                            pagination.last_cmid,
                            pagination.has_more,
                            pagination.offset,
                            app.messages.len()
                        );

                        if pagination.first_cmid.is_none() || pagination.last_cmid.is_none() {
                            tracing::warn!(
                                "Some messages have no cmid! first: id={}, cmid={:?}; last: id={}, cmid={:?}",
                                app.messages.first().map(|m| m.id).unwrap_or(0),
                                pagination.first_cmid,
                                app.messages.last().map(|m| m.id).unwrap_or(0),
                                pagination.last_cmid
                            );
                        }
                    }
                }
            } else {
                // No pagination state - first load
                app.messages = messages;
                app.messages_scroll = app.messages.len().saturating_sub(1);
            }

            // Mark messages as read
            if Some(peer_id) == app.current_peer_id {
                if let Some(chat) = app.chats.iter_mut().find(|c| c.id == peer_id) {
                    chat.unread_count = 0;
                }
                for msg in app.messages.iter_mut() {
                    if !msg.is_outgoing {
                        msg.is_read = true;
                    }
                }
            }

            // Update users cache
            for user in profiles {
                app.users.insert(user.id, user);
            }

            // If we have a target message, scroll to it
            if let Some(target_id) = app.target_message_id
                && let Some(pos) = app.messages.iter().position(|m| m.id == target_id)
            {
                app.messages_scroll = pos;
                app.target_message_id = None;
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
            forwards,
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
                if let Some(fwds) = forwards {
                    msg.forwards = fwds;
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
        // Search / UI
        Message::StartSearch => {
            if app.screen == Screen::Main {
                app.status = Some("Search not yet implemented".into());
            }
        }
        Message::ToggleHelp => {
            app.show_help = !app.show_help;
        }
        Message::ClosePopup => {
            app.show_help = false;
        }

        // Chat filter
        Message::StartChatFilter => {
            if app.screen == Screen::Main && app.focus == Focus::ChatList {
                let mut filter = crate::state::ChatFilter::new();
                // Initialize with all chats
                filter.filtered_indices = crate::search::filter_chats(&app.chats, "");
                app.chat_filter = Some(filter);
                // Reset selection to first chat
                app.selected_chat = 0;
                app.status = Some("Filter: (type to search, Esc to cancel)".into());
            }
        }
        Message::FilterChar(c) => {
            if let Some(filter) = &mut app.chat_filter {
                crate::input::insert_char_at(&mut filter.query, filter.cursor, c);
                filter.cursor += 1;
                // Update filtered indices
                filter.filtered_indices = crate::search::filter_chats(&app.chats, &filter.query);
                // Reset selection to first result
                app.selected_chat = 0;
                app.status = Some(format!(
                    "Filter: {} ({} matches)",
                    filter.query,
                    filter.filtered_indices.len()
                ));
            }
        }
        Message::FilterBackspace => {
            if let Some(filter) = &mut app.chat_filter
                && filter.cursor > 0
            {
                filter.cursor -= 1;
                crate::input::remove_char_at(&mut filter.query, filter.cursor);
                // Update filtered indices
                filter.filtered_indices = crate::search::filter_chats(&app.chats, &filter.query);
                // Reset selection to first result
                app.selected_chat = 0;
                app.status = Some(format!(
                    "Filter: {} ({} matches)",
                    filter.query,
                    filter.filtered_indices.len()
                ));
            }
        }
        Message::ClearFilter => {
            app.chat_filter = None;
            app.status = None;
        }

        // Global search
        Message::StartGlobalSearch => {
            let search = crate::state::GlobalSearch::new();
            app.global_search = Some(search);
            app.status = Some("Global search: (type to search, Esc to cancel)".into());
        }
        Message::GlobalSearchChar(c) => {
            if let Some(search) = &mut app.global_search {
                crate::input::insert_char_at(&mut search.query, search.cursor, c);
                search.cursor += 1;
                // Trigger search with debounce
                search.is_loading = true;
                let query = search.query.clone();
                let status = format!("Searching: {}", search.query);
                app.send_action(AsyncAction::SearchMessages(query));
                app.status = Some(status);
            }
        }
        Message::GlobalSearchBackspace => {
            if let Some(search) = &mut app.global_search
                && search.cursor > 0
            {
                search.cursor -= 1;
                crate::input::remove_char_at(&mut search.query, search.cursor);
                if search.query.is_empty() {
                    search.results.clear();
                    search.total_count = 0;
                    search.selected = 0;
                    app.status = Some("Global search: (type to search, Esc to cancel)".into());
                } else {
                    search.is_loading = true;
                    let query = search.query.clone();
                    let status = format!("Searching: {}", search.query);
                    app.send_action(AsyncAction::SearchMessages(query));
                    app.status = Some(status);
                }
            }
        }
        Message::ClearGlobalSearch => {
            app.global_search = None;
            app.status = None;
        }
        Message::GlobalSearchUp => {
            if let Some(search) = &mut app.global_search {
                search.selected = search.selected.saturating_sub(1);
            }
        }
        Message::GlobalSearchDown => {
            if let Some(search) = &mut app.global_search
                && search.selected + 1 < search.results.len()
            {
                search.selected += 1;
            }
        }
        Message::GlobalSearchSelect => {
            if let Some(search) = &app.global_search
                && let Some(result) = search.results.get(search.selected)
            {
                let peer_id = result.peer_id;
                let message_id = result.message_id;

                // Close search
                app.global_search = None;

                // Open chat and load messages around the found message
                app.current_peer_id = Some(peer_id);
                app.messages.clear();
                app.target_message_id = Some(message_id);
                app.is_loading = true;
                app.messages_pagination = Some(crate::state::MessagesPagination::new(peer_id));
                if let Some(pagination) = &mut app.messages_pagination {
                    pagination.is_loading = true;
                }
                app.send_action(AsyncAction::LoadMessagesAround(peer_id, message_id));
                app.send_action(AsyncAction::MarkAsRead(peer_id));
                app.status = Some("Loading chat...".to_string());
                app.focus = Focus::Messages;
            }
        }
        Message::SearchResultsLoaded {
            results,
            total_count,
        } => {
            if let Some(search) = &mut app.global_search {
                search.results = results;
                search.total_count = total_count;
                search.selected = 0;
                search.is_loading = false;
                app.status = Some(format!(
                    "Found {} results for '{}'",
                    total_count, search.query
                ));
            }
        }
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
                is_pinned: false,
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
                forwards: Vec::new(),
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
                is_pinned: false,
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
                forwards: Vec::new(),
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
                    is_pinned: false,
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
                    forwards: Vec::new(),
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
            message_id,
            peer_id,
            timestamp,
            text,
            from_id,
            is_outgoing,
        } => {
            if app.current_peer_id == Some(peer_id) {
                app.messages.push(ChatMessage {
                    id: message_id,
                    cmid: None,
                    from_id,
                    from_name: app.get_user_name(from_id),
                    text,
                    timestamp,
                    is_outgoing,
                    is_read: true,
                    is_edited: false,
                    is_pinned: false,
                    delivery: DeliveryStatus::Sent,
                    attachments: Vec::new(),
                    reply: None,
                    fwd_count: 0,
                    forwards: Vec::new(),
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
            if app.current_peer_id == Some(peer_id)
                && let Some(pos) = app.messages.iter().position(|m| m.id == message_id)
            {
                app.messages.remove(pos);
                if app.messages_scroll >= app.messages.len() && app.messages_scroll > 0 {
                    app.messages_scroll -= 1;
                }
                app.status = Some("Message deleted from web".into());
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

// command handling moved to commands.rs

// Helpers moved from app.rs
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

// Command parsing helpers for slash-commands
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

fn forward_filter(chats: &[Chat], query: &str) -> Vec<Chat> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return chats.to_vec();
    }
    chats
        .iter()
        .filter(|c| {
            let title = c.title.to_lowercase();
            title.contains(&q) || c.id.to_string().contains(&q)
        })
        .cloned()
        .collect()
}

pub fn flatten_forwards(
    items: &[crate::state::ForwardItem],
    indent: usize,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for item in items {
        let text = format!("{}: {}", item.from, truncate_str(&item.text, 120));
        out.push((indent, text));
        if !item.nested.is_empty() {
            out.extend(flatten_forwards(&item.nested, indent + 1));
        }
    }
    out
}

fn forwards_len(items: &[crate::state::ForwardItem]) -> usize {
    items.iter().map(|i| 1 + forwards_len(&i.nested)).sum()
}
