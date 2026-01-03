# Multi-Frontend Architecture Plan

Рефакторинг для поддержки TUI (ratatui) и GUI (Iced) версий приложения.

## Целевая структура workspace

```
vk-client/
├── Cargo.toml              # workspace root
├── vk-api/                  # [существует] VK API клиент
├── vk-core/                 # [новый] Shared бизнес-логика
├── vk-tui/                  # [рефакторинг] TUI frontend
└── vk-gui/                  # [новый] GUI frontend (Iced)
```

## Анализ текущего кода

### Что можно шарить между frontends

| Файл | Что выносим в vk-core | Что остаётся в vk-tui |
|------|----------------------|----------------------|
| `state.rs` | `Chat`, `ChatMessage`, `AttachmentInfo`, `SearchResult`, `MessagesPagination`, `ChatsPagination`, `AsyncAction` | `Screen`, `Focus`, `Mode`, `CompletionState`, `ForwardView` (TUI-specific UI state) |
| `actions.rs` | Все async функции (load_conversations, send_message, etc.) | — |
| `mapper.rs` | Все функции маппинга VK API → domain models | — |
| `message.rs` | Базовые `Message` варианты (VkEvent, ConversationsLoaded, etc.) | Keyboard-specific variants (InputChar, NavigateUp, etc.) |
| `update.rs` | Бизнес-логика обработки сообщений | UI-specific логика (mode switching, focus) |
| `longpoll.rs` | Вся логика LongPoll | — |

### Что уникально для каждого frontend

**vk-tui:**
- Ratatui widgets (`ui/mod.rs`)
- Crossterm event handling
- Vim-like режимы (Normal/Insert/Command)
- Terminal-specific rendering

**vk-gui (Iced):**
- Iced views и widgets
- Mouse/touch interactions
- Window management
- Native OS integrations

---

## Детальный план vk-core

### 1. Domain Models (`vk-core/src/models/`)

```rust
// vk-core/src/models/chat.rs
pub struct Chat {
    pub id: i64,
    pub title: String,
    pub last_message: String,
    pub last_message_time: i64,
    pub unread_count: u32,
    pub is_online: bool,
}

// vk-core/src/models/message.rs
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
    pub is_pinned: bool,
    pub delivery: DeliveryStatus,
    pub attachments: Vec<AttachmentInfo>,
    pub reply: Option<ReplyPreview>,
    pub fwd_count: usize,
    pub forwards: Vec<ForwardItem>,
}

// vk-core/src/models/attachment.rs
pub struct AttachmentInfo { ... }
pub enum AttachmentKind { ... }

// vk-core/src/models/search.rs
pub struct SearchResult { ... }
pub struct GlobalSearchState { ... }
```

### 2. State Management (`vk-core/src/state/`)

```rust
// vk-core/src/state/app_state.rs
/// Core application state - shared between frontends
pub struct CoreState {
    // Auth
    pub auth: AuthManager,
    pub vk_client: Option<Arc<VkClient>>,

    // Data
    pub users: HashMap<i64, User>,
    pub current_user: Option<User>,
    pub chats: Vec<Chat>,
    pub messages: Vec<ChatMessage>,

    // Selection (abstract)
    pub selected_chat: usize,
    pub selected_message: usize,
    pub current_peer_id: Option<i64>,

    // Pagination
    pub chats_pagination: ChatsPagination,
    pub messages_pagination: Option<MessagesPagination>,

    // Search
    pub search_results: Vec<SearchResult>,
}

// vk-core/src/state/pagination.rs
pub struct MessagesPagination { ... }
pub struct ChatsPagination { ... }
```

### 3. Commands/Actions (`vk-core/src/commands/`)

