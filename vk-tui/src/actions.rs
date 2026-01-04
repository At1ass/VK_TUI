//! Async action runners (VK API calls) extracted from main.rs for clarity.
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;
use vk_api::VkClient;

use crate::mapper::map_forward_tree;
use crate::mapper::{map_attachment, map_history_message, map_reply};
use crate::message::Message;
use crate::state::AttachmentInfo;

pub async fn validate_session(client: Arc<VkClient>, tx: mpsc::UnboundedSender<Message>) {
    match client.account().get_profile_info().await {
        Ok(_) => {
            let _ = tx.send(Message::SessionValidated {
                valid: true,
                error: None,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::SessionValidated {
                valid: false,
                error: Some(format!("Session validation failed: {}", e)),
            });
        }
    }
}

pub async fn load_conversations(
    client: Arc<VkClient>,
    offset: u32,
    tx: mpsc::UnboundedSender<Message>,
) {
    const COUNT: u32 = 50;

    match client.messages().get_conversations(offset, COUNT).await {
        Ok(response) => {
            let total_count = response.count as u32;
            let loaded_count = response.items.len() as u32;
            let has_more = offset + loaded_count < total_count;

            let chats: Vec<crate::state::Chat> = response
                .items
                .into_iter()
                .map(|item| {
                    let title = super::get_conversation_title(&item, &response.profiles);
                    let is_online =
                        super::get_user_online(&item.conversation.peer.id, &response.profiles);

                    crate::state::Chat {
                        id: item.conversation.peer.id,
                        title,
                        last_message: item.last_message.text.clone(),
                        last_message_time: item.last_message.date,
                        unread_count: item.conversation.unread_count.unwrap_or(0),
                        is_online,
                    }
                })
                .collect();

            let _ = tx.send(Message::ConversationsLoaded {
                chats,
                profiles: response.profiles,
                total_count,
                has_more,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load chats: {}", e)));
        }
    }
}

pub async fn load_messages(
    client: Arc<VkClient>,
    peer_id: i64,
    offset: u32,
    tx: mpsc::UnboundedSender<Message>,
) {
    const COUNT: u32 = 50;

    match client.messages().get_history(peer_id, offset, COUNT).await {
        Ok(response) => {
            let total_count = response.count as u32;
            let loaded_count = response.items.len() as u32;
            let has_more = offset + loaded_count < total_count;

            let out_read = response
                .conversations
                .first()
                .and_then(|c| c.out_read)
                .unwrap_or(0);

            let messages: Vec<crate::state::ChatMessage> = response
                .items
                .into_iter()
                .rev()
                .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                .collect();

            let _ = tx.send(Message::MessagesLoaded {
                peer_id,
                messages,
                profiles: response.profiles,
                total_count,
                has_more,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load messages: {}", e)));
        }
    }
}

/// Load messages around a specific message
pub async fn load_messages_around(
    client: Arc<VkClient>,
    peer_id: i64,
    message_id: i64,
    tx: mpsc::UnboundedSender<Message>,
) {
    const COUNT: u32 = 50;

    match client
        .messages()
        .get_history_around(peer_id, message_id, COUNT)
        .await
    {
        Ok(response) => {
            let total_count = response.count as u32;
            let has_more = true; // Always has more when loading around a message

            let out_read = response
                .conversations
                .first()
                .and_then(|c| c.out_read)
                .unwrap_or(0);

            let messages: Vec<crate::state::ChatMessage> = response
                .items
                .into_iter()
                .rev()
                .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                .collect();

            let _ = tx.send(Message::MessagesLoaded {
                peer_id,
                messages,
                profiles: response.profiles,
                total_count,
                has_more,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!(
                "Failed to load messages around target: {}",
                e
            )));
        }
    }
}

/// Load messages with offset from a specific message
/// Used for pagination (negative offset = older messages, positive = newer)
pub async fn load_messages_with_offset(
    client: Arc<VkClient>,
    peer_id: i64,
    start_message_id: i64,
    offset: i32,
    count: u32,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .get_history_with_offset(peer_id, start_message_id, offset, count)
        .await
    {
        Ok(response) => {
            let total_count = response.count as u32;
            let loaded_count = response.items.len() as u32;
            let has_more = loaded_count == count; // Has more if we got full page

            let out_read = response
                .conversations
                .first()
                .and_then(|c| c.out_read)
                .unwrap_or(0);

            let messages: Vec<crate::state::ChatMessage> = response
                .items
                .into_iter()
                .rev()
                .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                .collect();

            let _ = tx.send(Message::MessagesLoaded {
                peer_id,
                messages,
                profiles: response.profiles,
                total_count,
                has_more,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Failed to load messages: {}", e)));
        }
    }
}

pub async fn send_message(
    client: Arc<VkClient>,
    peer_id: i64,
    text: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.messages().send(peer_id, &text).await {
        Ok(sent) => {
            let _ = tx.send(Message::MessageSent(
                sent.message_id,
                sent.conversation_message_id,
            ));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!(
                "Failed to send message: {}",
                e
            )));
        }
    }
}

pub async fn send_forward(
    client: Arc<VkClient>,
    peer_id: i64,
    message_ids: Vec<i64>,
    comment: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .send_with_forward(peer_id, &comment, &message_ids)
        .await
    {
        Ok(sent) => {
            let _ = tx.send(Message::MessageSent(
                sent.message_id,
                sent.conversation_message_id,
            ));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!(
                "Failed to forward message: {}",
                e
            )));
        }
    }
}

pub async fn send_reply(
    client: Arc<VkClient>,
    peer_id: i64,
    reply_to: i64,
    text: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .send_with_reply(peer_id, &text, reply_to)
        .await
    {
        Ok(sent) => {
            let _ = tx.send(Message::MessageSent(
                sent.message_id,
                sent.conversation_message_id,
            ));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!("Failed to send reply: {}", e)));
        }
    }
}

pub async fn send_photo_attachment(
    client: Arc<VkClient>,
    peer_id: i64,
    path: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .send_photo(peer_id, Path::new(&path))
        .await
    {
        Ok(sent) => {
            let _ = tx.send(Message::MessageSent(
                sent.message_id,
                sent.conversation_message_id,
            ));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!("Failed to send photo: {}", e)));
        }
    }
}

