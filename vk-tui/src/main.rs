mod app;
mod event;
mod message;
mod ui;

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use app::{
    App, AsyncAction, AttachmentInfo, AttachmentKind, Chat, ChatMessage, DeliveryStatus, update,
};
use event::{Event, VkEvent};
use message::Message;
use vk_api::{VkClient, User};

/// Initialize terminal
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to original state
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Setup panic hook to restore terminal on panic
fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

/// Spawn async action handler
fn spawn_action_handler(
    mut action_rx: mpsc::UnboundedReceiver<AsyncAction>,
    message_tx: mpsc::UnboundedSender<Message>,
    vk_client: Option<Arc<VkClient>>,
) {
    tokio::spawn(async move {
        while let Some(action) = action_rx.recv().await {
            let client = match &vk_client {
                Some(c) => c.clone(),
                None => {
                    let _ = message_tx.send(Message::Error("Not authenticated".into()));
                    continue;
                }
            };

            let tx = message_tx.clone();

            match action {
                AsyncAction::LoadConversations => {
                    tokio::spawn(load_conversations(client, tx));
                }
                AsyncAction::LoadMessages(peer_id) => {
                    tokio::spawn(load_messages(client, peer_id, tx));
                }
                AsyncAction::SendMessage(peer_id, text) => {
                    tokio::spawn(send_message(client, peer_id, text, tx));
                }
                AsyncAction::StartLongPoll => {
                    tokio::spawn(run_long_poll(client, tx));
                }
                AsyncAction::MarkAsRead(peer_id) => {
                    tokio::spawn(mark_as_read(client, peer_id, tx));
                }
                AsyncAction::SendPhoto(peer_id, path) => {
                    tokio::spawn(send_photo_attachment(client, peer_id, path, tx));
                }
                AsyncAction::SendDoc(peer_id, path) => {
                    tokio::spawn(send_doc_attachment(client, peer_id, path, tx));
                }
                AsyncAction::DownloadAttachments(atts) => {
                    tokio::spawn(download_attachments(atts, tx));
                }
            }
        }
    });
}

/// Load conversations from VK API
async fn load_conversations(client: Arc<VkClient>, tx: mpsc::UnboundedSender<Message>) {
    match client.get_conversations(0, 50).await {
        Ok(response) => {
            let chats: Vec<Chat> = response
                .items
                .into_iter()
                .map(|item| {
                    let title = get_conversation_title(&item, &response.profiles);
                    let is_online = get_user_online(&item.conversation.peer.id, &response.profiles);

                    Chat {
                        id: item.conversation.peer.id,
                        title,
                        last_message: item.last_message.text.clone(),
                        last_message_time: item.last_message.date,
                        unread_count: item.conversation.unread_count.unwrap_or(0),
                        is_online,
                    }
                })
                .collect();

            let _ = tx.send(Message::ConversationsLoaded(chats, response.profiles));
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load chats: {}", e)));
        }
    }
}

/// Get conversation title from peer info
fn get_conversation_title(item: &vk_api::ConversationItem, profiles: &[User]) -> String {
    // For chat conversations, use chat_settings title
    if let Some(settings) = &item.conversation.chat_settings {
        return settings.title.clone();
    }

    // For user conversations, find user in profiles
    let peer_id = item.conversation.peer.id;
    if peer_id > 0
        && let Some(user) = profiles.iter().find(|u| u.id == peer_id)
    {
        return user.full_name();
    }

    // For groups (negative peer_id)
    if peer_id < 0 {
        return format!("Group {}", -peer_id);
    }

    format!("Chat {}", peer_id)
}

/// Get user online status
fn get_user_online(peer_id: &i64, profiles: &[User]) -> bool {
    if *peer_id > 0 {
        profiles
            .iter()
            .find(|u| u.id == *peer_id)
            .is_some_and(|u| u.is_online())
    } else {
        false
    }
}