```rust
// vk-core/src/commands/mod.rs
/// Commands that can be executed by any frontend
pub enum Command {
    // Navigation
    SelectChat(usize),
    SelectMessage(usize),

    // Data loading
    LoadConversations { offset: u32 },
    LoadMessages { peer_id: i64, offset: u32 },
    LoadMessagesAround { peer_id: i64, message_id: i64 },

    // Messaging
    SendMessage { peer_id: i64, text: String },
    SendReply { peer_id: i64, reply_to: i64, text: String },
    SendForward { peer_id: i64, message_ids: Vec<i64>, comment: String },
    EditMessage { peer_id: i64, message_id: i64, text: String },
    DeleteMessage { peer_id: i64, message_id: i64, for_all: bool },

    // Attachments
    SendPhoto { peer_id: i64, path: PathBuf },
    SendDoc { peer_id: i64, path: PathBuf },
    DownloadAttachment { url: String, filename: String },

    // Search
    SearchMessages { query: String },

    // Auth
    Authenticate { token: String },
    Logout,
}

/// Events from core to frontends
pub enum Event {
    // Data loaded
    ConversationsLoaded { chats: Vec<Chat>, total: u32, has_more: bool },
    MessagesLoaded { peer_id: i64, messages: Vec<ChatMessage>, total: u32, has_more: bool },
    SearchResultsLoaded { results: Vec<SearchResult>, total: i32 },

    // Actions completed
    MessageSent { message_id: i64, cmid: i64 },
    MessageEdited { message_id: i64 },
    MessageDeleted { message_id: i64 },

    // Real-time
    NewMessage(ChatMessage),
    MessageRead { peer_id: i64, message_id: i64 },
    UserOnline { user_id: i64 },
    UserOffline { user_id: i64 },

    // Errors
    Error(String),
}
```

### 4. Command Executor (`vk-core/src/executor/`)

```rust
// vk-core/src/executor/mod.rs
pub struct CommandExecutor {
    client: Arc<VkClient>,
    event_tx: mpsc::UnboundedSender<Event>,
}

impl CommandExecutor {
    pub async fn execute(&self, cmd: Command) {
        match cmd {
            Command::LoadConversations { offset } => {
                self.load_conversations(offset).await;
            }
            Command::SendMessage { peer_id, text } => {
                self.send_message(peer_id, text).await;
            }
            // ... остальные команды
        }
    }

    async fn load_conversations(&self, offset: u32) {
        // Текущий код из actions.rs::load_conversations
    }
}
```

### 5. Mappers (`vk-core/src/mapper/`)

```rust
// Перенос текущего mapper.rs без изменений
pub fn map_attachment(att: vk_api::Attachment) -> AttachmentInfo { ... }
pub fn map_history_message(profiles: &[User], msg: &Message, out_read: i64) -> ChatMessage { ... }
pub fn map_reply(profiles: &[User], r: &Message) -> ReplyPreview { ... }
pub fn map_forward_tree(profiles: &[User], m: &Message) -> ForwardItem { ... }
```

### 6. LongPoll (`vk-core/src/longpoll/`)

```rust
// Перенос текущего longpoll.rs
pub struct LongPollHandler { ... }
pub enum VkEvent { ... }
```

---

## Структура vk-core crate

```
vk-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── models/
    │   ├── mod.rs
    │   ├── chat.rs
    │   ├── message.rs
    │   ├── attachment.rs
    │   └── search.rs
    ├── state/
    │   ├── mod.rs
    │   ├── app_state.rs
    │   └── pagination.rs
    ├── commands/
    │   ├── mod.rs
    │   └── types.rs
    ├── executor/
    │   ├── mod.rs
    │   ├── conversations.rs
    │   ├── messages.rs
    │   ├── attachments.rs
    │   └── search.rs
    ├── mapper/
    │   └── mod.rs
    └── longpoll/
        └── mod.rs
```

### Cargo.toml для vk-core

```toml
[package]
name = "vk-core"
version.workspace = true
edition.workspace = true

[dependencies]
vk-api = { path = "../vk-api" }
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
tracing = "0.1"
directories = "6"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

---

## Обновлённая структура vk-tui

```
vk-tui/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point, event loop
    ├── app.rs               # TUI-specific App wrapper
    ├── state.rs             # TUI-specific state (Mode, Focus, etc.)
    ├── input.rs             # Keyboard event → Message mapping
    ├── message.rs           # TUI-specific messages
    ├── update.rs            # TUI-specific update logic
    ├── commands.rs          # :command parsing
    ├── search.rs            # Search UI state
    └── ui/
        ├── mod.rs           # Main render function
        ├── chat_list.rs     # Chat list widget
        ├── messages.rs      # Messages widget
        ├── input.rs         # Input widget
        ├── status.rs        # Status bar
        ├── help.rs          # Help popup
        └── popups/
            ├── mod.rs
            ├── forward.rs
            └── search.rs
