use crate::event::VkEvent;
use serde_json::Value;

/// Parse a single longpoll update into VkEvent, if applicable.
pub fn handle_update(update: &Value) -> Option<VkEvent> {
    let arr = update.as_array()?;
    let event_type = arr.first().and_then(|v| v.as_i64())?;

    match event_type {
        2 => {
            // Message deleted: [2, message_id, flags, peer_id]
            let message_id = arr.get(1).and_then(|v| v.as_i64())?;
            let peer_id = arr.get(3).and_then(|v| v.as_i64())?;
            Some(VkEvent::MessageDeletedFromLongPoll {
                peer_id,
                message_id,
            })
        }
        4 => {
            // New message: [4, message_id, flags, peer_id, timestamp, text, extra, attachments]
            let peer_id = arr.get(3).and_then(|v| v.as_i64())?;
            let text = arr
                .get(5)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let extra = arr.get(6);
            let from_id = extra
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("from"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .or(Some(peer_id))?;
            Some(VkEvent::NewMessage {
                peer_id,
                text,
                from_id,
            })
        }
        5 => {
            // Message flags changed (used as edit marker): [5, message_id, flags, peer_id, ...]
            let message_id = arr.get(1).and_then(|v| v.as_i64())?;
            let peer_id = arr.get(3).and_then(|v| v.as_i64())?;
            Some(VkEvent::MessageEditedFromLongPoll {
                peer_id,
                message_id,
            })
        }
        61 => {
            // User typing in private dialog: [61, user_id, flags]
            let user_id = arr.get(1).and_then(|v| v.as_i64())?;
            Some(VkEvent::UserTyping {
                peer_id: user_id,
                user_id,
            })
        }
        62 => {
            // User typing in chat: [62, user_id, chat_id]
            let user_id = arr.get(1).and_then(|v| v.as_i64())?;
            let chat_id = arr.get(2).and_then(|v| v.as_i64())?;
            let peer_id = 2000000000 + chat_id;
            Some(VkEvent::UserTyping { peer_id, user_id })
        }
        6 | 7 => {
            // Message read events: [6/7, peer_id, message_id, ...]
            let peer_id = arr.get(1).and_then(|v| v.as_i64())?;
            let message_id = arr.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
            Some(VkEvent::MessageRead {
                peer_id,
                message_id,
            })
        }
        _ => None,
    }
}
