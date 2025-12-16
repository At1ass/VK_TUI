//! Async Rust client for VK API
//!
//! This library provides a strongly-typed async client for VKontakte (VK) API.
//! It supports messaging, user operations, media uploads, and Long Poll for real-time updates.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use vk_api::{VkClient, auth::AuthManager};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Authenticate
//!     let auth = AuthManager::default();
//!     let token = auth.access_token().unwrap();
//!
//!     // Create client
//!     let client = VkClient::new(token.to_string());
//!
//!     // Get conversations using new namespace API
//!     let chats = client.messages().get_conversations(0, 20).await?;
//!     println!("Got {} chats", chats.items.len());
//!
//!     // Send message
//!     let msg_id = client.messages().send(12345, "Hello!").await?;
//!
//!     // Get user info
//!     let users = client.users().get(&[12345]).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Long Poll
//!
//! ```rust,no_run
//! use vk_api::VkClient;
//!
//! # async fn example(client: VkClient) -> anyhow::Result<()> {
//! let mut server = client.longpoll().get_server().await?;
//!
//! loop {
//!     match client.longpoll().poll(&server).await {
//!         Ok(response) => {
//!             if let Some(ts) = response.ts {
//!                 server.ts = ts;
//!             }
//!             // Process updates...
//!         }
//!         Err(e) => {
//!             eprintln!("Long Poll error: {}", e);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod client;
pub mod methods;
pub mod types;

// Re-exports for convenience
pub use client::VkClient;
pub use methods::{AccountApi, FriendsApi, LongPollApi, MessagesApi, UsersApi};
pub use types::*;

/// VK API version used by this library
pub const API_VERSION: &str = "5.199";

/// VK API base URL
pub const API_URL: &str = "https://api.vk.com/method";

// Keep old api module for backwards compatibility during transition
#[deprecated(
    since = "0.2.0",
    note = "Use client module and namespace API instead (e.g., client.messages().send())"
)]
pub mod api;
