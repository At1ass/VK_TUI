//! VK API type definitions

pub mod attachment;
pub mod common;
pub mod group;
pub mod longpoll;
pub mod message;
pub mod misc;
pub mod upload;
pub mod user;

// Re-export commonly used types
pub use attachment::{Attachment, Doc, Photo, PhotoSize};
pub use common::{Peer, VkError, VkResponse};
pub use group::Group;
pub use longpoll::{LongPollResponse, LongPollServer};
pub use message::{
    ChatPhoto, ChatSettings, Conversation, ConversationItem, ConversationsResponse, Message,
    MessagesHistoryResponse, SentMessage,
};
pub use misc::{CanWrite, City, Counters, Country, ProfileInfo};
pub use upload::{DocInfo, SavedDoc, SavedPhoto, UploadDocResponse, UploadServer};
pub use user::{LastSeen, User};
