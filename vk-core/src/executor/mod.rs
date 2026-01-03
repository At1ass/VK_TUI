//! Command executor for async VK API operations.
//!
//! This module handles all async operations and sends results
//! back to frontends via events.

use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;
use vk_api::VkClient;

use crate::commands::AsyncCommand;
use crate::events::CoreEvent;
use crate::mapper::{map_attachment, map_forward_tree, map_history_message, map_reply};
use crate::models::{AttachmentInfo, Chat, SearchResult};

/// Executes async commands and sends events to frontends.
pub struct CommandExecutor {
    client: Arc<VkClient>,
    event_tx: mpsc::UnboundedSender<CoreEvent>,
}

impl CommandExecutor {
    /// Create a new command executor.
    pub fn new(client: Arc<VkClient>, event_tx: mpsc::UnboundedSender<CoreEvent>) -> Self {
        Self { client, event_tx }
    }

    /// Execute an async command.
    pub async fn execute(&self, cmd: AsyncCommand) {
        match cmd {
            AsyncCommand::LoadConversations { offset } => {
                self.load_conversations(offset).await;
            }
            AsyncCommand::LoadMessages { peer_id, offset } => {
                self.load_messages(peer_id, offset).await;
            }
            AsyncCommand::LoadMessagesAround {
                peer_id,
                message_id,
            } => {
                self.load_messages_around(peer_id, message_id).await;
            }
            AsyncCommand::LoadMessagesWithOffset {
                peer_id,
                start_cmid,
                offset,
                count,
            } => {
                self.load_messages_with_offset(peer_id, start_cmid, offset, count)
                    .await;
            }
            AsyncCommand::SendMessage { peer_id, text } => {
                self.send_message(peer_id, text).await;
            }
            AsyncCommand::SendReply {
                peer_id,
                reply_to,
                text,
            } => {
                self.send_reply(peer_id, reply_to, text).await;
            }
            AsyncCommand::SendForward {
                peer_id,
                message_ids,
                comment,
            } => {
                self.send_forward(peer_id, message_ids, comment).await;
            }
            AsyncCommand::EditMessage {
                peer_id,
                message_id,
                cmid,
                text,
            } => {
                self.edit_message(peer_id, message_id, cmid, text).await;
            }
            AsyncCommand::DeleteMessage {
                message_id,
                for_all,
                ..
            } => {
                self.delete_message(message_id, for_all).await;
            }
            AsyncCommand::SendPhoto { peer_id, path } => {
                self.send_photo(peer_id, &path).await;
            }
            AsyncCommand::SendDoc { peer_id, path } => {
                self.send_doc(peer_id, &path).await;
            }
            AsyncCommand::DownloadAttachments { attachments } => {
                self.download_attachments(attachments).await;
            }
            AsyncCommand::SearchMessages { query } => {
                self.search_messages(query).await;
            }
            AsyncCommand::FetchMessageById { message_id } => {
                self.fetch_message_by_id(message_id).await;
            }
            AsyncCommand::StartLongPoll | AsyncCommand::MarkAsRead { .. } => {
                // Handled elsewhere or no-op for now
            }
        }
    }

    fn send_event(&self, event: CoreEvent) {
        let _ = self.event_tx.send(event);
    }