pub async fn send_doc_attachment(
    client: Arc<VkClient>,
    peer_id: i64,
    path: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.messages().send_doc(peer_id, Path::new(&path)).await {
        Ok(sent) => {
            let _ = tx.send(Message::MessageSent(
                sent.message_id,
                sent.conversation_message_id,
            ));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!("Failed to send file: {}", e)));
        }
    }
}

pub async fn edit_message(
    client: Arc<VkClient>,
    peer_id: i64,
    message_id: i64,
    cmid: Option<i64>,
    text: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .edit(peer_id, message_id, cmid, &text)
        .await
    {
        Ok(()) => {
            let _ = tx.send(Message::MessageEdited(message_id));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!(
                "Failed to edit message: {}",
                e
            )));
        }
    }
}

pub async fn delete_message(
    client: Arc<VkClient>,
    message_id: i64,
    delete_for_all: bool,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client
        .messages()
        .delete(&[message_id], delete_for_all)
        .await
    {
        Ok(()) => {
            let _ = tx.send(Message::MessageDeleted(message_id));
        }
        Err(e) => {
            let _ = tx.send(Message::SendFailed(format!(
                "Failed to delete message: {}",
                e
            )));
        }
    }
}

pub async fn fetch_message_by_id(
    client: Arc<VkClient>,
    msg_id: i64,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.messages().get_by_id(&[msg_id]).await {
        Ok(messages) => {
            if let Some(msg) = messages.first() {
                let attachments = msg
                    .attachments
                    .clone()
                    .into_iter()
                    .map(map_attachment)
                    .collect::<Vec<_>>();
                let reply = msg.reply_message.as_ref().map(|r| map_reply(&[], r));
                let forwards = msg
                    .fwd_messages
                    .iter()
                    .map(|m| map_forward_tree(&[], m))
                    .collect::<Vec<_>>();
                let fwd_count = forwards.len();

                let _ = tx.send(Message::MessageDetailsFetched {
                    message_id: msg.id,
                    cmid: msg.conversation_message_id,
                    text: Some(msg.text.clone()),
                    is_edited: msg.update_time.is_some(),
                    attachments: Some(attachments),
                    reply,
                    fwd_count: Some(fwd_count),
                    forwards: Some(forwards),
                });
            }
        }
        Err(e) => {
            tracing::warn!("Failed to fetch message details: {}", e);
        }
    }
}

