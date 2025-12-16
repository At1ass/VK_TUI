# vk-tui

Terminal User Interface (TUI) client for VKontakte messenger.

## Features

- ✅ **Real-time messaging** via Long Poll
- ✅ **Vi-like keybindings** for navigation
- ✅ **Send text, photos, files**
- ✅ **Download attachments**
- ✅ **Open links** in browser
- ✅ **Online/typing indicators**
- ✅ **Read receipts**
- ✅ **Clipboard integration** (Wayland/X11)
- 🚧 **Search & filters** (planned)
- 🚧 **Reply & forward** (planned)
- 🚧 **Edit & delete** (planned)

## Installation

```bash
cargo build --release
./target/release/vk-tui
```

## Usage

### First Launch

1. Press `Ctrl+O` to open OAuth URL in browser
2. Authorize the application
3. Copy the redirect URL
4. Paste it into the input field
5. Press Enter

Token is saved to `~/.config/vk_tui/token.json`

### Keybindings

#### Navigation (Normal mode)
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `h` / `←` - Previous panel
- `l` / `→` - Next panel
- `g` - Go to top
- `G` - Go to bottom
- `Tab` - Next panel
- `Shift+Tab` - Previous panel

#### Actions
- `Enter` / `i` - Select chat / Start input
- `Esc` - Back / Cancel
- `Ctrl+Q` / `Ctrl+C` - Quit

#### Messages
- `Ctrl+L` - Open link from selected message
- `Ctrl+D` - Download attachments

#### Slash Commands
- `/sendfile <path>` - Send file
- `/sendimg <path>` - Send image
- `/sendimg --clipboard` - Send image from clipboard

## Configuration

Config file: `~/.config/vk_tui/config.toml` (planned)

## Development

See [ROADMAP.md](../ROADMAP.md) for planned features.

## License

MIT OR Apache-2.0
