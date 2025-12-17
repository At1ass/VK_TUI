//! Messages API implementation
//!
//! Provides methods for working with VK messages, conversations, and related functionality.
//! References: https://dev.vk.com/method/messages

use anyhow::{Context, Result};
use rand::Rng;
use std::collections::HashMap;
use std::path::Path;

use crate::client::VkClient;
use crate::types::*;
use serde_json::Value;

/// Messages API namespace
pub struct MessagesApi<'a> {
    client: &'a VkClient,
}

impl<'a> MessagesApi<'a> {
    pub(crate) fn new(client: &'a VkClient) -> Self {
        Self { client }
    }

    // ========== Conversations ==========

    /// Get list of conversations
    ///
    /// # Arguments
    /// * `offset` - Offset for pagination (default: 0)
    /// * `count` - Number of conversations to return (max: 200, default: 20)
    ///
    /// # Returns
    /// ConversationsResponse with items and profiles
    ///
    /// # VK API
    /// Method: messages.getConversations
    /// https://dev.vk.com/method/messages.getConversations
    pub async fn get_conversations(
        &self,
        offset: u32,
        count: u32,
    ) -> Result<ConversationsResponse> {
        let mut params = HashMap::new();
        params.insert("offset", offset.to_string());
        params.insert("count", count.to_string());
        params.insert("extended", "1".to_string());

        self.client
            .request("messages.getConversations", params)
            .await
    }

    /// Get conversation by peer_id
    ///
    /// # VK API
    /// Method: messages.getConversationsById
    /// https://dev.vk.com/method/messages.getConversationsById
    pub async fn get_conversation_by_id(&self, peer_id: i64) -> Result<Conversation> {
        let conversations = self.get_conversations_by_ids(&[peer_id]).await?;
        conversations
            .into_iter()
            .next()
            .context("Conversation not found")
    }

    /// Get conversations by multiple IDs
    ///
    /// # Arguments
    /// * `peer_ids` - Array of peer IDs (up to 100)
    ///
    /// # VK API
    /// Method: messages.getConversationsById
    /// https://dev.vk.com/method/messages.getConversationsById
    pub async fn get_conversations_by_ids(&self, peer_ids: &[i64]) -> Result<Vec<Conversation>> {
        let mut params = HashMap::new();
        // VK API expects comma-separated list of IDs
        let peer_ids_str = peer_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        params.insert("peer_ids", peer_ids_str);

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            #[allow(dead_code)]
            count: i32,
            items: Vec<Conversation>,
        }

        let response: Response = self
            .client
            .request("messages.getConversationsById", params)
            .await?;

