//! Main application state and logic.

use std::collections::HashMap;
use std::sync::Arc;

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{
    Border, Color, Element, Font, Length, Shadow, Subscription, Task, Theme, Vector, font,
    font::{Family, Stretch, Style, Weight},
    widget::{button as button_widget, container as container_widget, text_input as input_widget},
};
use tokio::sync::mpsc;
use vk_api::auth::AuthManager;
use vk_api::{User, VkClient};
use vk_core::{
    AsyncCommand, Chat, ChatMessage, ChatsPagination, CommandExecutor, CoreEvent, DeliveryStatus,
    MessagesPagination, VkEvent,
};

use crate::message::Message;

const COSMIC_BG: Color = rgb8(12, 14, 20);
const COSMIC_SURFACE: Color = rgb8(18, 22, 32);
const COSMIC_SURFACE_ALT: Color = rgb8(26, 31, 44);
const COSMIC_BORDER: Color = rgb8(42, 50, 67);
const COSMIC_TEXT: Color = rgb8(231, 235, 242);
const COSMIC_MUTED: Color = rgb8(151, 160, 178);
const COSMIC_ACCENT: Color = rgb8(88, 170, 255);
const COSMIC_SUCCESS: Color = rgb8(92, 209, 147);
const COSMIC_DANGER: Color = rgb8(255, 122, 122);
const COSMIC_SELECTION: Color = rgb8(65, 92, 140);

const JETBRAINS_FONT_NAME: &str = "JetBrainsMono Nerd Font";
const JETBRAINS_BYTES: &[u8] = include_bytes!("../assets/JetBrainsMono.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForwardStage {
    SelectTarget,
    EnterComment,
}

/// Current view/screen.
#[derive(Debug, Clone, Default)]
pub enum View {
    #[default]
    Auth,
    Main,
}

/// Connection state.
#[derive(Debug, Clone, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

/// Main application state.
pub struct VkApp {
    // View state
    view: View,
    connection: ConnectionState,

    // Auth
    auth: AuthManager,
    token_input: String,

    // VK state
    vk_client: Option<Arc<VkClient>>,
    users: HashMap<i64, User>,

    // Chat data
    chats: Vec<Chat>,
    selected_chat: usize,
    current_peer_id: Option<i64>,

    // Messages
    messages: Vec<ChatMessage>,
    selected_message: usize,
    message_input: String,

    // Pagination
    chats_pagination: ChatsPagination,
    messages_pagination: Option<MessagesPagination>,

    // Reply state
    reply_to: Option<i64>,
    editing_message: Option<i64>,
    forward_source: Option<i64>,
    forward_target: Option<i64>,
    forward_stage: Option<ForwardStage>,
    forward_comment: String,
    delete_prompt: Option<i64>,
    font_loaded: bool,

    // Status
    status: Option<String>,

    // Command channel
    command_tx: Option<mpsc::UnboundedSender<AsyncCommand>>,
    event_rx: Option<mpsc::UnboundedReceiver<CoreEvent>>,
}

impl Default for VkApp {
    fn default() -> Self {
        Self {
            view: View::Auth,
            connection: ConnectionState::Disconnected,
            auth: AuthManager::default(),
            token_input: String::new(),
            vk_client: None,
            users: HashMap::new(),
            chats: Vec::new(),
            selected_chat: 0,
            current_peer_id: None,
            messages: Vec::new(),
            selected_message: 0,
            message_input: String::new(),
            chats_pagination: ChatsPagination::default(),
            messages_pagination: None,
            reply_to: None,
            editing_message: None,
            forward_source: None,
            forward_target: None,
            forward_stage: None,
            forward_comment: String::new(),
            delete_prompt: None,
            font_loaded: false,
            status: None,
            command_tx: None,
            event_rx: None,
        }
    }
}

impl VkApp {
    /// Create new application with initial command.
    pub fn new() -> (Self, Task<Message>) {
        let mut app = Self::default();
        let font_task = font::load(JETBRAINS_BYTES)
            .map(|res: Result<(), font::Error>| Message::FontLoaded(res.is_ok()));
        let mut tasks = vec![font_task];

        // Check for existing token
        if let Some(token) = app.auth.access_token().map(|t| t.to_string()) {
            app.token_input = token.clone();
            if app.auth.is_token_expired() {
                let _ = app.auth.logout();
                app.status = Some("Session expired. Please login again.".into());
            } else {
                app.connection = ConnectionState::Connecting;
                app.status = Some("Validating session...".into());
                tasks.push(Task::perform(
                    Self::validate_token(token.clone()),
                    move |result| Message::SessionValidated {
                        token: token.clone(),
                        valid: result.is_ok(),
                        error: result.err(),
                    },
                ));
            }
        }

        (app, Task::batch(tasks))
    }

