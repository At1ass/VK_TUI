//! Attachment types for messages.

/// Summary information about an attachment.
#[derive(Debug, Clone)]
pub struct AttachmentInfo {
    pub kind: AttachmentKind,
    pub title: String,
    pub url: Option<String>,
    pub size: Option<u64>,
    pub subtitle: Option<String>,
}

/// Type of attachment.
#[derive(Debug, Clone)]
pub enum AttachmentKind {
    Photo,
    Doc,
    Link,
    Audio,
    Sticker,
    Other(String),
}
