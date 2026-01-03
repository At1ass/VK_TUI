//! Domain models for VK client.
//!
//! These types are UI-agnostic and represent the core data structures
//! used throughout the application.

mod attachment;
mod chat;
mod message;
mod search;

pub use attachment::{AttachmentInfo, AttachmentKind};
pub use chat::Chat;
pub use message::{ChatMessage, DeliveryStatus, ForwardItem, ReplyPreview};
pub use search::SearchResult;