    /// Update application state based on message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // === Auth ===
            Message::TokenInputChanged(token) => {
                self.token_input = token;
                Task::none()
            }
            Message::OpenAuthUrl => {
                let url = AuthManager::get_auth_url();
                if let Err(e) = open::that(&url) {
                    self.status = Some(format!("Failed to open browser: {}", e));
                } else {
                    self.status = Some("Opened OAuth URL. Paste redirect URL below.".into());
                }
                Task::none()
            }
            Message::LoginPressed => {
                if self.token_input.is_empty() {
                    self.status = Some("Please paste the redirect URL".into());
                    return Task::none();
                }

                let input = self.token_input.trim().to_string();
                let token = if looks_like_oauth_url(&input) {
                    match self.auth.save_token_from_url(&input) {
                        Ok(()) => self.auth.access_token().map(|t| t.to_string()),
                        Err(e) => {
                            self.status = Some(format!("Invalid redirect URL: {}", e));
                            return Task::none();
                        }
                    }
                } else {
                    self.status = Some("Please paste the full redirect URL".into());
                    return Task::none();
                };

                let Some(token) = token else {
                    self.status = Some("Token not found in redirect URL".into());
                    return Task::none();
                };

                self.token_input = token.clone();
                tracing::info!("Login pressed, token length: {}", token.len());
                self.connection = ConnectionState::Connecting;
                self.status = Some("Validating session...".into());
                Task::perform(Self::validate_token(token.clone()), move |result| {
                    Message::SessionValidated {
                        token: token.clone(),
                        valid: result.is_ok(),
                        error: result.err(),
                    }
                })
            }

            // === Core Events ===
            Message::CoreEvent(event) => {
                tracing::debug!("Received core event: {:?}", std::mem::discriminant(&event));
                self.handle_core_event(event.clone());
                Task::none()
            }

            // === Chat Navigation ===
            Message::ChatSelected(idx) => {
                self.selected_chat = idx;
                if let Some(chat) = self.chats.get(idx) {
                    let peer_id = chat.id;
                    self.current_peer_id = Some(peer_id);
                    self.messages.clear();
                    self.selected_message = 0;
                    self.messages_pagination = Some(MessagesPagination::new(peer_id));
                    self.send_command(AsyncCommand::LoadMessages { peer_id, offset: 0 });

                    if let Some(source_id) = self.forward_source
                        && matches!(self.forward_stage, Some(ForwardStage::SelectTarget))
                    {
                        self.forward_target = Some(peer_id);
                        self.forward_stage = Some(ForwardStage::EnterComment);
                        self.status = Some("Enter forward comment".into());
                        // Keep selection so user can type comment
                        self.forward_source = Some(source_id);
                    }
                }
                Task::none()
            }

            // === Messaging ===
            Message::MessageInputChanged(input) => {
                self.message_input = input;
                Task::none()
            }
            Message::MessageSelected(idx) => {
                if idx < self.messages.len() {
                    self.selected_message = idx;
                }
                Task::none()
            }
            Message::ReplyPressed(message_id) => {
                self.reply_to = Some(message_id);
                Task::none()
            }
            Message::ForwardPressed(message_id) => {
                self.forward_source = Some(message_id);
                self.forward_target = None;
                self.forward_stage = Some(ForwardStage::SelectTarget);
                self.forward_comment.clear();
                self.status = Some("Select target chat to forward".into());
                Task::none()
            }
            Message::EditPressed(message_id) => {
                if let Some(msg) = self.messages.iter().find(|m| m.id == message_id) {
                    self.editing_message = Some(message_id);
                    self.message_input = msg.text.clone();
                }
                Task::none()
            }
            Message::DeletePressed(message_id) => {
                self.delete_prompt = Some(message_id);
                Task::none()
            }
            Message::DeleteForMe(message_id) => {
                if let Some(peer_id) = self.current_peer_id {
                    self.send_command(AsyncCommand::DeleteMessage {
                        peer_id,
                        message_id,
                        for_all: false,
                    });
                }
                self.delete_prompt = None;
                Task::none()
            }
            Message::DeleteForAll(message_id) => {
                if let Some(peer_id) = self.current_peer_id {
                    self.send_command(AsyncCommand::DeleteMessage {
                        peer_id,
                        message_id,
                        for_all: true,
                    });
                }
                self.delete_prompt = None;
                Task::none()
            }
            Message::CancelDelete => {
                self.delete_prompt = None;
                Task::none()
            }
            Message::SendPressed => {
                if let Some(peer_id) = self.current_peer_id {
                    let input = std::mem::take(&mut self.message_input);
                    if !input.is_empty() {
                        if let Some(message_id) = self.editing_message.take() {
                            let cmid = self
                                .messages
                                .iter()
                                .find(|m| m.id == message_id)
                                .and_then(|m| m.cmid);
                            self.send_command(AsyncCommand::EditMessage {
                                peer_id,
                                message_id,
                                cmid,
                                text: input,
                            });
                        } else if let Some(reply_to) = self.reply_to.take() {
                            self.send_command(AsyncCommand::SendReply {
                                peer_id,
                                reply_to,
                                text: input,
                            });
                        } else {
                            self.send_command(AsyncCommand::SendMessage {
                                peer_id,
                                text: input,
                            });
                        }
                    }
                }
                Task::none()
            }