        Ok(response.items)
    }

    // ========== Messages ==========

    /// Get message history for a conversation
    ///
    /// # Arguments
    /// * `peer_id` - Peer ID (user_id for DM, 2000000000+chat_id for group chats)
    /// * `offset` - Offset for pagination
    /// * `count` - Number of messages (max: 200)
    ///
    /// # Returns
    /// MessagesHistoryResponse with messages and profiles
    ///
    /// # VK API
    /// Method: messages.getHistory
    /// https://dev.vk.com/method/messages.getHistory
    pub async fn get_history(
        &self,
        peer_id: i64,
        offset: u32,
        count: u32,
    ) -> Result<MessagesHistoryResponse> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        params.insert("offset", offset.to_string());
        params.insert("count", count.to_string());
        params.insert("extended", "1".to_string());

        self.client.request("messages.getHistory", params).await
    }

    /// Get message by conversation_message_id
    ///
    /// # VK API
    /// Method: messages.getByConversationMessageId
    /// https://dev.vk.com/method/messages.getByConversationMessageId
    pub async fn get_by_conversation_message_id(
        &self,
        peer_id: i64,
        conversation_message_ids: &[i64],
    ) -> Result<Vec<Message>> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        let ids: Vec<String> = conversation_message_ids
            .iter()
            .map(|id| id.to_string())
            .collect();
        params.insert("conversation_message_ids", ids.join(","));

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<Message>,
        }

        let response: Response = self
            .client
            .request("messages.getByConversationMessageId", params)
            .await?;

        Ok(response.items)
    }

    // ========== Send Messages ==========

    /// Send text message
    ///
    /// # Arguments
    /// * `peer_id` - Recipient peer ID
    /// * `message` - Message text (max: 4096 chars)
    ///
    /// # Returns
    /// SentMessage with message_id and conversation_message_id
    ///
    /// # VK API
    /// Method: messages.send
    /// https://dev.vk.com/method/messages.send
    pub async fn send(&self, peer_id: i64, message: &str) -> Result<SentMessage> {
        self.send_with_params(peer_id, message, None, None, None)
            .await
    }

    /// Send message with reply
    ///
    /// # VK API
    /// Method: messages.send (with reply_to parameter)
    pub async fn send_with_reply(
        &self,
        peer_id: i64,
        message: &str,
        reply_to: i64,
    ) -> Result<SentMessage> {
        self.send_with_params(peer_id, message, Some(reply_to), None, None)
            .await
    }

    /// Send message with forward
    ///
    /// # VK API
    /// Method: messages.send (with forward_messages parameter)
    pub async fn send_with_forward(
        &self,
        peer_id: i64,
        message: &str,
        forward_messages: &[i64],
    ) -> Result<SentMessage> {
        self.send_with_params(peer_id, message, None, Some(forward_messages), None)
            .await
    }

    /// Send message with attachment
    ///
    /// # Arguments
    /// * `attachment` - Attachment string (e.g., "photo123_456" or "doc123_456")
    ///
    /// # VK API
    /// Method: messages.send (with attachment parameter)
    pub async fn send_with_attachment(
        &self,
        peer_id: i64,
        message: &str,
        attachment: &str,
    ) -> Result<SentMessage> {
        self.send_with_params(peer_id, message, None, None, Some(attachment))
            .await
    }

    /// Internal method to send message with various parameters
    async fn send_with_params(
        &self,
        peer_id: i64,
        message: &str,
        reply_to: Option<i64>,
        forward_messages: Option<&[i64]>,
        attachment: Option<&str>,
    ) -> Result<SentMessage> {
        let mut params = HashMap::new();
        // Use peer_id as in web version
        params.insert("peer_id", peer_id.to_string());

        if !message.is_empty() {
            params.insert("message", message.to_string());
        }

        if let Some(reply) = reply_to {
            params.insert("reply_to", reply.to_string());
        }

        if let Some(fwd) = forward_messages {
            let fwd_ids: Vec<String> = fwd.iter().map(|id| id.to_string()).collect();
            params.insert("forward_messages", fwd_ids.join(","));
        }

        if let Some(att) = attachment {
            params.insert("attachment", att.to_string());
        }

        params.insert("random_id", generate_random_id().to_string());

        // Parse response as object with cmid and message_id
        // VK can return either an object with {message_id, cmid} or a plain integer (message_id)
        let response: serde_json::Value = self.client.request("messages.send", params).await?;

        if let Some(obj) = response.as_object() {
            let message_id = obj
                .get("message_id")
                .and_then(|v| v.as_i64())
                .context("messages.send response missing message_id")?;
            let cmid = obj.get("cmid").and_then(|v| v.as_i64()).unwrap_or(0);

            Ok(SentMessage {
                message_id,
                conversation_message_id: cmid,
            })
        } else if let Some(message_id) = response.as_i64() {
            Ok(SentMessage {
                message_id,
                conversation_message_id: 0,
            })
        } else {
            anyhow::bail!("Unexpected messages.send response shape: {}", response);
        }
    }

    // ========== Edit/Delete ==========

    /// Edit message
    ///
    /// # Arguments
    /// * `peer_id` - Peer ID
    /// * `message_id` - Global message ID
    /// * `cmid` - Optional conversation message ID (for chats)
    /// * `message` - New message text
    ///
    /// # VK API
    /// Method: messages.edit
    /// https://dev.vk.com/method/messages.edit
    pub async fn edit(
        &self,
        peer_id: i64,
        message_id: i64,
        cmid: Option<i64>,
        message: &str,
    ) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        match cmid {
            Some(cmid) => {
                // VK requires either message_id OR cmid, not both
                params.insert("cmid", cmid.to_string());
            }
            None => {
                params.insert("message_id", message_id.to_string());
            }
        };
        params.insert("message", message.to_string());
        // Preserve forwards/snippets to mirror web behavior
        params.insert("keep_forward_messages", "1".into());
        params.insert("keep_snippets", "1".into());

        let _: i32 = self.client.request("messages.edit", params).await?;
        Ok(())
    }

    /// Delete messages
    ///
    /// # Arguments
    /// * `message_ids` - IDs of messages to delete
    /// * `delete_for_all` - Delete for all participants (only for own messages)
    ///
    /// # VK API
    /// Method: messages.delete
    /// https://dev.vk.com/method/messages.delete
    pub async fn delete(&self, message_ids: &[i64], delete_for_all: bool) -> Result<()> {
        let mut params = HashMap::new();
        let ids: Vec<String> = message_ids.iter().map(|id| id.to_string()).collect();
        params.insert("message_ids", ids.join(","));

        if delete_for_all {
            params.insert("delete_for_all", "1".to_string());
        }

        let _: serde_json::Value = self.client.request("messages.delete", params).await?;
        Ok(())
    }

    /// Get messages by their IDs
    ///
    /// # Arguments
    /// * `message_ids` - Array of message IDs (up to 100)
    ///
    /// # VK API
    /// Method: messages.getById
    /// https://dev.vk.com/method/messages.getById
    pub async fn get_by_id(&self, message_ids: &[i64]) -> Result<Vec<Message>> {
        let mut params = HashMap::new();
        let ids: Vec<String> = message_ids.iter().map(|id| id.to_string()).collect();
        params.insert("message_ids", ids.join(","));

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<Message>,
        }

        let response: Response = self.client.request("messages.getById", params).await?;

        Ok(response.items)
    }

    // ========== Search ==========

    /// Search messages
    ///
    /// # Arguments
    /// * `query` - Search query
    /// * `peer_id` - Search in specific conversation (None for global search)
    /// * `count` - Number of results (max: 100)
    ///
    /// # VK API
    /// Method: messages.search
    /// https://dev.vk.com/method/messages.search
    pub async fn search(
        &self,
        query: &str,
        peer_id: Option<i64>,
        count: u32,
    ) -> Result<Vec<Message>> {
        let mut params = HashMap::new();
        params.insert("q", query.to_string());
        params.insert("count", count.to_string());

        if let Some(pid) = peer_id {
            params.insert("peer_id", pid.to_string());
        }

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<Message>,
        }

        let response: Response = self.client.request("messages.search", params).await?;
        Ok(response.items)
    }

    /// Search conversations
    ///
    /// Returns a list of conversations that match search criteria.
    /// Useful for finding specific chats by name or title.
    ///
    /// # Arguments
    /// * `query` - Search query (name, chat title, etc.)
    /// * `count` - Number of results (max: 255, default: 20)
    ///
    /// # Returns
    /// ConversationsResponse with matching conversations and profiles
    ///
    /// # Example
    /// ```no_run
    /// # use vk_api::VkClient;
    /// # async fn example(client: VkClient) -> anyhow::Result<()> {
    /// // Find all chats with "Ivan" in the name
    /// let result = client.messages().search_conversations("Ivan", 20).await?;
    /// for item in result.items {
    ///     println!("Found chat: {:?}", item.conversation.peer.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # VK API
    /// Method: messages.searchConversations
    /// https://dev.vk.com/method/messages.searchConversations
    pub async fn search_conversations(&self, query: &str, count: u32) -> Result<Vec<Conversation>> {
        let mut params = HashMap::new();
        params.insert("q", query.to_string());
        params.insert("count", count.to_string());
        // Note: extended parameter doesn't work for searchConversations
        // It returns only conversations without profiles/groups

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            #[allow(dead_code)]
            count: i32,
            items: Vec<Conversation>,
        }

        let response: Response = self
            .client
            .request("messages.searchConversations", params)
            .await?;

        Ok(response.items)
    }

    // ========== Pin/Unpin ==========

    /// Pin message in conversation
    ///
    /// # VK API
    /// Method: messages.pin
    /// https://dev.vk.com/method/messages.pin
    pub async fn pin(&self, peer_id: i64, message_id: i64) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        params.insert("conversation_message_id", message_id.to_string());

        let _: serde_json::Value = self.client.request("messages.pin", params).await?;
        Ok(())
    }

    /// Unpin message in conversation
    ///
    /// # VK API
    /// Method: messages.unpin
    /// https://dev.vk.com/method/messages.unpin
    pub async fn unpin(&self, peer_id: i64) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());

        let _: serde_json::Value = self.client.request("messages.unpin", params).await?;
        Ok(())
    }

    // ========== Read Status ==========

    /// Mark messages as read
    ///
    /// # VK API
    /// Method: messages.markAsRead
    /// https://dev.vk.com/method/messages.markAsRead
    pub async fn mark_as_read(&self, peer_id: i64) -> Result<i32> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());

        self.client.request("messages.markAsRead", params).await
    }

    // ========== Activity ==========

    /// Set typing/recording activity
    ///
    /// # VK API
    /// Method: messages.setActivity
    /// https://dev.vk.com/method/messages.setActivity
    pub async fn set_activity(&self, peer_id: i64, activity_type: ActivityType) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        params.insert("type", activity_type.as_str().to_string());

        let _: i32 = self.client.request("messages.setActivity", params).await?;
        Ok(())
    }

    // ========== Reactions ==========

    /// Send reaction to message
    ///
    /// # VK API
    /// Method: messages.sendReaction
    /// https://dev.vk.com/method/messages.sendReaction
    pub async fn send_reaction(&self, peer_id: i64, cmid: i64, reaction_id: i64) -> Result<()> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        params.insert("cmid", cmid.to_string());
        params.insert("reaction_id", reaction_id.to_string());

        let _: i32 = self.client.request("messages.sendReaction", params).await?;
        Ok(())
    }

    /// Get available reaction assets
    ///
    /// # VK API
    /// Method: messages.getReactionsAssets
    /// https://dev.vk.com/method/messages.getReactionsAssets
    pub async fn get_reactions_assets(&self) -> Result<Vec<Reaction>> {
        let params = HashMap::new();

        #[derive(Debug, serde::Deserialize)]
        struct Response {
            items: Vec<Reaction>,
        }

        let response: Response = self
            .client
            .request("messages.getReactionsAssets", params)
            .await?;
        Ok(response.items)
    }

    // ========== Upload Methods ==========

    /// Send photo to peer (combines upload + save + send)
    ///
    /// This is a convenience method that:
    /// 1. Gets upload server
    /// 2. Uploads photo
    /// 3. Saves photo
    /// 4. Sends message with photo attachment
    pub async fn send_photo(&self, peer_id: i64, photo_path: &Path) -> Result<SentMessage> {
        // Get upload server
        let mut server_params = HashMap::new();
        server_params.insert("peer_id", peer_id.to_string());
        let upload_server: UploadServer = self
            .client
            .request("photos.getMessagesUploadServer", server_params)
            .await?;

        // Upload photo
        let (boundary, body) = build_multipart_body(photo_path, "photo")?;
        let response = self
            .client
            .http_client()
            .post(&upload_server.upload_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .context("Photo upload failed")?;

        let response_text = response.text().await?;

        // Parse upload response
        let upload_json: serde_json::Value = serde_json::from_str(&response_text)
            .context("Failed to parse photo upload response")?;

        // Save photo
        let mut save_params: HashMap<&str, String> = HashMap::new();
        if let Some(obj) = upload_json.as_object() {
            for (key, value) in obj {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => value.to_string(),
                };
                save_params.insert(key, value_str);
            }
        }

        let saved: Vec<SavedPhoto> = self
            .client
            .request("photos.saveMessagesPhoto", save_params)
            .await?;

        let attachment = saved
            .first()
            .map(|p| format!("photo{}_{}", p.owner_id, p.id))
            .context("No saved photo returned")?;

        // Send message with attachment
        self.send_with_attachment(peer_id, "", &attachment).await
    }

    /// Send document to peer (combines upload + save + send)
    ///
    /// This is a convenience method that:
    /// 1. Gets upload server
    /// 2. Uploads document
    /// 3. Saves document
    /// 4. Sends message with document attachment
    pub async fn send_doc(&self, peer_id: i64, doc_path: &Path) -> Result<SentMessage> {
        // Get upload server
        let mut params = HashMap::new();
        params.insert("type", "doc".to_string());
        params.insert("peer_id", peer_id.to_string());
        let upload_server: UploadServer = self
            .client
            .request("docs.getMessagesUploadServer", params)
            .await?;

        // Upload doc
        let (boundary, body) = build_multipart_body(doc_path, "file")?;
        let response = self
            .client
            .http_client()
            .post(&upload_server.upload_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .context("Doc upload failed")?;

        let response_text = response.text().await?;
        let upload_json: serde_json::Value = serde_json::from_str(&response_text).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse doc upload response: {}; body: {}",
                e,
                response_text
            )
        })?;

        // VK may return {file}, or {error, error_descr}
        if let Some(err) = upload_json.get("error").and_then(|v| v.as_str()) {
            let descr = upload_json
                .get("error_descr")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            anyhow::bail!(
                "Upload error from VK: {}{}",
                err,
                if descr.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", descr)
                }
            );
        }

        let file_id = upload_json
            .get("file")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .context(format!(
                "Upload response missing file id; body: {}",
                response_text
            ))?;

        // Save doc
        let mut save_params: HashMap<&str, String> = HashMap::new();
        save_params.insert("file", file_id);

        // Pass filename to docs.save so it is not "untitled"
        if let Some(title) = doc_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
        {
            save_params.insert("title", title);
        }

        let saved: Value = self.client.request("docs.save", save_params).await?;
        let attachment = extract_doc_attachment(&saved)?;

        // Send message with attachment
        self.send_with_attachment(peer_id, "", &attachment).await
    }
}