```

### TUI App wrapper

```rust
// vk-tui/src/app.rs
use vk_core::{CoreState, Command, Event};

pub struct TuiApp {
    // Shared core state
    pub core: CoreState,

    // TUI-specific state
    pub mode: Mode,
    pub focus: Focus,
    pub screen: Screen,
    pub show_help: bool,
    pub completion_state: CompletionState,
    pub forward_view: Option<ForwardView>,

    // Input buffers (TUI-specific)
    pub input: String,
    pub input_cursor: usize,
    pub command_input: String,
    pub command_cursor: usize,

    // Channels
    pub command_tx: mpsc::UnboundedSender<Command>,
    pub event_rx: mpsc::UnboundedReceiver<Event>,
}

impl TuiApp {
    /// Handle TUI-specific message
    pub fn update(&mut self, msg: TuiMessage) {
        match msg {
            // Mode/focus changes - TUI only
            TuiMessage::EnterNormalMode => self.mode = Mode::Normal,
            TuiMessage::EnterInsertMode => self.mode = Mode::Insert,
            TuiMessage::FocusNext => self.focus = self.focus.next(),

            // Delegate to core
            TuiMessage::SendMessage => {
                if let Some(peer_id) = self.core.current_peer_id {
                    let text = std::mem::take(&mut self.input);
                    self.command_tx.send(Command::SendMessage { peer_id, text });
                }
            }

            // Handle core events
            TuiMessage::CoreEvent(event) => self.handle_core_event(event),
        }
    }

    fn handle_core_event(&mut self, event: Event) {
        match event {
            Event::ConversationsLoaded { chats, .. } => {
                self.core.chats = chats;
            }
            Event::Error(msg) => {
                self.status = Some(msg);
            }
            // ...
        }
    }
}
```

---

## Структура vk-gui (Iced)

> **Источники по архитектуре Iced:**
> - [Iced Architecture Book](https://book.iced.rs/architecture.html)
> - [Iced GitHub Repository](https://github.com/iced-rs/iced)
> - [Iced Subscription Docs](https://docs.rs/iced/latest/iced/struct.Subscription.html)
> - [WebSocket Example](https://github.com/iced-rs/iced/blob/master/examples/websocket/src/main.rs)

### Архитектура Iced (The Elm Architecture)

Iced следует **The Elm Architecture** с четырьмя ключевыми концепциями:

1. **State (Model)** — данные приложения
2. **Messages** — события и взаимодействия
3. **Update logic** — как сообщения изменяют состояние
4. **View logic** — какие виджеты отображать

Этот паттерн создаёт feedback loop:
```
Messages → Update State → View renders widgets → Widgets produce Messages
```

### Subscription Pattern для LongPoll

Для интеграции с VK LongPoll используем `Subscription::run` с каналами:

```rust
// vk-gui/src/subscription.rs
use iced::subscription::{self, Subscription};
use vk_core::{Event, LongPollHandler};