/// Map VK attachment to UI attachment info
fn map_attachment(att: vk_api::Attachment) -> AttachmentInfo {
    match att.attachment_type.as_str() {
        "photo" => {
            let best = att
                .photo
                .as_ref()
                .and_then(|p| {
                    p.sizes
                        .iter()
                        .filter_map(|s| {
                            s.url.as_ref().map(|url| {
                                let score = s.width.unwrap_or(0) * s.height.unwrap_or(0);
                                (url.clone(), score as u64)
                            })
                        })
                        .max_by_key(|(_, score)| *score)
                })
                .map(|(url, _)| url);

            AttachmentInfo {
                kind: AttachmentKind::Photo,
                title: "Photo".into(),
                url: best,
                size: None,
            }
        }
        "doc" => {
            let doc = att.doc.unwrap_or_default();
            AttachmentInfo {
                kind: AttachmentKind::File,
                title: doc.title.unwrap_or_else(|| "Document".to_string()),
                url: doc.url,
                size: doc.size,
            }
        }
        other => AttachmentInfo {
            kind: AttachmentKind::Other(other.to_string()),
            title: other.to_string(),
            url: None,
            size: None,
        },
    }
}

/// Load messages from VK API
async fn load_messages(client: Arc<VkClient>, peer_id: i64, tx: mpsc::UnboundedSender<Message>) {
    match client.get_history(peer_id, 0, 50).await {
        Ok(response) => {
            // Messages come in reverse order (newest first), so reverse them
            let messages: Vec<ChatMessage> = response
                .items
                .into_iter()
                .rev()
                .map(|msg| {
                    let from_name = response
                        .profiles
                        .iter()
                        .find(|u| u.id == msg.from_id)
                        .map(|u| u.full_name())
                        .unwrap_or_else(|| {
                            if msg.from_id < 0 {
                                format!("Group {}", -msg.from_id)
                            } else {
                                format!("User {}", msg.from_id)
                            }
                        });

                    let is_outgoing = msg.is_outgoing();
                    let is_read = msg.is_read();
                    let text = if msg.text.is_empty() {
                        "[attachment]".to_string()
                    } else {
                        msg.text
                    };
                    let attachments = msg.attachments.into_iter().map(map_attachment).collect();

                    ChatMessage {
                        id: msg.id,
                        from_id: msg.from_id,
                        from_name,
                        text,
                        timestamp: msg.date,
                        is_outgoing,
                        is_read,
                        delivery: DeliveryStatus::Sent,
                        attachments,
                    }
                })
                .collect();

            let _ = tx.send(Message::MessagesLoaded(messages, response.profiles));
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load messages: {}", e)));
        }
    }
}

/// Send message via VK API
async fn send_message(
    client: Arc<VkClient>,
    peer_id: i64,
    text: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.send_message(peer_id, &text).await {
        Ok(msg_id) => {
            let _ = tx.send(Message::MessageSent(msg_id));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!(
                "Failed to send message: {}",
                e
            )));
        }
    }
}

/// Send photo attachment via VK API
async fn send_photo_attachment(
    client: Arc<VkClient>,
    peer_id: i64,
    path: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.send_photo(peer_id, Path::new(&path)).await {
        Ok(msg_id) => {
            let _ = tx.send(Message::MessageSent(msg_id));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!("Failed to send photo: {}", e)));
        }
    }
}

/// Send document/file attachment via VK API
async fn send_doc_attachment(
    client: Arc<VkClient>,
    peer_id: i64,
    path: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.send_doc(peer_id, Path::new(&path)).await {
        Ok(msg_id) => {
            let _ = tx.send(Message::MessageSent(msg_id));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!("Failed to send file: {}", e)));
        }
    }
}

