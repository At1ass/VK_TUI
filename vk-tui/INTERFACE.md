# VK TUI Interface Design

**Date**: 2025-12-16
**Status**: Implementation in progress

## Overview

VK TUI implements a vi-like modal interface for navigating and interacting with VKontakte messenger. The interface is built around three core concepts:

1. **Three Modes**: Normal, Insert, Command (inspired by Vim)
2. **Three Panels**: ChatList, Messages, Input
3. **Keyboard-first**: All operations accessible via keyboard shortcuts

## Layout

```
┌─────────────────────────────────────────────────────┐
│  Chats (30%)         │  Messages (70%)              │
│  ┌───────────────┐   │  ┌──────────────────────┐   │
│  │ ● John Doe    │   │  │ 12:30 John: Hello!   │   │
│  │   Hello!      │   │  │ 12:31 You: Hi! ✓✓    │   │
│  │               │   │  │ 12:32 John: How are  │   │
│  │ ○ Jane Smith  │   │  │         you?         │   │
│  │   Hey! (2)    │   │  │                      │   │
│  │               │   │  │                      │   │
│  │ ● Group Chat  │   │  │                      │   │
│  │   Someone: ... │   │  │                      │   │
│  └───────────────┘   │  └──────────────────────┘   │
│                      │                              │
│                      │  ┌──────────────────────┐   │
│                      │  │ Message (Enter send) │   │
│                      │  │ Type here...         │   │
│                      │  └──────────────────────┘   │
│                      │                              │
├─────────────────────────────────────────────────────┤
│ Status / Help / Command input                       │
└─────────────────────────────────────────────────────┘
```

- **Left panel** (30%): List of conversations
- **Right panel** (70%): Messages view + Input field
- **Bottom line**: Status bar / Command input / Help hints

## Modes

### 1. Normal Mode (Default)

Navigation and action mode. Context-aware based on current panel focus.

**ChatList Panel Focus:**
| Key | Action |
|-----|--------|
| `j`, `Down` | Navigate down |
| `k`, `Up` | Navigate up |
| `g` | Go to first chat |
| `G` | Go to last chat |
| `l`, `Enter` | Open selected chat (switch to Messages) |
| `/` | Search conversations |
| `?` | Show help popup |
| `:` | Enter Command mode |

**Messages Panel Focus:**
| Key | Action |
|-----|--------|
| `j`, `Down` | Scroll down |
| `k`, `Up` | Scroll up |
| `g` | Go to first message |
| `G` | Go to last message |
| `Ctrl+u` | Page up |
| `Ctrl+d` | Page down |
| `i`, `Enter` | Enter Insert mode (focus Input) |
| `h`, `Esc` | Go back to ChatList |
| `l` | Enter Insert mode (focus Input) |
| `r` | Reply to selected message |
| `f` | Forward selected message |
| `dd` | Delete selected message |
| `e` | Edit selected message (if outgoing) |
| `yy` | Copy message text to clipboard |
| `p` | Pin/unpin message |
| `o`, `Ctrl+l` | Open link in message |
| `a`, `Ctrl+d` | Download attachments |
| `/` | Search messages in chat |
| `?` | Show help popup |
| `:` | Enter Command mode |

**Global Keys (work in any panel):**
| Key | Action |
|-----|--------|
| `Tab` | Next panel (ChatList → Messages → Input → ChatList) |
| `Shift+Tab` | Previous panel |
| `Ctrl+c`, `Ctrl+q` | Quit application |
| `Esc` | Go back / Close popup |

### 2. Insert Mode

Text input mode. Activated when Input field is focused.

| Key | Action |
|-----|--------|
| Any char | Insert character |
| `Enter` | Send message and return to Normal mode |
| `Esc` | Exit to Normal mode (focus Messages) |
| `Backspace` | Delete character |
| `Ctrl+w` | Delete word |
| `Ctrl+u` | Clear line |

**Special input commands:**
- `/sendfile <path>` - Send file attachment
- `/sendimg <path>` - Send image attachment
- `/sendimg --clipboard` - Send image from clipboard

### 3. Command Mode

Vim-style command mode. Activated by pressing `:` in Normal mode.

Commands are entered in the bottom status line and executed with `Enter`.

| Command | Alias | Description |
|---------|-------|-------------|
| `:quit` | `:q` | Quit application |
| `:quitall` | `:qa` | Force quit without confirmation |
| `:back` | `:b` | Return to ChatList |
| `:search <query>` | `:s <query>` | Search conversations by name |
| `:msg <text>` | `:m <text>` | Quick send message to current chat |
| `:attach photo <path>` | `:ap <path>` | Send photo attachment |
| `:attach doc <path>` | `:ad <path>` | Send document attachment |
| `:help` | `:h` | Show help popup |
| `:close` | - | Close popup (if open) |

