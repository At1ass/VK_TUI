use anyhow::{Context, Result};
use rand::Rng;
use reqwest::Client;
use std::{collections::HashMap, path::Path};

use crate::types::*;
use crate::{API_URL as VK_API_URL, API_VERSION as VK_API_VERSION};

/// VK API client
pub struct VkClient {
    client: Client,
    access_token: String,
}

impl VkClient {
    /// Create new VK API client
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    /// Make API request
    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: HashMap<&str, String>,
    ) -> Result<T> {
        let mut params = params;
        params.insert("access_token", self.access_token.clone());
        params.insert("v", VK_API_VERSION.to_string());

        let url = format!("{}/{}", VK_API_URL, method);

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to send request")?;

        let vk_response: VkResponse<T> =
            response.json().await.context("Failed to parse response")?;

        if let Some(error) = vk_response.error {
            anyhow::bail!("VK API error {}: {}", error.error_code, error.error_msg);
        }

        vk_response.response.context("Empty response from VK API")
    }

    /// Get conversations list
    pub async fn get_conversations(
        &self,
        offset: u32,
        count: u32,
    ) -> Result<ConversationsResponse> {
        let mut params = HashMap::new();
        params.insert("offset", offset.to_string());
        params.insert("count", count.to_string());
        params.insert("extended", "1".to_string());

        self.request("messages.getConversations", params).await
    }

    /// Get message history
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

        self.request("messages.getHistory", params).await
    }

    /// Send message
    pub async fn send_message(&self, peer_id: i64, message: &str) -> Result<i64> {
        self.send_with_attachments(peer_id, message, None).await
    }

    /// Mark messages as read
    pub async fn mark_as_read(&self, peer_id: i64) -> Result<i32> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());

        self.request("messages.markAsRead", params).await
    }

    /// Get user info
    #[allow(dead_code)]
    pub async fn get_users(&self, user_ids: &[i64]) -> Result<Vec<User>> {
        let mut params = HashMap::new();
        let ids: Vec<String> = user_ids.iter().map(|id| id.to_string()).collect();
        params.insert("user_ids", ids.join(","));
        params.insert("fields", "photo_50,online".to_string());

        self.request("users.get", params).await
    }

    /// Get Long Poll server
    pub async fn get_long_poll_server(&self) -> Result<LongPollServer> {
        let mut params = HashMap::new();
        params.insert("lp_version", "3".to_string());

        self.request("messages.getLongPollServer", params).await
    }

    /// Poll for Long Poll updates
    pub async fn long_poll(&self, server: &LongPollServer) -> Result<LongPollResponse> {
        // mode=234: attachments(2) + extended_events(8) + pts(32) + random_id(64) + extra_fields(128)
        let url = format!(
            "https://{}?act=a_check&key={}&ts={}&wait=25&mode=234&version=3",
            server.server, server.key, server.ts
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Long poll request failed")?;

        response
            .json()
            .await
            .context("Failed to parse long poll response")
    }

    /// Send message with photo attachment
    pub async fn send_photo(&self, peer_id: i64, path: &Path) -> Result<i64> {
        let mut server_params = HashMap::new();
        server_params.insert("peer_id", peer_id.to_string());
        let upload_server: UploadServer = self
            .request("photos.getMessagesUploadServer", server_params)
            .await?;

        let (boundary, body) = build_multipart_body(path, "photo")?;
        let response = self
            .client
            .post(upload_server.upload_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .context("Photo upload failed")?;

        let response_text = response.text().await?;
        tracing::debug!("Photo upload response: {}", response_text);

        // Parse response as generic JSON to capture all fields
        let upload_json: serde_json::Value = serde_json::from_str(&response_text)
            .context("Failed to parse photo upload response")?;

        // Convert all fields from upload response to params for saveMessagesPhoto
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

        tracing::debug!(
            "Calling photos.saveMessagesPhoto with params: {:?}",
            save_params
        );

        let saved: Vec<SavedPhoto> = self
            .request("photos.saveMessagesPhoto", save_params)
            .await?;
        let attachment = saved
            .first()
            .map(|p| format!("photo{}_{}", p.owner_id, p.id))
            .context("No saved photo returned")?;

        self.send_with_attachments(peer_id, "", Some(attachment))
            .await
    }

    /// Send message with document/file attachment
    pub async fn send_doc(&self, peer_id: i64, path: &Path) -> Result<i64> {
        let mut params = HashMap::new();
        params.insert("type", "doc".to_string());
        params.insert("peer_id", peer_id.to_string());
        let upload_server: UploadServer =
            self.request("docs.getMessagesUploadServer", params).await?;

        let (boundary, body) = build_multipart_body(path, "file")?;
        let response = self
            .client
            .post(upload_server.upload_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(body)
            .send()
            .await
            .context("Doc upload failed")?;

        let response_text = response.text().await?;
        tracing::debug!("Doc upload response: {}", response_text);

        // Parse response as generic JSON to capture all fields
        let upload_json: serde_json::Value =
            serde_json::from_str(&response_text).context("Failed to parse doc upload response")?;

        // Convert all fields from upload response to params for docs.save
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

        tracing::debug!("Calling docs.save with params: {:?}", save_params);

        let saved: Vec<SavedDoc> = self.request("docs.save", save_params).await?;
        let doc = saved
            .iter()
            .find_map(|d| d.doc.as_ref())
            .context("No doc object in docs.save response")?;
        let attachment = format!("doc{}_{}", doc.owner_id, doc.id);

        self.send_with_attachments(peer_id, "", Some(attachment))
            .await
    }

    /// Send message with optional attachments
    async fn send_with_attachments(
        &self,
        peer_id: i64,
        message: &str,
        attachment: Option<String>,
    ) -> Result<i64> {
        let mut params = HashMap::new();
        params.insert("peer_id", peer_id.to_string());
        if !message.is_empty() {
            params.insert("message", message.to_string());
        }
        if let Some(att) = attachment {
            params.insert("attachment", att);
        }
        params.insert("random_id", rand_id().to_string());

        self.request("messages.send", params).await
    }
}

/// Generate random message ID
fn rand_id() -> i64 {
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

    let boundary = format!("vk_tui_boundary_{}", rand_id());
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
