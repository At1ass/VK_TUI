//! Async Rust client for VK API
//!
//! This library provides a strongly-typed async client for VKontakte (VK) API.
//! It supports messaging, user operations, media uploads, and Long Poll for real-time updates.
//!
//! # Examples
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
//!     // Get conversations
//!     let chats = client.get_conversations(0, 20).await?;
//!     println!("Got {} chats", chats.items.len());
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod auth;
pub mod types;

// Re-exports for convenience
pub use api::VkClient;
pub use types::*;

/// VK API version used by this library
pub const API_VERSION: &str = "5.199";

/// VK API base URL
pub const API_URL: &str = "https://api.vk.com/method";