            Message::CancelReply => {
                self.reply_to = None;
                Task::none()
            }
            Message::CancelEdit => {
                self.editing_message = None;
                Task::none()
            }
            Message::CancelForward => {
                self.forward_source = None;
                self.forward_target = None;
                self.forward_stage = None;
                self.forward_comment.clear();
                Task::none()
            }
            Message::FontLoaded(loaded) => {
                if loaded {
                    self.font_loaded = true;
                } else {
                    self.status = Some("Failed to load embedded font".into());
                }
                Task::none()
            }
            Message::SessionValidated {
                token,
                valid,
                error,
            } => {
                if valid {
                    self.start_session(token);
                } else if let Some(err) = error {
                    if is_auth_error(&err) {
                        let _ = self.auth.logout();
                        self.status = Some("Session expired. Please login again.".into());
                    } else {
                        self.status = Some(err);
                    }
                    self.connection = ConnectionState::Disconnected;
                }
                Task::none()
            }
            Message::ForwardCommentChanged(comment) => {
                self.forward_comment = comment;
                Task::none()
            }
            Message::ForwardSubmit => {
                if let (Some(source_id), Some(peer_id)) = (self.forward_source, self.forward_target)
                {
                    let comment = std::mem::take(&mut self.forward_comment);
                    if !comment.trim().is_empty() {
                        self.send_command(AsyncCommand::SendForward {
                            peer_id,
                            message_ids: vec![source_id],
                            comment,
                        });
                        self.forward_source = None;
                        self.forward_target = None;
                        self.forward_stage = None;
                        self.status = Some("Forwarded message".into());
                    } else {
                        self.forward_comment = comment;
                        self.status = Some("Comment is required".into());
                    }
                }
                Task::none()
            }
            Message::Tick => {
                if let Some(rx) = &mut self.event_rx {
                    let mut events = Vec::new();
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                    for event in events {
                        self.handle_core_event(event);
                    }
                }
                Task::none()
            }

            Message::Error(e) => {
                tracing::error!("Error: {}", e);
                self.status = Some(e);
                self.connection = ConnectionState::Disconnected;
                Task::none()
            }