pub fn longpoll_subscription(client: Arc<VkClient>) -> Subscription<Message> {
    Subscription::run_with_id(
        "vk-longpoll",
        stream::channel(100, |mut output| async move {
            let handler = LongPollHandler::new(client);

            loop {
                match handler.poll().await {
                    Ok(events) => {
                        for event in events {
                            let _ = output.send(Message::VkEvent(event)).await;
                        }
                    }
                    Err(e) => {
                        let _ = output.send(Message::Error(e.to_string())).await;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    )
}
```

### Структура файлов

```
vk-gui/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point
    ├── app.rs               # Iced Application trait impl
    ├── message.rs           # Message enum
    ├── state.rs             # GUI-specific state (View enum)
    ├── subscription.rs      # LongPoll & backend subscriptions
    ├── theme.rs             # Custom theme (VK colors)
    └── views/
        ├── mod.rs
        ├── auth.rs          # Login view
        ├── chat_list.rs     # Sidebar with chats
        ├── conversation.rs  # Messages view
        ├── input.rs         # Message input
        └── components/
            ├── mod.rs
            ├── avatar.rs
            ├── message_bubble.rs
            ├── attachment_preview.rs
            └── online_indicator.rs
```

### Cargo.toml для vk-gui

```toml
[package]
name = "vk-gui"
version.workspace = true
edition.workspace = true

[[bin]]
name = "vk-gui"
path = "src/main.rs"

[dependencies]
vk-api = { path = "../vk-api" }
vk-core = { path = "../vk-core" }

tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }

# GUI framework
iced = { version = "0.13", features = ["tokio", "image", "canvas"] }

# Image handling
image = "0.25"
```

### Iced Application Implementation

```rust
// vk-gui/src/app.rs
use iced::{Application, Command, Element, Subscription, Theme};
use iced::widget::{column, container, row, scrollable, text, text_input, button};
use vk_core::{CoreState, Command as CoreCommand, Event as CoreEvent};

/// Connection state enum (similar to WebSocket example)
#[derive(Debug, Clone)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

/// View state
#[derive(Debug, Clone)]
enum View {
    Auth { token_input: String },
    Main,
}

pub struct VkGuiApp {
    // Core state (shared with vk-tui)
    core: CoreState,

    // GUI-specific state
    view: View,
    connection: ConnectionState,
    message_input: String,

    // Command channel to core executor
    command_tx: Option<mpsc::UnboundedSender<CoreCommand>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Auth
    TokenInputChanged(String),
    LoginPressed,

    // Chat navigation
    ChatSelected(usize),
    ChatScrolled(scrollable::Viewport),

    // Messaging
    MessageInputChanged(String),
    SendPressed,
    MessageSelected(usize),

    // Actions
    ReplyPressed(i64),
    ForwardPressed(i64),
    DeletePressed(i64),

    // Core events (from subscription)
    CoreEvent(CoreEvent),

    // UI events
    Tick(std::time::Instant),
    WindowFocused,
    WindowUnfocused,
}

impl Application for VkGuiApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        // Spawn core executor in background
        tokio::spawn(async move {
            // CommandExecutor will process commands from command_rx
        });

        (Self {
            core: CoreState::default(),
            view: View::Auth { token_input: String::new() },
            connection: ConnectionState::Disconnected,
            message_input: String::new(),
            command_tx: Some(command_tx),
        }, Command::none())
    }

    fn title(&self) -> String {
        match &self.connection {
            ConnectionState::Connected => "VK Client".into(),
            ConnectionState::Connecting => "VK Client - Connecting...".into(),
            ConnectionState::Disconnected => "VK Client - Offline".into(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Auth
            Message::TokenInputChanged(token) => {
                if let View::Auth { token_input } = &mut self.view {
                    *token_input = token;
                }
            }
            Message::LoginPressed => {
                if let View::Auth { token_input } = &self.view {
                    self.send_command(CoreCommand::Authenticate {
                        token: token_input.clone()
                    });
                    self.connection = ConnectionState::Connecting;
                }
            }

            // Chat
            Message::ChatSelected(idx) => {
                self.core.selected_chat = idx;
                if let Some(chat) = self.core.chats.get(idx) {
                    self.send_command(CoreCommand::LoadMessages {
                        peer_id: chat.id,
                        offset: 0
                    });
                }
            }

            // Messaging
            Message::SendPressed => {
                if let Some(peer_id) = self.core.current_peer_id {
                    let text = std::mem::take(&mut self.message_input);
                    if !text.is_empty() {
                        self.send_command(CoreCommand::SendMessage { peer_id, text });
                    }
                }
            }

            // Core events
            Message::CoreEvent(event) => {
                self.handle_core_event(event);
            }

            _ => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        match &self.connection {
            ConnectionState::Connected => {
                // Subscribe to LongPoll events
                crate::subscription::longpoll_subscription(
                    self.core.vk_client.clone().unwrap()
                )
            }
            _ => Subscription::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.view {
            View::Auth { token_input } => self.view_auth(token_input),
            View::Main => self.view_main(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark // или custom VK theme
    }
}

impl VkGuiApp {
    fn send_command(&self, cmd: CoreCommand) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(cmd);
        }
    }

    fn handle_core_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::ConversationsLoaded { chats, .. } => {
                self.core.chats = chats;
                self.view = View::Main;
                self.connection = ConnectionState::Connected;
            }
            CoreEvent::MessagesLoaded { peer_id, messages, .. } => {
                if Some(peer_id) == self.core.current_peer_id {
                    self.core.messages = messages;
                }
            }
            CoreEvent::NewMessage(msg) => {
                if Some(msg.peer_id) == self.core.current_peer_id {
                    self.core.messages.push(msg);
                }
            }
            CoreEvent::Error(e) => {
                // Show error notification
                eprintln!("Error: {}", e);
            }
            _ => {}
        }
    }

    fn view_auth(&self, token: &str) -> Element<Message> {
        container(
            column![
                text("VK Client").size(32),
                text("Enter your access token:"),
                text_input("Token...", token)
                    .on_input(Message::TokenInputChanged)
                    .on_submit(Message::LoginPressed)
                    .padding(10),
                button("Login").on_press(Message::LoginPressed),
            ]
            .spacing(10)
            .padding(20)
        )
        .center_x()
        .center_y()
        .into()
    }

    fn view_main(&self) -> Element<Message> {
        let sidebar = self.view_chat_list();
        let content = self.view_conversation();

        row![sidebar, content]
            .spacing(0)
            .into()
    }

    fn view_chat_list(&self) -> Element<Message> {
        let chats: Vec<Element<Message>> = self.core.chats
            .iter()
            .enumerate()
            .map(|(idx, chat)| {
                let is_selected = idx == self.core.selected_chat;
                button(
                    column![
                        text(&chat.title).size(14),
                        text(&chat.last_message).size(12),
                    ]
                )
                .on_press(Message::ChatSelected(idx))
                .style(if is_selected {
                    button::primary
                } else {
                    button::secondary
                })
                .into()
            })
            .collect();

        container(
            scrollable(column(chats).spacing(2))
        )
        .width(300)
        .into()
    }

    fn view_conversation(&self) -> Element<Message> {
        let messages: Vec<Element<Message>> = self.core.messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| {
                crate::views::components::message_bubble(msg, idx == self.core.selected_message)
            })
            .collect();

        let input = row![
            text_input("Message...", &self.message_input)
                .on_input(Message::MessageInputChanged)
                .on_submit(Message::SendPressed),
            button("Send").on_press(Message::SendPressed),
        ]
        .spacing(10)
        .padding(10);

        column![
            scrollable(column(messages).spacing(5)).height(iced::Length::Fill),
            input,
        ]
        .into()
    }
}
```

### Best Practices для Iced (из документации)

1. **State Design**: "Make Impossible States Impossible" — используйте enum для состояний
2. **Message Modeling**: Сообщения должны быть `Debug + Clone`, представлять события
3. **Subscription Identity**: Используйте `run_with_id` для идентификации подписок
4. **Async Operations**: Возвращайте `Command` из `update` для async действий

---

## План миграции

### Этап 1: Создание vk-core (без изменения vk-tui)

1. Создать `vk-core/` crate
2. Скопировать и адаптировать:
   - Domain models из `state.rs`
   - Mappers из `mapper.rs`
   - Action runners из `actions.rs`
   - LongPoll из `longpoll.rs`
3. Определить `Command` и `Event` enums
4. Реализовать `CommandExecutor`

### Этап 2: Рефакторинг vk-tui

1. Добавить зависимость на `vk-core`
2. Заменить дублирующиеся типы на импорты из `vk-core`
3. Создать `TuiApp` wrapper вокруг `CoreState`
4. Рефакторить `update.rs`:
   - TUI-specific логика остаётся
   - Бизнес-логика делегируется в core через Commands
5. Разбить `ui/mod.rs` на отдельные виджеты

### Этап 3: Создание vk-gui

1. Создать `vk-gui/` crate
2. Реализовать базовую Iced Application
3. Подключить `vk-core`
4. Реализовать views:
   - Auth screen
   - Chat list sidebar
   - Messages view
   - Input area

### Этап 4: Общие улучшения

1. Добавить trait `Frontend` для унификации
2. Добавить конфигурацию shared между frontends
3. Реализовать кеширование в core
4. Добавить offline mode

---

## Диаграмма архитектуры

```
┌─────────────────────────────────────────────────────────────┐
│                        User Input                            │
└─────────────────────────────────────────────────────────────┘
            │                                    │
            ▼                                    ▼
┌───────────────────────┐          ┌───────────────────────┐
│       vk-tui          │          │       vk-gui          │
│  ┌─────────────────┐  │          │  ┌─────────────────┐  │
│  │ Crossterm Events│  │          │  │  Iced Events    │  │
│  └────────┬────────┘  │          │  └────────┬────────┘  │
│           ▼           │          │           ▼           │
│  ┌─────────────────┐  │          │  ┌─────────────────┐  │
│  │  TUI Messages   │  │          │  │  GUI Messages   │  │
│  └────────┬────────┘  │          │  └────────┬────────┘  │
│           ▼           │          │           ▼           │
│  ┌─────────────────┐  │          │  ┌─────────────────┐  │
│  │   TUI Update    │──┼──┐   ┌───┼──│   GUI Update    │  │
│  └─────────────────┘  │  │   │   │  └─────────────────┘  │
│           │           │  │   │   │           │           │
│           ▼           │  │   │   │           ▼           │
│  ┌─────────────────┐  │  │   │   │  ┌─────────────────┐  │
│  │ Ratatui Widgets │  │  │   │   │  │  Iced Views     │  │
│  └─────────────────┘  │  │   │   │  └─────────────────┘  │
└───────────────────────┘  │   │   └───────────────────────┘
                           │   │
                           ▼   ▼
              ┌─────────────────────────────┐
              │          vk-core            │
              │  ┌───────────────────────┐  │
              │  │      Commands         │◄─┼── From frontends
              │  └───────────┬───────────┘  │
              │              ▼              │
              │  ┌───────────────────────┐  │
              │  │   CommandExecutor     │  │
              │  └───────────┬───────────┘  │
              │              ▼              │
              │  ┌───────────────────────┐  │
              │  │     CoreState         │  │
              │  │  - chats, messages    │  │
              │  │  - users, pagination  │  │
              │  └───────────────────────┘  │
              │              │              │
              │              ▼              │
              │  ┌───────────────────────┐  │
              │  │       Events          │──┼── To frontends
              │  └───────────────────────┘  │
              └─────────────┬───────────────┘
                            │
                            ▼
              ┌─────────────────────────────┐
              │          vk-api             │
              │  ┌───────────────────────┐  │
              │  │     VkClient          │  │
              │  │  - messages()         │  │
              │  │  - users()            │  │
              │  │  - longpoll()         │  │
              │  └───────────────────────┘  │
              └─────────────┬───────────────┘
                            │
                            ▼
              ┌─────────────────────────────┐
              │        VK API Server        │
              └─────────────────────────────┘
```

---

## Обновлённый workspace Cargo.toml

```toml
[workspace]
members = ["vk-api", "vk-core", "vk-tui", "vk-gui"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["at1ass"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/at1ass/vk_tui"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "2"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# HTTP client (shared)
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

---

## Чеклист для начала работы

- [ ] Создать `vk-core/` директорию и `Cargo.toml`
- [ ] Определить базовые domain models
- [ ] Перенести mappers
- [ ] Определить Command и Event enums
- [ ] Реализовать CommandExecutor (начать с load_conversations)
- [ ] Перенести LongPoll handler
- [ ] Добавить vk-core как зависимость в vk-tui
- [ ] Постепенно заменять типы в vk-tui на импорты из vk-core
- [ ] Создать скелет vk-gui с базовым Iced Application
