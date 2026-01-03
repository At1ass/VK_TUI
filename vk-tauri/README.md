# VK Tauri Client

Modern VKontakte messenger built with Tauri (Rust + Svelte).

## Architecture

```
vk-tauri/
├── src/              # Rust backend
│   ├── main.rs       # Tauri app entry
│   ├── state.rs      # App state management
│   └── commands.rs   # Tauri commands (RPC)
├── ui/               # Svelte frontend
│   └── src/
│       ├── App.svelte
│       └── main.js
└── tauri.conf.json   # Tauri configuration
```

## Features

- ✅ OAuth authentication
- ✅ Real-time messaging via LongPoll
- ✅ **Video/audio playback** (webkit2gtk + GStreamer)
- ✅ **Animated stickers** (WebP support)
- ✅ Image attachments
- ✅ Reply/Forward messages
- ✅ Native notifications

## Development

```bash
# Install dependencies
npm install

# Run in dev mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Linux Requirements

For hardware-accelerated video (Nvidia):

```bash
sudo apt install \
    libwebkit2gtk-4.1-dev \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-libav \
    gstreamer1.0-vaapi
```

## Integration with vk-core

All business logic is handled by `vk-core` crate:

```rust
// Backend sends commands
AsyncCommand::LoadMessages { peer_id, offset }

// Receives events
CoreEvent::MessagesLoaded { messages, ... }
```

Frontend communicates via Tauri IPC:

```javascript
// Call Rust from JS
await invoke('load_messages', { peerId: 123, offset: 0 });

import { listen } from '@tauri-apps/api/event';

// Receive core events pushed from Rust
const unlisten = await listen('core:event', (event) => {
  console.log(event.payload);
});
```