/// Activity types for setActivity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    Typing,
    AudioMessage,
}

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::Typing => "typing",
            ActivityType::AudioMessage => "audiomessage",
        }
    }
}

/// Generate random message ID for VK API
fn generate_random_id() -> i64 {
    let mut rng = rand::thread_rng();
    let mut id: i64 = rng.r#gen::<u32>() as i64;
    if id == 0 {
        id = 1;
    }
    id
}

/// Build a simple multipart/form-data body with a single file part
fn build_multipart_body(path: &Path, field_name: &str) -> Result<(String, Vec<u8>)> {
    use std::io::Write;

    const MAX_UPLOAD_BYTES: u64 = 50 * 1024 * 1024; // 50 MB soft limit for TUI

    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_UPLOAD_BYTES {
        anyhow::bail!(
            "File is too large ({} bytes, limit {} bytes)",
            metadata.len(),
            MAX_UPLOAD_BYTES
        );
    }

    let boundary = format!("vk_api_boundary_{}", generate_random_id());
    let mut body = Vec::new();
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.bin");
    let data = std::fs::read(path)?;
    let content_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    write!(body, "--{}\r\n", boundary)?;
    write!(
        body,
        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
        field_name, filename
    )?;
    write!(body, "Content-Type: {}\r\n\r\n", content_type)?;
    body.extend_from_slice(&data);
    write!(body, "\r\n--{}--\r\n", boundary)?;

    Ok((boundary, body))
}