    async fn load_conversations(&self, offset: u32) {
        const COUNT: u32 = 50;

        match self
            .client
            .messages()
            .get_conversations(offset, COUNT)
            .await
        {
            Ok(response) => {
                let total_count = response.count as u32;
                let loaded_count = response.items.len() as u32;
                let has_more = offset + loaded_count < total_count;

                let chats: Vec<Chat> = response
                    .items
                    .into_iter()
                    .map(|item| {
                        let title = get_conversation_title(&item, &response.profiles);
                        let is_online =
                            get_user_online(&item.conversation.peer.id, &response.profiles);

                        Chat {
                            id: item.conversation.peer.id,
                            title,
                            last_message: item.last_message.text.clone(),
                            last_message_time: item.last_message.date,
                            unread_count: item.conversation.unread_count.unwrap_or(0),
                            is_online,
                        }
                    })
                    .collect();

                self.send_event(CoreEvent::ConversationsLoaded {
                    chats,
                    profiles: response.profiles,
                    total_count,
                    has_more,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::Error(format!("Failed to load chats: {}", e)));
            }
        }
    }

    async fn load_messages(&self, peer_id: i64, offset: u32) {
        const COUNT: u32 = 50;

        match self
            .client
            .messages()
            .get_history(peer_id, offset, COUNT)
            .await
        {
            Ok(response) => {
                let total_count = response.count as u32;
                let loaded_count = response.items.len() as u32;
                let has_more = offset + loaded_count < total_count;

                let out_read = response
                    .conversations
                    .first()
                    .and_then(|c| c.out_read)
                    .unwrap_or(0);

                let messages = response
                    .items
                    .into_iter()
                    .rev()
                    .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                    .collect();

                self.send_event(CoreEvent::MessagesLoaded {
                    peer_id,
                    messages,
                    profiles: response.profiles,
                    total_count,
                    has_more,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::Error(format!("Failed to load messages: {}", e)));
            }
        }
    }

    async fn load_messages_around(&self, peer_id: i64, message_id: i64) {
        const COUNT: u32 = 50;

        match self
            .client
            .messages()
            .get_history_around(peer_id, message_id, COUNT)
            .await
        {
            Ok(response) => {
                let total_count = response.count as u32;
                let has_more = true;

                let out_read = response
                    .conversations
                    .first()
                    .and_then(|c| c.out_read)
                    .unwrap_or(0);

                let messages = response
                    .items
                    .into_iter()
                    .rev()
                    .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                    .collect();

                self.send_event(CoreEvent::MessagesLoaded {
                    peer_id,
                    messages,
                    profiles: response.profiles,
                    total_count,
                    has_more,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::Error(format!(
                    "Failed to load messages around target: {}",
                    e
                )));
            }
        }
    }

    async fn load_messages_with_offset(
        &self,
        peer_id: i64,
        start_cmid: i64,
        offset: i32,
        count: u32,
    ) {
        match self
            .client
            .messages()
            .get_history_with_offset(peer_id, start_cmid, offset, count)
            .await
        {
            Ok(response) => {
                let total_count = response.count as u32;
                let loaded_count = response.items.len() as u32;
                let has_more = loaded_count == count;

                let out_read = response
                    .conversations
                    .first()
                    .and_then(|c| c.out_read)
                    .unwrap_or(0);

                let messages = response
                    .items
                    .into_iter()
                    .rev()
                    .map(|msg| map_history_message(&response.profiles, &msg, out_read))
                    .collect();

                self.send_event(CoreEvent::MessagesLoaded {
                    peer_id,
                    messages,
                    profiles: response.profiles,
                    total_count,
                    has_more,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::Error(format!("Failed to load messages: {}", e)));
            }
        }
    }

    async fn send_message(&self, peer_id: i64, text: String) {
        match self.client.messages().send(peer_id, &text).await {
            Ok(sent) => {
                self.send_event(CoreEvent::MessageSent {
                    message_id: sent.message_id,
                    cmid: sent.conversation_message_id,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to send message: {}",
                    e
                )));
            }
        }
    }

    async fn send_reply(&self, peer_id: i64, reply_to: i64, text: String) {
        match self
            .client
            .messages()
            .send_with_reply(peer_id, &text, reply_to)
            .await
        {
            Ok(sent) => {
                self.send_event(CoreEvent::MessageSent {
                    message_id: sent.message_id,
                    cmid: sent.conversation_message_id,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to send reply: {}",
                    e
                )));
            }
        }
    }

    async fn send_forward(&self, peer_id: i64, message_ids: Vec<i64>, comment: String) {
        match self
            .client
            .messages()
            .send_with_forward(peer_id, &comment, &message_ids)
            .await
        {
            Ok(sent) => {
                self.send_event(CoreEvent::MessageSent {
                    message_id: sent.message_id,
                    cmid: sent.conversation_message_id,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to forward message: {}",
                    e
                )));
            }
        }
    }

    async fn edit_message(&self, peer_id: i64, message_id: i64, cmid: Option<i64>, text: String) {
        match self
            .client
            .messages()
            .edit(peer_id, message_id, cmid, &text)
            .await
        {
            Ok(()) => {
                self.send_event(CoreEvent::MessageEdited { message_id });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to edit message: {}",
                    e
                )));
            }
        }
    }

    async fn delete_message(&self, message_id: i64, for_all: bool) {
        match self.client.messages().delete(&[message_id], for_all).await {
            Ok(()) => {
                self.send_event(CoreEvent::MessageDeleted { message_id });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to delete message: {}",
                    e
                )));
            }
        }
    }

    async fn send_photo(&self, peer_id: i64, path: &Path) {
        match self.client.messages().send_photo(peer_id, path).await {
            Ok(sent) => {
                self.send_event(CoreEvent::MessageSent {
                    message_id: sent.message_id,
                    cmid: sent.conversation_message_id,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!(
                    "Failed to send photo: {}",
                    e
                )));
            }
        }
    }

    async fn send_doc(&self, peer_id: i64, path: &Path) {
        match self.client.messages().send_doc(peer_id, path).await {
            Ok(sent) => {
                self.send_event(CoreEvent::MessageSent {
                    message_id: sent.message_id,
                    cmid: sent.conversation_message_id,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::SendFailed(format!("Failed to send file: {}", e)));
            }
        }
    }

    async fn download_attachments(&self, attachments: Vec<AttachmentInfo>) {
        let Some(base_dir) = directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|p| p.to_path_buf()))
            .or_else(|| Some(std::env::temp_dir()))
        else {
            self.send_event(CoreEvent::Error("No download directory available".into()));
            return;
        };

        if std::fs::create_dir_all(&base_dir).is_err() {
            self.send_event(CoreEvent::Error(
                "Failed to create download directory".into(),
            ));
            return;
        }

        let client = reqwest::Client::new();

        for (idx, att) in attachments.into_iter().enumerate() {
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
                            self.send_event(CoreEvent::Error(format!(
                                "Failed to save {}: {}",
                                path.display(),
                                e
                            )));
                        }
                    }
                    Err(e) => {
                        self.send_event(CoreEvent::Error(format!("Download failed: {}", e)));
                    }
                },
                Err(e) => {
                    self.send_event(CoreEvent::Error(format!("Download failed: {}", e)));
                }
            }
        }
    }

    async fn search_messages(&self, query: String) {
        match self.client.messages().search(&query, None, 20).await {
            Ok(response) => {
                let mut results = Vec::new();

                let conversations: std::collections::HashMap<i64, &vk_api::Conversation> = response
                    .conversations
                    .iter()
                    .map(|conv| (conv.peer.id, conv))
                    .collect();

                let users: std::collections::HashMap<i64, &vk_api::User> = response
                    .profiles
                    .iter()
                    .map(|user| (user.id, user))
                    .collect();

                for msg in response.items {
                    let peer_id = msg.peer_id;
                    let from_id = msg.from_id;

                    let chat_title = conversations
                        .get(&peer_id)
                        .and_then(|conv| {
                            conv.chat_settings
                                .as_ref()
                                .map(|s| s.title.clone())
                                .or_else(|| users.get(&peer_id).map(|u| u.full_name()))
                        })
                        .unwrap_or_else(|| format!("Chat {}", peer_id));

                    let from_name = users
                        .get(&from_id)
                        .map(|u| u.full_name())
                        .unwrap_or_else(|| format!("User {}", from_id));

                    results.push(SearchResult {
                        message_id: msg.id,
                        peer_id,
                        from_id,
                        from_name,
                        chat_title,
                        text: msg.text,
                        timestamp: msg.date,
                    });
                }

                self.send_event(CoreEvent::SearchResultsLoaded {
                    results,
                    total_count: response.count,
                });
            }
            Err(e) => {
                self.send_event(CoreEvent::Error(format!("Search failed: {}", e)));
            }
        }
    }

    async fn fetch_message_by_id(&self, message_id: i64) {
        match self.client.messages().get_by_id(&[message_id]).await {
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

                    self.send_event(CoreEvent::MessageDetailsFetched {
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
}

// === Helper functions ===

/// Get conversation title from response.
fn get_conversation_title(item: &vk_api::ConversationItem, profiles: &[vk_api::User]) -> String {
    if let Some(settings) = &item.conversation.chat_settings {
        return settings.title.clone();
    }

    let peer_id = item.conversation.peer.id;
    profiles
        .iter()
        .find(|u| u.id == peer_id)
        .map(|u| u.full_name())
        .unwrap_or_else(|| format!("Chat {}", peer_id))
}

/// Check if user is online.
fn get_user_online(peer_id: &i64, profiles: &[vk_api::User]) -> bool {
    profiles
        .iter()
        .find(|u| u.id == *peer_id)
        .and_then(|u| u.online)
        .map(|v| v != 0)
        .unwrap_or(false)
}
