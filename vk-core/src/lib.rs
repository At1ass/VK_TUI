//! VK Core - shared business logic for VK client applications.
//!
//! This crate provides UI-agnostic core functionality that can be used
//! by both TUI (ratatui) and GUI (Iced) frontends.

pub mod commands;
pub mod events;
pub mod executor;
pub mod longpoll;
pub mod mapper;
pub mod models;
pub mod state;

// Re-export commonly used types
pub use commands::{AsyncCommand, Command};
pub use events::{CoreEvent, VkEvent};
pub use executor::CommandExecutor;
pub use models::*;
pub use state::{ChatsPagination, CoreState, MessagesPagination};

// Re-export vk-api types that frontends might need
pub use vk_api::{User, VkClient};
