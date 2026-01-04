# VK TUI Refactoring Plan

## Executive Summary

This document outlines a comprehensive refactoring plan for the VK TUI application based on industry best practices from Ratatui, GitUI, and Datalust. The application currently uses The Elm Architecture (TEA) pattern effectively, but has grown to a point where code organization, testability, and maintainability need improvement.

**Status**: The application is feature-complete and usable as a text-based VKontakte client. This refactoring focuses on architectural improvements without changing functionality.

---

## Current Architecture Analysis

### What We're Doing Right

1. **The Elm Architecture (TEA)**: Clean separation of Model-Update-View
   - `App` struct holds all state (Model)
   - `update()` function handles all state transitions (Update)
   - `ui::view()` renders the interface (View)
   - Message-based communication with async operations

2. **Multi-crate workspace structure**:
   - `vk-api`: VK API client library (reusable)
   - `vk-tui`: Terminal UI application
   - Clean separation of concerns at the workspace level

3. **Async architecture**: Proper use of tokio channels for non-blocking operations

### What Needs Improvement

#### 1. Monolithic Files

| File | Lines | Issue |
|------|-------|-------|
| `vk-tui/src/update.rs` | 1583 | All business logic in one giant match statement |
| `vk-tui/src/ui/mod.rs` | 1243 | All rendering logic in one module |
| `vk-tui/src/state.rs` | Large | God Object pattern - App struct has too many responsibilities |

#### 2. Testing

- **No unit tests** for business logic
- **No integration tests** for async actions
- **Hard to test**: Direct VkClient coupling, no dependency injection
- **No mocking**: Cannot test without real VK API calls

#### 3. Code Organization

- Logic scattered across files without clear domain boundaries
- Hard to navigate: finding "message sending logic" requires searching multiple files
- Tight coupling between UI and business logic
- No clear module boundaries for different features (messages, chats, search, etc.)

#### 4. State Management

- Single `App` struct with 30+ fields
- No logical grouping by domain
- Difficult to understand which fields relate to which features
- Pagination state duplicated for different entities

---

## Research Findings: Industry Best Practices

### 1. The Elm Architecture (from Ratatui docs)

**Source**: [Ratatui - The Elm Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)