            // Unhandled messages
            _ => Task::none(),
        }
    }

    /// Run command executor - this processes one command and returns the result.
    async fn run_long_poll(client: Arc<VkClient>, event_tx: mpsc::UnboundedSender<CoreEvent>) {
        tracing::info!("Starting Long Poll...");
        let mut backoff = std::time::Duration::from_secs(1);

        let mut server = match client.longpoll().get_server().await {
            Ok(s) => {
                tracing::info!("Got Long Poll server: {}", s.server);
                s
            }
            Err(e) => {
                let _ = event_tx.send(CoreEvent::Error(format!("Long Poll error: {}", e)));
                return;
            }
        };

        let _ = event_tx.send(CoreEvent::VkEvent(VkEvent::ConnectionStatus(true)));

        loop {
            match client.longpoll().poll(&server).await {
                Ok(response) => {
                    if let Some(failed) = response.failed {
                        match failed {
                            1 => {
                                if let Some(ts) = response.ts {
                                    server.ts = ts;
                                }
                            }
                            2..=4 => match client.longpoll().get_server().await {
                                Ok(new_server) => server = new_server,
                                Err(e) => {
                                    let _ = event_tx
                                        .send(CoreEvent::Error(format!("Long Poll error: {}", e)));
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                }
                            },
                            _ => {}
                        }
                        continue;
                    }

                    if let Some(ts) = response.ts {
                        server.ts = ts;
                    }

                    if let Some(updates) = response.updates {
                        for update in updates {
                            if let Some(event) = vk_core::longpoll::handle_update(&update) {
                                let _ = event_tx.send(CoreEvent::VkEvent(event));
                            }
                        }
                    }
                    backoff = std::time::Duration::from_secs(1);
                }
                Err(e) => {
                    let _ = event_tx.send(CoreEvent::VkEvent(VkEvent::ConnectionStatus(false)));
                    let _ = event_tx.send(CoreEvent::Error(format!("Long Poll error: {}", e)));
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(std::time::Duration::from_secs(30));

                    match client.longpoll().get_server().await {
                        Ok(new_server) => {
                            server = new_server;
                            let _ =
                                event_tx.send(CoreEvent::VkEvent(VkEvent::ConnectionStatus(true)));
                            backoff = std::time::Duration::from_secs(1);
                        }
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    /// Handle events from vk-core.
    fn handle_core_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::ConversationsLoaded {
                chats,
                profiles,
                total_count,
                has_more,
            } => {
                tracing::info!("Handling ConversationsLoaded: {} chats", chats.len());
                self.chats = chats;
                for profile in profiles {
                    self.users.insert(profile.id, profile);
                }
                self.chats_pagination.total_count = Some(total_count);
                self.chats_pagination.has_more = has_more;
                self.chats_pagination.is_loading = false;
                self.view = View::Main;
                self.connection = ConnectionState::Connected;
                self.status = None;
            }
            CoreEvent::MessagesLoaded {
                peer_id,
                messages,
                profiles,
                total_count,
                has_more,
            } => {
                if Some(peer_id) == self.current_peer_id {
                    self.messages = messages;
                    for profile in profiles {
                        self.users.insert(profile.id, profile);
                    }
                    if let Some(ref mut pagination) = self.messages_pagination {
                        pagination.total_count = Some(total_count);
                        pagination.has_more = has_more;
                        pagination.is_loading = false;
                    }
                }
            }
            CoreEvent::MessageSent { .. } => {
                // Reload messages
                if let Some(peer_id) = self.current_peer_id {
                    self.send_command(AsyncCommand::LoadMessages { peer_id, offset: 0 });
                }
            }
            CoreEvent::MessageEdited { .. } | CoreEvent::MessageDeleted { .. } => {
                if let Some(peer_id) = self.current_peer_id {
                    self.send_command(AsyncCommand::LoadMessages { peer_id, offset: 0 });
                }
            }
            CoreEvent::MessageDetailsFetched {
                message_id,
                text,
                is_edited,
                attachments,
                reply,
                fwd_count,
                forwards,
                ..
            } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    if let Some(text) = text {
                        msg.text = text;
                    }
                    msg.is_edited = is_edited;
                    if let Some(attachments) = attachments {
                        msg.attachments = attachments;
                    }
                    if let Some(reply) = reply {
                        msg.reply = Some(reply);
                    }
                    if let Some(count) = fwd_count {
                        msg.fwd_count = count;
                    }
                    if let Some(forwards) = forwards {
                        msg.forwards = forwards;
                    }
                }
            }
            CoreEvent::Error(msg) => {
                self.status = Some(msg);
            }
            CoreEvent::SendFailed(msg) => {
                self.status = Some(format!("Send failed: {}", msg));
            }
            CoreEvent::VkEvent(event) => {
                self.handle_vk_event(event);
            }
            _ => {}
        }
    }

    fn handle_vk_event(&mut self, event: VkEvent) {
        match event {
            VkEvent::NewMessage {
                message_id,
                peer_id,
                timestamp,
                text,
                from_id,
            } => {
                if self.current_peer_id == Some(peer_id) {
                    let from_name = self.get_user_name(from_id);
                    self.messages.push(ChatMessage {
                        id: message_id,
                        cmid: None,
                        from_id,
                        from_name,
                        text,
                        timestamp,
                        is_outgoing: from_id == self.auth.user_id().unwrap_or(0),
                        is_read: true,
                        is_edited: false,
                        is_pinned: false,
                        delivery: DeliveryStatus::Sent,
                        attachments: Vec::new(),
                        reply: None,
                        fwd_count: 0,
                        forwards: Vec::new(),
                    });
                    self.selected_message = self.messages.len().saturating_sub(1);
                } else if let Some(chat) = self.chats.iter_mut().find(|c| c.id == peer_id) {
                    chat.unread_count += 1;
                }
            }
            VkEvent::MessageRead {
                peer_id,
                message_id,
            } => {
                if let Some(chat) = self.chats.iter_mut().find(|c| c.id == peer_id) {
                    chat.unread_count = 0;
                }
                if self.current_peer_id == Some(peer_id) {
                    if message_id > 0 {
                        for msg in self.messages.iter_mut() {
                            if msg.is_outgoing && msg.id <= message_id {
                                msg.is_read = true;
                                msg.delivery = DeliveryStatus::Sent;
                            }
                        }
                    } else {
                        for msg in self.messages.iter_mut().filter(|m| m.is_outgoing) {
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
                if self.current_peer_id == Some(peer_id) {
                    self.send_command(AsyncCommand::FetchMessageById { message_id });
                    self.status = Some("Message updated from web".into());
                }
            }
            VkEvent::MessageDeletedFromLongPoll {
                peer_id,
                message_id,
            } => {
                if self.current_peer_id == Some(peer_id)
                    && let Some(pos) = self.messages.iter().position(|m| m.id == message_id)
                {
                    self.messages.remove(pos);
                    if self.selected_message >= self.messages.len() && self.selected_message > 0 {
                        self.selected_message -= 1;
                    }
                    self.status = Some("Message deleted from web".into());
                }
            }
            VkEvent::UserTyping { peer_id, user_id } => {
                if self.current_peer_id == Some(peer_id) {
                    let name = self.get_user_name(user_id);
                    self.status = Some(format!("{} is typing...", name));
                }
            }
            VkEvent::ConnectionStatus(connected) => {
                self.status = Some(if connected {
                    "Connected to VK".into()
                } else {
                    "Disconnected from VK".into()
                });
            }
        }
    }

    fn start_session(&mut self, token: String) {
        let client = Arc::new(VkClient::new(token));
        self.vk_client = Some(client.clone());

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AsyncCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<CoreEvent>();
        self.command_tx = Some(cmd_tx);
        self.event_rx = Some(event_rx);

        let executor = CommandExecutor::new(client.clone(), event_tx.clone());
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                executor.execute(cmd).await;
            }
        });

        tokio::spawn(Self::run_long_poll(client, event_tx));
        self.send_command(AsyncCommand::LoadConversations { offset: 0 });
    }

    async fn validate_token(token: String) -> Result<(), String> {
        let client = VkClient::new(token);
        client
            .account()
            .get_profile_info()
            .await
            .map(|_| ())
            .map_err(|e| format!("Session validation failed: {}", e))
    }

    /// Send command to executor.
    fn send_command(&self, cmd: AsyncCommand) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(cmd);
        }
    }

    /// Create subscription for periodic updates.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(200)).map(|_| Message::Tick)
    }

    /// Get theme.
    pub fn theme(&self) -> Theme {
        Theme::custom(
            "Cosmic Dark".to_string(),
            iced::theme::Palette {
                background: COSMIC_BG,
                text: COSMIC_TEXT,
                primary: COSMIC_ACCENT,
                success: COSMIC_SUCCESS,
                danger: COSMIC_DANGER,
            },
        )
    }

    /// Render the view.
    pub fn view(&self) -> Element<'_, Message> {
        match &self.view {
            View::Auth => self.view_auth(),
            View::Main => self.view_main(),
        }
    }

    /// Render auth screen.
    fn view_auth(&self) -> Element<'_, Message> {
        let title = text("VK Client")
            .size(32)
            .font(self.font_ui_bold())
            .color(COSMIC_TEXT);

        let status_text = match &self.connection {
            ConnectionState::Connecting => text("Connecting...").size(14).font(self.font_ui()),
            ConnectionState::Connected => text("Connected").size(14).font(self.font_ui()),
            ConnectionState::Disconnected => {
                if let Some(status) = &self.status {
                    text(status).size(14).font(self.font_ui())
                } else {
                    text("Paste the redirect URL from the browser")
                        .size(14)
                        .font(self.font_ui())
                }
            }
        };

        let token_input = text_input("Paste redirect URL...", &self.token_input)
            .on_input(Message::TokenInputChanged)
            .on_submit(Message::LoginPressed)
            .style(cosmic_text_input)
            .padding(10)
            .width(Length::Fixed(400.0));

        let login_button = button(text("Login").font(self.font_ui_bold()))
            .on_press(Message::LoginPressed)
            .style(cosmic_button_primary)
            .padding([10, 20]);

        let open_button = button(text("Open OAuth URL").font(self.font_ui_bold()))
            .on_press(Message::OpenAuthUrl)
            .style(cosmic_button_secondary)
            .padding([10, 20]);

        let help_text = text("Authorize in browser, then paste redirect URL here")
            .size(12)
            .font(self.font_ui())
            .color(COSMIC_MUTED);

        let content = column![
            title,
            status_text,
            token_input,
            row![open_button, login_button].spacing(12),
            help_text
        ]
        .spacing(20)
        .align_x(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(cosmic_root)
            .into()
    }

    /// Render main screen.
    fn view_main(&self) -> Element<'_, Message> {
        let sidebar = self.view_chat_list();
        let content = self.view_conversation();
        let header = self.view_header();

        container(column![header, row![sidebar, content].height(Length::Fill)])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(cosmic_root)
            .into()
    }

    /// Render chat list sidebar.
    fn view_chat_list(&self) -> Element<'_, Message> {
        let chats: Vec<Element<'_, Message>> = self
            .chats
            .iter()
            .enumerate()
            .map(|(idx, chat)| {
                let is_selected = idx == self.selected_chat;

                let title_text = if chat.unread_count > 0 {
                    format!("{} ({})", chat.title, chat.unread_count)
                } else {
                    chat.title.clone()
                };

                let title = text(title_text).size(14).font(self.font_ui_bold());

                let preview_text = truncate_text(&chat.last_message, 30);
                let preview = text(preview_text)
                    .size(12)
                    .font(self.font_ui())
                    .color(COSMIC_MUTED);

                let online_indicator = if chat.is_online {
                    text(" ●").size(12).color(COSMIC_SUCCESS)
                } else {
                    text("").size(12)
                };

                let chat_row = column![row![title, online_indicator], preview].spacing(4);

                let btn = button(chat_row)
                    .on_press(Message::ChatSelected(idx))
                    .width(Length::Fill)
                    .padding(10)
                    .style(move |theme, status| cosmic_chat_button(theme, status, is_selected));

                btn.into()
            })
            .collect();

        let chat_list = scrollable(Column::with_children(chats).spacing(6)).height(Length::Fill);

        container(chat_list)
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
            .padding(6)
            .style(cosmic_sidebar)
            .into()
    }

    /// Render conversation view.
    fn view_conversation(&self) -> Element<'_, Message> {
        if self.current_peer_id.is_none() {
            return container(text("Select a chat").size(16).font(self.font_ui_bold()))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(cosmic_panel)
                .into();
        }

        let messages: Vec<Element<'_, Message>> = self
            .messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| {
                let is_selected = idx == self.selected_message;

                let from = text(&msg.from_name).size(12).font(self.font_ui_bold());
                let content_text = text(&msg.text).size(14).font(self.font_ui());

                let time = format_timestamp(msg.timestamp);
                let time_text = text(time).size(10).font(self.font_ui()).color(COSMIC_MUTED);

                let status = if msg.is_outgoing {
                    if msg.is_read {
                        text("✓✓").size(10).font(self.font_ui()).color(COSMIC_MUTED)
                    } else {
                        text("✓").size(10).font(self.font_ui()).color(COSMIC_MUTED)
                    }
                } else {
                    text("").size(10)
                };

                let msg_content =
                    column![row![from, time_text].spacing(10), content_text, status].spacing(4);

                let btn = button(msg_content)
                    .on_press(Message::MessageSelected(idx))
                    .width(Length::Fill)
                    .padding(10)
                    .style(move |theme, status| {
                        cosmic_message_button(theme, status, is_selected, msg.is_outgoing)
                    });

                btn.into()
            })
            .collect();

        let messages_view =
            scrollable(Column::with_children(messages).spacing(8)).height(Length::Fill);

        let selected_msg = self.messages.get(self.selected_message);
        let action_row = if let Some(msg) = selected_msg {
            let reply_btn = button(text("Reply").font(self.font_ui_bold()))
                .on_press(Message::ReplyPressed(msg.id))
                .style(cosmic_button_secondary);
            let forward_btn = button(text("Forward").font(self.font_ui_bold()))
                .on_press(Message::ForwardPressed(msg.id))
                .style(cosmic_button_secondary);
            let delete_btn = button(text("Delete").font(self.font_ui_bold()))
                .on_press(Message::DeletePressed(msg.id))
                .style(cosmic_button_danger);
            let edit_btn = if msg.is_outgoing {
                button(text("Edit").font(self.font_ui_bold()))
                    .on_press(Message::EditPressed(msg.id))
                    .style(cosmic_button_secondary)
            } else {
                button(text("Edit").font(self.font_ui_bold())).style(cosmic_button_secondary)
            };
            row![reply_btn, forward_btn, edit_btn, delete_btn].spacing(10)
        } else {
            row![]
        };

        let delete_row = if let Some(message_id) = self.delete_prompt {
            row![
                text("Delete message?")
                    .size(12)
                    .font(self.font_ui())
                    .color(COSMIC_MUTED),
                button(text("For me").font(self.font_ui_bold()))
                    .on_press(Message::DeleteForMe(message_id))
                    .style(cosmic_button_secondary)
                    .padding(6),
                button(text("For all").font(self.font_ui_bold()))
                    .on_press(Message::DeleteForAll(message_id))
                    .style(cosmic_button_primary)
                    .padding(6),
                button(text("Cancel").font(self.font_ui_bold()))
                    .on_press(Message::CancelDelete)
                    .style(cosmic_button_secondary)
                    .padding(6),
            ]
            .spacing(10)
        } else {
            row![]
        };

        // Reply indicator
        let reply_row = if let Some(reply_id) = self.reply_to {
            let reply_msg = self.messages.iter().find(|m| m.id == reply_id);
            let reply_text = reply_msg
                .map(|m| format!("Reply to: {}", truncate_text(&m.text, 30)))
                .unwrap_or_else(|| format!("Reply to message #{}", reply_id));

            row![
                text(reply_text)
                    .size(12)
                    .font(self.font_ui())
                    .color(COSMIC_MUTED),
                button(text("✕").size(12).font(self.font_ui_bold()))
                    .on_press(Message::CancelReply)
                    .style(cosmic_button_secondary)
                    .padding(4),
            ]
            .spacing(10)
        } else {
            row![]
        };

        let edit_row = if let Some(message_id) = self.editing_message {
            let edit_msg = self.messages.iter().find(|m| m.id == message_id);
            let edit_text = edit_msg
                .map(|m| format!("Editing: {}", truncate_text(&m.text, 30)))
                .unwrap_or_else(|| format!("Editing message #{}", message_id));
            row![
                text(edit_text)
                    .size(12)
                    .font(self.font_ui())
                    .color(COSMIC_MUTED),
                button(text("✕").size(12).font(self.font_ui_bold()))
                    .on_press(Message::CancelEdit)
                    .style(cosmic_button_secondary)
                    .padding(4),
            ]
            .spacing(10)
        } else {
            row![]
        };

        let forward_row = match self.forward_stage {
            Some(ForwardStage::SelectTarget) => row![
                text("Select target chat to forward")
                    .size(12)
                    .font(self.font_ui())
                    .color(COSMIC_MUTED),
                button(text("✕").size(12).font(self.font_ui_bold()))
                    .on_press(Message::CancelForward)
                    .style(cosmic_button_secondary)
                    .padding(4),
            ]
            .spacing(10),
            Some(ForwardStage::EnterComment) => {
                let comment_input =
                    text_input("Forward comment (required)...", &self.forward_comment)
                        .on_input(Message::ForwardCommentChanged)
                        .on_submit(Message::ForwardSubmit)
                        .style(cosmic_text_input)
                        .padding(8)
                        .width(Length::Fill);
                let send_btn = if self.forward_comment.trim().is_empty() {
                    button(text("Send forward").font(self.font_ui_bold()))
                        .style(cosmic_button_secondary)
                } else {
                    button(text("Send forward").font(self.font_ui_bold()))
                        .on_press(Message::ForwardSubmit)
                        .style(cosmic_button_primary)
                };
                row![
                    comment_input,
                    send_btn,
                    button(text("Cancel").font(self.font_ui_bold()))
                        .on_press(Message::CancelForward)
                        .style(cosmic_button_secondary)
                        .padding(6),
                ]
                .spacing(10)
            }
            None => row![],
        };

        // Input area
        let input = text_input("Type a message...", &self.message_input)
            .on_input(Message::MessageInputChanged)
            .on_submit(Message::SendPressed)
            .style(cosmic_text_input)
            .padding(10)
            .width(Length::Fill);

        let send_btn = button(text("Send").font(self.font_ui_bold()))
            .on_press(Message::SendPressed)
            .style(cosmic_button_primary)
            .padding([10, 20]);

        let input_row = row![input, send_btn].spacing(10);

        let content = column![
            messages_view,
            action_row,
            delete_row,
            reply_row,
            edit_row,
            forward_row,
            input_row
        ]
        .spacing(10)
        .padding(10);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(cosmic_panel)
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let title = text("Messages")
            .size(18)
            .font(self.font_ui_bold())
            .color(COSMIC_TEXT);
        let status = self.status.as_deref().unwrap_or("Ready");
        let status_text = text(status)
            .size(12)
            .font(self.font_ui())
            .color(COSMIC_MUTED);

        let content = row![title, status_text]
            .spacing(16)
            .align_y(iced::Alignment::Center);

        container(content)
            .padding(12)
            .style(cosmic_header)
            .width(Length::Fill)
            .into()
    }

    fn font_ui(&self) -> Font {
        if self.font_loaded {
            Font::with_name(JETBRAINS_FONT_NAME)
        } else {
            Font::DEFAULT
        }
    }

    fn font_ui_bold(&self) -> Font {
        if self.font_loaded {
            Font {
                family: Family::Name(JETBRAINS_FONT_NAME),
                weight: Weight::Semibold,
                stretch: Stretch::Normal,
                style: Style::Normal,
            }
        } else {
            Font {
                weight: Weight::Semibold,
                stretch: Stretch::Normal,
                style: Style::Normal,
                ..Font::DEFAULT
            }
        }
    }

    fn get_user_name(&self, user_id: i64) -> String {
        if let Some(user) = self.users.get(&user_id) {
            user.full_name()
        } else if user_id < 0 {
            format!("Group {}", -user_id)
        } else {
            format!("User {}", user_id)
        }
    }
}

