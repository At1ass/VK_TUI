mod actions;
mod app;
mod commands;
mod event;
mod input;
mod longpoll;
mod mapper;
mod message;
mod state;
mod ui;
mod update;

use std::io;
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

use event::{Event, VkEvent};
use longpoll::handle_update;
use message::Message;
use state::{App, AsyncAction, Screen};
use update::update;
use vk_api::{User, VkClient};

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
                    tokio::spawn(actions::load_conversations(client, tx));
                }
                AsyncAction::LoadMessages(peer_id) => {
                    tokio::spawn(actions::load_messages(client, peer_id, tx));
                }
                AsyncAction::SendMessage(peer_id, text) => {
                    tokio::spawn(actions::send_message(client, peer_id, text, tx));
                }
                AsyncAction::SendForward(peer_id, ids, comment) => {
                    tokio::spawn(actions::send_forward(client, peer_id, ids, comment, tx));
                }
                AsyncAction::StartLongPoll => {
                    tokio::spawn(run_long_poll(client, tx));
                }
                AsyncAction::MarkAsRead(peer_id) => {
                    tokio::spawn(mark_as_read(client, peer_id, tx));
                }
                AsyncAction::SendPhoto(peer_id, path) => {
                    tokio::spawn(actions::send_photo_attachment(client, peer_id, path, tx));
                }
                AsyncAction::SendDoc(peer_id, path) => {
                    tokio::spawn(actions::send_doc_attachment(client, peer_id, path, tx));
                }
                AsyncAction::DownloadAttachments(atts) => {
                    tokio::spawn(actions::download_attachments(atts, tx));
                }
                AsyncAction::EditMessage(peer_id, message_id, cmid, text) => {
                    tokio::spawn(actions::edit_message(
                        client, peer_id, message_id, cmid, text, tx,
                    ));
                }
                AsyncAction::DeleteMessage(_peer_id, msg_id, delete_for_all) => {
                    tokio::spawn(actions::delete_message(client, msg_id, delete_for_all, tx));
                }
                AsyncAction::FetchMessageById(msg_id) => {
                    tokio::spawn(actions::fetch_message_by_id(client, msg_id, tx));
                }
            }
        }
    });
}

/// Load conversations from VK API
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

// mapping helpers moved to mapper.rs

/// Load messages from VK API
/// Run Long Poll loop for real-time updates
async fn run_long_poll(client: Arc<VkClient>, tx: mpsc::UnboundedSender<Message>) {
    tracing::info!("Starting Long Poll...");
    let mut backoff = Duration::from_secs(1);

    // Get Long Poll server
    let mut server = match client.longpoll().get_server().await {
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
        match client.longpoll().poll(&server).await {
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
                        2..=4 => {
                            // Need to get new server
                            match client.longpoll().get_server().await {
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
                        if let Some(event) = handle_update(&update) {
                            let _ = tx.send(Message::VkEvent(event));
                        }
                    }
                }
                backoff = Duration::from_secs(1);
            }
            Err(e) => {
                let _ = tx.send(Message::VkEvent(VkEvent::ConnectionStatus(false)));
                let _ = tx.send(Message::Error(format!("Long Poll error: {}", e)));
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));

                // Try to reconnect
                match client.longpoll().get_server().await {
                    Ok(new_server) => {
                        server = new_server;
                        let _ = tx.send(Message::VkEvent(VkEvent::ConnectionStatus(true)));
                        backoff = Duration::from_secs(1);
                    }
                    Err(_) => continue,
                }
            }
        }
    }
}

/// Mark messages as read for a peer
async fn mark_as_read(client: Arc<VkClient>, peer_id: i64, tx: mpsc::UnboundedSender<Message>) {
    if let Err(e) = client.messages().mark_as_read(peer_id).await {
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
                        // Auth screen uses dedicated input handling; main screen uses modes
                        let msg = if app.screen == Screen::Auth {
                            Message::from_auth_key_event(key)
                        } else if let Some(fwd) = &app.forward {
                            Message::from_forward_key_event(key, fwd.stage.clone())
                        } else if app.forward_view.is_some() {
                            Message::from_forward_view_key_event(key)
                        } else {
                            Message::from_key_event(key, app.mode, app.focus, app.show_help)
                        };
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