**Key Points**:
- TEA is the recommended pattern for Ratatui apps ✓ (we're using this)
- Separate Model, Update, View ✓ (we have this)
- Use Message enum for all events ✓ (we have this)
- **Recommendation**: Break large Update functions into smaller, domain-specific handlers

**What we should adopt**:
```rust
// Instead of one giant update() function:
fn update(app: &mut App, msg: Message) -> Option<Message> {
    match msg {
        Message::Chat(chat_msg) => update_chat(app, chat_msg),
        Message::Messages(msg_msg) => update_messages(app, msg_msg),
        Message::Search(search_msg) => update_search(app, search_msg),
        // ...
    }
}
```

### 2. GitUI Architecture

**Source**: [GitUI GitHub](https://github.com/gitui-org/gitui)

**Key Findings**:

1. **Separate business logic crate**: `asyncgit` crate handles all git operations
   - Zero UI dependencies
   - Fully testable in isolation
   - Can be used in other applications

2. **Component-based UI**:
   ```
   src/components/
     ├── command_palette.rs
     ├── commit_details.rs
     ├── file_tree.rs
     ├── diff_view.rs
     └── ...
   ```
   - Each component is self-contained
   - Components handle their own state
   - Composable and reusable

3. **Thread pool for async operations**:
   - UI thread stays responsive
   - Background tasks run in thread pool
   - Results sent back via channels

**What we should adopt**:
- Create `vk-core` crate for business logic (separate from vk-api and vk-tui)
- Component-based UI architecture
- Better async task management

### 3. Datalust: Organizing Complex Rust Codebases

**Source**: [Datalust Blog](https://datalust.co/blog/rust-at-datalust-how-we-organize-a-complex-rust-codebase)

**Key Principles**:

1. **Module organization by domain, not by layer**:
   ```
   // Bad (by layer):
   models/
   views/
   controllers/

   // Good (by domain):
   messages/
     ├── state.rs
     ├── actions.rs
     ├── ui.rs
     └── update.rs
   ```

2. **Privacy boundaries**:
   - Use `pub(crate)` to limit visibility
   - Each module exposes minimal public API
   - Internal implementation details stay private

3. **Dependency injection via traits**:
   ```rust
   trait VkApi {
       async fn send_message(&self, peer_id: i64, text: String) -> Result<()>;
   }

   // Production
   impl VkApi for VkClient { ... }

   // Testing
   struct MockVkApi { ... }
   impl VkApi for MockVkApi { ... }
   ```

4. **Flat crate structure**: Avoid deep nesting, prefer flat modules with clear names

**What we should adopt**:
- Organize by feature/domain (messages, chats, search, auth)
- Use traits for external dependencies (VkClient, filesystem, etc.)
- Implement privacy boundaries with `pub(crate)`

---

## Refactoring Plan

### Phase 1: Modularize Update Logic (Week 1-2)

**Goal**: Break `update.rs` into domain-specific modules

#### Step 1.1: Create domain modules
```
vk-tui/src/
  ├── domains/
  │   ├── mod.rs
  │   ├── auth/
  │   │   ├── mod.rs
  │   │   ├── state.rs
  │   │   └── update.rs
  │   ├── chats/
  │   │   ├── mod.rs
  │   │   ├── state.rs
  │   │   ├── update.rs
  │   │   └── pagination.rs
  │   ├── messages/
  │   │   ├── mod.rs
  │   │   ├── state.rs
  │   │   ├── update.rs
  │   │   └── pagination.rs
  │   ├── search/
  │   │   ├── mod.rs
  │   │   ├── state.rs
  │   │   └── update.rs
  │   └── input/
  │       ├── mod.rs
  │       ├── state.rs
  │       └── update.rs
```

#### Step 1.2: Refactor App struct
```rust
// state.rs
pub struct App {
    pub auth: AuthState,
    pub chats: ChatsState,
    pub messages: MessagesState,
    pub search: SearchState,
    pub input: InputState,
    pub ui: UiState,

    // Shared/cross-cutting concerns
    pub vk_client: Option<Arc<VkClient>>,
    pub action_tx: Option<mpsc::UnboundedSender<AsyncAction>>,
}

// domains/chats/state.rs
pub struct ChatsState {
    pub items: Vec<ChatItem>,
    pub selected: usize,
    pub scroll: usize,
    pub filter: Option<String>,
    pub pagination: Pagination,
}

// domains/messages/state.rs
pub struct MessagesState {
    pub items: Vec<MessageItem>,
    pub selected: usize,
    pub scroll: usize,
    pub current_peer_id: Option<i64>,
    pub pagination: Pagination,
}
```

#### Step 1.3: Split Message enum
```rust
// message.rs
pub enum Message {
    Auth(AuthMessage),
    Chats(ChatsMessage),
    Messages(MessagesMessage),
    Search(SearchMessage),
    Input(InputMessage),
    Ui(UiMessage),
    VkEvent(VkEvent),
    Noop,
}

// domains/chats/mod.rs
pub enum ChatsMessage {
    LoadMore,
    Select(usize),
    Filter(String),
    ClearFilter,
    Loaded { items: Vec<ChatItem>, pagination: PaginationInfo },
}
```

#### Step 1.4: Domain-specific update functions
```rust
// domains/chats/update.rs
pub fn update(state: &mut ChatsState, msg: ChatsMessage, context: &mut UpdateContext) -> Option<Message> {
    match msg {
        ChatsMessage::LoadMore => {
            if !state.pagination.is_loading && state.pagination.has_more {
                context.send_action(AsyncAction::LoadConversations(state.pagination.offset));
                state.pagination.is_loading = true;
            }
            None
        }
        ChatsMessage::Select(idx) => {
            state.selected = idx;
            // Trigger message loading for selected chat
            Some(Message::Messages(MessagesMessage::LoadForPeer(state.items[idx].peer_id)))
        }
        // ...
    }
}

// update.rs (simplified)
pub fn update(app: &mut App, msg: Message) -> Option<Message> {
    match msg {
        Message::Auth(msg) => auth::update(&mut app.auth, msg, &mut UpdateContext::new(app)),
        Message::Chats(msg) => chats::update(&mut app.chats, msg, &mut UpdateContext::new(app)),
        Message::Messages(msg) => messages::update(&mut app.messages, msg, &mut UpdateContext::new(app)),
        // ...
    }
}
```

**Benefits**:
- Each domain is now ~200-300 lines instead of 1583
- Easy to find and modify specific feature logic
- Clear ownership and responsibilities
- Can test domains in isolation

### Phase 2: Component-Based UI (Week 3-4)

**Goal**: Break `ui/mod.rs` into reusable components

#### Step 2.1: Create component structure
```
vk-tui/src/ui/
  ├── mod.rs (main composition)
  ├── components/
  │   ├── mod.rs
  │   ├── chat_list.rs
  │   ├── message_list.rs
  │   ├── input_box.rs
  │   ├── help_popup.rs
  │   ├── search_bar.rs
  │   └── status_bar.rs
  └── styles/
      ├── mod.rs
      └── theme.rs
```

#### Step 2.2: Component trait
```rust
// ui/components/mod.rs
pub trait Component {
    type State;

    fn render(&self, state: &Self::State, area: Rect, buf: &mut Buffer);

    fn required_height(&self, state: &Self::State, width: u16) -> u16 {
        area.height
    }
}

// ui/components/chat_list.rs
pub struct ChatList;

impl Component for ChatList {
    type State = ChatsState;

    fn render(&self, state: &Self::State, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = state.items
            .iter()
            .map(|chat| ListItem::new(chat.title.clone()))
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::DarkGray));

        StatefulWidget::render(list, area, buf, &mut state.list_state.clone());
    }
}
```

#### Step 2.3: Compose in main view
```rust
// ui/mod.rs
pub fn view(app: &App, frame: &mut Frame) {
    let area = frame.size();

    match app.ui.screen {
        Screen::Auth => {
            components::AuthForm.render(&app.auth, area, frame.buffer_mut());
        }
        Screen::Main => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(area);

            components::ChatList.render(&app.chats, chunks[0], frame.buffer_mut());
            components::MessageList.render(&app.messages, chunks[1], frame.buffer_mut());

            if let Some(search) = &app.search.active {
                components::SearchPopup.render(search, area, frame.buffer_mut());
            }
        }
    }
}
```

**Benefits**:
- Components are testable (can render to buffer and assert output)
- Reusable across different screens
- Clear visual hierarchy
- Easier to theme and style consistently

### Phase 3: Dependency Injection & Testing (Week 5-6)

**Goal**: Make business logic testable

#### Step 3.1: Define traits for external dependencies
```rust
// vk-tui/src/traits.rs
#[async_trait]
pub trait VkApi: Send + Sync {
    async fn get_conversations(&self, offset: u32, count: u32) -> Result<ConversationsResponse>;
    async fn get_history(&self, peer_id: i64, offset: u32, count: u32) -> Result<MessagesHistoryResponse>;
    async fn send_message(&self, peer_id: i64, text: String) -> Result<SentMessage>;
    // ... other methods
}

#[async_trait]
impl VkApi for Arc<VkClient> {
    async fn get_conversations(&self, offset: u32, count: u32) -> Result<ConversationsResponse> {
        self.messages().get_conversations(offset, count).await
    }
    // ... implement other methods
}
```

#### Step 3.2: Update actions to use traits
```rust
// actions.rs
pub async fn load_conversations<T: VkApi>(
    client: Arc<T>,
    offset: u32,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.get_conversations(offset, 50).await {
        Ok(response) => {
            let items = response.items.into_iter().map(|item| {
                // ... mapping logic
            }).collect();

            let _ = tx.send(Message::Chats(ChatsMessage::Loaded {
                items,
                pagination: PaginationInfo {
                    total: response.count as u32,
                    loaded: items.len() as u32,
                },
            }));
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load conversations: {}", e)));
        }
    }
}
```

#### Step 3.3: Create mock for testing
```rust
// tests/mocks/vk_api.rs
pub struct MockVkApi {
    pub conversations: Vec<ConversationItem>,
    pub messages: HashMap<i64, Vec<Message>>,
}

#[async_trait]
impl VkApi for Arc<MockVkApi> {
    async fn get_conversations(&self, offset: u32, count: u32) -> Result<ConversationsResponse> {
        let start = offset as usize;
        let end = (start + count as usize).min(self.conversations.len());

        Ok(ConversationsResponse {
            count: self.conversations.len() as i32,
            items: self.conversations[start..end].to_vec(),
            profiles: vec![],
            groups: vec![],
        })
    }
    // ... implement other methods
}
```

#### Step 3.4: Write tests
```rust
// tests/domains/chats_test.rs
#[tokio::test]
async fn test_load_conversations() {
    let mock_api = Arc::new(MockVkApi {
        conversations: vec![
            create_test_conversation(1, "Chat 1"),
            create_test_conversation(2, "Chat 2"),
        ],
        messages: HashMap::new(),
    });

    let (tx, mut rx) = mpsc::unbounded_channel();

    load_conversations(mock_api, 0, tx).await;

    let msg = rx.recv().await.unwrap();
    match msg {
        Message::Chats(ChatsMessage::Loaded { items, pagination }) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].title, "Chat 1");
            assert_eq!(pagination.total, 2);
        }
        _ => panic!("Expected ChatsMessage::Loaded"),
    }
}

// tests/domains/messages_test.rs
#[test]
fn test_navigate_up_loads_older_messages() {
    let mut state = MessagesState {
        items: vec![create_test_message(1), create_test_message(2)],
        scroll: 0,
        pagination: Pagination {
            first_cmid: Some(1),
            has_more: true,
            is_loading: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut context = MockUpdateContext::new();
    let result = messages::update(&mut state, MessagesMessage::NavigateUp, &mut context);

    assert!(context.actions_sent.contains(&AsyncAction::LoadMessagesWithOffset(
        peer_id, 1, -1, 50
    )));
}
```

**Benefits**:
- Fast unit tests without network calls
- Test edge cases (errors, empty responses, pagination)
- Refactor with confidence
- Easier onboarding for new contributors

### Phase 4: Create vk-core Crate (Week 7-8)

**Goal**: Extract business logic to separate crate

#### Step 4.1: Create new crate
```
vk-core/
  ├── Cargo.toml
  └── src/
      ├── lib.rs
      ├── traits.rs
      ├── models.rs
      ├── chats/
      │   ├── mod.rs
      │   ├── service.rs
      │   └── types.rs
      ├── messages/
      │   ├── mod.rs
      │   ├── service.rs
      │   └── types.rs
      └── search/
          ├── mod.rs
          ├── service.rs
          └── types.rs
```

#### Step 4.2: Define services
```rust
// vk-core/src/chats/service.rs
pub struct ChatService<T: VkApi> {
    api: Arc<T>,
}

impl<T: VkApi> ChatService<T> {
    pub fn new(api: Arc<T>) -> Self {
        Self { api }
    }

    pub async fn load_conversations(
        &self,
        pagination: &Pagination,
    ) -> Result<LoadConversationsResult> {
        let response = self.api
            .get_conversations(pagination.offset, 50)
            .await?;

        let items = response.items
            .into_iter()
            .map(|item| self.map_conversation_item(item, &response.profiles))
            .collect();

        Ok(LoadConversationsResult {
            items,
            total_count: response.count as u32,
            has_more: items.len() == 50,
        })
    }

    fn map_conversation_item(&self, item: ConversationItem, profiles: &[User]) -> ChatItem {
        // ... mapping logic
    }
}
```

#### Step 4.3: Use in TUI
```rust
// vk-tui/src/actions.rs
pub async fn load_conversations<T: VkApi>(
    service: Arc<ChatService<T>>,
    pagination: Pagination,
    tx: mpsc::UnboundedSender<Message>,
) {
    match service.load_conversations(&pagination).await {
        Ok(result) => {
            let _ = tx.send(Message::Chats(ChatsMessage::Loaded {
                items: result.items,
                pagination: result.into(),
            }));
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load conversations: {}", e)));
        }
    }
}
```

**Benefits**:
- Business logic usable in other applications (GUI, web, etc.)
- Zero UI dependencies
- Clear API boundaries
- Full test coverage without UI concerns

---

## Comparison: Current vs. Recommended

| Aspect | Current | Recommended | Benefit |
|--------|---------|-------------|---------|
| **Update logic** | 1583-line function | Domain modules (~200 lines each) | Easier to find and modify code |
| **UI rendering** | 1243-line file | Component-based (50-100 lines each) | Reusable, testable components |
| **State** | Single App struct (30+ fields) | Domain-specific state structs | Clear ownership, easier to reason about |
| **Testing** | No tests | Trait-based DI + unit tests | Fast feedback, catch bugs early |
| **Dependencies** | Direct VkClient coupling | Trait-based abstraction | Mockable, testable, flexible |
| **Business logic** | Mixed with UI in vk-tui | Separate vk-core crate | Reusable, portable, focused |
| **Code navigation** | Search through monolithic files | Navigate to domain module | Faster development |
| **Onboarding** | Hard to understand structure | Clear domain boundaries | Easier for new contributors |

---

## Migration Strategy

### Risk Mitigation

1. **Feature branch**: Do all refactoring in `refactor/architecture` branch
2. **Incremental**: Migrate one domain at a time, keep app working
3. **Testing**: Add tests before and after each migration step
4. **Code freeze**: No new features during refactoring period
5. **Rollback plan**: Keep main branch stable, can abandon refactor branch if needed

### Week-by-Week Plan

| Week | Focus | Deliverable | Risk |
|------|-------|-------------|------|
| 1 | Create domain structure | Empty domain modules, move auth logic | Low - small scope |
| 2 | Migrate chats & messages | Full domain separation for core features | Medium - complex logic |
| 3 | Component infrastructure | Component trait, status bar component | Low - additive only |
| 4 | Migrate all UI components | Full component-based rendering | Medium - visual changes |
| 5 | Define VkApi trait | Trait + production impl | Low - wrapper only |
| 6 | Write tests | 80% test coverage | Low - tests don't break prod |
| 7 | Create vk-core crate | Separate crate with basic services | Medium - new abstraction |
| 8 | Final migration | Move all business logic to vk-core | High - large refactor |

### Success Metrics

- [ ] All files < 500 lines
- [ ] 80% test coverage for business logic
- [ ] No direct VkClient usage in update.rs
- [ ] Components reusable and composable
- [ ] CI/CD pipeline with tests
- [ ] Documentation for each module
- [ ] Performance: no regression in latency or memory

---

## Future Enhancements (Post-Refactor)

With clean architecture in place, these become easier:

1. **Audio/video playback**: Add media backends without touching core logic
2. **Theming system**: Component-based UI makes styling trivial
3. **Multiple accounts**: Service-based architecture supports multiple instances
4. **Offline mode**: Add caching layer in vk-core
5. **GUI version**: Reuse vk-core for desktop GUI
6. **Web version**: Reuse vk-core for web backend
7. **CI/CD**: Automated testing and releases
8. **Performance monitoring**: Trace points in service layer

---

## References

1. [The Elm Architecture - Ratatui](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
2. [GitUI - GitHub](https://github.com/gitui-org/gitui)
3. [How We Organize a Complex Rust Codebase - Datalust](https://datalust.co/blog/rust-at-datalust-how-we-organize-a-complex-rust-codebase)
4. [Building Rich Terminal UIs in Rust - BrightCoding](https://www.blog.brightcoding.dev/2025/09/13/ratatui-building-rich-terminal-user-interfaces-in-rust/)

---

## Conclusion

The VK TUI application is functionally complete and usable. This refactoring plan focuses on improving code quality, testability, and maintainability without changing user-facing behavior.

By following established patterns from successful Rust TUI applications (GitUI) and general Rust best practices (Datalust), we can transform the codebase into a well-organized, testable, and extensible foundation for future enhancements.

**Estimated effort**: 8 weeks part-time (2-3 hours/day)
**Risk level**: Medium (large scope but incremental approach)
**Recommended**: Yes - benefits outweigh costs for long-term maintenance