fn format_timestamp(timestamp: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp as u64);
    let now = std::time::SystemTime::now();

    if let Ok(duration) = now.duration_since(datetime) {
        let hours = duration.as_secs() / 3600;
        if hours < 24 {
            // Today - show time only
            let secs = timestamp % 86400;
            let h = (secs / 3600) % 24;
            let m = (secs % 3600) / 60;
            format!("{:02}:{:02}", h, m)
        } else {
            // Older - show date
            let days = hours / 24;
            format!("{}d ago", days)
        }
    } else {
        "".to_string()
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

fn looks_like_oauth_url(input: &str) -> bool {
    input.contains("access_token=")
        || input.contains("oauth.vk.com/blank.html")
        || input.starts_with("https://oauth.vk.com/blank.html#")
        || input.starts_with("oauth.vk.com/blank.html#")
        || input.starts_with("//oauth.vk.com/blank.html#")
}

const fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn cosmic_root(_theme: &Theme) -> container_widget::Style {
    container_widget::Style {
        text_color: Some(COSMIC_TEXT),
        background: Some(COSMIC_BG.into()),
        ..container_widget::Style::default()
    }
}

fn cosmic_header(_theme: &Theme) -> container_widget::Style {
    container_widget::Style {
        text_color: Some(COSMIC_TEXT),
        background: Some(COSMIC_SURFACE_ALT.into()),
        border: Border {
            width: 1.0,
            radius: 10.0.into(),
            color: COSMIC_BORDER,
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
    }
}

fn cosmic_panel(_theme: &Theme) -> container_widget::Style {
    container_widget::Style {
        text_color: Some(COSMIC_TEXT),
        background: Some(COSMIC_SURFACE.into()),
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: COSMIC_BORDER,
        },
        ..container_widget::Style::default()
    }
}

fn cosmic_sidebar(_theme: &Theme) -> container_widget::Style {
    container_widget::Style {
        text_color: Some(COSMIC_TEXT),
        background: Some(COSMIC_SURFACE.into()),
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: COSMIC_BORDER,
        },
        ..container_widget::Style::default()
    }
}