/// Download attachments to user's downloads directory
async fn download_attachments(atts: Vec<AttachmentInfo>, tx: mpsc::UnboundedSender<Message>) {
    let Some(base_dir) = directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
        .or_else(|| Some(std::env::temp_dir()))
    else {
        let _ = tx.send(Message::Error("No download directory available".into()));
        return;
    };

    if std::fs::create_dir_all(&base_dir).is_err() {
        let _ = tx.send(Message::Error("Failed to create download directory".into()));
        return;
    }

    let client = reqwest::Client::new();

    for (idx, att) in atts.into_iter().enumerate() {
        let Some(url) = att.url.clone() else {
            continue;
        };

        let name = if !att.title.is_empty() {
            att.title.clone()
        } else {
            format!("attachment_{}", idx)
        };

        let path = base_dir.join(name);

        match client.get(&url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    if let Err(e) = std::fs::write(&path, &bytes) {
                        let _ = tx.send(Message::Error(format!(
                            "Failed to save {}: {}",
                            path.display(),
                            e
                        )));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Message::Error(format!("Download failed: {}", e)));
                }
            },
            Err(e) => {
                let _ = tx.send(Message::Error(format!("Download failed: {}", e)));
            }
        }
    }
}

/// Run Long Poll loop for real-time updates
async fn run_long_poll(client: Arc<VkClient>, tx: mpsc::UnboundedSender<Message>) {
    tracing::info!("Starting Long Poll...");

    // Get Long Poll server
    let mut server = match client.get_long_poll_server().await {
        Ok(s) => {
            tracing::info!("Got Long Poll server: {}", s.server);
            s
        }
        Err(e) => {
            tracing::error!("Failed to get Long Poll server: {}", e);
            let _ = tx.send(Message::Error(format!("Long Poll error: {}", e)));
            return;
        }
    };

    let _ = tx.send(Message::VkEvent(VkEvent::ConnectionStatus(true)));

    loop {
        match client.long_poll(&server).await {
            Ok(response) => {
                // Handle failed responses
                if let Some(failed) = response.failed {
                    match failed {
                        1 => {
                            // Update ts
                            if let Some(ts) = response.ts {
                                server.ts = ts;
                            }
                        }
                        2 | 3 => {
                            // Need to get new server
                            match client.get_long_poll_server().await {
                                Ok(new_server) => server = new_server,
                                Err(e) => {
                                    let _ = tx.send(Message::Error(format!(
                                        "Long Poll reconnect error: {}",
                                        e
                                    )));
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                }
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                // Update ts
                if let Some(ts) = response.ts {
                    server.ts = ts;
                }

                // Process updates
                if let Some(updates) = response.updates {
                    tracing::debug!("Got {} updates", updates.len());
                    for update in updates {
                        tracing::trace!("Update: {:?}", update);
                        if let Some(arr) = update.as_array()
                            && let Some(event_type) = arr.first().and_then(|v| v.as_i64())
                        {
                            tracing::debug!("Event type: {}", event_type);
                            match event_type {
                                4 => {
                                    // New message
                                    // Format: [4, message_id, flags, peer_id, timestamp, text, extra, attachments]
                                    // extra is an object with "from" field containing user_id
                                    let peer_id = arr.get(3).and_then(|v| v.as_i64());
                                    let text = arr.get(5).and_then(|v| v.as_str()).unwrap_or("");
                                    let extra = arr.get(6);
                                    tracing::debug!(
                                        "Message event: peer_id={:?}, text={}, extra={:?}",
                                        peer_id,
                                        text,
                                        extra
                                    );

                                    let from_id = extra
                                        .and_then(|v| v.as_object())
                                        .and_then(|obj| obj.get("from"))
                                        .and_then(|v| v.as_str())
                                        .and_then(|s| s.parse::<i64>().ok())
                                        .or(peer_id); // fallback to peer_id for DMs

                                    if let (Some(peer_id), Some(from_id)) = (peer_id, from_id) {
                                        tracing::info!(
                                            "New message from {} in {}: {}",
                                            from_id,
                                            peer_id,
                                            text
                                        );
                                        let _ = tx.send(Message::VkEvent(VkEvent::NewMessage {
                                            peer_id,
                                            text: text.to_string(),
                                            from_id,
                                        }));
                                    }
                                }
                                61 => {
                                    // User typing in private dialog
                                    // Format: [61, user_id, flags]
                                    if let Some(user_id) = arr.get(1).and_then(|v| v.as_i64()) {
                                        let _ = tx.send(Message::VkEvent(VkEvent::UserTyping {
                                            peer_id: user_id,
                                            user_id,
                                        }));
                                    }
                                }
                                62 => {
                                    // User typing in chat
                                    // Format: [62, user_id, chat_id]
                                    if let (Some(user_id), Some(chat_id)) = (
                                        arr.get(1).and_then(|v| v.as_i64()),
                                        arr.get(2).and_then(|v| v.as_i64()),
                                    ) {
                                        let peer_id = 2000000000 + chat_id;
                                        let _ = tx.send(Message::VkEvent(VkEvent::UserTyping {
                                            peer_id,
                                            user_id,
                                        }));
                                    }
                                }
                                6 | 7 => {
                                    // Message read events (incoming/outgoing)
                                    if let Some(peer_id) = arr.get(1).and_then(|v| v.as_i64()) {
                                        let message_id =
                                            arr.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
                                        let _ = tx.send(Message::VkEvent(VkEvent::MessageRead {
                                            peer_id,
                                            message_id,
                                        }));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Message::VkEvent(VkEvent::ConnectionStatus(false)));
                let _ = tx.send(Message::Error(format!("Long Poll error: {}", e)));
                tokio::time::sleep(Duration::from_secs(5)).await;

                // Try to reconnect
                match client.get_long_poll_server().await {
                    Ok(new_server) => {
                        server = new_server;
                        let _ = tx.send(Message::VkEvent(VkEvent::ConnectionStatus(true)));
                    }
                    Err(_) => continue,
                }
            }
        }
    }
}

/// Mark messages as read for a peer
async fn mark_as_read(client: Arc<VkClient>, peer_id: i64, tx: mpsc::UnboundedSender<Message>) {
    if let Err(e) = client.mark_as_read(peer_id).await {
        let _ = tx.send(Message::Error(format!("Failed to mark as read: {}", e)));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup panic hook
    setup_panic_hook();

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create application state
    let mut app = App::new();

    // Create channels for async actions
    let (action_tx, action_rx) = mpsc::unbounded_channel::<AsyncAction>();
    let (message_tx, mut message_rx) = mpsc::unbounded_channel::<Message>();

    app.set_action_tx(action_tx);

    // Spawn action handler with current VK client
    spawn_action_handler(action_rx, message_tx.clone(), app.vk_client.clone());

    // If already authenticated, load conversations
    if app.vk_client.is_some() {
        app.is_loading = true;
        app.send_action(AsyncAction::LoadConversations);
        app.send_action(AsyncAction::StartLongPoll);
    }

    // Create event handler
    let mut events = event::EventHandler::new(Duration::from_millis(100));

    // Main loop
    while app.is_running() {
        // Draw UI
        terminal.draw(|frame| ui::view(&app, frame))?;

        // Handle events
        tokio::select! {
            event = events.next() => {
                match event? {
                    Event::Tick => {
                        // Periodic updates
                    }
                    Event::Key(key) => {
                        // Determine if we're in input mode
                        let in_input = app.screen == app::Screen::Auth
                            || (app.screen == app::Screen::Main && app.focus == app::Focus::Input);
                        let msg = Message::from_key_event(key, in_input);
                        let mut current_msg = Some(msg);

                        // Process message chain
                        while let Some(msg) = current_msg {
                            current_msg = update(&mut app, msg);
                        }

                        // If we just authenticated, restart action handler with new client
                        if app.vk_client.is_some() && !app.is_loading && app.chats.is_empty() {
                            let (new_action_tx, new_action_rx) = mpsc::unbounded_channel();
                            app.set_action_tx(new_action_tx);
                            spawn_action_handler(new_action_rx, message_tx.clone(), app.vk_client.clone());
                            app.is_loading = true;
                            app.send_action(AsyncAction::LoadConversations);
                            app.send_action(AsyncAction::StartLongPoll);
                        }
                    }
                    Event::Mouse(_) => {}
                    Event::Resize(_, _) => {}
                    Event::Vk(vk_event) => {
                        update(&mut app, Message::VkEvent(vk_event));
                    }
                }
            }
            Some(msg) = message_rx.recv() => {
                update(&mut app, msg);
            }
        }
    }

    // Restore terminal
    restore_terminal(&mut terminal)?;

    Ok(())
}
