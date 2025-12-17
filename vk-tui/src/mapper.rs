//! Helpers to map VK API models into internal UI models.
use crate::state::{
    AttachmentInfo, AttachmentKind, ChatMessage, DeliveryStatus, ForwardItem, ReplyPreview,
};
use vk_api::Message;
use vk_api::User;

pub fn map_attachment(att: vk_api::Attachment) -> AttachmentInfo {
    match att.attachment_type.as_str() {
        "photo" => {
            let best = att
                .photo
                .as_ref()
                .and_then(|p| {
                    p.sizes
                        .iter()
                        .filter_map(|s| {
                            s.url.as_ref().map(|url| {
                                let score = s.width.unwrap_or(0) * s.height.unwrap_or(0);
                                (url.clone(), score as u64)
                            })
                        })
                        .max_by_key(|(_, score)| *score)
                })
                .map(|(url, _)| url);

            AttachmentInfo {
                kind: AttachmentKind::Photo,
                title: "Photo".into(),
                url: best,
                size: None,
                subtitle: None,
            }
        }
        "doc" => {
            let doc = att.doc.unwrap_or_default();
            AttachmentInfo {
                kind: AttachmentKind::Doc,
                title: doc.title.unwrap_or_else(|| "Document".to_string()),
                url: doc.url,
                size: doc.size,
                subtitle: doc.extension,
            }
        }
        "link" => {
            let link = att.other.get("link").and_then(|v| v.as_object());
            let title = link
                .and_then(|o| o.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Link")
                .to_string();
            let url = link
                .and_then(|o| o.get("url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            AttachmentInfo {
                kind: AttachmentKind::Link,
                title,
                url,
                size: None,
                subtitle: None,
            }
        }
        "audio" => {
            let audio = att.other.get("audio").and_then(|v| v.as_object());
            let artist = audio
                .and_then(|o| o.get("artist"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = audio
                .and_then(|o| o.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Audio");
            let full_title = if artist.is_empty() {
                title.to_string()
            } else {
                format!("{} — {}", artist, title)
            };
            AttachmentInfo {
                kind: AttachmentKind::Audio,
                title: full_title,
                url: None,
                size: None,
                subtitle: None,
            }
        }
        "sticker" => AttachmentInfo {
            kind: AttachmentKind::Sticker,
            title: "Sticker".into(),
            url: att
                .other
                .get("sticker")
                .and_then(|v| v.get("images"))
                .and_then(|imgs| imgs.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .filter_map(|img| {
                            img.get("url").and_then(|u| u.as_str()).map(|s| {
                                let w = img.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                                (s.to_string(), w)
                            })
                        })
                        .max_by_key(|(_, w)| *w)
                        .map(|(u, _)| u)
                }),
            size: None,
            subtitle: None,
        },
        other => AttachmentInfo {
            kind: AttachmentKind::Other(other.to_string()),
            title: other.to_string(),
            url: None,
            size: None,
            subtitle: None,
        },
    }
}

pub fn map_reply(profiles: &[User], r: &Message) -> ReplyPreview {
    let attachments = r
        .attachments
        .clone()
        .into_iter()
        .map(map_attachment)
        .collect();
    ReplyPreview {
        from: get_name(profiles, r.from_id),
        text: if r.text.is_empty() {
            "[attachment]".to_string()
        } else {
            r.text.clone()
        },
        attachments,
    }
}

pub fn map_forward_tree(profiles: &[User], m: &Message) -> ForwardItem {
    let attachments = m
        .attachments
        .clone()
        .into_iter()
        .map(map_attachment)
        .collect();
    let nested = m
        .fwd_messages
        .iter()
        .map(|fm| map_forward_tree(profiles, fm))
        .collect();

    ForwardItem {
        from: get_name(profiles, m.from_id),
        text: if m.text.is_empty() {
            "[attachment]".to_string()
        } else {
            m.text.clone()
        },
        attachments,
        nested,
    }
}

pub fn map_history_message(profiles: &[User], msg: &Message, out_read: i64) -> ChatMessage {
    let from_name = get_name(profiles, msg.from_id);

    let is_outgoing = msg.is_outgoing();
    let is_read = if is_outgoing {
        msg.id <= out_read
    } else {
        msg.is_read()
    };
    let text = if msg.text.is_empty() {
        "[attachment]".to_string()
    } else {
        msg.text.clone()
    };
    let attachments = msg
        .attachments
        .clone()
        .into_iter()
        .map(map_attachment)
        .collect();
    let reply = msg.reply_message.as_ref().map(|r| map_reply(profiles, r));
    let forwards = msg
        .fwd_messages
        .iter()
        .map(|m| map_forward_tree(profiles, m))
        .collect::<Vec<_>>();
    let fwd_count = forwards.len();

    ChatMessage {
        id: msg.id,
        cmid: msg.conversation_message_id,
        from_id: msg.from_id,
        from_name,
        text,
        timestamp: msg.date,
        is_outgoing,
        is_read,
        is_edited: msg.update_time.is_some(),
        is_pinned: false,
        delivery: DeliveryStatus::Sent,
        attachments,
        reply,
        fwd_count,
        forwards,
    }
}

fn get_name(profiles: &[User], user_id: i64) -> String {
    profiles
        .iter()
        .find(|u| u.id == user_id)
        .map(|u| u.full_name())
        .unwrap_or_else(|| {
            if user_id < 0 {
                format!("Group {}", -user_id)
            } else {
                format!("User {}", user_id)
            }
        })
}
