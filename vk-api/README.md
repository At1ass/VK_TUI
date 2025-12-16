# vk-api

Async Rust client for VKontakte (VK) API.

## Features

- ✅ **Async/await** support with tokio
- ✅ **Strongly typed** API responses
- ✅ **Messages API** - send, receive, edit, delete messages
- ✅ **Long Poll** - real-time updates (version 3)
- ✅ **Media uploads** - photos and documents
- ✅ **OAuth authentication** with token storage
- ✅ **User & Friends API**
- 🚧 **Groups & Communities** (planned)
- 🚧 **Wall posts** (planned)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
vk-api = { path = "../vk-api" }
tokio = { version = "1", features = ["full"] }
```

### Quick Start

```rust
use vk_api::{VkClient, auth::AuthManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Authenticate
    let auth = AuthManager::default();
    let token = auth.access_token().expect("Not authenticated");

    // Create client
    let client = VkClient::new(token.to_string());

    // Get conversations
    let chats = client.get_conversations(0, 20).await?;
    println!("Got {} chats", chats.items.len());

    // Send message
    let message_id = client.send_message(12345, "Hello!").await?;
    println!("Sent message {}", message_id);

    Ok(())
}
```

### Authentication

```rust
use vk_api::auth::AuthManager;

let mut auth = AuthManager::default();

// Get OAuth URL
let url = AuthManager::get_auth_url();
println!("Open: {}", url);

// After authorization, save token from redirect URL
auth.save_token_from_url("https://oauth.vk.com/blank.html#access_token=...")?;
```

### Long Poll

```rust
use vk_api::VkClient;

let client = VkClient::new(token);
let mut server = client.get_long_poll_server().await?;

loop {
    match client.long_poll(&server).await {
        Ok(response) => {
            if let Some(ts) = response.ts {
                server.ts = ts;
            }

            if let Some(updates) = response.updates {
                for update in updates {
                    // Process events
                    println!("Update: {:?}", update);
                }
            }
        }
        Err(e) => eprintln!("Long Poll error: {}", e),
    }
}
```

## API Methods

### Messages

- `get_conversations(offset, count)` - Get list of conversations
- `get_history(peer_id, offset, count)` - Get message history
- `send_message(peer_id, text)` - Send text message
- `mark_as_read(peer_id)` - Mark messages as read
- `send_photo(peer_id, path)` - Send photo attachment
- `send_doc(peer_id, path)` - Send file attachment

### Users & Friends

- `get_users(user_ids)` - Get user info
- `search_users(query, count)` - Search users *(TODO)*
- `get_friends()` - Get friends list *(TODO)*

### Long Poll

- `get_long_poll_server()` - Get Long Poll server
- `long_poll(server)` - Poll for updates

## VK API Version

This library uses **VK API v5.199**.

## License

MIT OR Apache-2.0