fn cosmic_button_primary(_theme: &Theme, status: button_widget::Status) -> button_widget::Style {
    let bg = match status {
        button_widget::Status::Hovered => Color::from_rgb8(109, 186, 255),
        button_widget::Status::Pressed => Color::from_rgb8(70, 136, 210),
        _ => COSMIC_ACCENT,
    };

    button_widget::Style {
        background: Some(bg.into()),
        text_color: COSMIC_BG,
        border: Border {
            width: 0.0,
            radius: 10.0.into(),
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
    }
}

fn cosmic_button_secondary(_theme: &Theme, status: button_widget::Status) -> button_widget::Style {
    let bg = match status {
        button_widget::Status::Hovered => COSMIC_SURFACE_ALT,
        button_widget::Status::Pressed => Color::from_rgb8(32, 38, 54),
        _ => COSMIC_SURFACE,
    };

    button_widget::Style {
        background: Some(bg.into()),
        text_color: COSMIC_TEXT,
        border: Border {
            width: 1.0,
            radius: 10.0.into(),
            color: COSMIC_BORDER,
        },
        shadow: Shadow::default(),
    }
}

fn cosmic_button_danger(_theme: &Theme, status: button_widget::Status) -> button_widget::Style {
    let bg = match status {
        button_widget::Status::Hovered => Color::from_rgb8(255, 148, 148),
        button_widget::Status::Pressed => Color::from_rgb8(200, 90, 90),
        _ => COSMIC_DANGER,
    };

    button_widget::Style {
        background: Some(bg.into()),
        text_color: COSMIC_BG,
        border: Border {
            width: 0.0,
            radius: 10.0.into(),
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
    }
}

fn cosmic_chat_button(
    _theme: &Theme,
    status: button_widget::Status,
    selected: bool,
) -> button_widget::Style {
    let bg = if selected {
        COSMIC_SURFACE_ALT
    } else {
        COSMIC_SURFACE
    };
    let hover = if selected {
        COSMIC_SURFACE_ALT
    } else {
        Color::from_rgb8(30, 36, 50)
    };
    let pressed = Color::from_rgb8(24, 29, 41);

    button_widget::Style {
        background: Some(
            match status {
                button_widget::Status::Hovered => hover,
                button_widget::Status::Pressed => pressed,
                _ => bg,
            }
            .into(),
        ),
        text_color: COSMIC_TEXT,
        border: Border {
            width: if selected { 1.0 } else { 0.0 },
            radius: 10.0.into(),
            color: if selected {
                COSMIC_ACCENT
            } else {
                Color::TRANSPARENT
            },
        },
        shadow: Shadow::default(),
    }
}

fn cosmic_message_button(
    _theme: &Theme,
    status: button_widget::Status,
    selected: bool,
    outgoing: bool,
) -> button_widget::Style {
    let base = if outgoing {
        Color::from_rgb8(20, 32, 46)
    } else {
        COSMIC_SURFACE_ALT
    };
    let hover = if outgoing {
        Color::from_rgb8(26, 38, 56)
    } else {
        Color::from_rgb8(32, 38, 54)
    };
    let border = if selected {
        COSMIC_ACCENT
    } else {
        COSMIC_BORDER
    };

    button_widget::Style {
        background: Some(
            match status {
                button_widget::Status::Hovered => hover,
                button_widget::Status::Pressed => Color::from_rgb8(18, 26, 38),
                _ => base,
            }
            .into(),
        ),
        text_color: COSMIC_TEXT,
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: border,
        },
        shadow: Shadow::default(),
    }
}

fn cosmic_text_input(_theme: &Theme, status: input_widget::Status) -> input_widget::Style {
    let border = match status {
        input_widget::Status::Focused => COSMIC_ACCENT,
        input_widget::Status::Hovered => Color::from_rgb8(72, 82, 104),
        _ => COSMIC_BORDER,
    };

    input_widget::Style {
        background: COSMIC_SURFACE.into(),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: border,
        },
        icon: COSMIC_MUTED,
        placeholder: COSMIC_MUTED,
        value: COSMIC_TEXT,
        selection: COSMIC_SELECTION,
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() > max_chars {
        format!("{}...", chars[..max_chars].iter().collect::<String>())
    } else {
        text.to_string()
    }
}