pub async fn download_attachments(atts: Vec<AttachmentInfo>, tx: mpsc::UnboundedSender<Message>) {
    let Some(base_dir) = directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
        .or_else(|| Some(std::env::temp_dir()))
    else {
        let _ = tx.send(Message::Error("No download directory available".into()));
        return;
    };

    if std::fs::create_dir_all(&base_dir).is_err() {
        let _ = tx.send(Message::Error("Failed to create download directory".into()));
        return;
    }

    let client = reqwest::Client::new();

    for (idx, att) in atts.into_iter().enumerate() {
        let Some(url) = att.url.clone() else {
            continue;
        };

        let name = if !att.title.is_empty() {
            att.title.clone()
        } else {
            format!("attachment_{}", idx)
        };

        let path = base_dir.join(name);

        match client.get(&url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    if let Err(e) = std::fs::write(&path, &bytes) {
                        let _ = tx.send(Message::Error(format!(
                            "Failed to save {}: {}",
                            path.display(),
                            e
                        )));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Message::Error(format!("Download failed: {}", e)));
                }
            },
            Err(e) => {
                let _ = tx.send(Message::Error(format!("Download failed: {}", e)));
            }
        }
    }
}

/// Search messages globally
pub async fn search_messages(
    client: Arc<VkClient>,
    query: String,
    tx: mpsc::UnboundedSender<Message>,
) {
    match client.messages().search(&query, None, 20).await {
        Ok(response) => {
            let mut results = Vec::new();

            // Create a map of conversations for quick lookup
            let conversations: std::collections::HashMap<i64, &vk_api::Conversation> = response
                .conversations
                .iter()
                .map(|conv| (conv.peer.id, conv))
                .collect();

            // Create a map of users for quick lookup
            let users: std::collections::HashMap<i64, &vk_api::User> = response
                .profiles
                .iter()
                .map(|user| (user.id, user))
                .collect();

            for msg in response.items {
                let peer_id = msg.peer_id;
                let from_id = msg.from_id;

                // Get chat title
                let chat_title = conversations
                    .get(&peer_id)
                    .and_then(|conv| {
                        conv.chat_settings
                            .as_ref()
                            .map(|s| s.title.clone())
                            .or_else(|| {
                                // For DM, use user name
                                users.get(&peer_id).map(|u| u.full_name())
                            })
                    })
                    .unwrap_or_else(|| format!("Chat {}", peer_id));

                // Get sender name
                let from_name = users
                    .get(&from_id)
                    .map(|u| u.full_name())
                    .unwrap_or_else(|| format!("User {}", from_id));

                results.push(crate::state::SearchResult {
                    message_id: msg.id,
                    peer_id,
                    from_id,
                    from_name,
                    chat_title,
                    text: msg.text,
                    timestamp: msg.date,
                });
            }

            let _ = tx.send(Message::SearchResultsLoaded {
                results,
                total_count: response.count,
            });
        }
        Err(e) => {
            let _ = tx.send(Message::Error(format!("Search failed: {}", e)));
        }
    }
}