**Command mode keys:**
| Key | Action |
|-----|--------|
| Any char | Type command |
| `Enter` | Execute command |
| `Esc` | Cancel and return to Normal mode |
| `Backspace` | Delete character |
| `Ctrl+w` | Delete word |
| `Ctrl+u` | Clear command line |

## Mode Transitions

```
          i, Enter (from Messages)
Normal ──────────────────────────────> Insert
  │  ^                                    │
  │  │                                    │ Esc
  │  │                                    │
  │  └────────────────────────────────────┘
  │
  │  :
  └──────> Command
     ^       │
     │       │ Enter (execute)
     │       │ Esc (cancel)
     └───────┘
```

## Help Popup

Triggered by `?` in Normal mode or `:help` in Command mode.

Shows context-aware keybindings based on current focus:
- ChatList help when focused on ChatList
- Messages help when focused on Messages
- General help overlay with all modes

Press `Esc` or `q` to close.

## Status Bar

The bottom line shows different information based on mode:

**Normal Mode:**
- Shows context hints: `j/k nav | h/l panels | i/Enter select | ? help`
- Or current status messages (errors, loading, etc.)

**Insert Mode:**
- Shows input hints: `Enter send | /sendfile PATH | /sendimg PATH | Esc back`

**Command Mode:**
- Shows `:` prompt with current command input
- Cursor visible at command position

## Visual Indicators

- **Mode indicator**: None (removed per user request, help via popup instead)
- **Panel focus**: Focused panel has cyan border, unfocused has gray border
- **Selected item**: Highlighted with dark gray background and `▶` symbol
- **Unread messages**: Bold chat name + count in cyan `(N)`
- **Online status**: `●` green (online) / `○` gray (offline)
- **Message delivery**:
  - `...` pending
  - `✓` sent
  - `✓✓` read
  - `!` failed
- **Typing indicator**: Status bar shows "User is typing..."

## Implementation Details

### State Management

```rust
pub enum Mode {
    Normal,  // Default mode
    Insert,  // Text input mode
    Command, // : command mode
}

pub enum Focus {
    ChatList,  // Left panel
    Messages,  // Right panel (messages)
    Input,     // Right panel (input field)
}

pub struct App {
    mode: Mode,
    focus: Focus,
    command_input: String,
    command_cursor: usize,
    show_help: bool,
    // ... other fields
}
```

### Key Event Routing

1. Global shortcuts checked first (Ctrl+C, Ctrl+Q, etc.)
2. If popup is open (help), popup handles keys
3. Else route to current mode handler:
   - Normal mode → `normal_mode_key(focus)`
   - Insert mode → `insert_mode_key()`
   - Command mode → `command_mode_key()`

### Mode Logic

- **Normal mode**: Always active when `mode == Mode::Normal`
  - Behavior changes based on `focus` (ChatList vs Messages)
  - Most keys are panel-context-specific

- **Insert mode**: Active when `mode == Mode::Insert`
  - Automatically set when focus moves to Input
  - All printable chars go to input buffer
  - Esc returns to Normal and moves focus to Messages

- **Command mode**: Active when `mode == Mode::Command`
  - Triggered by `:` in Normal mode
  - Shows command prompt at bottom
  - Enter executes, Esc cancels

## Best Practices

### For Users

1. **Start in Normal mode**: Navigate with `j`/`k`, select with `Enter`
2. **Type messages**: Press `i` to enter Insert mode, type, press `Enter` to send
3. **Quick actions**: Use vi-like shortcuts (`dd`, `yy`, `p`, etc.) in Normal mode
4. **Commands**: Press `:` for advanced operations
5. **Get help**: Press `?` anytime for context-aware help

### For Developers

1. **Add new keys**:
   - Update `normal_mode_key()` or `insert_mode_key()` in `message.rs`
   - Add new `Message` variant if needed
   - Handle in `update()` function in `app.rs`

2. **Add new commands**:
   - Add variant to command parser
   - Handle in `handle_command()` function
   - Update help popup

3. **Mode changes**:
   - Always go through `app.mode` setter
   - Update focus if needed
   - Clear command input when leaving Command mode

## References

- **Vim**: Modal editing philosophy
- **Telegram CLI**: Conversation-based TUI layout
- **Weechat**: Multi-panel TUI design
- **VK API**: [Official Documentation](https://dev.vk.com/method)