fn extract_doc_attachment(value: &Value) -> Result<String> {
    // docs.save may return an array or an object {response:{type, doc}}
    if let Some(obj) = value.get("response") {
        return extract_doc_attachment(obj);
    }

    if let Some(arr) = value.as_array()
        && let Some(first) = arr.first()
        && let Some(doc_obj) = first
            .get("doc")
            .or_else(|| first.as_object().map(|_| first))
        && let (Some(owner_id), Some(id)) = (
            doc_obj.get("owner_id").and_then(|v| v.as_i64()),
            doc_obj.get("id").and_then(|v| v.as_i64()),
        )
    {
        return Ok(format!("doc{}_{}", owner_id, id));
    }

    if let Some(doc_obj) = value
        .get("doc")
        .or_else(|| value.as_object().map(|_| value))
        && let (Some(owner_id), Some(id)) = (
            doc_obj.get("owner_id").and_then(|v| v.as_i64()),
            doc_obj.get("id").and_then(|v| v.as_i64()),
        )
    {
        return Ok(format!("doc{}_{}", owner_id, id));
    }

    Err(anyhow::anyhow!(
        "Could not find doc id in docs.save response: {}",
        value
    ))
}

/// Reaction type
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Reaction {
    pub reaction_id: i64,
    pub title: String,
}
